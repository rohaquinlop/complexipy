from complexipy.types import OutputFormat

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
