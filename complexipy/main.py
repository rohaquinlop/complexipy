import os
import platform
from importlib.metadata import (
    PackageNotFoundError,
)
from importlib.metadata import (
    version as pkg_version,
)
from typing import (
    Dict,
    List,  # It's important to use this to make it compatible with python 3.8, don't remove it
    Optional,
    Tuple,
)

import typer
from rich.console import (
    Console,
)

from complexipy import (
    _complexipy,
)
from complexipy._complexipy import FileComplexity

from .types import (
    ColorTypes,
    Metric,
    OutputFormat,
    Sort,
)
from .utils.cache import remember_previous_functions
from .utils.csv import store_csv
from .utils.diff import DiffEntry, compute_diff, format_diff, has_regressions
from .utils.gitlab import store_gitlab
from .utils.json import store_json
from .utils.output import (
    has_success_functions,
    output_summary,
    print_invalid_paths,
)
from .utils.sarif import store_sarif
from .utils.snapshot import (
    build_snapshot_map,
    handle_snapshot_file_creation,
    handle_snapshot_functions_load,
    handle_snapshot_watermark,
)
from .utils.toml import (
    get_argument_value,
    get_arguments_value,
    get_complexipy_toml_config,
)

app = typer.Typer(name="complexipy")
console = Console(color_system="auto")
INVOCATION_PATH = os.getcwd()
TOML_CONFIG = get_complexipy_toml_config(INVOCATION_PATH)
DEFAULT_OUTPUT_FILENAMES = {
    OutputFormat.csv: "complexipy-results.csv",
    OutputFormat.json: "complexipy-results.json",
    OutputFormat.gitlab: "complexipy-results.gitlab.json",
    OutputFormat.sarif: "complexipy-results.sarif",
}
LEGACY_OUTPUT_FLAGS = {
    OutputFormat.csv: "--output-csv",
    OutputFormat.json: "--output-json",
    OutputFormat.gitlab: "--output-gitlab",
    OutputFormat.sarif: "--output-sarif",
}
LEGACY_OUTPUT_CONFIG_KEYS = {
    OutputFormat.csv: "output-csv",
    OutputFormat.json: "output-json",
    OutputFormat.gitlab: "output-gitlab",
    OutputFormat.sarif: "output-sarif",
}


def _version_callback(value: bool):
    if value:
        try:
            console.print(pkg_version("complexipy"))
        except PackageNotFoundError:
            console.print("Unknown version")
        raise typer.Exit()


