# Guía de Uso

Esta guía cubre todo lo que necesitas saber para usar complexipy de manera efectiva en tus proyectos de Python.

## Instalación

=== "pip"

    ```bash
    pip install complexipy
    ```

=== "uv"

    ```bash
    uv add complexipy
    ```

=== "uv tool"

    ```bash
    uv tool install complexipy
    ```

=== "poetry"

    ```bash
    poetry add complexipy
    ```

El comando `complexipy` es una implementación nativa escrita en Rust;
todo el pipeline de análisis se ejecuta sin un intérprete de Python.
La misma rueda también proporciona la API de Python para integrar el
análisis en tus propias herramientas.

## Uso en Línea de Comandos

### Análisis Básico

Analiza tu proyecto completo:

```bash
complexipy .
```

Analiza archivos o directorios específicos:

```bash
complexipy src/
complexipy src/main.py
complexipy src/ tests/
```

### Establecer Umbral de Complejidad

El umbral predeterminado es 15. Las funciones que superen este valor serán resaltadas:

```bash
complexipy . --max-complexity-allowed 10
```

### Filtrar Resultados

Mostrar solo las funciones que superen el umbral:

```bash
complexipy . --failed
```

Mostrar solo las N funciones más complejas entre todos los archivos analizados:

```bash
complexipy . --top 10
```

Mostrar sugerencias deterministas de refactorización para funciones fallidas:

```bash
complexipy . --failed --suggest-refactors
```

Cuando se usa `--top`, los resultados se reordenan globalmente por complejidad
descendente antes de truncarse.

Suprimir la salida del análisis (útil para pipelines de CI):

```bash
complexipy . --quiet
```

### Ordenar Resultados

Ordenar por puntuación de complejidad:

```bash
complexipy . --sort asc   # Ascendente (predeterminado)
complexipy . --sort desc  # Descendente
complexipy . --sort file_name  # Alfabéticamente por nombre de archivo
```

### Excluir Archivos y Directorios

Excluir rutas específicas del análisis:

```bash
# Excluir un directorio recursivamente
complexipy . --exclude "tests/**"

# Excluir múltiples directorios recursivamente
complexipy . --exclude "tests/**" --exclude "migrations/**" --exclude "build/**"

# Excluir archivos específicos
complexipy . --exclude "src/legacy/old_code.py"
```

!!! note "Cómo funciona la exclusión"

    - Las exclusiones son patrones glob evaluados de forma relativa a cada ruta raíz proporcionada, y de forma relativa al directorio donde se ejecuta el comando, donde está el archivo de configuración
    - Un patrón que empieza con `/`, `./` o `../`, o termina en `/`, nunca coincide; complexipy avisa de ello
    - Usa `directory/**` para excluir un directorio recursivamente
    - Usa una ruta relativa exacta, como `src/legacy/old_code.py`, para excluir un archivo
    - Las reglas de gitignore se siguen respetando durante el descubrimiento de archivos, y también los archivos `.ignore` y los archivos y directorios ocultos. `--report-ignored` lista cada archivo `*.py` que estas reglas omiten, y una ejecución que no selecciona ningún archivo imprime una advertencia en stderr, incluso bajo `--quiet`
    - Un enlace simbólico `*.py` se analiza solo cuando apunta a un archivo regular dentro del directorio analizado. Los enlaces que salen de él, los enlaces rotos, los dispositivos y los directorios llamados `*.py` se omiten con una advertencia
    - Un archivo fuente de más de 64 MiB se reporta como fallido en lugar de leerse

### Formatos de Salida

Guarda los resultados en JSON, CSV, GitLab Code Quality o SARIF:

```bash
# Salida JSON (guardada en complexipy-results.json)
complexipy . --output-format json

# Salida CSV (guardada en complexipy-results.csv)
complexipy . --output-format csv

# Ambos
complexipy . --output-format json --output-format csv

# Destino explícito para un solo formato
complexipy . --output-format gitlab --output complexipy-code-quality.json

# Varios formatos escritos en un directorio
complexipy . --output-format json --output-format sarif --output reports/

# Añade --suggest-refactors a cualquiera de los anteriores para incluir
# también los hallazgos de las reglas de refactorización (C001, C007, ...)
# como sus propios IDs de regla SARIF/GitLab, junto a los hallazgos de
# complejidad cognitiva que siempre están presentes.
complexipy . --output-format sarif --suggest-refactors
```

JSON, SARIF y GitLab Code Quality siguen la misma regla: los datos de los
planes de refactorización (`refactor_plans` en JSON, hallazgos por regla en
SARIF/GitLab) solo se emiten cuando también se pasa `--suggest-refactors`.
Sin esa flag, solo aparecen los hallazgos del umbral de complejidad
cognitiva -- igual que la salida enriquecida de la CLI, que también oculta
las sugerencias de refactorización a menos que se active la flag.

### Diff de Complejidad

Compara los resultados actuales contra cualquier referencia de git:

```bash
complexipy . --diff HEAD~1
complexipy . --diff main
complexipy src/ --max-complexity-allowed 10 --diff HEAD~1
```

