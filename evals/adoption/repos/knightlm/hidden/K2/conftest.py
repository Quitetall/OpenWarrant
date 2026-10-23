"""Shared harness for KnightLM hidden acceptance tests.

The check runs with the repository root as the working directory. The session
fixture builds the CLI once (`cargo build --locked -q`, a no-op after the
grader's prepare step), asks `cargo metadata` for the target directory (this
honours CARGO_TARGET_DIR), and runs `<target>/debug/knightlm` directly.
Every test writes its own fixtures under pytest's tmp_path.
"""
import json
import os
import subprocess
from pathlib import Path

import pytest

START_FEN = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1"
REPO = Path(os.environ.get("KNIGHTLM_REPO", os.getcwd())).resolve()


@pytest.fixture(scope="session")
def binary():
    build = subprocess.run(["cargo", "build", "--locked", "-q"], cwd=REPO,
                           capture_output=True, text=True, timeout=1500)
    if build.returncode:
        pytest.fail("cargo build failed:\n" + build.stderr[-4000:])
    meta = subprocess.run(["cargo", "metadata", "--format-version", "1", "--no-deps"],
                          cwd=REPO, capture_output=True, text=True, timeout=120, check=True)
    path = Path(json.loads(meta.stdout)["target_directory"]) / "debug" / "knightlm"
    if not path.is_file():
        pytest.fail(f"built CLI not found at {path}")
    return path


@pytest.fixture
def cli(binary):
    """Run the CLI; returns CompletedProcess (text mode)."""
    def run(*args, cwd=None):
        return subprocess.run([str(binary), *map(str, args)], cwd=cwd or REPO,
                              capture_output=True, text=True, timeout=120)
    return run


@pytest.fixture
def ok(cli):
    """Run the CLI, require exit 0, return stdout parsed as JSON."""
    def run(*args):
        result = cli(*args)
        assert result.returncode == 0, f"exit {result.returncode}: {args}\n{result.stderr}"
        return json.loads(result.stdout)
    return run


@pytest.fixture
def game(tmp_path):
    """Write a version-1 GameRecord JSON file and return its path."""
    counter = iter(range(10**6))

    def write(fen=START_FEN, moves=(), complete=True, game_id="hidden-test-game",
              source="hidden acceptance fixture"):
        path = tmp_path / f"game-{next(counter)}.json"
        path.write_text(json.dumps({
            "schema_version": 1, "game_id": game_id, "source": source,
            "initial_fen": fen, "history_complete": complete, "moves": list(moves),
        }))
        return path
    return write


def refused(result):
    """A refusal: nonzero exit and nothing on stdout."""
    return result.returncode != 0 and result.stdout.strip() == ""
