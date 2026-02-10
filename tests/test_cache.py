from pathlib import Path

from complexipy import _complexipy
from complexipy.utils.cache import CACHE_DIR_NAME, remember_previous_functions


class TestCache:
    def test_cache_directory_creates_gitignore(self, tmp_path: Path):
        """Test that .gitignore is created in cache directory with * content."""
        test_file = tmp_path / "test.py"
        test_file.write_text(
            "def example():\n    if True:\n        return 1\n    return 0\n",
            encoding="utf-8",
        )

        # Analyze the file to get FileComplexity objects
        files, _ = _complexipy.main([str(test_file)], False, [])

        # Call remember_previous_functions which should create the cache directory
        remember_previous_functions(
            invocation_path=str(tmp_path),
            targets=[str(test_file)],
            files_complexities=files,
        )

        # Verify cache directory was created
        cache_dir = tmp_path / CACHE_DIR_NAME
        assert cache_dir.exists()
        assert cache_dir.is_dir()

        # Verify .gitignore was created
        gitignore_file = cache_dir / ".gitignore"
        assert gitignore_file.exists()
        assert gitignore_file.is_file()

        # Verify .gitignore content
        content = gitignore_file.read_text(encoding="utf-8")
        assert content == "*\n"

    def test_gitignore_is_not_recreated_if_exists(self, tmp_path: Path):
        """Test that existing .gitignore is not overwritten."""
        test_file = tmp_path / "test.py"
        test_file.write_text(
            "def example():\n    return 1\n",
            encoding="utf-8",
        )

        # Create cache directory and custom .gitignore
        cache_dir = tmp_path / CACHE_DIR_NAME
        cache_dir.mkdir(exist_ok=True)
        gitignore_file = cache_dir / ".gitignore"
        custom_content = "# Custom gitignore\n*.json\n"
        gitignore_file.write_text(custom_content, encoding="utf-8")

        # Analyze the file to get FileComplexity objects
        files, _ = _complexipy.main([str(test_file)], False, [])

        # Call remember_previous_functions
        remember_previous_functions(
            invocation_path=str(tmp_path),
            targets=[str(test_file)],
            files_complexities=files,
        )

        # Verify .gitignore still has custom content
        content = gitignore_file.read_text(encoding="utf-8")
        assert content == custom_content

    def test_cache_failure_does_not_break_functionality(self, tmp_path: Path):
        """Test that cache operations don't break when filesystem operations fail."""
        test_file = tmp_path / "test.py"
        test_file.write_text(
            "def example():\n    return 1\n",
            encoding="utf-8",
        )

        # Analyze the file to get FileComplexity objects
        files, _ = _complexipy.main([str(test_file)], False, [])

        # Use an invalid path that can't be created
        result = remember_previous_functions(
            invocation_path="/dev/null/invalid_path",
            targets=[str(test_file)],
            files_complexities=files,
        )

        # Should return None gracefully without raising exceptions
        assert result is None
