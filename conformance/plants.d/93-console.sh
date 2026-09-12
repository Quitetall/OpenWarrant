# shellcheck shell=bash
# The one screen (OW-WAR-0069): `war console` shows what a human owes and
# `war commit` describes what changed. Neither may act on its own — the console
# signs nothing without the signer's ssh confirmation, and `war commit` stages
# nothing without `--write`. These plants hold both lines.

# The board is the signing queue, not a second opinion about it: whatever
# `war sign --list` names, the screen numbers, and nothing else.
C_LIST=$("$WAR" sign --list 2>/dev/null | grep -cE '^  (OW-WAR-[0-9]{4}|[0-9]+\.[0-9]+\.[0-9]+)')
C_BOARD=$("$WAR" console --json 2>/dev/null | python3 -c 'import json,sys; print(len(json.load(sys.stdin)["result"]["acts"]))')
if [[ "$C_LIST" -gt 0 && "$C_LIST" == "$C_BOARD" ]]; then
    printf 'ok    %-34s %s acts, the same ones war sign lists\n' "the board is the signing queue" "$C_BOARD"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s sign --list %s, console %s\n' "the board is the signing queue" "$C_LIST" "$C_BOARD"
    FAILED=$((FAILED + 1))
fi

# Every row is a human act through the agent's dialog. A row whose command
# could sign without `--ssh-sign` would be a row the tool could run itself.
C_JSON=$("$WAR" console --json 2>/dev/null)
C_VIA_SSH=$(CJSON="$C_JSON" python3 -c 'import json, os
b = json.loads(os.environ["CJSON"])["result"]
ok = bool(b["acts"]) and all(a["command"].endswith("--ssh-sign") for a in b["acts"])
print("yes" if ok else "no")')
if grep -q '"oh.war/console/v1"' <<< "$C_JSON" && [[ "$C_VIA_SSH" == "yes" ]]; then
    printf 'ok    %-34s schema named, every act via --ssh-sign\n' "the console projection"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "the console projection" "$(head -2 <<< "$C_JSON")"
    FAILED=$((FAILED + 1))
fi

# The presets come from the repository's config, not from the binary: with the
# block removed the screen offers none, and the signer is back to writing the
# reason by hand.
restore
python3 - <<'PY'
import re
p = "openwarrant.toml"
s = open(p).read()
open(p, "w").write(re.sub(r"\n\[\[sign\.preset\]\][^\[]*", "\n", s))
PY
assert_gone '[[sign.preset]]' openwarrant.toml
C_NONE=$("$WAR" console --json 2>/dev/null | python3 -c 'import json,sys; print(len(json.load(sys.stdin)["result"]["presets"]))')
restore
if [[ "$C_NONE" == "0" ]]; then
    printf 'ok    %-34s none offered once the config has none\n' "the presets are the repo's"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s presets with the block removed\n' "the presets are the repo's" "$C_NONE"
    FAILED=$((FAILED + 1))
fi

# The screen itself: a checklist, and an exit that writes nothing.
C_SCREEN=$(printf 'x\n' | "$WAR" console 2>&1)
if grep -q '\[ \]' <<< "$C_SCREEN" && grep -qi 'ssh' <<< "$C_SCREEN" \
    && git diff --quiet -- docs/ openwarrant.toml; then
    printf 'ok    %-34s a checklist, and exit wrote nothing\n' "the console screen"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "the console screen" "$(head -3 <<< "$C_SCREEN")"
    FAILED=$((FAILED + 1))
fi

# `war commit` names the record kinds it found and stages nothing. The mutation
# is a real edit to a tracked record, so the message has something to classify.
restore
printf '\n' >> docs/warrants/OW-WAR-0069/manifest.toml
assert_present 'local_alias' docs/warrants/OW-WAR-0069/manifest.toml
C_MSG=$("$WAR" commit 2>&1)
C_STAGED=$(git diff --cached --name-only | wc -l)
restore
if grep -qE '^(records|warrant|feat|docs|chore|sas|questions|conformance)' <<< "$C_MSG" \
    && grep -q 'OW-WAR-0069' <<< "$C_MSG" && [[ "$C_STAGED" == "0" ]]; then
    printf 'ok    %-34s drafted from the records, nothing staged\n' "war commit without --write"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s staged %s\n%s\n' "war commit without --write" "$C_STAGED" "$(head -3 <<< "$C_MSG")"
    FAILED=$((FAILED + 1))
fi