Por defecto `--diff` **aplica** el umbral de complejidad: la ejecución termina con código `1` solo cuando un cambio rompe el contrato definido por `--max-complexity-allowed`:

- Una función **nueva** introducida por encima del umbral.
- Una función **modificada** cuya complejidad aumentó **y** queda por encima del umbral (incluye funciones ya sobre el umbral que empeoran).

Las funciones que aumentan su complejidad pero se mantienen en o por debajo del umbral (por ejemplo `3 → 4` con `--max-complexity-allowed 15`) **no** son fallos.

Para ver el diff visualmente sin afectar el código de salida, usa `--diff-only` en su lugar:

```bash
complexipy . --diff-only HEAD~1
```

Para comparar el contenido **staged** (el índice de git) en lugar del árbol de trabajo - "¿qué complejidad estoy a punto de commitear?" - añade `--staged`:

```bash
# Cambios staged vs HEAD (la línea base por defecto de --staged)
complexipy . --staged

# Cambios staged vs una referencia específica
complexipy . --diff main --staged
```

Igual que `--diff`, `--staged` aplica el umbral contra el contenido staged y falla cuando un cambio staged empuja una función por encima de `--max-complexity-allowed`. Las eliminaciones staged producen entradas `REMOVED` y las adiciones staged entradas `NEW`. Usa `--diff-only` con `--staged` para una vista solo visual.

Esto requiere `git` y una ruta dentro de un repositorio. `--diff` y
`--diff-only` leen cada archivo del repositorio que lo contiene, así que
una ejecución desde fuera del repositorio compara contra el mismo historial
que una ejecución desde dentro. Un submódulo, es decir, un repositorio que
el repositorio que lo rodea rastrea como gitlink en su índice, se lee en el
commit que el repositorio exterior fija para él en la referencia, y un
anidamiento más profundo sigue cada fijación a su vez. Cuando la ejecución
empieza dentro del submódulo, se lee en la propia referencia. Un
repositorio dentro de otro que el exterior no rastrea se lee en la
referencia, como cualquier otro repositorio. `--staged` usa el repositorio
del directorio de trabajo. Cuando un archivo está fuera de todo
repositorio, la referencia no nombra ningún commit en el repositorio del
archivo, o el repositorio exterior no fija ningún commit para el submódulo
en la referencia, o fija uno que el submódulo no tiene, cada función de ese
archivo se reporta como `NEW` y la ejecución imprime una advertencia que
nombra la causa.

