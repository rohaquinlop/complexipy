# Flujos de Trabajo con Agentes de IA

Los agentes de programación como Claude Code y Codex CLI escriben código en un
bucle: editan archivos, ejecutan herramientas y continúan con la salida. La
complejidad cognitiva mide qué tan difícil es revisar ese código, y una función
por encima del límite se corrige más barato mientras el agente todavía tiene el
archivo en contexto, siempre que algo le avise.

Los hooks son comandos que un agente ejecuta en eventos de su ciclo de vida. Un
hook que ejecuta complexipy lleva el análisis al mismo turno, antes de que una
persona lea el diff. Esta página muestra dos hooks para ambos agentes: uno
después de cada edición de archivo y otro cuando el agente intenta terminar.

## Qué usan los hooks de complexipy

complexipy termina con estado 1 cuando una función supera
`max-complexity-allowed`, y con 0 cuando el código pasa. Un hook traduce ese
código de salida al protocolo de retroalimentación del agente, así que el hook
se mantiene pequeño.

Las recetas revisan el diff en lugar de todo el repositorio:

- `complexipy . --diff HEAD` revisa los cambios sin confirmar después de cada
  edición.
- `complexipy . --diff main` revisa la rama cuando termina el turno.

`--quiet` conserva el código de salida y descarta el reporte de la terminal,
porque el hook imprime su propio mensaje. Si prefieres la lista de funciones en
la retroalimentación, `--failed --plain` imprime una línea por cada función que
supera el límite:

```bash
complexipy src/app.py --failed --plain --max-complexity-allowed 15
```

La salida es una línea por cada función que supera el límite:

```text
src/app.py handle_order 22
```

Los scripts son `sh` POSIX y esperan `complexipy` en el `PATH`. Funcionan en
repositorios git, porque `--diff` necesita una referencia con la que comparar.

Ambos hooks comparan una referencia de git, no el archivo que acaba de
cambiar: el hook de edición revisa todos los cambios sin confirmar y el hook
Stop revisa toda la rama contra `main`. Empieza con un árbol limpio, o confirma
o guarda tu propio trabajo, para que la retroalimentación trate sobre los
cambios del agente.

## El hook de edición

Guarda este script en el directorio de hooks de tu agente y dale permiso de
ejecución:

```sh
#!/bin/sh
# Reports the functions above the cognitive complexity limit that the working
# tree adds or worsens, so the agent fixes them before the turn continues.
limit=${COMPLEXIPY_MAX_COMPLEXITY:-15}
ref=${COMPLEXIPY_EDIT_DIFF_REF:-HEAD}
root=$(git rev-parse --show-toplevel 2>/dev/null) || exit 0
cd "$root" || exit 0
complexipy . --diff "$ref" --max-complexity-allowed "$limit" --quiet
status=$?
if [ "$status" -eq 0 ]; then
    exit 0
fi
if [ "$status" -ne 1 ]; then
    printf '%s\n' "complexipy could not check the working tree (exit $status)." >&2
    exit 1
fi
printf '%s\n' "complexipy: the working tree adds or worsens a function above the cognitive complexity limit of $limit." >&2
printf '%s\n' "Run 'complexipy . --diff $ref --failed' to list those functions, then refactor them." >&2
exit 2
```

El script se ejecuta después de cada edición, así que `--diff HEAD` limita el
análisis a las funciones que las ediciones introdujeron o empeoraron. El estado
de salida 2 es la señal de retroalimentación para ambos agentes: Claude Code
muestra el texto de stderr al modelo y Codex CLI reemplaza el resultado de la
herramienta con él.

## El hook de fin de turno

Guarda este script junto al hook de edición:

```sh
#!/bin/sh
# Reports the functions above the cognitive complexity limit that the branch
# adds or worsens, so the agent keeps working instead of stopping.
limit=${COMPLEXIPY_MAX_COMPLEXITY:-15}
ref=${COMPLEXIPY_DIFF_REF:-main}
input=$(cat)
if printf '%s' "$input" | grep -Eq '"stop_hook_active"[[:space:]]*:[[:space:]]*true'; then
    exit 0
fi
root=$(git rev-parse --show-toplevel 2>/dev/null) || exit 0
cd "$root" || exit 0
complexipy . --diff "$ref" --max-complexity-allowed "$limit" --quiet
status=$?
if [ "$status" -eq 0 ]; then
    exit 0
fi
if [ "$status" -ne 1 ]; then
    printf '%s\n' "complexipy could not check the branch (exit $status)." >&2
    exit 1
fi
printf '%s\n' "complexipy: the branch adds or worsens a function above the cognitive complexity limit of $limit." >&2
printf '%s\n' "Run 'complexipy . --diff $ref --failed' to list those functions, then refactor them." >&2
exit 2
```

Compara la rama con `main` y se ejecuta de nuevo cada vez que el agente
intenta detenerse. Un estado fallido devuelve al agente al trabajo con el
mensaje; un estado correcto deja terminar el turno. El script lee
`stop_hook_active`, así que bloquea como máximo una vez por turno. Usa
`COMPLEXIPY_DIFF_REF` cuando tu rama base no sea `main`, y
`COMPLEXIPY_MAX_COMPLEXITY` cuando uses un límite distinto. El hook de edición
toma su propia referencia de `COMPLEXIPY_EDIT_DIFF_REF` (por defecto `HEAD`),
lo que ayuda cuando el agente confirma un commit después de cada edición.

Un hook Stop se ejecuta al final de cada respuesta, no solo cuando la tarea
termina, así que la revisión corre en cada turno y bloquea cuando la rama
agrega o empeora una función por encima del límite.

