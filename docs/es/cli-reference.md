# Opciones de CLI

| Opción | Descripción | Predeterminado |
| -- | -- | -- |
| `--exclude` | Excluye patrones glob relativos a cada ruta proporcionada. Usa patrones como `tests/**` para directorios o `src/legacy/file.py` para archivos específicos. |  |
| `--select <ids>` | Reporta solo las reglas de refactorización o los grupos de reglas nombrados, por ejemplo `--select C001,C007` o `--select complexity`. Se puede repetir y acepta listas separadas por comas | todas las reglas |
| `--ignore <ids>` | Nunca reporta las reglas de refactorización o los grupos de reglas nombrados, por ejemplo `--ignore C007` o `--ignore readability`. Gana sobre `--select`. Se puede repetir y acepta listas separadas por comas |  |
| `--max-complexity-allowed` | Umbral de complejidad | `15` |
| `--snapshot-create` | Guarda las violaciones actuales que superen el umbral en `complexipy-snapshot.json` | `false` |
| `--snapshot-ignore` | Omite la comparación con un snapshot aunque exista | `false` |
| `--failed` | Muestra solo las funciones que superen el umbral de complejidad | `false` |
| `--suggest-refactors` | Muestra planes deterministas de refactorización basados en el AST de Rust en la salida CLI enriquecida. Ignorado por `--plain` | `false` |
| `--fix` | Aplica cada sugerencia aplicable (Seguro de aplicar y empalmable) de las funciones por encima de `max-complexity-allowed` a los archivos fuente y luego informa el estado posterior a las correcciones (ver [Aplicar Sugerencias Aplicables](refactoring-rules.md#aplicar-sugerencias-aplicables)). Implica `--suggest-refactors` | `false` |
| `--dry-run` | Imprime solo el diff unificado de las correcciones que `--fix` aplicaría, y no escribe nada. Implica `--fix` | `false` |
| `--color <auto\|yes\|no>` | Usa color | `auto` |
| `--sort <asc\|desc\|file_name>` | Ordena los resultados | `asc` |
| `--quiet` | Suprime la salida | `false` |
| `--ignore-complexity` | No termina con error al superar el umbral | `false` |
| `--version` | Muestra la versión instalada de complexipy y sale | - |
| `--top <n>` | Muestra solo las `n` funciones más complejas, ordenadas globalmente por complejidad descendente | - |
| `--plain` | Emite líneas de texto plano como `<path> <function> <complexity>`. No se puede combinar con `--quiet` | `false` |
| `--output-format <format>` | Selecciona un formato de salida legible por máquinas. Repite la flag para varios formatos (`json`, `csv`, `gitlab`, `sarif`) | - |
| `--output <path>` | Escribe la salida legible por máquinas en un archivo o directorio. Usa un directorio cuando emitas varios formatos. Usa `-` para escribir un solo formato en stdout: el reporte de consola no se imprime, y las advertencias y los errores van a stderr | - |
| `--diff <ref>` | Muestra un diff de complejidad contra una referencia de git y aplica el umbral. Falla si hay regresiones por encima de `--max-complexity-allowed` (ver [Diff de Complejidad](diff-and-snapshots.md#diff-de-complejidad)) | - |
| `--diff-only <ref>` | Muestra un diff de complejidad visualmente sin afectar el código de salida (ver [Diff de Complejidad](diff-and-snapshots.md#diff-de-complejidad)) | - |
| `--staged` | Compara los cambios staged (índice de git) contra la referencia de `--diff` (por defecto `HEAD`). Responde "¿qué complejidad estoy a punto de commitear?" (ver [Diff de Complejidad](diff-and-snapshots.md#diff-de-complejidad)) | `false` |
| `--check-script` | Reporta la complejidad a nivel módulo (script) como una entrada sintética `<module>` | `false` |
| `--no-ignore` | Analiza cada función, ignorando los comentarios de ignore en línea (`# complexipy: ignore`, `# noqa: complexipy`) | `false` |
| `--report-ignored` | Lista cada archivo:línea donde un comentario de ignore suprime una función. `--quiet` oculta la lista | `false` |
| `--force-exclude` | Aplica `exclude` a los archivos pasados explícitamente como argumentos, como los nombres de archivo que pasa pre-commit | `false` |
| `--isolated` | Ignora todos los archivos de configuración. No se puede combinar con `--config` | `false` |
| `--config <ruta>` | Lee la configuración solo de este archivo, en lugar de los archivos del directorio de invocación | - |
| `--explain <regla>` | Imprime el id, el nombre, la categoría, la aplicabilidad, la descripción y el enlace a la documentación de una regla de refactorización, y termina sin analizar nada. Un id desconocido termina con un error que lista los ids válidos | - |

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
