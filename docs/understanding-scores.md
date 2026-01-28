# Understanding Complexity Scores

## What Do the Numbers Mean?

Cognitive complexity scores represent the mental effort required to understand a piece of code. Higher scores indicate code that is more difficult to comprehend and maintain.

### Recommended Thresholds

| Score Range | Interpretation | Recommendation |
|-------------|----------------|----------------|
| **0-5** | Simple | Easy to understand, no action needed |
| **6-10** | Moderate | Generally acceptable, but watch for further growth |
| **11-15** | Complex | Consider refactoring if functionality is being added |
| **16-25** | High | Refactoring recommended |
| **26+** | Very High | Refactoring strongly recommended |

!!! note "Default Threshold"
    complexipy uses a default threshold of **15**. Functions exceeding this score will trigger warnings.

## How Complexity is Calculated

Cognitive complexity is calculated by analyzing the Abstract Syntax Tree (AST) of your Python code and applying these rules:

### 1. Base Complexity Increments

Each control flow structure adds to the complexity:

| Structure | Score | Example |
|-----------|-------|---------|
| `if` statement | +1 | `if condition:` |
| `elif` clause | +1 | `elif other_condition:` |
| `else` clause | +0 | `else:` (nesting only) |
| `for` loop | +1 | `for item in items:` |
| `while` loop | +1 | `while condition:` |
| `except` handler | +1 | `except ValueError:` |
| `finally` clause | +0 | `finally:` (nesting only) |
| Ternary operator | +1 | `x if condition else y` |

### 2. Nesting Multiplier

**This is the key innovation**: nested structures add extra complexity based on their depth.

```python
def example():
    if a:           # +1 (base)
        if b:       # +2 (1 base + 1 nesting)
            if c:   # +3 (1 base + 2 nesting)
                pass
# Total complexity: 6
```

The formula is: `complexity = 1 + nesting_level`

### 3. Boolean Operators

Complex logical conditions increase cognitive load:

```python
# Score: 3
def check(a, b, c):
    if a and b or c:  # +1 (if) +2 (boolean operators)
        return True
```

- Each `and` or `or` operator: +1
- Operator type changes (mixing `and`/`or`): +1 additional

### 4. Special Cases

**Break and Continue**
```python
# Score: 3
for item in items:  # +1
    if condition:   # +2 (1 + nesting)
        break       # +0 (no additional score)
```

**Match Statements** (Python 3.10+)
```python
# Score: 2
match value:        # +0 (match itself doesn't count)
    case 1:
        if x:       # +1
            pass
    case 2:
        if y:       # +1
            pass
```

**Recursion**
Recursive calls are analyzed but don't add extra complexity beyond their control structures.

## Detailed Examples

### Example 1: Simple Sequential Logic

```python
def process_user(user):
    if not user:              # +1
        return None

    if user.is_active:        # +1
        send_welcome_email()

    if user.needs_verification: # +1
        send_verification()

    return user
# Total complexity: 3
```

**Analysis**: Three sequential if statements, no nesting. Easy to follow.

### Example 2: Nested Logic

```python
def validate_order(order):
    if order:                        # +1
        if order.items:              # +2 (1 + nesting)
            for item in order.items: # +3 (1 + nesting)
                if item.quantity > 0: # +4 (1 + nesting)
                    process(item)
    return False
# Total complexity: 10
```

**Analysis**: Deep nesting creates exponential growth in cognitive load.

### Example 3: Complex Conditions

```python
def check_eligibility(user, product):
    if user.age >= 18 and user.verified and product.available: # +4
        #  ^^^ if: +1, and operators: +2, total: +4
        return True
    return False
# Total complexity: 4
```

**Analysis**: Multiple boolean operators increase the mental load of understanding the condition.

### Example 4: Exception Handling

```python
def load_config(path):
    try:                              # try itself: +0
        with open(path) as f:         # +1 (context manager treated as if)
            data = json.load(f)
            if validate(data):        # +2 (1 + nesting)
                return data
    except FileNotFoundError:         # +1
        create_default_config()
    except json.JSONDecodeError:      # +1
        log_error()
    finally:                          # +0 (finally itself)
        if log_enabled:               # +1
            log("Config load attempted")
# Total complexity: 6
```

## Practical Guidelines

### What Should I Refactor?

Focus on functions with scores above your threshold (default: 15). Common refactoring strategies:

1. **Extract Methods** - Pull nested logic into separate functions
2. **Early Returns** - Use guard clauses to reduce nesting
3. **Simplify Conditions** - Break complex boolean expressions into named variables
4. **Strategy Pattern** - Replace nested if/else with polymorphism

### Example Refactoring

**Before (Complexity: 12)**
```python
def process_order(order):
    if order:                          # +1
        if order.is_valid():           # +2
            if order.payment:          # +3
                if order.payment.process(): # +4
                    ship_order(order)
                else:
                    refund(order)      # +0 (else clause)
            else:
                notify_payment_required() # Still nested
        else:
            log_invalid_order()
    return False
```

**After (Complexity: 4)**
```python
def process_order(order):
    if not order:                  # +1
        return False

    if not order.is_valid():       # +1
        log_invalid_order()
        return False

    if not order.payment:          # +1
        notify_payment_required()
        return False

    if order.payment.process():    # +1
        ship_order(order)
        return True

    refund(order)
    return False
```

**Key improvements:**
- Reduced nesting from 4 levels to 1
- Complexity dropped from 12 to 4
- Logic is now linear and easier to follow

## Score Interpretation Tips

1. **Context Matters** - A score of 20 might be acceptable for a complex algorithm but problematic for business logic
2. **Trend Over Time** - Watch for increasing complexity in functions you modify frequently
3. **Relative Scores** - Compare functions within the same codebase to identify outliers
4. **Team Agreement** - Establish thresholds that work for your team and project

## Further Reading

- [SonarSource Cognitive Complexity White Paper](https://www.sonarsource.com/resources/cognitive-complexity/)
- [Cyclomatic Complexity (McCabe, 1976)](https://en.wikipedia.org/wiki/Cyclomatic_complexity)
