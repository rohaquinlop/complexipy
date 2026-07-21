from __future__ import annotations

from unittest.mock import patch

import pytest

from complexipy.main import resolve_config
from complexipy.types import ColorTypes, OutputFormat, Sort

_NO_TOML = patch("complexipy.main.TOML_CONFIG", None)


class TestRunConfigDefaults:
    def test_default_complexity(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.max_complexity_allowed == 15

    def test_default_booleans(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.snapshot_create is False
        assert cfg.snapshot_ignore is False
        assert cfg.quiet is False
        assert cfg.ignore_complexity is False
        assert cfg.failed is False
        assert cfg.check_script is False
        assert cfg.no_ignore is False
        assert cfg.report_ignored is False
        assert cfg.ratchet is False

    def test_default_enums(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.color == ColorTypes.auto
        assert cfg.sort == Sort.asc

    def test_default_optional_none(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.output is None
        assert cfg.diff is None
        assert cfg.diff_only is None
        assert cfg.top is None

    def test_default_lists(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.output_format == []
        assert cfg.exclude == []

    def test_default_plain_and_suggest(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.plain is False
        assert cfg.suggest_refactors is False


class TestResolveConfigOverrides:
    def test_cli_max_complexity(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=25,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.max_complexity_allowed == 25

    def test_cli_quiet(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=True,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.quiet is True

    def test_cli_color_yes(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=ColorTypes.yes,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.color == ColorTypes.yes

    def test_cli_sort_desc(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=Sort.desc,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.sort == Sort.desc

    def test_cli_exclude(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=["tests/**"],
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.exclude == ["tests/**"]

    def test_cli_output_format(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=["csv", "json"],
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.output_format == ["csv", "json"]

    def test_cli_diff(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff="main",
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.diff == "main"
        assert cfg.diff_only is None

    def test_cli_top(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=10,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.top == 10


class TestBooleanNormalization:
    def test_no_ignore_normalized_to_bool(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert isinstance(cfg.no_ignore, bool)
        assert cfg.no_ignore is False

    def test_report_ignored_normalized_to_bool(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert isinstance(cfg.report_ignored, bool)
        assert cfg.report_ignored is False

    def test_ratchet_normalized_to_bool(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert isinstance(cfg.ratchet, bool)
        assert cfg.ratchet is False


class TestLegacyOutputFlags:
    def test_all_none_by_default(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.legacy_cli_output_flags == {
            OutputFormat.csv: None,
            OutputFormat.json: None,
            OutputFormat.gitlab: None,
            OutputFormat.sarif: None,
        }

    def test_csv_flag_stored(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=True,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.legacy_cli_output_flags[OutputFormat.csv] is True
        assert cfg.legacy_cli_output_flags[OutputFormat.json] is None


class TestPlainAndSuggestRefactors:
    def test_plain_resolved_to_false_by_default(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.plain is False

    def test_suggest_refactors_resolved_to_false_by_default(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.suggest_refactors is False

    def test_plain_true(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=True,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.plain is True

    def test_plain_and_quiet_raises(self):
        with pytest.raises(Exception):
            with _NO_TOML:
                resolve_config(
                    paths=["."],
                    max_complexity_allowed=None,
                    snapshot_create=None,
                    snapshot_ignore=None,
                    quiet=True,
                    ignore_complexity=None,
                    failed=None,
                    color=None,
                    sort=None,
                    output_format=None,
                    output=None,
                    output_csv=None,
                    output_json=None,
                    output_gitlab=None,
                    output_sarif=None,
                    diff=None,
                    diff_only=None,
                    ratchet=None,
                    top=None,
                    plain=True,
                    suggest_refactors=None,
                    exclude=None,
                    check_script=None,
                    no_ignore=None,
                    report_ignored=None,
                    version=False,
                )


class TestDiffFlagResolution:
    def test_diff_only_when_both_set(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff="main",
                diff_only="main",
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.diff is None
        assert cfg.diff_only == "main"

    def test_diff_standalone(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff="main",
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.diff == "main"
        assert cfg.diff_only is None


class TestCheckScriptAndNoIgnore:
    def test_check_script(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=True,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.check_script is True

    def test_no_ignore(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=True,
                report_ignored=None,
                version=False,
            )
        assert cfg.no_ignore is True


class TestOutputDir:
    def test_output_dir(self):
        with _NO_TOML:
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output="/tmp/results",
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.output == "/tmp/results"


class TestRatchetFromToml:
    def test_ratchet_true_from_toml(self):
        with patch(
            "complexipy.main.TOML_CONFIG",
            {"ratchet": True},
        ):
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff="main",
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.ratchet is True

    def test_ratchet_from_toml_without_diff_errors(self):
        with pytest.raises(Exception):
            with patch(
                "complexipy.main.TOML_CONFIG",
                {"ratchet": True},
            ):
                resolve_config(
                    paths=["."],
                    max_complexity_allowed=None,
                    snapshot_create=None,
                    snapshot_ignore=None,
                    quiet=None,
                    ignore_complexity=None,
                    failed=None,
                    color=None,
                    sort=None,
                    output_format=None,
                    output=None,
                    output_csv=None,
                    output_json=None,
                    output_gitlab=None,
                    output_sarif=None,
                    diff=None,
                    diff_only=None,
                    ratchet=None,
                    top=None,
                    plain=None,
                    suggest_refactors=None,
                    exclude=None,
                    check_script=None,
                    no_ignore=None,
                    report_ignored=None,
                    version=False,
                )

    def test_ratchet_false_by_default_in_toml(self):
        with patch(
            "complexipy.main.TOML_CONFIG",
            {},
        ):
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.ratchet is False


class TestTomlOverrides:
    def test_toml_max_complexity(self):
        with patch(
            "complexipy.main.TOML_CONFIG",
            {"max-complexity-allowed": 30},
        ):
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=None,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.max_complexity_allowed == 30

    def test_cli_overrides_toml(self):
        with patch(
            "complexipy.main.TOML_CONFIG",
            {"max-complexity-allowed": 30},
        ):
            cfg = resolve_config(
                paths=["."],
                max_complexity_allowed=10,
                snapshot_create=None,
                snapshot_ignore=None,
                quiet=None,
                ignore_complexity=None,
                failed=None,
                color=None,
                sort=None,
                output_format=None,
                output=None,
                output_csv=None,
                output_json=None,
                output_gitlab=None,
                output_sarif=None,
                diff=None,
                diff_only=None,
                ratchet=None,
                top=None,
                plain=None,
                suggest_refactors=None,
                exclude=None,
                check_script=None,
                no_ignore=None,
                report_ignored=None,
                version=False,
            )
        assert cfg.max_complexity_allowed == 10
