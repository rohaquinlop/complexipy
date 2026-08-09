from __future__ import annotations

import platform
from typing import (
    Dict,
    List,
    Optional,
    Tuple,
    Union,
)

import typer
from rich.console import Console
from rich.markup import escape
from rich.padding import Padding
from rich.syntax import Syntax
from rich.text import Text

from complexipy._complexipy import (
    Applicability,
    CodeSuggestion,
    FileComplexity,
    FunctionComplexity,
    RefactorPlan,
    RuleCategory,
)
from complexipy.types import (
    ColorTypes,
    OutputFormat,
    Sort,
)
from complexipy.utils.cache import remember_previous_functions
from complexipy.utils.dataclasses import FileEntry, FunctionRow
from complexipy.utils.paths import resolve_output_paths


def handle_console_settings(
    color: ColorTypes, quiet: bool, plain: bool = False
) -> Console:
    if plain:
        return Console(color_system=None, highlight=False)

    if color == ColorTypes.no:
        console = Console(color_system=None)
    elif color == ColorTypes.yes:
        console = Console(color_system="standard")
    else:
        console = Console(color_system="auto")

    if not quiet:
        if platform.system() == "Windows":
            console.rule("complexipy")
        else:
            console.rule(":octopus: complexipy")

    return console


def handle_display(
    console: Console,
    files_complexities: List[FileComplexity],
    paths: List[str],
    failed: bool,
    sort: Sort,
    ignore_complexity: bool,
    max_complexity_allowed: int,
    active_snapshot_map: Optional[Dict],
    quiet: bool,
    plain: bool,
    invocation_path: str,
    top: Optional[int] = None,
    suggest_refactors: bool = False,
) -> bool:
    if files_complexities:
        previous_functions = remember_previous_functions(
            invocation_path, paths, files_complexities
        )
    else:
        previous_functions = None

    if quiet:
        return has_success_functions(
            files_complexities, max_complexity_allowed, active_snapshot_map
        )

    effective_sort = Sort.desc if top is not None else sort
    has_success = output_summary(
        console,
        files_complexities,
        failed,
        effective_sort,
        ignore_complexity,
        max_complexity_allowed,
        previous_functions,
        active_snapshot_map,
        plain,
        top,
        suggest_refactors,
        invocation_path,
    )
    return has_success


def handle_results_storage(
    console: Console,
    output_formats: List[OutputFormat],
    output: Optional[str],
    files_complexities: List[FileComplexity],
    sort: str,
    show_details: bool,
    max_complexity: int,
    invocation_path: str,
    suggest_refactors: bool = False,
) -> None:
    output_paths = resolve_output_paths(output_formats, output, invocation_path)

    for output_format in output_formats:
        output_path = output_paths[output_format]

        if output_format == OutputFormat.csv:
            from complexipy.utils.csv import store_csv

            store_csv(
                output_path,
                files_complexities,
                sort,
                show_details,
                max_complexity,
            )
        elif output_format == OutputFormat.json:
            from complexipy.utils.json import store_json

            store_json(
                output_path,
                files_complexities,
                show_details,
                max_complexity,
                suggest_refactors,
            )
        elif output_format == OutputFormat.gitlab:
            from complexipy.utils.gitlab import store_gitlab

            store_gitlab(
                output_path,
                files_complexities,
                max_complexity,
                suggest_refactors,
            )
        elif output_format == OutputFormat.sarif:
            from complexipy.utils.sarif import store_sarif

            store_sarif(
                output_path,
                files_complexities,
                max_complexity,
                suggest_refactors,
            )

        console.print(f"Results saved at {output_path}")


