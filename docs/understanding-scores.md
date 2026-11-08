# Understanding Complexity Scores

## What Do the Numbers Mean?

Cognitive complexity scores represent the mental effort required to understand a piece of code. Higher scores indicate code that is more difficult to comprehend and maintain.

### Recommended Thresholds

| Score Range | Interpretation | Recommendation |
| -- | -- | -- |
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
| -- | -- | -- |
| `if` statement | +1 | `if condition:` |
| `elif` clause | +1 | `elif other_condition:` |
| `else` clause | +1 | `else:` after `if`, `for`, `while`, or `try` (no nesting increment; its body is nested) |
| `for` loop | +1 | `for item in items:` |
| `while` loop | +1 | `while condition:` |
| `except` handler | +1 | `except ValueError:` |
| `finally` clause | +0 | `finally:` (its body is not nested) |
| Ternary operator | +1 | `x if condition else y` |
| Comprehension clause | +0 | `[x for x in xs if x]` |

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

- Each run of like operators, read left to right: +1
- Each change between `and` and `or` starts a new run: `a or b and c or d` reads `or`, `and`, `or` and scores +3

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
# Score: 5
match value:        # +1
    case 1:
        if x:       # +2 (1 + nesting)
            pass
    case 2:
        if y:       # +2 (1 + nesting)
            pass
```

**Recursion**
Each function in a recursion cycle adds +1 once, however many recursive calls it makes. The cycle can be direct (`fact` calls `fact`) or indirect (`a` calls `b`, `b` calls `a`). The +1 of a nested function, or of a method of a class nested in a function, goes to the function that holds it. A call counts when it resolves in the same file: a bare name to the nearest enclosing function or module scope that defines it, or `self.m()` and `cls.m()` to a method of the caller's class (a closure in a method uses that method's `self`). A call inside a lambda, or in the decorators, default arguments, bases, or class body of a nested definition, counts for the function that holds it, and a lambda's own parameters hide outer names. A bare decorator such as `@f` is not a call, so it does not count. Calls through `super()`, other objects, the class name, or imports do not count, cycles across files are not detected, and a parameter or variable that shadows a function name is not tracked.

**Python constructs the paper does not name**

- Comprehensions: the `for` and `if` clauses add nothing and do not raise the nesting level. The paper ignores shorthand that lets several statements be written readably as one, and a comprehension is shorthand for a loop that builds a value. A ternary or a boolean operator inside a comprehension still counts.
- `else` on `for`, `while`, and `try`: +1, with no nesting increment, and its body raises the nesting level, like `else` after `if`. The `else` body runs only when the loop ends without `break` or the `try` body raised nothing, so it is a branch the reader must track. The paper calls `else` a hybrid increment: no nesting increment, but a higher nesting level for what it holds.
- `case` guards: +0 for the guard itself. A boolean operator sequence or a ternary inside it counts, at the nesting level of the `match`. The paper gives a `switch` and all its cases a single increment and does not name guards, and SonarJava scores the `when` guards of Java `switch` cases the same way.
- Mixed boolean operators: the operators of one condition are read left to right, as they appear in the code, and each change between `and` and `or` starts a new sequence. A `not` starts a new condition for its operand.

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
    if user.age >= 18 and user.verified and product.available: # +2
        #  ^^^ if: +1, and run: +1, total: +2
        return True
    return False
# Total complexity: 2
```

**Analysis**: Multiple boolean operators increase the mental load of understanding the condition.

### Example 4: Exception Handling

```python
def load_config(path):
    try:                              # try itself: +0
        with open(path) as f:         # +0 (with adds nothing and does not nest)
            data = json.load(f)
            if validate(data):        # +1 (try and with bodies are not nested)
                return data
    except FileNotFoundError:         # +1
        create_default_config()
    except json.JSONDecodeError:      # +1
        log_error()
    finally:                          # +0 (finally itself)
        if log_enabled:               # +1
            log("Config load attempted")
# Total complexity: 4
```

## Practical Guidelines

### What Should I Refactor?

Focus on functions with scores above your threshold (default: 15). Common refactoring strategies:

1. **Extract Methods** - Pull nested logic into separate functions
1. **Early Returns** - Use guard clauses to reduce nesting
1. **Simplify Conditions** - Break complex boolean expressions into named variables
1. **Strategy Pattern** - Replace nested if/else with polymorphism

### Example Refactoring

**Before (Complexity: 13)**

```python
def process_order(order):
    if order:                          # +1
        if order.is_valid():           # +2
            if order.payment:          # +3
                if order.payment.process(): # +4
                    ship_order(order)
                else:                          # +1 (else clause)
                    refund(order)
            else:                      # +1 (else clause)
                notify_payment_required()
        else:                          # +1 (else clause)
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
- Complexity dropped from 13 to 4
- Logic is now linear and easier to follow

## Score Interpretation Tips

1. **Context Matters** - A score of 20 might be acceptable for a complex algorithm but problematic for business logic
1. **Trend Over Time** - Watch for increasing complexity in functions you modify frequently
1. **Relative Scores** - Compare functions within the same codebase to identify outliers
1. **Team Agreement** - Establish thresholds that work for your team and project

## Further Reading

- [SonarSource Cognitive Complexity White Paper](https://www.sonarsource.com/resources/cognitive-complexity/)
- [Cyclomatic Complexity (McCabe, 1976)](https://en.wikipedia.org/wiki/Cyclomatic_complexity)