@app.command()
def main(
    paths: Optional[List[str]] = typer.Argument(
        None,
        help="Paths to the directories or files to analyze, it can be a local paths or a git repository URL.",
    ),
    exclude: Optional[List[str]] = typer.Option(
        None,
        "--exclude",
        "-e",
        help="Paths to the directories or files to exclude.",
    ),
    max_complexity_allowed: Optional[int] = typer.Option(
        None,
        "--max-complexity-allowed",
        "-mx",
        help="Max complexity allowed per function.",
    ),
    snapshot_create: Optional[bool] = typer.Option(
        None,
        "--snapshot-create",
        "-spc",
        help="Creates a snapshot of the current project state.",
    ),
    snapshot_ignore: Optional[bool] = typer.Option(
        None,
        "--snapshot-ignore",
        "-spi",
        help="Skip comparing against the existing snapshot file.",
    ),
    quiet: Optional[bool] = typer.Option(
        None,
        "--quiet",
        "-q",
        help="Suppress the output to the console.",
    ),
    ignore_complexity: Optional[bool] = typer.Option(
        None,
        "--ignore-complexity",
        "-i",
        help="Ignore the complexity and show all functions.",
    ),
    failed: Optional[bool] = typer.Option(
        None,
        "--failed",
        "-f",
        help="Show only functions that exceed the max complexity threshold.",
    ),
    color: Optional[ColorTypes] = typer.Option(
        None,
        "--color",
        "-C",
        help="Whether the output should be in color: either 'auto', 'yes' or 'no'. Default is 'auto'.",
    ),
    sort: Optional[Sort] = typer.Option(
        None,
        "--sort",
        "-s",
        help="Sort the output by complexity, it can be 'asc', 'desc' or 'name'. Default is 'asc'.",
    ),
    output_format: Optional[List[str]] = typer.Option(
        None,
        "--output-format",
        help=(
            "Output format to emit. Repeat the flag to request multiple formats: "
            "csv, json, gitlab, sarif."
        ),
    ),
    output: Optional[str] = typer.Option(
        None,
        "--output",
        help=(
            "Destination file or directory for machine-readable output. "
            "Use a directory when emitting multiple formats."
        ),
    ),
    output_csv: Optional[bool] = typer.Option(
        None,
        "--output-csv",
        "-c",
        help="Output the results to a CSV file.",
    ),
    output_json: Optional[bool] = typer.Option(
        None,
        "--output-json",
        "-j",
        help="Output the results to a JSON file.",
    ),
    output_gitlab: Optional[bool] = typer.Option(
        None,
        "--output-gitlab",
        help="Output the results as a GitLab Code Quality JSON report.",
    ),
    diff: Optional[str] = typer.Option(
        None,
        "--diff",
        "-d",
        help=(
            "Show a complexity diff against a git reference (e.g. HEAD~1, main, "
            "a commit SHA).  Requires git to be available and the paths to be "
            "inside a git repository."
        ),
    ),
    ratchet: Optional[bool] = typer.Option(
        None,
        "--ratchet",
        "-R",
        help=(
            "Only fail when a change breaches --max-complexity-allowed. "
            "Requires --diff. Exit 1 if a new function exceeds the threshold "
            "or an existing function regresses above it (already-over "
            "functions that get worse also fail). Regressions that stay "
            "within the threshold are allowed."
        ),
    ),
    output_sarif: Optional[bool] = typer.Option(
        None,
        "--output-sarif",
        "-sr",
        help="Output the results to a SARIF 2.1.0 file for use with GitHub Code Scanning and other SARIF-aware tools.",
    ),
    top: Optional[int] = typer.Option(
        None,
        "--top",
        "-t",
        help="Show only the N most complex functions, sorted by complexity descending.",
    ),
    plain: Optional[bool] = typer.Option(
        None,
        "--plain",
        help=(
            "Use plain text output instead of rich formatting. "
            "Each line is: <path> <function> <complexity> (space-separated). "
            "Useful for AI agents and scripting. CLI-only flag (not supported in TOML)."
        ),
    ),
    check_script: Optional[bool] = typer.Option(
        None,
        "--check-script",
        "-cs",
        help="Report cognitive complexity of module-level (script) code as '<module>'.",
    ),
    metric: Optional[Metric] = typer.Option(
        None,
        "--metric",
        help=(
            "Complexity metric to gate on: 'cognitive' (default) or "
            "'cyclomatic'. When 'cyclomatic', the selected metric is used "
            "for threshold checks and emitted alongside cognitive in "
            "machine-readable outputs."
        ),
    ),
    version: bool = typer.Option(
        False,
        "--version",
        help="Show the complexipy version and exit.",
        callback=_version_callback,
        is_eager=True,
    ),
):
    global console

    legacy_cli_output_flags = {
        OutputFormat.csv: output_csv,
        OutputFormat.json: output_json,
        OutputFormat.gitlab: output_gitlab,
        OutputFormat.sarif: output_sarif,
    }

    (
        paths,
        max_complexity_allowed,
        snapshot_create,
        snapshot_ignore,
        quiet,
        ignore_complexity,
        failed,
        color,
        sort,
        output_format,
        output,
        exclude,
        check_script,
    ) = get_arguments_value(
        TOML_CONFIG,
        paths,
        max_complexity_allowed,
        snapshot_create,
        snapshot_ignore,
        quiet,
        ignore_complexity,
        failed,
        color,
        sort,
        output_format,
        output,
        output_csv,
        output_json,
        output_gitlab,
        output_sarif,
        exclude,
        check_script,
    )

    ratchet = bool(get_argument_value(TOML_CONFIG, "ratchet", ratchet, False))
    validate_ratchet(ratchet, diff)

    metric = get_argument_value(TOML_CONFIG, "metric", metric, Metric.cognitive)
    if isinstance(metric, str):
        try:
            metric = Metric(metric)
        except ValueError as exc:
            valid_values = ", ".join(m.value for m in Metric)
            raise typer.BadParameter(
                f"Invalid metric '{metric}'. Expected one of: {valid_values}."
            ) from exc

    # --plain is intentionally CLI-only (not resolved via TOML) because it is
    # a session-level display preference, not a project-wide default.
    if plain is None:
        plain = False

    if plain and quiet:
        raise typer.BadParameter("--plain and --quiet cannot be used together.")

    if top is not None and top < 1:
        raise typer.BadParameter("--top must be a positive integer.")

    handle_console_settings(color, quiet, plain)

    compute_cyclomatic = metric == Metric.cyclomatic
    result: Tuple[List[FileComplexity], List[str]] = _complexipy.main(
        paths, quiet, exclude, check_script, compute_cyclomatic
    )
    files_complexities, failed_paths = result
    emit_deprecated_output_warnings(
        legacy_cli_output_flags,
        TOML_CONFIG,
    )
    output_formats = resolve_output_formats(
        output_format,
        legacy_cli_output_flags,
        TOML_CONFIG,
    )
    output_snapshot_path = f"{INVOCATION_PATH}/complexipy-snapshot.json"

    handle_snapshot_file_creation(
        snapshot_create,
        output_snapshot_path,
        max_complexity_allowed,
        files_complexities,
    )

    snapshot_file_exists = os.path.exists(output_snapshot_path)
    snapshot_files = handle_snapshot_functions_load(output_snapshot_path)
    should_run_snapshot_watermark = snapshot_file_exists and not snapshot_ignore
    active_snapshot_map = (
        build_snapshot_map(snapshot_files)
        if should_run_snapshot_watermark
        else None
    )
    watermark_success, watermark_messages = handle_snapshot_watermark(
        should_run_snapshot_watermark,
        snapshot_file_exists,
        output_snapshot_path,
        files_complexities,
        snapshot_files,
        max_complexity_allowed,
    )

    handle_results_storage(
        output_formats,
        output,
        files_complexities,
        sort.value,
        not failed,
        max_complexity_allowed,
    )

    has_success = handle_display(
        console,
        files_complexities,
        paths,
        failed,
        sort,
        ignore_complexity,
        max_complexity_allowed,
        active_snapshot_map,
        quiet,
        plain,
        top,
        metric,
    )

    snapshot_result = handle_snapshot(
        should_run_snapshot_watermark,
        quiet,
        watermark_messages,
        output_snapshot_path,
        watermark_success,
    )
    if should_run_snapshot_watermark:
        # When the snapshot watermark is active, it is the authoritative
        # success check. Functions exceeding the threshold that are already
        # in the snapshot (and haven't regressed) should not cause a failure.
        has_success = snapshot_result
    else:
        has_success = has_success and snapshot_result

    valid_paths = print_invalid_paths(console, quiet, failed_paths)
    diff_entries = handle_diff_output(diff, files_complexities, quiet)
    has_success = resolve_final_success(
        has_success,
        valid_paths,
        snapshot_result,
        ratchet,
        diff_entries,
        max_complexity_allowed,
    )

    if not has_success:
        raise typer.Exit(code=1)


