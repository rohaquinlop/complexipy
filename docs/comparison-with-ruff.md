# Comparison with Ruff

## Overview

Both [Ruff](https://docs.astral.sh/ruff/) and complexipy help improve Python code quality, but they measure different aspects of complexity and serve complementary purposes.

## Ruff's PLR0912: Too Many Branches

Ruff's [PLR0912 (too-many-branches)](https://docs.astral.sh/ruff/rules/too-many-branches/) rule is based on **cyclomatic complexity** and counts the number of decision points in a function.

### How Ruff Counts Branches

Ruff counts each of these as a branch:
- `if`, `elif`, `else` statements
- `for` and `while` loops
- `and`, `or` boolean operators
- `except` clauses
- Pattern matching cases

The default threshold is typically **12 branches** per function.

### Example: Ruff Analysis

```python
def example(a, b, c, d):
    if a:           # Branch 1
        return 1
    elif b:         # Branch 2
        return 2
    elif c:         # Branch 3
        return 3
    elif d:         # Branch 4
        return 4
    else:           # Branch 5
        return 0
# Ruff: 5 branches
```

## complexipy: Cognitive Complexity

complexipy implements **cognitive complexity**, which weights branches by their nesting level to reflect human understanding.

### How complexipy Scores

complexipy uses the formula: `complexity = base_score + nesting_level`

```python
def example(a, b, c, d):
    if a:           # +1 (1 + 0 nesting)
        return 1
    elif b:         # +1 (elif counts as +1)
        return 2
    elif c:         # +1
        return 3
    elif d:         # +1
        return 4
    else:           # +0 (else doesn't add base score)
        return 0
# complexipy: 4 points
```

## Key Differences

### 1. Nesting is Critical in complexipy

**Ruff treats all branches equally:**
```python
# Ruff: 3 branches
def flat_logic(a, b, c):
    if a:           # Branch 1
        return 1
    if b:           # Branch 2
        return 2
    if c:           # Branch 3
        return 3
```

```python
# Ruff: 3 branches (same as above)
def nested_logic(a, b, c):
    if a:           # Branch 1
        if b:       # Branch 2
            if c:   # Branch 3
                return 1
```

**complexipy accounts for nesting:**
```python
# complexipy: 3 points
def flat_logic(a, b, c):
    if a:           # +1
        return 1
    if b:           # +1
        return 2
    if c:           # +1
        return 3
```

```python
# complexipy: 6 points (much worse!)
def nested_logic(a, b, c):
    if a:           # +1 (1 + 0)
        if b:       # +2 (1 + 1)
            if c:   # +3 (1 + 2)
                return 1
```

### 2. else Clauses

**Ruff**: Counts `else` as a branch (+1)

**complexipy**: `else` doesn't add base complexity, only nesting penalty

```python
# Ruff: 2 branches
# complexipy: 1 point
def check(value):
    if value > 0:   # Ruff: +1, complexipy: +1
        return "positive"
    else:           # Ruff: +1, complexipy: +0
        return "non-positive"
```

### 3. Boolean Operators

Both count boolean operators, but with different philosophies:

**Ruff**: Counts each `and`/`or` as a separate branch

**complexipy**: Counts operators and penalizes mixed operator types

```python
# Ruff: 3 branches (if + and + or)
# complexipy: 3 points (1 for if, +2 for boolean ops)
def check(a, b, c):
    if a and b or c:
        return True
```

### 4. Match Statements (Python 3.10+)

**Ruff**: Counts each `case` as a branch

**complexipy**: Match itself adds no complexity, only nested content counts

```python
# Ruff: 3 branches (match + 2 cases)
# complexipy: 2 points (only the nested ifs)
match value:        # Ruff: +1, complexipy: +0
    case 1:         # Ruff: +1, complexipy: +0
        if x:       # Ruff: +1, complexipy: +1
            pass
    case 2:         # Already counted by Ruff
        if y:       # complexipy: +1
            pass
```

## Practical Comparison Example

Here's a real-world scenario showing how the two tools differ:

```python
def process_payment(order):
    if order is None:                              # Ruff: 1, complexipy: 1
        return False

    if not order.is_valid():                       # Ruff: 2, complexipy: 2
        return False

    if order.payment_method == "credit_card":      # Ruff: 3, complexipy: 3
        if order.amount > 1000:                    # Ruff: 4, complexipy: 5 (1+1 nesting)
            if not verify_fraud_check(order):      # Ruff: 5, complexipy: 8 (1+2 nesting)
                return False
        return process_credit_card(order)
    elif order.payment_method == "paypal":         # Ruff: 6, complexipy: 4
        return process_paypal(order)
    else:                                           # Ruff: 7, complexipy: 4
        return False

# Final Scores:
# Ruff: 7 branches
# complexipy: 8 points
```

In this example:
- Ruff flags the function for having 7 branches (over typical threshold)
- complexipy gives it 8 points, with most complexity coming from the nested fraud check
- complexipy better identifies that the deeply nested fraud check is the problematic part

## Which Should You Use?

### Use Ruff (PLR0912) When:
- You want to limit the absolute number of decision points
- You're enforcing a strict "small function" policy
- You want simple, objective counts
- You're already using Ruff for other linting

### Use complexipy When:
- You want to identify truly hard-to-understand code
- Nesting is a concern in your codebase
- You want to align with human comprehension
- You're conducting code reviews focused on maintainability

### Use Both! (Recommended)

Ruff and complexipy are complementary:

```bash
# In your pre-commit or CI pipeline
ruff check . --select=PLR0912  # Catch functions with too many branches
complexipy . --max-complexity-allowed 15  # Catch deeply nested code
```

**Ruff** catches wide functions (many branches), while **complexipy** catches deep functions (heavy nesting). Together, they provide comprehensive complexity coverage.

## Configuration Examples

### Ruff Configuration

```toml
# pyproject.toml
[tool.ruff]
select = ["PLR0912"]

[tool.ruff.pylint]
max-branches = 12
```

### complexipy Configuration

```toml
# pyproject.toml
[tool.complexipy]
max-complexity-allowed = 15
```

### Using Both in CI

```yaml
# .github/workflows/quality.yml
- name: Ruff Linting
  run: ruff check . --select=PLR0912

- name: Cognitive Complexity Check
  run: complexipy . --max-complexity-allowed 15
```

## Migration Strategy

If you're already using Ruff and want to add complexipy:

1. **Baseline First**: Run `complexipy . --snapshot-create` to capture current state
2. **Set Thresholds**: Start with a higher threshold (e.g., 20) and lower it over time
3. **Fix New Code**: Only fail CI on new violations
4. **Gradual Improvement**: Refactor legacy code opportunistically

## Summary

| Feature | Ruff PLR0912 | complexipy |
|---------|--------------|------------|
| **Based on** | Cyclomatic Complexity | Cognitive Complexity |
| **Counts nesting** | ❌ No | ✅ Yes |
| **else penalty** | ✅ Yes | ❌ No (only nesting) |
| **Boolean operators** | ✅ Yes | ✅ Yes |
| **match statements** | ✅ Yes | Partial (content only) |
| **Best for** | Total decision count | Human comprehension |
| **Threshold** | ~12 branches | ~15 points |
| **Performance** | Fast (Rust) | Very fast (Rust) |

**The Bottom Line**: Ruff's PLR0912 enforces a limit on decision points, while complexipy measures how hard code is to understand. Both are valuable, and using them together provides the best coverage for code quality.

## Further Reading

- [Ruff Documentation](https://docs.astral.sh/ruff/)
- [Ruff PLR0912 Rule](https://docs.astral.sh/ruff/rules/too-many-branches/)
- [complexipy Understanding Scores](understanding-scores.md)
