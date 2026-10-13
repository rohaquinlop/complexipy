# Integración con Editores

complexipy incluye un servidor de Language Server Protocol (LSP), de modo que
tu editor puede mostrar el mismo análisis de complejidad cognitiva que produce
la CLI mientras escribes código. El servidor usa el mismo motor Rust que
potencia `complexipy` y la API de Python, y no necesita configuración para
empezar a ser útil: instala complexipy, apunta tu editor a `complexipy lsp` y
sigue trabajando.

En el editor obtienes:

- **Inlay hints** - la complejidad cognitiva de una función, mostrada al final
  de su línea `def` como `cognitive: 18`.
- **Hover** - al pasar el cursor por cualquier punto dentro de una función se
  muestra su nombre, su complejidad cognitiva, si supera el umbral permitido y
  el título de la principal sugerencia de refactorización cuando existe.
- **Diagnósticos** - una advertencia por cada función que supere
  `max-complexity-allowed`, con el mensaje
  `cognitive complexity 18 exceeds the allowed 15` en un rango que cubre toda
  la función.

Los resultados se actualizan mientras escribes. El servidor usa sincronización
completa del documento y analiza solo los documentos que tienes abiertos, así
que no hay un escaneo de todo el espacio de trabajo ni hace falta guardar.

## Instalación

Instala complexipy para que el comando `complexipy` esté en tu `PATH`:

```bash
uv tool install complexipy
```

o, con pip:

```bash
pip install complexipy
```

Cualquier cosa que instale el comando `complexipy` funciona. El editor arranca
el servidor como un subproceso con `complexipy lsp`; no existe una descarga de
binario aparte. Actualiza la herramienta con `uv tool upgrade complexipy` o
`pip install --upgrade complexipy` para obtener las nuevas reglas y ajustes.

## Neovim

Neovim 0.11 y posteriores pueden configurar y habilitar el servidor con
`vim.lsp.config` y `vim.lsp.enable`:

```lua
vim.lsp.config("complexipy", {
  cmd = { "complexipy", "lsp" },
  filetypes = { "python" },
  root_markers = { "complexipy.toml", ".complexipy.toml", "pyproject.toml", ".git" },
})

vim.lsp.enable("complexipy")
```

En Neovim 0.10, arranca el servidor directamente en su lugar:

```lua
vim.lsp.start({ name = "complexipy", cmd = { "complexipy", "lsp" } })
```

Los inlay hints son opcionales en Neovim. Habilítalos una vez, de forma global:

```lua
vim.lsp.inlay_hint.enable(true)
```

o por búfer con `vim.lsp.inlay_hint.enable(true, { bufnr = 0 })`.

## Zed

Zed solo arranca un servidor de lenguaje cuando Zed o una extensión lo
registra. La extensión de complexipy está en la carpeta `zed/` del repositorio.
Hasta que se publique en el registro de extensiones de Zed, instálala como
extensión de desarrollo:

1. Clona `https://github.com/rohaquinlop/complexipy`.
1. Ejecuta `zed: install dev extension` desde la paleta de comandos y elige la
   carpeta `zed/`.

Después habilita el servidor y los inlay hints para Python en `settings.json`:

```json
{
  "inlay_hints": { "enabled": true },
  "languages": {
    "Python": {
      "language_servers": ["complexipy", "..."]
    }
  }
}
```

La entrada `"..."` mantiene habilitados el resto de servidores de lenguaje de
Python que ya uses. Si tus ajustes ya listan servidores de Python sin `"..."`,
añade `"complexipy"` a esa lista, o Zed no lo arranca.

Zed desactiva los inlay hints de forma predeterminada. `inlay_hints.enabled` los
activa para todos los servidores de lenguaje, así que los hints de tipos de
servidores como `ty` o `basedpyright` aparecen junto al hint de complexipy.

La extensión ejecuta `complexipy lsp` desde el `PATH`. Para usar otro binario,
define su ruta:

```json
{
  "lsp": {
    "complexipy": {
      "binary": {
        "path": "/ruta/completa/a/complexipy",
        "arguments": ["lsp"]
      }
    }
  }
}
```

## Configuración

El servidor lee los mismos archivos de configuración que la CLI -
`complexipy.toml`, `.complexipy.toml` o `pyproject.toml` (bajo
`[tool.complexipy]`) - descubiertos en la raíz de cada carpeta del espacio de trabajo:

