# complexipy

An extremely fast Python library to calculate the cognitive complexity of
python files, written in Rust.

## Installation

```bash
pip install complexipy
```

## Usage

To calculate the cognitive complexity of a single file, you can use the following command:

```bash
complexipy path/to/file.py
```

To calculate the cognitive complexity of a directory, you can use the following command:

```bash
complexipy path/to/directory -d
```

by default the maximum cognitive complexity is 15, you can change it using the following command:

```bash
complexipy path/to/directory -d -m 20
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- This project is inspired by the sonar way to calculate the cognitive complexity.
