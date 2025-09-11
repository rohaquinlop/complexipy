from complexipy import _complexipy
import unittest
from pathlib import Path

from complexipy import (
    file_complexity,
    code_complexity,
)


class TestFiles(unittest.TestCase):
    local_path = Path(__file__).resolve().parent

    def test_path(self):
        path = self.local_path / "src"
        files, _ = _complexipy.main([path.resolve().as_posix()], quiet=False)
        total_complexity = sum([file.complexity for file in files])
        self.assertEqual(41, total_complexity)

    def test(self):
        path = self.local_path / "src/test.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], quiet=False)
        total_complexity = sum([file.complexity for file in files])
        self.assertEqual(9, total_complexity)

    def test_break_continue(self):
        path = self.local_path / "src/test_break_continue.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], quiet=False)
        total_complexity = sum([file.complexity for file in files])
        self.assertEqual(3, total_complexity)

    def test_class(self):
        path = self.local_path / "src/test_class.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], quiet=False)
        total_complexity = sum([file.complexity for file in files])
        self.assertEqual(1, total_complexity)

    def test_decorator(self):
        path = self.local_path / "src/test_decorator.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], quiet=False)
        total_complexity = sum([file.complexity for file in files])
        self.assertEqual(1, total_complexity)

    def test_for(self):
        path = self.local_path / "src/test_for.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], quiet=False)
        total_complexity = sum([file.complexity for file in files])
        self.assertEqual(5, total_complexity)

    def test_for_assign(self):
        path = self.local_path / "src/test_for_assign.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], quiet=False)
        total_complexity = sum([file.complexity for file in files])
        self.assertEqual(1, total_complexity)

    def test_if(self):
        path = self.local_path / "src/test_if.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], quiet=False)
        total_complexity = sum([file.complexity for file in files])
        self.assertEqual(3, total_complexity)

    def test_main(self):
        path = self.local_path / "src/test_main.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], quiet=False)
        total_complexity = sum([file.complexity for file in files])
        self.assertEqual(0, total_complexity)

    def test_match(self):
        path = self.local_path / "src/test_match.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], quiet=False)
        total_complexity = sum([file.complexity for file in files])
        self.assertEqual(0, total_complexity)

    def test_multiple_func(self):
        path = self.local_path / "src/test_multiple_func.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], quiet=False)
        total_complexity = sum([file.complexity for file in files])
        self.assertEqual(0, total_complexity)

    def test_nested_func(self):
        path = self.local_path / "src/test_nested_func.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], quiet=False)
        total_complexity = sum([file.complexity for file in files])
        self.assertEqual(2, total_complexity)

    def test_recursive(self):
        path = self.local_path / "src/test_recursive.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], quiet=False)
        total_complexity = sum([file.complexity for file in files])
        self.assertEqual(0, total_complexity)

    def test_ternary_op(self):
        path = self.local_path / "src/test_ternary_op.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], quiet=False)
        total_complexity = sum([file.complexity for file in files])
        self.assertEqual(1, total_complexity)

    def test_try(self):
        path = self.local_path / "src/test_try.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], quiet=False)
        total_complexity = sum([file.complexity for file in files])
        self.assertEqual(3, total_complexity)

    def test_try_nested(self):
        path = self.local_path / "src/test_try_nested.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], quiet=False)
        total_complexity = sum([file.complexity for file in files])
        self.assertEqual(12, total_complexity)

    def test_file_complexity(self):
        path = self.local_path / "src/test_try_nested.py"
        result = file_complexity(str(path))
        self.assertEqual(12, result.complexity)

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
        self.assertEqual(2, result.complexity)

    def test_noqa_complexipy_ignore(self):
        path = self.local_path / "src/test_noqa_complex.py"
        files, _ = _complexipy.main([path.resolve().as_posix()], quiet=False)
        total_complexity = sum([file.complexity for file in files])
        # The only function has a noqa: complexipy, so it is ignored.
        self.assertEqual(0, total_complexity)


if __name__ == "__main__":
    unittest.main()
