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

=== "poetry"
    ```bash
    poetry add complexipy
    ```

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

Suprimir toda la salida (útil para pipelines de CI):

```bash
complexipy . --quiet
```

### Ordenar Resultados

Ordenar por puntuación de complejidad:

```bash
complexipy . --sort asc   # Ascendente (predeterminado)
complexipy . --sort desc  # Descendente
complexipy . --sort name  # Alfabéticamente por nombre de función
```

### Excluir Archivos y Directorios

Excluir rutas específicas del análisis:

```bash
# Excluir un directorio
complexipy . --exclude tests

# Excluir múltiples rutas
complexipy . --exclude tests --exclude migrations --exclude build

# Excluir archivos específicos
complexipy . --exclude src/legacy/old_code.py
```

!!! note "Cómo funciona la exclusión"
    - Las entradas se resuelven a directorios existentes (coincidencia por prefijo) o archivos (coincidencia exacta)
    - Las entradas inexistentes se ignoran silenciosamente
    - Las rutas son relativas a cada ruta raíz proporcionada

### Formatos de Salida

Guarda los resultados en JSON o CSV:

```bash
# Salida JSON (guardada en complexipy-results.json)
complexipy . --output-json

# Salida CSV (guardada en complexipy-results.csv)
complexipy . --output-csv

# Ambos
complexipy . --output-json --output-csv
```

**Estructura de Salida JSON:**
```json
{
  "files": [
    {
      "path": "src/main.py",
      "complexity": 42,
      "functions": [
        {
          "name": "process_data",
          "complexity": 18,
          "line_start": 10,
          "line_end": 45
        }
      ]
    }
  ],
  "total_complexity": 42
}
```

### Salida en Color

Controla la salida en color:

```bash
complexipy . --color auto  # Predeterminado: detecta automáticamente la compatibilidad con la terminal
complexipy . --color yes   # Forza colores
complexipy . --color no    # Deshabilita colores
```

## Archivos de Configuración

### Prioridad de Configuración

complexipy carga la configuración en este orden (de mayor a menor prioridad):

1. Argumentos de línea de comandos
2. `complexipy.toml`
3. `.complexipy.toml`
4. `pyproject.toml` (bajo `[tool.complexipy]`)

### Configuraciones de Ejemplo

=== "complexipy.toml"
    ```toml
    paths = ["src", "tests"]
    max-complexity-allowed = 10
    exclude = ["migrations", "build"]
    snapshot-create = false
    snapshot-ignore = false
    quiet = false
    ignore-complexity = false
    failed = false
    color = "auto"
    sort = "asc"
    output-csv = false
    output-json = false
    ```

=== "pyproject.toml"
    ```toml
    [tool.complexipy]
    paths = ["src", "tests"]
    max-complexity-allowed = 10
    exclude = ["migrations", "build"]
    failed = true
    sort = "desc"
    ```

=== ".complexipy.toml"
    ```toml
    # Archivo de configuración oculto para ajustes específicos del equipo
    max-complexity-allowed = 15
    exclude = ["venv", ".venv", "node_modules"]
    ```

## API de Python

### Analizar Archivos

```python
from complexipy import file_complexity

# Analizar un archivo
result = file_complexity("src/main.py")

print(f"Total complexity: {result.complexity}")
print(f"File path: {result.path}")

# Iterar sobre las funciones
for func in result.functions:
    print(f"{func.name}:")
    print(f"  Complexity: {func.complexity}")
    print(f"  Lines: {func.line_start}-{func.line_end}")
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

result = code_complexity(code)
print(f"Complexity: {result.complexity}")

for func in result.functions:
    print(f"{func.name}: {func.complexity}")
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
{
  "version": "1.0",
  "threshold": 15,
  "functions": {
    "src/legacy.py::old_function": {
      "complexity": 23,
      "line_start": 10,
      "line_end": 50
    }
  }
}
```

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

!!! note "Sintaxis Obsoleta"
    La sintaxis `# noqa: complexipy` está obsoleta y será eliminada en una versión futura.
    Por favor, migra a `# complexipy: ignore` en su lugar.

    **¿Por qué?** Herramientas como [yesqa](https://github.com/asottile/yesqa) eliminan automáticamente los comentarios `# noqa`
    que flake8 no reconoce, lo que eliminaría silenciosamente tus supresiones de complexipy.
    La nueva sintaxis evita este conflicto por completo.

!!! warning "Usar con Moderación"
    Los ignorados en línea deben ser temporales. Documenta por qué la complejidad es necesaria y rastrea la deuda técnica.

## Integración con CI/CD

### GitHub Actions

Usa la acción oficial:

```yaml
- uses: rohaquinlop/complexipy-action@v2
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

### Hook de Pre-commit

Agrega a `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/rohaquinlop/complexipy-pre-commit
    rev: v4.2.0
    hooks:
      - id: complexipy
        args: [--max-complexity-allowed=15]
```

### GitLab CI

```yaml
complexity:
  image: python:3.11
  script:
    - pip install complexipy
    - complexipy . --max-complexity-allowed 15
  only:
    - merge_requests
    - main
```

## Integración con VS Code

Instala la [extensión de complexipy](https://marketplace.visualstudio.com/items?itemName=rohaquinlop.complexipy) para análisis de complejidad en tiempo real:

- Puntuaciones de complejidad en línea
- Tooltips al pasar el cursor con detalles
- Indicadores con código de color
- Sugerencias de corrección rápida

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
complexipy . --output-json
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
complexipy . --exclude venv --exclude .venv --exclude node_modules
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
