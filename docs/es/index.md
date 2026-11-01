# complexipy

<div align="center">
  <img src="img/complexipy_icon.svg" alt="complexipy" width="120" height="120">

<p><em>Análisis ultrarrápido de complejidad cognitiva para Python, escrito en Rust.</em></p>

<p>Puntúa los 2.929 archivos de Django en unos 0,45 s en un Apple M1 Pro. <a href="benchmarks/">Puntos de referencia</a></p>

<p>
    <a href="https://pypi.org/project/complexipy"><img src="https://img.shields.io/pypi/v/complexipy?color=blue&style=flat-square" alt="PyPI"></a>
    <a href="https://pepy.tech/project/complexipy"><img src="https://static.pepy.tech/badge/complexipy" alt="Downloads"></a>
    <a href="https://github.com/rohaquinlop/complexipy/blob/main/LICENSE"><img src="https://img.shields.io/github/license/rohaquinlop/complexipy?style=flat-square" alt="License"></a>
  </p>

<p>
    <a href="#instalación">Instalación</a> •
    <a href="#inicio-rápido">Inicio Rápido</a> •
    <a href="#funcionalidades">Funcionalidades</a> •
    <a href="#integraciones">Integraciones</a> •
    <a href="#aprende-más">Aprende Más</a> •
    <a href="https://www.complexipy-teams.com/">Complexipy Teams</a>
  </p>
</div>

<p align="center">
  <img src="img/demo.gif" alt="complexipy puntúa cada función de checkout.py, marca apply_discount como fallida con 14 frente a un límite de 10 y sugiere extraer una función auxiliar">
</p>

## Instalación

=== "uv tool"

    ```bash
    uv tool install complexipy
    ```

=== "uv add"

    ```bash
    uv add complexipy
    ```

=== "pip"

    ```bash
    pip install complexipy
    ```

!!! note "Git para las opciones de diff"

    Las opciones `--diff`, `--diff-only` y `--staged` requieren Git 2.30 o posterior
    en tu `PATH`. Comprueba tu versión con `git --version`. Git no es necesario
    para el análisis sin estas opciones.

## ¿Qué es la Complejidad Cognitiva?

> La complejidad cognitiva mide qué tan difícil es entender el código para los seres humanos, no para las máquinas.

Estas dos funciones tienen la misma complejidad ciclomática, 4. Su complejidad cognitiva es diferente:

=== "Plana: puntuación 3"

    ```python
    def example1(a, b, c):
        if a:        # +1
            return 1
        if b:        # +1
            return 2
        if c:        # +1
            return 3
        return 4
    ```

=== "Anidada: puntuación 6"

    ```python
    def example2(a, b, c):
        if a:        # +1
            if b:    # +2 (1 + nesting_level)
                if c:  # +3 (1 + nesting_level)
                    return 1
        return 4
    ```

