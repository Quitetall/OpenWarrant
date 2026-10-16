# SPDX-License-Identifier: Apache-2.0
"""A real corrupt index is unavailable evidence, never a Warrant failure."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

repo, war, scratch = map(Path, sys.argv[1:])
with tempfile.TemporaryDirectory(dir=scratch) as directory:
    root = Path(directory) / "repo"
    shutil.copytree(repo / "conformance/fixtures/inbox/repository", root)
    subprocess.run(["git", "init", "--quiet"], cwd=root, check=True, timeout=10)
    env = dict(os.environ, OPENWARRANT_NO_PROJECTS="1", OPENWARRANT_NO_UPDATE_CHECK="1")
    def request():
        output = subprocess.run([str(war), "verify", "IX-WAR-0003", "--performer", "fixture-performer", "--json"], cwd=root, env=env, capture_output=True, text=True, timeout=60)
        return json.loads(output.stdout)
    positive = request()
    assert positive["exit_code"] == 0, positive
    (root / ".git/index").write_bytes(b"broken")
    result = request()
    assert result["exit_code"] != 0, result
    assert result["counts"]["unknown"] == 1 and result["counts"]["error"] == 0, result
    diagnostic = result["diagnostics"][0]
    assert diagnostic["rule"] == "verify.subject-unavailable", result
    assert "exit status: 128" in diagnostic["message"], result
    assert "index file smaller than expected" in diagnostic["message"], result
    assert not (root / "docs/warrants/IX-WAR-0003/verifications").exists()
    (root / ".git/index").unlink()
    restored = request()
    assert restored["exit_code"] == 0, restored
    print("PASS: real Git inventory, corrupt-index UNKNOWN with cause, exact restore; no verdict")
