# CI/CD Integration

## GitHub Actions

Use the official action:

```yaml
- uses: rohaquinlop/complexipy-action@v8
  with:
      paths: src tests
      max_complexity_allowed: 15
      output_json: true
```

Or run directly:

```yaml
- name: Check complexity
  run: |
      pip install complexipy
      complexipy . --max-complexity-allowed 15
```

## GitHub Code Scanning (SARIF)

Upload complexity violations as inline PR annotations using SARIF:

```yaml
- name: Run complexipy
  run: complexipy . --output-format sarif --output complexipy-results.sarif --ignore-complexity

- name: Upload SARIF results
  uses: github/codeql-action/upload-sarif@v3
  with:
      sarif_file: complexipy-results.sarif
```

## Pre-commit Hook

Add to `.pre-commit-config.yaml`:

```yaml
repos:
    - repo: https://github.com/rohaquinlop/complexipy-pre-commit
      rev: v8.3.0
      hooks:
          - id: complexipy
            args: [--max-complexity-allowed=15]
```

pre-commit passes each changed file as an argument, and `exclude` does not apply to explicit file arguments unless `--force-exclude` is set. Add `--force-exclude` to `args` for the configured `exclude` to apply.

## GitLab CI

```yaml
.complexipy_code_quality:
    image: python:3.11
    script:
        - pip install complexipy
        - complexipy . --output-format gitlab --output complexipy-code-quality.json --ignore-complexity --max-complexity-allowed 15
    artifacts:
        when: always
        reports:
            codequality: complexipy-code-quality.json

complexity:
    extends: .complexipy_code_quality
    rules:
        - if: $CI_PIPELINE_SOURCE == "merge_request_event"
        - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
```

Use `--ignore-complexity` when your main goal is to upload the report even if violations exist. GitLab will still display the findings in the merge request widget and pipeline UI.

If you also want the job to fail when the threshold is exceeded, split the workflow into two jobs:

```yaml
complexity_check:
    image: python:3.11
    script:
        - pip install complexipy
        - complexipy . --max-complexity-allowed 15

complexity_report:
    image: python:3.11
    script:
        - pip install complexipy
        - complexipy . --output-format gitlab --output complexipy-code-quality.json --ignore-complexity --max-complexity-allowed 15
    artifacts:
        when: always
        reports:
            codequality: complexipy-code-quality.json
```