El código anidado obliga a quien lo lee a mantener más contexto, así que cada nivel de anidamiento suma a la puntuación. complexipy implementa para Python la métrica de [Complejidad Cognitiva](https://www.sonarsource.com/resources/cognitive-complexity/) de G. Ann Campbell. [Cómo se calculan las puntuaciones](understanding-scores.md)

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
result = file_complexity("app.py")
print(f"File complexity: {result.complexity}")

for func in result.functions:
    print(f"{func.name}: {func.complexity}")
```

## Funcionalidades

<div class="grid cards" markdown>

- :material-brain: **Puntuaciones de complejidad cognitiva**

    ______________________________________________________________________

    Puntuaciones que siguen el artículo de G. Ann Campbell: anidamiento, interrupciones de flujo y secuencias booleanas.

    [:octicons-arrow-right-24: Cómo funcionan las puntuaciones](understanding-scores.md)

- :material-source-branch: **Bloquea solo lo que un cambio empeora**

    ______________________________________________________________________

    `--diff` y `--staged` comparan cada función con una referencia de git.

    [:octicons-arrow-right-24: Diff de complejidad](diff-and-snapshots.md#diff-de-complejidad)

- :material-camera: **Snapshots base**

    ______________________________________________________________________

    Registra el estado actual de una base de código existente y falla solo con funciones nuevas o que empeoran.

    [:octicons-arrow-right-24: Snapshots](diff-and-snapshots.md#snapshots-base)

- :material-auto-fix: **Sugerencias de refactorización y `--fix`**

    ______________________________________________________________________

    Reglas como cláusulas de guarda y extraer función auxiliar, y correcciones automáticas donde son seguras.

    [:octicons-arrow-right-24: Reglas de refactorización](refactoring-rules.md)

- :material-file-export: **Formatos de salida**

    ______________________________________________________________________

    JSON, CSV, SARIF para GitHub code scanning y GitLab Code Quality.

    [:octicons-arrow-right-24: Formatos de salida](usage-guide.md#formatos-de-salida)

- :material-code-braces: **Servidor de lenguaje**

    ______________________________________________________________________

    Inlay hints, hover y advertencias en VS Code, Neovim y Zed.

    [:octicons-arrow-right-24: Integración con editores](editors.md)

- :material-check-decagram: **GitHub Action y hook de pre-commit**

    ______________________________________________________________________

    Ejecuta el mismo control en CI y antes de cada commit.

    [:octicons-arrow-right-24: Integración con CI/CD](ci.md)

</div>

## Preguntas Frecuentes

**[¿Cómo se calcula la complejidad?](understanding-scores.md)**
Aprende sobre el algoritmo de puntuación, qué aporta cada estructura de control y cómo afecta el anidamiento a la puntuación final.

**[¿Cómo se compara esto con PLR0912 de Ruff?](comparison-with-ruff.md)**
Comprende las diferencias clave entre la complejidad ciclomática (Ruff) y la complejidad cognitiva (complexipy), y por qué podrías querer usar ambas.

**[¿Es esto un producto de SonarSource/Sonar?](about.md)**
No. complexipy es un proyecto independiente inspirado en la investigación de G. Ann Campbell, pero no está afiliado ni respaldado por SonarSource.

## Integraciones

=== "GitHub Actions"

    ```yaml
    - uses: rohaquinlop/complexipy-action@v8
      with:
          paths: .
          max_complexity_allowed: 10
          output_json: true
    ```

=== "Hook de Pre-commit"

    ```yaml
    repos:
        - repo: https://github.com/rohaquinlop/complexipy-pre-commit
          rev: v8.3.0
          hooks:
              - id: complexipy
    ```

=== "Editores"

    Todos los editores usan `complexipy lsp`, un servidor de lenguaje con inlay hints, hover y advertencias que se actualizan mientras escribes. Instala `complexipy` primero (`uv tool install complexipy`).

    Para VS Code, instala la extensión desde el [Visual Studio Marketplace](https://marketplace.visualstudio.com/items?itemName=rohaquinlop.complexipy) o desde [OpenVSX](https://open-vsx.org/extension/rohaquinlop/complexipy). Para Neovim y Zed, consulta [Integración con Editores](editors.md).

## Aprende Más

- [Guía de Uso](usage-guide.md) - instalación, el flujo básico de la CLI y solución de problemas
- [Adoptar en una Base de Código Existente](adoption.md) - línea base, bloquear código nuevo y luego bajar el umbral
- [Configuración](configuration.md) - archivos de configuración y la caché
- [Diff y Snapshots](diff-and-snapshots.md) - diff de complejidad y snapshots base
- [Ignorados y Selección de Reglas](ignores.md) - ignorados en línea y selección de reglas
- [Opciones de CLI](cli-reference.md) - todas las flags de CLI
- [Integración con Editores](editors.md) - el servidor de lenguaje, los inlay hints y la configuración de VS Code, Neovim y Zed
- [Referencia de la API](api-reference.md) - la API completa de Python
- [Qué Significan las Puntuaciones](understanding-scores.md) - cómo funciona el algoritmo de puntuación
- [Comparación con Ruff](comparison-with-ruff.md) - complejidad cognitiva vs ciclomática
- [Reglas de Refactorización](refactoring-rules.md) - las reglas detrás de `--suggest-refactors`
- [Registro de Cambios](changelog.md) - qué cambió en cada versión

______________________________________________________________________

<div align="center">

<p style="margin: 0.25rem 0"><sub>Inspirado en la investigación de <a href="https://www.sonarsource.com/resources/cognitive-complexity/">Complejidad Cognitiva</a> de G. Ann Campbell</sub></p>
<p style="margin: 0.25rem 0"><sub>complexipy es un proyecto independiente y no está afiliado ni respaldado por SonarSource</sub></p>
<p style="margin: 0.25rem 0"><sub>Desarrollado con ❤️ por <a href="https://github.com/rohaquinlop">@rohaquinlop</a> y <a href="https://github.com/rohaquinlop/complexipy/graphs/contributors">colaboradores</a></sub></p>

</div>
