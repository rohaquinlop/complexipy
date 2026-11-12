"""Exact complexity totals for the tests/src fixtures, plus paper conformance."""

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
        assert 70 == total_complexity

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
        assert 1 == total_complexity

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
        assert 12 == total_complexity

    def test_nested_definitions(self):
        # Definitions under a module-level try/except and methods of a nested
        # class are scored as functions; the except handler stays module code.
        path = self.local_path / "src/test_nested_definitions.py"
        result = file_complexity(str(path))
        functions = {f.name: f.complexity for f in result.functions}
        assert functions == {"fast_path": 3, "Outer::Inner::method": 1}
        assert 5 == result.complexity

    def test_file_complexity(self):
        path = self.local_path / "src/test_try_nested.py"
        result = file_complexity(str(path))
        assert 12 == result.complexity

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
    SonarSource v1.7), plus the scores chosen for Python constructs the paper
    does not name, as `docs/understanding-scores.md` records them.
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

    @pytest.mark.parametrize(
        ("guard", "expected"),
        [("a", 1), ("a and b", 2), ("(b if a else c)", 2)],
        ids=["bare", "boolean-sequence", "ternary"],
    )
    def test_case_guard_adds_nothing_of_its_own(
        self, guard: str, expected: int
    ):
        code = f"def f(v, a, b, c):\n    match v:\n        case 1 if {guard}:\n            pass\n"
        assert self._c(code) == expected

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

    @pytest.mark.parametrize(
        ("code", "expected"),
        [
            pytest.param(
                "def a(n):\n    return b(n)\ndef b(n):\n    return a(n)\n",
                2,
                id="new-mutual",
            ),
            pytest.param(
                "def a():\n    return b()\ndef b():\n    return c()\ndef c():\n    return a()\n",
                3,
                id="new-three-cycle",
            ),
            pytest.param(
                "class C:\n    def m(self, n):\n        return self.m(n - 1)\n",
                1,
                id="new-self-method",
            ),
            pytest.param(
                "class C:\n    def a(self):\n        return self.b()\n    def b(self):\n        return self.a()\n",
                2,
                id="new-sibling-self-cycle",
            ),
            pytest.param(
                "def f():\n    class C:\n        def m(self):\n            return self.m()\n    return C\n",
                1,
                id="new-method-of-class-in-function",
            ),
            pytest.param(
                "def a():  # complexipy: ignore\n    return b()\ndef b():\n    return a()\n",
                1,
                id="ignored-def-keeps-cycle",
            ),
            pytest.param(
                "class C:\n    @classmethod\n    def m(cls, n):\n        return cls.m(n - 1)\n",
                1,
                id="new-cls-method",
            ),
            pytest.param(
                "def f():\n    def g(n):\n        return g(n - 1)\n    return g(3)\n",
                1,
                id="new-nested-function",
            ),
            pytest.param(
                "class C:\n    def m(self):\n        def inner():\n            return self.m()\n        return inner()\n",
                2,
                id="new-closure-self-method",
            ),
            pytest.param(
                "def foo():\n    def bar():\n        return foo()\n    return bar()\n",
                2,
                id="new-closure-and-outer",
            ),
            pytest.param(
                "def deco(a):\n    def inner(n):\n        return inner(n - 1)\n    return inner\n",
                1,
                id="new-decorator-factory-inner",
            ),
            pytest.param(
                "class C:\n    def get(self, x):\n        return get(x)\n",
                0,
                id="decrease-method-bare-name-not-self",
            ),
            pytest.param(
                "def f(n):\n    return list(map(lambda k: f(k - 1), range(n)))\n",
                1,
                id="lambda-call",
            ),
            pytest.param(
                "def g(n):\n    def inner(x=g(n - 1)):\n        return x\n    return inner\n",
                1,
                id="nested-default",
            ),
            pytest.param(
                "def h(n):\n    @h(n)\n    def inner():\n        pass\n    return inner\n",
                1,
                id="nested-decorator",
            ),
            pytest.param(
                "def h(n):\n    @h\n    def inner():\n        pass\n    return inner\n",
                0,
                id="guard-bare-decorator",
            ),
            pytest.param(
                "def f(n):\n    return lambda f: f()\n",
                0,
                id="guard-lambda-parameter",
            ),
            pytest.param(
                "class B:\n    def m(self):\n        return 1\nclass C(B):\n    def m(self):\n        return super().m()\n",
                0,
                id="guard-super",
            ),
            pytest.param(
                "class C:\n    @staticmethod\n    def m(x):\n        return x.m()\n",
                0,
                id="guard-staticmethod",
            ),
            pytest.param(
                "class C:\n    def m(self, other):\n        return other.m()\n",
                0,
                id="guard-other-receiver",
            ),
            pytest.param(
                "class C:\n    def m(self):\n        return C.m(self)\n",
                1,
                id="new-class-name",
            ),
            pytest.param(
                "class K:\n    @staticmethod\n    def s(n):\n        return K.s(n - 1)\n",
                1,
                id="new-class-name-staticmethod",
            ),
            pytest.param(
                "class K:\n    @classmethod\n    def s(cls, n):\n        return K.s(n - 1)\n",
                1,
                id="new-class-name-classmethod",
            ),
            pytest.param(
                "class K:\n    @staticmethod\n    def s(n):\n        return K.s(n - 1)\nK = object()\n",
                0,
                id="guard-class-name-rebound",
            ),
            pytest.param(
                "class K:\n    @staticmethod\n    def s(n):\n        return K.s(n - 1)\nclass Box[K]:\n    item: K\n",
                1,
                id="new-class-name-other-class-type-param",
            ),
            pytest.param(
                "def f(fs):\n    return [f(x) for f in fs]\n",
                0,
                id="guard-comprehension-variable",
            ),
            pytest.param(
                "def f(f):\n    return f()\n",
                0,
                id="guard-parameter",
            ),
            pytest.param(
                "def g(n):\n    g = n\n    return g()\n",
                0,
                id="guard-local-assignment",
            ),
            pytest.param(
                "def f(n):\n    (f := n)\n    return f()\n",
                0,
                id="guard-walrus",
            ),
            pytest.param(
                "def f(n):\n    for f in n:\n        pass\n    return f()\n",
                1,
                id="guard-for-target",
            ),
            pytest.param(
                "def f(n):\n    with n as f:\n        return f()\n",
                0,
                id="guard-with-target",
            ),
            pytest.param(
                "def f(n):\n    try:\n        pass\n    except E as f:\n        return f()\n",
                1,
                id="guard-except-name",
            ),
            pytest.param(
                "def f(n):\n    match n:\n        case [f]:\n            return f()\n",
                1,
                id="guard-match-capture",
            ),
            pytest.param(
                "def f(n):\n    del f\n    return f()\n",
                0,
                id="guard-del",
            ),
            pytest.param(
                "def f(n):\n    import f\n    return f()\n",
                0,
                id="guard-import",
            ),
            pytest.param(
                "def f(n):\n    type f = int\n    return f()\n",
                0,
                id="guard-type-alias",
            ),
            pytest.param(
                "def f(n):\n    f: int\n    return f()\n",
                0,
                id="guard-annotation-only-local",
            ),
            pytest.param(
                "def f(n):\n    return f()\nclass f:\n    pass\n",
                0,
                id="guard-class-after-def",
            ),
            pytest.param(
                "def f(n):\n    return f()\nf = wrap(f)\n",
                0,
                id="guard-rebound-after-def",
            ),
            pytest.param(
                "def outer(xs):\n    for x in xs:\n        f = 1\n        def f(n):\n            return f()\n    return f\n",
                1,
                id="guard-binding-in-same-loop",
            ),
            pytest.param(
                "f = 1\ndef f(n):\n    return f()\n",
                1,
                id="new-binding-before-def",
            ),
            pytest.param(
                "def f(n):\n    return f()\nf: int\n",
                1,
                id="new-annotation-only-after-def",
            ),
            pytest.param(
                "def outer(x):\n    if x:\n        f = 1\n        def f(n):\n            return f()\n    return f\n",
                2,
                id="new-binding-before-def-in-same-branch",
            ),
            pytest.param(
                "def outer(x):\n    if x:\n        def f(n):\n            return f()\n    else:\n        f = 1\n    return f\n",
                3,
                id="new-binding-in-sibling-branch",
            ),
            pytest.param(
                "def outer(xs):\n    for x in (f := xs):\n        def f(n):\n            return f()\n    return f\n",
                2,
                id="new-walrus-in-for-iterable",
            ),
        ],
    )
    def test_recursion_cycles(self, code: str, expected: int):
        assert self._c(code) == expected

    @pytest.mark.parametrize(
        ("code", "expected"),
        [
            (
                "def a_decorator(a, b):\n    def inner(func):\n        if a:\n            print(b)\n        func()\n    return inner\n",
                1,
            ),
            (
                "def not_a_decorator(a, b):\n    my_var = a * b\n    def inner(func):\n        if a:\n            print(b)\n        func()\n    return inner\n",
                2,
            ),
            (
                "def decorator_generator(a):\n    def generator(func):\n        def decorator(func):\n            if a:\n                print(a)\n            return func()\n        return decorator\n    return generator\n",
                1,
            ),
            (
                "def f(a):\n    def g():\n        if a:\n            pass\n    return g()\n",
                2,
            ),
            (
                'def deco(func):\n    """Doc."""\n    def inner():\n        if func:\n            pass\n    return inner\n',
                1,
            ),
            (
                "def deco(f):\n    def wrapper():\n        if f:\n            pass\n    return functools.wraps(f)(wrapper)\n",
                1,
            ),
            (
                "def deco(f):\n    def wrapper():\n        if f:\n            pass\n    return update_wrapper(wrapper=wrapper, wrapped=f)\n",
                1,
            ),
            (
                "def f(a):\n    def g(x):\n        if a:\n            pass\n    return g(g)\n",
                2,
            ),
            (
                "def deco(f, a):\n    def wrapper():\n        pass\n    return wraps(f if a else None)(wrapper)\n",
                1,
            ),
            (
                "def outer(a):\n    x = 1\n    def deco(f):\n        def wrapper():\n            pass\n        return wraps(f if a else None)(wrapper)\n    return deco\n",
                2,
            ),
            (
                "def deco(f):\n    def wrapper():\n        if f:\n            pass\n    return update(f, **wrapper)\n",
                2,
            ),
        ],
    )
    def test_python_decorator_exception(self, code: str, expected: int):
        assert self._c(code) == expected

    @pytest.mark.parametrize(
        ("code", "expected"),
        [
            (
                "def f(a, b):\n    if a:\n        def g():\n            if b:\n                pass\n",
                4,
            ),
            (
                "def f(a):\n    with a:\n        def g():\n            if a:\n                pass\n",
                2,
            ),
            (
                "def f(a):\n    try:\n        def g():\n            if a:\n                pass\n    except E:\n        pass\n",
                3,
            ),
            (
                "def f(xs):\n    for x in xs:\n        def g():\n            if x:\n                pass\n",
                4,
            ),
            (
                "def f(a):\n    def g():\n        if a:\n            pass\n    g()\n",
                2,
            ),
            (
                "def f(a):\n    class C:\n        def m(self):\n            if a:\n                pass\n",
                2,
            ),
            (
                "def f(a):\n    class C:\n        if a:\n            def m(self):\n                if a:\n                    pass\n",
                4,
            ),
            (
                "def f(a):\n    class C:\n        class D:\n            def m(self):\n                if a:\n                    pass\n",
                2,
            ),
            (
                "def f(a):\n    class C:\n        if a:\n            def m(self, x=(1 if a else 2)):\n                pass\n",
                3,
            ),
            (
                "def f(a):\n    x = a\n    def deco(g):\n        def w(x=(1 if a else 2)):\n            pass\n        return w\n    return deco\n",
                2,
            ),
            (
                "def f(a):\n    x = a\n    def deco(g):\n        def w():\n            if a:\n                pass\n        return w\n    return deco\n",
                2,
            ),
        ],
    )
    def test_nested_definitions_raise_nesting(self, code: str, expected: int):
        assert self._c(code) == expected

    @pytest.mark.parametrize(
        ("code", "module", "file_level"),
        [
            ("class C:\n    if a:\n        x = 1\n", 1, 0),
            (
                "class C:\n    def m(self):\n        if a:\n            pass\n",
                0,
                0,
            ),
            ("if b:\n    class C:\n        if a:\n            x = 1\n", 3, 1),
            (
                "class C:\n    class D:\n        if a:\n            x = 1\n",
                1,
                0,
            ),
            (
                "if b:\n    class C:\n        def m(self):\n            if a:\n                pass\n",
                1,
                1,
            ),
        ],
    )
    def test_module_class_bodies_count_only_with_check_script(
        self, code: str, module: int, file_level: int
    ):
        scripted = code_complexity(code, check_script=True).functions
        assert [f.complexity for f in scripted if f.name == "<module>"] == [
            module
        ]
        plain = code_complexity(code)
        functions = sum(f.complexity for f in plain.functions)
        assert plain.complexity - functions == file_level

    def test_lambda_raises_nesting(self):
        assert self._c("g = lambda x: (1 if x else 2)\n") == 2
        assert self._c("g = lambda x: x and x\n") == 1

    @pytest.mark.parametrize(
        ("expression", "expected"),
        [
            ("[x for x in xs]", 0),
            ("[x for x in xs if x]", 0),
            ("[y for x in xs for y in x]", 0),
            ("[[y for y in x] for x in xs]", 0),
            ("any(x > 0 for x in xs)", 0),
            ("[y for x in xs for y in x if y]", 0),
            ("[1 if x else 2 for x in xs]", 1),
            ("any(x and x for x in xs)", 1),
            ("[x for x in xs if x and y]", 1),
            ("{x: y for x, y in xs if x}", 0),
        ],
    )
    def test_comprehension_clauses_add_nothing(
        self, expression: str, expected: int
    ):
        assert self._c(f"def f(xs):\n    return {expression}\n") == expected

    def test_nested_ternary_gets_nesting_increment(self):
        assert self._c("x = 1 if a else (2 if b else 3)\n") == 3

    @pytest.mark.parametrize(
        ("expression", "expected"),
        [
            ("h(k=x if d else y)", 1),
            ("o.m(k=x if d else y)", 1),
            ("g(x if d else y).m()", 1),
            ("(x if d else y).m", 1),
            ("a + (x if d else y)", 1),
            ("t[x if d else y]", 1),
            ("t[a:(x if d else y)]", 1),
            ("f'{x if d else y}'", 1),
            ("h(*(x if d else y))", 1),
            ("(z := x if d else y)", 1),
            ("{x if d else y: 1}", 1),
            ("-(x if d else y)", 1),
            ("~(x if d else y)", 1),
            ("not (x if d else y)", 1),
            ("lambda v=(x if d else y): v", 1),
            ("await h(x if d else y)", 1),
            ("(a or b) + 1", 1),
            ("t[a or b]", 1),
            ("f'{a and b}'", 1),
            ("h(k=[v if d else 0 for v in d])", 1),
            ("a and (x if d else y)", 2),
            ("a or h(k=x if d else y)", 2),
            ("h(k=(lambda v: v if v else 0)(d))", 2),
        ],
    )
    def test_scored_expression_inside_any_expression_counts(
        self, expression: str, expected: int
    ):
        code = f"def f(d):\n    return {expression}\n"
        assert self._c(code) == expected

    def test_bare_expression_boolean_sequence(self):
        assert self._c("def f(a, b):\n    foo(a and b)\n") == 1

    def test_each_boolean_sequence_counts_once(self):
        def returning(expression: str) -> str:
            return f"def f(a, b, c, d):\n    return {expression}\n"

        assert self._c(returning("(a or b) and (c and d)")) == 2
        assert self._c(returning("(a or b) and c and (c or d)")) == 3
        assert self._c(returning("a and not (b or c) and (c or d)")) == 3
        assert self._c(returning("not (a and (b or c))")) == 2
        repeated = " and ".join(["(a or b) and (c and d)"] * 100)
        assert self._c(returning(repeated)) == 200

    @pytest.mark.parametrize(
        "code",
        [
            "def f(xs, x):\n    for i in xs:\n        pass\n    else:\n        if x:\n            pass\n",
            "def f(a, x):\n    while a:\n        pass\n    else:\n        if x:\n            pass\n",
            "def f(x):\n    try:\n        pass\n    except E:\n        pass\n    else:\n        if x:\n            pass\n",
        ],
    )
    def test_else_on_loops_and_try_is_nested(self, code: str):
        assert self._c(code) == 4

    @pytest.mark.parametrize(
        ("code", "expected"),
        [
            (
                "def f(xs):\n    for x in xs:\n        pass\n    else:\n        pass\n",
                2,
            ),
            (
                "def f(a):\n    while a:\n        pass\n    else:\n        pass\n",
                2,
            ),
            (
                "def f():\n    try:\n        pass\n    except E:\n        pass\n    else:\n        pass\n",
                2,
            ),
            (
                "def f():\n    try:\n        pass\n    except E:\n        pass\n    finally:\n        pass\n",
                1,
            ),
        ],
    )
    def test_else_on_loops_and_try_adds_one(self, code: str, expected: int):
        assert self._c(code) == expected

    @pytest.mark.parametrize(
        ("expression", "expected"),
        [
            ("a or b and c or d", 3),
            ("a and b or c", 2),
            ("a and b and c", 1),
            ("(a or b) or c", 1),
            ("a and not (b and c)", 2),
            ("a or b and c and d or e", 3),
            ("(a or (b and c)) and d", 2),
        ],
    )
    def test_mixed_boolean_operators_read_left_to_right(
        self, expression: str, expected: int
    ):
        assert (
            self._c(f"def f(a, b, c, d):\n    return {expression}\n")
            == expected
        )
