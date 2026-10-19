#!/bin/bash

# Manual fallback for the deploy-docs job in release.yml, which only runs after a release.
set -e

cd "$(dirname "$0")"

uv sync --group dev --no-install-project --frozen --inexact
uv run --no-sync mkdocs build
uv run --no-sync mkdocs build -f mkdocs.es.yml
uv run --no-sync ghp-import -n -p -f site

echo "Docs deployed to the gh-pages branch."
