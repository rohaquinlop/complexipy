"""Python API behavior on repository files crafted to mislead or stall analysis."""

import os
import sys

import pytest

from complexipy import file_complexity

unix_only = pytest.mark.skipif(sys.platform == "win32", reason="needs symlinks")


@unix_only
def test_a_source_symlinked_to_a_device_is_refused_without_reading(tmp_path):
    source = tmp_path / "zero.py"
    os.symlink("/dev/zero", source)

    with pytest.raises(ValueError, match="not a regular file"):
        file_complexity(str(source))
