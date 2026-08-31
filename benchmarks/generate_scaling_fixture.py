import pathlib
import sys


def generate(base_functions: int, out_dir: pathlib.Path) -> None:
    header = ["import os", "", "def helper_a(x):", "    return x * 2", ""]
    body = []
    for i in range(base_functions):
        body.extend(
            [
                f"def function_{i}(a, b, c):",
                "    result = 0",
                "    if a > 0 and b < 10 or c == 3:",
                "        for j in range(a):",
                "            if j % 2 == 0 and b != 0:",
                "                result += j",
                "    elif a < 0:",
                "        while b > 0:",
                "            b -= 1",
                "    try:",
                "        result = result / (a - c)",
                "    except ZeroDivisionError:",
                "        result = 0",
                "    return result",
            ]
        )
    one = "\n".join(header + body) + "\n"
    (out_dir / "scaling_1x.py").write_text(one)
    (out_dir / "scaling_2x.py").write_text(one + one)
    (out_dir / "scaling_4x.py").write_text(one + one + one + one)


if __name__ == "__main__":
    base_functions = int(sys.argv[1])
    out_dir = pathlib.Path(sys.argv[2])
    out_dir.mkdir(parents=True, exist_ok=True)
    generate(base_functions, out_dir)