def resolve_output_formats(
    output_format_values: List[str],
) -> List[OutputFormat]:
    output_formats = []

    for value in output_format_values:
        try:
            normalized = OutputFormat(value)
        except ValueError as exc:
            valid_values = ", ".join(
                available.value for available in OutputFormat
            )
            raise typer.BadParameter(
                f"Invalid output format '{value}'. Expected one of: "
                f"{valid_values}."
            ) from exc

        if normalized not in output_formats:
            output_formats.append(normalized)

    return output_formats


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
    invocation_path: str = "",
) -> bool:
    file_entries, total_functions, all_pass = build_output_rows(
        files, failed_only, sort, max_complexity, snapshot_map
    )
    has_success = all_pass or ignore_complexity

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
            "No files were found with functions. No complexity was calculated."
        )
    else:
        output_file_entries(
            console,
            file_entries,
            previous_functions,
            max_complexity,
            suggest_refactors,
            invocation_path,
        )
    return has_success


def output_plain(
    console: Console,
    file_entries: List[FileEntry],
) -> None:
    for entry in file_entries:
        for function in entry.functions:
            path = normalize_path(function.path, function.file_name)
            console.print(f"{path} {function.name} {function.complexity}")


def truncate_top_n(
    file_entries: List[FileEntry],
    n: int,
) -> List[FileEntry]:
    all_functions: List[Tuple[str, FunctionRow]] = []
    for entry in file_entries:
        for function in entry.functions:
            all_functions.append((entry.path, function))

    all_functions.sort(key=lambda x: x[1].complexity, reverse=True)
    top_functions = all_functions[:n]

    result: List[FileEntry] = []
    for path, function in top_functions:
        if result and result[-1].path == path:
            result[-1].functions.append(function)
        else:
            result.append(FileEntry(path=path, functions=[function]))
    return result


def output_file_entries(
    console: Console,
    file_entries: List[FileEntry],
    previous_functions: Optional[Dict[Tuple[str, str, str], int]],
    max_complexity: int,
    suggest_refactors: bool = False,
    invocation_path: str = "",
) -> None:
    for i, entry in enumerate(file_entries):
        console.print(f"[bold]{entry.path}[/bold]")
        for function in entry.functions:
            status_text = format_status_text(function.passed)
            complexity_text = colorize_complexity(
                function.complexity, max_complexity
            )
            delta_text = output_delta_text(
                previous_functions, function, max_complexity
            )
            console.print(
                f"    {function.name} {complexity_text}{delta_text} {status_text}"
            )
            if suggest_refactors:
                output_refactor_plans(console, function, invocation_path)
        if i < len(file_entries) - 1:
            console.print()

    if not file_entries:
        return

    all_pass = all(
        fn.passed for entry in file_entries for fn in entry.functions
    )
    if all_pass:
        console.print()
        console.print(
            "[bold green]All functions are within the allowed complexity.[/bold green]"
        )


def output_refactor_plans(
    console: Console, function: FunctionRow, invocation_path: str = ""
) -> None:
    if not function.refactor_plans:
        return

    display_path = normalize_path(function.path, function.file_name)
    source_lines = _read_source_lines(
        invocation_path, function.path, function.file_name
    )

    console.print("\n      [bold]Refactor Suggestions:[/bold]")
    if len(function.refactor_plans) > 1:
        console.print(
            "      [dim]Each estimate is independent and assumes applying "
            "that suggestion alone -- they don't sum.[/dim]"
        )
    for index, plan in enumerate(function.refactor_plans, start=1):
        _output_single_plan(console, plan, index, display_path, source_lines)

    if function.additional_refactor_plans:
        suffix = "s" if function.additional_refactor_plans != 1 else ""
        console.print(
            f"\n      [dim]... and {function.additional_refactor_plans} "
            f"more suggestion{suffix}[/dim]"
        )


def _read_source_lines(
    invocation_path: str, path: str, file_name: str
) -> Optional[List[str]]:
    full_path = normalize_path(path, file_name)
    if invocation_path and not full_path.startswith("/"):
        full_path = f"{invocation_path.rstrip('/')}/{full_path}"
    try:
        with open(full_path, "r", encoding="utf-8") as source_file:
            return source_file.read().splitlines()
    except OSError:
        return None


