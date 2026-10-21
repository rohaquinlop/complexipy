# Downstream `update-version.yml` hardening

`release.yml : notify-downstream` sends a `repository_dispatch` to
`rohaquinlop/complexipy-action` and `rohaquinlop/complexipy-pre-commit`. This
repository now validates the tag and builds the payload with `jq --arg`, so the
version that reaches the dispatch is a plain `X.Y.Z`. The downstream workflows
still interpolate `${{ github.event.client_payload.version }}` straight into
`run:` blocks, so a value that ever carried shell metacharacters would run in
the downstream job, which holds `contents: write`.

These are draft patches for the two downstream repositories. They are not
applied by this PR, because those repositories are separate. Each one moves the
payload value into the environment through `env:`, so the shell receives it as
data, never as part of the script. Validate once, up front, and read `$VERSION`
from the environment everywhere after that.

## `rohaquinlop/complexipy-action` `.github/workflows/update-version.yml`

Add an `env:` block to the job and replace every
`VERSION="${{ github.event.client_payload.version }}"` with a read of
`$VERSION`:

```yaml
jobs:
  update:
    runs-on: ubuntu-latest
    if: github.event.client_payload.version
    env:
      VERSION: ${{ github.event.client_payload.version }}
    steps:
      - uses: actions/checkout@v4

      - name: Validate version
        run: |
          if ! printf '%s' "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
            echo "Invalid version: ${VERSION}" >&2
            exit 1
          fi

      - name: Update version
        run: |
          MAJOR=$(printf '%s' "$VERSION" | cut -d. -f1)
          sed -i "s/complexipy==.*/complexipy==${VERSION}/" Dockerfile
          sed -i "s/complexipy-action@v[0-9]*/complexipy-action@v${MAJOR}/" README.md

      - name: Commit and tag
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add Dockerfile README.md
          git diff --cached --quiet || git commit -m "chore: bump complexipy to ${VERSION}"
          git tag "v${VERSION}" 2>/dev/null || echo "tag v${VERSION} already exists"
          git push origin main
          git push origin "v${VERSION}" || echo "tag v${VERSION} already pushed"
          MAJOR=$(printf '%s' "$VERSION" | cut -d. -f1)
          git tag -f "v${MAJOR}"
          git push -f origin "v${MAJOR}"

      - name: Create GitHub Release
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          gh release view "v${VERSION}" >/dev/null 2>&1 || gh release create "v${VERSION}" --title "v${VERSION}" --generate-notes
```

The `env:` value is still the untrusted payload, but the shell now reads it from
the environment, so no value can break out of the string. The regex check stays
as a second line of defense; with the upstream `jq --arg` payload it can never
fail in practice.

## `rohaquinlop/complexipy-pre-commit` `.github/workflows/update-version.yml`

Same change:

```yaml
jobs:
  update:
    runs-on: ubuntu-latest
    if: github.event.client_payload.version
    env:
      VERSION: ${{ github.event.client_payload.version }}
    steps:
      - uses: actions/checkout@v4

      - name: Validate version
        run: |
          if ! printf '%s' "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+$'; then
            echo "Invalid version: ${VERSION}" >&2
            exit 1
          fi

      - name: Update version
        run: |
          sed -i "s/^version = \".*\"/version = \"${VERSION}\"/" pyproject.toml
          sed -i "s/complexipy==.*\"/complexipy==${VERSION}\"/" pyproject.toml
          sed -i "s/rev: v[0-9.]*/rev: v${VERSION}/" README.md

      - name: Commit and tag
        run: |
          git config user.name "github-actions[bot]"
          git config user.email "github-actions[bot]@users.noreply.github.com"
          git add pyproject.toml README.md
          git diff --cached --quiet || git commit -m "chore: bump complexipy to ${VERSION}"
          git tag "v${VERSION}" 2>/dev/null || echo "tag v${VERSION} already exists"
          git push origin main
          git push origin "v${VERSION}" || echo "tag v${VERSION} already pushed"

      - name: Create GitHub Release
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        run: |
          gh release view "v${VERSION}" >/dev/null 2>&1 || gh release create "v${VERSION}" --title "v${VERSION}" --generate-notes
```

## Why `env:` and not only the regex

A `run:` block is rendered by GitHub Actions before the shell sees it, so
`${{ ... }}` becomes literal script text. A value placed in `env:` is passed to
the process as an environment variable instead, so the shell reads it as one
string. The regex already rejects a non-`X.Y.Z` value, but it runs inside a
`run:` block too; moving the value to `env:` removes the injection point itself,
so the check no longer has to be perfect.
