# Diff y Snapshots

## Diff de Complejidad

Las opciones `--diff`, `--diff-only` y `--staged` requieren Git 2.30 o posterior
en tu `PATH`. Comprueba tu versión con `git --version`. Git no es necesario
para el análisis sin estas opciones.

Compara los resultados actuales contra cualquier referencia de git:

```bash
complexipy . --diff HEAD~1
complexipy . --diff main
complexipy src/ --max-complexity-allowed 10 --diff HEAD~1
```

Por defecto `--diff` **aplica** el umbral de complejidad: la ejecución termina con código `1` solo cuando un cambio rompe el contrato definido por `--max-complexity-allowed`:

- Una función **nueva** introducida por encima del umbral.
- Una función **modificada** cuya complejidad aumentó **y** queda por encima del umbral (incluye funciones ya sobre el umbral que empeoran).

Las funciones que aumentan su complejidad pero se mantienen en o por debajo del umbral (por ejemplo `3 → 4` con `--max-complexity-allowed 15`) **no** son fallos.

Para ver el diff visualmente sin afectar el código de salida, usa `--diff-only` en su lugar:

```bash
complexipy . --diff-only HEAD~1
```

Para comparar el contenido **staged** (el índice de git) en lugar del árbol de trabajo - "¿qué complejidad estoy a punto de commitear?" - añade `--staged`:

```bash
# Cambios staged vs HEAD (la línea base por defecto de --staged)
complexipy . --staged

# Cambios staged vs una referencia específica
complexipy . --diff main --staged
```

Igual que `--diff`, `--staged` aplica el umbral contra el contenido staged y falla cuando un cambio staged empuja una función por encima de `--max-complexity-allowed`. Compara solo los archivos staged que la ejecución selecciona: los argumentos de ruta, `paths` y `exclude` se aplican igual que sin `--staged`, al índice y no al árbol de trabajo, así que un archivo regular staged se compara tenga lo que tenga el árbol de trabajo en su ruta. Las eliminaciones staged que el descubrimiento habría seleccionado producen entradas `REMOVED`: bajo un directorio seleccionado, fuera de rutas ocultas y sin coincidir con `exclude`, `.gitignore`, `.ignore` ni el archivo de exclusión de git. Las adiciones staged producen entradas `NEW`. Usa `--diff-only` con `--staged` para una vista solo visual.

Un renombrado o un movimiento pasa el control mientras el cuerpo de la función quede como estaba. Un archivo que git empareja como renombrado se compara con su ruta anterior cuando la ejecución cubre esa ruta; un archivo movido desde fuera de las rutas seleccionadas se compara como añadido, y uno movido fuera de ellas no reporta nada. Una ruta que no era un archivo regular en la referencia, como un enlace simbólico, no se lee allí: se compara como añadida, y su eliminación no reporta nada. Una función por encima del umbral que el cambio añade pasa solo cuando reemplaza a una función `REMOVED` con el mismo cuerpo y al menos su complejidad, cada función eliminada reemplazada una sola vez, y los archivos comparados no contienen más funciones con ese cuerpo que en la referencia. La complejidad importa porque una llamada al propio nombre de la función cuenta como recursión, así que un renombrado puede hacer más complejo el mismo cuerpo. Los archivos comparados son los que compara el diff: modificados, renombrados desde una ruta cubierta, eliminados de una ruta cubierta y añadidos. Los cuerpos se comparan por estructura, sin tener en cuenta el formato ni los comentarios, y dejan fuera el nombre, los decoradores y los parámetros, así que un renombrado de función, un renombrado de clase, un movimiento a otro archivo y un método que pasa a ser función pasan, mientras que una función que se mueve y cambia falla. Las funciones que oculta un comentario de ignorado cuentan en los dos lados, así que una función copiada de otra que ahora está oculta falla. Los cuerpos se cuentan y se emparejan solo dentro del repositorio que contiene cada archivo, ya que cada repositorio se compara en su propia referencia. La tabla del diff nombra, en una columna `Gate`, la función `REMOVED` con el mismo cuerpo a la que reemplaza cada función que pasa. Una función que ahora oculta un comentario de ignorado no se eliminó, así que no se reporta como `REMOVED`. Un archivo de Python eliminado que el descubrimiento habría seleccionado reporta sus funciones como `REMOVED`, así que una función que salió de él también pasa.

Cuando `--diff` o `--staged` decide el código de salida, el reporte muestra el veredicto del control. Una función que hace fallar la ejecución, REGRESSED o NEW por encima del umbral, aparece como `FAILED` y se marca `FAILS` en la columna `Gate` de la tabla del diff, y la tabla termina con una línea `Gate:` que las cuenta. Una función por encima del umbral que el control deja pasar aparece como `ALLOWED`, y `--failed` lista solo las funciones que hacen fallar la ejecución. Una ejecución con snapshot también lista como `ALLOWED` las funciones que su snapshot deja pasar.