En lugar de repetir la referencia en cada llamada, declara la política de
comparación una sola vez en un archivo de configuración - ver
[Configuración de Diff](#configuraci%C3%B3n-de-diff).

### Salida en Texto Plano

Usa la salida en texto plano cuando necesites una línea legible por máquina por
función:

```bash
complexipy . --plain
complexipy . --plain --top 5
complexipy . --plain --failed --max-complexity-allowed 10
```

Cada línea se emite con este formato:

```text
<path> <function> <complexity>
```

`--plain` es solo para CLI y no se puede combinar con `--quiet`.

### Complejidad de Script a Nivel de Módulo

Usa `--check-script` para incluir el código a nivel de módulo en los resultados
como `<module>`:

```bash
complexipy path/to/script.py --check-script
complexipy path/to/script.py --check-script --max-complexity-allowed 5
```

Esto es útil para scripts con flujo de control complejo en el nivel superior,
fuera de las funciones.

Cada función que define un módulo se puntúa por separado, esté donde esté el
`def`: bajo un `if`, `try`, `with`, `for`, `while` o `match` a nivel de módulo,
y en clases anidadas, cuyos métodos se llaman `Outer::Inner::method`. Una
función definida dentro de otra función cuenta para la función exterior.

Un archivo fuente con una declaración de codificación PEP 263 se lee en esa
codificación: UTF-8 y Latin-1 siempre, y una codificación compatible con ASCII
como `cp1252` cuando el archivo es ASCII puro. Un archivo que declara otra
codificación, como `utf-7`, se reporta como fallido, porque leerlo como UTF-8
no mostraría el código que ejecuta Python.

### Sugerencias de Refactorización

Usa `--suggest-refactors` para imprimir un conjunto pequeño y ordenado de planes deterministas de refactorización junto a los resultados enriquecidos de la CLI:

```bash
complexipy . --failed --suggest-refactors
```

Salida de ejemplo (abreviada -- la salida real también muestra un tramo subrayado con acentos circunflejos, el código fuente circundante y un enlace a la documentación):

```text
      [1] C007 Merge nested if statements
          --> sample.py:4:9
          Category: ◆ Readability | Applicability: * Safe to apply
          Lines 4-6 -> Estimated reduction: -2 complexity (6 -> 4)

          Suggestion: * Safe to apply
          Merge nested conditions into `if item.active and item.ready:`
```

Los planes se basan solo en el análisis AST de Rust; no se usa IA y no se reescribe código automáticamente. Las reducciones estimadas son aproximadas, ordenadas y limitadas, así que trátalas como orientación, no como puntuaciones futuras exactas. `--plain --suggest-refactors` mantiene la salida plana sin cambios.

### Aplicar Sugerencias Aplicables

Usa `--fix` para escribir las sugerencias aplicables en los archivos fuente. Una sugerencia es aplicable cuando su aplicabilidad es **Seguro de aplicar** y su reemplazo es un empalme fiel (`"spliceable": true` en la salida JSON). Hoy eso cubre C002 (guardas de bucle) y C007 (if plegable). C005 es seguro de aplicar, pero su reemplazo contiene un cuerpo de marcador de posición y una segunda edición en la llamada, así que `--fix` lo salta.

```bash
complexipy . --fix
```

`--fix` solo cambia funciones cuya complejidad supera `max-complexity-allowed`, la misma regla que usa el control. Cada paso vuelve a comprobar el umbral, así que una función deja de cambiar en cuanto lo cumple. Una función en el umbral o por debajo conserva sus sugerencias en el informe y nunca se reescribe. Una ejecución aplica cada plan aplicable de esas funciones en cada archivo analizado y repite paso a paso, hasta 8 pasos, hasta que no quede ningún plan aplicable; un paso posterior etiqueta su diff como `pass 2:`. Por cada archivo corregido, la ejecución imprime un diff unificado con resaltado de sintaxis de los cambios aplicados, con números de línea, una línea de contexto por bloque y la regla que se activó, y luego un resumen de las correcciones aplicadas y omitidas. El resumen muestra la reducción de cada corrección como `(-3 complexity)`, con una tilde delante del número (`(- ~3 complexity)`) cuando el número es estimado. Las líneas del resumen usan los números de línea del archivo anteriores a la ejecución, en cada paso; una corrección dentro de una región que un paso anterior reescribió informa las líneas de esa región. Cuando no hay nada aplicable, la ejecución imprime `No fixes to apply.` El informe y cada control miden el estado después de las correcciones. Reanalizar un archivo corregido muestra que el hallazgo desapareció y que la complejidad bajó en la reducción medida del plan.

`--fix --dry-run` previsualiza los mismos cambios: imprime solo el diff unificado y no escribe nada, ni siquiera la caché o el snapshot. `--fix` se ejecuta donde estés: los cambios sin confirmar, los archivos sin rastrear y la ausencia de un repositorio de git nunca lo bloquean, y no hace falta ninguna bandera extra. Una corrección conserva los finales de línea del archivo y su salto de línea final. El filtrado por regla se apoya en `--select` e `--ignore`.

`--fix` reescribe solo archivos regulares dentro del directorio desde el que se ejecuta. Omite un enlace simbólico y un archivo que cambió en disco después de que complexipy lo leyera. Escribe un archivo nuevo junto al original y lo renombra en su lugar, conservando los bits de permisos y el grupo, pero no setuid, setgid, los atributos extendidos ni las ACL.

**Estructura de Salida JSON:**

```json
[
    {
        "path": "src",
        "file_name": "main.py",
        "function_name": "process_items",
        "complexity": 6,
        "refactor_plans": [
            {
                "rule_id": "C007",
                "kind": "collapsible_if",
                "title": "Merge nested if statements",
                "line_start": 4,
                "line_end": 6,
                "column_start": 9,
                "current_complexity": 6,
                "estimated_reduction": 2,
                "estimated_complexity_after": 4,
                "category": "Readability",
                "applicability": "MachineApplicable",
                "description": "Merge nested if statements into a single if with combined conditions",
                "explanation": "Nested if statements with a single body can be merged into a single if with combined conditions using 'and'. This reduces nesting and improves readability.",
                "references": [],
                "suggestion": {
                    "replacement": "        if item.active and item.ready:\n            total += item.value",
                    "applicability": "MachineApplicable",
                    "description": "Merge nested conditions into `if item.active and item.ready:`"
                },
                "help": null,
                "doc_url": "https://complexipy.com/refactoring-rules/#c007-collapsible-if"
            }
        ]
    }
]
```

La salida JSON contiene una entrada por cada función emitida. La lista `refactor_plans` solo se completa cuando también se pasa `--suggest-refactors` -- de lo contrario es `[]`, igual que el comportamiento de la salida enriquecida de la CLI. La salida CSV no incluye planes, y una celda que una hoja de cálculo ejecutaría como fórmula (una que empieza con `=`, `+`, `-` o `@`) se escribe con un `'` inicial. Los rangos de líneas de funciones están disponibles mediante la API de Python (`line_start`, `line_end`), pero no se incluyen en las entradas de función JSON/CSV legibles por máquina de la CLI.

### Salida en Color

Controla la salida en color:

```bash
complexipy . --color auto  # Predeterminado: detecta automáticamente la compatibilidad con la terminal
complexipy . --color yes   # Forza colores
complexipy . --color no    # Deshabilita colores
```

`auto` da estilo a un flujo solo cuando es una terminal y `NO_COLOR` no está definido o está vacío. `--color` cubre cada línea con estilo, incluido el diff de `--fix` y los avisos en stderr, y la salida `--plain` nunca lleva estilo.

El texto que viene de los archivos analizados se imprime con sus caracteres de control escapados: una ruta, una línea de código o un valor de configuración muestra un carácter de escape como `\u{1b}` y un salto de línea como `\u{a}`, así que no puede mover el cursor, reescribir líneas anteriores, cambiar el título de la ventana ni imprimir un enlace oculto.

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
    ```

| Clave | Descripción | Valor por defecto |
| -- | -- | -- |
| `inlay-hints` | `threshold` muestra la pista por función solo por encima de `max-complexity-allowed`, `always` la muestra en cada función, `never` oculta todas las pistas | `threshold` |
| `per-line-hints` | Muestra además la complejidad que aporta cada línea, como `+2`. Las líneas que no aportan nada se omiten | `false` |
| `diagnostics` | Publica una advertencia por cada función por encima de `max-complexity-allowed` | `true` |

El servidor lee las claves de nivel superior `max-complexity-allowed`,
`exclude`, `select`, `ignore` y `no-ignore`, e ignora el resto de claves del
mismo archivo: `paths`, `quiet`, `failed`, `sort`, `color`, `output`,
`output-format`, `cache-dir`, `snapshot-create`, `snapshot-ignore`,
`ignore-complexity`, `check-script`, `report-ignored` y `diff`. Una función
cuya complejidad es igual al umbral pasa, igual que en la línea de comandos.

## Opciones de CLI

| Opción | Descripción | Predeterminado |
| -- | -- | -- |
| `--exclude` | Excluye patrones glob relativos a cada ruta proporcionada. Usa patrones como `tests/**` para directorios o `src/legacy/file.py` para archivos específicos. |  |
| `--select <ids>` | Reporta solo las reglas de refactorización nombradas, por ejemplo `--select C001,C007`. Se puede repetir y acepta listas separadas por comas | todas las reglas |
| `--ignore <ids>` | Nunca reporta las reglas de refactorización nombradas, por ejemplo `--ignore C007`. Gana sobre `--select`. Se puede repetir y acepta listas separadas por comas |  |
| `--max-complexity-allowed` | Umbral de complejidad | `15` |
| `--snapshot-create` | Guarda las violaciones actuales que superen el umbral en `complexipy-snapshot.json` | `false` |
| `--snapshot-ignore` | Omite la comparación con un snapshot aunque exista | `false` |
| `--failed` | Muestra solo las funciones que superen el umbral de complejidad | `false` |
| `--suggest-refactors` | Muestra planes deterministas de refactorización basados en el AST de Rust en la salida CLI enriquecida. Ignorado por `--plain` | `false` |
| `--fix` | Aplica cada sugerencia aplicable (Seguro de aplicar y empalmable) de las funciones por encima de `max-complexity-allowed` a los archivos fuente y luego informa el estado posterior a las correcciones (ver [Aplicar Sugerencias Aplicables](#aplicar-sugerencias-aplicables)). Implica `--suggest-refactors` | `false` |
| `--dry-run` | Imprime solo el diff unificado de las correcciones que `--fix` aplicaría, y no escribe nada. Implica `--fix` | `false` |
| `--color <auto\|yes\|no>` | Usa color | `auto` |
| `--sort <asc\|desc\|file_name>` | Ordena los resultados | `asc` |
| `--quiet` | Suprime la salida | `false` |
| `--ignore-complexity` | No termina con error al superar el umbral | `false` |
| `--version` | Muestra la versión instalada de complexipy y sale | - |
| `--top <n>` | Muestra solo las `n` funciones más complejas, ordenadas globalmente por complejidad descendente | - |
| `--plain` | Emite líneas de texto plano como `<path> <function> <complexity>`. No se puede combinar con `--quiet` | `false` |
| `--output-format <format>` | Selecciona un formato de salida legible por máquinas. Repite la flag para varios formatos (`json`, `csv`, `gitlab`, `sarif`) | - |
| `--output <path>` | Escribe la salida legible por máquinas en un archivo o directorio. Usa un directorio cuando emitas varios formatos | - |
| `--diff <ref>` | Muestra un diff de complejidad contra una referencia de git y aplica el umbral. Falla si hay regresiones por encima de `--max-complexity-allowed` (ver [Diff de Complejidad](#diff-de-complejidad)) | - |
| `--diff-only <ref>` | Muestra un diff de complejidad visualmente sin afectar el código de salida (ver [Diff de Complejidad](#diff-de-complejidad)) | - |
| `--staged` | Compara los cambios staged (índice de git) contra la referencia de `--diff` (por defecto `HEAD`). Responde "¿qué complejidad estoy a punto de commitear?" (ver [Diff de Complejidad](#diff-de-complejidad)) | `false` |
| `--check-script` | Reporta la complejidad a nivel módulo (script) como una entrada sintética `<module>` | `false` |
| `--no-ignore` | Analiza cada función, ignorando los comentarios de ignore en línea (`# complexipy: ignore`, `# noqa: complexipy`) | `false` |
| `--report-ignored` | Lista cada archivo:línea donde un comentario de ignore suprime una función. `--quiet` oculta la lista | `false` |
| `--isolated` | Ignora todos los archivos de configuración. No se puede combinar con `--config` | `false` |
| `--config <ruta>` | Lee la configuración solo de este archivo, en lugar de los archivos del directorio de invocación | - |

Las banderas booleanas aceptan un valor explícito solo en la forma
adjunta `--flag=<true|false>`. Una bandera sin valor significa `true`,
y el argumento siguiente nunca se consume como su valor.

Ejemplo:

```
# Excluye un directorio recursivamente bajo la raíz proporcionada
complexipy . --exclude "tests/**"
# Excluye un archivo específico
complexipy . --exclude "src/legacy/old_code.py"
```

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

## API de Python

### Analizar Archivos

```python
from complexipy import file_complexity

# Analizar un archivo
result = file_complexity("src/main.py", check_script=True)

print(f"Total complexity: {result.complexity}")
print(f"File path: {result.path}")

# Iterar sobre las funciones
for func in result.functions:
    print(f"{func.name}:")
    print(f"  Complexity: {func.complexity}")
    print(f"  Lines: {func.line_start}-{func.line_end}")

# Analizar sin respetar los comentarios de ignorado en línea
result = file_complexity("src/main.py", no_ignore=True)
```

### Analizar Cadenas de Código

```python
from complexipy import code_complexity

# Analizar fragmento de código
code = """
def calculate_discount(price, customer):
    if customer.is_premium:
        if price > 100:
            return price * 0.8
        else:
            return price * 0.9
    return price
"""

result = code_complexity(code, check_script=True)
print(f"Complexity: {result.complexity}")

for func in result.functions:
    print(f"{func.name}: {func.complexity}")

# Analizar cadena de código sin respetar los comentarios de ignorado
result = code_complexity(code, no_ignore=True)
```

Ambas funciones construyen planes de refactorización por defecto. Pasa
`with_plans=False` cuando solo necesites las puntuaciones: `refactor_plans`
queda vacío y el análisis omite el trabajo de los planes.

```python
result = file_complexity("src/main.py", with_plans=False)
```

### Comparar Contra una Referencia de Git

`compute_diff` compara los resultados de complejidad actuales contra una
referencia de git (commit, etiqueta o rama) y devuelve objetos `DiffEntry` -
uno por función que cambió, apareció o desapareció. `has_regressions` informa
si alguna entrada supera un umbral de complejidad (una función REGRESSED o NEW
por encima de *max_complexity*). Cada archivo se lee del repositorio de git
que lo contiene, y un submódulo en el commit que fija su repositorio
exterior, igual que con `--diff`. `invocation_path`, por defecto
el directorio de trabajo, ancla las rutas de archivo relativas y hace el
papel del directorio donde empieza una ejecución de la CLI.

```python
from complexipy import (
    compute_diff,
    has_regressions,
    file_complexity,
    DiffEntry,
    DiffStatus,
)

# Analizar el estado actual de los archivos de interés
current = [file_complexity(p) for p in changed_files]

# Comparar contra una referencia de git (el directorio de trabajo es el cwd por defecto)
entries = compute_diff(current, "origin/main")

# Filtrar regresiones por encima de tu umbral
regressions = [
    e
    for e in entries
    if e.status == DiffStatus.REGRESSED and e.new_complexity > 15
]

# O usar la compuerta de ratchet directamente (falla con REGRESSED/NEW sobre el umbral)
if has_regressions(entries, 15):
    raise SystemExit("Regresiones de complejidad detectadas")
```

`DiffEntry` expone `file_path`, `func_name`, `old_complexity` y
`new_complexity` (cualquiera puede ser `None` para funciones NEW / REMOVED),
más la propiedad `status`. `status` es un miembro de
`DiffStatus` - un enum basado en `str`, por lo que compara igual a su valor
de cadena (p. ej. `DiffStatus.REGRESSED == "REGRESSED"`).

```python
for e in entries:
    if e.status != DiffStatus.UNCHANGED:
        print(f"{e.file_path}::{e.func_name}: {e.status} "
              f"{e.old_complexity} -> {e.new_complexity}")
```

### Uso Práctico de la API

**Ejemplo: Hook de Pre-commit**

```python
#!/usr/bin/env python3
"""Verifica la complejidad de los archivos Python en staging."""
import sys
from pathlib import Path
from complexipy import file_complexity

MAX_COMPLEXITY = 15

def main():
    # Obtener los archivos Python en staging (integrar con git)
    staged_files = get_staged_python_files()

    violations = []
    for filepath in staged_files:
        result = file_complexity(str(filepath))

        for func in result.functions:
            if func.complexity > MAX_COMPLEXITY:
                violations.append({
                    'file': filepath,
                    'function': func.name,
                    'complexity': func.complexity,
                    'line': func.line_start
                })

    if violations:
        print("Complexity violations found:")
        for v in violations:
            print(f"  {v['file']}:{v['line']} - "
                  f"{v['function']} (complexity: {v['complexity']})")
        sys.exit(1)

    print("All functions pass complexity check!")
    sys.exit(0)

if __name__ == "__main__":
    main()
```

**Ejemplo: Panel de Calidad de Código**

```python
from pathlib import Path
from complexipy import file_complexity
import json

def analyze_project(root_path: str):
    """Genera un informe de complejidad para todo el proyecto."""
    project = Path(root_path)
    results = []

    for py_file in project.rglob("*.py"):
        if "venv" in str(py_file) or ".venv" in str(py_file):
            continue

        try:
            result = file_complexity(str(py_file))
            results.append({
                'file': str(py_file),
                'complexity': result.complexity,
                'functions': [
                    {
                        'name': f.name,
                        'complexity': f.complexity,
                        'line_start': f.line_start,
                        'line_end': f.line_end
                    }
                    for f in result.functions
                ]
            })
        except Exception as e:
            print(f"Error analyzing {py_file}: {e}")

    # Ordenar por complejidad
    results.sort(key=lambda x: x['complexity'], reverse=True)

    # Guardar informe
    with open("complexity-report.json", "w") as f:
        json.dump(results, f, indent=2)

    # Imprimir resumen
    total_files = len(results)
    total_complexity = sum(r['complexity'] for r in results)
    avg_complexity = total_complexity / total_files if total_files else 0

    print(f"Analyzed {total_files} files")
    print(f"Total complexity: {total_complexity}")
    print(f"Average complexity: {avg_complexity:.2f}")

    # Los 10 archivos más complejos
    print("\nTop 10 most complex files:")
    for r in results[:10]:
        print(f"  {r['file']}: {r['complexity']}")

if __name__ == "__main__":
    analyze_project("./src")
```

## Snapshots Base

Los snapshots te permiten adoptar complexipy gradualmente en bases de código grandes y existentes.

### Crear un Snapshot

```bash
complexipy . --snapshot-create --max-complexity-allowed 15
```

Esto crea `complexipy-snapshot.json` en tu directorio de trabajo, registrando todas las funciones que actualmente superan el umbral.

Crea la línea base desde la línea de comandos. `snapshot-create = true` en un
archivo de configuración vuelve a registrar la línea base en cada ejecución,
así que toda violación actual pasa; esa ejecución imprime una advertencia en
stderr que nombra el archivo.

### Cómo Funcionan las snapshots

Una vez que exista un snapshot, complexipy:

- ✅ **Pasa**: Funciones que ya estaban en el snapshot y no han empeorado
- ✅ **Pasa**: Funciones que mejoraron (se eliminan automáticamente del snapshot)
- ❌ **Falla**: Funciones nuevas que superan el umbral
- ❌ **Falla**: Funciones rastreadas que se volvieron más complejas

### Usar snapshots en CI

```yaml
# .github/workflows/complexity.yml
name: Complexity Check

on: [push, pull_request]

jobs:
    check:
        runs-on: ubuntu-latest
        steps:
            - uses: actions/checkout@v4

            - name: Install complexipy
              run: pip install complexipy

            - name: Check complexity
              run: complexipy . --max-complexity-allowed 15
```

El archivo de snapshot (`complexipy-snapshot.json`) debería ser commiteado al control de versiones.

### Ignorar snapshots

Deshabilitar temporalmente la verificación de snapshots:

```bash
complexipy . --snapshot-ignore
```

Úsalo al:

- Refactorizar múltiples archivos a la vez
- Regenerar la línea base
- Probar diferentes umbrales

### Formato del Archivo de snapshot

```json
[
    {
        "path": "src/legacy.py",
        "file_name": "legacy.py",
        "functions": [
            {
                "name": "old_function",
                "complexity": 23
            }
        ]
    }
]
```

Los snapshots se guardan como un arreglo JSON de archivos analizados. Cada entrada contiene solo las funciones por encima del umbral cuando se escribió el snapshot. El archivo se reescribe después de verificaciones de snapshot exitosas, así que las funciones que mejoran se eliminan automáticamente. Las actualizaciones solo afectan a los archivos analizados en la ejecución - las entradas de archivos fuera del análisis se conservan, por lo que ejecutar sobre un subconjunto de archivos (por ejemplo, a través de un hook de pre-commit) nunca reduce la línea base. Es posible que los snapshots creados por versiones anteriores de complexipy deban regenerarse con `--snapshot-create`.

Las funciones de un mismo archivo que comparten nombre, como el getter y el setter de una propiedad, reciben entradas separadas: la primera en orden de código fuente conserva el nombre, y las siguientes se guardan como `name#2`, `name#3`. `--diff` compara con las mismas claves.

## Selección de Reglas

Las reglas de refactorización llevan ids como `C001` (ver
[Reglas de Refactorización](refactoring-rules.md)). Dos flags eligen qué reglas
están activas, y ambas aceptan listas separadas por comas y se pueden repetir:

| Flag | Efecto |
| -- | -- |
| `--select C001,C007` | Reporta solo las reglas nombradas |
| `--ignore C007` | Nunca reporta las reglas nombradas |

Sin ninguna flag, todas las reglas están activas. Cuando ambas flags nombran la
misma regla, `--ignore` gana. Los ids de reglas no distinguen mayúsculas de
minúsculas. Un id desconocido imprime una advertencia en stderr de que no
coincide con ninguna regla registrada. Una lista `select` sigue limitando el
conjunto activo aunque uno de sus ids sea desconocido, y un segundo aviso lo
indica cuando `select` e `ignore` no dejan ninguna regla activa.

Las mismas claves funcionan en `complexipy.toml`, `.complexipy.toml` y
`pyproject.toml`:

```toml
select = ["C001", "C007"]
ignore = ["C002"]
```

Un valor toml también puede ser una sola cadena separada por comas, como las
flags: `select = "C001, C007"`.

Una flag reemplaza la clave toml que nombra. La selección de reglas nunca
cambia el código de salida: el código de salida sigue al umbral de complejidad,
a los snapshots, a las rutas y al gating de diff. Una regla que no está activa
nunca aparece en la terminal, en los planes de refactorización, en JSON, CSV,
SARIF ni en la salida de GitLab.

## Ignorar en Línea

Suprime las advertencias de complejidad para funciones específicas usando el comentario `# complexipy: ignore`:

```python
def complex_legacy_function():  # complexipy: ignore
    # Lógica compleja que no puede ser refactorizada
    pass

# O con un motivo
def another_complex_function():  # complexipy: ignore (technical debt: issue #123)
    pass
```

El comentario de ignorar también puede colocarse en la línea anterior a la definición de la función:

```python
# complexipy: ignore
def complex_function():
    pass
```

Una marca cuenta solo dentro de un comentario: en la línea del `def`, en una
línea de su firma o de sus decoradores, o como comentario en su propia línea
justo encima. El mismo texto en una cadena, un docstring o un argumento de
decorador, o un comentario al final de la línea de código anterior, no suprime
nada. Las palabras se comparan completas y separadas solo por espacios o
tabuladores, así que `# complexipy: ignored` no es una marca.

### Suprimir Reglas Específicas

Añade una lista de reglas entre corchetes para conservar la función pero
ocultar las reglas nombradas:

```python
def tangled(a, b):  # complexipy: ignore[C007]
    pass
```

La función sigue analizándose y reportándose; solo se suprimen las reglas
nombradas para esa función. Separa varios ids con comas:
`# complexipy: ignore[C001,C007]`. La forma `# noqa: complexipy[C007]` se
comporta igual. Un marcador sin lista sigue suprimiendo la función completa.
Un texto entre corchetes que no es una lista de ids de reglas es un motivo, así
que `# complexipy: ignore [technical debt]` sigue suprimiendo la función
completa.

!!! note "Sintaxis Obsoleta"

    La sintaxis `# noqa: complexipy` está obsoleta y será eliminada en una versión futura.
    Por favor, migra a `# complexipy: ignore` en su lugar.

    **¿Por qué?** Herramientas como [yesqa](https://github.com/asottile/yesqa) eliminan automáticamente los comentarios `# noqa`
    que flake8 no reconoce, lo que eliminaría silenciosamente tus supresiones de complexipy.
    La nueva sintaxis evita este conflicto por completo.

!!! warning "Usar con Moderación"

    Los ignorados en línea deben ser temporales. Documenta por qué la complejidad es necesaria y rastrea la deuda técnica.

### Deshabilitar Ignorados en Línea

Usa `--no-ignore` para ignorar todos los comentarios de supresión y analizar cada función:

```bash
complexipy . --no-ignore
```

Las funciones previamente suprimidas por `# complexipy: ignore` o `# noqa: complexipy` se analizarán normalmente y pueden superar el umbral.

### Reportar Funciones Ignoradas

Usa `--report-ignored` para listar cada ubicación donde un comentario de ignore suprime una función:

```bash
# Listar funciones ignoradas
complexipy . --report-ignored

# Combinar con --no-ignore para reportar mientras se analiza todo
complexipy . --report-ignored --no-ignore
```

Cada ubicación se imprime como `ruta:línea  # texto-del-comentario`, seguida del total. Cuando `--output-format json` también está activo, las ubicaciones ignoradas se exportan a `complexipy-ignored.json`. `--quiet` oculta la lista, y la exportación JSON se escribe de todos modos.

Ambos flags también están disponibles en la API de Python mediante `no_ignore=True`:

```python
from complexipy import file_complexity, code_complexity

# Analizar sin respetar los comentarios de ignore
result = file_complexity("app.py", no_ignore=True)
```

Para recolectar ubicaciones ignoradas programáticamente, usa `collect_all_ignored_locations()`:

```python
from complexipy import collect_all_ignored_locations

locations, failed = collect_all_ignored_locations(
    paths=["src"],
    exclude=["tests/**"],
)
for loc in locations:
    print(f"{loc.path}:{loc.line}  {loc.comment}")
```

### Eliminar Comentarios de Ignore Obsoletos

Un comentario de ignore solo es necesario mientras la complejidad de la función
suprimida supere `--max-complexity-allowed`. Cuando la complejidad de la función
baje al límite o por debajo de él, el comentario queda obsoleto y puede
eliminarse. complexipy detecta esto automáticamente en cada ejecución e informa
las ubicaciones que puedes limpiar:

```bash
complexipy .
```

Ejemplo de salida:

```text
The following ignore comment(s) are no longer necessary (complexity is within the allowed limit) and can be removed:
src/legacy.py:42  function=parse_legacy_config complexity=8  # complexipy: ignore
```

El informe es puramente informativo: nunca afecta al código de salida y se
suprime bajo `--quiet`. La misma detección está disponible programáticamente vía
`collect_removable_ignored_locations()`:

```python
from complexipy import collect_removable_ignored_locations

removable, failed = collect_removable_ignored_locations(
    paths=["src"],
    exclude=["tests/**"],
    max_complexity_allowed=15,
)
for rem in removable:
    print(f"{rem.path}:{rem.line}  function={rem.function} complexity={rem.complexity}")
```

## Integración con CI/CD

### GitHub Actions

Usa la acción oficial:

```yaml
- uses: rohaquinlop/complexipy-action@v8
  with:
      paths: src tests
      max_complexity_allowed: 15
      output_json: true
```

O ejecuta directamente:

```yaml
- name: Check complexity
  run: |
      pip install complexipy
      complexipy . --max-complexity-allowed 15
```

### GitHub Code Scanning (SARIF)

Sube violaciones de complejidad como anotaciones inline en PRs usando SARIF:

```yaml
- name: Run complexipy
  run: complexipy . --output-format sarif --output complexipy-results.sarif --ignore-complexity

- name: Upload SARIF results
  uses: github/codeql-action/upload-sarif@v3
  with:
      sarif_file: complexipy-results.sarif
```

### Hook de Pre-commit

Agrega a `.pre-commit-config.yaml`:

```yaml
repos:
    - repo: https://github.com/rohaquinlop/complexipy-pre-commit
      rev: v8.1.0
      hooks:
          - id: complexipy
            args: [--max-complexity-allowed=15]
```

### GitLab CI

```yaml
.complexipy_code_quality:
    image: python:3.11
    script:
        - pip install complexipy
        - complexipy . --output-format gitlab --output complexipy-code-quality.json --ignore-complexity --max-complexity-allowed 15
    artifacts:
        when: always
        reports:
            codequality: complexipy-code-quality.json

complexity:
    extends: .complexipy_code_quality
    rules:
        - if: $CI_PIPELINE_SOURCE == "merge_request_event"
        - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
```

Usa `--ignore-complexity` cuando tu objetivo principal sea publicar el reporte aunque existan violaciones. GitLab seguirá mostrando los hallazgos en el widget del merge request y en la interfaz del pipeline.

Si además quieres que el job falle cuando se supere el umbral, separa el flujo en dos jobs:

```yaml
complexity_check:
    image: python:3.11
    script:
        - pip install complexipy
        - complexipy . --max-complexity-allowed 15

complexity_report:
    image: python:3.11
    script:
        - pip install complexipy
        - complexipy . --output-format gitlab --output complexipy-code-quality.json --ignore-complexity --max-complexity-allowed 15
    artifacts:
        when: always
        reports:
            codequality: complexipy-code-quality.json
```

## Integración con VS Code

Instala la [extensión de complexipy](https://marketplace.visualstudio.com/items?itemName=rohaquinlop.complexipy) desde el Visual Studio Marketplace, o desde [OpenVSX](https://open-vsx.org/extension/rohaquinlop/complexipy). La extensión es un cliente del [servidor de lenguaje](editors.md#vs-code) y necesita el comando `complexipy` instalado.

## Consejos y Mejores Prácticas

### 1. Comienza con Umbrales Altos

Al introducir complexipy en una base de código existente:

```bash
# Crear línea base
complexipy . --snapshot-create --max-complexity-allowed 25

# Reducir gradualmente el umbral con el tiempo
complexipy . --max-complexity-allowed 20
complexipy . --max-complexity-allowed 15
```

### 2. Enfócate en el Código de Alto Tráfico

No todo el código complejo necesita refactorización inmediata:

```bash
# Centrarse en los archivos que cambian con frecuencia
complexipy src/core/ --max-complexity-allowed 10
complexipy src/legacy/ --max-complexity-allowed 25
```

### 3. Usar con Revisiones de Código

```bash
# Verificar solo los archivos en la rama actual
git diff --name-only main | grep '.py$' | xargs complexipy
```

### 4. Combinar con Cobertura de Pruebas

Alta complejidad + baja cobertura = alto riesgo

```bash
# Verificar cobertura para funciones complejas
pytest --cov=src --cov-report=term-missing
complexipy src/ --failed
```

### 5. Rastrear Tendencias

```bash
# Generar datos históricos
complexipy . --output-format json
# Hacer commit de complexipy-results.json para rastrear cambios a lo largo del tiempo
```

## Solución de Problemas

### No se encontraron archivos Python

Asegúrate de estar en el directorio correcto y de que tus archivos tengan extensiones `.py`.

### Errores de sintaxis en archivos analizados

complexipy requiere sintaxis Python válida. Corrige primero los errores de sintaxis:

```bash
python -m py_compile file.py
```

### Problemas de rendimiento en bases de código grandes

Excluir directorios innecesarios:

```bash
complexipy . --exclude "venv/**" --exclude ".venv/**" --exclude "node_modules/**"
```

### Resultados diferentes a los esperados

Verifica la precedencia de los archivos de configuración. Usa `--help` para ver la configuración activa:

```bash
complexipy --help
```

## Próximos Pasos

- Lee [Entendiendo las Puntuaciones de Complejidad](understanding-scores.md) para interpretar los resultados
- Consulta [Comparación con Ruff](comparison-with-ruff.md) para herramientas complementarias
- Visita [Acerca de complexipy](about.md) para obtener más información sobre el proyecto
