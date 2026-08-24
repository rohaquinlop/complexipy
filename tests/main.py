from pathlib import Path
from typing import List, Tuple

import pytest

from complexipy import (
    code_complexity,
    collect_removable_ignored_locations,
    file_complexity,
)
from complexipy._complexipy import FileComplexity


def _analyze_paths(
    paths: List[Path], check_script: bool = False, no_ignore: bool = False
) -> Tuple[List[FileComplexity], List[str]]:
    """Analyze files or directories via the public file_complexity API.

    Directories are walked recursively for ``.py`` files in sorted order.
    Missing paths are reported in the failed list.
    """
    successful: List[FileComplexity] = []
    failed: List[str] = []

    for raw_path in paths:
        path = Path(raw_path).resolve()
        if not path.exists():
            failed.append(str(raw_path))
            continue
        if path.is_dir():
            files = sorted(p for p in path.rglob("*.py") if p.is_file())
        else:
            files = [path]

        for file in files:
            try:
                successful.append(
                    file_complexity(
                        str(file),
                        check_script=check_script,
                        no_ignore=no_ignore,
                    )
                )
            except (
                FileNotFoundError,
                PermissionError,
                UnicodeDecodeError,
                SyntaxError,
            ):
                failed.append(str(file))

    return successful, failed