=== "complexipy.toml"

    ```toml
    max-complexity-allowed = 15

    [lsp]
    inlay-hints = "threshold"   # "always" | "threshold" | "never"
    per-line-hints = false
    diagnostics = true
    ```

=== "pyproject.toml"

    ```toml
    [tool.complexipy]
    max-complexity-allowed = 15

    [tool.complexipy.lsp]
    inlay-hints = "threshold"   # "always" | "threshold" | "never"
    per-line-hints = false
    diagnostics = true
    ```

| Clave | Predeterminado | Descripción |
| -- | -- | -- |
| `max-complexity-allowed` | `15` | Se reportan las funciones estrictamente por encima de este valor. Una función en el límite pasa. |
| `exclude` | `[]` | Patrones glob, emparejados de forma relativa a la carpeta del espacio de trabajo que contiene el archivo de configuración. Los archivos excluidos no producen hints ni diagnósticos. |
| `no-ignore` | `false` | Analiza las funciones aunque un comentario de ignorado en línea las suprima. |
| `lsp.inlay-hints` | `"threshold"` | Cuándo mostrar el hint por función: `"threshold"`, `"always"` o `"never"`. |
| `lsp.per-line-hints` | `false` | Muestra además un hint `+N` en cada línea con un incremento de complejidad distinto de cero. |
| `lsp.diagnostics` | `true` | Publica advertencias para las funciones por encima de `max-complexity-allowed`. |

El servidor ignora el resto de claves del mismo archivo: `paths`, `quiet`,
`failed`, `sort`, `color`, `output`, `output-format`, `cache-dir`,
`snapshot-create`, `snapshot-ignore`, `ignore-complexity`, `check-script`,
`report-ignored` y `diff`. Esas claves se usan con `complexipy <path>`.

El servidor vuelve a leer la configuración cuando guardas un cambio en uno de
estos archivos en la raíz de una carpeta del espacio de trabajo. El editor debe
admitir el registro dinámico de `workspace/didChangeWatchedFiles`. Si un
archivo guardado no se puede analizar, el editor muestra una advertencia y la
carpeta usa la configuración por defecto. Con un editor que no vigila archivos,
reinicia el servidor después de cambiar la configuración.

### Inlay hints

El hint se coloca al final de la última línea de la declaración de la función:
la línea que cierra la firma. En una firma de una sola línea esa es la línea
`def`, y en una función decorada es la línea `def` y no la línea `@decorator`
de encima. En una firma que ocupa varias líneas el hint queda tras el `)` o el
`):` de cierre:

```python
def heavy(
    a,
    b,
):
```

En este ejemplo el hint queda al final de la línea `):`.

- `inlay-hints = "threshold"` (el predeterminado) muestra el hint por función
  (`cognitive: 18`) solo cuando la función está **por encima** de
  `max-complexity-allowed`.
- `inlay-hints = "always"` lo muestra para cada función.
- `inlay-hints = "never"` suprime todos los hints, incluidos los per-line
  hints.
- `per-line-hints = true` muestra además un hint `+N` al final de cada línea
  que tenga un incremento de complejidad distinto de cero. Las líneas con un
  incremento de cero nunca producen un hint.

El servidor reporta las mismas funciones que reporta `complexipy <path>`:
funciones de nivel superior, más los métodos de las clases. Una función
anidada dentro de otra se pliega en su función padre, así que no tiene un hint
propio. Por eso, al pasar el cursor dentro de una función anidada se muestra su
función padre.

### Diagnósticos

- `diagnostics = false` no publica ninguna advertencia.
- El umbral reportado coincide con `complexipy <path>`: una función cuya
  complejidad **es igual** a `max-complexity-allowed` pasa. Solo se reportan
  los valores estrictamente mayores, como
  `cognitive complexity 18 exceeds the allowed 15`.
- Un documento que el analizador sintáctico no puede leer produce exactamente
  una advertencia con el código `complexipy-parse-error` en el rango donde
  falla el analizador, con su mensaje.
  Mientras esa advertencia está presente, los hints y el hover siguen
  respondiendo con el último texto que sí se analizó, así que el archivo no se
  queda en blanco mientras escribes. La advertencia desaparece en cuanto el
  documento vuelve a analizarse.

### Ignorados y exclusiones

- Los globs de `exclude` se emparejan de forma relativa a la carpeta del
  espacio de trabajo que contiene el archivo de configuración. Los archivos excluidos no producen hints ni diagnósticos.
