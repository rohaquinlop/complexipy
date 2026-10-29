# Changelog

All notable changes to complexipy are documented here, newest first. Each
release section links to its GitHub release notes for the full details.

## Unreleased

### Added

- `--force-exclude`, also `force-exclude = true` in the configuration, applies
  `exclude` patterns to files passed explicitly as arguments. Without it, an
  explicit file is analyzed as before. Pass it under pre-commit, which names
  each changed file, so the configured `exclude` applies there.

### Changed

- Every CLI warning prints to stderr, and `--quiet` no longer hides warnings:
  it hides the report only. The `--diff` and `--staged` warnings moved from
  stdout to stderr, so they no longer mix with a report that another tool
  reads from a pipe
  ([#294](https://github.com/rohaquinlop/complexipy/issues/294)).
- The docs state that `--diff`, `--diff-only`, and `--staged` require Git 2.30
  or newer on `PATH`. Analysis without these options does not require Git
  ([#300](https://github.com/rohaquinlop/complexipy/issues/300)).
- The aarch64 musllinux wheel allocates with mimalloc, like the x86_64 one.
  On the django source tree it runs about 4 to 5 times faster
  ([#303](https://github.com/rohaquinlop/complexipy/issues/303)).

### Fixed

- The x86_64 musllinux wheel imports again. In 8.3.0 it failed on import with
  `initial-exec TLS resolves to dynamic definition`, because mimalloc used a
  thread-local storage model that musl cannot load from a Python extension.

## [8.3.0] - 2026-10-26

### Added

- A run gated by `--diff` or `--staged` shows which functions fail it. A
  REGRESSED or NEW function above the threshold is listed as `FAILED` and
  marked `FAILS` in the diff table, which ends with a `Gate:` line that counts
  them. A function above the threshold that the gate lets through is listed
  as `ALLOWED` instead of `FAILED`, and `--failed` lists only the functions
  that fail the run. A snapshot run lists the functions its snapshot lets
  through as `ALLOWED` too, instead of `PASSED`.
- SARIF output is diff-aware. With a diff, every result carries the
  `baselineState` of its function, `new`, `unchanged`, or `updated`, against
  the reference, and when the diff decides the exit code a complexity
  result's `level` is `error` for a function that fails the run, through the
  gate or the snapshot, and `note` for one the run lets through. The GitLab
  Code Quality report rates the same functions `major` and `info`.
  Fingerprints are unchanged.

### Fixed

- `--diff` and `--diff-only` read each file from the repository that holds
  it, so a run from outside the repository compares against the ref instead
  of reporting every function as `NEW`. A submodule, a repository that the
  repository around it tracks as a gitlink, is read at the commit the outer
  repository pins for it at the ref, unless the run starts inside it. A
  file outside any repository, a ref that names no commit in the file's
  repository, or a submodule without a pinned commit it has, still reports
  its functions as `NEW`, and the run now prints a warning that names the
  cause.
- `--diff-only` alone no longer warns that `--diff` and `--diff-only` are
  both set.
- `--staged` compares only the staged files the run selects: the path
  arguments, `paths`, and `exclude` apply as they do without it, so
  `complexipy one.py --staged` no longer reports the other staged files. They
  apply to the index, so a staged regular file is compared whatever the
  working tree holds at its path. A staged deletion that discovery would have
  selected still reports its functions as `REMOVED`: under a selected
  directory, outside hidden paths, and not matched by `exclude`, `.gitignore`,
  `.ignore`, or git's exclude file. Each file is named by the path the rest of
  the run reports.
- A rename or a move no longer fails `--diff` or `--staged` when the function's
  body stays as it was. A file that git pairs as renamed compares with its old
  path when the run covers that path, and as added when it moves in from outside
  the selected paths; a file moved out of them reports nothing. A function above
  the threshold that the change adds passes only when it replaces a `REMOVED`
  function with the same body and at least its complexity, each removed function
  replaced once, and the compared files (modified, renamed from a covered path,
  deleted from a covered path, or added) do not hold more functions with that
  body than they did at the ref, counting the functions that ignore comments
  hide. Bodies count and pair only within the repository that holds each file.
  Bodies compare structurally, without formatting, comments, the name, the
  decorators, or the parameters, so a function or class rename, a move to
  another file, and a method that becomes a function pass, and a function that
  moves and changes fails. The diff table names, in a `Gate` column, the
  `REMOVED` function each passing one replaces. `has_regressions` sees no
  bodies, so it still fails a moved function above the threshold. A function
  that an ignore comment now hides is not reported as `REMOVED`. A deleted
  Python file that discovery would have selected reports its functions as
  `REMOVED` in `--diff` too, and a deleted symbolic link reports nothing. The
  snapshot gate does not pair functions: a moved or renamed function above the
  threshold fails it until `--snapshot-create` records the function again, and a
  snapshot rewrite drops the entries of files that no longer exist.
- `--diff` reports the functions of a file that does not parse at the
  reference as `NEW`, as `--staged` does, instead of leaving the file out of
  the comparison.
- The `C002` loop guard negates its condition the way a linter expects:
  `x in y` becomes `x not in y`, `a is b` becomes `a is not b`, and a name or
  a call gets a bare `not`, so `--fix` no longer writes `E713`/`E714`
  violations. Parentheses stay around `and`, `or`, other comparisons,
  conditional expressions, lambdas, and walrus assignments.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/8.3.0)
for the full details.

## [8.2.0] - 2026-10-25

### Added

- `file_complexity()` and `code_complexity()` take `with_plans`, `True` by
  default. Pass `with_plans=False` to skip building refactor plans when only
  the scores are needed.
- The language server answers `complexipy/lineComplexities` with the total of
  every function and the increment of every line, whatever the hint settings
  say. The VS Code extension draws its colored badges from it
  ([#292](https://github.com/rohaquinlop/complexipy/discussions/292)).
- `[lsp] diagnostic-range` and `[lsp] hover` choose what the warning
  underlines and where hover answers.
- `--isolated` ignores every configuration file, and `--config <path>` reads
  only the named file: its `[tool.complexipy]` section when the file is named
  `pyproject.toml`, and the whole file otherwise. The two flags cannot be
  combined. Pass either one in CI to pin the gate, so a change to the
  repository's configuration cannot change it.

### Changed

- The language server is quieter by default. The warning underlines the
  function name instead of the whole function, and hover answers only on the
  `def` lines, so it no longer covers the editor's own hover inside the body
  ([#290](https://github.com/rohaquinlop/complexipy/discussions/290)). Set
  `diagnostic-range = "function"` or `hover = "function"` under `[lsp]` for the
  previous behaviour.
- Faster runs. File discovery walks the tree once, in parallel, and no longer
  reads gitignored directories; paths that fail to analyze are listed in path
  order. The default report, `--report-ignored`, and the stale ignore comment
  report share one read of each file. File arguments are analyzed in parallel,
  and `--fix` analyzes again only the files it wrote. The analysis cache keeps
  the entries of other targets unparsed, rewrites its file only when this
  run's results changed, and holds at most 32 MiB of entries. On macOS,
  Windows, and 64-bit Linux, complexipy allocates with mimalloc.
- Refactor plans are measured on their own function instead of the whole
  file, so `--suggest-refactors` grows linearly with the file size
  ([#274](https://github.com/rohaquinlop/complexipy/issues/274)). `--failed`,
  `--fix`, and `--dry-run` build plans only for functions over the limit,
  unless an output lists the others. The web demo builds no plans.
- `--diff` and `--staged` read every file at the ref through one
  `git cat-file --batch` process, and look each file up at its own repository
  path. A file that moved since the ref is reported as new, and a run from a
  subdirectory finds the history of each file.
- The language server analyzes on its own thread, so requests are answered
  while other documents are analyzed. Each document has its own debounce, and
  `$/cancelRequest` cancels a request that is still waiting.
- The Python API releases the GIL while it analyzes and while `compute_diff`
  runs. `functions`, `line_complexities`, and `refactor_plans` return a new
  list over objects built once, instead of copying the data on every access.
- A bare `complexipy` run whose configuration file sets no `paths`, or an
  empty list, stops with the missing paths error instead of analyzing nothing
  and passing. Set `paths` in the file or name the paths on the command line.
- `paths`, `output`, and `cache-dir` read from a configuration file must stay
  inside the directory complexipy runs from, symbolic links included, or the
  run stops with an error that names the key. The same values on the command
  line are not restricted, so pass a path outside the project there. A diff
  reference that starts with `-`, from `--diff`, `--diff-only`, or the
  `[diff]` `branch`, also stops the run.
- complexipy reads the first configuration file that parses and has valid
  values, and never merges files. The language server now moves on to the
  next file when one has an invalid value, as the CLI does. When a
  higher-priority file shadows a lower one that also holds settings, every
  run prints a warning that names the file not applied, so a project with
  both `complexipy.toml` and a `[tool.complexipy]` section in
  `pyproject.toml` sees it until it keeps its settings in one file. A
  configuration file that is not a regular file, that is larger than 4 MiB,
  or that is a symbolic link resolving outside its directory is skipped with
  a warning. A configuration error names the file and the line and column,
  or the key, instead of quoting the file.
- A configuration file that sets a `[diff]` `branch`, `[diff]`
  `staged = true`, or `snapshot-create = true` prints a warning on stderr on
  every run the setting applies to, even under `--quiet`. The warning names
  the file and says how the setting changes the exit code. Pin the gate in CI
  with the matching flag, `--isolated`, or `--config <path>`, and create a
  snapshot with `--snapshot-create` on the command line instead of in the
  file.
- Every function a module defines is scored on its own: a `def` under a
  module-level `if`, `try`, `with`, `for`, `while`, or `match`, and a method
  of a nested class, named `Outer::Inner::method`. These functions appear in
  every output and the gate checks them, so a run that passed before can fail
  on one, and a file's total can change. A function defined inside another
  function still counts toward the outer one.
- An ignore marker counts only inside a comment: on the `def` line, on a line
  of its signature or decorators, or on its own line directly above. The same
  text in a string, a docstring, or a decorator argument, or in a trailing
  comment on the code line above, no longer suppresses the function. The
  marker words must be whole and separated by spaces or tabs only, so
  `# complexipy: ignored` is not a marker. Move such a marker into a comment
  on the `def` line.
- A source file with a PEP 263 coding cookie is read in that encoding: UTF-8
  and Latin-1 always, and an ASCII-compatible encoding such as `cp1252` when
  the file is pure ASCII, so a Latin-1 file with non-ASCII bytes, which failed
  before, is now analyzed. A file that declares any other encoding, such as
  `utf-7`, or that declares an ASCII-compatible encoding and holds non-ASCII
  bytes, is reported as failed, and `file_complexity()` raises `ValueError`
  for it.
- Functions of one file that share a name, such as a property's getter and
  setter, get separate entries in the snapshot, in `--diff`, and in
  `compute_diff()`: the first in source order keeps the name, and the next
  ones are keyed `name#2`, `name#3`. A snapshot written before this change can
  fail on such a file; run `--snapshot-create` once to rewrite it.
- File discovery selects a `*.py` symbolic link only when it resolves to a
  regular file inside the analyzed directory. A link that leaves it, a
  dangling link, a device, a FIFO, a directory named `*.py`, and a directory
  that cannot be read are skipped, with a warning on stderr unless `--quiet`
  is set. A file whose name is not valid UTF-8 is reported as failed instead
  of skipped silently. A run that selects no file prints a warning even
  under `--quiet`, and `--report-ignored` also lists the `*.py` files that
  `.gitignore`, `.ignore`, and hidden-path rules skip.
- A source file larger than 64 MiB is reported as failed instead of read, and
  `file_complexity()` raises `ValueError` for it and for a path that is not a
  regular file, such as a device. A snapshot that is not a regular file of at
  most 64 MiB inside its own directory stops the run with an error, and a
  cache file like that is ignored. The default cache is not used when
  `.complexipy_cache` resolves outside the directory complexipy runs from.
- `--fix` rewrites only regular files inside the directory complexipy runs
  from. It skips a symbolic link, a file outside that directory, and a file
  that changed on disk after complexipy read it, and says so on stderr. It
  writes a new file next to the original and renames it into place, keeping
  the permission bits and the group but not setuid, setgid, extended
  attributes, or ACLs. Result files, the snapshot, and the cache are written
  the same way, so a symbolic link at an output path is replaced by a regular
  file instead of written through. `--fix --dry-run` writes nothing, not even
  the cache or the snapshot.
- Text that comes from the analyzed files or the configuration, such as a
  path, a function name, a source line, or a configuration value, prints with
  its control characters and bidirectional formatting characters escaped: an
  escape character shows as `\u{1b}` and a line break as `\u{a}`. Tabs print
  as they are. This covers the report, `--plain`, the `--fix` diff, the
  warnings, and the language server log.
- CSV output writes a cell that starts with `=`, `+`, `-`, `@`, a tab, or a
  carriage return with a leading `'`, so a spreadsheet shows it as text.
- When the configuration has a problem, such as an unknown rule id or a
  shadowed configuration file, the language server shows the editor one fixed
  warning that points to the complexipy output log, and writes the details
  only to that log.
- The VS Code extension runs in Restricted Mode instead of staying disabled
  there. In Restricted Mode it ignores a `complexipy.server.path` from
  workspace settings and runs the path from your user settings, or
  `complexipy` from the `PATH`, and trusting the workspace restarts the
  server with the workspace value. A relative `complexipy.server.path`
  resolves against the first workspace folder, where the server now starts,
  and relative `PATH` entries are skipped.
- Boolean operators add one per sequence of like operators, as the cognitive
  complexity paper specifies. A boolean group nested in another one was
  compared with every sibling of its parent, so an expression that mixes
  operators across groups scored too high: `(a or b) and (c and d)` now scores
  2 instead of 4, and `not a and (b or c)` scores 2 instead of 3. On a
  1,915-function corpus, 4 scores changed, each down by 1.
- GitLab Code Quality fingerprints no longer hash the function's line number,
  so a line added above a function keeps the fingerprint of every finding
  below it. A fingerprint now hashes the check, the path, the function name,
  and the finding's rank among the findings that share them. Every existing
  fingerprint changes once with this release, so the first comparison with a
  report from an earlier release shows every finding as new and every earlier
  one as fixed.
- SARIF reports percent-encode `artifactLocation.uri`, so a path with spaces,
  quotes, non-ASCII characters, `#`, `?`, or `%` points at the right file.
  `$schema` names the published OASIS SARIF 2.1.0 schema instead of a URL
  that returned 404. Each result carries `partialFingerprints`, which stay the
  same when lines are added above the finding, so uploads through a code
  scanning API do not duplicate alerts.
- `--quiet --ignore-complexity` exits 0, like `--ignore-complexity` alone; it
  exited 1 when a function was over the limit. When stdout closes early, as
  in `complexipy . | head`, the output stops and the run keeps its exit code,
  instead of panicking with exit code 101 from the binary or 1 from the
  console script. The language server answers a message that is not valid
  JSON or JSON-RPC with an error response and keeps running, instead of
  exiting with 1.
- An `exclude` pattern matches a path relative to each path argument, as
  before, and also relative to the working directory, so the CLI and the
  language server exclude the same files. With `paths = ["src", "legacy"]` and
  `exclude = ["legacy/**"]`, they used to exclude opposite files. A CLI run
  whose path argument differs from the working directory can exclude more
  files than before, and never fewer. The server also matches each pattern
  relative to each `paths` entry, resolves `..` in document paths before
  matching, and gives a nested workspace folder without a configuration file
  the configuration of the nearest enclosing folder that has one. A pattern
  that starts with `/`, `./`, or `../`, or ends with `/`, never matches, and
  the CLI and the server now warn about it. A file named on the command line
  is still analyzed when a pattern matches it, as in ruff. A
  `--force-exclude` flag that also excludes those files is tracked in
  [#293](https://github.com/rohaquinlop/complexipy/issues/293).
- C002 offers a loop guard replacement only when the nested `if` chain is the
  last statement of the loop body and each `if` the last statement of its
  parent. When statements follow the chain, the plan carries help text
  instead, because a `continue` guard would skip them: `--fix` used to write
  that guard, which could turn a `while` loop whose counter followed the
  chain into an infinite loop. C007 moves a single comment on the merged `if`
  lines, such as `# type: ignore`, to the merged line, and gives help text
  when those lines carry two or more comments or the inner `if` has its body
  on its header line.
- `--report-ignored`, `complexipy-ignored.json`, and the stale ignore comment
  report show a file inside the working directory at the same path as the
  results. They used to show paths relative to the parent of each analyzed
  directory, or to a file argument's own directory, so `complexipy .`
  prefixed every path with the working directory's name. Each result's
  `file_name` now matches its `path`, also for a file reached through a
  symlink.
- An internal error in the Python API raises `ValueError` from the analysis,
  diff, and ignore collection functions, and `RuntimeError` from `run_cli`
  and `run_lsp`, instead of `pyo3_runtime.PanicException`, which derives from
  `BaseException`. `except Exception` now catches it, so one file that hits
  an internal error no longer aborts a batch. The stubs and docs now name the
  errors each function raises.

### Fixed

- `--diff` and `--staged` read a path at the ref only when it was a regular
  file there. A path that was a symbolic link at the ref and is now a regular
  file compares as added instead of against the link's target text, so its
  functions above the threshold fail the gate. `--staged` also compares a
  staged change between a symbolic link and a regular file, which it skipped.
- `--staged` reads a staged file whose name starts like a stage number, such
  as `0:a.py`, instead of reading the staged `a.py` in its place.
- `branch = ""` in the `[diff]` section turns the diff off, and the section's
  `staged` value with it, as documented, so the run checks every function
  against the threshold. Before, it compared against the index instead, so
  the exit code checked only the functions that changed.
- `--diff` and `compute_diff()` read no old version of a path whose name
  holds a control character, so its functions report NEW. A name that ended
  in a carriage return read the old version of the same name without it.
- A lone CR (`\r`) and CRLF now end a line everywhere, as they do for the
  tokenizer and in LSP 3.17. A file with CR line endings no longer panics
  while refactor plans are built or while the caret under a suggestion is
  drawn, reports every function on line 1, lets one ignore comment suppress
  the whole file, or takes quadratic time and output. `--fix` keeps a file's
  CR line endings, and the column of the caret under a suggestion counts
  characters, not bytes.
- A long chained expression, such as `a + a + ... + a`, `a.b.b...`,
  `f()()...`, or `a[0][0]...`, no longer overflows the stack, which aborted
  the CLI and the language server and crashed the Python interpreter. A
  300,000-term chain now analyzes in the CLI, the Python API, the language
  server, and the web demo.
- Scoring a long boolean expression that mixes operators, and building
  refactor plans for a deep `if` chain, no longer take quadratic time.
- `--fix` keeps what the rewritten code does. An `else` written as `else :`,
  or one after a condition that holds a run of quotes, is no longer missed.
  `a or(b)` and `not a or(b)` are parenthesized and inverted correctly. Each
  line keeps its own indentation, so a file that mixes tabs and spaces stays
  valid, and a tab-indented file gets tab-indented replacements. An inner
  `if` with its body on its header line no longer loses that body, C002 no
  longer rewrites a chain inside a nested function, and a `#` inside a string
  no longer hides a later multi-line string from the check that refuses the
  rewrite.
- A file reached through several path arguments, such as
  `complexipy . src src/a.py`, or through a symlink next to it, is analyzed,
  reported, and fixed once.
- `--report-ignored` lists each suppressed location as
  `path:line  # comment`, followed by the count, as its help text and the
  docs describe. It used to print only the count.
- Suggestion snippets honor `--color no`, `NO_COLOR`, and piped output, and a
  comment in a snippet no longer colors every line after it. Color is decided
  on each run, so a Python host that calls `run_cli` several times follows
  each run's terminal and `NO_COLOR`.
- A `--diff` ref wider than the terminal no longer panics while the banner is
  drawn.
- `compute_diff(current, "origin/main")` works as documented:
  `invocation_path` defaults to `None`. Ctrl-C stops the console script at
  once instead of after the run finishes. A process forked after a parallel
  call no longer hangs on its next parallel call. The stubs and docs drop the
  nonexistent `DiffEntry.delta`, fix an `exclude` example that matched
  nothing, and show the real snapshot entry format.
- The language server reads each frame as it arrives, up to 256 MiB, so a
  huge `Content-Length` no longer makes it allocate that size or abort, and a
  broken frame is reported on stderr before the server stops. A request after
  `shutdown`, or one whose id is neither an integer nor a string, gets an
  `InvalidRequest` error instead of no answer. After a failed handshake the
  server exits instead of waiting for stdin to close. `file://localhost/...`
  resolves to the local path, and a URI with a remote host resolves to a UNC
  path on Windows and to no path elsewhere, so the server reads the right
  configuration. Hover shows a function name such as `Box::__init__`
  literally instead of rendering it as Markdown.
- The VS Code extension stops a server that does not start within 30 seconds
  and shows an error, so a later restart is no longer blocked. It keeps one
  output channel instead of adding one per restart.
- The web demo loads a fresh analyzer after an analysis crashes, instead of
  failing every later analysis until the page reloads.

### Security

- complexipy now treats the content of the analyzed repository as untrusted.
  This release fixes the issues described in
  [GHSA-8p2q-4h62-q593](https://github.com/rohaquinlop/complexipy/security/advisories/GHSA-8p2q-4h62-q593),
  [GHSA-9qgh-5xjp-8g3m](https://github.com/rohaquinlop/complexipy/security/advisories/GHSA-9qgh-5xjp-8g3m),
  [GHSA-wg3w-cmfg-c94w](https://github.com/rohaquinlop/complexipy/security/advisories/GHSA-wg3w-cmfg-c94w),
  [GHSA-mh97-m2f8-q8vj](https://github.com/rohaquinlop/complexipy/security/advisories/GHSA-mh97-m2f8-q8vj),
  and, for the VS Code extension 0.4.0,
  [GHSA-wpx6-54p3-527q](https://github.com/rohaquinlop/complexipy/security/advisories/GHSA-wpx6-54p3-527q).
  Upgrading is recommended for every user.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/8.2.0)
for the full details.

## [8.1.0] - 2026-10-18

### Added

- A language server: `complexipy lsp` speaks the Language Server Protocol on
  stdio, so any editor with an LSP client gets complexity feedback with no
  per-editor code. It answers inlay hint requests with the cognitive
  complexity of each function and, optionally, the increment contributed by
  each line; it answers hover requests with the function total, its threshold
  status, and the top refactoring suggestion, built on demand for the hovered
  function so large files stay responsive; and it publishes a warning for
  every function above `max-complexity-allowed`. Diagnostics respect inline
  ignore comments, so the editor and the CLI agree. The per-function hint sits
  at the end of the line that closes the declaration, so a wrapped signature
  keeps the hint on its closing line, and a document that does not parse keeps
  the last parsed hints plus one `complexipy-parse-error` warning, placed where
  the parser fails, until it parses again. Configuration problems, such as an
  unknown rule id in `select`, appear in the editor as a warning message
  ([#272](https://github.com/rohaquinlop/complexipy/issues/272)). The server
  reloads the configuration when a configuration file at a folder root changes,
  in editors that watch files for it
  ([#257](https://github.com/rohaquinlop/complexipy/issues/257)). A Zed
  extension in `zed/` starts the server in Zed, and the inlay hints carry a
  left padding so they do not touch the code
  ([#255](https://github.com/rohaquinlop/complexipy/issues/255)). See
  [Editor Integration](https://complexipy.com/editors/) for the VS Code, Neovim,
  and Zed setup.

- The VS Code extension is now a client of the language server and drops its
  bundled WASM engine
  ([#253](https://github.com/rohaquinlop/complexipy/issues/253)). Results
  update while you type, with no save
  ([#133](https://github.com/rohaquinlop/complexipy/issues/133)), and hints
  and warnings follow `max-complexity-allowed` and the
  `[tool.complexipy.lsp]` settings
  ([#127](https://github.com/rohaquinlop/complexipy/issues/127)). The
  extension needs the `complexipy` command, and a `complexipy.server.path`
  setting points it at a specific binary.

- The VS Code extension is published to OpenVSX as well as the Visual Studio
  Marketplace, so VSCodium and other VS Code forks can install it
  ([#145](https://github.com/rohaquinlop/complexipy/issues/145)). A
  `vscode-v*` tag publishes the same VSIX to both registries.

- A `[tool.complexipy.lsp]` configuration section, with `inlay-hints`
  (`threshold`, `always`, or `never`), `per-line-hints`, and `diagnostics`.
  Hints are gated on `max-complexity-allowed` by default, so a project that
  sets no options sees feedback only where it matters.

- `complexipy-core` gains a `config` feature that owns configuration file
  discovery, so the CLI and the server read `complexipy.toml`,
  `.complexipy.toml`, and `pyproject.toml` through one code path.

- The server applies `exclude` one pattern at a time: a malformed pattern is
  reported once per configuration load and skipped while the remaining patterns
  keep applying, and a list too large to compile as one program is reported once
  and still applied. A CLI run stops on either.

- Each workspace folder reads its own configuration
  ([#273](https://github.com/rohaquinlop/complexipy/issues/273)). In a
  workspace with several folders, a document uses the threshold, rule
  selection, `exclude` patterns, and `[tool.complexipy.lsp]` settings of the
  deepest folder that contains it, and a document outside every folder uses
  the first folder. The server declares workspace folder support, so folders
  that the editor adds or removes take effect without a restart.

- Per-rule suppression and rule selection
  ([#208](https://github.com/rohaquinlop/complexipy/issues/208)). `--select`
  and `--ignore` (repeatable and comma-separated) pick the active refactoring
  rules, and `ignore` wins on conflict. `complexipy.toml`,
  `.complexipy.toml`, and `pyproject.toml` take matching `select` and `ignore`
  keys, and a flag replaces the toml key it names. A toml value splits on
  commas like the flags, so `select = "C007, C002"` names two rules. An
  ignore comment accepts a
  bracketed rule list, `# complexipy: ignore[C007]`, which suppresses only the
  named rules for one function, while a bare marker keeps suppressing the
  whole function. An inactive rule disappears from every output format and
  never consumes a slot of the per-function plan cap, so a hidden rule can
  never block a visible one. An unknown rule id warns on stderr that it
  matches no registered rule, and a second warning says so when select and
  ignore leave no rule active. Rule selection never changes the exit code.

- `--fix` applies machine-applicable suggestions to the source files
  ([#204](https://github.com/rohaquinlop/complexipy/issues/204)). A
  suggestion is fixable when its tier is `MachineApplicable` and its
  replacement is a faithful splice (`spliceable`), which covers C002 (loop
  guards) and C007 (collapsible if); C005 is not spliceable and stays out.
  `--fix` changes only functions whose complexity is above
  `max-complexity-allowed`, and every pass checks the threshold again, so a
  function stops changing once it passes. Functions at or under the
  threshold keep their suggestions in the report and are never rewritten.
  One run fixes every analyzed file, bottom-to-top within each file, and
  prints a syntax-highlighted unified diff of the applied changes, with
  line numbers, one context line per hunk, and the rule that fired, plus a
  summary of fixed and skipped fixes that shows the reduction per fix. Every
  summary line carries the file's line numbers from before the run, and each
  skip reports once. The run repeats pass by pass, up to 8 passes, until no
  fixable plan remains,
  so a fix that exposes a new fixable plan is applied in a later pass;
  later passes label their diffs. Long diff rows truncate at the terminal
  width. When nothing is fixable the run prints `No fixes to apply.`
  `--fix` runs in any git state: uncommitted changes, untracked files, and
  a missing repository never block it, and no extra flag is needed. A fix
  keeps the file's line endings and its trailing newline. A fix that would produce invalid Python is reverted and
  reported, and the write is a parse-checked temp file renamed over the
  original with its permissions kept. The report and the gates measure the
  post-fix state, so re-analyzing a fixed file shows the finding gone and
  the exact measured reduction. `--fix --dry-run` prints only the diff and
  writes nothing. `--select`/`--ignore` decide which rules
  may fix.

- Python 3.15 support
  ([#265](https://github.com/rohaquinlop/complexipy/issues/265)). Lazy
  imports (PEP 810) and unpacking in comprehensions (PEP 798) now parse in
  the CLI, the language server, the Python API, and the web build. A lazy
  import adds no complexity, and an unpacking comprehension scores like the
  plain one. Releases now publish CPython 3.15 wheels for Windows and macOS.

- A JSON schema for the configuration
  ([#270](https://github.com/rohaquinlop/complexipy/issues/270)).
  `complexipy.schema.json` describes every key of `complexipy.toml`,
  `.complexipy.toml`, and `[tool.complexipy]`, including `[diff]` and
  `[lsp]`, so a TOML language server such as Taplo offers completion, hover
  text, and validation. The schema rejects unknown keys. complexipy itself
  still ignores them.

### Changed

- `lsp` is a reserved first argument: `complexipy lsp` starts the language
  server, so a file or directory with that name needs `complexipy -- lsp` to be
  analyzed. On a terminal the server prints that hint when both the reserved
  word and a path named `lsp` are present.
- `StringOrList` moved from `complexipy-cli::types` to the new
  `complexipy-core` `config` module, and the CLI still re-exports it from its
  old path. Every workspace crate remains `publish = false`, so no published
  Rust API changes.
- Added `manylinux_2_31_riscv64` wheel builds for RISC-V support. The
  release matrix now separates the Rust toolchain triple (`target`) from
  the platform shorthand (`arch`), which keeps the existing wheel and
  artifact names unchanged. (#259)
- `RuleCategory`, `Applicability`, and `DiffStatus` are real `enum.Enum`
  subclasses: they iterate, they carry `.name` and `.value`, and
  `isinstance(x, enum.Enum)` holds.
- `requires-python` is now `>=3.8,<3.16`, so installers stop choosing
  complexipy on a Python version no release has built or tested. The bound
  rises when a release adds the next Python version. On Python 3.16 and later,
  pip falls back to 8.0.1, the last release without the bound.
  ([#281](https://github.com/rohaquinlop/complexipy/issues/281))

### Fixed

- `--color` governs every styled line the CLI writes, on stdout and stderr.
  `auto` turns color off when the stream is not a terminal or `NO_COLOR` is
  set, `no` removes it from the report, the banner, and the warnings, and
  `--plain` output is never styled. In 8.0.1 the report carried ANSI codes on
  a pipe, under `--color no`, and under `NO_COLOR`.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/8.1.0)
for the full details.

## [8.0.1] - 2026-09-06

### Changed

- Docs, package metadata, and embedded links (rule doc URLs in
  `--suggest-refactors` output, SARIF report URIs) now point to the
  `complexipy.com` custom domain instead of the GitHub Pages default.
  (#252)

### Fixed

- The sdist now includes the `LICENSE` file, which was previously missing
  from the published source distribution.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/8.0.1)
for the full details.

## [8.0.0] - 2026-09-03

!!! note "Migration"

    See the [migration guide](https://rohaquinlop.github.io/complexipy/migration/)
    for the breaking changes below.

### Changed

- The Python CLI is retired: `complexipy` is now a native Rust
  implementation exposed through a thin console-script shim, so the whole
  pipeline runs without a Python interpreter. The Python API is unchanged
  and now includes the diff comparison (`compute_diff`, `has_regressions`,
  `DiffEntry`, `DiffStatus`) backed by the same Rust engine as the CLI.
  The extension module keeps being distributed via maturin wheels.
  ([#224](https://github.com/rohaquinlop/complexipy/issues/224), [#243](https://github.com/rohaquinlop/complexipy/issues/243))

### Removed

- **Git-URL analysis** (`complexipy <repository-url>`) - local paths only.
- **Legacy cache migration** - the previous
  `.complexipy_cache/<hash>.json` layout is no longer migrated; existing
  legacy files are ignored and the delta history starts fresh.
- **Legacy snapshot files** - snapshots created with older versions are no
  longer detected; recreate them with `--snapshot-create`.
- **`-mx` short flag** for `--max-complexity-allowed` - carried over from
  the Python CLI but never ported to the clap-based parser; use
  `--max-complexity-allowed` instead.

### Fixed

- The C007 collapsible-if rule no longer suggests merging a nested `if`
  when a same-level statement with side effects precedes it - the merge
  would change program behavior. Comments in multiline headers and
  trailing comments are handled instead of being misplaceable by the
  replacement.
  ([#228](https://github.com/rohaquinlop/complexipy/issues/228), [#236](https://github.com/rohaquinlop/complexipy/issues/236))
- The C007 preceding-statement guard no longer fails when blank or
  comment-only lines sit between the outer and inner `if`: the indent
  step detection now skips such lines, so the merge is still rejected
  instead of producing a replacement that drops code.
  ([#245](https://github.com/rohaquinlop/complexipy/issues/245))
- The C002 loop-guards suggestion keeps statements that sit between the
  chained `if`s: they now appear in the replacement between the
  corresponding guards instead of being dropped. Guard conditions are now
  parenthesized (`if not (<cond>):`), so inverting conditions with `and`
  or `or` no longer silently changes what the guard skips.
- The C005 extract-predicate suggestion keeps the statement keyword:
  `while` conditions stay `while` loops, and `elif` conditions get help
  text only instead of a broken standalone `if` replacement. The
  predicate body indent now follows the file's indent step.
- C002 and C007 refuse machine suggestions when the shifted body holds a
  multi-line string literal: dedenting would change the string's value.
  The plans carry help text instead.
- C002 loop guards strip a redundant top-level `not` from the guard
  condition (`if not a:` becomes guard `if a:`), and C005 predicate names
  get an underscore suffix when the source already defines the generated
  name.
- C005 extract-predicate now emits a module-level helper whose parameters
  are the condition's free variables (attribute bases included, builtins
  excluded), so the helper is unit-testable. The snippet shows the
  enclosing context at the statement's real indentation. Conditions that
  bind a name (`:=`) or contain a lambda/comprehension get help text only.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/8.0.0)
for the full details.

## [7.0.1] - 2026-08-12

### Changed

- The release pipeline now retries artifact uploads with pinned workflow
  versions and only notifies downstream repositories on tag releases,
  keeping non-tag pushes silent.
- Pinned maturin version in the release workflow to avoid GitHub API
  rate limits when resolving the latest version inside Docker build
  containers.

### Fixed

- Snapshot entries for analyzed files are updated in place instead of
  being moved to the end of the file: partial runs (e.g. pre-commit hooks
  analyzing only staged files) no longer reorder the snapshot, so the
  snapshot stays byte-identical across commits that do not change
  complexity. ([#226](https://github.com/rohaquinlop/complexipy/issues/226))

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/7.0.1)
for the full details.

## [7.0.0] - 2026-08-10

!!! note "Migration"

    The deprecated `--output-json`, `--output-csv`, `--output-gitlab`,
    `--output-sarif`, and `--ratchet` flags and their TOML keys were
    removed; use `--output-format` and `--diff` instead. See the
    [migration guide](https://rohaquinlop.github.io/complexipy/migration/)
    for each removed flag and key with its replacement.

### Added

- `--suggest-refactors` is now a clippy-style lint system: stable rule IDs
  (C001-C005, C007, C011), category and applicability metadata,
  `path:line:col` anchors with caret spans, verbatim suggestion rendering,
  and a documentation link per rule. (#209)
- Refactor-plan findings are included in JSON, SARIF, and GitLab exports
  when `--suggest-refactors` is passed; the SARIF rule catalog is built
  dynamically from the plans encountered. (#209)
- `compute_diff`, `has_regressions`, `DiffEntry`, and `DiffStatus` are now
  part of the public Python API, so CI tools can consume diff results as
  objects instead of parsing terminal output; `DiffStatus` provides named
  constants such as `DiffStatus.REGRESSED`. (#210)
- `collect_removable_ignored_locations()` and `RemovableIgnore` are exported
  from the Python API. Every run now reports ignore comments that are no
  longer necessary (`path:line function=X complexity=N <comment>`) when the
  suppressed function is back under the allowed limit - the exit code is
  unaffected, the report is suppressed under `--quiet`, and it works under
  `--plain`. (#213)
- `--staged` flag for git-index comparison - answers "what complexity am I
  about to commit?" instead of only what changed in the working tree;
  `--staged` alone defaults the baseline to `HEAD` and enforces, while
  `--diff <ref> --staged` enforces against the ref. (#218)
- `[tool.complexipy.diff]` TOML section so the comparison policy lives in
  the repository config: `branch = "main"` makes a plain `complexipy .` run
  behave like `--diff main` (enforcement included), `staged = true` enables
  staged comparison by default, and `branch = ""` opts out. CLI flags take
  precedence over the section. (#219)
- Refactor reductions are now measured instead of estimated: for every
  machine-applicable suggestion (C002, C007) the replacement is spliced
  into the source, re-parsed, and re-scored, so `estimated_reduction` and
  `estimated_complexity_after` report the literal delta of applying the
  suggestion - and ranking, overlap resolution, and the noise filter all
  operate on measured values. The new `reduction_is_measured` flag on
  `RefactorPlan` separates measured plans from help-only formula
  estimates, which the CLI renders with a `~` qualifier
  (`Estimated reduction: ~-2`) while measured plans render plain
  (`Reduction: -2`). Guard suggestions are now faithful splices for loops
  with statements before or after the if-chain and for multi-line loop
  headers; measurement failures fall back to the formula estimate - never
  a panic, never a fabricated number. (#225)

### Changed

- Refactor-plan reduction estimates are now honest: the reduction math was
  rewritten and validated against measured before/after complexity, C004 no
  longer suggests splitting `match` statements, C006 was deleted because its
  gate could never fire, and C011 now fires on `try` → `with` → `try`
  chains. Overlapping plans are deduped against every overlap and capped at
  5, reporting dropped ones as "... and N more suggestions". (#209)
- Condition extraction in the refactor rules now tracks bracket depth,
  string literals, and walrus `:=` instead of a naive `rfind(':')`. (#209)
- `file_complexity()` now returns cwd-relative paths (matching
  `git diff --name-only`), and nested invocations resolve git paths via a
  `git ls-files` basename lookup - without this, diffing per-file results
  silently marked every function as NEW. (#210)
- Community standards files were added: `CODE_OF_CONDUCT.md`,
  `CONTRIBUTING.md`, `SECURITY.md`, issue templates, and a pull request
  template. (#201)

### Fixed

- Snapshot updates now merge with the existing snapshot instead of
  replacing it: only the files analyzed in a run are touched, so partial
  runs (e.g. pre-commit hooks analyzing only staged files) no longer erase
  the baseline for unanalyzed files. (#215)
- The docs footer now renders its links as styled links instead of raw
  markdown text, on both the English and Spanish landing pages. (#216)

### Removed

- `RefactorPlan.steps` - replaced by the concrete `suggestion` / `help`
  fields on the plan. (#209)
- `CodeSnippet` from the public API; `CodeSuggestion` is exported in its
  place. (#209)
- `--output-json` / `-j`, `--output-csv` / `-c`, `--output-gitlab`, and
  `--output-sarif` / `-sr` - use `--output-format` instead. (#221)
- `--ratchet` / `-R` - `--diff` enforces the threshold by default. (#221)
- TOML keys `output-json`, `output-csv`, `output-gitlab`, `output-sarif`,
  flat `staged`, flat `ratchet`, and the undocumented `details = "low"`
  alias. (#221)

These removals land in 7.0.0. See the
[migration guide](https://rohaquinlop.github.io/complexipy/migration/) for
each removed flag and key with its replacement.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/7.0.0)
for the full details.

## [6.2.0] - 2026-07-23

### Added

- `--exclude` and `--output-format` now accept comma-separated values
  (`--exclude tests/**,src/**`), avoiding shell expansion issues. (#199)

### Fixed

- Output now shows the correct relative path from the current working
  directory when analyzing a single file or a directory. (#198)
- `--output-sarif` help text now carries a deprecation notice, clarifying
  that SARIF output is moving to a different mechanism. (#199)

### Changed

- Decomposed the monolithic 316-line `main()` function into a clean
  orchestrator (~80 lines) with extracted business logic in domain-specific
  `utils/` modules (`config.py`, `paths.py`, `ignored.py`). (#199)
- Introduced `RunConfig`, `ExitReport`, and `SnapshotEvaluation` dataclasses,
  replacing mutable accumulators and positional tuple unpacking. (#199)
- Eliminated all `global console` declarations - the `console` instance is
  constructed once and passed explicitly. (#199)
- Refactored `get_arguments_value` from a 20-parameter function returning a
  15-element tuple to a dict-based approach. (#199)
- Added 30 new tests covering config resolution and snapshot evaluation
  logic. (#199)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/6.2.0)
for the full details.

## [6.1.0] - 2026-07-21

### Fixed

- Fixed git path resolution when running `--diff` from a nested subdirectory
  inside a repository, which previously caused "not a git repository"
  errors. (#196)
- Fixed ruff lint configuration to use a glob pattern instead of a directory
  exclusion for test files. (#196)

### Changed

- Simplified the diff CLI - `--diff` now always enforces
  `--max-complexity-allowed`, while `--diff-only` provides the visual-only
  comparison. `--ratchet` is deprecated in favour of this model. (#196)
- Integrated diff output into the main analysis flow instead of producing it
  as a separate post-processing step. (#196)
- Removed the redundant "Failed functions" output section - failed functions
  are now shown inline in the per-file summary. (#196)
- Added `AGENTS.md` for AI agent context.
- Added a CI workflow that notifies downstream repositories on new
  releases. (#197)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/6.1.0)
for the full details.

## [6.0.1] - 2026-07-03

### Fixed

- Normalized Windows paths for wax glob compatibility in the exclude
  handling. (#194)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/6.0.1)
for the full details.

## [6.0.0] - 2026-06-26

!!! note "Migration"

    This release brings the cognitive complexity algorithm into full
    conformance with the SonarSource white paper (v1.7). Scores change for
    files using `match`, `try`/`except`, `with`, comprehensions, lambdas,
    recursion, or nested ternaries - in most cases they increase. Re-run
    `complexipy` after upgrading and review the new scores; if you use
    `--max-complexity` / `--failed` in CI you will likely need to raise
    thresholds. See the [migration guide](https://rohaquinlop.github.io/complexipy/migration/)
    for upgrade guidance.

### Fixed

- `match` statements now apply a structural + nesting increment per the
  paper, instead of being scored as 0. (#192)
- `try`/`else`/`finally` blocks are now collected at the current nesting
  level instead of `+1`, matching the paper's rule. (#192)
- `except` handlers now charge `1 + nesting_level` instead of a flat
  `+1`. (#192)
- `with` blocks no longer incorrectly raise the nesting level. (#192)
- Direct recursion now emits `+1` for each self-call via a scope-aware
  `RecursionFinder`, correctly skipping nested function/class definitions
  and lambdas. (#192)
- Lambda expressions are now included in boolean-op counting, recursing
  into the body at `nesting_level + 1`. (#192)
- Comprehensions (`ListComp`, `SetComp`, `DictComp`, `Generator`) now charge
  `1 + nesting_level` per generator and `+1` per `if` filter. (#192)
- Nested ternaries now recurse at `nesting_level + 1`, so inner ternaries
  receive the correct nesting increment. (#192)
- `for`/`while` `else` clauses are now collected at the loop's own nesting
  level, not `+1`. (#192)
- Bare expression statements now count boolean ops (e.g.
  `foo(a and b)`). (#192)
- The `for` iterable now counts boolean ops, for parity with `while`. (#192)

### Changed

- Extracted all path orchestration (file I/O, directory traversal, URL
  cloning) from `cognitive_complexity.rs` into a new `src/runner.rs` module,
  gated `#[cfg(feature = "python")]`, which also resolves a pre-existing
  wasm build failure. (#192)
- Extracted `push_line`, `absorb`, `absorb_with_regions`, `finalize_region`,
  `count_line_bool_ops`, `loop_complexity`, `is_ignored`, and
  `analyze_function` helpers to eliminate the repeated fold/push pattern
  across all statement arms. (#192)
- Unified structurally identical `Stmt::For` and `Stmt::While` handling into
  a single `loop_complexity` call. (#192)
- Collapsed 7 near-identical statement arms through
  `count_line_bool_ops`. (#192)
- Derived `Default` on `ComplexityRegion` and `RegionKind`. (#192)
- Eliminated `merge_child` and its internal `child.regions.clone()`. (#192)
- Hardened the Windows release unit-test job against transient Cargo HTTP/2
  framing failures. (#189)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/6.0.0)
for the full details.

## [5.6.1] - 2026-06-16

### Fixed

- Fixed a panic in `extract_comment_marker` when a multi-byte UTF-8
  character (em-dash, emoji, accented letters, CJK) straddles byte offset 16
  of a comment, which previously aborted analysis of the entire directory.
  Error-prone byte-slicing was replaced with regex-based matching. (#187)

### Removed

- Removed the unreliable `release-plz.yml` auto-release workflow and its
  supporting files (`CHANGELOG.md`, `cliff.toml`). Manual releases via
  `release.yml` (tag → build → test → publish) are now the standard
  path. (#188)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/5.6.1)
for the full details.

## [5.6.0] - 2026-06-14

### Added

- `--no-ignore` flag to disregard `# complexipy: ignore` and
  `# noqa: complexipy` comments. (#182)
- `--report-ignored` flag to list all suppressed functions, with optional
  JSON export to `complexipy-ignored.json`. (#182)
- `IgnoredLocation` type and `collect_all_ignored_locations()` to the
  Python API. (#182)

### Fixed

- Fixed diff mode showing all files as NEW on Windows. (#177)

### Changed

- Migrated pyo3 to 0.29 and updated CI runners. (#185)
- Added musllinux_1_2 wheel builds for Alpine Linux support. (#180)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/5.6.0)
for the full details.

## [5.5.0] - 2026-05-22

### Added

- Deterministic refactor plans: a new algorithm generates actionable,
  deterministic suggestions to reduce cognitive complexity. Plans are
  displayed in the rich CLI output and included in JSON output.
- Recursive exclude globs: exclude patterns now support `**` (e.g.
  `tests/**`). The glob engine was replaced with `wax` for correct recursive
  matching relative to the caller's working directory.

### Fixed

- Fixed unbounded growth of target-set entries in the cache that could
  degrade performance on large projects.

### Changed

- Output internals now use typed `FunctionRow` / `FileEntry` dataclasses
  instead of untyped `Dict` structures, extracted into
  `complexipy/utils/dataclasses.py`.
- Docs deploys now trigger only on release events, not on every push.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/5.5.0)
for the full details.

## [5.4.1] - 2026-05-05

### Fixed

- Stabilized snapshot output by omitting transient line-level fields from
  Python-facing serialized Rust structures.
- Moved shared output constants into `complexipy.utils.constants` so output
  filenames, legacy CLI flags, and legacy TOML keys are defined in one
  place.
- Clarified deprecated output flag messaging for `--output-csv`,
  `--output-json`, and `--output-gitlab` to point users at
  `--output-format`.

### Changed

- Updated CI so pull requests run a faster quick-test matrix, while full
  wheel builds, source distribution builds, and full platform test jobs run
  for tags or manual workflow dispatches.
- Enabled uv dependency caching in CI jobs.
- Updated README and docs to match the current Python API return types and
  the current JSON array snapshot format.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/5.4.1)
for the full details.

## [5.4.0] - 2026-04-25

### Removed

- Removed comprehension complexity scoring - rolls back the
  `ListComp`/`SetComp`/`Generator`/`DictComp` AST node handling from
  `count_bool_ops()` and the `count_comprehension_complexity()` helper added
  in v5.3.0. (#166)

### Fixed

- Fixed feature gating for WASM builds and included version metadata. (#164)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/5.4.0)
for the full details.

## [5.3.0] - 2026-04-16

### Added

- `--ratchet` flag - CI fails only when complexity increases past the
  threshold; regressions are blocked, improvements always pass. (#159)
- `--plain` flag - machine-readable plain-text output for
  scripting/piping. (#158)
- `--top N` flag - display the N most complex functions, globally sorted
  across all files. (#157)
- `--check-script` / `--script-strict` flags - analyse module-level (script)
  complexity in addition to functions. (#156)
- Unified output destinations - consistent `--output-*` routing across all
  report formats. (#155)
- GitLab Code Quality report via `--output-gitlab`. (#153)
- SARIF 2.1.0 output via `--output-sarif` for IDE and GitHub Advanced
  Security integration. (#141)
- Git diff analysis - `--diff <ref>` reports complexity changes relative to
  any git reference. (#140)
- Comprehension complexity - list/dict/set comprehensions and generator
  expressions now contribute to cognitive complexity scores. (#139)
- `# complexipy: ignore` - new canonical inline suppression comment;
  `# noqa: complexipy` is deprecated. (#146)
- Glob patterns in the config `exclude` field. (#142)
- Spanish documentation. (#147)

### Fixed

- `--top` results now preserve global descending order across multi-file
  runs.
- `--top N` rejects `N ≤ 0` with a clear error.
- `--script-strict` now correctly requires `--check-script`.
- Ignore markers (`# complexipy: ignore`) now work on multiline function
  definitions.
- JSON output includes a final newline (POSIX compliance). (#148)
- Snapshot-allowed functions are now shown as `PASSED` in output.
- The snapshot watermark correctly drives the exit code when active.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/5.3.0)
for the full details.

## [5.2.0] - 2026-01-28

### Fixed

- Fixed `# noqa: complexipy` comments on decorated functions. (#128)

### Changed

- Updated the pre-commit complexipy version in the docs. (#126)
- Updated the documentation. (#132)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/5.2.0)
for the full details.

## [5.1.0] - 2025-12-09

### Fixed

- Fixed invalid results output paths on Windows. (#120)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/5.1.0)
for the full details.

## [5.0.0] - 2025-11-26

!!! note "Migration"

    Conditional scoring now counts each `elif`/`else` branch as +1
    complexity (plus its boolean test), aligning with Sonar's
    cognitive-complexity rules. Expect higher scores for branching. See the
    [migration guide](https://rohaquinlop.github.io/complexipy/migration/)
    for upgrade guidance.

### Added

- Snapshots: `--snapshot-create` writes `complexipy-snapshot.json`;
  comparisons block regressions, auto-refresh on improvements, and can be
  bypassed with `--snapshot-ignore`. (#111)
- Change tracking: a per-target cache in `.complexipy_cache` shows
  deltas/new failures for over-threshold functions using stable BLAKE2
  keys. (#115)
- Output controls: `--failed` to show only violations (#114); `--color auto|yes|no`
  (#109); richer summaries of failing functions and invalid paths.
- Python 3.14 support. (#106)

### Changed

- Excludes and errors: exclude entries are resolved relative to the root and
  only applied when they match real files/dirs; missing paths are reported
  cleanly instead of panicking. (#113)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/5.0.0)
for the full details.

## [4.2.0] - 2025-09-21

### Added

- Exclude files from analysis.
- Inline ignores to exclude functions from analysis.

### Fixed

- Python 3.8 support. (#96)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/4.2.0)
for the full details.

## [4.1.0] - 2025-09-08

### Added

- Version support. (#93)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/4.1.0)
for the full details.

## [4.0.2] - 2025-08-22

Patch release; no changes were recorded in its release notes.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/4.0.2)
for the full details.

## [4.0.1] - 2025-08-21

### Fixed

- Fixed the PyPI README error.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/4.0.1)
for the full details.

## [4.0.0] - 2025-08-21

!!! note "Migration"

    The logic for counting boolean operators in conditions was updated to
    align with the definition from the original paper. Existing functions
    may report higher complexities. See the
    [migration guide](https://rohaquinlop.github.io/complexipy/migration/)
    for upgrade guidance.

### Added

- Configuration support via `complexipy.toml` or `[tool.complexipy]` in
  `pyproject.toml` - users can now define default arguments.

### Fixed

- Fixed an infinite loop when analyzing modules with invalid Python
  syntax. (#85, resolved in #88)

### Changed

- Improved performance and Rust implementation details.
- Updated and improved documentation.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/4.0.0)
for the full details.

## [3.3.0] - 2025-07-17

### Added

- `--max-complexity-allowed` (`-mx`) - customize the maximum allowed
  cognitive complexity threshold per function. The default remains 15 to
  maintain existing behavior.
- GitHub Actions integration for the custom threshold.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/3.3.0)
for the full details.

## [3.2.0] - 2025-07-09

### Fixed

- Fixed an error when using complexipy on Windows, related to the `rich`
  library used to draw the console output with emojis.
- Fixed the `quiet` parameter still drawing output that wasn't handled in
  the Rust code.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/3.2.0)
for the full details.

## [3.1.1] - 2025-07-06

### Changed

- The maximum complexity threshold is now `15`, matching the Sonar
  threshold to make adoption of the library easier. (#78)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/3.1.1)
for the full details.

## [3.0.0] - 2025-06-16

!!! note "Migration"

    `--max-complexity` was removed; complexipy now uses a fixed cognitive
    complexity threshold of 15. The tool exits with an error when a function
    meets or exceeds the threshold. Use `--ignore-complexity` (`-i`) to show
    all functions regardless of their complexity score. See the
    [migration guide](https://rohaquinlop.github.io/complexipy/migration/)
    for upgrade guidance.

### Added

- WebAssembly (WASM) support - the core analysis engine can now be compiled
  to WebAssembly, enabling browser-based analysis and tools like the
  VSCode extension. (#72)
- JSON output via `--output-json` (`-j`) for machine-readable results. (#74)
- `--ignore-complexity` (`-i`) flag to display all functions regardless of
  whether they exceed the complexity threshold. (#73)
- `--details` (`-d`) now also affects CSV and JSON outputs. (#73)
- Sort results by complexity score (`asc`, `desc`) or by `name`. (#73)
- Pre-commit hook support with documentation for easy setup. (#75)

### Removed

- The `--max-complexity` argument. (#73)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/3.0.0)
for the full details.

## [2.1.1] - 2025-04-24

### Fixed

- Fixed compatibility with Python 3.8. (#66)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/2.1.1)
for the full details.

## [2.1.0] - 2025-04-23

### Fixed

- Fixed dictionary expression bool op counting. (#64)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/2.1.0)
for the full details.

## [2.0.0] - 2025-04-18

### Changed

- Changed the parser from `rustpython` to `ruff_python_parser`. (#62)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/2.0.0)
for the full details.

## [1.2.0] - 2024-12-15

### Fixed

- Fixed the `output_summary` function call that was missing the
  `files_complexities` argument. (#58)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/1.2.0)
for the full details.

## [1.1.0] - 2024-12-14

### Added

- Multiple paths support - pass several paths to analyze at once. (#56)

### Removed

- The deprecated `-l` option, simplifying the command-line interface. (#56)
- File-level analysis, in order to focus on functions instead of the whole
  file. (#56)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/1.1.0)
for the full details.

## [0.5.0] - 2024-10-28

### Added

- Python API - call complexipy from your own Python code with
  `file_complexity()` and `code_complexity()`. (#45)
- Library usage documented in the README. (#49)
- Improved package usability. (#53)
- Updated documentation. (#54)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/0.5.0)
for the full details.

## [0.4.0] - 2024-06-21

### Added

- The cognitive complexity now considers the `If Expression` used in the
  code, including when used inside a `Call Expression` and so on. (#44)

### Fixed

- Fixed an edge case that could cause a memory overflow by filtering the
  cognitive complexity of the `orelse` values in an `If Statement` and
  keeping the nesting level subtraction propagation. (#44)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/0.4.0)
for the full details.

## [0.3.3] - 2024-04-27

### Changed

- Updated CI and removed unused dependencies. (#43)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/0.3.3)
for the full details.

## [0.3.2] - 2024-03-22

### Changed

- When using `--details low`, an empty summary table is no longer printed;
  an informational message is shown instead. (#38)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/0.3.2)
for the full details.

## [0.3.1] - 2024-03-13

### Added

- `-s` / `--sort` optional parameter to sort the output. (#30)
- Python >= 3.8 requirement. (#35)

### Fixed

- Fixed the logic used to calculate cognitive complexity: assignment
  statements only add complexity when `IfExp` is used, and `BinOp` by
  itself does not add complexity. (#34)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/0.3.1)
for the full details.

## [0.3.0] - 2024-03-07

### Added

- Function-level complexity analysis by default - the maximum complexity is
  evaluated for each function inside the Python files; per-file cognitive
  complexity is still available. (#21)
- New parameters. (#14)
- Progress bars. (#24)
- `--quiet` option. (#26)
- Unit testing. (#20)
- Cognitive complexity explanation in the docs. (#22)

### Changed

- Enhanced the algorithm to measure cognitive complexity - results are
  closer to the Sonar implementation. (#18)
- Reduced verbosity, with more information about the stages when running
  `complexipy` over git repositories (using the URL).
- CSV report generation is now implemented in Rust instead of Python,
  improving performance.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/0.3.0)
for the full details.

## [0.2.2] - 2024-02-27

Patch release; no changes were recorded in its release notes.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/0.2.2)
for the full details.

## [0.2.1] - 2024-02-27

### Changed

- Updated the README.

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/0.2.1)
for the full details.

## [0.2.0] - 2024-02-27

### Added

- Ignore paths support. (#6)
- Git repository URL support. (#7)
- CSV output format. (#8)
- Cognitive complexity algorithm fixes. (#10)
- Updated README. (#9)

See the [release notes](https://github.com/rohaquinlop/complexipy/releases/tag/0.2.0)
for the full details.
