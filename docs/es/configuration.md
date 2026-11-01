# Configuración

## Archivos de Configuración

### Prioridad de Configuración

complexipy carga la configuración en este orden (de mayor a menor prioridad):

1. Argumentos de línea de comandos
1. `complexipy.toml`
1. `.complexipy.toml`
1. `pyproject.toml` (bajo `[tool.complexipy]`)

complexipy lee el primero de estos archivos que se analiza y tiene valores
válidos, y nunca combina archivos. Cuando un archivo de mayor prioridad oculta
a otro de menor prioridad que también tiene ajustes, una advertencia nombra el
archivo que no se aplica.

`--isolated` ignora todos los archivos de configuración, y `--config <ruta>`
lee solo el archivo indicado: su sección `[tool.complexipy]` cuando el archivo
se llama `pyproject.toml`, y el archivo completo en otro caso. Usa cualquiera
de los dos en CI para fijar el gate, de modo que un cambio en la configuración
del repositorio no pueda alterarlo.

Un archivo de configuración que no es un archivo regular, que pesa más de
4 MiB, o que es un enlace simbólico que sale del directorio se omite con una
advertencia. Un error de configuración nombra el archivo, la línea y la
columna, o la clave, y nunca repite el contenido del archivo.

Los valores leídos de un archivo de configuración se validan antes de usarse:
`paths`, `output` y `cache-dir` deben quedar dentro del directorio desde el
que se ejecuta complexipy, incluidos los enlaces simbólicos, y una `branch` de
diff no puede empezar con `-`. Pasa una ruta fuera del proyecto en la línea de
comandos.

### Configuraciones de Ejemplo

=== "complexipy.toml"

    ```toml
    paths = ["src", "tests"]
    max-complexity-allowed = 10
    exclude = ["migrations/**", "build/**"]
    snapshot-create = false
    snapshot-ignore = false
    quiet = false
    ignore-complexity = false
    failed = false
    color = "auto"
    sort = "asc"
    output-format = ["json", "gitlab"]
    output = "reports/"
    check-script = false
    no-ignore = false
    report-ignored = false
    force-exclude = false
    cache-dir = ".complexipy_cache"
    ```

=== "pyproject.toml"

    ```toml
    [tool.complexipy]
    paths = ["src", "tests"]
    max-complexity-allowed = 10
    exclude = ["migrations/**", "build/**"]
    failed = true
    sort = "desc"
    check-script = true
    cache-dir = ".complexipy_cache"
    ```

=== ".complexipy.toml"

    ```toml
    # Archivo de configuración oculto para ajustes específicos del equipo
    max-complexity-allowed = 15
    exclude = ["venv/**", ".venv/**", "node_modules/**"]
    cache-dir = ".complexipy_cache"
    ```

`check-script` está soportado en TOML. `--top` y `--plain` son flags solo de CLI.

### JSON Schema

`complexipy.schema.json`, en la raíz del repositorio, describe cada clave de
la configuración, con su tipo, su valor por defecto y su descripción. Un
servidor de lenguaje TOML como [Taplo](https://taplo.tamasfe.dev/) (Even
Better TOML en VS Code) lo usa para el autocompletado, el texto al pasar el
cursor y la validación. Para usarlo en `complexipy.toml` o `.complexipy.toml`,
agrega esta línea al inicio del archivo:

```toml
#:schema https://raw.githubusercontent.com/rohaquinlop/complexipy/main/complexipy.schema.json
```

El esquema rechaza las claves desconocidas, así que el editor marca un error
de escritura como `max-complexity-alowed`. complexipy ignora las claves
desconocidas.

### Configuración de Diff

La política de comparación se puede declarar una sola vez en el repositorio
en lugar de pasar los mismos flags en cada llamada. Añade una sección
`[tool.complexipy.diff]` al mismo archivo de configuración:

=== "complexipy.toml"

    ```toml
    [diff]
    branch = "main"
    staged = true
    ```

=== "pyproject.toml"

    ```toml
    [tool.complexipy.diff]
    branch = "main"
    staged = true
    ```

