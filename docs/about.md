# About complexipy

## Origins and Inspiration

complexipy is inspired by the [Cognitive Complexity](https://www.sonarsource.com/resources/cognitive-complexity/) research paper by **G. Ann Campbell** from SonarSource. This groundbreaking work introduced a new approach to measuring code complexity that better aligns with human understanding.

!!! info "Independent Project"
    While complexipy implements the cognitive complexity methodology described in Campbell's research, **it is not affiliated with or endorsed by SonarSource or Sonar products**. complexipy is an independent, open-source implementation written in Rust for Python code analysis.

## Why Cognitive Complexity?

Traditional metrics like cyclomatic complexity treat all control flow decisions equally, regardless of context. However, human comprehension doesn't work that way. Deeply nested code is significantly harder to understand than the same number of sequential decisions.

### The Problem with Cyclomatic Complexity

Cyclomatic complexity (McCabe, 1976) counts every decision point:

```python
# Cyclomatic complexity: 4
def example1(a, b, c):
    if a:
        return 1
    if b:
        return 2
    if c:
        return 3
    return 4
```

```python
# Cyclomatic complexity: 4 (same as above!)
def example2(a, b, c):
    if a:
        if b:
            if c:
                return 1
    return 4
```

Both functions have the same cyclomatic complexity, but any developer will tell you `example2` is much harder to understand.

### The Cognitive Complexity Solution

Cognitive complexity accounts for nesting:

```python
# Cognitive complexity: 3
def example1(a, b, c):
    if a:        # +1
        return 1
    if b:        # +1
        return 2
    if c:        # +1
        return 3
    return 4
```

```python
# Cognitive complexity: 6
def example2(a, b, c):
    if a:        # +1
        if b:    # +2 (1 + nesting_level)
            if c:  # +3 (1 + nesting_level)
                return 1
    return 4
```

This aligns with human intuition: nested code requires holding more context in your mind.

## Project Goals

1. **Performance** - Blazingly fast analysis using Rust
2. **Accuracy** - Faithful implementation of cognitive complexity principles
3. **Accessibility** - Easy integration with Python development workflows
4. **Actionability** - Clear, actionable insights for improving code quality

## The Team

complexipy is built with ❤️ by [@rohaquinlop](https://github.com/rohaquinlop) and [contributors](https://github.com/rohaquinlop/complexipy/graphs/contributors).

## License

complexipy is released under the [MIT License](https://github.com/rohaquinlop/complexipy/blob/main/LICENSE).
