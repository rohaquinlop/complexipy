# Guía de Uso

Esta guía cubre todo lo que necesitas saber para usar complexipy de manera efectiva en tus proyectos de Python.

!!! info "Secciones movidas"

    Algunas secciones de esta guía ahora tienen su propia página:

    - Archivos de configuración y la caché: [Configuración](configuration.md)
    - Diff de complejidad y snapshots base: [Diff y Snapshots](diff-and-snapshots.md)
    - Selección de reglas e ignorados en línea: [Ignorados y Selección de Reglas](ignores.md)
    - Sugerencias de refactorización y `--fix`: [Reglas de Refactorización](refactoring-rules.md#uso-de-las-reglas-de-refactorizacion)
    - Integración con CI/CD: [Integración con CI/CD](ci.md)
    - Todas las opciones de CLI: [Opciones de CLI](cli-reference.md)
    - Ejemplos de la API de Python: [Referencia de la API](api-reference.md#api-de-python)
    - VS Code: [Integración con Editores](editors.md)

## Instalación

=== "pip"

    ```bash
    pip install complexipy
    ```

=== "uv"

    ```bash
    uv add complexipy
    ```

=== "uv tool"

    ```bash
    uv tool install complexipy
    ```

=== "poetry"

    ```bash
    poetry add complexipy
    ```

El comando `complexipy` es una implementación nativa escrita en Rust;
todo el pipeline de análisis se ejecuta sin un intérprete de Python.
La misma rueda también proporciona la API de Python para integrar el
análisis en tus propias herramientas.

Las opciones `--diff`, `--diff-only` y `--staged` requieren Git 2.30 o posterior
en tu `PATH`. Comprueba tu versión con `git --version`. Git no es necesario
para el análisis sin estas opciones.

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

Mostrar solo las N funciones más complejas entre todos los archivos analizados:

```bash
complexipy . --top 10
```

Mostrar sugerencias deterministas de refactorización para funciones fallidas:

```bash
complexipy . --failed --suggest-refactors
```

Cuando se usa `--top`, los resultados se reordenan globalmente por complejidad
descendente antes de truncarse.

Suprimir la salida del análisis (útil para pipelines de CI):

```bash
complexipy . --quiet
```

`--quiet` solo oculta el reporte. Cada advertencia se sigue imprimiendo en
stderr, así que un log de CI explica un resultado inesperado.

### Ordenar Resultados

Ordenar por puntuación de complejidad:

```bash
complexipy . --sort asc   # Ascendente (predeterminado)
complexipy . --sort desc  # Descendente
complexipy . --sort file_name  # Alfabéticamente por nombre de archivo
```

### Excluir Archivos y Directorios

Excluir rutas específicas del análisis:

```bash
# Excluir un directorio recursivamente
complexipy . --exclude "tests/**"

# Excluir múltiples directorios recursivamente
complexipy . --exclude "tests/**" --exclude "migrations/**" --exclude "build/**"

# Excluir archivos específicos
complexipy . --exclude "src/legacy/old_code.py"
```

!!! note "Cómo funciona la exclusión"

    - Las exclusiones son patrones glob evaluados de forma relativa a cada ruta raíz proporcionada, y de forma relativa al directorio donde se ejecuta el comando, donde está el archivo de configuración
    - Un patrón que empieza con `/`, `./` o `../`, o termina en `/`, nunca coincide; complexipy avisa de ello
    - Usa `directory/**` para excluir un directorio recursivamente
    - Usa una ruta relativa exacta, como `src/legacy/old_code.py`, para excluir un archivo
    - Las reglas de gitignore se siguen respetando durante el descubrimiento de archivos, y también los archivos `.ignore` y los archivos y directorios ocultos. `--report-ignored` lista cada archivo `*.py` que estas reglas omiten, y una ejecución que no selecciona ningún archivo imprime una advertencia en stderr, incluso bajo `--quiet`
    - Un archivo pasado explícitamente como argumento se analiza aunque un patrón coincida con él. Con `--force-exclude`, o `force-exclude = true` en la configuración, los patrones también se aplican a los argumentos de archivo, solo de forma relativa al directorio donde se ejecuta el comando, no al directorio del propio archivo. Las reglas de gitignore y las rutas ocultas nunca se aplican a los argumentos de archivo
    - Un enlace simbólico `*.py` se analiza solo cuando apunta a un archivo regular dentro del directorio analizado. Los enlaces que salen de él, los enlaces rotos, los dispositivos y los directorios llamados `*.py` se omiten con una advertencia
    - Un archivo fuente de más de 64 MiB se reporta como fallido en lugar de leerse

### Formatos de Salida

Guarda los resultados en JSON, CSV, GitLab Code Quality o SARIF:

```bash
# Salida JSON (guardada en complexipy-results.json)
complexipy . --output-format json

# Salida CSV (guardada en complexipy-results.csv)
complexipy . --output-format csv

# Ambos
complexipy . --output-format json --output-format csv

# Destino explícito para un solo formato
complexipy . --output-format gitlab --output complexipy-code-quality.json

# Varios formatos escritos en un directorio
complexipy . --output-format json --output-format sarif --output reports/

# Reporte de GitLab Code Quality
complexipy . --output-format gitlab

# Salida SARIF
complexipy . --output-format sarif

# Añade --suggest-refactors a cualquiera de los anteriores para incluir
# también los hallazgos de las reglas de refactorización (C001, C007, ...)
# como sus propios IDs de regla SARIF/GitLab, junto a los hallazgos de
# complejidad cognitiva que siempre están presentes.
complexipy . --output-format sarif --suggest-refactors
```

JSON, SARIF y GitLab Code Quality siguen la misma regla: los datos de los
planes de refactorización (`refactor_plans` en JSON, hallazgos por regla en
SARIF/GitLab) solo se emiten cuando también se pasa `--suggest-refactors`.
Sin esa flag, solo aparecen los hallazgos del umbral de complejidad
cognitiva -- igual que la salida enriquecida de la CLI, que también oculta
las sugerencias de refactorización a menos que se active la flag.

Con `--diff`, `--diff-only` o `--staged`, cada resultado de SARIF, incluidos
los hallazgos de refactorización, lleva el `baselineState` de su función
respecto a la referencia: `new` para una función que la referencia no tiene,
`updated` cuando su complejidad cambió y `unchanged` cuando no cambió o
cuando el diff no tocó su archivo. Una función que el control empareja con
otra a la que reemplaza se compara con esa función. Cuando el diff decide el
código de salida, el `level` de un resultado de complejidad es `error` para
una función que hace fallar la ejecución, por el control o por el snapshot,
y `note` para una que la ejecución deja pasar, y el reporte de GitLab Code
Quality califica esas mismas funciones como `major` e `info`. En otro caso
los niveles siguen siendo `warning` y `minor`. Las huellas (fingerprints) no
cambian.

### Salida en Texto Plano

Usa la salida en texto plano cuando necesites una línea legible por máquina por
función:

```bash
complexipy . --plain
complexipy . --plain --top 5
complexipy . --plain --failed --max-complexity-allowed 10
```

Cada línea se emite con este formato:

```text
<path>:<line> <function> <complexity>
```

`<line>` es la línea donde empieza la función, así que los editores y las terminales pueden enlazarla. Con `--suggest-refactors`, cada plan agrega una línea con sangría con su id de regla, su título y su reducción. Un `~` marca una reducción estimada, y `complexipy --explain <regla>` imprime el texto completo de la regla:

```text
src/main.py:1 nested 6
  C007 Merge 3 nested if statements (-4)
```

`--plain` es solo para CLI y no se puede combinar con `--quiet`.

### Complejidad de Script a Nivel de Módulo

Usa `--check-script` para incluir el código a nivel de módulo en los resultados
como `<module>`:

```bash
complexipy path/to/script.py --check-script
complexipy path/to/script.py --check-script --max-complexity-allowed 5
```

Esto es útil para scripts con flujo de control complejo en el nivel superior,
fuera de las funciones.

Cada función que define un módulo se puntúa por separado, esté donde esté el
`def`: bajo un `if`, `try`, `with`, `for`, `while` o `match` a nivel de módulo,
y en clases anidadas, cuyos métodos se llaman `Outer::Inner::method`. Una
función definida dentro de otra función cuenta para la función exterior.

Un archivo fuente con una declaración de codificación PEP 263 se lee en esa
codificación: UTF-8 y Latin-1 siempre, y una codificación compatible con ASCII
como `cp1252` cuando el archivo es ASCII puro. Un archivo que declara otra
codificación, como `utf-7`, se reporta como fallido, porque leerlo como UTF-8
no mostraría el código que ejecuta Python.

### Salida en Color

Controla la salida en color:

```bash
complexipy . --color auto  # Predeterminado: detecta automáticamente la compatibilidad con la terminal
complexipy . --color yes   # Forza colores
complexipy . --color no    # Deshabilita colores
```

`auto` da estilo a un flujo solo cuando es una terminal y `NO_COLOR` no está definido o está vacío. `--color` cubre cada línea con estilo, incluido el diff de `--fix` y los avisos en stderr, y la salida `--plain` nunca lleva estilo.

El texto que viene de los archivos analizados se imprime con sus caracteres de control escapados: una ruta, una línea de código o un valor de configuración muestra un carácter de escape como `\u{1b}` y un salto de línea como `\u{a}`, así que no puede mover el cursor, reescribir líneas anteriores, cambiar el título de la ventana ni imprimir un enlace oculto.

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
complexipy . --output-format json
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
complexipy . --exclude "venv/**" --exclude ".venv/**" --exclude "node_modules/**"
```

### Resultados diferentes a los esperados

Verifica la precedencia de los archivos de configuración: complexipy lee solo el primer archivo de configuración que se analiza, y una advertencia nombra el archivo que no aplica. Ejecuta con `--isolated` para comparar con los valores por defecto:

```bash
complexipy . --isolated
```

## Próximos Pasos

- Lee [Entendiendo las Puntuaciones de Complejidad](understanding-scores.md) para interpretar los resultados
- Consulta [Comparación con Ruff](comparison-with-ruff.md) para herramientas complementarias
- Visita [Acerca de complexipy](about.md) para obtener más información sobre el proyecto
