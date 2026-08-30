# Puntos de Referencia

Esta página documenta cómo se compara la CLI de Rust de la versión 8.0.0
con la CLI de Python de la versión 7.0.1. Ambas versiones comparten el
mismo motor de análisis en Rust; la diferencia es la tubería de la CLI: el
análisis de argumentos, la resolución de configuración, el recorrido de
rutas y la representación de la salida ahora se ejecutan en Rust, y el
intérprete de Python ya no realiza ningún trabajo de análisis.

## Metodología

- **CLI nueva (pre-lanzamiento):** compilada desde la rama de
  lanzamiento con `uv run maturin develop --release`, invocada como
  `.venv/bin/complexipy`. Reporta la versión 7.0.1 porque el incremento
  de versión ocurre en el momento del lanzamiento; el commit exacto está
  registrado en el bloque de entorno más abajo.
- **CLI de referencia:** `complexipy==7.0.1` instalada desde PyPI con
  `uv add` en un proyecto aislado, invocada como `bin/complexipy` de su
  entorno virtual.
- **Paridad de invocación:** ambas CLIs se ejecutan como scripts de
  consola directos con banderas idénticas. El envoltorio `uv run` queda
  excluido de la medición para que ningún lado pague el costo del proceso
  de uv.
- **Corpus:** repositorios reales de código abierto, clonados de forma
  superficial en commits fijados, más una prueba de un solo archivo que
  aísla el arranque del intérprete y de la CLI del análisis del árbol.
- **Métricas:** tiempo de pared con hyperfine (calentamiento 3, 5
  ejecuciones; la prueba de un solo archivo usa calentamiento 5, 20
  ejecuciones) con la salida estándar descartada; RSS máximo con
  `/usr/bin/time -l`, 3 ejecuciones, se reporta el máximo.
- **Puerta de paridad:** antes de medir, ambas CLIs exportan JSON para
  cada repositorio del corpus. Las exportaciones son idénticas byte a byte
  y los códigos de salida coinciden, por lo que la comparación mide el
  mismo trabajo en ambos lados.

El punto de referencia es repetible:

```bash
bash benchmarks/benchmark-cli.sh
```

--8<-- "benchmarks/results.md"

## Qué dicen los números

- **Las cargas dominadas por el arranque ganan más.** La prueba de un
  solo archivo es unas 3.7 veces más rápida: es el costo del intérprete de
  Python y de la importación de typer/rich eliminado del camino crítico.
- **Los árboles pequeños y medianos** (requests, flask) son
  aproximadamente 1.7-2.7 veces más rápidos.
- **Los árboles grandes** (django, ~2900 archivos) son aproximadamente
  1.4-1.6 veces más rápidos; allí domina el motor compartido y queda menos
  sobrecarga de CLI por eliminar.
- **La representación de la salida también es más barata.** Mostrar la
  tabla completa de resultados en django cuesta ~0.4 s en la CLI de Rust
  frente a ~1.5 s en la CLI de Python (la diferencia entre el modo por
  defecto y --quiet), así que la brecha de tiempo crece en árboles grandes
  cuando los resultados se imprimen de verdad - las filas [render] miden
  ese camino de extremo a extremo.
- **La memoria máxima es menor en todos los casos**, entre 10 y 25 MB
  menos, porque el intérprete de Python y sus bibliotecas de presentación
  ya no están en el proceso.
