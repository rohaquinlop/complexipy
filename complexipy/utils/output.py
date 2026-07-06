from __future__ import annotations

from typing import (
    Dict,
    List,
    Optional,
    Tuple,
    Union,
)

from rich.align import Align
from rich.console import Console
from rich.text import Text

from complexipy._complexipy import (
    Applicability,
    FileComplexity,
    FunctionComplexity,
    RuleCategory,
)
from complexipy.types import (
    Sort,
)
from complexipy.utils import dataclasses as dc


def output_summary(
    console: Console,
    files: List[FileComplexity],
    failed_only: bool,
    sort: Sort,
    ignore_complexity: bool,
    max_complexity: int,
    previous_functions: Optional[Dict[Tuple[str, str, str], int]],
    snapshot_map: Optional[Dict[Tuple[str, str, str], int]] = None,
    plain: bool = False,
    top: Optional[int] = None,
    suggest_refactors: bool = False,
) -> bool:
    (
        file_entries,
        failing_functions,
        total_functions,
    ) = build_output_rows(
        files, failed_only, sort, max_complexity, snapshot_map
    )
    has_success = not failing_functions or ignore_complexity

    if top is not None:
        file_entries = truncate_top_n(file_entries, top)

    if plain:
        output_plain(console, file_entries)
        return has_success

    if failed_only and not file_entries:
        console.print(
            f"No function{'s' if len(files) > 1 else ''} were found with complexity greater than {max_complexity}."
        )
    elif total_functions == 0:
        console.print(
            Align.center(
                "No files were found with functions. No complexity was calculated."
            )
        )
    else:
        output_file_entries(
            console,
            file_entries,
            failing_functions,
            ignore_complexity,
            previous_functions,
            max_complexity,
            suggest_refactors,
        )
    return has_success


def output_plain(
    console: Console,
    file_entries: List[dc.FileEntry],
) -> None:
    for entry in file_entries:
        for function in entry.functions:
            path = normalize_path(function.path, function.file_name)
            console.print(f"{path} {function.name} {function.complexity}")


def truncate_top_n(
    file_entries: List[dc.FileEntry],
    n: int,
) -> List[dc.FileEntry]:
    all_functions: List[Tuple[str, dc.FunctionRow]] = []
    for entry in file_entries:
        for function in entry.functions:
            all_functions.append((entry.path, function))

    all_functions.sort(key=lambda x: x[1].complexity, reverse=True)
    top_functions = all_functions[:n]

    result: List[dc.FileEntry] = []
    for path, function in top_functions:
        if result and result[-1].path == path:
            result[-1].functions.append(function)
        else:
            result.append(dc.FileEntry(path=path, functions=[function]))
    return result


def output_file_entries(
    console: Console,
    file_entries: List[dc.FileEntry],
    failing_functions: Dict[str, List[str]],
    ignore_complexity: bool,
    previous_functions: Optional[Dict[Tuple[str, str, str], int]],
    max_complexity: int,
    suggest_refactors: bool = False,
) -> None:
    for entry in file_entries:
        console.print(f"[bold]{entry.path}[/bold]")
        for function in entry.functions:
            status_text = format_status_text(function.passed)
            delta_text = output_delta_text(
                previous_functions, function, max_complexity
            )
            console.print(
                f"    {function.name} {function.complexity}{delta_text} {status_text}"
            )
            if suggest_refactors:
                output_refactor_plans(console, function)
        console.print()

    if failing_functions:
        console.print("[bold red]Failed functions:[/bold red]")
        for path, functions in failing_functions.items():
            console.print(f" - {path}: {', '.join(sorted(functions))}")
        if ignore_complexity:
            console.print(
                "[yellow]--ignore-complexity enabled: failures will not affect the exit code.[/yellow]"
            )
    else:
        console.print(
            "[bold green]All functions are within the allowed complexity.[/bold green]"
        )


def output_refactor_plans(console: Console, function: dc.FunctionRow) -> None:
    if not function.refactor_plans:
        return

    console.print("\n      [bold]Refactor Suggestions:[/bold]")
    for index, plan in enumerate(function.refactor_plans, start=1):
        _output_single_plan(console, plan, index)


