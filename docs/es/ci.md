# Integración con CI/CD

## GitHub Actions

Usa la acción oficial:

```yaml
- uses: rohaquinlop/complexipy-action@v8
  with:
      paths: src tests
      max_complexity_allowed: 15
      output_json: true
```

O ejecuta directamente:

```yaml
- name: Check complexity
  run: |
      pip install complexipy
      complexipy . --max-complexity-allowed 15
```

## GitHub Code Scanning (SARIF)

Sube violaciones de complejidad como anotaciones inline en PRs usando SARIF:

```yaml
- name: Run complexipy
  run: complexipy . --output-format sarif --output complexipy-results.sarif --ignore-complexity

- name: Upload SARIF results
  uses: github/codeql-action/upload-sarif@v3
  with:
      sarif_file: complexipy-results.sarif
```

## Hook de Pre-commit

Agrega a `.pre-commit-config.yaml`:

```yaml
repos:
    - repo: https://github.com/rohaquinlop/complexipy-pre-commit
      rev: v9.0.0
      hooks:
          - id: complexipy
            args: [--max-complexity-allowed=15]
```

pre-commit pasa cada archivo modificado como argumento, y `exclude` no se aplica a los argumentos de archivo explícitos salvo que se use `--force-exclude`. Agrega `--force-exclude` a `args` para que se aplique el `exclude` configurado.

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

Usa `--ignore-complexity` cuando tu objetivo principal sea publicar el reporte aunque existan violaciones. GitLab seguirá mostrando los hallazgos en el widget del merge request y en la interfaz del pipeline.

Si además quieres que el job falle cuando se supere el umbral, separa el flujo en dos jobs:

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
