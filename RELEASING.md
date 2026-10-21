# Releasing

## complexipy

Pushing a version tag such as `8.1.0` runs `.github/workflows/release.yml`. The `release` job publishes to PyPI only when the tag matches `[workspace.package] version` in `Cargo.toml`, and it runs in the `pypi` environment.

One-time setup:

1. Under Settings, Environments, give the `pypi` environment a required reviewer and a deployment tag rule for version tags.
1. On PyPI, set the trusted publisher of `complexipy` to workflow `release.yml` with environment `pypi`, so a token minted outside that environment cannot publish.

## VS Code Extension

The extension in `vscode/complexipy` has its own version and its own tags (`vscode-vX.Y.Z`). Pushing such a tag runs `.github/workflows/vscode-release.yml`, which builds one VSIX and publishes it to the Visual Studio Marketplace and to OpenVSX. These tags do not start the complexipy release.

One-time setup:

1. Create an Eclipse account, link it on [open-vsx.org](https://open-vsx.org), and sign the publisher agreement.
1. Create a temporary OpenVSX access token and run `npx ovsx create-namespace rohaquinlop -p <token>` once from your machine. Delete the token afterwards.
1. OpenVSX takes a trusted publisher only for an extension that already exists, so publish the first version by hand: in `vscode/complexipy`, run `npm ci` and `npx @vscode/vsce package --out complexipy.vsix`, then `npx ovsx publish complexipy.vsix -p <token>` with a temporary access token. Delete the token afterwards.
1. Under [trusted publishers](https://open-vsx.org/user-settings/trusted-publishers), register repository `rohaquinlop/complexipy` with workflow `vscode-release.yml` and environment `vscode-marketplace`. The workflow publishes to OpenVSX with a short-lived OIDC token, so no OpenVSX secret is stored.
1. Create an Azure DevOps personal access token with the "Marketplace: Manage" scope and save it as the `VSCE_PAT` secret of the `vscode-marketplace` environment. Give that environment a required reviewer.

Each release:

1. Bump `version` in `vscode/complexipy/package.json`, run `npm install` in that folder to update `package-lock.json`, and add an entry to `vscode/complexipy/CHANGELOG.md`.
1. Merge the PR.
1. Tag `main` and push the tag: `git tag vscode-vX.Y.Z && git push origin vscode-vX.Y.Z`. The workflow fails if the tag and `package.json` versions differ.

The extension runs the user's installed `complexipy lsp`. When an extension release needs a new server feature, push its tag only after the complexipy release with that feature is on PyPI.