def handle_console_settings(
    color: ColorTypes, quiet: bool, plain: bool = False
):
    global console
    if plain:
        console = Console(color_system=None, highlight=False)
        return
    if color == ColorTypes.no:
        console = Console(color_system=None)
    elif color == ColorTypes.yes:
        console = Console(color_system="standard")

    if not quiet:
        if platform.system() == "Windows":
            console.rule("complexipy")
        else:
            console.rule(":octopus: complexipy")


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
    top: Optional[int] = None,
    metric: Metric = Metric.cognitive,
) -> bool:
    if files_complexities:
        previous_functions = remember_previous_functions(
            INVOCATION_PATH, paths, files_complexities
        )
    else:
        previous_functions = None

    if quiet:
        return has_success_functions(
            files_complexities, max_complexity_allowed, metric
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
        metric,
    )
    if not plain:
        if platform.system() == "Windows":
            console.rule("Analysis completed!")
        else:
            console.rule(":tada: Analysis completed! :tada:")
    return has_success


def handle_results_storage(
    output_formats: List[OutputFormat],
    output: Optional[str],
    files_complexities: List[FileComplexity],
    sort: str,
    show_details: bool,
    max_complexity: int,
) -> None:
    global console

    output_paths = resolve_output_paths(output_formats, output)

    for output_format in output_formats:
        output_path = output_paths[output_format]

        if output_format == OutputFormat.csv:
            store_csv(
                output_path,
                files_complexities,
                sort,
                show_details,
                max_complexity,
            )
        elif output_format == OutputFormat.json:
            store_json(
                output_path,
                files_complexities,
                show_details,
                max_complexity,
            )
        elif output_format == OutputFormat.gitlab:
            store_gitlab(
                output_path,
                files_complexities,
                max_complexity,
            )
        elif output_format == OutputFormat.sarif:
            store_sarif(
                output_path,
                files_complexities,
                max_complexity,
            )

        console.print(f"Results saved at {output_path}")