Debajo de la tabla del diff, el reporte lista cada función que un comentario de ignorado oculta ahora pero no ocultaba en la referencia: un comentario añadido a una función existente, o una función nueva añadida con uno. Cada línea da `ruta:línea`, la función y su puntuación sin tener en cuenta el comentario. La lista informa a quien revisa y nunca cambia el código de salida. Una función renombrada o movida que conservó su comentario también aparece, porque el nombre bajo el que se oculta es nuevo. `--quiet` oculta la lista, como oculta la tabla.

Esto requiere `git` y una ruta dentro de un repositorio. `--diff` y
`--diff-only` leen cada archivo del repositorio que lo contiene, así que
una ejecución desde fuera del repositorio compara contra el mismo historial
que una ejecución desde dentro. Un submódulo, es decir, un repositorio que
el repositorio que lo rodea rastrea como gitlink en su índice, se lee en el
commit que el repositorio exterior fija para él en la referencia, y un
anidamiento más profundo sigue cada fijación a su vez. Cuando la ejecución
empieza dentro del submódulo, se lee en la propia referencia. Un
repositorio dentro de otro que el exterior no rastrea se lee en la
referencia, como cualquier otro repositorio. `--staged` usa el repositorio
del directorio de trabajo; fuera de un repositorio imprime una advertencia
y no compara nada, así que el reporte lista como `FAILED` las funciones por
encima del umbral pero el umbral no decide el código de salida. Cuando un
archivo está fuera de todo repositorio, la referencia no nombra ningún
commit en el repositorio del archivo, o el repositorio exterior no fija
ningún commit para el submódulo en la referencia, o fija uno que el
submódulo no tiene, cada función de ese archivo se reporta como `NEW` y la
ejecución imprime una advertencia que nombra la causa. Las funciones de un
archivo que no se puede analizar en la referencia también se reportan como
`NEW`.

En lugar de repetir la referencia en cada llamada, declara la política de
comparación una sola vez en un archivo de configuración - ver
[Configuración de Diff](configuration.md#configuracion-de-diff).

## Snapshots Base

Los snapshots te permiten adoptar complexipy gradualmente en bases de código grandes y existentes.

### Crear un Snapshot

```bash
complexipy . --snapshot-create --max-complexity-allowed 15
```

Esto crea `complexipy-snapshot.json` en tu directorio de trabajo, registrando todas las funciones que actualmente superan el umbral.

Crea la línea base desde la línea de comandos. `snapshot-create = true` en un
archivo de configuración vuelve a registrar la línea base en cada ejecución,
así que toda violación actual pasa; esa ejecución imprime una advertencia en
stderr que nombra el archivo.

### Cómo Funcionan las snapshots

Una vez que exista un snapshot, complexipy:

- ✅ **Pasa**: Funciones que ya estaban en el snapshot y no han empeorado
- ✅ **Pasa**: Funciones que mejoraron (se eliminan automáticamente del snapshot)
- ❌ **Falla**: Funciones nuevas que superan el umbral
- ❌ **Falla**: Funciones por encima del umbral que se movieron a otro archivo o cambiaron de nombre. El snapshot identifica cada entrada por archivo y nombre, así que ejecuta `--snapshot-create` después de un cambio así para volver a registrarlas
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
[
    {
        "path": "src/legacy.py",
        "file_name": "legacy.py",
        "functions": [
            {
                "name": "old_function",
                "complexity": 23
            }
        ]
    }
]
```

Los snapshots se guardan como un arreglo JSON de archivos analizados. Cada entrada contiene solo las funciones por encima del umbral cuando se escribió el snapshot. El archivo se reescribe después de verificaciones de snapshot exitosas, así que las funciones que mejoran se eliminan automáticamente. Las actualizaciones solo afectan a los archivos analizados en la ejecución - las entradas de archivos fuera del análisis se conservan, por lo que ejecutar sobre un subconjunto de archivos (por ejemplo, a través de un hook de pre-commit) nunca reduce la línea base. Las entradas de archivos que ya no existen se eliminan cuando se reescribe el archivo. Es posible que los snapshots creados por versiones anteriores de complexipy deban regenerarse con `--snapshot-create`.

Las funciones de un mismo archivo que comparten nombre, como el getter y el setter de una propiedad, reciben entradas separadas: la primera en orden de código fuente conserva el nombre, y las siguientes se guardan como `name#2`, `name#3`. `--diff` compara con las mismas claves.