class TestFiles:
    local_path = Path(__file__).resolve().parent

    def test_missing_path_is_reported(self):
        missing = self.local_path / "this_file_does_not_exist.py"

        files, failed = _analyze_paths([missing])

        assert files == []
        assert failed == [str(missing)]

    def test_path(self):
        path = self.local_path / "src"
        files, _ = _analyze_paths([path])
        total_complexity = sum([file.complexity for file in files])
        assert 64 == total_complexity

    def test(self):
        path = self.local_path / "src/test.py"
        files, _ = _analyze_paths([path])
        total_complexity = sum([file.complexity for file in files])
        assert 9 == total_complexity

    def test_break_continue(self):
        path = self.local_path / "src/test_break_continue.py"
        files, _ = _analyze_paths([path])
        total_complexity = sum([file.complexity for file in files])
        assert 3 == total_complexity

    def test_class(self):
        path = self.local_path / "src/test_class.py"
        files, _ = _analyze_paths([path])
        total_complexity = sum([file.complexity for file in files])
        assert 1 == total_complexity

    def test_decorator(self):
        path = self.local_path / "src/test_decorator.py"
        files, _ = _analyze_paths([path])
        total_complexity = sum([file.complexity for file in files])
        assert 1 == total_complexity

    def test_for(self):
        path = self.local_path / "src/test_for.py"
        files, _ = _analyze_paths([path])
        total_complexity = sum([file.complexity for file in files])
        assert 5 == total_complexity

    def test_for_assign(self):
        path = self.local_path / "src/test_for_assign.py"
        files, _ = _analyze_paths([path])
        total_complexity = sum([file.complexity for file in files])
        assert 2 == total_complexity

    def test_if(self):
        path = self.local_path / "src/test_if.py"
        files, _ = _analyze_paths([path])
        total_complexity = sum([file.complexity for file in files])
        assert 3 == total_complexity

    def test_main(self):
        path = self.local_path / "src/test_main.py"
        files, _ = _analyze_paths([path])
        total_complexity = sum([file.complexity for file in files])
        assert 0 == total_complexity

    def test_match(self):
        path = self.local_path / "src/test_match.py"
        files, _ = _analyze_paths([path])
        total_complexity = sum([file.complexity for file in files])
        assert 1 == total_complexity

    def test_multiple_func(self):
        path = self.local_path / "src/test_multiple_func.py"
        files, _ = _analyze_paths([path])
        total_complexity = sum([file.complexity for file in files])
        assert 0 == total_complexity

    def test_nested_func(self):
        path = self.local_path / "src/test_nested_func.py"
        files, _ = _analyze_paths([path])
        total_complexity = sum([file.complexity for file in files])
        assert 2 == total_complexity

    def test_recursive(self):
        path = self.local_path / "src/test_recursive.py"
        files, _ = _analyze_paths([path])
        total_complexity = sum([file.complexity for file in files])
        assert 1 == total_complexity

    def test_ternary_op(self):
        path = self.local_path / "src/test_ternary_op.py"
        files, _ = _analyze_paths([path])
        total_complexity = sum([file.complexity for file in files])
        assert 1 == total_complexity

    def test_try(self):
        path = self.local_path / "src/test_try.py"
        files, _ = _analyze_paths([path])
        total_complexity = sum([file.complexity for file in files])
        assert 3 == total_complexity

    def test_try_nested(self):
        path = self.local_path / "src/test_try_nested.py"
        files, _ = _analyze_paths([path])
        total_complexity = sum([file.complexity for file in files])
        assert 10 == total_complexity

    def test_file_complexity(self):
        path = self.local_path / "src/test_try_nested.py"
        result = file_complexity(str(path))
        assert 10 == result.complexity

    def test_code_complexity(self):
        snippet = """\
def hello_world(s: str) -> str:
    ans = ""

    def nested_func(s: str) -> str:
        if s == "complexipy":
            return "complexipy is awesome!"
        return f"I don't know what to say, hello {s}(?)"

    ans = nested_func(s)

    return ans
"""
        result = code_complexity(snippet)
        assert 2 == result.complexity

    def test_utf8_multi_byte_comment_no_panic(self):
        """Multi-byte UTF-8 characters in comments must not cause panics.

        The old byte-slicing implementation would panic when slicing at
        positions that fell in the middle of multi-byte UTF-8 characters.
        This test ensures the regex replacement handles all Unicode safely.
        """
        path = self.local_path / "src/test_utf8_comment.py"
        files, failed = _analyze_paths([path])
        # All marker-commented functions should be ignored; only
        # not_ignored_normal (no marker) should count.
        total_complexity = sum([file.complexity for file in files])
        assert 5 == total_complexity, (
            f"Expected 5 (only not_ignored_normal), got {total_complexity}"
        )
        assert failed == [], f"Unexpected failures: {failed}"

    def test_noqa_complexipy_ignore(self):
        path = self.local_path / "src/test_noqa_complex.py"
        files, _ = _analyze_paths([path])
        total_complexity = sum([file.complexity for file in files])
        # The only function has a noqa: complexipy, so it is ignored.
        assert 0 == total_complexity

    def test_noqa_complexipy_ignore_with_decorator(self):
        path = self.local_path / "src/test_noqa_decorator.py"
        files, _ = _analyze_paths([path])
        total_complexity = sum([file.complexity for file in files])
        assert 0 == total_complexity

    def test_complexipy_ignore(self):
        path = self.local_path / "src/test_complexipy_ignore.py"
        files, _ = _analyze_paths([path])
        total_complexity = sum([file.complexity for file in files])
        # The only complex function has a complexipy: ignore, so it is ignored.
        assert 0 == total_complexity

    def test_complexipy_ignore_with_decorator(self):
        path = self.local_path / "src/test_complexipy_ignore_decorator.py"
        files, _ = _analyze_paths([path])
        total_complexity = sum([file.complexity for file in files])
        assert 0 == total_complexity

    # ── no-ignore tests ──────────────────────────────────────────────

    def test_no_ignore_analyzes_ignored_function(self):
        """With no_ignore, functions with '# complexipy: ignore' are analyzed."""
        path = self.local_path / "src/test_complexipy_ignore.py"
        files, _ = _analyze_paths([path], no_ignore=True)
        total_complexity = sum([file.complexity for file in files])
        assert total_complexity > 0

    def test_no_ignore_analyzes_noqa_function(self):
        """With no_ignore, functions with '# noqa: complexipy' are analyzed."""
        path = self.local_path / "src/test_noqa_complex.py"
        files, _ = _analyze_paths([path], no_ignore=True)
        total_complexity = sum([file.complexity for file in files])
        assert total_complexity > 0

    def test_no_ignore_analyzes_decorated_ignored(self):
        """With no_ignore, decorated ignored functions are analyzed."""
        path = self.local_path / "src/test_complexipy_ignore_decorator.py"
        files, _ = _analyze_paths([path], no_ignore=True)
        total_complexity = sum([file.complexity for file in files])
        assert total_complexity > 0

    def test_no_ignore_analyzes_decorated_noqa(self):
        """With no_ignore, decorated noqa functions are analyzed."""
        path = self.local_path / "src/test_noqa_decorator.py"
        files, _ = _analyze_paths([path], no_ignore=True)
        total_complexity = sum([file.complexity for file in files])
        assert total_complexity > 0

    def test_no_ignore_false_is_default(self):
        """Without no_ignore, behavior is unchanged from before."""
        path = self.local_path / "src/test_complexipy_ignore.py"
        files_default, _ = _analyze_paths([path])
        files_explicit, _ = _analyze_paths([path], no_ignore=False)
        assert sum(f.complexity for f in files_default) == sum(
            f.complexity for f in files_explicit
        )

    def test_no_ignore_code_complexity_api(self):
        """Python API: code_complexity() with no_ignore analyzes ignored functions."""
        code = (
            "def ignored(a):  # complexipy: ignore\n"
            "    if a:\n"
            "        return a\n"
            "    return 0\n"
        )
        result_without = code_complexity(code)
        result_with = code_complexity(code, no_ignore=True)
        assert result_without.complexity == 0
        assert result_with.complexity > 0

    def test_no_ignore_file_complexity_api(self, tmp_path):
        """Python API: file_complexity() with no_ignore analyzes ignored functions."""
        source = tmp_path / "ignored.py"
        source.write_text(
            "def ignored(a):  # complexipy: ignore\n"
            "    if a:\n"
            "        return a\n"
            "    return 0\n",
            encoding="utf-8",
        )
        result_without = file_complexity(str(source))
        result_with = file_complexity(str(source), no_ignore=True)
        assert result_without.complexity == 0
        assert result_with.complexity > 0

    # ── collect_removable_ignored_locations API ──────────────────────

    def test_collect_removable_ignored_locations_api(self, tmp_path):
        """Python API: collect_removable_ignored_locations returns removable markers."""
        source = tmp_path / "app.py"
        source.write_text(
            "def simple(a):  # complexipy: ignore\n    return a\n",
            encoding="utf-8",
        )
        removable, failed = collect_removable_ignored_locations(
            [str(source)], [], 15
        )
        assert failed == []
        assert len(removable) == 1
        assert removable[0].path == "app.py"
        assert removable[0].line == 1
        assert removable[0].function == "simple"
        assert removable[0].complexity == 0
        assert removable[0].comment == "# complexipy: ignore"


