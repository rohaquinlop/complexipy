# Ignorados y Selección de Reglas

## Selección de Reglas

Las reglas de refactorización llevan ids como `C001` (ver
[Reglas de Refactorización](refactoring-rules.md)). Dos flags eligen qué reglas
están activas, y ambas aceptan listas separadas por comas y se pueden repetir:

| Flag | Efecto |
| -- | -- |
| `--select C001,C007` | Reporta solo las reglas nombradas |
| `--ignore C007` | Nunca reporta las reglas nombradas |
| `--select complexity` | Reporta solo las reglas de un grupo |
| `--ignore readability` | Nunca reporta las reglas de un grupo |

Un nombre de grupo representa todas las reglas de esa categoría, como se
listan en [Reglas de Refactorización](refactoring-rules.md#referencia-de-ids-de-reglas):
`complexity` o `readability`. Los nombres de grupo se combinan con ids de
reglas, como en `--select complexity,C007`.

Sin ninguna flag, todas las reglas están activas. Cuando ambas flags nombran la
misma regla, `--ignore` gana. Los ids de reglas y los nombres de grupo no
distinguen mayúsculas de minúsculas. Un id desconocido imprime una advertencia en stderr de que no
coincide con ninguna regla registrada. Una lista `select` sigue limitando el
conjunto activo aunque uno de sus ids sea desconocido, y un segundo aviso lo
indica cuando `select` e `ignore` no dejan ninguna regla activa.

Las mismas claves funcionan en `complexipy.toml`, `.complexipy.toml` y
`pyproject.toml`:

```toml
select = ["C001", "C007"]
ignore = ["C002"]
```

Un valor toml también puede ser una sola cadena separada por comas, como las
flags: `select = "C001, C007"`.

Una flag reemplaza la clave toml que nombra. La selección de reglas nunca
cambia el código de salida: el código de salida sigue al umbral de complejidad,
a los snapshots, a las rutas y al gating de diff. Una regla que no está activa
nunca aparece en la terminal, en los planes de refactorización, en JSON, CSV,
SARIF ni en la salida de GitLab.

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

Una marca cuenta solo dentro de un comentario: en la línea del `def`, en una
línea de su firma o de sus decoradores, o como comentario en su propia línea
justo encima. El mismo texto en una cadena, un docstring o un argumento de
decorador, o un comentario al final de la línea de código anterior, no suprime
nada. Las palabras se comparan completas y separadas solo por espacios o
tabuladores, así que `# complexipy: ignored` no es una marca.

### Suprimir Reglas Específicas

Añade una lista de reglas entre corchetes para conservar la función pero
ocultar las reglas nombradas:

```python
def tangled(a, b):  # complexipy: ignore[C007]
    pass
```

La función sigue analizándose y reportándose; solo se suprimen las reglas
nombradas para esa función. Separa varios ids con comas:
`# complexipy: ignore[C001,C007]`. La forma `# noqa: complexipy[C007]` se
comporta igual. Un marcador sin lista sigue suprimiendo la función completa.
Un texto entre corchetes que no es una lista de ids de reglas es un motivo, así
que `# complexipy: ignore [technical debt]` sigue suprimiendo la función
completa. Aquí los nombres de grupo no son ids de reglas:
`# complexipy: ignore[readability]` también suprime la función completa.

!!! note "Sintaxis Obsoleta"

    La sintaxis `# noqa: complexipy` está obsoleta y será eliminada en una versión futura.
    Por favor, migra a `# complexipy: ignore` en su lugar.

    **¿Por qué?** Herramientas como [yesqa](https://github.com/asottile/yesqa) eliminan automáticamente los comentarios `# noqa`
    que flake8 no reconoce, lo que eliminaría silenciosamente tus supresiones de complexipy.
    La nueva sintaxis evita este conflicto por completo.

!!! warning "Usar con Moderación"

    Los ignorados en línea deben ser temporales. Documenta por qué la complejidad es necesaria y rastrea la deuda técnica.

### Deshabilitar Ignorados en Línea

Usa `--no-ignore` para ignorar todos los comentarios de supresión y analizar cada función:

```bash
complexipy . --no-ignore
```

Las funciones previamente suprimidas por `# complexipy: ignore` o `# noqa: complexipy` se analizarán normalmente y pueden superar el umbral.

### Reportar Funciones Ignoradas

Usa `--report-ignored` para listar cada ubicación donde un comentario de ignore suprime una función:

```bash
# Listar funciones ignoradas
complexipy . --report-ignored

# Combinar con --no-ignore para reportar mientras se analiza todo
complexipy . --report-ignored --no-ignore
```

Cada ubicación se imprime como `ruta:línea  # texto-del-comentario`, seguida del total. Cuando `--output-format json` también está activo, las ubicaciones ignoradas se exportan a `complexipy-ignored.json`. `--quiet` oculta la lista, y la exportación JSON se escribe de todos modos.

Ambos flags también están disponibles en la API de Python mediante `no_ignore=True`:

```python
from complexipy import file_complexity, code_complexity

# Analizar sin respetar los comentarios de ignore
result = file_complexity("app.py", no_ignore=True)
```

Para recolectar ubicaciones ignoradas programáticamente, usa `collect_all_ignored_locations()`:

```python
from complexipy import collect_all_ignored_locations

locations, failed = collect_all_ignored_locations(
    paths=["src"],
    exclude=["tests/**"],
)
for loc in locations:
    print(f"{loc.path}:{loc.line}  {loc.comment}")
```

### Eliminar Comentarios de Ignore Obsoletos

Un comentario de ignore solo es necesario mientras la complejidad de la función
suprimida supere `--max-complexity-allowed`. Cuando la complejidad de la función
baje al límite o por debajo de él, el comentario queda obsoleto y puede
eliminarse. complexipy detecta esto automáticamente en cada ejecución e informa
las ubicaciones que puedes limpiar:

```bash
complexipy .
```

Ejemplo de salida:

```text
The following ignore comment(s) are no longer necessary (complexity is within the allowed limit) and can be removed:
src/legacy.py:42  function=parse_legacy_config complexity=8  # complexipy: ignore
```

El informe es puramente informativo: nunca afecta al código de salida y se
suprime bajo `--quiet`. La misma detección está disponible programáticamente vía
`collect_removable_ignored_locations()`:

```python
from complexipy import collect_removable_ignored_locations

removable, failed = collect_removable_ignored_locations(
    paths=["src"],
    exclude=["tests/**"],
    max_complexity_allowed=15,
)
for rem in removable:
    print(f"{rem.path}:{rem.line}  function={rem.function} complexity={rem.complexity}")
```
