import json
from pathlib import Path

from complexipy import _complexipy
from complexipy.utils.cache import (
    CACHE_DIR_NAME,
    FUNCTIONS_CACHE_KEY,
    MAX_CACHE_ENTRIES,
    remember_previous_functions,
)


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

        # Verify support files were created
        gitignore_file = cache_dir / ".gitignore"
        assert gitignore_file.exists()
        assert gitignore_file.is_file()
        assert gitignore_file.read_text(encoding="utf-8") == "*\n"

        cachedir_tag = cache_dir / "CACHEDIR.TAG"
        assert cachedir_tag.exists()
        assert cachedir_tag.is_file()
        assert cachedir_tag.read_text(encoding="utf-8").startswith(
            "Signature: 8a477f597d28d172789f06886806bc55"
        )

        readme = cache_dir / "README.md"
        assert readme.exists()
        assert readme.is_file()
        assert "complexipy cache directory" in readme.read_text(encoding="utf-8")

        cache_file = cache_dir / "v" / "cache" / FUNCTIONS_CACHE_KEY
        assert cache_file.exists()
        assert cache_file.is_file()

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

    def test_cache_reuses_single_functions_file_for_target_sets(self, tmp_path: Path):
        """Test that target-set caches are entries in one value file."""
        first_file = tmp_path / "first.py"
        first_file.write_text("def first():\n    return 1\n", encoding="utf-8")
        second_file = tmp_path / "second.py"
        second_file.write_text("def second():\n    return 2\n", encoding="utf-8")

        first_files, _ = _complexipy.main([str(first_file)], False, [])
        second_files, _ = _complexipy.main([str(second_file)], False, [])

        assert (
            remember_previous_functions(
                invocation_path=str(tmp_path),
                targets=[str(first_file)],
                files_complexities=first_files,
            )
            is None
        )
        first_previous = remember_previous_functions(
            invocation_path=str(tmp_path),
            targets=[str(first_file)],
            files_complexities=first_files,
        )
        assert first_previous is not None
        assert len(first_previous) == 1

        remember_previous_functions(
            invocation_path=str(tmp_path),
            targets=[str(second_file)],
            files_complexities=second_files,
        )

        cache_dir = tmp_path / CACHE_DIR_NAME
        value_files = [path for path in cache_dir.rglob("*") if path.is_file()]
        cache_values = [path for path in value_files if path.name == FUNCTIONS_CACHE_KEY]
        legacy_json_files = list(cache_dir.glob("*.json"))

        assert len(cache_values) == 1
        assert legacy_json_files == []

    def test_cache_migrates_and_removes_legacy_hash_file(self, tmp_path: Path):
        """Test that existing per-target JSON files are migrated and cleaned up."""
        test_file = tmp_path / "test.py"
        test_file.write_text("def example():\n    return 1\n", encoding="utf-8")
        files, _ = _complexipy.main([str(test_file)], False, [])

        remember_previous_functions(
            invocation_path=str(tmp_path),
            targets=[str(test_file)],
            files_complexities=files,
        )

        cache_file = tmp_path / CACHE_DIR_NAME / "v" / "cache" / FUNCTIONS_CACHE_KEY
        cache_payload = json.loads(cache_file.read_text(encoding="utf-8"))
        [cache_key] = cache_payload["entries"].keys()
        cache_file.unlink()

        legacy_file = tmp_path / CACHE_DIR_NAME / f"{cache_key}.json"
        legacy_file.write_text(
            json.dumps(next(iter(cache_payload["entries"].values()))),
            encoding="utf-8",
        )

        previous = remember_previous_functions(
            invocation_path=str(tmp_path),
            targets=[str(test_file)],
            files_complexities=files,
        )

        assert previous is not None
        assert len(previous) == 1
        assert not legacy_file.exists()
        assert cache_file.exists()

    def test_cache_prunes_old_target_set_entries(self, tmp_path: Path):
        """Test that the single value file does not grow unbounded."""
        files = []
        for index in range(MAX_CACHE_ENTRIES + 1):
            test_file = tmp_path / f"test_{index}.py"
            test_file.write_text(
                f"def example_{index}():\n    return {index}\n",
                encoding="utf-8",
            )
            files.append(test_file)

        for test_file in files:
            complexities, _ = _complexipy.main([str(test_file)], False, [])
            remember_previous_functions(
                invocation_path=str(tmp_path),
                targets=[str(test_file)],
                files_complexities=complexities,
            )

        cache_file = tmp_path / CACHE_DIR_NAME / "v" / "cache" / FUNCTIONS_CACHE_KEY
        cache_payload = json.loads(cache_file.read_text(encoding="utf-8"))

        assert len(cache_payload["entries"]) == MAX_CACHE_ENTRIES

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