def emit_deprecated_output_warnings(
    legacy_cli_output_flags: Dict[OutputFormat, Optional[bool]],
    toml_config,
) -> None:
    global console

    for output_format, flag_name in LEGACY_OUTPUT_FLAGS.items():
        if legacy_cli_output_flags[output_format]:
            console.print(
                f"[yellow]Deprecated:[/yellow] {flag_name} will be removed "
                f"in a future release. Use --output-format "
                f"{output_format.value} instead."
            )

    if toml_config is None:
        return

    for output_format, config_key in LEGACY_OUTPUT_CONFIG_KEYS.items():
        if bool(toml_config.get(config_key, False)):
            console.print(
                f"[yellow]Deprecated:[/yellow] `{config_key}` in TOML will "
                f"be removed in a future release. Use `output-format = "
                f'["{output_format.value}"]` instead.'
            )


def resolve_output_formats(
    output_format_values: List[str],
    legacy_cli_output_flags: Dict[OutputFormat, Optional[bool]],
    toml_config,
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

    for output_format in OutputFormat:
        if legacy_cli_output_flags[output_format]:
            output_formats.append(output_format)
            continue

        config_key = LEGACY_OUTPUT_CONFIG_KEYS[output_format]
        if toml_config is not None and bool(toml_config.get(config_key, False)):
            output_formats.append(output_format)

    return list(dict.fromkeys(output_formats))


def resolve_output_paths(
    output_formats: List[OutputFormat],
    output: Optional[str],
) -> Dict[OutputFormat, str]:
    if not output_formats:
        return {}

    if output is None:
        return build_output_paths(INVOCATION_PATH, output_formats)

    if output == "-":
        raise typer.BadParameter(
            "Writing machine-readable output to stdout is not supported."
        )

    destination = os.path.abspath(output)
    is_directory_hint = is_directory_output_hint(output)

    if len(output_formats) > 1:
        ensure_directory_destination(destination, is_directory_hint)
        return build_output_paths(destination, output_formats)

    output_format = output_formats[0]
    if os.path.isdir(destination) or is_directory_hint:
        os.makedirs(destination, exist_ok=True)
        return build_output_paths(destination, [output_format])

    parent_dir = os.path.dirname(destination)
    if parent_dir:
        os.makedirs(parent_dir, exist_ok=True)

    return {output_format: destination}


def build_output_paths(
    destination: str, output_formats: List[OutputFormat]
) -> Dict[OutputFormat, str]:
    return {
        output_format: os.path.join(
            destination,
            DEFAULT_OUTPUT_FILENAMES[output_format],
        )
        for output_format in output_formats
    }


def is_directory_output_hint(output: str) -> bool:
    return output.endswith(os.sep) or (
        os.altsep is not None and output.endswith(os.altsep)
    )


def ensure_directory_destination(
    destination: str, is_directory_hint: bool
) -> None:
    if os.path.exists(destination):
        if not os.path.isdir(destination):
            raise typer.BadParameter(
                "When multiple output formats are selected, --output "
                "must point to a directory."
            )
    elif not is_directory_hint:
        raise typer.BadParameter(
            "When multiple output formats are selected, --output must "
            "point to a directory or end with a path separator."
        )

    os.makedirs(destination, exist_ok=True)


def handle_snapshot(
    should_run_snapshot_watermark: bool,
    quiet: bool,
    watermark_messages: List[str],
    output_snapshot_path: str,
    watermark_success: bool,
) -> bool:
    global console

    if should_run_snapshot_watermark:
        if not quiet:
            if watermark_messages:
                for message in watermark_messages:
                    console.print(
                        f"[bold red]Snapshot watermark[/bold red]: {message}"
                    )
            else:
                console.print(
                    f"Snapshot watermark passed. Baseline stored at {output_snapshot_path}"
                )
        return watermark_success
    return True


def handle_diff_output(
    diff: Optional[str],
    files_complexities: List[FileComplexity],
    quiet: bool,
) -> Optional[List[DiffEntry]]:
    global console
    if diff and files_complexities:
        entries = compute_diff(files_complexities, diff, INVOCATION_PATH)
        if not quiet:
            console.print(format_diff(entries, diff))
        return entries
    return None


def validate_ratchet(ratchet: bool, diff: Optional[str]) -> None:
    if ratchet and not diff:
        console.print("[bold red]Error:[/bold red] --ratchet requires --diff")
        raise typer.Exit(code=2)


def resolve_final_success(
    has_success: bool,
    valid_paths: bool,
    snapshot_result: bool,
    ratchet: bool,
    diff_entries: Optional[List[DiffEntry]],
    max_complexity_allowed: int,
) -> bool:
    if ratchet and diff_entries is not None:
        ratchet_ok = not has_regressions(diff_entries, max_complexity_allowed)
        return ratchet_ok and valid_paths and snapshot_result
    return has_success and valid_paths


if __name__ == "__main__":
    app()
