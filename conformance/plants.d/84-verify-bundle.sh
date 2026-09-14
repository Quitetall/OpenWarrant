# shellcheck shell=bash
# The verification bundle and a configured verifier (slice C3, §46 / §75.2):
# the bundle is deterministic, a missing verifier is said, a fixture verifier
# is ingested through the same seam, and a self-verifying one is refused.

VB=docs/warrants/OW-WAR-0063/verifications

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
    "true" \
    verify OW-WAR-0063 --performer claude --run

# A configured verifier's response goes through the unchanged ingest.
plant_cmd "a configured verifier is ingested" "verify.recorded" "fixture-verifier" 0 \
    "printf '\n[verify]\nverifier_argv = [\"bash\", \"conformance/fixtures/verifier/establishes-all.sh\"]\n' >> openwarrant.toml; assert_present 'establishes-all.sh' openwarrant.toml" \
    verify OW-WAR-0063 --performer claude --run
rm -f "$VB"/bundle-*.json "$VB"/response-*.toml
git checkout -- "$VB" 2>/dev/null || true

# A verifier that answers as the performer is refused by the seam.
plant_cmd "a self-verifying verifier is refused" "verify.inadmissible" "claude" 2 \
    "printf '\n[verify]\nverifier_argv = [\"bash\", \"conformance/fixtures/verifier/self-verifying.sh\"]\n' >> openwarrant.toml; assert_present 'self-verifying.sh' openwarrant.toml" \
    verify OW-WAR-0063 --performer claude --run
rm -f "$VB"/bundle-*.json "$VB"/response-*.toml
git checkout -- "$VB" 2>/dev/null || true