def _output_single_plan(
    console: Console,
    plan: RefactorPlan,
    index: int,
    display_path: str,
    source_lines: Optional[List[str]],
) -> None:
    category_icon = _get_category_icon(plan.category)
    category_name = _get_category_name(plan.category)
    applicability_icon = _get_applicability_icon(plan.applicability)
    applicability_name = _get_applicability_name(plan.applicability)

    console.print(
        f"\n      [{index}] [bold cyan]{plan.rule_id}[/bold cyan] {escape(plan.title)}"
    )
    anchor = Text("          --> ", style="dim")
    anchor.append(f"{display_path}:{plan.line_start}:{plan.column_start}")
    console.print(anchor, soft_wrap=True)
    _output_caret_span(console, plan, source_lines)
    console.print(
        f"          Category: {category_icon} {category_name} "
        f"| Applicability: {applicability_icon} {applicability_name}"
    )
    reduction_label = (
        "Reduction" if plan.reduction_is_measured else "Estimated reduction"
    )
    qualifier = "" if plan.reduction_is_measured else "~"
    console.print(
        f"          Lines {plan.line_start}-{plan.line_end} "
        f"-> {reduction_label}: [green]-{qualifier}{plan.estimated_reduction}[/green] complexity "
        f"({plan.current_complexity} -> {plan.estimated_complexity_after})"
    )

    if plan.description:
        console.print(f"\n          [dim]{escape(plan.description)}[/dim]")

    if plan.explanation:
        console.print(f"\n          [bold]>[/bold] {escape(plan.explanation)}")

    suggestion = plan.suggestion
    if suggestion:
        _output_suggestion(console, plan, suggestion, source_lines)
    elif plan.help:
        _output_help(console, plan.help)

    _output_plan_references(console, plan.doc_url, plan.references)


def _get_category_icon(category: Union[str, RuleCategory]) -> str:
    category_str = str(category)
    if "Complexity" in category_str:
        return "[bold]\u25b2[/bold]"
    elif "Readability" in category_str:
        return "[bold]\u25c6[/bold]"
    return "[bold]\u2022[/bold]"


def _get_category_name(category: Union[str, RuleCategory]) -> str:
    category_str = str(category)
    if "Complexity" in category_str:
        return "Complexity"
    elif "Readability" in category_str:
        return "Readability"
    return category_str


def _get_applicability_icon(applicability: Union[str, Applicability]) -> str:
    applicability_str = str(applicability)
    if "MachineApplicable" in applicability_str:
        return "[green]*[/green]"
    elif "MaybeIncorrect" in applicability_str:
        return "[yellow]![/yellow]"
    elif "Informational" in applicability_str:
        return "[blue]i[/blue]"
    return "?"


def _get_applicability_name(applicability: Union[str, Applicability]) -> str:
    applicability_str = str(applicability)
    if "MachineApplicable" in applicability_str:
        return "Safe to apply"
    elif "MaybeIncorrect" in applicability_str:
        return "Needs review"
    elif "Informational" in applicability_str:
        return "Informational"
    return applicability_str


def _output_plan_references(
    console: Console, doc_url: str, references: list
) -> None:
    if doc_url or references:
        console.print("\n          [dim]References:[/dim]")
        if doc_url:
            console.print(
                Text(f"            {doc_url}", style="blue underline"),
                soft_wrap=True,
            )
        for ref in references:
            console.print(
                Text(f"            {ref}", style="blue underline"),
                soft_wrap=True,
            )


def _output_suggestion(
    console: Console,
    plan: RefactorPlan,
    suggestion: CodeSuggestion,
    source_lines: Optional[List[str]],
) -> None:
    applicability_icon = _get_applicability_icon(suggestion.applicability)
    applicability_name = _get_applicability_name(suggestion.applicability)

    console.print(
        f"\n          [bold]Suggestion:[/bold] {applicability_icon} {applicability_name}"
    )
    if suggestion.description:
        console.print(f"          [dim]{escape(suggestion.description)}[/dim]")

    if source_lines:
        original_start = plan.line_start
        original_end = min(plan.line_end, len(source_lines))
        original_code = "\n".join(
            source_lines[original_start - 1 : original_end]
        )
        if original_code:
            console.print("\n          [dim]Original:[/dim]")
            _output_code_snippet(console, original_code, original_start)

    if suggestion.replacement:
        console.print("\n          [dim]Replacement:[/dim]")
        _output_code_snippet(console, suggestion.replacement, plan.line_start)


