import os
import platform
from importlib.metadata import (
    PackageNotFoundError,
)
from importlib.metadata import (
    version as pkg_version,
)
from typing import (
    List,
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
from complexipy.types import (
    ColorTypes,
    ExitReport,
    OutputFormat,
    Sort,
)
from complexipy.utils.config import (
    _comma_separated_list,
    resolve_config,
)
from complexipy.utils.diff import (
    handle_diff_output,
    has_regressions,
    resolve_diff_flags,
)
from complexipy.utils.ignored import handle_report_ignored
from complexipy.utils.output import (
    emit_deprecated_output_warnings,
    handle_console_settings,
    handle_display,
    handle_results_storage,
    print_invalid_paths,
    resolve_output_formats,
)
from complexipy.utils.snapshot import (
    evaluate_snapshot,
    handle_snapshot,
)
from complexipy.utils.toml import (
    get_complexipy_toml_config,
)

app = typer.Typer(name="complexipy")
INVOCATION_PATH = os.getcwd()
TOML_CONFIG = get_complexipy_toml_config(INVOCATION_PATH)


def _version_callback(value: bool):
    if value:
        version_console = Console(color_system="auto")
        try:
            version_console.print(pkg_version("complexipy"))
        except PackageNotFoundError:
            version_console.print("Unknown version")
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
        parser=_comma_separated_list,
        help=(
            "Paths to the directories or files to exclude. "
            "Comma-separated or repeated flags. "
            "Wrap glob patterns in quotes to prevent shell expansion: "
            "--exclude 'tests/**' or --exclude=tests/**."
        ),
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
    output: Optional[str] = typer.Option(
        None,
        "--output",
        help=(
            "Destination file or directory for machine-readable output. "
            "Use a directory when emitting multiple formats."
        ),
    ),
    output_format: Optional[List[str]] = typer.Option(
        None,
        "--output-format",
        parser=_comma_separated_list,
        help=(
            "Output format to emit. Comma-separated or repeated flags: "
            "csv, json, gitlab, sarif."
        ),
    ),
    output_csv: Optional[bool] = typer.Option(
        None,
        "--output-csv",
        "-c",
        help="Deprecated. Use `--output-format csv` instead. Output the results to a CSV file.",
    ),
    output_json: Optional[bool] = typer.Option(
        None,
        "--output-json",
        "-j",
        help="Deprecated. Use `--output-format json` instead. Output the results to a JSON file.",
    ),
    output_gitlab: Optional[bool] = typer.Option(
        None,
        "--output-gitlab",
        help="Deprecated. Use `--output-format gitlab` instead. Output the results as a GitLab Code Quality JSON report.",
    ),
    output_sarif: Optional[bool] = typer.Option(
        None,
        "--output-sarif",
        "-sr",
        help="Deprecated. Use `--output-format sarif` instead. Output the results to a SARIF 2.1.0 file for use with GitHub Code Scanning and other SARIF-aware tools.",
    ),
    diff: Optional[str] = typer.Option(
        None,
        "--diff",
        "-d",
        help=(
            "Show a complexity diff against a git reference and fail if any "
            "function regresses above --max-complexity-allowed. "
            "Use --diff-only for visual-only output without enforcement."
        ),
    ),
    diff_only: Optional[str] = typer.Option(
        None,
        "--diff-only",
        help=(
            "Show a complexity diff against a git reference without affecting "
            "the exit code. Visual-only, no enforcement."
        ),
    ),
    ratchet: Optional[bool] = typer.Option(
        None,
        "--ratchet",
        "-R",
        help="Deprecated. --diff now enforces by default.",
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
    suggest_refactors: Optional[bool] = typer.Option(
        None,
        "--suggest-refactors",
        help=(
            "Show deterministic refactor plans for displayed functions in rich output. "
            "Ignored when --plain is used."
        ),
    ),
    check_script: Optional[bool] = typer.Option(
        None,
        "--check-script",
        "-cs",
        help="Report cognitive complexity of module-level (script) code as '<module>'.",
    ),
    no_ignore: Optional[bool] = typer.Option(
        None,
        "--no-ignore",
        help="Disregard all '# complexipy: ignore' and '# noqa: complexipy' "
        "comments, analyzing every function "
        "(bare '# noqa' is not recognized).",
    ),
    report_ignored: Optional[bool] = typer.Option(
        None,
        "--report-ignored",
        help="List every file:line where a '# complexipy: ignore' or '# noqa: complexipy' comment exists.",
    ),
    version: bool = typer.Option(
        False,
        "--version",
        help="Show the complexipy version and exit.",
        callback=_version_callback,
        is_eager=True,
    ),
):
    cfg = resolve_config(
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
        diff,
        diff_only,
        ratchet,
        top,
        plain,
        suggest_refactors,
        exclude,
        check_script,
        no_ignore,
        report_ignored,
    )

    console = handle_console_settings(cfg.color, cfg.quiet, cfg.plain)

    cfg.diff, cfg.diff_only = resolve_diff_flags(
        console, cfg.diff, cfg.diff_only, cfg.ratchet
    )

    result: Tuple[List[FileComplexity], List[str]] = _complexipy.main(
        cfg.paths,
        cfg.quiet,
        cfg.exclude,
        cfg.check_script,
        cfg.no_ignore,
        INVOCATION_PATH,
    )
    files_complexities, failed_paths = result
    legacy_cli_output_flags = {
        OutputFormat.csv: output_csv,
        OutputFormat.json: output_json,
        OutputFormat.gitlab: output_gitlab,
        OutputFormat.sarif: output_sarif,
    }
    emit_deprecated_output_warnings(
        console,
        legacy_cli_output_flags,
        TOML_CONFIG,
    )
    output_formats = resolve_output_formats(
        cfg.output_format,
        legacy_cli_output_flags,
        TOML_CONFIG,
    )
    output_snapshot_path = f"{INVOCATION_PATH}/complexipy-snapshot.json"

    snap = evaluate_snapshot(
        cfg.snapshot_create,
        cfg.snapshot_ignore,
        output_snapshot_path,
        cfg.max_complexity_allowed,
        files_complexities,
    )

    handle_results_storage(
        console,
        output_formats,
        cfg.output,
        files_complexities,
        cfg.sort.value,
        not cfg.failed,
        cfg.max_complexity_allowed,
        INVOCATION_PATH,
    )

    display_ok = handle_display(
        console,
        files_complexities,
        cfg.paths,
        cfg.failed,
        cfg.sort,
        cfg.ignore_complexity,
        cfg.max_complexity_allowed,
        snap.active_snapshot_map,
        cfg.quiet,
        cfg.plain,
        INVOCATION_PATH,
        cfg.top,
        cfg.suggest_refactors,
    )

    handle_report_ignored(
        console,
        cfg.report_ignored,
        cfg.paths,
        cfg.exclude,
        output_formats,
        cfg.output,
        cfg.no_ignore,
        INVOCATION_PATH,
    )

    if cfg.quiet:
        snapshot_ok = snap.watermark_success if snap.should_run else True
    else:
        snapshot_ok = handle_snapshot(
            console,
            snap,
            output_snapshot_path,
        )
    _ = print_invalid_paths(console, cfg.quiet, failed_paths)
    paths_ok = not failed_paths
    diff_ref = cfg.diff or cfg.diff_only
    diff_entries = handle_diff_output(
        console, diff_ref, files_complexities, cfg.quiet, INVOCATION_PATH
    )
    diff_ok = True
    if cfg.diff and diff_entries is not None:
        diff_ok = not has_regressions(diff_entries, cfg.max_complexity_allowed)
    report = ExitReport(
        display_ok=display_ok,
        snapshot_ok=snapshot_ok,
        paths_ok=paths_ok,
        diff_ok=diff_ok,
        enforce_diff=bool(cfg.diff),
    )
    if not cfg.quiet and not cfg.plain:
        if platform.system() == "Windows":
            console.rule("Analysis completed!")
        else:
            console.rule(":tada: Analysis completed! :tada:")
    if not report.success:
        raise typer.Exit(code=1)


if __name__ == "__main__":
    app()
