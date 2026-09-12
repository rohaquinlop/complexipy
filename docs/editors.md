# Editor Integration

complexipy ships a Language Server Protocol (LSP) server, so your editor can
surface the same cognitive complexity analysis the CLI produces while you
write code. The server runs the same Rust engine as `complexipy` and the Python
API, and it needs no configuration to start being useful: install complexipy,
point your editor at `complexipy lsp`, and keep working.

Inside the editor you get:

- **Inlay hints** - the cognitive complexity of a function, shown at the end of
  its `def` line as `cognitive: 18`.
- **Hover** - hovering anywhere inside a function shows its name, its
  cognitive complexity, whether it exceeds the allowed threshold, and the title
  of the top refactoring suggestion when one exists.
- **Diagnostics** - one warning per function above `max-complexity-allowed`,
  with the message `cognitive complexity 18 exceeds the allowed 15` on a range
  that covers the whole function.

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

Zed registers language servers in `settings.json`. Add the server under `lsp`
and enable it for Python under `languages`:

```json
{
  "lsp": {
    "complexipy": {
      "binary": {
        "path": "complexipy",
        "arguments": ["lsp"]
      }
    }
  },
  "languages": {
    "Python": {
      "language_servers": ["complexipy", "..."]
    }
  }
}
```

The `"..."` entry keeps every other Python language server you already use
enabled. Without it, only the servers you list explicitly are started.

!!! warning "Not verified yet"

    Zed only launches language servers it knows about, and the `binary` block
    above is an override for a server it has already registered. If `complexipy`
    is not offered as a Python server when you edit the `language_servers` list
    in `settings.json`, Zed has not registered it and the block has no effect
    until an extension registers the server. The Neovim setup below has been
    tested; the Zed one has not.

## Configuration

The server reads the same configuration files as the CLI - `complexipy.toml`,
`.complexipy.toml`, or `pyproject.toml` (under `[tool.complexipy]`) - discovered
at the workspace root:

=== "complexipy.toml"

    ```toml
    max-complexity-allowed = 15

    [lsp]
    inlay-hints = "threshold"   # "always" | "threshold" | "never"
    per-line-hints = false
    diagnostics = true
    ```

=== "pyproject.toml"

    ```toml
    [tool.complexipy]
    max-complexity-allowed = 15

    [tool.complexipy.lsp]
    inlay-hints = "threshold"   # "always" | "threshold" | "never"
    per-line-hints = false
    diagnostics = true
    ```

| Key | Default | Description |
| -- | -- | -- |
| `max-complexity-allowed` | `15` | Functions strictly above this value are reported. A function at the limit passes. |
| `exclude` | `[]` | Glob patterns, matched relative to the workspace root. Excluded files produce no hints and no diagnostics. |
| `no-ignore` | `false` | Analyze functions even when an inline ignore comment suppresses them. |
| `lsp.inlay-hints` | `"threshold"` | When to show the per-function hint: `"threshold"`, `"always"`, or `"never"`. |
| `lsp.per-line-hints` | `false` | Also show a `+N` hint on every line with a non-zero complexity increment. |
| `lsp.diagnostics` | `true` | Publish warnings for functions above `max-complexity-allowed`. |

### Inlay hints

The hint is placed at the end of the `def` line. For a decorated function that
is the `def` line, not the `@decorator` line above it, and for a signature that
spans several lines it is the line where the signature starts.

- `inlay-hints = "threshold"` (the default) shows the per-function hint
  (`cognitive: 18`) only when the function is **above** `max-complexity-allowed`.
- `inlay-hints = "always"` shows it for every function.
- `inlay-hints = "never"` suppresses every hint, including per-line hints.
- `per-line-hints = true` additionally shows a `+N` hint at the end of every
  line that has a non-zero complexity increment. Lines with an increment of
  zero never produce a hint.

The server reports the same functions `complexipy <path>` reports: top-level
functions, plus methods of classes. A function nested inside another one is
folded into its parent, so there is no separate hint for it. Hovering inside a
nested function therefore shows its parent.

### Diagnostics

- `diagnostics = false` publishes no warnings at all.
- The reported threshold matches `complexipy <path>`: a function whose
  complexity **equals** `max-complexity-allowed` passes. Only strictly greater
  values are reported, as
  `cognitive complexity 18 exceeds the allowed 15`.

### Ignores and exclusions

- `exclude` globs are matched relative to the workspace root. Excluded files
  produce no hints and no diagnostics.
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
  block.

Alternatively, start it through `uv` without installing anything globally:

```lua
cmd = { "uvx", "complexipy", "lsp" },
```

### Where to read the server logs

The server never writes anything except protocol frames to stdout; all logging
goes to stderr. Read it where your editor collects it:

- Neovim: `:LspLog`.
- Zed: open the log panel from the command palette (`zed: open log`).

### Hints do not appear

Inlay hints are opt-in in most editors. Enable them in the editor itself
(`vim.lsp.inlay_hint.enable(true)` in Neovim), and remember that the default
`inlay-hints = "threshold"` only shows the hint when a function is above
`max-complexity-allowed`. Set `inlay-hints = "always"` to see every function.

### The whole file shows warnings

A limit lower than expected usually means the wrong configuration is active.
The server reads `complexipy.toml`, `.complexipy.toml`, or `pyproject.toml`
from the workspace root; check that the file the editor opened as the
workspace root is the one holding your settings. The server reads the file at
startup and again whenever the editor reports a configuration change, so after
editing it ask the editor to reload (`:LspRestart` in Neovim, or reload the
window in Zed).

### Per-line hints do not appear

`per-line-hints` is off by default. Set it to `true` to add the `+N` hints.

## Limits

The `--diff` ratchet checks and the exit-code gates remain CLI-only. The
language server is meant to guide you while you write, not to replace the CI
contract, so existing CI usage is unchanged.