Ambos agentes envían `stop_hook_active` en la entrada del hook. El script
termina con 0 mientras esa bandera sea verdadera, lo que evita que un hook Stop
bloquee para siempre. Claude Code además corta un hook Stop después de ocho
bloqueos consecutivos.

## Claude Code

Copia ambos scripts a `.claude/hooks/` en el repositorio y dales permiso de
ejecución:

```bash
chmod +x .claude/hooks/complexipy-edit.sh .claude/hooks/complexipy-stop.sh
```

Agrega ambos eventos a `.claude/settings.json`. Si el archivo ya tiene un objeto
`hooks`, agrega estas entradas como hermanas de las claves de evento
existentes:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/complexipy-edit.sh"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "\"$CLAUDE_PROJECT_DIR\"/.claude/hooks/complexipy-stop.sh",
            "timeout": 120
          }
        ]
      }
    ]
  }
}
```

Claude Code recarga los hooks normalmente cuando cambia el archivo de
configuración; reinicia la sesión si un hook nuevo no aparece. Ejecuta `/hooks`
para confirmar que ambos aparecen en `PostToolUse` y `Stop`.

Claude Code ejecuta los hooks del proyecto solo después de que confíes en el
espacio de trabajo.

Para probar un hook sin un turno del agente, pásale por stdin la entrada que
envía el evento y revisa el código de salida:

```bash
echo '{"hook_event_name":"Stop","stop_hook_active":false}' | .claude/hooks/complexipy-stop.sh
```

El estado 0 deja terminar el turno, y el estado 2 bloquea la parada y le pasa
el mensaje al agente. Consulta la
[referencia de hooks de Claude Code](https://code.claude.com/docs/en/hooks) para
los campos de los eventos y las reglas de los códigos de salida, y la
[guía de hooks](https://code.claude.com/docs/en/hooks-guide) para más ejemplos.

## Codex CLI

Copia ambos scripts a `.codex/hooks/` y crea `.codex/hooks.json`:

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "matcher": "apply_patch|Edit|Write",
        "hooks": [
          {
            "type": "command",
            "command": "sh \"$(git rev-parse --show-toplevel)/.codex/hooks/complexipy-edit.sh\"",
            "timeout": 120,
            "statusMessage": "Checking cognitive complexity"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "sh \"$(git rev-parse --show-toplevel)/.codex/hooks/complexipy-stop.sh\"",
            "timeout": 120,
            "statusMessage": "Checking cognitive complexity"
          }
        ]
      }
    ]
  }
}
```

Los comandos resuelven los scripts desde la raíz de git porque Codex puede
iniciarse en un subdirectorio. Codex ejecuta un hook del proyecto solo después
de que confíes en la capa `.codex` del proyecto y revises el hook con `/hooks`,
y vuelve a preguntar cuando el hook cambia. Los mismos hooks pueden declararse
en línea en `.codex/config.toml` bajo `[[hooks.PostToolUse]]` y `[[hooks.Stop]]`
si prefieres un solo archivo. Consulta la
[referencia de hooks de Codex](https://learn.chatgpt.com/docs/hooks) para el
esquema completo y las reglas de confianza.

## Recetas sin hooks

El mismo veredicto funciona fuera del bucle del agente:

- Bloquea la rama en pre-commit o CI con
  `complexipy . --diff main --max-complexity-allowed 15`; consulta
  [Integración con CI/CD](ci.md).
- Pide un plan con `complexipy <path> --suggest-refactors`, revisa el parche con
  `complexipy <path> --fix --dry-run` y aplícalo con `--fix`.
- Escribe la regla en el archivo de instrucciones que lee el agente (`AGENTS.md`
  o `CLAUDE.md`): "Mantén cada función en 15 de complejidad cognitiva o menos.
  Antes de terminar, ejecuta
  `complexipy . --diff main --max-complexity-allowed 15` y refactoriza lo que
  marque."
- Apunta el editor al servidor de lenguaje cuando el agente lea diagnósticos;
  consulta [Integración con Editores](editors.md).

## Notas

- `--plain` imprime `<path> <function> <complexity>` sin números de línea
  ([#326](https://github.com/rohaquinlop/complexipy/issues/326) sigue abierto).
  La salida legible por máquina en stdout no forma parte de 8.3.0;
  `--output -` ya está en `main` y llega en la próxima versión. Los hooks de
  esta página usan solo el código de salida, así que funcionan con 8.3.0 y
  posteriores.
- Los hooks tratan el estado 1 como incumplimiento del límite, y complexipy
  también termina con 1 ante algunos errores de configuración. Cuando la
  retroalimentación no corresponda a una función del diff, ejecuta el mismo
  comando sin `--quiet` para leer el mensaje.
- `--diff` necesita Git 2.30 o posterior. Si la referencia no existe, por
  ejemplo cuando la rama por defecto es `master`, complexipy trata todas las
  funciones como nuevas y el hook informa un fallo; configura la referencia en
  la primera ejecución.
- Los hooks se ejecutan con tus permisos. Revisa los scripts antes de
  confirmarlos en un repositorio, como cualquier otro comando que una
  herramienta ejecute por ti.
- Los scripts necesitan un shell POSIX. En Windows, instala Git para Windows,
  que proporciona `sh`, y asegúrate de que el agente lo encuentre en el `PATH`.
- Los formatos de los hooks cambian entre versiones de los agentes. Los
  ejemplos siguen las referencias enlazadas arriba, y los scripts no necesitan
  cambios cuando el esquema de configuración cambia.