def _output_single_plan(console: Console, plan, index: int) -> None:
    category_icon = _get_category_icon(plan.category)
    category_name = _get_category_name(plan.category)
    applicability_icon = _get_applicability_icon(plan.applicability)
    applicability_name = _get_applicability_name(plan.applicability)

    console.print(
        f"\n      [{index}] [bold cyan]{plan.rule_id}[/bold cyan] {plan.title}"
    )
    console.print(
        f"          Category: {category_icon} {category_name} "
        f"| Applicability: {applicability_icon} {applicability_name}"
    )
    console.print(
        f"          Lines {plan.line_start}-{plan.line_end} "
        f"-> Estimated reduction: [green]-{plan.estimated_reduction}[/green] complexity "
        f"({plan.current_complexity} -> {plan.estimated_complexity_after})"
    )

    if plan.description:
        console.print(f"\n          [dim]{plan.description}[/dim]")

    if plan.before_code or plan.after_code:
        console.print()
        _output_code_comparison(console, plan)
        if plan.applicability and "MaybeIncorrect" in str(plan.applicability):
            console.print(
                "\n          [dim]Note: The 'After' code is illustrative and may need adjustments.[/dim]"
            )

    if plan.explanation:
        console.print(
            f"\n          [yellow]\u26a1[/yellow] {plan.explanation}"
        )

    if not plan.before_code and plan.steps:
        console.print("\n          [bold]Steps:[/bold]")
        for step in plan.steps:
            console.print(f"            - {step}")

    _output_plan_references(console, plan.rule_id, plan.references)


def _get_category_icon(category: Union[str, RuleCategory]) -> str:
    category_str = str(category)
    if "Complexity" in category_str:
        return "\U0001f9e9"
    elif "Readability" in category_str:
        return "\U0001f4d6"
    elif "Maintainability" in category_str:
        return "\U0001f527"
    return "\u2699"


def _get_category_name(category: Union[str, RuleCategory]) -> str:
    category_str = str(category)
    if "Complexity" in category_str:
        return "Complexity"
    elif "Readability" in category_str:
        return "Readability"
    elif "Maintainability" in category_str:
        return "Maintainability"
    return category_str


def _get_applicability_icon(applicability: Union[str, Applicability]) -> str:
    applicability_str = str(applicability)
    if "MachineApplicable" in applicability_str:
        return "[green]\u2705[/green]"
    elif "MaybeIncorrect" in applicability_str:
        return "[yellow]\u26a0\ufe0f[/yellow]"
    elif "Informational" in applicability_str:
        return "[blue]\u2139\ufe0f[/blue]"
    return "\u2753"


def _get_applicability_name(applicability: Union[str, Applicability]) -> str:
    applicability_str = str(applicability)
    if "MachineApplicable" in applicability_str:
        return "Auto-applicable"
    elif "MaybeIncorrect" in applicability_str:
        return "Needs review"
    elif "Informational" in applicability_str:
        return "Informational"
    return applicability_str


def _output_plan_references(console: Console, rule_id: str, references: list) -> None:
    doc_url = _get_doc_url(rule_id)
    if doc_url or references:
        console.print("\n          [dim]References:[/dim]")
        if doc_url:
            console.print(
                f"            [blue underline]{doc_url}[/blue underline]"
            )
        for ref in references:
            console.print(
                f"            [blue underline]{ref}[/blue underline]"
            )


def _get_doc_url(rule_id: str) -> str:
    doc_anchors = {
        "C001": "c001-flatten-nested-conditions",
        "C002": "c002-loop-guards",
        "C003": "c003-extract-helper-function",
        "C004": "c004-split-dispatcher",
        "C005": "c005-extract-predicate",
        "C006": "c006-reduce-nesting-depth",
        "C011": "c011-flatten-tryexcept",
    }
    anchor = doc_anchors.get(rule_id)
    if anchor:
        return f"https://rohaquinlop.github.io/complexipy/refactoring-rules/#{anchor}"
    return ""


def _output_code_comparison(console: Console, plan) -> None:
    console.print("          [bold]Code:[/bold]")

    if plan.before_code:
        console.print("          [dim]Before:[/dim]")
        _output_code_snippet(console, plan.before_code, indent=12)

    if plan.after_code:
        console.print("\n          [dim]After:[/dim]")
        _output_code_snippet(console, plan.after_code, indent=12)


def _output_code_snippet(console, snippet, indent: int = 8) -> None:
    if not snippet or not snippet.text:
        return

    lines = snippet.text.split("\n")
    line_num = snippet.line_start

    for line in lines:
        highlighted = _highlight_python_line(line)
        console.print(f"{' ' * indent}{line_num:4d} | {highlighted}")
        line_num += 1