def _output_caret_span(
    console: Console, plan: RefactorPlan, source_lines: Optional[List[str]]
) -> None:
    if not source_lines or plan.column_start <= 0:
        return

    line_index = plan.line_start - 1
    if line_index < 0 or line_index >= len(source_lines):
        return

    source_line = source_lines[line_index]
    column_index = plan.column_start - 1
    if column_index >= len(source_line):
        return

    caret_width = len(source_line[column_index:].rstrip())
    if caret_width <= 0:
        return

    gutter = f"{plan.line_start:>5} | "
    blank_gutter = " " * (len(gutter) - 2) + "| "
    console.print(Text(f"          {blank_gutter}"), soft_wrap=True)
    console.print(
        Text(f"          {gutter}") + Text(source_line), soft_wrap=True
    )
    console.print(
        Text(
            f"          {blank_gutter}{' ' * column_index}{'^' * caret_width}"
        ),
        soft_wrap=True,
    )
    console.print(Text(f"          {blank_gutter}"), soft_wrap=True)


def _output_help(console: Console, help_text: str) -> None:
    console.print("\n          [bold]Help:[/bold]")
    console.print(f"          {escape(help_text)}")


def _output_code_snippet(
    console: Console, code: str, start_line: int, indent: int = 12
) -> None:
    if not code:
        return

    syntax = Syntax(
        code,
        "python",
        line_numbers=True,
        start_line=start_line,
        dedent=False,
        word_wrap=False,
    )
    console.print(Padding(syntax, (0, 0, 0, indent)))


def format_status_text(passed: bool) -> str:
    if passed:
        return "[bold black on green] :white_heavy_check_mark: PASSED [/bold black on green]"
    return "[bold white on red] :cross_mark: FAILED [/bold white on red]"


def output_delta_text(
    previous_functions: Optional[Dict[Tuple[str, str, str], int]],
    function: FunctionRow,
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
) -> Tuple[List[FileEntry], int, bool]:
    file_entries: List[FileEntry] = []
    total_functions = 0
    all_pass = True

    for file in files:
        sorted_functions = sort_functions(file.functions, sort)
        displayable_functions: List[FunctionRow] = []

        for function in sorted_functions:
            total_functions += 1
            passed = _is_function_passing(
                function,
                file.path,
                file.file_name,
                max_complexity,
                snapshot_map,
            )

            if not passed:
                all_pass = False

            if failed_only and passed:
                continue

            displayable_functions.append(
                FunctionRow(
                    name=function.name,
                    complexity=function.complexity,
                    passed=passed,
                    path=file.path,
                    file_name=file.file_name,
                    refactor_plans=function.refactor_plans,
                    additional_refactor_plans=function.additional_refactor_plans,
                )
            )

        if displayable_functions:
            file_entries.append(
                FileEntry(
                    path=normalize_path(file.path, file.file_name),
                    functions=displayable_functions,
                )
            )

    return file_entries, total_functions, all_pass


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


def colorize_complexity(complexity: int, max_complexity: int) -> str:
    if complexity <= max_complexity:
        return f"[green]{complexity}[/green]"
    return f"[red]{complexity}[/red]"


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
    files: List[FileComplexity],
    max_complexity: int,
    snapshot_map: Optional[Dict[Tuple[str, str, str], int]] = None,
) -> bool:
    return all(
        all(
            _is_function_passing(
                function,
                file.path,
                file.file_name,
                max_complexity,
                snapshot_map,
            )
            for function in file.functions
        )
        for file in files
    )