class TestScriptComplexity:
    """Tests for module-level (script) complexity analysis."""

    local_path = Path(__file__).resolve().parent

    def test_script_simple_default(self):
        """Default behavior: module-level code not reported as <module>."""
        path = self.local_path / "src/test_script_simple.py"
        files, _ = _analyze_paths([path])
        assert len(files) == 1
        assert len(files[0].functions) == 0

    def test_script_simple_check_script(self):
        """Simple assignments have 0 complexity, <module> still emitted."""
        path = self.local_path / "src/test_script_simple.py"
        files, _ = _analyze_paths([path], check_script=True)
        assert len(files) == 1
        module_funcs = [f for f in files[0].functions if f.name == "<module>"]
        assert len(module_funcs) == 1
        assert module_funcs[0].complexity == 0

    def test_script_complex_default(self):
        """Default: complex script still reports 0 functions."""
        path = self.local_path / "src/test_script_complex.py"
        files, _ = _analyze_paths([path])
        assert len(files) == 1
        assert len(files[0].functions) == 0

    def test_script_complex_check_script(self):
        """With check_script: complex script reports <module>."""
        path = self.local_path / "src/test_script_complex.py"
        files, _ = _analyze_paths([path], check_script=True)
        assert len(files) == 1
        module_funcs = [f for f in files[0].functions if f.name == "<module>"]
        assert len(module_funcs) == 1
        assert module_funcs[0].complexity == 7

    def test_script_mixed_check_script(self):
        """Mixed file: both function and <module> reported."""
        path = self.local_path / "src/test_script_mixed.py"
        files, _ = _analyze_paths([path], check_script=True)
        assert len(files) == 1
        func_names = {f.name for f in files[0].functions}
        assert "simple_func" in func_names
        assert "<module>" in func_names
        simple = next(f for f in files[0].functions if f.name == "simple_func")
        module = next(f for f in files[0].functions if f.name == "<module>")
        assert simple.complexity == 1
        assert module.complexity == 3

    def test_code_complexity_check_script(self):
        """Python API: code_complexity with check_script."""
        code = "for i in range(10):\n    if i > 5:\n        print(i)\n"
        result = code_complexity(code, check_script=True)
        module_funcs = [f for f in result.functions if f.name == "<module>"]
        assert len(module_funcs) == 1
        assert module_funcs[0].complexity == 3

    def test_code_complexity_default_no_module(self):
        """Python API: code_complexity default does not emit <module>."""
        code = "for i in range(10):\n    if i > 5:\n        print(i)\n"
        result = code_complexity(code)
        module_funcs = [f for f in result.functions if f.name == "<module>"]
        assert len(module_funcs) == 0


