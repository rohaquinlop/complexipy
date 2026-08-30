# Refactoring Rules

complexipy includes a clippy-inspired refactoring system that provides actionable suggestions for reducing cognitive complexity. Each rule has a unique ID, category, and applicability level.

## Rule Categories

| Category | Icon | Description |
| -- | -- | -- |
| **Complexity** | ▲ | Rules that directly reduce cognitive complexity |
| **Readability** | ◆ | Rules that improve code readability |

## Applicability Levels

| Level | Icon | Description |
| -- | -- | -- |
| **Safe to apply** | \* | High confidence the generated code is correct as written -- no automatic application yet, this is a confidence signal, not a promise of automation |
| **Needs review** | ! | May be incorrect in some cases, needs human review |
| **Informational** | i | Just guidance, not directly actionable |

______________________________________________________________________

## Complexity Rules

### C001: Flatten Nested Conditions

- **Category:** ▲ Complexity
- **Applicability:** i Informational
- **Priority:** High (4/5)

Flatten nested condition blocks by using guard clauses with early returns.

#### When does it trigger?

This rule triggers when a function has deeply nested `if` statements (2+ levels of nesting) that add significant complexity.

#### Example

**Before:**

```python
def process_data(data):
    if data:
        if data.is_valid():
            if data.is_ready():
                return process(data)
    return None
```

**After:**

```python
def process_data(data):
    if not data:
        return None
    if not data.is_valid():
        return None
    if not data.is_ready():
        return None
    return process(data)
```

#### Why this helps

Deeply nested conditions are hard to follow. Using guard clauses with early returns reduces cognitive load by keeping the main path at a lower indentation level.

______________________________________________________________________

### C002: Loop Guards

- **Category:** ▲ Complexity
- **Applicability:** * Safe to apply
- **Priority:** Medium (3/5)

Use continue guards at the top of loops to reduce nesting.

#### When does it trigger?

This rule triggers when a loop contains nested `if` statements that could be converted to early `continue` guards.

#### Example

**Before:**

```python
def process_items(items):
    total = 0
    for item in items:
        if item.active:
            if item.ready:
                total += item.value
    return total
```

**After:**

```python
def process_items(items):
    total = 0
    for item in items:
        if not (item.active):
            continue
        if not (item.ready):
            continue
        total += item.value
    return total
```

#### Why this helps

Nested conditions inside loops add unnecessary indentation. Using continue guards keeps the main logic at a lower nesting level and makes the loop easier to follow.

______________________________________________________________________

### C003: Extract Helper Function

- **Category:** ▲ Complexity
- **Applicability:** i Informational
- **Priority:** Low (2/5)

Extract complex code blocks into separate helper functions.

#### When does it trigger?

This rule triggers when a code block has high complexity (6+) and spans multiple lines (5+).

#### Example

**Before:**

```python
def process_order(order):
    # Complex validation and processing logic
    if order.items:
        for item in order.items:
            if item.quantity > 0:
                if item.price > 0:
                    total = item.quantity * item.price
                    if total > 100:
                        apply_discount(total)
                    process_item(item)
```

**After:**

```python
def process_order(order):
    if not order.items:
        return
    for item in order.items:
        process_order_item(item)

def process_order_item(item):
    if item.quantity <= 0 or item.price <= 0:
        return
    total = item.quantity * item.price
    if total > 100:
        apply_discount(total)
    process_item(item)
```

#### Why this helps

Complex code blocks should be extracted into named functions to improve readability and testability. The extracted function can be given a descriptive name that explains its purpose.

______________________________________________________________________

### C004: Split Dispatcher

- **Category:** ▲ Complexity
- **Applicability:** i Informational
- **Priority:** Low (2/5)

Split long elif chains into separate handlers.

#### When does it trigger?

This rule triggers when an `if/elif` chain has 3+ branches. `match` statements are
intentionally excluded: complexipy's cognitive complexity model charges a `match` a flat
cost regardless of how many `case` clauses it has (unlike an `elif` chain, where every
additional branch adds to the score), so splitting a `match` into separate handlers would
not actually reduce the measured complexity.

The suggested refactor depends on the chain's shape:

- If every branch compares the **same variable** against a **literal** with `==`, the
  chain can become a `match` statement -- this is recommended over a dispatch dict,
  since it needs no extra indirection and complexipy's own model scores it as free of
  the per-branch cost the `elif` chain has.
