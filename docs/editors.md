# Editor Integration

complexipy ships a Language Server Protocol (LSP) server, so your editor can
surface the same cognitive complexity analysis the CLI produces while you
write code. The server runs the same Rust engine as `complexipy` and the Python
API, and it needs no configuration to start being useful: install complexipy,
point your editor at `complexipy lsp`, and keep working.

Inside the editor you get:

- **Inlay hints** - the cognitive complexity of a function, shown at the end of
  its `def` line as `cognitive: 18`.
- **Hover** - hovering the `def` lines of a function shows its name, its
  cognitive complexity, whether it exceeds the allowed threshold, and the title
  of the top refactoring suggestion when one exists.
- **Diagnostics** - one warning per function above `max-complexity-allowed`,
  with the message `cognitive complexity 18 exceeds the allowed 15` on the
  function name.

Results refresh as you type. The server uses full document sync and analyzes
only the documents you have open, so there is no workspace-wide scan and no
need to save.

## Installation

Install complexipy so that the `complexipy` command is on your `PATH`:

```bash
uv tool install complexipy
```

or, with pip:

```bash
pip install complexipy
```

Anything that installs the `complexipy` command works. The editor starts the
server as a subprocess with `complexipy lsp`; no separate binary download
exists. Upgrade the tool with `uv tool upgrade complexipy` or
`pip install --upgrade complexipy` to pick up new rules and settings.

## Neovim

Neovim 0.11 and later can configure and enable the server with
`vim.lsp.config` and `vim.lsp.enable`:

```lua
vim.lsp.config("complexipy", {
  cmd = { "complexipy", "lsp" },
  filetypes = { "python" },
  root_markers = { "complexipy.toml", ".complexipy.toml", "pyproject.toml", ".git" },
})

vim.lsp.enable("complexipy")
```

On Neovim 0.10, start the server directly instead:

```lua
vim.lsp.start({ name = "complexipy", cmd = { "complexipy", "lsp" } })
```

Inlay hints are opt-in in Neovim. Enable them once, globally:

```lua
vim.lsp.inlay_hint.enable(true)
```

or per buffer with `vim.lsp.inlay_hint.enable(true, { bufnr = 0 })`.

## Zed

Zed starts a language server only when Zed or an extension registers it. The
complexipy extension lives in the `zed/` folder of the repository. Until it is
published in the Zed extension registry, install it as a dev extension:

1. Clone `https://github.com/rohaquinlop/complexipy`.
1. Run `zed: install dev extension` from the command palette and select the
   `zed/` folder.

Then enable the server and inlay hints for Python in `settings.json`:

```json
{
  "inlay_hints": { "enabled": true },
  "languages": {
    "Python": {
      "language_servers": ["complexipy", "..."]
    }
  }
}
```

The `"..."` entry keeps every other Python language server you already use
enabled. If your settings already list Python language servers without
`"..."`, add `"complexipy"` to that list, or Zed does not start it.

Zed turns inlay hints off by default. `inlay_hints.enabled` turns them on for
every language server, so type hints from servers such as `ty` or `basedpyright`
appear next to the complexipy hint.

The `lsp.hover` and `lsp.diagnostic-range` settings apply in Zed as well. The
colored badges of the VS Code extension do not: Zed shows the inlay hints.

The extension runs `complexipy lsp` from the `PATH`. To use another binary,
set its path:

```json
{
  "lsp": {
    "complexipy": {
      "binary": {
        "path": "/full/path/to/complexipy",
        "arguments": ["lsp"]
      }
    }
  }
}
```

## VS Code

