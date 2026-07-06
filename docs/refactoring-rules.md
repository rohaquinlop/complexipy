# Refactoring Rules

complexipy includes a clippy-inspired refactoring system that provides actionable suggestions for reducing cognitive complexity. Each rule has a unique ID, category, and applicability level.

## Rule Categories

| Category            | Icon | Description                                     |
| ------------------- | ---- | ----------------------------------------------- |
| **Complexity**      | •    | Rules that directly reduce cognitive complexity |
| **Readability**     | •    | Rules that improve code readability             |
| **Maintainability** | •    | Rules that improve long-term maintainability    |

## Applicability Levels

| Level               | Icon | Description                                        |
| ------------------- | ---- | -------------------------------------------------- |
| **Auto-applicable** | \*   | Safe to apply automatically without human review   |
| **Needs review**    | !    | May be incorrect in some cases, needs human review |
| **Informational**   | i    | Just guidance, not directly actionable             |

______________________________________________________________________

## Complexity Rules

### C001: Flatten Nested Conditions

**Category:** • Complexity\
**Applicability:** ! Needs review\
**Priority:** High (4/5)

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

**Category:** • Complexity\
**Applicability:** ! Needs review\
**Priority:** Medium (3/5)

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
        if not item.active:
            continue
        if not item.ready:
            continue
        total += item.value
    return total
```

#### Why this helps

Nested conditions inside loops add unnecessary indentation. Using continue guards keeps the main logic at a lower nesting level and makes the loop easier to follow.

______________________________________________________________________

### C003: Extract Helper Function

**Category:** • Complexity\
**Applicability:** ! Needs review\
**Priority:** Medium (3/5)

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

**Category:** • Complexity\
**Applicability:** ! Needs review\
**Priority:** Low (2/5)

Split long elif chains or match statements into separate handlers.

#### When does it trigger?

This rule triggers when:

- An `if/elif` chain has 3+ branches
- A `match` statement has 4+ cases

#### Example

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
    handlers = {
        "create": create_resource,
        "read": read_resource,
        "update": update_resource,
        "delete": delete_resource,
    }
    handler = handlers.get(action)
    return handler() if handler else None
```

#### Why this helps

Long conditional chains are hard to maintain and extend. Splitting them into separate handlers makes each case independently testable and the dispatch logic clearer.

______________________________________________________________________

### C006: Reduce Nesting Depth

**Category:** • Complexity\
**Applicability:** * Auto-applicable\
**Priority:** High (4/5)

Reduce nesting depth by using early returns and guard clauses.

#### When does it trigger?

This rule triggers when code has deep nesting (3+ levels) that makes it hard to follow.

#### Example

**Before:**

```python
def validate(user):
    if user.is_active:
        if user.has_permission:
            if user.is_verified:
                return check_access(user)
    return False
```

**After:**

```python
def validate(user):
    if not user.is_active:
        return False
    if not user.has_permission:
        return False
    if not user.is_verified:
        return False
    return check_access(user)
```

#### Why this helps

Deep nesting (3+ levels) makes code hard to follow. Consider extracting inner blocks or using guard clauses to keep indentation shallow.

______________________________________________________________________

### C011: Flatten Try/Except

**Category:** • Complexity\
**Applicability:** ! Needs review\
**Priority:** Low (2/5)

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

**Category:** • Readability\
**Applicability:** ! Needs review\
**Priority:** Medium (3/5)

Extract complex boolean conditions into named predicate functions.

#### When does it trigger?

This rule triggers when a boolean condition contains 2+ logical operators (and, or, not).

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
        if plan.before_code:
            print(f"  Before:\n{plan.before_code.text}")
        if plan.after_code:
            print(f"  After:\n{plan.after_code.text}")
```

### JSON Output

The JSON output includes all rule metadata for programmatic consumption:

```json
{
  "rule_id": "C001",
  "kind": "flatten_condition",
  "title": "Flatten nested condition block with guard clauses",
  "category": "Complexity",
  "applicability": "MachineApplicable",
  "description": "Flatten nested condition blocks by using guard clauses with early returns",
  "line_start": 10,
  "line_end": 15,
  "current_complexity": 12,
  "estimated_reduction": 4,
  "estimated_complexity_after": 8,
  "before_code": {
    "text": "if data:\n    if data.is_valid():\n        return process(data)",
    "line_start": 10,
    "line_end": 12
  },
  "after_code": {
    "text": "if not data:\n    return None\nif not data.is_valid():\n    return None\nreturn process(data)",
    "line_start": 10,
    "line_end": 14
  },
  "explanation": "Deeply nested conditions are hard to follow...",
  "references": [
    "https://rohaquinlop.github.io/complexipy/refactoring-rules/#c001-flatten-nested-conditions"
  ]
}
```

______________________________________________________________________

## Rule ID Reference

| ID                                      | Name                      | Category      | Applicability      | Priority |
| --------------------------------------- | ------------------------- | ------------- | ------------------ | -------- |
| [C001](#c001-flatten-nested-conditions) | Flatten Nested Conditions | • Complexity  | ! Needs review     | High     |
| [C002](#c002-loop-guards)               | Loop Guards               | • Complexity  | ! Needs review     | Medium   |
| [C003](#c003-extract-helper-function)   | Extract Helper Function   | • Complexity  | ! Needs review     | Medium   |
| [C004](#c004-split-dispatcher)          | Split Dispatcher          | • Complexity  | ! Needs review     | Low      |
| [C005](#c005-extract-predicate)         | Extract Predicate         | • Readability | ! Needs review     | Medium   |
| [C006](#c006-reduce-nesting-depth)      | Reduce Nesting Depth      | • Complexity  | \* Auto-applicable | High     |
| [C011](#c011-flatten-tryexcept)         | Flatten Try/Except        | • Complexity  | ! Needs review     | Low      |
