# complexipy

<p align="center">
    <a href="https://rohaquinlop.github.io/complexipy/"><img src="https://raw.githubusercontent.com/rohaquinlop/complexipy/main/docs/img/logo-vector.svg" alt="complexipy"></a>
</p>

<p align="center">
    <em>An extremely fast Python library to calculate the cognitive complexity of python files, written in Rust.</em>
</p>

<p align="center">
    <a href="https://sonarcloud.io/summary/new_code?id=rohaquinlop_complexipy" target="_blank">
        <img src="https://sonarcloud.io/api/project_badges/measure?project=rohaquinlop_complexipy&metric=alert_status" alt="Quality Gate">
    </a>
    <a href="https://pypi.org/project/complexipy" target="_blank">
    <img src="https://img.shields.io/pypi/v/complexipy?color=%2334D058&label=pypi%20package" alt="Package version">
</a>
</p>

---

**Documentation**: <a href="https://rohaquinlop.github.io/complexipy/" target="_blank">https://rohaquinlop.github.io/complexipy/</a>

**Source Code**: <a href="https://github.com/rohaquinlop/complexipy" target="_blank">https://github.com/rohaquinlop/complexipy</a>

**PyPI**: <a href="https://pypi.org/project/complexipy/" target="_blank">https://pypi.org/project/complexipy/</a>

---

## Requirements

- Python >= 3.11
- You also need to install `git` in your computer if you want to analyze a git repository.

## Installation

```bash
pip install complexipy
```

## Usage

To run **complexipy** you can use the following command:

```shell
complexipy .                         # Use complexipy to analyze the current directory and any subdirectories
complexipy path/to/directory         # Use complexipy to analyze a specific directory and any subdirectories
complexipy git_repository_url        # Use complexipy to analyze a git repository
complexipy path/to/file.py           # Use complexipy to analyze a specific file
complexipy path/to/file.py -c 20     # Use the -c option to set the maximum congnitive complexity, default is 15
complexipy path/to/directory -c 0    # Set the maximum cognitive complexity to 0 to disable the exit with error
complexipy path/to/directory -o      # Use the -o option to output the results to a CSV file, default is False
```

If the cognitive complexity of a file is greater than the maximum cognitive, then
the return code will be 1 and exit with error, otherwise it will be 0.

## Example

### Analyzing a file

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

The cognitive complexity of the file is 1, and the output of the command
`complexipy path/to/file.py` will be:

```txt
─────────────────────── complexipy 0.2.2 🐙 ───────────────────────
- Finished analysis in test_decorator.py
──────────────────── 🎉 Analysis completed!🎉 ─────────────────────
                       Summary
┏━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━━━━━━━━┳━━━━━━━━━━━━┓
┃ Path              ┃ File              ┃ Complexity ┃
┡━━━━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━━━━━━━━╇━━━━━━━━━━━━┩
│ test_decorator.py │ test_decorator.py │ 1          │
└───────────────────┴───────────────────┴────────────┘
🧠 Total Cognitive Complexity in ./tests/test_decorator.py: 1
1 files analyzed in 0.0005 seconds
```

#### Output to a CSV file

If you want to output the results to a CSV file, you can use the `-o` option,
this is really useful if you want to integrate **complexipy** with other tools,
for example, a CI/CD pipeline. You will get the output in the console and will
create a CSV file with the results of the analysis.

The filename will be `complexipy.csv` and will be saved in the current directory.

```bash
$ complexipy path/to/file.py -o
```

The output will be:

```csv
Path,File Name,Cognitive Complexity
test_decorator.py,test_decorator.py,1
```

### Analyzing a directory

You can also analyze a directory, for example:

```bash
$ complexipy .
```

And **complexipy** will analyze all the files in the current directory and any
subdirectories.

### Analyzing a git repository

You can also analyze a git repository, for example:

```bash
$ complexipy https://github.com/rohaquinlop/complexipy
```

And to generate the output to a CSV file:

```bash
$ complexipy https://github.com/rohaquinlop/complexipy -o
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file
for details.

## Acknowledgments

- Thanks to G. Ann Campbell for publishing the paper "Cognitive Complexity a new
way to measure understandability".
- This project is inspired by the Sonar way to calculate the cognitive
complexity.

## References

- [Cognitive Complexity](https://www.sonarsource.com/resources/cognitive-complexity/)
