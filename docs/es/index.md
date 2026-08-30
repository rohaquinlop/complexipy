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
    <a href="#aprende-más">Aprende Más</a> •
    <a href="https://www.complexipy-teams.com/">Complexipy Teams</a>
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

# Establece un umbral personalizado
complexipy . --max-complexity-allowed 10

# Muestra las funciones fallidas con sugerencias de refactorización
complexipy . --failed --suggest-refactors

# Guarda los resultados en JSON
complexipy . --output-format json

# Bloquea regresiones contra una referencia de git
complexipy . --diff main

# Excluye rutas con patrones glob
complexipy . --exclude "tests/**"
```

### API de Python

```python
from complexipy import file_complexity

# Analiza un archivo
result = file_complexity("app.py", check_script=True)
print(f"File complexity: {result.complexity}")

for func in result.functions:
    print(f"{func.name}: {func.complexity}")
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
<summary><strong>🪝 Hook de Pre-commit</strong></summary>

```yaml
repos:
    - repo: https://github.com/rohaquinlop/complexipy-pre-commit
      rev: v5.1.0
      hooks:
          - id: complexipy
```

</details>

<details>
<summary><strong>🔌 Extensión de VS Code</strong></summary>

Instálala desde el [marketplace](https://marketplace.visualstudio.com/items?itemName=rohaquinlop.complexipy) para tener análisis de complejidad en tiempo real con indicadores visuales.

</details>

## Aprende Más

- [Guía de Uso](usage-guide.md) - todas las flags de CLI, archivos de configuración, snapshots, diff de complejidad e ignores en línea
- [Referencia de la API](api-reference.md) - la API completa de Python
- [Qué Significan las Puntuaciones](understanding-scores.md) - cómo funciona el algoritmo de puntuación
- [Comparación con Ruff](comparison-with-ruff.md) - complejidad cognitiva vs ciclomática
- [Reglas de Refactorización](refactoring-rules.md) - las reglas detrás de `--suggest-refactors`
- [Registro de Cambios](changelog.md) - qué cambió en cada versión

______________________________________________________________________

<div align="center">

<p style="margin: 0.25rem 0"><sub>Inspirado en la investigación de <a href="https://www.sonarsource.com/resources/cognitive-complexity/">Complejidad Cognitiva</a> de G. Ann Campbell</sub></p>
<p style="margin: 0.25rem 0"><sub>complexipy es un proyecto independiente y no está afiliado ni respaldado por SonarSource</sub></p>
<p style="margin: 0.25rem 0"><strong><a href="https://rohaquinlop.github.io/complexipy/">Documentación</a> • <a href="https://pypi.org/project/complexipy/">PyPI</a> • <a href="https://github.com/rohaquinlop/complexipy">GitHub</a></strong></p>
<p style="margin: 0.25rem 0"><sub>Desarrollado con ❤️ por <a href="https://github.com/rohaquinlop">@rohaquinlop</a> y <a href="https://github.com/rohaquinlop/complexipy/graphs/contributors">colaboradores</a></sub></p>

</div>
