# SPDX-License-Identifier: Apache-2.0
"""Separate Git object/ref storage for verification; not an execution sandbox."""
import subprocess
import time
from pathlib import Path

from verification import require


def git(directory, deadline, *args):
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        raise TimeoutError("Verification workspace time exhausted")
    return subprocess.check_output(["git", "-c", "core.hooksPath=/dev/null", *args],
                                   cwd=directory, text=True, stderr=subprocess.PIPE,
                                   timeout=min(30, remaining)).strip()


def unchanged(directory, revision, deadline):
    directory = Path(directory)
    require(directory.is_dir() and not directory.is_symlink(), "Real verifier workspace required")
    require(Path(git(directory, deadline, "rev-parse", "--show-toplevel")) == directory.resolve(),
            "Verifier workspace identity changed")
    require(git(directory, deadline, "rev-parse", "HEAD") == revision, "Verifier candidate revision changed")
    require(not git(directory, deadline, "status", "--porcelain", "--untracked-files=all"),
            "Verifier candidate workspace changed")


def prepare(source, destination, revision, deadline):
    """Fresh exact-commit clone without shared objects or mutable Git refs."""
    import re
    source, destination = Path(source), Path(destination)
    require(isinstance(revision, str) and re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", revision),
            "Exact verifier revision required")
    require(source.is_dir() and not source.is_symlink(), "Real source repository required")
    require(not destination.exists() and not destination.is_symlink()
            and destination.parent.is_dir(), "Fresh verifier destination required")
    source = source.resolve()
    destination = destination.parent.resolve() / destination.name
    require(destination != source and source not in destination.parents,
            "Verifier workspace must be outside performer workspace")
    require(git(source, deadline, "rev-parse", revision + "^{commit}") == revision,
            "Candidate commit unavailable")
    destination.mkdir(mode=0o700)
    # A fetch into an empty repository copies objects through Git's transport;
    # unlike a linked worktree or --shared clone it shares no mutable refs/objects.
    git(destination, deadline, "init", "--quiet", "--template=")
    git(destination, deadline, "-c", "protocol.file.allow=always", "fetch", "--no-tags", str(source), revision)
    git(destination, deadline, "checkout", "--quiet", "--detach", revision)
    unchanged(destination, revision, deadline)
    require(not (destination / ".git/objects/info/alternates").exists(), "Verifier cannot share object storage")
    require(git(destination, deadline, "rev-parse", "--path-format=absolute", "--git-common-dir")
            != git(source, deadline, "rev-parse", "--path-format=absolute", "--git-common-dir"),
            "Verifier cannot share mutable Git state")
    return destination