- Un glob mal formado se reporta una vez por carga de configuración y se
  omite. El resto de patrones de la lista siguen aplicándose. Cada problema de
  configuración (un glob mal formado, una lista demasiado grande, un id de regla
  desconocido en `select` o `ignore`, una selección que no deja ninguna regla
  activa o un archivo que no se puede leer) aparece en el editor como un mensaje
  de advertencia y en el registro del servidor.
- Una lista demasiado grande para compilarse como un solo programa se reporta
  una vez por carga de configuración y se sigue aplicando patrón a patrón. Una
  ejecución de la CLI se detiene con esa misma lista.
- Los comentarios de supresión en línea (`# noqa: complexipy` y
  `# complexipy: ignore`) se respetan exactamente igual que en la CLI, a menos
  que `no-ignore = true`.

## Solución de Problemas

### El editor no encuentra el comando `complexipy`

El editor arranca el servidor como un subproceso, así que `complexipy` debe
estar en el `PATH` que hereda el proceso del editor. Apunta el servidor a una
ruta absoluta cuando la herramienta esté instalada en un lugar que el editor no
puede ver:

- Neovim: define `cmd = { "/ruta/completa/a/complexipy", "lsp" }`.
- Zed: define `"path"` con la ruta absoluta del binario en el bloque
  `lsp.complexipy.binary`. Sin él, la extensión busca `complexipy` en el
  `PATH`, y Zed muestra un error cuando no está.

Como alternativa, arráncalo a través de `uv` sin instalar nada globalmente:

```lua
cmd = { "uvx", "complexipy", "lsp" },
```

### Dónde leer los registros del servidor

El servidor nunca escribe nada que no sean tramas de protocolo en stdout; todos
los registros van a stderr. Léelos donde tu editor los recopila:

- Neovim: `:LspLog`.
- Zed: abre el panel de registros desde la paleta de comandos
  (`zed: open log`).

### Los hints no aparecen

Los inlay hints son opcionales en la mayoría de los editores. Habilítalos en el
propio editor (`vim.lsp.inlay_hint.enable(true)` en Neovim,
`"inlay_hints": { "enabled": true }` en Zed) y recuerda que el
`inlay-hints = "threshold"` predeterminado solo muestra el hint cuando una
función está por encima de `max-complexity-allowed`. Usa
`inlay-hints = "always"` para ver todas las funciones.

### Todo el archivo muestra advertencias

Un límite más bajo de lo esperado normalmente significa que la configuración
activa no es la correcta. El servidor lee `complexipy.toml`,
`.complexipy.toml` o `pyproject.toml` desde la raíz de la carpeta del espacio
de trabajo del documento; comprueba que la carpeta que abrió el editor sea la
que contiene tus ajustes. El servidor lee el archivo al arrancar, y de nuevo
cuando lo guardas en un editor que vigila archivos para el servidor. En otros
editores, reinicia el servidor después de editarlo (`:LspRestart` en Neovim,
`editor: restart language server` en Zed).

### Los per-line hints no aparecen

`per-line-hints` está desactivado de forma predeterminada. Ponlo en `true` para
añadir los hints `+N`.

## Límites

Las comprobaciones de ratchet con `--diff` y las compuertas por código de
salida siguen siendo solo de la CLI. El servidor de lenguaje está pensado para
guiarte mientras escribes, no para sustituir el contrato de CI, así que el uso
actual de CI no cambia.

Otros límites del servidor actual:

- El servidor solo analiza documentos de Python. Un documento cuyo lenguaje no
  sea `python` y cuya ruta no termine en `.py` no produce hints ni
  advertencias.
- El servidor declara sincronización completa del documento, así que espera el
  búfer entero en cada cambio. Un cambio que lleve un rango se ignora y se
  escribe una línea en el registro del servidor.
- En un espacio de trabajo con varias carpetas, cada carpeta lee su propio
  archivo de configuración. Un documento usa la carpeta más profunda que lo
  contiene, y un documento fuera de todas las carpetas usa la primera. Las
  carpetas que el editor añade o quita se aplican sin reiniciar.
- Las `initializationOptions` del cliente se ignoran. La configuración sale del
  archivo de configuración.
- La palabra `lsp` reserva el primer argumento: `complexipy lsp` arranca el
  servidor. Para analizar un archivo o directorio llamado `lsp`, ejecuta
  `complexipy -- lsp`. En una terminal, el servidor imprime esa pista cuando
  existe una ruta llamada `lsp`.
