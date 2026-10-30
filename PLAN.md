# Plan: let `--fix` apply C005 (extract predicate)

Issue: #263

## Current state

- `generate_predicate_suggestion` (`crates/complexipy-core/src/rules/complexity.rs`) already builds a real helper: `def _check_condition_L<n>(<free names>) -> bool: return <condition>`. The free names of the condition become parameters, and the call site passes the same names. The body is not a placeholder any more.
- The replacement is one snippet: the helper `def`, a blank line, then the rewritten `if`/`while` header inside its enclosing headers. It cannot replace the plan's span as is, so `spliceable` is `false`.
- `--fix` (`fix::apply_fixes`) and reduction measurement (`registry::measure_reduction`) both do one span replacement per plan through `splice_window`. Neither can place a second edit.
- `docs/refactoring-rules.md` still says the C005 body is a placeholder. That line is stale.

## Design

### Data model

Add one field to `CodeSuggestion`:

```rust
pub struct CodeInsertion {
    pub before_line: u64,
    pub text: String,
}

pub struct CodeSuggestion {
    pub replacement: String,
    pub applicability: Applicability,
    pub description: String,
    pub spliceable: bool,
    pub insertions: Vec<CodeInsertion>,
}
```

- `replacement` keeps its meaning: the text for the plan's span.
- `insertions` holds extra text inserted before a line of the original file, outside the plan's span. Every rule except C005 leaves it empty.
- `#[serde(default)]` on `insertions` keeps old JSON readable.
- `CodeInsertion` crosses into Python, so it changes in the four lockstep places: the Rust type, the `#[pymodule]` export list, `complexipy/_complexipy.pyi`, and `tests/test_stubs.py`.

### C005 suggestion

- `replacement` becomes the rewritten header line only: `<indent>if _check_condition_L<n>(<params>):`, plus whatever followed the colon on that line (a trailing comment or an inline body). The span is that one header line. Conditions are read from one line today (`split_header`), so a multi-line header still returns `None`.
- `insertions` holds one entry: the helper `def` plus two blank lines, inserted before the first line of the top-level statement that contains the condition (the module-level `def`, `class`, or decorator line). The helper lands at module level, as the current snippet already documents.
- The generated `def` drops the `-> bool` annotation. `and` and `or` return an operand, not a bool, so the annotation is wrong. The call site's truthiness does not change.
- `spliceable` becomes `true` only when every free name of the condition is bound for sure before the statement: a parameter of the enclosing function, or a name the function never assigns (a global, builtin, or closure name). The call passes every free name eagerly, so a local that short-circuit would skip (`if flag or value > 0:`, where `value` is bound on some paths only) would raise `UnboundLocalError`. Any other condition keeps today's snippet with `spliceable: false` for hand application.
- Applicability stays `MachineApplicable`. The fix gate in `fix::fixable` does not change.
- Cases that return `None` today (`elif`, `:=`, lambda, comprehension) still return `None` and keep `help` text.
- The CLI display (`output/refactor.rs`) renders the insertion above the replacement, so the printed snippet stays readable for hand application.

### Shared application path

- New `fix::apply_edits(source, index, plan, suggestion, newline) -> Option<String>` applies the insertions and the span replacement bottom-to-top in one pass, so every offset comes from the original index.
- `splice_plan` calls it. `measure_in_file` uses it, so C005 reduction becomes measured.
- `measure_in_function` keeps its one-window fast path only when `insertions` is empty. With insertions it falls back to `measure_in_file`, because the helper lands outside the function window.
- `apply_fixes` checks overlap on the span and on each insertion line. Two C005 plans in one file both insert above the same top-level statement; the second plan's helper lands above the first one's, which is valid Python, so the insertion line does not count as overlap. The parse check after each splice still guards the result.
- `LineMap::register_fix` gets one call per edit (insertion as a zero-length span grown to N lines), so later passes still map lines back to the original file.

### Helper placement

- Module level only, above the outermost statement that contains the condition. Free names, including `self` and closure variables, are already parameters, so the helper never needs the enclosing scope.
- Nested placement is out of scope.

## Tests

- `crates/complexipy-core/src/fix/tests.rs`: C005 on a plain function, on a method, under a decorator, and two C005 plans in one function. Each patched text parses, holds the helper at module level, and re-analysis yields no C005 plan for the fixed function.
- `crates/complexipy-core/src/rules/complexity/tests.rs`: a condition over a local assigned in the function stays `spliceable: false`; a condition over parameters and globals is `spliceable: true`; the helper has no `-> bool`.
- `crates/complexipy-core/src/rules/registry/tests.rs`: C005 reduction is measured (`reduction_is_measured == true`).
- `tests/test_fix.py`: replace the "Skipped C005" assertion with an end-to-end `--fix` run that applies C005.
- `tests/test_stubs.py`: covers `CodeInsertion` and the new field.
- CRLF file: the inserted helper uses the file's line break.

## Docs and changelog

- `docs/refactoring-rules.md` and `docs/es/refactoring-rules.md`: replace the stale placeholder line, describe the two edits C005 applies and where the helper lands, and add `insertions` to the JSON example.
- `CHANGELOG.md` and `docs/es/changelog.md` under Unreleased: `--fix` applies C005; C005 reduction is measured; `CodeSuggestion` gains `insertions`.

## Out of scope

- Nested helper placement.
- Extending C005 to `elif`, walrus, lambda, or comprehension conditions.