Install the
[complexipy extension](https://marketplace.visualstudio.com/items?itemName=rohaquinlop.complexipy)
from the Visual Studio Marketplace, or from
[OpenVSX](https://open-vsx.org/extension/rohaquinlop/complexipy) in VSCodium and other forks.
It is a client of `complexipy lsp`: it starts the server for Python files and
shows its inlay hints, hover, and warnings. VS Code shows inlay hints by
default, so no editor setting is needed.

The extension also adds a status bar item with the number of functions over
the threshold in the active file. Click it to open the Problems panel.

To see colored badges instead of inlay hints, set `complexipy.display` to
`decorations`. The extension then shows `ƒ N` after every function declaration
and `+N` after every line that adds complexity, in green, yellow, or red:

- A function is green up to half of `max-complexity-allowed`, yellow up to the
  limit, and red above it.
- A line is yellow from `complexipy.decorations.lineWarningAt` (default `3`)
  and red from `complexipy.decorations.lineErrorAt` (default `5`).

```json
{
  "complexipy.display": "decorations",
  "complexipy.decorations.lineWarningAt": 3,
  "complexipy.decorations.lineErrorAt": 5
}
```

The badges show every function and every line that adds complexity, whatever
`inlay-hints` and `per-line-hints` say. They need complexipy 8.2.0 or later.
With an older server the extension shows a warning and keeps the inlay hints.

The extension runs `complexipy lsp` from the `PATH`. To use another binary,
set its path in `settings.json`:

```json
{
  "complexipy.server.path": "/full/path/to/complexipy"
}
```

A relative path resolves against the first workspace folder. In Restricted
Mode, VS Code ignores a `complexipy.server.path` set in the workspace's
settings, so an untrusted repository cannot choose the binary; the extension
then runs the path from your user settings, or `complexipy` from the `PATH`.

## Configuration

The server reads the same configuration files as the CLI - `complexipy.toml`,
`.complexipy.toml`, or `pyproject.toml` (under `[tool.complexipy]`) - discovered
at the root of each workspace folder:

=== "complexipy.toml"

    ```toml
    max-complexity-allowed = 15

    [lsp]
    inlay-hints = "threshold"   # "always" | "threshold" | "never"
    per-line-hints = false
    diagnostics = true
    diagnostic-range = "name"   # "name" | "function"
    hover = "signature"         # "signature" | "function" | "never"
    ```

=== "pyproject.toml"

    ```toml
    [tool.complexipy]
    max-complexity-allowed = 15

    [tool.complexipy.lsp]
    inlay-hints = "threshold"   # "always" | "threshold" | "never"
    per-line-hints = false
    diagnostics = true
    diagnostic-range = "name"   # "name" | "function"
    hover = "signature"         # "signature" | "function" | "never"
    ```

| Key | Default | Description |
| -- | -- | -- |
| `max-complexity-allowed` | `15` | Functions strictly above this value are reported. A function at the limit passes. |
| `exclude` | `[]` | Glob patterns, matched relative to the workspace folder that holds the configuration file and to each of its `paths` entries. Excluded files produce no hints and no diagnostics. |
| `no-ignore` | `false` | Analyze functions even when an inline ignore comment suppresses them. |
| `lsp.inlay-hints` | `"threshold"` | When to show the per-function hint: `"threshold"`, `"always"`, or `"never"`. |
| `lsp.per-line-hints` | `false` | Also show a `+N` hint on every line with a non-zero complexity increment. |
| `lsp.diagnostics` | `true` | Publish warnings for functions above `max-complexity-allowed`. |
| `lsp.diagnostic-range` | `"name"` | What the warning underlines: `"name"` the function name, or `"function"` the whole function. |
| `lsp.hover` | `"signature"` | Where hover answers: `"signature"` on the `def` lines, `"function"` on any line of the function, or `"never"`. |

The server ignores every other key of the same file: `paths`, `quiet`,
`failed`, `sort`, `color`, `output`, `output-format`, `cache-dir`,
`snapshot-create`, `snapshot-ignore`, `ignore-complexity`, `check-script`,
`report-ignored`, `force-exclude`, and `diff`. Run those keys through `complexipy <path>`.

The server reads the configuration again when you save a change to one of
these files at the root of a workspace folder. The editor must support dynamic
registration of `workspace/didChangeWatchedFiles`. If a saved file does not
parse, the editor shows a warning and the folder uses the default settings.
With an editor that does not watch files, restart the server after you change
the configuration.

### Inlay hints

The hint is placed at the end of the last line of the function declaration: the
line that closes the signature. For a single-line signature that is the `def`
line, and for a decorated function it is the `def` line rather than the
`@decorator` line above it. For a signature that spans several lines the hint
sits after the closing `)` or `):`:

```python
def heavy(
    a,
    b,
):
```

Here the hint sits at the end of the `):` line.

- `inlay-hints = "threshold"` (the default) shows the per-function hint
  (`cognitive: 18`) only when the function is **above** `max-complexity-allowed`.
- `inlay-hints = "always"` shows it for every function.
- `inlay-hints = "never"` suppresses every hint, including per-line hints.
- `per-line-hints = true` additionally shows a `+N` hint at the end of every
  line that has a non-zero complexity increment. Lines with an increment of
  zero never produce a hint.

The server reports the same functions `complexipy <path>` reports: top-level
functions, plus methods of classes. A function nested inside another one is
folded into its parent, so there is no separate hint for it. With
`hover = "function"`, hovering inside a nested function therefore shows its
parent.

### Hover

- `hover = "signature"` (the default) answers only on the declaration lines:
  from the `def` line to the line that closes the signature. Hovering a call in
  the body leaves the editor's own hover alone.
- `hover = "function"` answers on any line of the function.
- `hover = "never"` turns hover off.

### Diagnostics

- `diagnostics = false` publishes no warnings at all.
- `diagnostic-range = "name"` (the default) underlines only the function name
  on its `def` line. `diagnostic-range = "function"` underlines the whole
  function.
- The reported threshold matches `complexipy <path>`: a function whose
  complexity **equals** `max-complexity-allowed` passes. Only strictly greater
  values are reported, as
  `cognitive complexity 18 exceeds the allowed 15`.
- A document the parser cannot read produces exactly one warning with the code
  `complexipy-parse-error` on the range where the parser fails, with the
  parser's message. While that warning is present,
  hints and hover keep answering from the last text that parsed, so the file
  does not go blank while you type. The warning clears itself as soon as the
  document parses again.

### Ignores and exclusions

- `exclude` globs are matched relative to the workspace folder that holds the
  configuration file, and relative to each `paths` entry of that file, the
  same way a CLI run in that folder matches them. Excluded files produce no
  hints and no diagnostics.
- A nested workspace folder without a configuration file of its own uses the
  configuration of the nearest enclosing folder that has one.
- A pattern that can never match, such as `./vendor/**` or `vendor/`, is
  reported as a warning.
- A malformed glob is reported once per configuration load and skipped.
  Each configuration problem (a malformed glob, an oversized list, an unknown
  rule id in `select` or `ignore`, a selection that leaves no rule active, or
  a file that does not parse) is written to the server log, and the editor
  shows one fixed warning that points to that log. The warning never quotes
  the configuration, so a repository cannot place a link in it. The other
  patterns in the list keep applying.
- A list too large to compile as one program is reported once per
  configuration load and still applied one pattern at a time. A CLI run stops
  on the same list.
- Inline suppression comments (`# noqa: complexipy` and
  `# complexipy: ignore`) are honored exactly as in the CLI, unless
  `no-ignore = true`.

## Troubleshooting

### The editor cannot find the `complexipy` command

The editor starts the server as a subprocess, so `complexipy` must be on the
`PATH` that the editor process inherits. Point the server at an absolute path
when the tool is installed somewhere the editor cannot see:

- Neovim: set `cmd = { "/full/path/to/complexipy", "lsp" }`.
- Zed: set `"path"` to the absolute binary in the `lsp.complexipy.binary`
  block. Without it, the extension looks for `complexipy` on the `PATH`, and
  Zed shows an error when it is not there.
- VS Code: set `complexipy.server.path` to the absolute binary. Without it,
  the extension looks for `complexipy` on the `PATH`, and VS Code shows an
  error when it is not there.

Alternatively, start it through `uv` without installing anything globally:

```lua
cmd = { "uvx", "complexipy", "lsp" },
```

### Where to read the server logs

The server never writes anything except protocol frames to stdout; all logging
goes to stderr. Read it where your editor collects it:

- Neovim: `:LspLog`.
- Zed: open the log panel from the command palette (`zed: open log`).
- VS Code: open the Output panel and select the `complexipy` channel.

### Hints do not appear

Inlay hints are opt-in in most editors. Enable them in the editor itself
(`vim.lsp.inlay_hint.enable(true)` in Neovim, `"inlay_hints": { "enabled": true }`
in Zed), and remember that the default
`inlay-hints = "threshold"` only shows the hint when a function is above
`max-complexity-allowed`. Set `inlay-hints = "always"` to see every function.

### The whole file shows warnings

A limit lower than expected usually means the wrong configuration is active.
The server reads `complexipy.toml`, `.complexipy.toml`, or `pyproject.toml`
from the root of the document's workspace folder; check that the folder the
editor opened is the one holding your settings. The server reads the file at
startup, and again when you save it in an editor that watches files for the
server. In other editors, restart the server after you edit it (`:LspRestart`
in Neovim, `editor: restart language server` in Zed).

### Per-line hints do not appear

`per-line-hints` is off by default. Set it to `true` to add the `+N` hints.

## Limits

The `--diff` ratchet checks and the exit-code gates remain CLI-only. The
language server is meant to guide you while you write, not to replace the CI
contract, so existing CI usage is unchanged.

Other limits of the current server:

- The server analyzes Python documents only. A document whose language is not
  `python` and whose path does not end in `.py` produces no hints and no
  warnings.
- The server declares full document sync, so it expects the whole buffer on
  every change. A change that carries a range is ignored, and one line is
  written to the server log.
- In a workspace with several folders, each folder reads its own configuration
  file. A document uses the deepest folder that contains it, and a document
  outside every folder uses the first folder. Folders that the editor adds or
  removes take effect without a restart.
- The `initializationOptions` of the client are ignored. Settings come from the
  configuration file.
- The `lsp` word reserves the first command-line argument: `complexipy lsp`
  starts the server. To analyze a file or directory named `lsp`, run
  `complexipy -- lsp`. On a terminal the server prints that hint when a path
  named `lsp` exists.
