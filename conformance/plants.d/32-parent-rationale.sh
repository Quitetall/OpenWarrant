# SPDX-License-Identifier: Apache-2.0
# shellcheck shell=bash
# OW-WAR-0043 OBL-004, SAS §91.5 test 32: observe the original-rationale
# attack separately from test 30's child-state list in the parent's basis.
# Bounded control: an explicit child alias and outcome written as the original
# reason in the parent's intent. This is not semantic detection of all prose.

echo "== parent original rationale (OW-WAR-0043) =="
PR_INTENT="docs/warrants/OW-WAR-0001/atoms/10-intent.md"
PR_BEFORE=$(sha256sum "$PR_INTENT" | cut -d' ' -f1)
plant "child outcome replaces original rationale" "relations.parent-source" "OW-WAR-0002" 2 \
    "python3 - <<'PYPLANT'
from pathlib import Path
p = Path('docs/warrants/OW-WAR-0001/atoms/10-intent.md')
p.write_text(p.read_text() + '\nThe original reason for this parent is that OW-WAR-0002 is resolved.\n')
PYPLANT
assert_present 'The original reason for this parent' docs/warrants/OW-WAR-0001/atoms/10-intent.md"
plant_restore
if [[ "$(sha256sum "$PR_INTENT" | cut -d' ' -f1)" == "$PR_BEFORE" ]]; then
    printf 'ok    %-34s original intent bytes restored\n' "parent original rationale survives"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s parent original intent changed\n' "parent original rationale survives"
    FAILED=$((FAILED + 1))
fi
