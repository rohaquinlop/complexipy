# Adoptar complexipy en una Base de Código Existente

Una base de código que ya tiene funciones complejas falla en la primera ejecución. Esta guía agrega complexipy sin corregir todo primero: registra lo que existe, bloquea todo lo nuevo o lo que empeora y reduce el trabajo pendiente con el tiempo.

La salida de esta página viene de una ejecución sobre el código fuente de [Flask](https://github.com/pallets/flask) con el umbral por defecto de 15.

## 1. Ver el estado actual

Lista las funciones más complejas:

```bash
complexipy src --top 5
```

```text
src/flask/sansio/blueprints.py
    Blueprint::register 29  ❌  FAILED

src/flask/app.py
    Flask::make_response 26  ❌  FAILED
    Flask::url_for 22  ❌  FAILED

src/flask/cli.py
    find_best_app 18  ❌  FAILED
    find_app_by_string 17  ❌  FAILED
```

Aquí un `complexipy src` simple termina con código 1, así que todavía no puede ejecutarse en CI.

## 2. Registrar una línea base

```bash
complexipy src --snapshot-create
```

```text
No function fails the run. 10 functions above 15 are ALLOWED: no worse than the baseline the gate compares with.
Snapshot watermark passed. Baseline stored at ./complexipy-snapshot.json
```

Haz commit de `complexipy-snapshot.json`. Desde ahora, cada ejecución lo lee: una función del snapshot pasa mientras no se vuelva más compleja, y una función que mejora sale del snapshot. Consulta [Snapshots Base](diff-and-snapshots.md#snapshots-base).

## 3. Bloquear funciones nuevas y las que empeoran

Ejecuta el mismo comando en CI, sin `--snapshot-create`. Una función nueva por encima del umbral hace fallar la ejecución:

```text
src/flask/helpers.py
    merge_headers 23 (new, Δ = +23)  ❌  FAILED
```

Para revisar solo lo que cambia una rama, compárala con la rama destino:

```bash
complexipy src --diff origin/main
```

```text
+--------+-------------------------------------+-----------+-------+
| Status | Location                            | Change    | Gate  |
+==================================================================+
| NEW    | src/flask/helpers.py::merge_headers | 23  (new) | FAILS |
+--------+-------------------------------------+-----------+-------+

Net: 1 new
Gate: 1 function fails: REGRESSED or NEW above the threshold.
```

Consulta [Integración con CI/CD](ci.md) para la GitHub Action, el hook de pre-commit y GitLab, y [Diff de Complejidad](diff-and-snapshots.md#diff-de-complejidad) para las opciones de diff.

## 4. Ver las puntuaciones en el editor

El servidor de lenguaje muestra la puntuación de cada función por encima del umbral mientras escribes, así una función recibe atención antes de llegar a CI. Establece `inlay-hints = "always"` en la [configuración del servidor de lenguaje](configuration.md#configuracion-del-servidor-de-lenguaje) para mostrar la puntuación de todas las funciones. Consulta [Integración con Editores](editors.md) para VS Code, Neovim y Zed.

## 5. Reducir el trabajo pendiente

Pide sugerencias de refactorización para las funciones que fallan:

```bash
complexipy src --failed --suggest-refactors
```

Previsualiza las correcciones que complexipy puede aplicar por sí mismo y luego aplícalas:

```bash
complexipy src --fix --dry-run
complexipy src --fix
```

```text
Fixed C002 at src/flask/helpers.py:684-694 (-4 complexity)
Skipped C001 at src/flask/helpers.py:684-694 (not safe to auto-apply)
```

Aquí `merge_headers` pasó de 23 a 19. La sugerencia omitida necesita un cambio manual. Consulta [Reglas de Refactorización](refactoring-rules.md) para saber qué hace cada regla y cuáles aplica `--fix`.

## 6. Bajar el umbral

Cuando el trabajo pendiente sea pequeño, establece un `max-complexity-allowed` más bajo en tu archivo de configuración y registra la línea base de nuevo con `--snapshot-create`. Repite en pasos pequeños. Consulta [Configuración](configuration.md) y [Qué Significan las Puntuaciones](understanding-scores.md) para los umbrales recomendados.