def _highlight_python_line(line: str) -> str:
    import re

    keywords = [
        "def",
        "class",
        "if",
        "elif",
        "else",
        "for",
        "while",
        "return",
        "import",
        "from",
        "try",
        "except",
        "finally",
        "with",
        "as",
        "raise",
        "pass",
        "break",
        "continue",
        "yield",
        "lambda",
    ]

    constants = ["True", "False", "None"]
    boolean_ops = ["and", "or", "not", "in", "is"]

    result = line

    result = re.sub(
        r'("""[^"]*"""|\'\'\'[^\']*\'\'\' |"[^"]*"|\'[^\']*\')',
        r"[green]\1[/green]",
        result,
    )

    result = re.sub(r"(\s*#.*)$", r"[dim]\1[/dim]", result)

    for keyword in keywords:
        result = re.sub(
            rf"\b{keyword}\b", f"[bold blue]{keyword}[/bold blue]", result
        )

    for op in boolean_ops:
        result = re.sub(
            rf"\b{op}\b", f"[bold magenta]{op}[/bold magenta]", result
        )

    for constant in constants:
        result = re.sub(
            rf"\b{constant}\b", f"[bold cyan]{constant}[/bold cyan]", result
        )

    result = re.sub(r"\b(\d+)\b", r"[yellow]\1[/yellow]", result)

    return result


def format_status_text(passed: bool) -> str:
    if passed:
        return "[bold black on green] :white_heavy_check_mark: PASSED [/bold black on green]"
    return "[bold white on red] :cross_mark: FAILED [/bold white on red]"


def output_delta_text(
    previous_functions: Optional[Dict[Tuple[str, str, str], int]],
    function: dc.FunctionRow,
    max_complexity: int,
) -> str:
    if previous_functions is None:
        return ""

    if function.complexity <= max_complexity:
        return ""

    key = (function.path, function.file_name, function.name)
    previous = previous_functions.get(key)
    if previous is None:
        return f" (new, \u0394 = +{function.complexity})"
    if previous != function.complexity:
        delta = function.complexity - previous
        return f" (last: {previous}, \u0394 = {delta:+d})"
    return ""


def _is_function_passing(
    function: FunctionComplexity,
    file_path: str,
    file_name: str,
    max_complexity: int,
    snapshot_map: Optional[Dict[Tuple[str, str, str], int]],
) -> bool:
    if function.complexity <= max_complexity:
        return True
    if snapshot_map is None:
        return False
    prev = snapshot_map.get((file_path, file_name, function.name))
    return prev is not None and function.complexity <= prev


def build_output_rows(
    files: List[FileComplexity],
    failed_only: bool,
    sort: Sort,
    max_complexity: int,
    snapshot_map: Optional[Dict[Tuple[str, str, str], int]] = None,
) -> Tuple[List[dc.FileEntry], Dict[str, List[str]], int]:
    file_entries: List[dc.FileEntry] = []
    failing_functions: Dict[str, List[str]] = {}
    total_functions = 0

    for file in files:
        sorted_functions = sort_functions(file.functions, sort)
        displayable_functions: List[dc.FunctionRow] = []

        for function in sorted_functions:
            total_functions += 1
            passed = _is_function_passing(
                function,
                file.path,
                file.file_name,
                max_complexity,
                snapshot_map,
            )

            if failed_only and passed:
                continue

            if not passed:
                full_path = normalize_path(file.path, file.file_name)
                failing_functions.setdefault(full_path, []).append(
                    function.name
                )

            displayable_functions.append(
                dc.FunctionRow(
                    name=function.name,
                    complexity=function.complexity,
                    passed=passed,
                    path=file.path,
                    file_name=file.file_name,
                    refactor_plans=function.refactor_plans,
                )
            )

        if displayable_functions:
            file_entries.append(
                dc.FileEntry(
                    path=normalize_path(file.path, file.file_name),
                    functions=displayable_functions,
                )
            )

    return file_entries, failing_functions, total_functions


def sort_functions(
    functions: List[FunctionComplexity], sort: Sort
) -> List[FunctionComplexity]:
    if sort == Sort.file_name:
        return sorted(functions, key=lambda f: f.name.lower())
    reverse = sort == Sort.desc
    return sorted(functions, key=lambda f: f.complexity, reverse=reverse)


def normalize_path(path: str, file_name: str) -> str:
    cleaned_path = path.rstrip("/")
    if cleaned_path.endswith(file_name):
        return cleaned_path
    if cleaned_path:
        return f"{cleaned_path}/{file_name}"
    return file_name


def print_invalid_paths(
    console: Console, quiet: bool, invalid_paths: List[str]
):
    has_success = True

    if invalid_paths:
        has_success = False

    if quiet:
        return has_success

    for failed_path in invalid_paths:
        text = Text()
        text.append("error", style="bold red")
        text.append(f": Failed to process {failed_path}", style="bold white")
        text.append(" - Please check file/folder exists or check syntax")
        console.print(text)

    return has_success


def has_success_functions(
    files: List[FileComplexity], max_complexity: int
) -> bool:
    return all(
        all(
            function.complexity <= max_complexity for function in file.functions
        )
        for file in files
    )
