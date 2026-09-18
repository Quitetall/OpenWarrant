# SPDX-License-Identifier: Apache-2.0
"""Produce stage evidence by running configured checks on an exact Git revision."""
import subprocess
import time
from pathlib import Path

from harness import bounded_command
from stages import plan, require, completed_from_evidence


def checkpoint(config, source_sha256, revision, worktree, stages, deadline):
    config = plan(config)
    # Validate identifiers even when no checks ultimately run.
    completed_from_evidence(config, source_sha256, revision, None)
    require(isinstance(stages, (list, tuple)) and stages
            and all(isinstance(s, str) and s in config["stages"] for s in stages)
            and len(stages) == len(set(stages)), "Known distinct stages required")
    selected = set(stages)
    require(all(set(config["stages"][s]["dependencies"]) <= selected for s in stages),
            "Stage checks require prerequisite closure")
    root = Path(worktree)
    require(root.is_dir() and not root.is_symlink(), "Real worktree required")
    r = {"schema": "oh.war/stage-checkpoint/v1", "source_sha256": source_sha256,
         "revision": revision, "plan": config, "execution_state": "unknown",
         "stage_checks": {}, "cause": "Checks not completed"}

    def git(*args):
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            raise TimeoutError("Stage check time exhausted")
        return subprocess.check_output(["git", "-c", "core.hooksPath=/dev/null", *args],
            cwd=root, text=True, stderr=subprocess.PIPE, timeout=min(10, remaining)).strip()

    def unchanged():
        require(Path(git("rev-parse", "--show-toplevel")) == root.resolve(), "Worktree identity mismatch")
        require(git("rev-parse", "HEAD") == revision, "Stage check revision changed")
        require(not git("status", "--porcelain", "--untracked-files=all"), "Stage checks require clean worktree")

    try:
        unchanged()
        pending, checked = set(stages), set()
        while pending:
            stage = next(s for s in sorted(pending) if set(config["stages"][s]["dependencies"]) <= checked)
            observations = r["stage_checks"][stage] = []
            for command in config["stages"][stage]["checks"]:
                observation = {"argv": command, "exit_code": None}
                observations.append(observation)
                code, _, _ = bounded_command(command, root, b"", deadline, observation)
                observation["exit_code"] = code
                unchanged()
            pending.remove(stage)
            checked.add(stage)
        r.update(execution_state="stopped", cause="Configured checks observed on unchanged revision")
    except Exception as error:
        r.update(execution_state="unknown", cause=type(error).__name__ + ": " + str(error)[:500])
    return r
