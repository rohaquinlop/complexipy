"""Python API behavior on repository files crafted to mislead or stall analysis."""

import os
import sys

import pytest

from complexipy import file_complexity

unix_only = pytest.mark.skipif(sys.platform == "win32", reason="needs symlinks")


EVIL_BODY = (
    "    for x in a:\n"
    "        if x:\n"
    "            if b:\n"
    "                return x\n"
    "    return 0\n"
)


def test_a_utf7_coding_cookie_fails_instead_of_hiding_a_function(tmp_path):
    source = tmp_path / "utf7.py"
    source.write_bytes(
        b"# -*- coding: utf-7 -*-\n"
        b"ok = 1  # +AAo-def evil(a, b): return a+AAo-\n"
    )

    with pytest.raises(ValueError, match="utf_7"):
        file_complexity(str(source))


def test_a_latin1_coding_cookie_is_decoded(tmp_path):
    source = tmp_path / "latin1.py"
    source.write_bytes(
        b"# -*- coding: latin-1 -*-\ndef caf\xe9(a, b):\n" + EVIL_BODY.encode()
    )

    result = file_complexity(str(source))

    assert [(f.name, f.complexity) for f in result.functions] == [
        ("caf\u00e9", 6)
    ]


def test_marker_text_in_a_string_does_not_suppress(tmp_path):
    source = tmp_path / "marker.py"
    source.write_text(
        'def evil(a, b, note="# complexipy: ignore"):\n' + EVIL_BODY,
        encoding="utf-8",
    )

    result = file_complexity(str(source))

    assert [f.name for f in result.functions] == ["evil"]


@unix_only
def test_a_source_symlinked_to_a_device_is_refused_without_reading(tmp_path):
    source = tmp_path / "zero.py"
    os.symlink("/dev/zero", source)

    with pytest.raises(ValueError, match="not a regular file"):
        file_complexity(str(source))
