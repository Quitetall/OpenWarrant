# shellcheck shell=bash
# The verification bundle and a configured verifier (slice C3, §46 / §75.2):
# the bundle is deterministic, a missing verifier is said, a fixture verifier
# is ingested through the same seam, and a self-verifying one is refused.

VB=docs/warrants/OW-WAR-0063/verifications

# This repository configures a verifier (OW-WAR-0117), so each plant sets the
# `[verify] verifier_argv` it needs rather than assuming the table is absent:
# empty for "none configured", a fixture otherwise. Appending a second
# [verify] table would be a TOML error, and leaving the real one in place
# would call a model from inside the battery. openwarrant.toml is restored by
# the battery like every mutated path.
vb_verifier() { # argv as a TOML array body, e.g. '"bash", "x.sh"' or ''
    python3 - "$1" <<'PY'
import re, sys
p = "openwarrant.toml"; t = open(p).read()
line = "verifier_argv = [%s]" % sys.argv[1]
if re.search(r"(?m)^verifier_argv = .*$", t):
    t = re.sub(r"(?m)^verifier_argv = .*$", line, t, count=1)
else:
    t += "\n[verify]\n%s\n" % line
open(p, "w").write(t)
PY
    grep -qxF "verifier_argv = [$1]" openwarrant.toml
}

# Deterministic: two bundles of one tree share one digest, hence one file.
"$WAR" verify OW-WAR-0063 --performer claude --bundle >/dev/null 2>&1
"$WAR" verify OW-WAR-0063 --performer claude --bundle >/dev/null 2>&1
VB_N=$(ls "$VB"/bundle-*.json 2>/dev/null | wc -l)
if [[ "$VB_N" -eq 1 ]]; then
    printf 'ok    %-34s two bundles, one digest, one file\n' "the bundle is deterministic"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s bundle file(s) after two runs\n' "the bundle is deterministic" "$VB_N"
    FAILED=$((FAILED + 1))
fi
rm -f "$VB"/bundle-*.json

plant_cmd "no verifier configured is said" "verify.no-verifier" "verifier_argv" 2 \
    "vb_verifier ''" \
    verify OW-WAR-0063 --performer claude --run

# A configured verifier's response goes through the unchanged ingest.
plant_cmd "a configured verifier is ingested" "verify.recorded" "fixture-verifier" 0 \
    "vb_verifier '\"bash\", \"conformance/fixtures/verifier/establishes-all.sh\"'; assert_present 'establishes-all.sh' openwarrant.toml" \
    verify OW-WAR-0063 --performer claude --run
rm -f "$VB"/bundle-*.json "$VB"/response-*.toml
git checkout -- "$VB" 2>/dev/null || true

# A verifier that answers as the performer is refused by the seam.
plant_cmd "a self-verifying verifier is refused" "verify.inadmissible" "claude" 2 \
    "vb_verifier '\"bash\", \"conformance/fixtures/verifier/self-verifying.sh\"'; assert_present 'self-verifying.sh' openwarrant.toml" \
    verify OW-WAR-0063 --performer claude --run
rm -f "$VB"/bundle-*.json "$VB"/response-*.toml
git checkout -- "$VB" 2>/dev/null || true