- `branch` establece la referencia por defecto para `--diff` y
  `--diff-only`. Un `complexipy .` simple entonces se comporta como
  `complexipy . --diff main`, incluida la aplicación del umbral. Pasa
  `--diff <ref>` o `--diff-only <ref>` para sobrescribir la referencia en
  una ejecución concreta.
- `staged` habilita la comparación staged por defecto, como pasar
  `--staged` en cada llamada.
- Los flags de CLI siempre tienen prioridad sobre los valores de la sección.
- `branch = ""` deshabilita el diff para el repositorio actual (opt-out),
  y con él el valor `staged` de la sección.
- Una sección `[diff]` cambia lo que verifica el código de salida, así que
  cada ejecución en la que se aplica imprime una advertencia en stderr que
  nombra el archivo. Pasa `--diff`, `--isolated` o `--config <ruta>` para
  fijar el gate en CI.
- Orden de resolución: flag de CLI, luego la sección `diff`.
- Si la `branch` configurada no existe en el clon local (por ejemplo un
  clon recién hecho o shallow), cada función se reporta como `NEW`, la
  aplicación del umbral sigue activa y la ejecución imprime una advertencia
  que nombra la referencia y el repositorio. Haz fetch de la rama o pasa
  `--diff <ref>` con una referencia existente.

### Configuración del Servidor de Lenguaje

El [servidor de lenguaje](editors.md) lee los mismos archivos de configuración
que la CLI, desde la raíz del espacio de trabajo, más una sección `lsp` que
controla lo que muestra el editor:

=== "complexipy.toml"

    ```toml
    max-complexity-allowed = 15
    exclude = ["migrations/**"]

    [lsp]
    inlay-hints = "threshold"
    per-line-hints = false
    diagnostics = true
    workspace-diagnostics = false
    ```

=== "pyproject.toml"

    ```toml
    [tool.complexipy]
    max-complexity-allowed = 15
    exclude = ["migrations/**"]

    [tool.complexipy.lsp]
    inlay-hints = "threshold"
    per-line-hints = false
    diagnostics = true
    workspace-diagnostics = false
    ```

| Clave | Descripción | Valor por defecto |
| -- | -- | -- |
| `inlay-hints` | `threshold` muestra la pista por función solo por encima de `max-complexity-allowed`, `always` la muestra en cada función, `never` oculta todas las pistas | `threshold` |
| `per-line-hints` | Muestra además la complejidad que aporta cada línea, como `+2`. Las líneas que no aportan nada se omiten | `false` |
| `diagnostics` | Publica una advertencia por cada función por encima de `max-complexity-allowed` | `true` |
| `workspace-diagnostics` | Publica también advertencias para los archivos que no están abiertos, a partir de un recorrido de la carpeta del espacio de trabajo | `false` |

El servidor lee las claves de nivel superior `max-complexity-allowed`,
`paths`, `exclude`, `select`, `ignore` y `no-ignore`, e ignora el resto de claves del
mismo archivo: `quiet`, `failed`, `sort`, `color`, `output`,
`output-format`, `cache-dir`, `snapshot-create`, `snapshot-ignore`,
`ignore-complexity`, `check-script`, `report-ignored` y `diff`. Una función
cuya complejidad es igual al umbral pasa, igual que en la línea de comandos.

## Configuración de la caché

complexipy almacena los datos de complejidad de cada función entre ejecuciones
para permitir comparaciones de cambios/diferencias (delta/diff). De forma
predeterminada, esta caché se encuentra en `.complexipy_cache/` dentro del directorio
desde el que se ejecuta la herramienta. En lugar de utilizar la ubicación
predeterminada, puedes redirigir la caché a cualquier directorio. Esto resulta útil
para CI, directorios de caché compartidos o para evitar añadir archivos innecesarios
al árbol del proyecto.

```bash
# Flag de CLI (mayor prioridad)
complexipy . --cache-dir .cache/complexipy
```

O declárala una vez en un archivo de configuración:

=== "complexipy.toml"

    ```toml
    cache-dir = ".cache/complexipy"
    ```

=== "pyproject.toml"

    ```toml
    cache-dir = ".cache/complexipy"
    ```

=== ".complexipy.toml"

    ```toml
    cache-dir = ".cache/complexipy"
    ```
