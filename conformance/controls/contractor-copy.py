#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""The real contractor clone must preserve files without sharing object inodes."""
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[2]
source = (root / "conformance/plants.d/56-contractor.sh").read_text()
start = source.index("CT_FROZEN=(")
end = source.index("\n)", start) + 2
frozen = source[start:end]
start = source.index("CT_COPY=$(mktemp -d)")
end = source.index("\nsed -i", start)
copy = source[start:end]

with tempfile.TemporaryDirectory(prefix="ow-contractor-control-") as temp:
    repo = Path(temp) / "source"
    repo.mkdir()
    paths = subprocess.check_output(["bash", "-c", frozen + '\nprintf "%s\\n" "${CT_FROZEN[@]}"'], text=True).splitlines()
    for path in paths:
        target = repo / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes((root / path).read_bytes())
    for args in [["init", "-q"], ["add", "."], ["-c", "user.name=Fixture", "-c", "user.email=fixture@invalid", "-c", "commit.gpgsign=false", "commit", "-qm", "frozen modules"]]:
        subprocess.run(["git", "-C", str(repo), *args], check=True, capture_output=True)
    shell = f'''set -uo pipefail
export TMPDIR={temp!r}
REPO_ROOT={str(repo)!r}
{frozen}
{copy}
printf '%s\\n' "$CT_COPY/r"
'''
    result = subprocess.run(["bash", "-c", shell], capture_output=True, text=True, timeout=30)
    assert result.returncode == 0, f"copy failed: {result.stdout!r} {result.stderr!r}"
    clone = Path(result.stdout.strip())
    for path in paths:
        assert (clone / path).read_bytes() == (repo / path).read_bytes(), f"missing or changed frozen module: {path}"
    objects = [p for p in (repo / ".git/objects").rglob("*") if p.is_file()]
    assert objects, "fixture produced no Git objects"
    for obj in objects:
        copied = clone / ".git/objects" / obj.relative_to(repo / ".git/objects")
        original_stat, copy_stat = obj.stat(), copied.stat()
        assert (original_stat.st_dev, original_stat.st_ino) != (copy_stat.st_dev, copy_stat.st_ino), f"clone hard-linked Git object: {obj.name}"
    print(f"ok: {len(paths)} frozen modules preserved; {len(objects)} objects copied without hard links")
