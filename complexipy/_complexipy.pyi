"""
Type stubs for complexipy._complexipy module.

This module provides Rust-powered cognitive complexity analysis for Python code.
Cognitive complexity is a metric that measures how difficult a function is to
understand and maintain, focusing on control flow structures that make code
harder to reason about.
"""

from typing import List

class LineComplexity:
    """
    Represents the cognitive complexity contribution of a single line of code.

    Cognitive complexity is incremented by control flow structures like:
    - if/elif statements (+1 each)
    - loops (for/while) (+1 each)
    - except clauses (+1 each)
    - Nested structures (+1 for each level of nesting)
    - Boolean operators in conditions (and/or) (+1 each)

    This class tracks which specific lines contribute to the overall complexity
    score, helping developers identify the most complex parts of their functions.

    Example:
        >>> # For a line like: if condition and other_condition:
        >>> line_complexity = LineComplexity(line=5, complexity=2)
        >>> print(f"Line {line_complexity.line} adds {line_complexity.complexity} to complexity")
    """

    line: int
    """The line number (1-indexed) in the source code."""

    complexity: int
    """
    The cognitive complexity value contributed by this line.

    - 0: No complexity contribution (simple statements)
    - 1+: Complexity added by control flow structures on this line
    """

    def __init__(self, line: int, complexity: int) -> None: ...

class FunctionComplexity:
    """
    Represents the cognitive complexity analysis of a Python function.

    This class provides a complete complexity analysis for a single function,
    including the total complexity score and line-by-line breakdown. The
    cognitive complexity score helps assess how difficult a function is to
    understand and maintain.

    Complexity Guidelines:
    - 1-10: Simple function, easy to understand
    - 11-15: Moderate complexity, consider refactoring
    - 16+: High complexity, should be refactored

    The analysis includes nested complexity penalties, where deeply nested
    control structures add additional complexity points.

    Example:
        >>> func = FunctionComplexity(
        ...     name="process_data",
        ...     complexity=8,
        ...     line_start=10,
        ...     line_end=25,
        ...     line_complexities=[LineComplexity(12, 2), LineComplexity(15, 1)]
        ... )
        >>> print(f"Function '{func.name}' has complexity {func.complexity}")
        >>> # Find the most complex lines
        >>> complex_lines = [lc for lc in func.line_complexities if lc.complexity > 0]
    """

    name: str
    """
    The name of the function as it appears in the source code.

    For methods, this includes just the method name (not the class).
    For nested functions, this is the inner function name.
    """

    complexity: int
    """
    The total cognitive complexity score for this function.

    This is the sum of all complexity contributions from control flow
    structures within the function, including nesting penalties.
    Higher values indicate more complex, harder-to-understand code.
    """

    line_start: int
    """
    The starting line number of the function definition (1-indexed).

    Points to the line containing 'def function_name(...):' 
    """

    line_end: int
    """
    The ending line number of the function definition (1-indexed).

    Points to the last line that belongs to this function's body.
    """

    line_complexities: List[LineComplexity]
    """
    Detailed complexity information for each line in the function.

    This list contains LineComplexity objects for every line in the function,
    allowing you to identify exactly which lines contribute to the overall
    complexity score. Lines with zero complexity are included for completeness.

    Example:
        >>> # Find lines that add complexity
        >>> complex_lines = [lc for lc in func.line_complexities if lc.complexity > 0]
        >>> for line_info in complex_lines:
        ...     print(f"Line {line_info.line}: +{line_info.complexity}")
    """

    def __init__(
        self,
        name: str,
        complexity: int,
        line_start: int,
        line_end: int,
        line_complexities: List[LineComplexity],
    ) -> None: ...

