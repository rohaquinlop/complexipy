# Complexipy VSCode Extension

Cognitive complexity for Python in VS Code, as you type. The extension is a
client of the complexipy language server (`complexipy lsp`), so it runs the
same engine and reads the same configuration as the `complexipy` CLI.

## Features

- **Inlay hints**: the cognitive complexity of a function at the end of its
  `def` line, for example `cognitive: 18`.
- **Hover**: the function name, its complexity, whether it exceeds the
  threshold, and the top refactoring suggestion when one exists.
- **Warnings**: one warning per function above `max-complexity-allowed`, in
  the editor and in the Problems panel.
- **Status bar**: the number of functions over the threshold in the active
  Python file. Click it to open the Problems panel.

Results update on every edit. You do not need to save.

## Requirements

- Visual Studio Code 1.85.0 or higher.
- The `complexipy` command, from a release that includes `complexipy lsp`:

```bash
uv tool install complexipy
```

or `pip install complexipy`.

## Settings

- `complexipy.server.path`: the path to the `complexipy` executable. The
  extension runs `<path> lsp`. Leave it empty to use `complexipy` from the
  `PATH`.

If the extension cannot start the server, VS Code shows an error and the
status bar shows an error icon. Server logs are in the Output panel, in the
`complexipy` channel.

## Configuration

What appears is controlled by the complexipy configuration file in the
workspace folder: `complexipy.toml`, `.complexipy.toml`, or `pyproject.toml`
under `[tool.complexipy]`.

```toml
max-complexity-allowed = 15

[lsp]
inlay-hints = "threshold"
```

By default, hints and warnings appear only for functions above
`max-complexity-allowed`. Set `inlay-hints = "always"` to see a hint on every
function, and `per-line-hints = true` to see a `+N` hint on every line that
adds complexity. See [Editor Integration](https://complexipy.com/editors/) for every
setting.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

Run the tests with `npm test`.

## Changelog

See `CHANGELOG.md`.

## License

This extension is licensed under the MIT License - see the LICENSE file for details.
