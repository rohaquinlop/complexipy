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

Zed registra los servidores de lenguaje en `settings.json`. Añade el servidor
bajo `lsp` y habilítalo para Python bajo `languages`:

```json
{
  "lsp": {
    "complexipy": {
      "binary": {
        "path": "complexipy",
        "arguments": ["lsp"]
      }
    }
  },
  "languages": {
    "Python": {
      "language_servers": ["complexipy", "..."]
    }
  }
}
```

La entrada `"..."` mantiene habilitados el resto de servidores de lenguaje de
Python que ya uses. Sin ella, solo se arrancan los servidores que listes
explícitamente.

!!! warning "Sin verificar todavía"

    Zed solo arranca servidores de lenguaje que conoce, y el bloque `binary`
    de arriba es un reemplazo para un servidor que ya tiene registrado. Si
    `complexipy` no aparece como servidor de Python al editar la lista
    `language_servers` en `settings.json`, Zed no lo tiene registrado y el
    bloque no hace nada hasta que una extensión registre el servidor. La
    configuración de Neovim está probada; la de Zed no.

## Configuración

El servidor lee los mismos archivos de configuración que la CLI -
`complexipy.toml`, `.complexipy.toml` o `pyproject.toml` (bajo
`[tool.complexipy]`) - descubiertos en la raíz del espacio de trabajo:

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
| `exclude` | `[]` | Patrones glob, emparejados de forma relativa a la raíz del espacio de trabajo. Los archivos excluidos no producen hints ni diagnósticos. |
| `no-ignore` | `false` | Analiza las funciones aunque un comentario de ignorado en línea las suprima. |
| `lsp.inlay-hints` | `"threshold"` | Cuándo mostrar el hint por función: `"threshold"`, `"always"` o `"never"`. |
| `lsp.per-line-hints` | `false` | Muestra además un hint `+N` en cada línea con un incremento de complejidad distinto de cero. |
| `lsp.diagnostics` | `true` | Publica advertencias para las funciones por encima de `max-complexity-allowed`. |

### Inlay hints

El hint se coloca al final de la línea `def`. En una función decorada es la
línea `def`, no la línea `@decorator` de encima, y en una firma que ocupa
varias líneas es la línea donde empieza la firma.

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

### Ignorados y exclusiones

- Los globs de `exclude` se emparejan de forma relativa a la raíz del espacio
  de trabajo. Los archivos excluidos no producen hints ni diagnósticos.
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
  `lsp.complexipy.binary`.

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
propio editor (`vim.lsp.inlay_hint.enable(true)` en Neovim) y recuerda que el
`inlay-hints = "threshold"` predeterminado solo muestra el hint cuando una
función está por encima de `max-complexity-allowed`. Usa
`inlay-hints = "always"` para ver todas las funciones.

### Todo el archivo muestra advertencias

Un límite más bajo de lo esperado normalmente significa que la configuración
activa no es la correcta. El servidor lee `complexipy.toml`,
`.complexipy.toml` o `pyproject.toml` desde la raíz del espacio de trabajo;
comprueba que el archivo que el editor abrió como raíz del espacio de trabajo
sea el que contiene tus ajustes. El servidor lee el archivo al arrancar y de
nuevo cada vez que el editor informa un cambio de configuración, así que
después de editarlo pide al editor que recargue (`:LspRestart` en Neovim, o
recargar la ventana en Zed).

### Los per-line hints no aparecen

`per-line-hints` está desactivado de forma predeterminada. Ponlo en `true` para
añadir los hints `+N`.

## Límites

Las comprobaciones de ratchet con `--diff` y las compuertas por código de
salida siguen siendo solo de la CLI. El servidor de lenguaje está pensado para
guiarte mientras escribes, no para sustituir el contrato de CI, así que el uso
actual de CI no cambia.
