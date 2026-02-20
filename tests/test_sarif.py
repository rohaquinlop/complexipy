from __future__ import annotations

import json
import os
import tempfile

from complexipy import code_complexity
from complexipy.utils.sarif import store_sarif, _RULE_ID

# Minimal snippet with one complex and one simple function.
_SNIPPET = """\
def simple(x):
    return x + 1


def complex_func(data):
    if data:
        for item in data:
            if item:
                if item > 0:
                    for sub in item:
                        if sub:
                            pass
"""


class TestSarif:
    def _build_file_complexity(self, max_complexity: int = 5):
        result = code_complexity(_SNIPPET)
        # Wrap the CodeComplexity in a lightweight stand-in so we can pass it
        # to store_sarif, which expects List[FileComplexity].  We import the
        # real FileComplexity class from the Rust extension.
        from complexipy._complexipy import FileComplexity

        return [
            FileComplexity(
                path="src/example.py",
                file_name="example.py",
                complexity=result.complexity,
                functions=result.functions,
            )
        ], max_complexity

    def test_sarif_file_created(self):
        files, max_complexity = self._build_file_complexity(5)
        with tempfile.TemporaryDirectory() as tmp:
            out = os.path.join(tmp, "results.sarif")
            store_sarif(out, files, max_complexity)
            assert os.path.isfile(out)

    def test_sarif_valid_json(self):
        files, max_complexity = self._build_file_complexity(5)
        with tempfile.TemporaryDirectory() as tmp:
            out = os.path.join(tmp, "results.sarif")
            store_sarif(out, files, max_complexity)
            with open(out) as f:
                doc = json.load(f)
        assert doc["version"] == "2.1.0"
        assert "$schema" in doc
        assert len(doc["runs"]) == 1

    def test_sarif_contains_violations(self):
        files, max_complexity = self._build_file_complexity(5)
        with tempfile.TemporaryDirectory() as tmp:
            out = os.path.join(tmp, "results.sarif")
            store_sarif(out, files, max_complexity)
            with open(out) as f:
                doc = json.load(f)
        results = doc["runs"][0]["results"]
        # complex_func should exceed max_complexity=5
        assert len(results) >= 1
        assert all(r["ruleId"] == _RULE_ID for r in results)

    def test_sarif_no_violations_when_threshold_high(self):
        files, _ = self._build_file_complexity(5)
        with tempfile.TemporaryDirectory() as tmp:
            out = os.path.join(tmp, "results.sarif")
            store_sarif(out, files, max_complexity=9999)
            with open(out) as f:
                doc = json.load(f)
        assert doc["runs"][0]["results"] == []

    def test_sarif_result_has_location(self):
        files, max_complexity = self._build_file_complexity(5)
        with tempfile.TemporaryDirectory() as tmp:
            out = os.path.join(tmp, "results.sarif")
            store_sarif(out, files, max_complexity)
            with open(out) as f:
                doc = json.load(f)
        result = doc["runs"][0]["results"][0]
        loc = result["locations"][0]["physicalLocation"]
        assert "artifactLocation" in loc
        assert "region" in loc
        assert loc["region"]["startLine"] >= 1

    def test_sarif_rule_defined(self):
        files, max_complexity = self._build_file_complexity(5)
        with tempfile.TemporaryDirectory() as tmp:
            out = os.path.join(tmp, "results.sarif")
            store_sarif(out, files, max_complexity)
            with open(out) as f:
                doc = json.load(f)
        rules = doc["runs"][0]["tool"]["driver"]["rules"]
        assert len(rules) == 1
        assert rules[0]["id"] == _RULE_ID
