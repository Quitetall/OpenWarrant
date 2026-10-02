# shellcheck shell=bash
# The verification bundle and a configured verifier (slice C3, §46 / §75.2):
# the bundle is deterministic, a missing verifier is said, a fixture verifier
# is ingested through the same seam, and a self-verifying one is refused.

VB=docs/warrants/OW-WAR-0063/verifications

# Keep only this plant's exact prior records and note existing archives. The
# fixture ingestion may retain these bytes; cleanup must not remove history
# that was already present before this plant ran.
VB_SNAPSHOT=$(mktemp -d)
python3 - "$VB" "$VB_SNAPSHOT" <<'PY_CAPTURE'
import hashlib, json, pathlib, shutil, sys
root, snapshot = map(pathlib.Path, sys.argv[1:])
records = []
for path in sorted(root.glob("*.toml")):
    if path.is_symlink():
        raise SystemExit("fixture record must be a regular file")
    digest = hashlib.sha256(path.read_bytes()).hexdigest()
    shutil.copyfile(path, snapshot / path.name)
    records.append({"name": path.name, "digest": digest,
                    "already_present": (root / "history" / (digest + ".toml")).exists()})
(snapshot / "manifest.json").write_text(json.dumps(records))
PY_CAPTURE

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

# Deterministic: two runs over one tree write the same digests, hence the
# same files. OW-WAR-0063 exceeds `[verify] max_bundle_tokens`, so each run
# writes one bundle per obligation (t-9f7e); the second adds none.
"$WAR" verify OW-WAR-0063 --performer claude --bundle >/dev/null 2>&1
VB_1=$(ls "$VB"/bundle-*.json 2>/dev/null | wc -l)
"$WAR" verify OW-WAR-0063 --performer claude --bundle >/dev/null 2>&1
VB_N=$(ls "$VB"/bundle-*.json 2>/dev/null | wc -l)
if [[ "$VB_1" -ge 1 && "$VB_N" -eq "$VB_1" ]]; then
    printf 'ok    %-34s two runs, the same %s digest(s), no new file\n' "the bundle is deterministic" "$VB_N"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s bundle file(s) after one run, %s after two\n' "the bundle is deterministic" "$VB_1" "$VB_N"
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
rm -rf "$VB"/bundle-*.json "$VB"/response-*.toml "$VB"/responses
git checkout -- "$VB" 2>/dev/null || true

VB_HISTORY_RESULT=$(python3 - "$VB" "$VB_SNAPSHOT" <<'PY_HISTORY'
import json, pathlib, sys
root, snapshot = map(pathlib.Path, sys.argv[1:])
records = json.loads((snapshot / "manifest.json").read_text())
retained = bool(records)
for record in records:
    history = root / "history" / (record["digest"] + ".toml")
    retained &= history.is_file() and not history.is_symlink() and history.read_bytes() == (snapshot / record["name"]).read_bytes()
    if not record["already_present"] and (history.exists() or history.is_symlink()):
        history.unlink()
if not retained:
    raise SystemExit("exact prior fixture records were not retained")
print(f"{len(records)} exact prior records retained; pre-existing history untouched")
PY_HISTORY
)
VB_HISTORY_STATUS=$?
if [[ "$VB_HISTORY_STATUS" -eq 0 ]]; then
    printf 'ok    %-34s %s\n' "replacement retains prior records" "$VB_HISTORY_RESULT"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exact prior fixture records unavailable\n' "replacement retains prior records"
    FAILED=$((FAILED + 1))
fi
rm -rf -- "$VB_SNAPSHOT"
unset VB_SNAPSHOT

# A verifier that answers as the performer is refused by the seam.
plant_cmd "a self-verifying verifier is refused" "verify.inadmissible" "claude" 2 \
    "vb_verifier '\"bash\", \"conformance/fixtures/verifier/self-verifying.sh\"'; assert_present 'self-verifying.sh' openwarrant.toml" \
    verify OW-WAR-0063 --performer claude --run
rm -rf "$VB"/bundle-*.json "$VB"/response-*.toml "$VB"/responses
git checkout -- "$VB" 2>/dev/null || true