class TestPaperConformance:
    """Conformance with the Cognitive Complexity white paper (G. Ann Campbell,
    SonarSource v1.7). Each expected value is the one prescribed by the paper.
    """

    def _c(self, code: str) -> int:
        return code_complexity(code).complexity

    def test_match_top_level_structural_increment(self):
        # A switch/match and all its cases combined incur a single structural
        # increment (B1).
        code = (
            "def f(x):\n"
            "    match x:\n"
            "        case 1:\n"
            "            return 'one'\n"
            "        case _:\n"
            "            return 'other'\n"
        )
        assert self._c(code) == 1

    def test_match_nested_gets_nesting_increment(self):
        # match inside a for: for(+1) + match(+1 structural +1 nesting) = 3.
        code = (
            "def f(xs, x):\n"
            "    for i in xs:\n"
            "        match x:\n"
            "            case 1:\n"
            "                pass\n"
        )
        assert self._c(code) == 3

    def test_try_body_is_not_nested(self):
        # try/except: an `if` directly in the try body is charged at nesting 0.
        code = "def f(x):\n    try:\n        if x:\n            pass\n    except Exception:\n        pass\n"
        # try body if (+1) + except handler (+1) = 2
        assert self._c(code) == 2

    def test_finally_is_not_nested(self):
        code = "def f(x):\n    try:\n        pass\n    finally:\n        if x:\n            pass\n"
        assert self._c(code) == 1

    def test_except_handler_gets_nesting_increment(self):
        # except nested in a for: for(+1) + handler(+1 +1 nesting) = 3.
        code = (
            "def f(xs):\n"
            "    for i in xs:\n"
            "        try:\n"
            "            pass\n"
            "        except Exception:\n"
            "            pass\n"
        )
        assert self._c(code) == 3

    def test_with_does_not_nest(self):
        code = (
            "def f(x, y):\n    with open(x):\n        if y:\n            pass\n"
        )
        assert self._c(code) == 1

    def test_direct_recursion_increments(self):
        code = "def fact(n):\n    return fact(n - 1)\n"
        assert self._c(code) == 1

    def test_recursion_in_nested_control_flow(self):
        # The recursive call lives inside an `if`, not the immediate body.
        code = "def fact(n):\n    if n:\n        return fact(n - 1)\n    return 1\n"
        assert self._c(code) == 2

    def test_nested_function_calling_outer_is_not_recursion(self):
        # A closure calling its enclosing function is a separate scope, not
        # direct self-recursion of the outer function.
        code = "def foo():\n    def bar():\n        foo()\n    return bar\n"
        assert self._c(code) == 0

    def test_lambda_raises_nesting(self):
        assert self._c("g = lambda x: (1 if x else 2)\n") == 2
        assert self._c("g = lambda x: x and x\n") == 1

    def test_comprehension_loop_and_filter(self):
        assert self._c("def f(xs):\n    return [x for x in xs if x > 0]\n") == 2
        assert (
            self._c("def f(xs):\n    return [y for x in xs for y in x if y]\n")
            == 3
        )
        assert self._c("def f(xs):\n    return any(x and x for x in xs)\n") == 2

    def test_nested_ternary_gets_nesting_increment(self):
        assert self._c("x = 1 if a else (2 if b else 3)\n") == 3

    def test_bare_expression_boolean_sequence(self):
        assert self._c("def f(a, b):\n    foo(a and b)\n") == 1

    def test_loop_else_is_not_nested(self):
        code = "def f(xs, x):\n    for i in xs:\n        pass\n    else:\n        if x:\n            pass\n"
        assert self._c(code) == 2