- Otherwise (ranges, multiple variables, non-equality comparisons -- anything a plain
  `match case <value>:` can't express) a dispatch dictionary mapping cases to handler
  functions is suggested instead.

#### Example: single-variable equality → `match`

**Before:**

```python
def handle_action(action):
    if action == "create":
        return create_resource()
    elif action == "read":
        return read_resource()
    elif action == "update":
        return update_resource()
    elif action == "delete":
        return delete_resource()
    return None
```

**After:**

```python
def handle_action(action):
    match action:
        case "create":
            return create_resource()
        case "read":
            return read_resource()
        case "update":
            return update_resource()
        case "delete":
            return delete_resource()
    return None
```

#### Example: ranges → dispatch dictionary

**Before:**

```python
def classify(score):
    if score < 60:
        return "fail"
    elif score < 70:
        return "d"
    elif score < 85:
        return "b"
    elif score < 95:
        return "a"
    return "a+"
```

**After:**

```python
def classify(score):
    thresholds = [(60, "fail"), (70, "d"), (85, "b"), (95, "a")]
    for limit, grade in thresholds:
        if score < limit:
            return grade
    return "a+"
```

#### Why this helps

Long conditional chains are hard to maintain and extend. Splitting them into separate handlers -- a `match` statement when the chain is a simple equality dispatch, a dispatch dictionary otherwise -- makes each case independently testable and the dispatch logic clearer.

______________________________________________________________________

### C011: Flatten Try/Except

- **Category:** ▲ Complexity
- **Applicability:** i Informational
- **Priority:** Low (2/5)

Flatten nested try/except blocks by combining or restructuring.

#### When does it trigger?

This rule triggers when try/except blocks are nested inside each other.

#### Example

**Before:**

```python
def read_config(path):
    try:
        with open(path) as f:
            try:
                return json.load(f)
            except json.JSONDecodeError:
                return default_config()
    except FileNotFoundError:
        return default_config()
```

**After:**

```python
def read_config(path):
    try:
        with open(path) as f:
            return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        return default_config()
```

#### Why this helps

Nested try/except blocks are confusing and hard to maintain. Consider merging them or extracting the inner block into a separate function with its own error handling.

______________________________________________________________________

## Readability Rules

### C005: Extract Predicate

- **Category:** ◆ Readability
- **Applicability:** * Safe to apply
- **Priority:** Low (2/5)

Extract complex boolean conditions into named predicate functions.

#### When does it trigger?

This rule triggers when a boolean condition contains 2+ logical operators (and, or, not).

The suggestion keeps the statement keyword: an `if` condition stays an `if`, a `while` condition stays a `while`. Conditions on `elif` lines only get help text, because an extracted function cannot be placed inside an if-chain.

The extracted helper is emitted at module level with the condition's free variables as parameters (attribute bases included, builtins excluded), so it is unit-testable. The snippet shows the enclosing context at the statement's real indentation, with `...` placeholders for skipped statements. Conditions that bind a name with `:=` or contain a lambda/comprehension get help text only.

#### Example

**Before:**

```python
def is_eligible(user, order):
    if (user.is_active and user.has_subscription) or (order.total > 100 and not order.is_gift):
        return True
    return False
```

**After:**

```python
def is_eligible(user, order):
    return has_active_subscription(user) or is_qualifying_order(order)

def has_active_subscription(user):
    return user.is_active and user.has_subscription

def is_qualifying_order(order):
    return order.total > 100 and not order.is_gift
```

#### Why this helps

Complex boolean expressions are hard to understand at a glance. Extracting them into named predicates makes the code self-documenting and easier to test.

______________________________________________________________________

### C007: Collapsible If

- **Category:** ◆ Readability
- **Applicability:** * Safe to apply
- **Priority:** Highest (5/5)

Merge nested if statements into a single if with combined conditions.

#### When does it trigger?

This rule triggers when an `if` statement's entire body is a single nested `if` with no `else` branch, for a chain of two or more such levels.

The merge is skipped when any statement or comment sits between the outer and inner `if`, so the suggestion never drops code.

#### Example

**Before:**

```python
def check_eligibility(user):
    if user.is_active:
        if user.has_permission:
            return True
    return False
```

**After:**

```python
def check_eligibility(user):
    if user.is_active and user.has_permission:
        return True
    return False
```

#### Why this helps

Nested if statements with a single body can be merged into a single if with combined conditions using `and`. This reduces nesting and improves readability.

______________________________________________________________________

## Using Refactoring Rules

### Command Line

```bash
# Show refactoring suggestions
complexipy . --suggest-refactors

# Show suggestions for failing functions only
complexipy . --failed --suggest-refactors

# Export suggestions to JSON
complexipy . --output-format json --suggest-refactors
```

### Python API

```python
from complexipy import code_complexity

code = """
def process(data):
    if data:
        if data.is_valid():
            return process(data)
    return None
"""

result = code_complexity(code)
for func in result.functions:
    for plan in func.refactor_plans:
        print(f"[{plan.rule_id}] {plan.title}")
        print(f"  Category: {plan.category}")
        print(f"  Applicability: {plan.applicability}")
        print(f"  Reduction: -{plan.estimated_reduction} complexity")
        if plan.suggestion:
            print(f"  Suggested replacement:\n{plan.suggestion.replacement}")
        elif plan.help:
            print(f"  Help: {plan.help}")
        print(f"  Docs: {plan.doc_url}")
```

### JSON Output

The JSON output includes all rule metadata for programmatic consumption:

```json
{
  "rule_id": "C007",
  "kind": "collapsible_if",
  "title": "Merge nested if statements",
  "category": "Readability",
  "applicability": "MachineApplicable",
  "description": "Merge nested if statements into a single if with combined conditions",
  "line_start": 3,
  "line_end": 5,
  "column_start": 5,
  "current_complexity": 4,
  "estimated_reduction": 1,
  "estimated_complexity_after": 3,
  "reduction_is_measured": true,
  "suggestion": {
    "replacement": "    if data and data.is_valid():\n        return process(data)",
    "applicability": "MachineApplicable",
    "spliceable": true,
    "description": "Merge nested conditions into `if data and data.is_valid():`"
  },
  "help": null,
  "explanation": "Nested if statements with a single body can be merged into a single if with combined conditions using 'and'. This reduces nesting and improves readability.",
  "references": [],
  "doc_url": "https://rohaquinlop.github.io/complexipy/refactoring-rules/#c007-collapsible-if"
}
```

### Measured vs estimated reductions

Every plan reports how much applying it lowers the complexity
(`estimated_reduction` / `estimated_complexity_after`) together with a
confidence flag, `reduction_is_measured`:

- **Measured** (`true`): the rule spliced its suggestion into the real
  source, re-parsed it, and re-ran the scorer. The number is the literal
  answer to "apply this and re-score" - exactly what C002 and C007
  report, since they carry machine-applicable replacements. The CLI shows
  these without a qualifier: `Reduction: -2 complexity (7 -> 5)`.
- **Estimated** (`false`): the number comes from the rule's hand-derived
  formula (help-only rules C001, C003, C004, C011, and the C005 snippet,
  plus any fallback when a splice cannot be re-parsed). The CLI renders
  these with a tilde: `Estimated reduction: ~-2 complexity (7 -> 5)`.

A measured reduction of 0 means the suggestion does not actually lower
complexity; the plan is dropped rather than shown.

______________________________________________________________________

## Rule ID Reference

| ID | Name | Category | Applicability | Priority |
| -- | -- | -- | -- | -- |
| [C001](#c001-flatten-nested-conditions) | Flatten Nested Conditions | ▲ Complexity | i Informational | High |
| [C002](#c002-loop-guards) | Loop Guards | ▲ Complexity | \* Safe to apply | Medium |
| [C003](#c003-extract-helper-function) | Extract Helper Function | ▲ Complexity | i Informational | Low |
| [C004](#c004-split-dispatcher) | Split Dispatcher | ▲ Complexity | i Informational | Low |
| [C005](#c005-extract-predicate) | Extract Predicate | ◆ Readability | \* Safe to apply | Low |
| [C007](#c007-collapsible-if) | Collapsible If | ◆ Readability | \* Safe to apply | Highest |
| [C011](#c011-flatten-tryexcept) | Flatten Try/Except | ▲ Complexity | i Informational | Low |