class FileComplexity:
    """
    Represents the cognitive complexity analysis of a Python source file.

    This class aggregates complexity information for all functions found in a
    Python file, providing both individual function analysis and file-level
    metrics. It's useful for understanding the overall complexity of a module
    and identifying which functions need attention.

    File Complexity Guidelines:
    - Low: Most functions have complexity < 10
    - Medium: Some functions have complexity 10-15
    - High: Multiple functions have complexity > 15

    The file complexity is the sum of all function complexities, helping
    identify files that may be doing too much or need to be split up.

    Example:
        >>> file_analysis = FileComplexity(
        ...     path="/path/to/mymodule.py",
        ...     file_name="mymodule.py",
        ...     functions=[func1, func2, func3],
        ...     complexity=25
        ... )
        >>> # Find the most complex functions
        >>> complex_funcs = [f for f in file_analysis.functions if f.complexity > 10]
        >>> print(f"File has {len(complex_funcs)} complex functions")
    """

    path: str
    """
    The absolute file path to the analyzed Python file.

    This is the complete path as resolved by the system, useful for
    identifying the exact location of the file in the filesystem.
    """

    file_name: str
    """
    The basename of the file (filename without directory path).

    Example: For "/path/to/mymodule.py", this would be "mymodule.py"
    """

    functions: List[FunctionComplexity]
    """
    List of complexity analysis results for each function in the file.

    This includes all top-level functions and methods found in the file.
    Nested functions are analyzed as part of their containing function.
    The list is ordered by appearance in the source code.

    Example:
        >>> # Find the most complex function
        >>> most_complex = max(file_analysis.functions, key=lambda f: f.complexity)
        >>> print(f"Most complex function: {most_complex.name} ({most_complex.complexity})")
    
            >>> # Get functions that exceed threshold
        >>> threshold = 15
        >>> problematic = [f for f in file_analysis.functions if f.complexity > threshold]
    """

    complexity: int
    """
    The total cognitive complexity of the entire file.

    This is calculated as the sum of complexity scores from all functions
    in the file. It provides a high-level metric for the overall complexity
    of the module and can help identify files that need refactoring.
    """

    def __init__(
        self,
        path: str,
        file_name: str,
        functions: List[FunctionComplexity],
        complexity: int,
    ) -> None: ...

class CodeComplexity:
    """
    Represents the cognitive complexity analysis of a Python code string.

    This class is used when analyzing Python code provided as a string rather
    than from a file. It's particularly useful for:
    - Analyzing code snippets or templates
    - Testing complexity in unit tests
    - Analyzing dynamically generated code
    - Integration with code editors and IDEs

    The analysis works identically to file-based analysis, parsing the code
    string to identify functions and calculate their complexity scores.

    Example:
        >>> code = '''
        ... def complex_function(x, y):
        ...     if x > 0:
        ...         for i in range(y):
        ...             if i % 2 == 0 and i > 5:
        ...                 return i * x
        ...     return 0
        ... '''
        >>> analysis = code_complexity(code)
        >>> print(f"Code has {len(analysis.functions)} functions")
        >>> print(f"Total complexity: {analysis.complexity}")
    """

    functions: List[FunctionComplexity]
    """
    List of complexity analysis results for each function found in the code.

    This includes all function definitions found in the provided code string,
    analyzed in the same way as file-based analysis. Functions are ordered
    by their appearance in the code.

    Example:
        >>> for func in analysis.functions:
        ...     print(f"Function '{func.name}': complexity {func.complexity}")
        ...     if func.complexity > 10:
        ...         print("  -> Consider refactoring this function")
    """

    complexity: int
    """
    The total cognitive complexity of all functions in the code string.

    This is the sum of complexity scores from all functions found in the
    provided code. It gives an overall measure of how complex the code is.
    """

    def __init__(
        self, functions: List[FunctionComplexity], complexity: int
    ) -> None: ...

