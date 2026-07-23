# Contributing to complexipy

Thanks for your interest in contributing! Here's how to get started.

## Reporting Bugs

Open a [bug report](https://github.com/rohaquinlop/complexipy/issues/new?template=bug_report.md) using the issue template. Include steps to reproduce, expected vs. actual behavior, and your environment details.

## Suggesting Features

Open a [feature request](https://github.com/rohaquinlop/complexipy/issues/new?template=feature_request.md) using the issue template. Describe the problem you're trying to solve and your proposed solution.

## Development Setup

1. Clone the repository:

    ```bash
    git clone https://github.com/rohaquinlop/complexipy.git
    cd complexipy
    ```

1. Install dependencies:

    ```bash
    uv sync
    ```

1. Build the Rust extension:

    ```bash
    uv run maturin develop
    ```

## Running Tests

```bash
uv run pytest tests/
```

## Linting & Formatting

```bash
uv run ruff check .
uv run ruff format .
```

## Pull Requests

- PR titles must follow [Conventional Commits](https://www.conventionalcommits.org/): `type(scope): description` (e.g., `fix(diff): resolve path for nested invocation`). This is enforced by CI.
- Keep code clean — no comments. Use descriptive variable and function names instead.
- Run tests and linter before submitting.
- Keep changes focused. One feature or fix per PR.
