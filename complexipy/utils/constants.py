from __future__ import annotations

from complexipy.types import OutputFormat

DEFAULT_OUTPUT_FILENAMES = {
    OutputFormat.csv: "complexipy-results.csv",
    OutputFormat.json: "complexipy-results.json",
    OutputFormat.gitlab: "complexipy-results.gitlab.json",
    OutputFormat.sarif: "complexipy-results.sarif",
}