def main(paths: List[str], quiet: bool) -> List[FileComplexity]:
    """
    Analyze cognitive complexity of Python files and directories.

    This is the main analysis function that processes multiple paths, which can
    be individual Python files, directories containing Python files, or Git
    repository URLs. It recursively analyzes all Python files found.

    The function handles various input types:
    - Local Python files: '/path/to/file.py'
    - Local directories: '/path/to/project/' (analyzes all .py files)
    - Git repositories: 'https://github.com/user/repo.git'

    Args:
        paths: List of file paths, directory paths, or Git repository URLs to analyze.
               Each path is processed independently and all results are combined.
        quiet: If True, suppresses console output during analysis. Useful for
               programmatic usage where you only want the returned data.

    Returns:
        List of FileComplexity objects, one for each Python file analyzed.
        Files are ordered by discovery order during filesystem traversal.

    Raises:
        Various exceptions may be raised for invalid paths, permission errors,
        or Git repository access issues.

    Example:
        >>> # Analyze a single file
        >>> results = main(['/path/to/script.py'], quiet=True)
        >>>
        >>> # Analyze entire project directory
        >>> results = main(['/path/to/project/'], quiet=False)
        >>>
        >>> # Find files with high complexity
        >>> complex_files = [f for f in results if f.complexity > 50]
    """
    ...

def code_complexity(code: str) -> CodeComplexity:
    """
    Analyze cognitive complexity of Python code provided as a string.

    This function parses and analyzes Python source code from a string,
    identifying all function definitions and calculating their cognitive
    complexity scores. It's ideal for analyzing code snippets, templates,
    or dynamically generated code.

    The analysis follows the same cognitive complexity rules as file-based
    analysis, measuring control flow structures and nesting levels that
    make code harder to understand.

    Args:
        code: A string containing valid Python source code. The code should
              be properly formatted and syntactically correct. Syntax errors
              will cause the analysis to fail.

    Returns:
        CodeComplexity object containing the analysis results, including
        individual function complexity scores and total complexity.

    Raises:
        SyntaxError: If the provided code string contains invalid Python syntax.

    Example:
        >>> code_sample = '''
        ... def fibonacci(n):
        ...     if n <= 1:
        ...         return n
        ...     elif n == 2:
        ...         return 1
        ...     else:
        ...         return fibonacci(n-1) + fibonacci(n-2)
        ...
        ... def factorial(n):
        ...     result = 1
        ...     for i in range(1, n + 1):
        ...         result *= i
        ...     return result
        ... '''
        >>> analysis = code_complexity(code_sample)
        >>> print(f"Found {len(analysis.functions)} functions")
        >>> print(f"Total complexity: {analysis.complexity}")
        >>>
        >>> # Analyze each function
        >>> for func in analysis.functions:
        ...     print(f"{func.name}: complexity {func.complexity}")
    """
    ...

def file_complexity(file_path: str, base_path: str) -> FileComplexity:
    """
    Analyze cognitive complexity of a single Python source file.

    This function reads and analyzes a Python file from the filesystem,
    identifying all function definitions and calculating their cognitive
    complexity scores. It's useful for analyzing individual files or
    integrating complexity analysis into custom tools.

    The function handles file reading, parsing, and complexity calculation,
    providing detailed results for all functions found in the file.

    Args:
        file_path: Absolute path to the Python file to analyze. The file
                   must exist and be readable. Should point to a .py file
                   containing valid Python source code.
        base_path: Base directory path used for calculating relative paths
                   in the analysis results. This is typically the project
                   root directory and affects the 'path' field in the
                   returned FileComplexity object.

    Returns:
        FileComplexity object containing complete analysis results for the
        file, including all functions found and their complexity scores.

    Raises:
        FileNotFoundError: If the specified file_path does not exist.
        PermissionError: If the file cannot be read due to permissions.
        SyntaxError: If the Python file contains syntax errors.
        UnicodeDecodeError: If the file cannot be decoded as text.

    Example:
        >>> # Analyze a specific file
        >>> result = file_complexity(
        ...     file_path="/project/src/mymodule.py",
        ...     base_path="/project"
        ... )
        >>> print(f"File: {result.file_name}")
        >>> print(f"Total complexity: {result.complexity}")
        >>> print(f"Functions analyzed: {len(result.functions)}")
        >>>
        >>> # Find the most complex function
        >>> if result.functions:
        ...     most_complex = max(result.functions, key=lambda f: f.complexity)
        ...     print(f"Most complex: {most_complex.name} (score: {most_complex.complexity})")
    """
    ...

