# complexipy

<div align="center">
  <img src="img/complexipy_icon.svg" alt="complexipy" width="120" height="120">

  <p><em>Análisis ultrarrápido de complejidad cognitiva para Python, escrito en Rust.</em></p>

  <p>
    <a href="https://pypi.org/project/complexipy"><img src="https://img.shields.io/pypi/v/complexipy?color=blue&style=flat-square" alt="PyPI"></a>
    <a href="https://pepy.tech/project/complexipy"><img src="https://static.pepy.tech/badge/complexipy" alt="Downloads"></a>
    <a href="https://github.com/rohaquinlop/complexipy/blob/main/LICENSE"><img src="https://img.shields.io/github/license/rohaquinlop/complexipy?style=flat-square" alt="License"></a>
  </p>

  <p>
    <a href="#instalación">Instalación</a> •
    <a href="#inicio-rápido">Inicio Rápido</a> •
    <a href="#integraciones">Integraciones</a> •
    <a href="https://rohaquinlop.github.io/complexipy/">Documentación</a>
  </p>
</div>

## ¿Qué es la Complejidad Cognitiva?

> La complejidad cognitiva mide qué tan difícil es entender el código para los seres humanos, no para las máquinas.

A diferencia de métricas tradicionales como la complejidad ciclomática, la complejidad cognitiva tiene en cuenta la profundidad de anidamiento y los patrones de flujo de control que afectan la comprensión humana. Inspirado en la [investigación de G. Ann Campbell](https://www.sonarsource.com/resources/cognitive-complexity/) en SonarSource, complexipy ofrece una implementación rápida y precisa para Python.

**Beneficios clave:**

- **Orientado al ser humano** - Penaliza el anidamiento, las interrupciones de flujo y la lógica difícil de comprender
- **Perspectivas accionables** - Identifica código genuinamente difícil de mantener
- **Diferente a la ciclomática** - Mide la legibilidad mientras que la ciclomática mide la densidad estructural, de pruebas y de ramas

## Preguntas Frecuentes

**[¿Cómo se calcula la complejidad?](understanding-scores.md)**
Aprende sobre el algoritmo de puntuación, qué aporta cada estructura de control y cómo afecta el anidamiento a la puntuación final.

**[¿Cómo se compara esto con PLR0912 de Ruff?](comparison-with-ruff.md)**
Comprende las diferencias clave entre la complejidad ciclomática (Ruff) y la complejidad cognitiva (complexipy), y por qué podrías querer usar ambas.

**[¿Es esto un producto de SonarSource/Sonar?](about.md)**
No. complexipy es un proyecto independiente inspirado en la investigación de G. Ann Campbell, pero no está afiliado ni respaldado por SonarSource.

## Instalación

```bash
pip install complexipy
# o
uv add complexipy
```

## Inicio Rápido

### Línea de Comandos

```bash
# Analiza el directorio actual
complexipy .

# Analiza un directorio o archivo específico
complexipy path/to/code.py

# Analisis con un límite (threshold) personalizado
complexipy . --max-complexity-allowed 10

# Guarda los resultados en JSON/CSV
complexipy . --output-format json --output-format csv

# Muestra las 5 funciones más complejas
complexipy . --top 5

# Emite texto plano para scripts o agentes de IA
complexipy . --plain

# Incluye la complejidad del código a nivel de módulo como <module>
complexipy . --check-script

# Guarda un reporte de GitLab en una ruta determinista
complexipy . --output-format gitlab --output complexipy-code-quality.json

# Compara la complejidad contra una referencia de git
complexipy . --diff HEAD~1

# Incluye la complejidad del script a nivel módulo como <module>
complexipy . --check-script

# Analiza el directorio actual excluyendo ciertos archivos
complexipy . --exclude path/to/exclude.py --exclude path/to/other/exclude.py
```

### API de Python

```python
from complexipy import file_complexity, code_complexity

# Analiza un archivo
result = file_complexity("app.py", check_script=True)
print(f"File complexity: {result.complexity}")

for func in result.functions:
    print(f"{func.name}: {func.complexity}")

# Analiza código en una cadena
snippet = """
def complex_function(data):
    if data:
        for item in data:
            if item.is_valid():
                process(item)
"""

result = code_complexity(snippet, check_script=True)
print(f"Complexity: {result.complexity}")
```

## Integraciones

<details>
<summary><strong>🔧 GitHub Actions</strong></summary>

```yaml
- uses: rohaquinlop/complexipy-action@v2
  with:
      paths: .
      max_complexity_allowed: 10
      output_format: json
```

</details>

<details>
<summary><strong>🪝 Pre-commit Hook</strong></summary>

```yaml
repos:
    - repo: https://github.com/rohaquinlop/complexipy-pre-commit
      rev: v4.2.0
      hooks:
          - id: complexipy
```

</details>

<details>
<summary><strong>🔌 VS Code Extension</strong></summary>

Instálala desde el [marketplace](https://marketplace.visualstudio.com/items?itemName=rohaquinlop.complexipy) para tener análisis de complejidad en tiempo real con indicadores visuales.

</details>

## Configuración

### Archivos de Configuración TOML

complexipy admite configuración mediante archivos TOML. Los archivos de configuración se cargan en este orden de precedencia:

1. `complexipy.toml` (configuración específica del proyecto)
2. `.complexipy.toml` (archivo de configuración oculto)
3. `pyproject.toml` (bajo la sección `[tool.complexipy]`)

#### Configuración de Ejemplo

```toml
# complexipy.toml o .complexipy.toml
paths = ["src", "tests"]
max-complexity-allowed = 10
snapshot-create = false
snapshot-ignore = false
quiet = false
ignore-complexity = false
failed = false
color = "auto"
sort = "asc"
exclude = []
check-script = false
output-format = ["json", "gitlab"]
output = "reports/"
check-script = false
```

```toml
# pyproject.toml
[tool.complexipy]
paths = ["src", "tests"]
max-complexity-allowed = 10
snapshot-create = false
snapshot-ignore = false
quiet = false
ignore-complexity = false
failed = false
color = "auto"
sort = "asc"
exclude = []
check-script = false
output-format = ["json"]
output = "complexipy-results.json"
check-script = false
```

Las claves TOML heredadas como `output-json = true` y las flags de CLI como
`--output-json` todavía funcionan por ahora, pero están deprecadas a favor de
`output-format` y `--output-format`.

`check-script` está soportado en TOML. `--top` y `--plain` son flags solo de CLI.

### Opciones de CLI

| Opción                          | Descripción                                                                                                                                                                                                 | Predeterminado |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------- |
| `--exclude`                     | Excluye entradas relativas a cada ruta proporcionada. Las entradas se resuelven a directorios existentes (coincidencia por prefijo) o archivos (coincidencia exacta). Las entradas inexistentes se ignoran. |                |
| `--max-complexity-allowed`      | Umbral de complejidad                                                                                                                                                                                       | `15`           |
| `--snapshot-create`             | Guarda las violaciones actuales que superen el umbral en `complexipy-snapshot.json`                                                                                                                         | `false`        |
| `--snapshot-ignore`             | Omite la comparación con un snapshot aunque exista                                                                                                                                                          | `false`        |
| `--failed`                      | Muestra solo las funciones que superen el umbral de complejidad                                                                                                                                             | `false`        |
| `--color <auto\|yes\|no>`       | Usa color                                                                                                                                                                                                   | `auto`         |
| `--sort <asc\|desc\|file_name>` | Ordena los resultados                                                                                                                                                                                   | `asc`          |
| `--quiet`                       | Suprime la salida                                                                                                                                                                                           | `false`        |
| `--ignore-complexity`           | No termina con error al superar el umbral                                                                                                                                                                   | `false`        |
| `--version`                     | Muestra la versión instalada de complexipy y sale                                                                                                                                                           | -              |
| `--top <n>`                     | Muestra solo las `n` funciones más complejas, ordenadas globalmente por complejidad descendente                                                                                                             | —              |
| `--plain`                       | Emite líneas de texto plano como `<path> <function> <complexity>`. No se puede combinar con `--quiet`                                                                                                      | `false`        |
| `--output-format <format>`      | Selecciona un formato de salida legible por máquinas. Repite la flag para varios formatos (`json`, `csv`, `gitlab`, `sarif`)                                                                                | —              |
| `--output <path>`               | Escribe la salida legible por máquinas en un archivo o directorio. Usa un directorio cuando emitas varios formatos                                                                                          | —              |
| `--diff <ref>`                  | Muestra un diff de complejidad contra una referencia de git (por ejemplo, `HEAD~1`, `main`)                                                                                                                 | —              |
| `--check-script`                | Reporta la complejidad a nivel módulo (script) como una entrada sintética `<module>`                                                                                                                        | `false`        |
| `--output-json`                 | Alias deprecado de `--output-format json`                                                                                                                                                                   | `false`        |
| `--output-csv`                  | Alias deprecado de `--output-format csv`                                                                                                                                                                    | `false`        |
| `--output-gitlab`               | Alias deprecado de `--output-format gitlab`                                                                                                                                                                 | `false`        |
| `--output-sarif`                | Alias deprecado de `--output-format sarif`            

Ejemplo:

```
# Excluye solo el directorio 'tests' de nivel superior bajo la raíz proporcionada
complexipy . --exclude tests
# Esto no excluirá './complexipy/utils.py' si pasas '--exclude utils' en la raíz del repositorio,
# porque no hay ningún directorio o archivo './utils' en ese nivel.
```

### Snapshots Base

Usa snapshots para adoptar complexipy gradualmente en bases de código grandes y existentes sin tocar cada función heredada de una vez.

```bash
# Registra el estado actual (crea complexipy-snapshot.json en el directorio de trabajo)
complexipy . --snapshot-create --max-complexity-allowed 15

# Bloquea regresiones mientras permite las funciones previamente registradas
complexipy . --max-complexity-allowed 15

# Omite temporalmente el control de snapshots
complexipy . --snapshot-ignore
```

El archivo de snapshot solo almacena las funciones cuya complejidad supera el umbral configurado. Cuando existe un archivo de snapshot, complexipy automáticamente:

- falla si una nueva función supera el umbral,
- falla si una función registrada se vuelve más compleja, y
- pasa (y actualiza la snapshot) cuando todo es estable o ha mejorado, eliminando automáticamente las entradas que ahora cumplen el estándar.

Usa `--snapshot-ignore` si necesitas omitir temporalmente el control de snapshot (por ejemplo, durante una refactorización o al regenerar la línea base).

### Diff de Complejidad

Compara la complejidad contra cualquier referencia de git para ver si una rama o commit mejoró o empeoró el código:

```bash
# Compara el working tree con el commit anterior
complexipy . --diff HEAD~1

# Compara contra una rama nombrada
complexipy . --diff main
```

El diff se añade después de la salida normal del análisis y no afecta el código de salida. Requiere que `git` esté disponible y que las rutas analizadas estén dentro de un repositorio git.

### Complejidad de Script

Usa `--check-script` cuando también quieras medir el flujo de control a nivel de módulo, no solo las funciones:

```bash
complexipy scripts/bootstrap.py --check-script
```

La misma capacidad está disponible en la API de Python mediante `check_script=True` tanto en `file_complexity()` como en `code_complexity()`.

### Ignorar en Línea

Puedes ignorar explícitamente una función compleja conocida en línea usando `# complexipy: ignore`:

```python
def legacy_adapter(x, y):  # complexipy: ignore (safe wrapper)
    if x and y:
        return x + y
    return 0
```

Coloca `# complexipy: ignore` en la línea de definición de la función (o en la línea inmediatamente anterior). Se puede proporcionar un motivo opcional entre paréntesis; el analizador lo ignora.

> **Nota:** La sintaxis `# noqa: complexipy` está obsoleta. Herramientas como [yesqa](https://github.com/asottile/yesqa) eliminan los comentarios `# noqa` no reconocidos, lo que eliminaría silenciosamente tus supresiones. Migra a `# complexipy: ignore` para evitar este problema.

## Referencia de la API

```python
# Funciones principales
file_complexity(path: str, check_script: bool = False) -> FileComplexity
code_complexity(source: str, check_script: bool = False) -> CodeComplexity

# Tipos de retorno
FileComplexity:
  ├─ path: str
  ├─ complexity: int
  └─ functions: List[FunctionComplexity]

FunctionComplexity:
  ├─ name: str
  ├─ complexity: int
  ├─ line_start: int
  └─ line_end: int
```

---

<div align="center">

<sub>Inspirado en la investigación de <a href="https://www.sonarsource.com/resources/cognitive-complexity/">Complejidad Cognitiva</a> de G. Ann Campbell</sub><br>
<sub>complexipy es un proyecto independiente y no está afiliado ni respaldado por SonarSource</sub>

**[Documentación](https://rohaquinlop.github.io/complexipy/) • [PyPI](https://pypi.org/project/complexipy/) • [GitHub](https://github.com/rohaquinlop/complexipy)**

<sub>Desarrollado con ❤️ por <a href="https://github.com/rohaquinlop">@rohaquinlop</a> y <a href="https://github.com/rohaquinlop/complexipy/graphs/contributors">colaboradores</a></sub>

</div>
