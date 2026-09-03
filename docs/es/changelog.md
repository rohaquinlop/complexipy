# Registro de Cambios

Todos los cambios notables de complexipy se documentan aquí, de más reciente
a más antiguo. Cada sección de versión enlaza a sus notas de versión de
GitHub con todos los detalles.

## Sin publicar

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
