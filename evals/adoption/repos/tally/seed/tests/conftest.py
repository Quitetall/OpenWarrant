import subprocess
import sys
import textwrap
from pathlib import Path

import pytest

REPO = Path(__file__).resolve().parent.parent


def run_tally(*args, cwd=REPO):
    """Run the CLI as a user would and return the completed process."""
    return subprocess.run(
        [sys.executable, "-m", "tally", *map(str, args)],
        cwd=cwd,
        capture_output=True,
        text=True,
        timeout=60,
    )


@pytest.fixture
def journal(tmp_path):
    """Write dedented journal text to a file and return its path."""

    def write(text, name="main.ledger"):
        path = tmp_path / name
        path.write_text(textwrap.dedent(text).lstrip("\n"), encoding="utf-8")
        return path

    return write
