# Referencia de la API

La API completa de Python que expone complexipy. Importa todo desde el paquete
`complexipy`. Para ejemplos de uso, consulta la sección
[API de Python](#api-de-python) de la guía de uso.

```text
# Funciones principales
file_complexity(path: str, check_script: bool = False, no_ignore: bool = False, with_plans: bool = True) -> FileComplexity
code_complexity(source: str, check_script: bool = False, no_ignore: bool = False, with_plans: bool = True) -> CodeComplexity
collect_all_ignored_locations(paths: List[str], exclude: List[str], invocation_path: str = ".") -> Tuple[List[IgnoredLocation], List[str]]
collect_removable_ignored_locations(paths: List[str], exclude: List[str], max_complexity_allowed: int, invocation_path: str = ".") -> Tuple[List[RemovableIgnore], List[str]]
compute_diff(current_files: List[FileComplexity], git_ref: str, invocation_path: Optional[str] = None) -> List[DiffEntry]
has_regressions(entries: List[DiffEntry], max_complexity: int) -> bool

# Tipos de retorno
FileComplexity:
  ├─ path: str (relative to the working directory, or absolute when outside it)
  ├─ file_name: str
  ├─ complexity: int
  └─ functions: List[FunctionComplexity]

FunctionComplexity:
  ├─ name: str
  ├─ complexity: int
  ├─ line_start: int
  ├─ line_end: int
  ├─ line_complexities: List[LineComplexity]
  └─ refactor_plans: List[RefactorPlan]

RefactorPlan:
  ├─ kind: str
  ├─ title: str
  ├─ line_start: int
  ├─ line_end: int
  ├─ column_start: int
  ├─ current_complexity: int
  ├─ estimated_reduction: int
  ├─ estimated_complexity_after: int
  ├─ rule_id: str
  ├─ category: RuleCategory
  ├─ applicability: Applicability
  ├─ description: str
  ├─ explanation: str
  ├─ references: List[str]
  ├─ suggestion: Optional[CodeSuggestion]
  ├─ help: Optional[str]
  └─ doc_url: str

CodeSuggestion:
  ├─ replacement: str
  ├─ applicability: Applicability
  ├─ description: str
  └─ insertions: List[CodeInsertion]

CodeInsertion:
  ├─ before_line: int
  └─ text: str

RuleCategory: Complexity | Readability
Applicability: MachineApplicable | MaybeIncorrect | Informational

LineComplexity:
  ├─ line: int
  └─ complexity: int

IgnoredLocation:
  ├─ path: str
  ├─ line: int
  └─ comment: str

CodeComplexity:
  ├─ complexity: int
  └─ functions: List[FunctionComplexity]

DiffEntry:
  ├─ file_path: str
  ├─ func_name: str
  ├─ old_complexity: Optional[int]
  ├─ new_complexity: Optional[int]
  └─ status: DiffStatus (property)

DiffStatus: REGRESSED | IMPROVED | UNCHANGED | NEW | REMOVED
```

## Errores

`file_complexity` lanza `ValueError` cuando el archivo no se puede leer o decodificar como UTF-8, o no se puede analizar sintácticamente. `code_complexity` lanza `ValueError` cuando el código no se puede analizar sintácticamente. Un error interno en cualquier función de análisis también aparece como `ValueError`, así que un solo `except ValueError` omite un archivo que no se puede analizar.

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
por encima de *max_complexity*). Las entradas no llevan los cuerpos de las
funciones, así que una función movida o renombrada cuenta como NEW; solo
`--diff`, que compara los cuerpos, deja pasar una copia exacta. Cada archivo se
lee del repositorio de git que lo contiene, en su propia ruta, y un submódulo
en el commit que fija su repositorio exterior, igual que con `--diff`, así que
un archivo movido desde la referencia reporta sus funciones como NEW: su ruta
anterior no está entre los archivos que pasas. Igual que `--diff`, necesita Git
2.30 o posterior en tu `PATH`. `invocation_path`, por defecto
el directorio de trabajo, ancla las rutas de archivo relativas y hace el papel
del directorio donde empieza una ejecución de la CLI.

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
