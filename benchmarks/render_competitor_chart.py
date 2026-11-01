from __future__ import annotations

import json
import pathlib
import sys
from html import escape
from typing import Any

THEMES = {
    "light": {
        "text": "#24292f",
        "muted": "#57606a",
        "bar": "#b6bdc6",
        "lead": "#7e57c2",
    },
    "dark": {
        "text": "#e6edf3",
        "muted": "#8b949e",
        "bar": "#484f58",
        "lead": "#b39ddb",
    },
}
WIDTH = 640
LABEL_WIDTH = 270
BAR_WIDTH = 290
ROW_HEIGHT = 44
BAR_HEIGHT = 24
FONT = "-apple-system, BlinkMacSystemFont, 'Segoe UI', Helvetica, Arial, sans-serif"


def render_row(
    index: int, name: str, mean: float, slowest: float, theme: dict[str, str]
) -> list[str]:
    lead = index == 0
    top = index * ROW_HEIGHT + 8
    middle = top + BAR_HEIGHT / 2
    width = max(2.0, BAR_WIDTH * mean / slowest)
    weight = "600" if lead else "400"
    color = theme["text"] if lead else theme["muted"]
    bar = theme["lead"] if lead else theme["bar"]
    text = f'dominant-baseline="central" fill="{color}" font-weight="{weight}"'
    return [
        f'<text x="{LABEL_WIDTH - 16}" y="{middle}" text-anchor="end" {text}>{escape(name)}</text>',
        f'<rect x="{LABEL_WIDTH}" y="{top}" width="{width:.1f}" height="{BAR_HEIGHT}" rx="3" fill="{bar}"/>',
        f'<text x="{LABEL_WIDTH + width + 10:.1f}" y="{middle}" {text}>{mean:.2f}s</text>',
    ]


def render_svg(rows: list[tuple[str, float]], theme: dict[str, str]) -> str:
    slowest = max(mean for _, mean in rows)
    height = ROW_HEIGHT * len(rows) + 8
    parts = [
        (
            f'<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{height}" '
            f'viewBox="0 0 {WIDTH} {height}" font-family="{FONT}" font-size="16">'
        ),
        "<title>Wall time to score cognitive complexity of the Django codebase</title>",
    ]
    for index, (name, mean) in enumerate(rows):
        parts.extend(render_row(index, name, mean, slowest, theme))
    parts.append("</svg>")
    return "\n".join(parts) + "\n"


def render_table(results: list[dict[str, Any]]) -> str:
    lead = results[0]["mean"]
    lines = ["| Tool | Mean | Relative to complexipy |", "| -- | -- | -- |"]
    for result in results:
        mean, stddev = result["mean"], result["stddev"]
        lines.append(
            f"| {result['command']} | {mean:.3f} s ± {stddev:.3f} s | {mean / lead:.1f}x |"
        )
    return "\n".join(lines) + "\n"


def main(
    hyperfine_json: pathlib.Path,
    table_out: pathlib.Path,
    svg_dirs: list[pathlib.Path],
) -> None:
    results = json.loads(hyperfine_json.read_text())["results"]
    rows = [(result["command"], result["mean"]) for result in results]
    table_out.write_text(render_table(results))
    for svg_dir in svg_dirs:
        for name, theme in THEMES.items():
            (svg_dir / f"competitors-{name}.svg").write_text(
                render_svg(rows, theme)
            )


if __name__ == "__main__":
    main(
        pathlib.Path(sys.argv[1]),
        pathlib.Path(sys.argv[2]),
        [pathlib.Path(p) for p in sys.argv[3:]],
    )
