# Guía de Migración

Esta página registra las flags y claves eliminadas y sus reemplazos.
Consúltala antes de actualizar entre versiones mayores.

## Eliminado en la siguiente versión mayor

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
