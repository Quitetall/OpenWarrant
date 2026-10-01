#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Exercise the actual 69-current signature assertion with controlled CLI outputs."""
from pathlib import Path
import subprocess
import tempfile

root = Path(__file__).resolve().parents[2]
source = (root / "conformance/plants.d/69-current.sh").read_text()
start = source.index('DRY_ALL=$("$WAR" sign --all --dry-run')
end = source.index("# ---------------------------------------------------------------- OBL-002", start)
assertion = source[start:end]
helpers = (root / "conformance/lib.sh").read_text()
start = helpers.index("line_has() {")
end = helpers.index("\n}", start) + 2
line_has = helpers[start:end]

with tempfile.TemporaryDirectory(prefix="ow-current-signature-") as temp:
    stub = Path(temp) / "war"
    stub.write_text('''#!/usr/bin/env bash
if [[ "$1" == sign ]]; then
    printf '%s\\n' "$DRY_TEXT"
    exit "$DRY_STATUS"
fi
printf '%s\\n' 'contract_digest = "691f51ce-fixture"'
exit 0
''')
    stub.chmod(0o700)
    for status, text, expected in [(1, "WELL-FORMED (record only)", False), (0, "WELL-FORMED (record only)", True), (0, "", False), (0, "ERROR authorize.no-amendment OW-WAR-0073\nWELL-FORMED (record only)", False)]:
        shell = f'''set -uo pipefail
WAR={str(stub)!r}
export DRY_STATUS={status}
export DRY_TEXT={text!r}
{line_has}
cu_ok() {{ printf 'CONTROL_PASS\\n'; }}
cu_fail() {{ printf 'CONTROL_FAIL\\n'; }}
{assertion}
'''
        result = subprocess.run(["bash", "-c", shell], capture_output=True, text=True, timeout=10)
        assert result.returncode == 0, f"control could not run: {result.stderr!r}"
        assert result.stdout in ("CONTROL_PASS\n", "CONTROL_FAIL\n"), f"no assertion result: {result.stdout!r}"
        passed = result.stdout == "CONTROL_PASS\n"
        assert passed == expected, f"exit {status}: expected pass={expected}, got {result.stdout!r}, stderr={result.stderr!r}"
        print(f"ok: dry-run exit {status}: {'accepted' if expected else 'refused'}")
