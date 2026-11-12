# Guía de Migración

Esta página lista, para cada versión mayor, los cambios que pueden requerir
acción al actualizar: las flags y claves eliminadas con sus reemplazos, y los
cambios en las puntuaciones y la salida. Consúltala antes de actualizar entre
versiones mayores.

## Eliminado en 7.0.0

Las siguientes flags de CLI y claves TOML se eliminaron. Usa sus reemplazos:

| Flag/clave eliminada | Reemplazo |
| -- | -- |
| `--output-json` / `-j` | `--output-format json` |
| `--output-csv` / `-c` | `--output-format csv` |
| `--output-gitlab` | `--output-format gitlab` |
| `--output-sarif` / `-sr` | `--output-format sarif` |
| `--ratchet` / `-R` | `--diff <ref>` (aplica por defecto) |
| `output-json = true` | `output-format = ["json"]` |
| `output-csv = true` | `output-format = ["csv"]` |
| `output-gitlab = true` | `output-format = ["gitlab"]` |
| `output-sarif = true` | `output-format = ["sarif"]` |
| `ratchet = true` | `[tool.complexipy.diff] branch` o `--diff <ref>` |
| `staged = true` | `[tool.complexipy.diff] staged = true` |
| `details = "low"` | `failed = true` |

## Qué hace cada reemplazo

- `--output-format <formato>` selecciona el formato de salida legible por
  máquinas (`json`, `csv`, `gitlab`, `sarif`). Repite la flag para varios
  formatos.
- `--diff <ref>` muestra un diff de complejidad contra una referencia de
  git y aplica el umbral por defecto.
- `[tool.complexipy.diff] branch` establece la referencia por defecto del
  diff en TOML; `[tool.complexipy.diff] staged = true` habilita la
  comparación staged por defecto.
- `failed = true` muestra solo las funciones por encima del umbral de
  complejidad.

## Eliminado en 8.0.0

La CLI se reescribió en Rust. Las siguientes características de
compatibilidad con versiones antiguas se eliminaron:

- **Análisis por URL de Git** (`complexipy <repository-url>`) - solo rutas
  locales.
- **Migración de caché heredada** - el diseño anterior
  `.complexipy_cache/<hash>.json` ya no se migra; los archivos heredados
  existentes se ignoran y el historial de diferencias comienza vacío.
- **Archivos de snapshot heredados** - los snapshots creados con versiones
  antiguas ya no se detectan; vuelve a crearlos con `--snapshot-create`.
- **Flag corta `-mx`** - usa `--max-complexity-allowed` en su lugar.

## Cambios en 9.0.0

No se eliminó ninguna flag ni clave. Cambian las puntuaciones y parte de la
salida:

- **Puntuaciones** - esta versión aplica más reglas del paper de complejidad
  cognitiva: comprensiones, `else` en bucles y `try`, operadores booleanos
  mezclados, ciclos de recursión, definiciones anidadas y expresiones
  condicionales en más lugares. Sobre el código fuente de django, cambia la
  puntuación de 1.946 de 30.813 funciones: 1.481 bajan y 465 suben, y el
  total de todos los archivos baja un 7%. Vuelve a ejecutar `complexipy` y
  revisa `max-complexity-allowed`. Si usas un snapshot, vuelve a crearlo con
  `--snapshot-create`, porque una función cuya puntuación subió falla la
  comprobación. El [registro de cambios](changelog.md) lista cada regla.
- **Salida de `--plain`** - cada línea es ahora
  `<path>:<line> <function> <complexity>`. Un script que lee el primer campo
  como la ruta debe quitar el sufijo `:<line>`.
- **Advertencias** - cada advertencia de la CLI se imprime en stderr,
  incluidas las de `--diff` y `--staged`, y `--quiet` ya no las oculta. Un
  script que lee estas advertencias desde stdout debe leer stderr.
- **Helper de C005** - el predicado extraído ya no lleva la anotación
  `-> bool`.
