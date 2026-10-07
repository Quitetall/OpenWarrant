#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Observe clone preparation and parent isolation through the battery entrypoint."""
from pathlib import Path
import os
import shutil
import subprocess
import tempfile

source = Path(__file__).resolve().parents[2]
real_git = shutil.which("git")
assert real_git
with tempfile.TemporaryDirectory(prefix="ow-isolated-index-") as temp:
    base = Path(temp)
    repo = base / "source"
    (repo / "conformance").mkdir(parents=True)
    shutil.copyfile(source / "conformance/plant-isolated.sh", repo / "conformance/plant-isolated.sh")
    (repo / "conformance/fake-battery.sh").write_text('''#!/usr/bin/env bash
printf 'BATTERY_STARTED\\n'
''')
    for args in [["init", "-q"], ["add", "."], ["-c", "user.name=Fixture", "-c", "user.email=fixture@invalid", "-c", "commit.gpgsign=false", "commit", "-qm", "fixture"]]:
        subprocess.run([real_git, "-C", str(repo), *args], check=True, capture_output=True)
    before = (repo / ".git/index").read_bytes()
    tools = base / "tools"
    tools.mkdir()
    wrapper = tools / "git"
    wrapper.write_text('''#!/usr/bin/env bash
for arg in "$@"; do
    if [[ "$arg" == update-index ]]; then
        printf '%s\\n' "$2" >> "$OW_INDEX_TRACE"
        [[ "${OW_FAIL_REFRESH:-0}" != 1 ]] || exit 19
    fi
done
exec "$OW_REAL_GIT" "$@"
''')
    wrapper.chmod(0o700)
    trace = base / "refreshes"
    env = dict(os.environ, PATH=str(tools) + os.pathsep + os.environ["PATH"], OW_REAL_GIT=real_git, OW_INDEX_TRACE=str(trace), OPENWARRANT_ISOLATED_PLANT_SH="conformance/fake-battery.sh", TMPDIR=str(base))
    env.pop("OPENWARRANT_IN_BATTERY", None)
    result = subprocess.run(["bash", "conformance/plant-isolated.sh"], cwd=repo, env=env, capture_output=True, text=True, timeout=30)
    assert result.returncode == 0, result.stderr
    assert "BATTERY_STARTED" in result.stdout
    refreshed = trace.read_text().splitlines() if trace.exists() else []
    assert len(refreshed) == 1, f"expected one clone-index refresh before the battery, got {refreshed}"
    assert refreshed[0] != str(repo), "refreshed the caller index"
    assert (repo / ".git/index").read_bytes() == before, "caller index changed"
    assert not list(base.glob("war-isolated-battery.*")), "clone leaked"
    print("ok: clone index prepared once; caller index unchanged; clone removed")
    env["OW_FAIL_REFRESH"] = "1"
    result = subprocess.run(["bash", "conformance/plant-isolated.sh"], cwd=repo, env=env, capture_output=True, text=True, timeout=30)
    assert result.returncode == 2, f"failed refresh did not refuse: {result.returncode}, {result.stdout!r}"
    assert "plant-isolated.index-refresh-failed" in result.stderr
    assert "BATTERY_STARTED" not in result.stdout, "battery ran after preparation failed"
    assert (repo / ".git/index").read_bytes() == before
    assert not list(base.glob("war-isolated-battery.*"))
    print("ok: failed refresh refuses before battery; caller preserved; clone removed")
