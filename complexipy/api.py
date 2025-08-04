from complexipy import _complexipy
from pathlib import Path
from complexipy._complexipy import CodeComplexity, FileComplexity


def code_complexity(
    code: str,
) -> CodeComplexity:
    """
    Analyze cognitive complexity of Python code provided as a string.

    This function parses and analyzes Python source code from a string,
    identifying all function definitions and calculating their cognitive
    complexity scores. Perfect for analyzing code snippets, templates,
    or dynamically generated code.

    Args:
        code: A string containing valid Python source code. Must be
              syntactically correct Python code.

    Returns:
        CodeComplexity object containing the analysis results, including
        individual function complexity scores and total complexity.

    Raises:
        SyntaxError: If the provided code string contains invalid Python syntax.

    Example:
        >>> code = '''
        ... def fibonacci(n):
        ...     if n <= 1:
        ...         return n
        ...     else:
        ...         return fibonacci(n-1) + fibonacci(n-2)
        ... '''
        >>> result = code_complexity(code)
        >>> print(f"Total complexity: {result.complexity}")
    """
    return _complexipy.code_complexity(code)


def file_complexity(file_path: str) -> FileComplexity:
    """
    Analyze cognitive complexity of a single Python source file.

    This function reads and analyzes a Python file from the filesystem,
    identifying all function definitions and calculating their cognitive
    complexity scores. Useful for analyzing individual files or integrating
    complexity analysis into custom tools.

    Args:
        file_path: Path to the Python file to analyze. Can be relative or
                   absolute. The file must exist and be readable.

    Returns:
        FileComplexity object containing complete analysis results for the
        file, including all functions found and their complexity scores.

    Raises:
        FileNotFoundError: If the specified file does not exist.
        PermissionError: If the file cannot be read due to permissions.
        SyntaxError: If the Python file contains syntax errors.

    Example:
        >>> result = file_complexity('mymodule.py')
        >>> print(f"File: {result.file_name}")
        >>> print(f"Total complexity: {result.complexity}")
        >>> for func in result.functions:
        ...     if func.complexity > 10:
        ...         print(f"Complex function: {func.name} ({func.complexity})")
    """
    path = Path(file_path)
    base_path = path.parent
    return _complexipy.file_complexity(
        file_path=path.resolve().as_posix(),
        base_path=base_path.resolve().as_posix(),
    )
