from __future__ import annotations

from complexipy.utils.toml import (
    get_complexipy_toml_config,
    load_toml_config,
    load_values_from_toml_key,
)


class TestDiffSectionParsing:
    def test_diff_section_in_complexipy_toml(self, tmp_path):
        (tmp_path / "complexipy.toml").write_text(
            "max-complexity-allowed = 15\n"
            "[diff]\n"
            'branch = "main"\n'
            "staged = true\n"
        )
        config = load_toml_config(str(tmp_path), "complexipy.toml")
        assert config["diff"] == {"branch": "main", "staged": True}

    def test_diff_section_in_dot_complexipy_toml(self, tmp_path):
        (tmp_path / ".complexipy.toml").write_text('[diff]\nbranch = "main"\n')
        config = load_toml_config(str(tmp_path), ".complexipy.toml")
        assert config["diff"] == {"branch": "main"}

    def test_diff_section_in_pyproject_toml(self, tmp_path):
        (tmp_path / "pyproject.toml").write_text(
            "[tool.complexipy]\n"
            "max-complexity-allowed = 15\n"
            "[tool.complexipy.diff]\n"
            'branch = "main"\n'
            "staged = true\n"
        )
        config = get_complexipy_toml_config(str(tmp_path))
        assert config["max-complexity-allowed"] == 15
        assert config["diff"] == {"branch": "main", "staged": True}

    def test_diff_section_with_only_staged(self, tmp_path):
        (tmp_path / "complexipy.toml").write_text("[diff]\nstaged = true\n")
        config = load_toml_config(str(tmp_path), "complexipy.toml")
        assert config["diff"] == {"staged": True}

    def test_empty_diff_section_parses(self, tmp_path):
        (tmp_path / "complexipy.toml").write_text("[diff]\n")
        config = load_toml_config(str(tmp_path), "complexipy.toml")
        assert config["diff"] == {}

    def test_diff_key_normalizes_nested_values(self):
        normalized = load_values_from_toml_key(
            "diff", {"branch": "main", "staged": True}
        )
        assert normalized == {"branch": "main", "staged": True}

    def test_non_dict_diff_key_passes_through(self):
        value = "main"
        assert load_values_from_toml_key("diff", value) == "main"
