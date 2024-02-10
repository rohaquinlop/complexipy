# complexipy

An extremely fast Python library to calculate the cognitive complexity of
python files, written in Rust.

## Installation

```bash
pip install complexipy
```

## Usage

To calculate the cognitive complexity of a single file, you can use the
following command:

```bash
complexipy path/to/file.py
```

To calculate the cognitive complexity of a directory, you can use the following
command:

```bash
complexipy path/to/directory
```

by default the maximum cognitive complexity is 15, you can change it using the
`-m` option, for example:

```bash
complexipy path/to/directory -m 20
```

For example, given the following file:

```python
def a_decorator(a, b):
    def inner(func):
        return func
    return inner

def b_decorator(a, b):
    def inner(func):
        if func:
            return None
        return func
    return inner
```

The cognitive complexity of the file is 3, and the output of the command
`complexipy path/to/file.py` will be:

```bash
───────────────────────────── complexipy 0.1.0 🐙 ──────────────────────────────
test_decorator.py
Analysis completed! 🎉
                  Summary
┏━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━━━━┓
┃ File              ┃ Cognitive Complexity ┃
┡━━━━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━━━━━━━━━┩
│ test_decorator.py │ 1                    │
└───────────────────┴──────────────────────┘
Total files: 1
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file
for details.

## Acknowledgments

- This project is inspired by the sonar way to calculate the cognitive
complexity.

## References

- [Cognitive Complexity](https://www.sonarsource.com/resources/cognitive-complexity/)