def output_csv(
    output_path: str,
    files_complexities: List[FileComplexity],
    sort: str,
    show_details: bool,
    max_complexity: int,
) -> None:
    """
    Export complexity analysis results to a CSV file for reporting and analysis.

    This function creates a CSV file containing complexity metrics that can be
    imported into spreadsheet applications, data analysis tools, or used for
    automated reporting. The CSV format makes it easy to track complexity
    trends over time or compare different codebases.

    The CSV includes columns for file paths, function names, complexity scores,
    line numbers, and other relevant metrics. The level of detail depends on
    the show_details parameter.

    Args:
        output_path: File system path where the CSV file will be written.
                     If the file exists, it will be overwritten. The directory
                     must exist and be writable.
        files_complexities: List of FileComplexity objects from analysis
                            results. Each file's functions will be included
                            in the CSV output.
        sort: Sort order for the results in the CSV file. Valid options:
              - 'asc': Sort by complexity score (ascending, lowest first)
              - 'desc': Sort by complexity score (descending, highest first)
              - 'name': Sort alphabetically by function name
        show_details: If True, includes detailed per-function information.
                      If False, only includes summary information.
        max_complexity: Functions with complexity above this threshold will
                        be highlighted or filtered in the output. Use 0 to
                        include all functions regardless of complexity.

    Raises:
        PermissionError: If the output path is not writable.
        FileNotFoundError: If the output directory does not exist.

    Example:
        >>> results = main(['/path/to/project'], quiet=True)
        >>> output_csv(
        ...     output_path='/tmp/complexity_report.csv',
        ...     files_complexities=results,
        ...     sort='desc',
        ...     show_details=True,
        ...     max_complexity=15
        ... )
        >>> print("CSV report generated successfully")
    """
    ...

def output_json(
    output_path: str,
    files_complexities: List[FileComplexity],
    show_details: bool,
    max_complexity: int,
) -> None:
    """
    Export complexity analysis results to a JSON file for programmatic consumption.

    This function serializes complexity analysis results into a structured JSON
    format that can be easily consumed by other tools, CI/CD pipelines, or
    custom applications. The JSON format preserves the full hierarchy of
    files, functions, and line-by-line complexity information.

    The JSON output is machine-readable and perfect for integration with
    automated quality gates, dashboards, or further data processing. It
    maintains all the detailed information from the analysis.

    Args:
        output_path: File system path where the JSON file will be written.
                     If the file exists, it will be overwritten. The directory
                     must exist and be writable.
        files_complexities: List of FileComplexity objects from analysis
                            results. The complete structure will be serialized
                            to JSON, preserving all nested information.
        show_details: If True, includes detailed line-by-line complexity
                      information for each function. If False, only includes
                      summary metrics for better performance and smaller files.
        max_complexity: Functions with complexity above this threshold may
                        be flagged or filtered in the output. Use 0 to include
                        all functions regardless of complexity.

    Raises:
        PermissionError: If the output path is not writable.
        FileNotFoundError: If the output directory does not exist.
        JSONEncodeError: If the data cannot be serialized to JSON.

    Example:
        >>> results = main(['/path/to/project'], quiet=True)
        >>> output_json(
        ...     output_path='/tmp/complexity_report.json',
        ...     files_complexities=results,
        ...     show_details=True,
        ...     max_complexity=0
        ... )
        >>>
        >>> # The JSON can be loaded by other tools
        >>> import json
        >>> with open('/tmp/complexity_report.json') as f:
        ...     data = json.load(f)
        >>> print(f"Analyzed {len(data)} files")
    """
    ...
