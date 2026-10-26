# Registro de Cambios

Todos los cambios notables de complexipy se documentan aquí, de más reciente
a más antiguo. Cada sección de versión enlaza a sus notas de versión de
GitHub con todos los detalles.

## Sin publicar

### Añadido

- `file_complexity()` y `code_complexity()` aceptan `with_plans`, `True` por
  defecto. Pasa `with_plans=False` para no construir los planes de
  refactorización cuando solo necesitas las puntuaciones.
- El servidor de lenguaje responde `complexipy/lineComplexities` con el total
  de cada función y el incremento de cada línea, sin importar la configuración
  de los hints. La extensión de VS Code dibuja con él sus insignias de colores
  ([#292](https://github.com/rohaquinlop/complexipy/discussions/292)).
- `[lsp] diagnostic-range` y `[lsp] hover` eligen qué subraya la advertencia y
  dónde responde el hover.
- `--isolated` ignora todos los archivos de configuración, y `--config <ruta>`
  lee solo el archivo indicado: su sección `[tool.complexipy]` cuando el
  archivo se llama `pyproject.toml`, y el archivo completo en otro caso. Las
  dos banderas no se pueden combinar. Pasa cualquiera de las dos en CI para
  fijar el control, de modo que un cambio en la configuración del repositorio
  no pueda alterarlo.

### Cambiado

- El servidor de lenguaje es más discreto por defecto. La advertencia subraya
  el nombre de la función en lugar de toda la función, y el hover responde
  solo en las líneas `def`, así que ya no tapa el hover propio del editor
  dentro del cuerpo
  ([#290](https://github.com/rohaquinlop/complexipy/discussions/290)). Define
  `diagnostic-range = "function"` o `hover = "function"` bajo `[lsp]` para
  recuperar el comportamiento anterior.
- Ejecuciones más rápidas. El descubrimiento de archivos recorre el árbol una
  sola vez, en paralelo, y ya no lee los directorios ignorados por git; las
  rutas que no se pueden analizar se listan en orden de ruta. El reporte por
  defecto, `--report-ignored` y el reporte de comentarios de ignorado obsoletos
  comparten una sola lectura de cada archivo. Los archivos pasados como
  argumentos se analizan en paralelo, y `--fix` vuelve a analizar solo los
  archivos que escribió. La caché de análisis guarda sin interpretar las
  entradas de otros objetivos, reescribe su archivo solo cuando cambian los
  resultados de esta ejecución y conserva como máximo 32 MiB de entradas. En
  macOS, Windows y Linux de 64 bits, complexipy reserva memoria con mimalloc.
- Los planes de refactorización se miden sobre su propia función en lugar del
  archivo completo, así que `--suggest-refactors` crece linealmente con el
  tamaño del archivo
  ([#274](https://github.com/rohaquinlop/complexipy/issues/274)). `--failed`,
  `--fix` y `--dry-run` construyen planes solo para las funciones por encima
  del límite, salvo que una salida liste las demás. La demo web no construye
  planes.
- `--diff` y `--staged` leen cada archivo en la referencia mediante un único
  proceso `git cat-file --batch`, y buscan cada archivo en su propia ruta del
  repositorio. Un archivo que se movió desde la referencia se reporta como
  nuevo, y una ejecución desde un subdirectorio encuentra el historial de cada
  archivo.
- El servidor de lenguaje analiza en su propio hilo, así que responde a las
  solicitudes mientras analiza otros documentos. Cada documento tiene su
  propio retardo de espera, y `$/cancelRequest` cancela una solicitud que aún
  está esperando.
- La API de Python libera el GIL mientras analiza y mientras se ejecuta
  `compute_diff`. `functions`, `line_complexities` y `refactor_plans`
  devuelven una lista nueva sobre objetos construidos una sola vez, en lugar
  de copiar los datos en cada acceso.
- Una ejecución de `complexipy` sin argumentos cuyo archivo de configuración
  no define `paths`, o lo define como una lista vacía, se detiene con el error
  de rutas faltantes en lugar de no analizar nada y pasar. Define `paths` en
  el archivo o indica las rutas en la línea de comandos.
- `paths`, `output` y `cache-dir` leídos de un archivo de configuración deben
  quedar dentro del directorio desde el que se ejecuta complexipy, incluidos
  los enlaces simbólicos, o la ejecución se detiene con un error que nombra la
  clave. Los mismos valores en la línea de comandos no tienen esta
  restricción, así que pasa allí una ruta fuera del proyecto. Una referencia
  de diff que empieza con `-`, venga de `--diff`, de `--diff-only` o de la
  `branch` de `[diff]`, también detiene la ejecución.
- complexipy lee el primer archivo de configuración que se analiza y tiene
  valores válidos, y nunca combina archivos. El servidor de lenguaje ahora
  pasa al siguiente archivo cuando uno tiene un valor inválido, como hace la
  CLI. Cuando un archivo de mayor prioridad oculta a otro de menor prioridad
  que también tiene ajustes, cada ejecución imprime una advertencia que nombra
  el archivo que no se aplica, así que un proyecto con `complexipy.toml` y una
  sección `[tool.complexipy]` en `pyproject.toml` la ve hasta que deja sus
  ajustes en un solo archivo. Un archivo de configuración que no es un archivo
  regular, que pesa más de 4 MiB o que es un enlace simbólico que sale de su
  directorio se omite con una advertencia. Un error de configuración nombra
  el archivo y la línea y la columna, o la clave, en lugar de citar el
  archivo.
- Un archivo de configuración que define una `branch` en `[diff]`,
  `staged = true` en `[diff]` o `snapshot-create = true` imprime una
  advertencia en stderr en cada ejecución en la que se aplica el ajuste,
  incluso bajo `--quiet`. La advertencia nombra el archivo y explica cómo
  cambia el código de salida. Fija el control en CI con la bandera
  correspondiente, `--isolated` o `--config <ruta>`, y crea el snapshot con
  `--snapshot-create` en la línea de comandos en lugar de en el archivo.
- Cada función que define un módulo se puntúa por separado: un `def` bajo un
  `if`, `try`, `with`, `for`, `while` o `match` a nivel de módulo, y un método
  de una clase anidada, que se llama `Outer::Inner::method`. Estas funciones
  aparecen en todas las salidas y el control las verifica, así que una
  ejecución que pasaba puede fallar por una de ellas, y el total de un archivo
  puede cambiar. Una función definida dentro de otra función sigue contando
  para la función exterior.
- Una marca de ignorado cuenta solo dentro de un comentario: en la línea del
  `def`, en una línea de su firma o de sus decoradores, o en su propia línea
  justo encima. El mismo texto en una cadena, un docstring o un argumento de
  decorador, o en un comentario al final de la línea de código anterior, ya
  no suprime la función. Las palabras de la marca deben ser completas y estar
  separadas solo por espacios o tabuladores, así que `# complexipy: ignored`
  no es una marca. Mueve una marca así a un comentario en la línea del `def`.
- Un archivo fuente con una declaración de codificación PEP 263 se lee en esa
  codificación: UTF-8 y Latin-1 siempre, y una codificación compatible con
  ASCII como `cp1252` cuando el archivo es ASCII puro, así que un archivo
  Latin-1 con bytes no ASCII, que antes fallaba, ahora se analiza. Un archivo
  que declara otra codificación, como `utf-7`, o que declara una codificación
  compatible con ASCII y contiene bytes no ASCII, se reporta como fallido, y
  `file_complexity()` lanza `ValueError` para él.
- Las funciones de un mismo archivo que comparten nombre, como el getter y el
  setter de una propiedad, reciben entradas separadas en el snapshot, en
  `--diff` y en `compute_diff()`: la primera en orden de código fuente
  conserva el nombre, y las siguientes usan las claves `name#2`, `name#3`. Un
  snapshot escrito antes de este cambio puede fallar en un archivo así;
  ejecuta `--snapshot-create` una vez para reescribirlo.
- El descubrimiento de archivos selecciona un enlace simbólico `*.py` solo
  cuando apunta a un archivo regular dentro del directorio analizado. Un
  enlace que sale de él, un enlace roto, un dispositivo, un FIFO, un
  directorio llamado `*.py` y un directorio que no se puede leer se omiten,
  con una advertencia en stderr salvo con `--quiet`. Un archivo cuyo nombre no
  es UTF-8 válido se reporta como fallido en lugar de omitirse sin aviso. Una
  ejecución que no selecciona ningún archivo imprime una advertencia incluso
  bajo `--quiet`, y `--report-ignored` también lista los archivos `*.py` que
  omiten las reglas de `.gitignore`, `.ignore` y rutas ocultas.
- Un archivo fuente de más de 64 MiB se reporta como fallido en lugar de
  leerse, y `file_complexity()` lanza `ValueError` para él y para una ruta que
  no es un archivo regular, como un dispositivo. Un snapshot que no es un
  archivo regular de como máximo 64 MiB dentro de su propio directorio detiene
  la ejecución con un error, y un archivo de caché así se ignora. La caché por
  defecto no se usa cuando `.complexipy_cache` apunta fuera del directorio
  desde el que se ejecuta complexipy.
- `--fix` reescribe solo archivos regulares dentro del directorio desde el que
  se ejecuta complexipy. Omite un enlace simbólico, un archivo fuera de ese
  directorio y un archivo que cambió en disco después de que complexipy lo
  leyera, y lo indica en stderr. Escribe un archivo nuevo junto al original y
  lo renombra en su lugar, conservando los bits de permisos y el grupo, pero
  no setuid, setgid, los atributos extendidos ni las ACL. Los archivos de
  resultados, el snapshot y la caché se escriben de la misma forma, así que un
  enlace simbólico en una ruta de salida se reemplaza por un archivo regular
  en lugar de escribirse a través de él. `--fix --dry-run` no escribe nada, ni
  siquiera la caché o el snapshot.
- El texto que viene de los archivos analizados o de la configuración, como
  una ruta, un nombre de función, una línea de código o un valor de
  configuración, se imprime con sus caracteres de control y de formato
  bidireccional escapados: un carácter de escape se muestra como `\u{1b}` y un
  salto de línea como `\u{a}`. Los tabuladores se imprimen tal cual. Esto
  cubre el reporte, `--plain`, el diff de `--fix`, las advertencias y el log
  del servidor de lenguaje.
- La salida CSV escribe con un `'` inicial una celda que empieza con `=`,
  `+`, `-`, `@`, un tabulador o un retorno de carro, así que una hoja de
  cálculo la muestra como texto.
- Cuando la configuración tiene un problema, como un id de regla desconocido
  o un archivo de configuración oculto por otro, el servidor de lenguaje
  muestra en el editor una sola advertencia fija que remite al log de salida
  de complexipy, y escribe los detalles solo en ese log.
- La extensión de VS Code funciona en Modo Restringido en lugar de quedar
  deshabilitada allí. En Modo Restringido ignora un `complexipy.server.path`
  de la configuración del espacio de trabajo y ejecuta la ruta de tu
  configuración de usuario, o `complexipy` desde el `PATH`, y confiar en el
  espacio de trabajo reinicia el servidor con el valor del espacio de trabajo.
  Un `complexipy.server.path` relativo se resuelve contra la primera carpeta
  del espacio de trabajo, donde ahora arranca el servidor, y las entradas
  relativas del `PATH` se omiten.
- Los operadores booleanos suman uno por cada secuencia de operadores
  iguales, como especifica el artículo de complejidad cognitiva. Un grupo
  booleano anidado dentro de otro se comparaba con cada hermano de su padre,
  así que una expresión que mezcla operadores entre grupos puntuaba de más:
  `(a or b) and (c and d)` ahora puntúa 2 en lugar de 4, y
  `not a and (b or c)` puntúa 2 en lugar de 3. En un corpus de 1915
  funciones cambiaron 4 puntuaciones, cada una 1 menos.
- Las huellas (fingerprints) de GitLab Code Quality ya no incluyen el número
  de línea de la función, así que una línea añadida encima de una función
  conserva la huella de cada hallazgo debajo de ella. Una huella ahora se
  calcula a partir de la comprobación, la ruta, el nombre de la función y la
  posición del hallazgo entre los hallazgos que comparten esos datos. Todas
  las huellas existentes cambian una vez con esta versión, así que la primera
  comparación con un reporte de una versión anterior muestra cada hallazgo
  como nuevo y cada hallazgo anterior como corregido.
- Los reportes SARIF codifican `artifactLocation.uri` con porcentajes, así
  que una ruta con espacios, comillas, caracteres no ASCII, `#`, `?` o `%`
  apunta al archivo correcto. `$schema` nombra el esquema SARIF 2.1.0
  publicado por OASIS en lugar de una URL que devolvía 404. Cada resultado
  lleva `partialFingerprints`, que no cambian cuando se añaden líneas encima
  del hallazgo, así que las subidas mediante una API de code scanning no
  duplican alertas.
- `--quiet --ignore-complexity` termina con código 0, igual que
  `--ignore-complexity` solo; antes terminaba con 1 cuando una función
  superaba el límite. Cuando stdout se cierra antes de tiempo, como en
  `complexipy . | head`, la salida se detiene y la ejecución conserva su
  código de salida, en lugar de producir un panic y terminar con el código
  101 desde el binario o 1 desde el script de consola. El servidor de
  lenguaje responde a un mensaje que no es JSON ni JSON-RPC válido con una
  respuesta de error y sigue funcionando, en lugar de terminar con 1.
- Un patrón de `exclude` se compara con la ruta relativa a cada argumento de
  ruta, como antes, y también con la ruta relativa al directorio de trabajo,
  así que la CLI y el servidor de lenguaje excluyen los mismos archivos. Con
  `paths = ["src", "legacy"]` y `exclude = ["legacy/**"]`, antes excluían
  archivos opuestos. Una ejecución de la CLI cuyo argumento de ruta difiere
  del directorio de trabajo puede excluir más archivos que antes, y nunca
  menos. El servidor también compara cada patrón con la ruta relativa a cada
  entrada de `paths`, resuelve `..` en las rutas de los documentos antes de
  comparar, y da a una carpeta anidada del espacio de trabajo sin archivo de
  configuración propio la configuración de la carpeta contenedora más
  cercana que tenga uno. Un patrón que empieza por `/`, `./` o `../`, o que
  termina en `/`, nunca coincide, y la CLI y el servidor ahora avisan de
  ello. Un archivo nombrado en la línea de comandos se sigue analizando
  aunque un patrón coincida con él, como en ruff. Una opción
  `--force-exclude` que también excluya esos archivos se sigue en
  [#293](https://github.com/rohaquinlop/complexipy/issues/293).
- C002 ofrece un reemplazo de guardas de bucle solo cuando la cadena de `if`
  anidados es la última sentencia del cuerpo del bucle y cada `if` es la
  última sentencia de su padre. Cuando hay sentencias después de la cadena,
  el plan lleva texto de ayuda en su lugar, porque una guarda `continue` las
  omitiría: antes `--fix` escribía esa guarda, que podía convertir en un
  bucle infinito un bucle `while` cuyo contador seguía a la cadena. C007
  mueve un único comentario de las líneas `if` fusionadas, como
  `# type: ignore`, a la línea fusionada, y da texto de ayuda cuando esas
  líneas llevan dos o más comentarios o cuando el `if` interior tiene su
  cuerpo en la línea de cabecera.
- `--report-ignored`, `complexipy-ignored.json` y el reporte de comentarios
  de ignorado obsoletos muestran un archivo dentro del directorio de trabajo
  en la misma ruta que los resultados. Antes mostraban las rutas relativas al
  padre de cada directorio analizado, o al propio directorio de un archivo
  pasado como argumento, así que `complexipy .` anteponía a cada ruta el
  nombre del directorio de trabajo. El `file_name` de cada resultado ahora
  coincide con su `path`, también para un archivo alcanzado mediante un
  enlace simbólico.
- Un error interno en la API de Python lanza `ValueError` desde las
  funciones de análisis, de diff y de recopilación de ignorados, y
  `RuntimeError` desde `run_cli` y `run_lsp`, en lugar de
  `pyo3_runtime.PanicException`, que deriva de `BaseException`.
  `except Exception` ahora lo captura, así que un archivo que provoca un
  error interno ya no interrumpe un lote. Los stubs y la documentación ahora
  nombran los errores que lanza cada función.

### Corregido

- `--diff` y `--staged` leen una ruta en la referencia solo cuando allí era
  un archivo regular. Una ruta que era un enlace simbólico en la referencia y
  ahora es un archivo regular se compara como añadida en lugar de contra el
  texto de destino del enlace, así que sus funciones por encima del umbral
  hacen fallar el control. `--staged` además compara un cambio staged entre
  un enlace simbólico y un archivo regular, que antes omitía.
- `--staged` lee un archivo staged cuyo nombre empieza como un número de
  etapa, como `0:a.py`, en lugar de leer en su lugar el `a.py` staged.
- `branch = ""` en la sección `[diff]` desactiva el diff, y con él el valor
  `staged` de la sección, como dice la documentación, así que la ejecución
  verifica cada función contra el umbral. Antes comparaba contra el índice,
  así que el código de salida solo verificaba las funciones que cambiaban.
- `--diff` y `compute_diff()` no leen ninguna versión anterior de una ruta
  cuyo nombre contiene un carácter de control, así que sus funciones se
  reportan como NEW. Un nombre que terminaba en un retorno de carro leía la
  versión anterior del mismo nombre sin él.
- Un CR (`\r`) aislado y CRLF ahora terminan una línea en todas partes, como
  en el tokenizador y en LSP 3.17. Un archivo con finales de línea CR ya no
  produce un panic al construir los planes de refactorización ni al dibujar
  los acentos circunflejos bajo una sugerencia, ya no reporta todas las
  funciones en la línea 1, ya no deja que un comentario de ignorado suprima
  el archivo entero y ya no tarda un tiempo cuadrático ni genera una salida
  cuadrática. `--fix` conserva los finales de línea CR de un archivo, y la
  columna de los acentos circunflejos bajo una sugerencia cuenta caracteres,
  no bytes.
- Una expresión encadenada larga, como `a + a + ... + a`, `a.b.b...`,
  `f()()...` o `a[0][0]...`, ya no desborda la pila, lo que abortaba la CLI
  y el servidor de lenguaje y hacía fallar el intérprete de Python. Una
  cadena de 300.000 términos ahora se analiza en la CLI, la API de Python, el
  servidor de lenguaje y la demo web.
- Puntuar una expresión booleana larga que mezcla operadores, y construir
  los planes de refactorización de una cadena de `if` profunda, ya no tarda
  un tiempo cuadrático.
- `--fix` conserva lo que hace el código reescrito. Un `else` escrito como
  `else :`, o uno después de una condición que contiene una serie de
  comillas, ya no se pasa por alto. `a or(b)` y `not a or(b)` se parentizan e
  invierten correctamente. Cada línea conserva su propia sangría, así que un
  archivo que mezcla tabulaciones y espacios sigue siendo válido, y un
  archivo sangrado con tabulaciones recibe reemplazos sangrados con
  tabulaciones. Un `if` interior con su cuerpo en la línea de cabecera ya no
  pierde ese cuerpo, C002 ya no reescribe una cadena dentro de una función
  anidada, y un `#` dentro de una cadena de texto ya no oculta una cadena
  multilínea posterior a la comprobación que rechaza la reescritura.
- Un archivo alcanzado mediante varios argumentos de ruta, como
  `complexipy . src src/a.py`, o mediante un enlace simbólico a su lado, se
  analiza, se reporta y se corrige una sola vez.
- `--report-ignored` lista cada ubicación suprimida como
  `path:line  # comment`, seguida del recuento, como describen su texto de
  ayuda y la documentación. Antes imprimía solo el recuento.
- Los fragmentos de las sugerencias respetan `--color no`, `NO_COLOR` y la
  salida por tubería, y un comentario en un fragmento ya no colorea todas
  las líneas que le siguen. El color se decide en cada ejecución, así que un
  proceso de Python que llama a `run_cli` varias veces sigue la terminal y
  `NO_COLOR` de cada ejecución.
- Una referencia de `--diff` más ancha que la terminal ya no produce un
  panic al dibujar el banner.
- `compute_diff(current, "origin/main")` funciona como está documentado:
  `invocation_path` vale `None` por defecto. Ctrl-C detiene el script de
  consola de inmediato en lugar de al terminar la ejecución. Un proceso
  creado con fork después de una llamada en paralelo ya no se bloquea en su
  siguiente llamada en paralelo. Los stubs y la documentación eliminan el
  inexistente `DiffEntry.delta`, corrigen un ejemplo de `exclude` que no
  coincidía con nada y muestran el formato real de una entrada de snapshot.
- El servidor de lenguaje lee cada trama a medida que llega, hasta 256 MiB,
  así que un `Content-Length` enorme ya no hace que reserve ese tamaño ni que
  aborte, y una trama rota se reporta en stderr antes de que el servidor se
  detenga. Una solicitud después de `shutdown`, o una cuyo id no es ni un
  entero ni una cadena, recibe un error `InvalidRequest` en lugar de no
  recibir respuesta. Tras un handshake fallido, el servidor termina en lugar
  de esperar a que se cierre stdin. `file://localhost/...` se resuelve a la
  ruta local, y una URI con un host remoto se resuelve a una ruta UNC en
  Windows y a ninguna ruta en otros sistemas, así que el servidor lee la
  configuración correcta. El hover muestra un nombre de función como
  `Box::__init__` literalmente en lugar de interpretarlo como Markdown.
- La extensión de VS Code detiene un servidor que no arranca en 30 segundos
  y muestra un error, así que un reinicio posterior ya no queda bloqueado.
  Mantiene un solo canal de salida en lugar de añadir uno por reinicio.
- La demo web carga un analizador nuevo después de que un análisis falle, en
  lugar de fallar en cada análisis posterior hasta recargar la página.

## [8.1.0] - 2026-10-18

### Añadido

- Un servidor de lenguaje: `complexipy lsp` habla el Language Server Protocol
  por stdio, así que cualquier editor con cliente LSP obtiene complejidad sin
  código específico por editor. Responde a las peticiones de inlay hints con la
  complejidad cognitiva de cada función y, opcionalmente, con el incremento que
  aporta cada línea; responde al hover con el total de la función, su estado
  respecto al umbral y la mejor sugerencia de refactorización, que se calcula
  bajo demanda para la función bajo el cursor, así que los archivos grandes
  siguen respondiendo; y publica una
  advertencia por cada función por encima de `max-complexity-allowed`. Los
  diagnósticos respetan los comentarios de ignorado en línea, así que el editor
  y la CLI coinciden. El hint por función se coloca al final de la línea que
  cierra la declaración, así que una firma repartida en varias líneas conserva
  el hint en su línea de cierre, y un documento que no se puede analizar
  conserva los últimos hints analizados más una advertencia
  `complexipy-parse-error`, situada donde falla el analizador, hasta que vuelve
  a analizarse. Los problemas de configuración, como un id de regla desconocido
  en `select`, aparecen en el editor como un mensaje de advertencia
  ([#272](https://github.com/rohaquinlop/complexipy/issues/272)). El servidor
  vuelve a cargar la configuración cuando cambia un archivo de configuración en
  la raíz de una carpeta, en los editores que vigilan archivos para él
  ([#257](https://github.com/rohaquinlop/complexipy/issues/257)). Una
  extensión de Zed en `zed/` arranca el servidor en Zed, y los inlay hints
  llevan un margen a la izquierda para no tocar el código
  ([#255](https://github.com/rohaquinlop/complexipy/issues/255)). Consulta
  [Integración con Editores](https://complexipy.com/es/editors/) para la
  configuración de VS Code, Neovim y Zed.

- La extensión de VS Code ahora es un cliente del servidor de lenguaje y
  elimina su motor WASM empaquetado
  ([#253](https://github.com/rohaquinlop/complexipy/issues/253)). Los
  resultados se actualizan mientras escribes, sin guardar
  ([#133](https://github.com/rohaquinlop/complexipy/issues/133)), y los hints
  y las advertencias siguen `max-complexity-allowed` y la configuración de
  `[tool.complexipy.lsp]`
  ([#127](https://github.com/rohaquinlop/complexipy/issues/127)). La
  extensión necesita el comando `complexipy`, y la opción
  `complexipy.server.path` la apunta a un binario concreto.

- La extensión de VS Code se publica en OpenVSX además de en el Visual Studio
  Marketplace, así que VSCodium y otros forks de VS Code pueden instalarla
  ([#145](https://github.com/rohaquinlop/complexipy/issues/145)). Una etiqueta
  `vscode-v*` publica el mismo VSIX en ambos registros.

- Una sección de configuración `[tool.complexipy.lsp]`, con `inlay-hints`
  (`threshold`, `always` o `never`), `per-line-hints` y `diagnostics`. Las
  pistas se filtran por `max-complexity-allowed` por defecto, así que un
  proyecto sin opciones recibe información solo donde importa.

- `complexipy-core` incorpora una feature `config` que posee el descubrimiento
  de archivos de configuración, para que la CLI y el servidor lean
  `complexipy.toml`, `.complexipy.toml` y `pyproject.toml` por un único camino.

- El servidor aplica `exclude` patrón a patrón: un patrón mal formado se
  reporta una vez por carga de configuración y se omite mientras el resto siguen
  aplicándose, y una lista demasiado grande para compilarse como un solo
  programa se reporta una vez y se sigue aplicando. Una ejecución de la CLI se
  detiene con cualquiera de los dos.

- Cada carpeta del espacio de trabajo lee su propia configuración
  ([#273](https://github.com/rohaquinlop/complexipy/issues/273)). En un
  espacio de trabajo con varias carpetas, un documento usa el umbral, la
  selección de reglas, los patrones de `exclude` y los ajustes de
  `[tool.complexipy.lsp]` de la carpeta más profunda que lo contiene, y un
  documento fuera de todas las carpetas usa la primera. El servidor declara
  soporte de carpetas del espacio de trabajo, así que las carpetas que el editor
  añade o quita se aplican sin reiniciar.

- Supresión por regla y selección de reglas
  ([#208](https://github.com/rohaquinlop/complexipy/issues/208)). `--select` y
  `--ignore` (repetibles y con listas separadas por comas) eligen las reglas de
  refactorización activas, y `ignore` gana en caso de conflicto.
  `complexipy.toml`, `.complexipy.toml` y `pyproject.toml` aceptan las claves
  `select` e `ignore`, y una flag reemplaza la clave toml que nombra. Un valor
  toml se separa por comas como las flags, así que `select = "C007, C002"`
  nombra dos reglas. Un
  comentario de ignorado acepta una lista de reglas entre corchetes,
  `# complexipy: ignore[C007]`, que suprime solo las reglas nombradas para una
  función, mientras que un marcador sin lista sigue suprimiendo la función
  completa. Una regla inactiva desaparece de todos los formatos de salida y
  nunca ocupa una plaza del tope de planes por función, así que una regla
  oculta nunca puede bloquear una visible. Un id de regla desconocido avisa en
  stderr de que no coincide con ninguna regla registrada, y un segundo aviso
  lo indica cuando `select` e `ignore` no dejan ninguna regla activa. La selección de reglas nunca cambia el código de salida.

- `--fix` aplica sugerencias aplicables por máquina a los archivos fuente
  ([#204](https://github.com/rohaquinlop/complexipy/issues/204)). Una
  sugerencia es aplicable cuando su nivel es `MachineApplicable` y su
  reemplazo es un empalme fiel (`spliceable`), lo que cubre C002 (guardas de
  bucle) y C007 (if plegable); C005 no es empalmable y queda fuera. `--fix`
  solo cambia funciones cuya complejidad supera `max-complexity-allowed`, y
  cada paso vuelve a comprobar el umbral, así que una función deja de cambiar
  en cuanto lo cumple. Las funciones en el umbral o por debajo conservan sus
  sugerencias en el informe y nunca se reescriben. Una
  ejecución corrige cada archivo analizado, de abajo a arriba dentro de cada
  archivo, e imprime un diff unificado con resaltado de sintaxis de los
  cambios aplicados, con números de línea, una línea de contexto por bloque
  y la regla que se activó, más un resumen de las correcciones aplicadas y
  omitidas que muestra la reducción de cada corrección. Cada línea del
  resumen lleva los números de línea del archivo anteriores a la ejecución,
  y cada omisión se informa una vez. La ejecución repite
  paso a paso, hasta 8 pasos, hasta que no quede ningún plan aplicable, así
  que una corrección que expone un nuevo plan aplicable se resuelve en un
  paso posterior; los pasos posteriores etiquetan sus diffs. Las filas del
  diff muy largas se recortan al ancho de la terminal. Cuando no hay nada
  aplicable, la ejecución imprime `No fixes to apply.` `--fix` se ejecuta
  en cualquier estado de git: los cambios sin confirmar, los archivos sin
  rastrear y la ausencia de repositorio nunca lo bloquean, y no hace falta
  ninguna bandera extra. Una corrección conserva los finales de línea del
  archivo y su salto de línea final. Una corrección que
  produciría Python inválido se revierte y se informa, y la escritura usa un
  archivo temporal con comprobación de análisis que se renombra sobre el
  original conservando sus permisos. El informe y los controles miden el
  estado posterior a las correcciones, así que reanalizar un archivo corregido
  muestra que el hallazgo desapareció y la reducción medida exacta.
  `--fix --dry-run` imprime solo el diff y no escribe nada. `--select`/`--ignore`
  deciden qué reglas pueden corregir.

- Soporte de Python 3.15
  ([#265](https://github.com/rohaquinlop/complexipy/issues/265)). Las
  importaciones diferidas (PEP 810) y el desempaquetado en comprensiones
  (PEP 798) ahora se analizan en la CLI, el servidor de lenguaje, la API de
  Python y la versión web. Una importación diferida no suma complejidad, y
  una comprensión con desempaquetado puntúa igual que la simple. Las
  versiones publican ahora wheels de CPython 3.15 para Windows y macOS.

- Un esquema JSON para la configuración
  ([#270](https://github.com/rohaquinlop/complexipy/issues/270)).
  `complexipy.schema.json` describe cada clave de `complexipy.toml`,
  `.complexipy.toml` y `[tool.complexipy]`, incluidos `[diff]` y `[lsp]`, así
  que un servidor de lenguaje TOML como Taplo ofrece autocompletado, texto al
  pasar el cursor y validación. El esquema rechaza las claves desconocidas.
  complexipy las sigue ignorando.

### Cambiado

- `lsp` es un primer argumento reservado: `complexipy lsp` arranca el servidor
  de lenguaje, así que un archivo o directorio con ese nombre necesita
  `complexipy -- lsp` para analizarse. En una terminal, el servidor imprime esa
  pista cuando están presentes la palabra reservada y una ruta llamada `lsp`.
- `StringOrList` se movió de `complexipy-cli::types` al nuevo módulo `config`
  de `complexipy-core`, y la CLI lo sigue reexportando desde su ruta anterior.
  Todos los crates del workspace siguen siendo `publish = false`, así que no
  cambia ninguna API pública de Rust.
- Se añadieron builds de wheels `manylinux_2_31_riscv64` para soporte de
  RISC-V. La matriz de release ahora separa el triple del toolchain de Rust
  (`target`) del nombre corto de la plataforma (`arch`), lo que mantiene sin
  cambios los nombres de wheels y artefactos existentes. (#259)
- `RuleCategory`, `Applicability` y `DiffStatus` son auténticas subclases de
  `enum.Enum`: se pueden iterar, exponen `.name` y `.value`, y
  `isinstance(x, enum.Enum)` se cumple.
- `requires-python` ahora es `>=3.8,<3.16`, así que los instaladores dejan de
  elegir complexipy en una versión de Python que ningún release ha compilado ni
  probado. El límite sube cuando un release añade la siguiente versión de
  Python. En Python 3.16 o posterior, pip recurre a 8.0.1, el último release
  sin el límite.
  ([#281](https://github.com/rohaquinlop/complexipy/issues/281))

### Corregido

- `--color` controla cada línea con estilo que escribe la CLI, en stdout y en
  stderr. `auto` desactiva el color cuando el flujo no es una terminal o
  cuando `NO_COLOR` está definido, `no` lo quita del informe, del banner y de
  los avisos, y la salida `--plain` nunca lleva estilo. En 8.0.1 el informe
  llevaba códigos ANSI en una tubería, con `--color no` y con `NO_COLOR`.

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/8.1.0)
para todos los detalles.

## [8.0.1] - 2026-09-06

### Cambiado

- La documentación, los metadatos del paquete y los enlaces embebidos (URLs
  de reglas en la salida de `--suggest-refactors`, URIs del reporte SARIF)
  ahora apuntan al dominio propio `complexipy.com` en lugar del dominio
  predeterminado de GitHub Pages. (#252)

### Corregido

- El sdist ahora incluye el archivo `LICENSE`, que anteriormente faltaba en
  la distribución de código fuente publicada.

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/8.0.1)
para todos los detalles.

## [8.0.0] - 2026-09-03

!!! note "Migration"

    Consulta la [guía de migración](https://rohaquinlop.github.io/complexipy/es/migracion/)
    para los cambios incompatibles descritos abajo.

### Cambiado

- La CLI de Python se retira: `complexipy` ahora es una implementación
  nativa en Rust expuesta a través de un shim de console-script delgado,
  de modo que todo el pipeline se ejecuta sin intérprete de Python. La
  API de Python no cambia y ahora incluye la comparación de diffs
  (`compute_diff`, `has_regressions`, `DiffEntry`, `DiffStatus`)
  respaldada por el mismo motor Rust que la CLI. El módulo de extensión
  se sigue distribuyendo vía wheels de maturin.
  ([#224](https://github.com/rohaquinlop/complexipy/issues/224), [#243](https://github.com/rohaquinlop/complexipy/issues/243))

### Eliminado

- **Análisis por URL de Git** (`complexipy <repository-url>`) - solo rutas
  locales.
- **Migración de caché heredada** - el diseño anterior
  `.complexipy_cache/<hash>.json` ya no se migra; los archivos heredados
  existentes se ignoran y el historial de diferencias comienza vacío.
- **Archivos de snapshot heredados** - los snapshots creados con versiones
  antiguas ya no se detectan; vuelve a crearlos con `--snapshot-create`.
- **Flag corta `-mx`** para `--max-complexity-allowed` - venía de la CLI de
  Python pero nunca se portó al parser basado en clap; usa
  `--max-complexity-allowed` en su lugar.

### Corregido

- La regla C007 (collapsible-if) ya no sugiere fusionar un `if` anidado
  cuando una sentencia con efectos secundarios del mismo nivel la
  precede - la fusión cambiaría el comportamiento del programa. Los
  comentarios en cabeceras multilínea y los comentarios finales se
  manejan en lugar de poder ser desplazados por el reemplazo.
  ([#228](https://github.com/rohaquinlop/complexipy/issues/228), [#236](https://github.com/rohaquinlop/complexipy/issues/236))
- El guardián de la regla C007 ya no falla cuando hay líneas en blanco o
  solo comentarios entre el `if` exterior y el interior: la detección del
  paso de sangría ahora omite esas líneas, por lo que la fusión se sigue
  rechazando en lugar de producir un reemplazo que elimina código.
  ([#245](https://github.com/rohaquinlop/complexipy/issues/245))
- La sugerencia de guards de bucle C002 conserva las sentencias que quedan
  entre los `if` encadenados: ahora aparecen en el reemplazo entre los
  guards correspondientes en lugar de eliminarse. Las condiciones de guarda
  ahora se parentizan (`if not (<cond>):`), de modo que invertir condiciones
  con `and` u `or` ya no cambia silenciosamente lo que omite la guarda.
- La sugerencia de extracción de predicado C005 conserva la palabra clave
  de la sentencia: las condiciones `while` siguen siendo bucles `while`, y
  las condiciones `elif` reciben solo texto de ayuda en lugar de un
  reemplazo `if` independiente inválido. La sangría del cuerpo del
  predicado ahora sigue el paso de sangría del archivo.
- C002 y C007 rechazan sugerencias de máquina cuando el cuerpo desplazado
  contiene un literal de cadena multilínea: quitar sangría cambiaría su
  valor. Los planes llevan texto de ayuda en su lugar.
- Los guards de bucle C002 eliminan un `not` redundante de la condición
  (`if not a:` se convierte en guard `if a:`), y los nombres de predicado
  de C005 reciben un sufijo de guion bajo cuando el código fuente ya define
  el nombre generado.
- La extracción de predicado C005 ahora emite un helper a nivel de módulo
  cuyos parámetros son las variables libres de la condición (bases de
  atributos incluidas, builtins excluidas), de modo que el helper es
  testeable por unidad. El fragmento muestra el contexto que lo rodea en la
  indentación real de la sentencia. Las condiciones que vinculan un nombre
  (`:=`) o contienen un lambda/comprensión reciben solo texto de ayuda.

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/8.0.0)
para todos los detalles.

## [7.0.1] - 2026-08-12

### Cambiado

- El pipeline de release ahora reintenta la subida de artefactos con
  versiones de workflow fijadas y solo notifica a los repositorios
  downstream en releases con tag, manteniendo silenciosos los pushes
  sin tag.
- Se fijó la versión de maturin en el workflow de release para evitar
  límites de tasa de la API de GitHub al resolver la última versión
  dentro de los contenedores Docker de build.

### Corregido

- Las entradas de snapshot de los archivos analizados se actualizan en su
  posición en lugar de moverse al final del archivo: las ejecuciones
  parciales (p. ej. hooks de pre-commit que analizan solo los archivos
  staged) ya no reordenan el snapshot, por lo que el snapshot permanece
  byte-idéntico entre commits que no cambian la complejidad. ([#226](https://github.com/rohaquinlop/complexipy/issues/226))

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/7.0.1)
para todos los detalles.

## [7.0.0] - 2026-08-10

!!! note "Migración"

    Las flags `--output-json`, `--output-csv`, `--output-gitlab`,
    `--output-sarif` y `--ratchet`, ya deprecadas, y sus claves TOML se
    eliminaron; usa `--output-format` y `--diff` en su lugar. Consulta la
    [guía de migración](https://rohaquinlop.github.io/complexipy/es/migracion/)
    para cada flag y clave eliminada con su reemplazo.

### Añadido

- `--suggest-refactors` ahora es un sistema de lint estilo clippy: IDs de
  reglas estables (C001-C005, C007, C011), metadatos de categoría y
  aplicabilidad, anclas `path:line:col` con span de caret, renderizado
  verbatim de las sugerencias y un enlace de documentación por regla. (#209)
- Los hallazgos de refactorización se incluyen en las exportaciones JSON,
  SARIF y GitLab cuando se pasa `--suggest-refactors`; el catálogo de
  reglas SARIF se construye dinámicamente a partir de los planes
  encontrados. (#209)
- `compute_diff`, `has_regressions`, `DiffEntry` y `DiffStatus` ahora
  forman parte de la API pública de Python, de modo que las herramientas de
  CI pueden consumir los resultados del diff como objetos en lugar de
  parsear la salida del terminal; `DiffStatus` proporciona constantes con
  nombre como `DiffStatus.REGRESSED`. (#210)
- `collect_removable_ignored_locations()` y `RemovableIgnore` se exportan
  desde la API de Python. Cada ejecución informa ahora de los comentarios
  de ignorado que ya no son necesarios (`path:line function=X complexity=N <comment>`) cuando la función suprimida vuelve a estar bajo el límite
  permitido - el código de salida no se ve afectado, el informe se suprime
  con `--quiet` y funciona con `--plain`. (#213)
- Flag `--staged` para la comparación del índice git - responde "¿qué
  complejidad estoy a punto de commitear?" en lugar de solo lo que cambió
  en el working tree; `--staged` solo usa `HEAD` como baseline por defecto
  y aplica el umbral, mientras que `--diff <ref> --staged` aplica contra la
  ref. (#218)
- Sección TOML `[tool.complexipy.diff]` para que la política de comparación
  viva en la configuración del repositorio: `branch = "main"` hace que una
  ejecución simple de `complexipy .` se comporte como `--diff main` (con
  aplicación del umbral), `staged = true` habilita la comparación staged
  por defecto y `branch = ""` la desactiva. Las flags de CLI tienen
  precedencia sobre la sección. (#219)
- Las reducciones de refactorización ahora se miden en lugar de estimarse:
  para cada sugerencia aplicable por máquina (C002, C007) el reemplazo se
  inserta en el código, se re-analiza y se vuelve a puntuar, de modo que
  `estimated_reduction` y `estimated_complexity_after` reportan el delta
  literal de aplicar la sugerencia - y el ranking, la resolución de
  solapamientos y el filtro de ruido operan con valores medidos. El nuevo
  flag `reduction_is_measured` en `RefactorPlan` separa los planes medidos
  de las estimaciones por fórmula solo con ayuda, que la CLI muestra con
  un calificador `~` (`Estimated reduction: ~-2`) mientras que los planes
  medidos se muestran sin él (`Reduction: -2`). Las sugerencias de guardas
  ahora son splices fieles para bucles con sentencias antes o después de la
  cadena de ifs y para cabeceras de bucle multilínea; los fallos de
  medición caen a la estimación por fórmula - nunca un panic, nunca un
  número fabricado. (#225)

### Cambiado

- Las estimaciones de reducción de los planes de refactorización ahora son
  honestas: la matemática de reducción se reescribió y validó contra la
  complejidad medida antes/después, C004 ya no sugiere dividir sentencias
  `match`, C006 se eliminó porque su gate nunca podía dispararse y C011
  ahora se dispara en cadenas `try` → `with` → `try`. Los planes
  solapados se deduplican contra cada solapamiento y se limitan a 5,
  informando los descartados como "... and N more suggestions". (#209)
- La extracción de condiciones en las reglas de refactorización ahora
  rastrea la profundidad de corchetes, los literales de cadena y el walrus
  `:=` en lugar de un `rfind(':')` ingenuo. (#209)
- `file_complexity()` ahora devuelve rutas relativas al cwd (igual que
  `git diff --name-only`), y las invocaciones anidadas resuelven las rutas
  git mediante una búsqueda por basename con `git ls-files` - sin esto,
  comparar los resultados por archivo marcaba silenciosamente cada función
  como NEW. (#210)
- Se añadieron archivos de estándares comunitarios: `CODE_OF_CONDUCT.md`,
  `CONTRIBUTING.md`, `SECURITY.md`, plantillas de issue y una plantilla de
  pull request. (#201)

### Corregido

- Las actualizaciones de snapshot ahora se fusionan con el snapshot
  existente en lugar de reemplazarlo: solo se tocan los archivos analizados
  en la ejecución, de modo que las ejecuciones parciales (p. ej. hooks de
  pre-commit que analizan solo los archivos staged) ya no borran el
  baseline de los archivos no analizados. (#215)
- El footer de los docs ahora renderiza sus enlaces como enlaces con estilo
  en lugar de markdown crudo, tanto en la página de inicio en inglés como
  en español. (#216)

### Eliminado

- `RefactorPlan.steps` - reemplazado por los campos concretos `suggestion`
  / `help` del plan. (#209)
- `CodeSnippet` de la API pública; `CodeSuggestion` se exporta en su
  lugar. (#209)
- `--output-json` / `-j`, `--output-csv` / `-c`, `--output-gitlab` y
  `--output-sarif` / `-sr` - usa `--output-format` en su lugar. (#221)
- `--ratchet` / `-R` - `--diff` aplica el umbral por defecto. (#221)
- Las claves TOML `output-json`, `output-csv`, `output-gitlab`,
  `output-sarif`, `staged` plana, `ratchet` plana y el alias no documentado
  `details = "low"`. (#221)

Estas eliminaciones llegan en 7.0.0. Consulta la
[guía de migración](https://rohaquinlop.github.io/complexipy/es/migracion/)
para cada flag y clave eliminada con su reemplazo.

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/7.0.0)
para todos los detalles.

## [6.2.0] - 2026-07-23

### Añadido

- `--exclude` y `--output-format` ahora aceptan valores separados por comas
  (`--exclude tests/**,src/**`), evitando problemas de expansión del
  shell. (#199)

### Corregido

- La salida ahora muestra la ruta relativa correcta desde el directorio de
  trabajo al analizar un archivo o directorio. (#198)
- El texto de ayuda de `--output-sarif` ahora incluye un aviso de
  deprecación, aclarando que la salida SARIF pasa a otro mecanismo. (#199)

### Cambiado

- Se descompuso la función monolítica `main()` de 316 líneas en un
  orquestador limpio (~80 líneas) con la lógica de negocio extraída en
  módulos `utils/` específicos (`config.py`, `paths.py`, `ignored.py`). (#199)
- Se introdujeron los dataclasses `RunConfig`, `ExitReport` y
  `SnapshotEvaluation`, reemplazando los acumuladores mutables y el
  desempaquetado de tuplas posicionales. (#199)
- Se eliminaron todas las declaraciones `global console`: la instancia de
  `console` se construye una vez y se pasa explícitamente. (#199)
- Se refactorizó `get_arguments_value`, de una función de 20 parámetros que
  devolvía una tupla de 15 elementos, a un enfoque basado en diccionarios.
  (#199)
- Se añadieron 30 tests nuevos que cubren la resolución de configuración y
  la lógica de evaluación de snapshots. (#199)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/6.2.0)
para más detalles.

## [6.1.0] - 2026-07-21

### Corregido

- Se corrigió la resolución de rutas git al ejecutar `--diff` desde un
  subdirectorio anidado dentro de un repositorio, que antes provocaba
  errores de "not a git repository". (#196)
- Se corrigió la configuración de lint de ruff para usar un patrón glob en
  lugar de una exclusión de directorio para los archivos de test. (#196)

### Cambiado

- Se simplificó el CLI de diff - `--diff` ahora siempre aplica
  `--max-complexity-allowed`, mientras que `--diff-only` ofrece la
  comparación solo visual. `--ratchet` queda deprecado en favor de este
  modelo. (#196)
- Se integró la salida de diff en el flujo principal de análisis en lugar de
  producirla como un paso de post-procesamiento separado. (#196)
- Se eliminó la sección redundante de "Failed functions" - las funciones que
  fallan ahora se muestran en línea en el resumen por archivo. (#196)
- Se añadió `AGENTS.md` para el contexto de asistentes de IA.
- Se añadió un workflow de CI que notifica a los repositorios downstream en
  cada nueva versión. (#197)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/6.1.0)
para más detalles.

## [6.0.1] - 2026-07-03

### Corregido

- Se normalizaron las rutas de Windows para la compatibilidad con los globs
  de wax en el manejo de exclusiones. (#194)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/6.0.1)
para más detalles.

## [6.0.0] - 2026-06-26

!!! note "Migración"

    Esta versión alinea el algoritmo de complejidad cognitiva con el white
    paper de SonarSource (v1.7). Las puntuaciones cambian para archivos que
    usan `match`, `try`/`except`, `with`, comprehensions, lambdas,
    recursión o ternarios anidados - en la mayoría de los casos aumentan.
    Vuelve a ejecutar `complexipy` tras actualizar y revisa las nuevas
    puntuaciones; si usas `--max-complexity` / `--failed` en CI,
    probablemente necesitarás subir los umbrales. Consulta la
    [guía de migración](https://rohaquinlop.github.io/complexipy/es/migracion/)
    para obtener orientación sobre la actualización.

### Corregido

- Las sentencias `match` ahora aplican un incremento estructural + de
  anidamiento según el paper, en lugar de puntuarse como 0. (#192)
- Los bloques `try`/`else`/`finally` ahora se recogen en el nivel de
  anidamiento actual en lugar de `+1`, según la regla del paper. (#192)
- Los handlers `except` ahora cobran `1 + nesting_level` en lugar de un
  `+1` fijo. (#192)
- Los bloques `with` ya no elevan incorrectamente el nivel de
  anidamiento. (#192)
- La recursión directa ahora emite `+1` por cada auto-llamada mediante un
  `RecursionFinder` consciente del ámbito, que omite correctamente
  definiciones anidadas de funciones/clases y lambdas. (#192)
- Las expresiones lambda ahora se incluyen en el conteo de operadores
  booleanos, recursando en el cuerpo con `nesting_level + 1`. (#192)
- Las comprehensions (`ListComp`, `SetComp`, `DictComp`, `Generator`) ahora
  cobran `1 + nesting_level` por generador y `+1` por filtro `if`. (#192)
- Los ternarios anidados ahora recursan con `nesting_level + 1`, de modo que
  los ternarios internos reciben el incremento correcto. (#192)
- Las cláusulas `else` de `for`/`while` ahora se recogen en el nivel de
  anidamiento del propio bucle, no en `+1`. (#192)
- Las sentencias de expresión sueltas ahora cuentan operadores booleanos
  (p. ej. `foo(a and b)`). (#192)
- El iterable de `for` ahora cuenta operadores booleanos, en paridad con
  `while`. (#192)

### Cambiado

- Se extrajo toda la orquestación de rutas (I/O de archivos, recorrido de
  directorios, clonado de URLs) de `cognitive_complexity.rs` a un nuevo
  módulo `src/runner.rs`, con `#[cfg(feature = "python")]`, lo que también
  resuelve un fallo de compilación wasm preexistente. (#192)
- Se extrajeron los helpers `push_line`, `absorb`, `absorb_with_regions`,
  `finalize_region`, `count_line_bool_ops`, `loop_complexity`, `is_ignored`
  y `analyze_function` para eliminar el patrón repetido de fold/push en
  todos los brazos de sentencias. (#192)
- Se unificó el manejo estructuralmente idéntico de `Stmt::For` y
  `Stmt::While` en una única llamada `loop_complexity`. (#192)
- Se colapsaron 7 brazos de sentencias casi idénticos mediante
  `count_line_bool_ops`. (#192)
- Se derivó `Default` en `ComplexityRegion` y `RegionKind`. (#192)
- Se eliminó `merge_child` y su `child.regions.clone()` interno. (#192)
- Se endureció el job de tests unitarios de release en Windows contra fallos
  transitorios de la capa HTTP/2 de Cargo. (#189)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/6.0.0)
para más detalles.

## [5.6.1] - 2026-06-16

### Corregido

- Se corrigió un panic en `extract_comment_marker` cuando un carácter
  multi-byte UTF-8 (em-dash, emoji, letras acentuadas, CJK) cruzaba el byte
  offset 16 de un comentario, lo que antes abortaba el análisis del
  directorio completo. El byte-slicing propenso a errores se reemplazó con
  coincidencia basada en regex. (#187)

### Eliminado

- Se eliminó el workflow de auto-release `release-plz.yml` y sus archivos de
  soporte (`CHANGELOG.md`, `cliff.toml`). Los releases manuales mediante
  `release.yml` (tag → build → test → publish) son ahora el camino
  estándar. (#188)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/5.6.1)
para más detalles.

## [5.6.0] - 2026-06-14

### Añadido

- Flag `--no-ignore` para ignorar los comentarios `# complexipy: ignore` y
  `# noqa: complexipy`. (#182)
- Flag `--report-ignored` para listar todas las funciones suprimidas, con
  exportación JSON opcional a `complexipy-ignored.json`. (#182)
- Tipo `IgnoredLocation` y `collect_all_ignored_locations()` en la API de
  Python. (#182)

### Corregido

- Se corrigió el modo diff que mostraba todos los archivos como NEW en
  Windows. (#177)

### Cambiado

- Se migró pyo3 a 0.29 y se actualizaron los runners de CI. (#185)
- Se añadieron builds de wheels musllinux_1_2 para soporte de Alpine
  Linux. (#180)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/5.6.0)
para más detalles.

## [5.5.0] - 2026-05-22

### Añadido

- Planes de refactorización deterministas: un algoritmo nuevo genera
  sugerencias accionables y deterministas para reducir la complejidad
  cognitiva. Los planes se muestran en la salida rica del CLI y se incluyen
  en la salida JSON.
- Globs de exclusión recursivos: los patrones de exclusión ahora admiten
  `**` (p. ej. `tests/**`). El motor de globs se reemplazó por `wax` para un
  emparejamiento recursivo correcto relativo al directorio de trabajo del
  llamador.

### Corregido

- Se corrigió el crecimiento ilimitado de las entradas target-set en la
  caché, que podía degradar el rendimiento en proyectos grandes.

### Cambiado

- Los internals de salida ahora usan dataclasses tipados `FunctionRow` /
  `FileEntry` en lugar de estructuras `Dict` sin tipar, extraídos en
  `complexipy/utils/dataclasses.py`.
- Los despliegues de docs ahora se disparan solo en eventos de release, no
  en cada push.

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/5.5.0)
para más detalles.

## [5.4.1] - 2026-05-05

### Corregido

- Se estabilizó la salida de snapshots omitiendo los campos transitorios a
  nivel de línea de las estructuras Rust serializadas hacia Python.
- Se movieron las constantes de salida compartidas a
  `complexipy.utils.constants` para que los nombres de archivo de salida,
  las flags CLI heredadas y las claves TOML heredadas se definan en un solo
  lugar.
- Se aclaró el mensaje de deprecación de `--output-csv`, `--output-json` y
  `--output-gitlab` para apuntar a `--output-format`.

### Cambiado

- Se actualizó el CI para que los pull requests ejecuten una matriz de
  quick-test más rápida, mientras que los builds completos de wheels, sdist
  y los jobs de test de todas las plataformas se ejecutan para tags o
  despachos manuales.
- Se habilitó el caching de dependencias uv en los jobs de CI.
- Se actualizaron el README y los docs para reflejar los tipos de retorno
  actuales de la API de Python y el formato actual de array JSON de los
  snapshots.

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/5.4.1)
para más detalles.

## [5.4.0] - 2026-04-25

### Eliminado

- Se eliminó el scoring de complejidad de comprehensions - revierte el
  manejo de nodos AST `ListComp`/`SetComp`/`Generator`/`DictComp` en
  `count_bool_ops()` y el helper `count_comprehension_complexity()` añadido
  en v5.3.0. (#166)

### Corregido

- Se corrigió el feature gating de los builds WASM y se incluyeron los
  metadatos de versión. (#164)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/5.4.0)
para más detalles.

## [5.3.0] - 2026-04-16

### Añadido

- Flag `--ratchet` - el CI falla solo cuando la complejidad aumenta por
  encima del umbral; las regresiones se bloquean, las mejoras siempre
  pasan. (#159)
- Flag `--plain` - salida de texto plano legible por máquinas para scripting
  y pipes. (#158)
- Flag `--top N` - muestra las N funciones más complejas, ordenadas
  globalmente entre todos los archivos. (#157)
- Flags `--check-script` / `--script-strict` - analizan la complejidad a
  nivel de módulo (script) además de las funciones. (#156)
- Destinos de salida unificados - enrutamiento consistente de `--output-*`
  en todos los formatos de reporte. (#155)
- Reporte de GitLab Code Quality mediante `--output-gitlab`. (#153)
- Salida SARIF 2.1.0 mediante `--output-sarif` para integración con IDE y
  GitHub Advanced Security. (#141)
- Análisis de diff git - `--diff <ref>` informa de los cambios de
  complejidad relativos a cualquier referencia git. (#140)
- Complejidad de comprehensions - las comprehensions de lista/dict/set y las
  expresiones generador ahora contribuyen a las puntuaciones de complejidad
  cognitiva. (#139)
- `# complexipy: ignore` - nuevo comentario canónico de supresión en línea;
  `# noqa: complexipy` queda deprecado. (#146)
- Patrones glob en el campo `exclude` de la configuración. (#142)
- Documentación en español. (#147)

### Corregido

- Los resultados de `--top` ahora preservan el orden descendente global en
  ejecuciones multi-archivo.
- `--top N` rechaza `N ≤ 0` con un error claro.
- `--script-strict` ahora requiere `--check-script` correctamente.
- Los marcadores de ignorar (`# complexipy: ignore`) ahora funcionan en
  definiciones de función multilínea.
- La salida JSON incluye una nueva línea final (cumplimiento POSIX). (#148)
- Las funciones permitidas por snapshot ahora se muestran como `PASSED` en
  la salida.
- El watermark de snapshot ahora controla correctamente el código de salida
  cuando está activo.

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/5.3.0)
para más detalles.

## [5.2.0] - 2026-01-28

### Corregido

- Se corrigieron los comentarios `# noqa: complexipy` en funciones con
  decoradores. (#128)

### Cambiado

- Se actualizó la versión de complexipy pre-commit en los docs. (#126)
- Se actualizó la documentación. (#132)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/5.2.0)
para más detalles.

## [5.1.0] - 2025-12-09

### Corregido

- Se corrigieron las rutas de salida de resultados inválidas en
  Windows. (#120)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/5.1.0)
para más detalles.

## [5.0.0] - 2025-11-26

!!! note "Migración"

    El scoring condicional ahora cuenta cada rama `elif`/`else` como +1 de
    complejidad (más su test booleano), alineándose con las reglas de
    complejidad cognitiva de Sonar. Espera puntuaciones más altas en código
    con ramificaciones. Consulta la
    [guía de migración](https://rohaquinlop.github.io/complexipy/es/migracion/)
    para obtener orientación sobre la actualización.

### Añadido

- Snapshots: `--snapshot-create` escribe `complexipy-snapshot.json`; las
  comparaciones bloquean regresiones, se auto-refrescan en mejoras y pueden
  omitirse con `--snapshot-ignore`. (#111)
- Seguimiento de cambios: una caché por target en `.complexipy_cache`
  muestra deltas/nuevos fallos de las funciones por encima del umbral con
  claves estables BLAKE2. (#115)
- Controles de salida: `--failed` para mostrar solo violaciones (#114);
  `--color auto|yes|no` (#109); resúmenes más ricos de funciones con fallos
  y rutas inválidas.
- Soporte de Python 3.14. (#106)

### Cambiado

- Exclusiones y errores: las entradas de exclusión se resuelven relativas a
  la raíz y solo se aplican cuando coinciden con archivos/directorios
  reales; las rutas inexistentes se informan limpiamente en lugar de
  producir un panic. (#113)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/5.0.0)
para más detalles.

## [4.2.0] - 2025-09-21

### Añadido

- Excluir archivos del análisis.
- Ignorados en línea para excluir funciones del análisis.

### Corregido

- Soporte de Python 3.8. (#96)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/4.2.0)
para más detalles.

## [4.1.0] - 2025-09-08

### Añadido

- Soporte de versión. (#93)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/4.1.0)
para más detalles.

## [4.0.2] - 2025-08-22

Versión de parche; no se registraron cambios en sus notas de versión.

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/4.0.2)
para más detalles.

## [4.0.1] - 2025-08-21

### Corregido

- Se corrigió el error del README en PyPI.

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/4.0.1)
para más detalles.

## [4.0.0] - 2025-08-21

!!! note "Migración"

    La lógica de conteo de operadores booleanos en condiciones se actualizó
    para alinearse con la definición del paper original. Las funciones
    existentes pueden reportar complejidades más altas. Consulta la
    [guía de migración](https://rohaquinlop.github.io/complexipy/es/migracion/)
    para obtener orientación sobre la actualización.

### Añadido

- Soporte de configuración mediante `complexipy.toml` o `[tool.complexipy]`
  en `pyproject.toml` - los usuarios pueden ahora definir argumentos por
  defecto.

### Corregido

- Se corrigió un bucle infinito al analizar módulos con sintaxis Python
  inválida. (#85, resuelto en #88)

### Cambiado

- Se mejoró el rendimiento y los detalles de implementación en Rust.
- Se actualizó y mejoró la documentación.

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/4.0.0)
para más detalles.

## [3.3.0] - 2025-07-17

### Añadido

- `--max-complexity-allowed` (`-mx`) - personaliza el umbral máximo de
  complejidad cognitiva permitido por función. El valor por defecto sigue
  siendo 15 para mantener el comportamiento existente.
- Integración con GitHub Actions para el umbral personalizado.

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/3.3.0)
para más detalles.

## [3.2.0] - 2025-07-09

### Corregido

- Se corrigió un error al usar complexipy en Windows, relacionado con la
  librería `rich` usada para dibujar la salida de consola con emojis.
- Se corrigió que el parámetro `quiet` siguiera dibujando salida no
  manejada en el código Rust.

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/3.2.0)
para más detalles.

## [3.1.1] - 2025-07-06

### Cambiado

- El umbral máximo de complejidad ahora es `15`, coincidiendo con el umbral
  de Sonar para facilitar la adopción de la librería. (#78)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/3.1.1)
para más detalles.

## [3.0.0] - 2025-06-16

!!! note "Migración"

    `--max-complexity` se eliminó; complexipy ahora usa un umbral fijo de
    complejidad cognitiva de 15. La herramienta sale con error cuando una
    función alcanza o supera el umbral. Usa `--ignore-complexity` (`-i`)
    para mostrar todas las funciones independientemente de su puntuación.
    Consulta la
    [guía de migración](https://rohaquinlop.github.io/complexipy/es/migracion/)
    para obtener orientación sobre la actualización.

### Añadido

- Soporte de WebAssembly (WASM) - el motor de análisis central ahora puede
  compilarse a WebAssembly, habilitando el análisis en el navegador y
  herramientas como la extensión de VSCode. (#72)
- Salida JSON mediante `--output-json` (`-j`) para resultados legibles por
  máquinas. (#74)
- Flag `--ignore-complexity` (`-i`) para mostrar todas las funciones
  independientemente de si superan el umbral de complejidad. (#73)
- `--details` (`-d`) ahora afecta también a las salidas CSV y JSON. (#73)
- Ordenar resultados por puntuación de complejidad (`asc`, `desc`) o por
  `name`. (#73)
- Soporte de pre-commit hook con documentación para una configuración
  sencilla. (#75)

### Eliminado

- El argumento `--max-complexity`. (#73)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/3.0.0)
para más detalles.

## [2.1.1] - 2025-04-24

### Corregido

- Se corrigió la compatibilidad con Python 3.8. (#66)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/2.1.1)
para más detalles.

## [2.1.0] - 2025-04-23

### Corregido

- Se corrigió el conteo de operadores booleanos en expresiones de
  diccionario. (#64)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/2.1.0)
para más detalles.

## [2.0.0] - 2025-04-18

### Cambiado

- Se cambió el parser de `rustpython` a `ruff_python_parser`. (#62)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/2.0.0)
para más detalles.

## [1.2.0] - 2024-12-15

### Corregido

- Se corrigió la llamada a `output_summary`, a la que faltaba el argumento
  `files_complexities`. (#58)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/1.2.0)
para más detalles.

## [1.1.0] - 2024-12-14

### Añadido

- Soporte de múltiples rutas - pasa varios paths para analizar a la
  vez. (#56)

### Eliminado

- La opción deprecada `-l`, simplificando la interfaz de línea de
  comandos. (#56)
- El análisis a nivel de archivo, para centrarse en las funciones en lugar
  de todo el archivo. (#56)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/1.1.0)
para más detalles.

## [0.5.0] - 2024-10-28

### Añadido

- API de Python - llama a complexipy desde tu propio código Python con
  `file_complexity()` y `code_complexity()`. (#45)
- Uso de la librería documentado en el README. (#49)
- Usabilidad del paquete mejorada. (#53)
- Documentación actualizada. (#54)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/0.5.0)
para más detalles.

## [0.4.0] - 2024-06-21

### Añadido

- La complejidad cognitiva ahora considera el `If Expression` usado en el
  código, incluido dentro de un `Call Expression`, etc. (#44)

### Corregido

- Se corrigió un edge case que podía causar un desbordamiento de memoria,
  filtrando la complejidad cognitiva de los valores `orelse` en un `If Statement` y manteniendo la propagación de la resta del nivel de
  anidamiento. (#44)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/0.4.0)
para más detalles.

## [0.3.3] - 2024-04-27

### Cambiado

- Se actualizó el CI y se eliminaron dependencias sin uso. (#43)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/0.3.3)
para más detalles.

## [0.3.2] - 2024-03-22

### Cambiado

- Al usar `--details low`, ya no se imprime una tabla de resumen vacía; se
  muestra un mensaje informativo. (#38)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/0.3.2)
para más detalles.

## [0.3.1] - 2024-03-13

### Añadido

- Parámetro opcional `-s` / `--sort` para ordenar la salida. (#30)
- Requisito de Python >= 3.8. (#35)

### Corregido

- Se corrigió la lógica usada para calcular la complejidad cognitiva: las
  sentencias de asignación solo añaden complejidad cuando se usa `IfExp`, y
  `BinOp` por sí solo no añade complejidad. (#34)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/0.3.1)
para más detalles.

## [0.3.0] - 2024-03-07

### Añadido

- Análisis de complejidad a nivel de función por defecto - la complejidad
  máxima se evalúa para cada función dentro de los archivos Python; la
  complejidad cognitiva por archivo sigue disponible. (#21)
- Nuevos parámetros. (#14)
- Barras de progreso. (#24)
- Opción `--quiet`. (#26)
- Tests unitarios. (#20)
- Explicación de la complejidad cognitiva en los docs. (#22)

### Cambiado

- Se mejoró el algoritmo de medición de la complejidad cognitiva - los
  resultados se acercan más a la implementación de Sonar. (#18)
- Se redujo la verbosidad, con más información sobre las etapas al ejecutar
  `complexipy` sobre repositorios git (usando la URL).
- La generación del reporte CSV ahora está implementada en Rust en lugar de
  Python, mejorando el rendimiento.

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/0.3.0)
para más detalles.

## [0.2.2] - 2024-02-27

Versión de parche; no se registraron cambios en sus notas de versión.

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/0.2.2)
para más detalles.

## [0.2.1] - 2024-02-27

### Cambiado

- Se actualizó el README.

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/0.2.1)
para más detalles.

## [0.2.0] - 2024-02-27

### Añadido

- Soporte de rutas a ignorar. (#6)
- Soporte de URLs de repositorios git. (#7)
- Formato de salida CSV. (#8)
- Correcciones del algoritmo de complejidad cognitiva. (#10)
- README actualizado. (#9)

Consulta las [notas de la versión](https://github.com/rohaquinlop/complexipy/releases/tag/0.2.0)
para más detalles.
