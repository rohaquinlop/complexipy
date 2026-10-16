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
- Keep code clean - no comments. Use descriptive variable and function names instead.
- Run tests and linter before submitting.
- Keep changes focused. One feature or fix per PR.

## Releasing the VS Code Extension

The extension in `vscode/complexipy` has its own version and its own tags (`vscode-vX.Y.Z`). Pushing such a tag runs `.github/workflows/vscode-release.yml`, which builds one VSIX and publishes it to the Visual Studio Marketplace and to OpenVSX. These tags do not start the complexipy release.

One-time setup:

1. Create an Eclipse account, link it on [open-vsx.org](https://open-vsx.org), and sign the publisher agreement.
1. Create a temporary OpenVSX access token and run `npx ovsx create-namespace rohaquinlop -p <token>` once from your machine. Delete the token afterwards.
1. Under [trusted publishers](https://open-vsx.org/user-settings/trusted-publishers), register repository `rohaquinlop/complexipy` with workflow `vscode-release.yml` and no environment. The workflow publishes to OpenVSX with a short-lived OIDC token, so no OpenVSX secret is stored.
1. Create an Azure DevOps personal access token with the "Marketplace: Manage" scope and save it as the `VSCE_PAT` repository secret.

Each release:

1. Bump `version` in `vscode/complexipy/package.json`, run `npm install` in that folder to update `package-lock.json`, and add an entry to `vscode/complexipy/CHANGELOG.md`.
1. Merge the PR.
1. Tag `main` and push the tag: `git tag vscode-vX.Y.Z && git push origin vscode-vX.Y.Z`. The workflow fails if the tag and `package.json` versions differ.

The extension runs the user's installed `complexipy lsp`. When an extension release needs a new server feature, push its tag only after the complexipy release with that feature is on PyPI.
