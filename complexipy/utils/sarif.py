from __future__ import annotations

import json
from importlib.metadata import PackageNotFoundError
from importlib.metadata import version as pkg_version
from typing import List

from complexipy._complexipy import FileComplexity

_RULE_ID = "CC001"
_SCHEMA = (
    "https://raw.githubusercontent.com/oasis-tcs/sarif-spec"
    "/master/Schemata/sarif-schema-2.1.0.json"
)
_INFO_URI = "https://rohaquinlop.github.io/complexipy/"
_HELP_URI = "https://rohaquinlop.github.io/complexipy/understanding-scores/"


def _get_version() -> str:
    try:
        return pkg_version("complexipy")
    except PackageNotFoundError:
        return "unknown"


def store_sarif(
    output_path: str,
    files: List[FileComplexity],
    max_complexity: int,
) -> None:
    """Write complexity violations as a SARIF 2.1.0 file.

    Only functions whose complexity exceeds *max_complexity* are emitted as
    SARIF results.  The file is written to *output_path*.
    """
    results = []

    for file in files:
        for function in file.functions:
            if function.complexity <= max_complexity:
                continue

            results.append(
                {
                    "ruleId": _RULE_ID,
                    "level": "warning",
                    "message": {
                        "text": (
                            f"Function '{function.name}' has a cognitive complexity"
                            f" of {function.complexity}, which exceeds the maximum"
                            f" allowed complexity of {max_complexity}."
                        )
                    },
                    "locations": [
                        {
                            "physicalLocation": {
                                "artifactLocation": {
                                    "uri": file.path,
                                    "uriBaseId": "%SRCROOT%",
                                },
                                "region": {
                                    "startLine": int(function.line_start),
                                    "endLine": int(function.line_end),
                                },
                            },
                            "logicalLocations": [
                                {"name": function.name, "kind": "function"}
                            ],
                        }
                    ],
                }
            )

    sarif_doc = {
        "version": "2.1.0",
        "$schema": _SCHEMA,
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "complexipy",
                        "version": _get_version(),
                        "informationUri": _INFO_URI,
                        "rules": [
                            {
                                "id": _RULE_ID,
                                "name": "CognitiveComplexity",
                                "shortDescription": {
                                    "text": "Cognitive complexity exceeds threshold"
                                },
                                "helpUri": _HELP_URI,
                                "properties": {
                                    "tags": ["maintainability", "readability"]
                                },
                            }
                        ],
                    }
                },
                "results": results,
            }
        ],
    }

    with open(output_path, "w", encoding="utf-8") as f:
        json.dump(sarif_doc, f, indent=2)
