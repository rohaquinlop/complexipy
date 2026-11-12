# Puntos de Referencia

Esta página documenta cómo se compara la CLI de Rust actual con la CLI de
Python de la versión 7.0.1. Ambas versiones comparten el mismo motor de
análisis en Rust; la diferencia es la tubería de la CLI: el análisis de
argumentos, la resolución de configuración, el recorrido de rutas y la
representación de la salida ahora se ejecutan en Rust, y el intérprete de
Python ya no realiza ningún trabajo de análisis. El bloque de entorno más
abajo indica la versión y el commit exactos.

## Metodología

- **CLI nueva:** compilada desde el repositorio con
  `uv run maturin develop --release`, invocada como
  `.venv/bin/complexipy`.
- **CLI de referencia:** `complexipy==7.0.1` instalada desde PyPI con
  `uv add` en un proyecto aislado, invocada como `bin/complexipy` de su
  entorno virtual.
- **Paridad de invocación:** ambas CLIs se ejecutan como scripts de
  consola directos con flags idénticas, cada carga desde su propio
  directorio vacío. El envoltorio `uv run` queda excluido de la medición
  para que ningún lado pague el costo del proceso de uv.
- **Corpus:** repositorios reales de código abierto, clonados de forma
  superficial en commits fijados, más una prueba de un solo archivo que
  aísla el arranque del intérprete y de la CLI del análisis del árbol.
- **Métricas:** tiempo de pared con hyperfine (calentamiento 3, 5
  ejecuciones; la prueba de un solo archivo usa calentamiento 5, 20
  ejecuciones) con la salida estándar descartada; RSS máximo con
  `/usr/bin/time -l`, 3 ejecuciones, se reporta el máximo.
- **Puerta de paridad:** antes de medir, ambas CLIs exportan JSON para el
  repositorio requests y la prueba de un solo archivo. Los códigos de
  salida coinciden y cada función que reporta la CLI de referencia tiene
  la misma puntuación en la CLI nueva, así que ambos lados puntúan el
  mismo código. La CLI nueva también puntúa funciones definidas bajo un
  `if` o un `try` a nivel de módulo, que la 7.0.1 omite.

El punto de referencia es repetible:

```bash
bash benchmarks/benchmark-cli.sh
```

--8<-- "benchmarks/results.md"

## Qué dicen los números

- **Los árboles grandes ganan más.** django (~2.900 archivos, con su suite
  de pruebas incluida) es aproximadamente 68-91 veces más rápido: el
  motor de Rust analiza el árbol en paralelo en todos los núcleos,
  mientras que la CLI de Python lo procesaba en uno.
- **Los árboles pequeños y medianos** (requests, flask) son
  aproximadamente 9-15 veces más rápidos.
- **El arranque es más barato.** La prueba de un solo archivo es unas 3,3
  veces más rápida: es el costo del intérprete de Python y de la
  importación de typer/rich eliminado del camino crítico.
- **La tabla de escalado** al final de los resultados mide el motor de
  Rust con una fixture sintética en tamaños 1x, 2x y 4x. Razones
  cercanas a 2 significan análisis lineal; razones cercanas a 4
  significan comportamiento cuadrático. Las razones registradas rondan
  1,0-1,2.
- **La representación de la salida también es más barata.** Mostrar la
  tabla completa de resultados en django cuesta ~0,02 s en la CLI de Rust
  frente a ~5,7 s en la CLI de Python (la diferencia entre el modo por
  defecto y --quiet), así que la brecha de tiempo crece en árboles grandes
  cuando los resultados se imprimen de verdad - las filas [render] miden
  ese camino de extremo a extremo.
- **La memoria máxima es menor en árboles pequeños y mayor en los
  grandes.** La CLI de Rust usa 3-5 MB menos en requests y flask y unos
  13 MB menos en la prueba de un solo archivo, porque el intérprete de
  Python y sus bibliotecas de presentación ya no están en el proceso. En
  django alcanza unos 60 MB más: analiza los archivos en paralelo y
  reserva memoria con mimalloc, que cambian memoria por velocidad.

## Comparación con otras herramientas de complejidad cognitiva

<p align="center">
  <img src="../img/competitors-light.svg#only-light" alt="Gráfico de barras del tiempo de pared para puntuar la complejidad cognitiva del código de Django con complexipy, flake8-cognitive-complexity y pylint-cognitive-complexity">
  <img src="../img/competitors-dark.svg#only-dark" alt="Gráfico de barras del tiempo de pared para puntuar la complejidad cognitiva del código de Django con complexipy, flake8-cognitive-complexity y pylint-cognitive-complexity">
</p>

Esta comparación mide las herramientas de Python que calculan la misma métrica, la complejidad cognitiva, sobre el árbol completo de django. Las herramientas que calculan la complejidad ciclomática, como radon o la regla `C901` de Ruff, miden otra métrica y no se incluyen.

- **flake8-cognitive-complexity** se ejecuta como plugin de flake8 con solo `CCR001` seleccionada.
- **pylint-cognitive-complexity** se ejecuta como plugin de pylint con todas las demás comprobaciones desactivadas. pylint usa un solo núcleo por defecto.
- Cada herramienta usa un umbral de 15, se ejecuta con su paralelismo por defecto e ignora cualquier configuración del proyecto. Todas las implementaciones siguen el mismo artículo, pero no reportan la misma puntuación para cada función.

```bash
bash benchmarks/benchmark-competitors.sh
```

--8<-- "benchmarks/competitors.md"
