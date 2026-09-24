# shellcheck shell=bash
# OW-WAR-0121 — storage: atomic writes, crash recovery, named crash damage.
#
# A scratch program, a key generated here, and a throwaway ssh-agent holding
# only that key. Nothing here is the owner's key or this repository's corpus:
# the one thing read from this checkout is the source of the six routed
# writers, for OBL-005's grep, and that is read, never written.
#
# The crash is the debug build's fault hook (`OPENWARRANT_FAULT`, compiled out
# of release builds): `after-temp` exits after the temp file is written and
# before its rename; `pause-before-rename:<ms>` sleeps there.
# `OPENWARRANT_FAULT_FILE` names the one target it applies to, so the stop
# lands on the record under test and not on the first write of the act.
#
# Every claim is paired with the control seen refusing, and every refusal is
# matched by the rule that fired, not by a non-zero exit.

echo "== storage (OW-WAR-0121) =="
PLANT_ROOT=$(scratch_corpus ST)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
ST_TMP=$(mktemp -d)
ST_A="ST-WAR-0001"
ST_DIR="$PLANT_ROOT/docs/warrants/$ST_A"
ST_REL="docs/warrants/$ST_A"
[[ -d "$ST_DIR" ]] || { printf 'PLANT SETUP FAILED: the scratch program has no %s\n' "$ST_A" >&2; exit 9; }

ssh-keygen -q -t ed25519 -N "" -C plant -f "$ST_TMP/id_plant"
ST_PUB=$(cut -d' ' -f1,2 "$ST_TMP/id_plant.pub")
printf 'plant namespaces="oh.war/response,oh.war/dsse" %s\n' "$ST_PUB" > "$PLANT_ROOT/docs/authority/allowed_signers"
cat > "$PLANT_ROOT/docs/authority/roles.toml" <<'ROLES'
[[assignment]]
actor = "Plant Signer"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "conformance/plants.d/53-storage.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while the storage plants run."
ssh_principal = "plant"
ROLES
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "register" >/dev/null 2>&1

ST_OLD_SOCK=${SSH_AUTH_SOCK:-}
ST_OLD_PID=${SSH_AGENT_PID:-}
eval "$(ssh-agent -s > "$ST_TMP/agent.env"; cat "$ST_TMP/agent.env")" >/dev/null
ssh-add -q "$ST_TMP/id_plant" 2>/dev/null
# The agent must hold this key and nothing else: a signature by any other key
# would be a signature this plant has no business making.
ST_KEYS=$(ssh-add -l 2>/dev/null)
ST_FP=$(ssh-keygen -lf "$ST_TMP/id_plant.pub" | awk '{print $2}')
if [[ $(grep -c . <<<"$ST_KEYS") -ne 1 ]] || ! grep -qF -- "$ST_FP" <<<"$ST_KEYS"; then
    printf 'PLANT SETUP FAILED: the throwaway agent holds more than the plant key:\n%s\n' "$ST_KEYS" >&2
    ssh-agent -k >/dev/null 2>&1
    exit 9
fi

st_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
st_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
st_war()  { "$WAR" --root "$PLANT_ROOT" "$@"; }
st_sign() { st_war sign "$ST_A" --ssh-sign --as "Plant Signer" </dev/null; }
st_temps() { (cd "$PLANT_ROOT" && find docs -name '*.war-tmp' -type f | sort); }
st_sha()  { if [[ -e "$1" || -L "$1" ]]; then sha256sum < "$1" | cut -d' ' -f1; else echo absent; fi; }
# Every file under docs/ but the atomic writer's own temp files, by content.
st_tree() { (cd "$PLANT_ROOT" && find docs -type f ! -name '*.war-tmp' -print0 | sort -z | xargs -0 sha256sum); }
st_errors() { grep -E '^(ERROR|WARN)' <<<"$1" | head -3 | tr '\n' '|'; }

# ── Controls: a clean program reports no stray temp file ─────────────────────
ST_OUT=$(st_war check 2>&1)
if grep -q 'storage.stray-temp' <<<"$ST_OUT"; then
    st_fail "a clean program has no stray temp" "$(grep 'storage.stray-temp' <<<"$ST_OUT" | head -1)"
else
    st_ok "a clean program has no stray temp" "storage.stray-temp silent"
fi

# ── OBL-001: a crash mid-write never leaves a partial record ────────────────
# (a) During `war sign --ssh-sign`, stopped between the temp file and the
#     rename of authorization.toml.
ST_BEFORE=$(st_sha "$ST_DIR/authorization.toml")
ST_OUT=$(OPENWARRANT_FAULT=after-temp OPENWARRANT_FAULT_FILE=authorization.toml st_sign 2>&1); ST_STATUS=$?
ST_AFTER=$(st_sha "$ST_DIR/authorization.toml")
ST_TEMP=$(st_temps)
if [[ $ST_STATUS -eq 86 ]] && grep -q 'OPENWARRANT_FAULT=after-temp' <<<"$ST_OUT" \
    && [[ "$ST_BEFORE" == "$ST_AFTER" ]] && [[ "$ST_TEMP" == "$ST_REL"/.authorization.toml.*.war-tmp ]]; then
    st_ok "crash in sign: no partial record" "authorization.toml $ST_AFTER, temp $(basename "$ST_TEMP")"
else
    st_fail "crash in sign: no partial record" "exit $ST_STATUS, before $ST_BEFORE after $ST_AFTER, temps [$ST_TEMP]: $(st_errors "$ST_OUT")"
fi
ST_OUT=$(st_war check 2>&1)
if [[ -n "$ST_TEMP" ]] && grep -F "$ST_TEMP" <<<"$ST_OUT" | grep -qE 'WARN +storage\.stray-temp' \
    && grep 'storage.stray-temp' <<<"$ST_OUT" | grep -qF "replace $ST_REL/authorization.toml"; then
    st_ok "the stopped sign is named" "storage.stray-temp → $ST_REL/authorization.toml"
else
    st_fail "the stopped sign is named" "no storage.stray-temp naming [$ST_TEMP]: $(grep 'storage\.' <<<"$ST_OUT" | head -2 | tr '\n' '|')"
fi
# The paired refusal: removing the temp file silences the rule, so it fires
# on the file and not on something else about the stopped act.
[[ -n "$ST_TEMP" ]] && rm -f "$PLANT_ROOT/$ST_TEMP"
if grep -q 'storage.stray-temp' <<<"$(st_war check 2>&1)"; then
    st_fail "no temp file, no stray-temp" "storage.stray-temp still reported"
else
    st_ok "no temp file, no stray-temp" "storage.stray-temp silent"
fi
corpus_reset "$PLANT_ROOT"

# (b) During `war compile`, with an atom edited so the compile has something
#     to write. The control first: an unfaulted compile of the same edit does
#     move generated files, so the faulted run's silence is the hook's doing.
printf '\nA line the storage plant adds so the views must change.\n' >> "$ST_DIR/atoms/10-intent.md"
st_war compile >/dev/null 2>&1
if git -C "$PLANT_ROOT" diff --quiet -- docs/warrants/"$ST_A"/generated; then
    st_fail "control: the edit moves the views" "an unfaulted compile wrote nothing; the plant would prove nothing"
else
    st_ok "control: the edit moves the views" "$(git -C "$PLANT_ROOT" diff --name-only | grep -c generated) generated file(s) change"
fi
git -C "$PLANT_ROOT" checkout -q -- docs
printf '\nA line the storage plant adds so the views must change.\n' >> "$ST_DIR/atoms/10-intent.md"
ST_FILES=$(st_tree)
ST_OUT=$(OPENWARRANT_FAULT=after-temp st_war compile 2>&1); ST_STATUS=$?
ST_TEMP=$(st_temps)
if [[ $ST_STATUS -eq 86 ]] && [[ "$ST_FILES" == "$(st_tree)" ]] && [[ $(grep -c . <<<"$ST_TEMP") -eq 1 ]]; then
    st_ok "crash in compile: views byte-identical" "every file under docs/ unchanged; one temp $(basename "$ST_TEMP")"
else
    st_fail "crash in compile: views byte-identical" "exit $ST_STATUS, moved: $(diff <(echo "$ST_FILES") <(st_tree) | grep '^[<>]' | awk '{print $1, $3}' | tr '\n' ' ') temps [$ST_TEMP]"
fi
ST_OUT=$(st_war check --generated 2>&1)
ST_RECORD="${ST_TEMP%.*.*.war-tmp}"; ST_RECORD="$(dirname "$ST_RECORD")/$(basename "$ST_RECORD" | sed 's/^\.//')"
if [[ -n "$ST_TEMP" ]] && grep -F "$ST_TEMP" <<<"$ST_OUT" | grep -qE 'WARN +storage\.stray-temp' \
    && grep 'storage.stray-temp' <<<"$ST_OUT" | grep -qF "replace $ST_RECORD," \
    && [[ "$ST_FILES" == "$(st_tree)" ]]; then
    st_ok "the stopped compile is named" "storage.stray-temp → $ST_RECORD; committed views intact"
else
    st_fail "the stopped compile is named" "[$ST_TEMP → $ST_RECORD]: $(grep -E 'storage\.|generated\.' <<<"$ST_OUT" | head -2 | tr '\n' '|')"
fi
# No partial file: every generated file still parses as what it claims to be.
ST_BAD=""
while IFS= read -r f; do
    case "$f" in
        *.json) python3 -c 'import json,sys; json.load(open(sys.argv[1]))' "$PLANT_ROOT/$f" 2>/dev/null || ST_BAD="$ST_BAD $f" ;;
    esac
done < <(cd "$PLANT_ROOT" && find docs -path '*generated*' -type f ! -name '*.war-tmp')
if [[ -z "$ST_BAD" ]] && ! grep -qiE 'truncat|unexpected end|EOF while parsing' <<<"$ST_OUT"; then
    st_ok "no partial generated file" "every generated JSON parses; check --generated names no truncation"
else
    st_fail "no partial generated file" "unparseable:$ST_BAD"
fi
corpus_reset "$PLANT_ROOT"

# ── OBL-002: a record changed since it was read is not overwritten ──────────
# The act reads authorization.toml as absent, then pauses between its temp
# file and the rename; the plant writes its own bytes there meanwhile.
ST_PLANTED='# written by 53-storage.sh while war sign was paused
planted = true
'
( OPENWARRANT_FAULT=pause-before-rename:5000 OPENWARRANT_FAULT_FILE=authorization.toml st_sign > "$ST_TMP/race.out" 2>&1; echo $? > "$ST_TMP/race.status" ) &
ST_BG=$!
ST_SEEN=""
for _ in $(seq 1 300); do
    ST_SEEN=$(st_temps | grep '/\.authorization\.toml\.' || true)
    [[ -n "$ST_SEEN" ]] && break
    sleep 0.1
done
if [[ -n "$ST_SEEN" ]]; then
    printf '%s' "$ST_PLANTED" > "$ST_DIR/authorization.toml"
fi
wait "$ST_BG"
ST_STATUS=$(cat "$ST_TMP/race.status" 2>/dev/null || echo "?")
ST_OUT=$(cat "$ST_TMP/race.out" 2>/dev/null)
if [[ -n "$ST_SEEN" ]] && [[ "$ST_STATUS" != 0 ]] && grep -qE 'ERROR +storage\.prestate-moved' <<<"$ST_OUT" \
    && [[ "$(cat "$ST_DIR/authorization.toml")" == "${ST_PLANTED%$'\n'}" ]] && [[ -z "$(st_temps)" ]]; then
    st_ok "a moved record is not overwritten" "storage.prestate-moved (exit $ST_STATUS); the plant's bytes stand"
else
    st_fail "a moved record is not overwritten" "paused=[${ST_SEEN:+yes}] exit $ST_STATUS, temps [$(st_temps)]: $(st_errors "$ST_OUT")"
fi
corpus_reset "$PLANT_ROOT"

# The pair: the same pause with nobody writing records the authorization. The
# refusal above is the concurrent write's, not the pause's.
ST_OUT=$(OPENWARRANT_FAULT=pause-before-rename:200 OPENWARRANT_FAULT_FILE=authorization.toml st_sign 2>&1); ST_STATUS=$?
if [[ $ST_STATUS -eq 0 ]] && grep -q 'authorize.recorded' <<<"$ST_OUT" && ! grep -q 'storage\.' <<<"$ST_OUT" \
    && grep -q '^schema = "oh.war/authorization/v1"' "$ST_DIR/authorization.toml" && [[ -z "$(st_temps)" ]]; then
    st_ok "an unmoved record is written" "authorize.recorded through the same pause"
else
    st_fail "an unmoved record is written" "exit $ST_STATUS: $(st_errors "$ST_OUT")"
fi
# Kept: the journal plants below want a Warrant with more than one event.
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "signed through the pause" >/dev/null 2>&1

# ── OBL-003: a symlinked target is refused ──────────────────────────────────
# A second Warrant, unsigned. The pair first: signed with nothing in the way,
# its record is written as a file.
st_war new "A second Warrant for the symlink plant" >/dev/null 2>&1
st_war compile >/dev/null 2>&1
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "a second Warrant" >/dev/null 2>&1
ST_B="ST-WAR-0002"
ST_BDIR="$PLANT_ROOT/docs/warrants/$ST_B"
ST_OUT=$(st_war sign "$ST_B" --ssh-sign --as "Plant Signer" </dev/null 2>&1); ST_STATUS=$?
if [[ $ST_STATUS -eq 0 ]] && grep -q 'authorize.recorded' <<<"$ST_OUT" && [[ -f "$ST_BDIR/authorization.toml" && ! -L "$ST_BDIR/authorization.toml" ]]; then
    st_ok "an unlinked record is written" "authorize.recorded"
else
    st_fail "an unlinked record is written" "exit $ST_STATUS: $(st_errors "$ST_OUT")"
fi
# The attack: authorization.toml is a link to a file outside the program. The
# file is that same record, unsigned once the response is gone, so every
# reader before the write takes it for a record awaiting its signature — the
# case a signature is most likely to be asked for. A link to anything else is
# refused earlier, as unparseable.
cp "$ST_BDIR/authorization.toml" "$ST_TMP/outside.toml"
corpus_reset "$PLANT_ROOT"
ST_OUTSIDE=$(st_sha "$ST_TMP/outside.toml")
ln -s "$ST_TMP/outside.toml" "$ST_BDIR/authorization.toml"
ST_OUT=$(st_war sign "$ST_B" --ssh-sign --as "Plant Signer" </dev/null 2>&1); ST_STATUS=$?
if [[ $ST_STATUS -ne 0 ]] && grep -qE 'ERROR +storage\.symlink-target' <<<"$ST_OUT" \
    && [[ "$(st_sha "$ST_TMP/outside.toml")" == "$ST_OUTSIDE" ]] && [[ -L "$ST_BDIR/authorization.toml" ]] \
    && [[ "$(readlink "$ST_BDIR/authorization.toml")" == "$ST_TMP/outside.toml" ]] && [[ -z "$(st_temps)" ]]; then
    st_ok "a symlinked record is refused" "storage.symlink-target (exit $ST_STATUS); the linked file untouched"
else
    st_fail "a symlinked record is refused" "exit $ST_STATUS, outside $(st_sha "$ST_TMP/outside.toml") (was $ST_OUTSIDE): $(st_errors "$ST_OUT")$(grep -m1 '^error' <<<"$ST_OUT")"
fi
rm -f "$ST_BDIR/authorization.toml"
corpus_reset "$PLANT_ROOT"
# The same helper under `war compile`: a generated view linked outside.
printf 'outside the program; must never be written\n' > "$ST_TMP/outside.md"
ST_OUTSIDE=$(st_sha "$ST_TMP/outside.md")
rm -f "$ST_DIR/generated/WAR.md"
ln -s "$ST_TMP/outside.md" "$ST_DIR/generated/WAR.md"
ST_OUT=$(st_war compile 2>&1); ST_STATUS=$?
if [[ $ST_STATUS -ne 0 ]] && grep -q 'storage.symlink-target' <<<"$ST_OUT" && [[ "$(st_sha "$ST_TMP/outside.md")" == "$ST_OUTSIDE" ]] \
    && [[ -L "$ST_DIR/generated/WAR.md" ]] && [[ -z "$(st_temps)" ]]; then
    st_ok "a symlinked view is refused" "storage.symlink-target (exit $ST_STATUS); the linked file untouched"
else
    st_fail "a symlinked view is refused" "exit $ST_STATUS, outside $(st_sha "$ST_TMP/outside.md"): $(head -c 300 <<<"$ST_OUT" | tr '\n' '|')"
fi
rm -f "$ST_DIR/generated/WAR.md"
corpus_reset "$PLANT_ROOT"

# ── OBL-004: a torn journal tail is named, and only it ──────────────────────
ST_J="$ST_DIR/journal.jsonl"
ST_JREL="$ST_REL/journal.jsonl"
ST_JLINES=$(grep -c . "$ST_J")
ST_JSHA=$(st_sha "$ST_J")
cp "$ST_J" "$ST_TMP/orig.jsonl"
ST_JSIZE=$(stat -c %s "$ST_J")
ST_LAST=$(tail -n 1 "$ST_J")
ST_HALF=${ST_LAST:0:$((${#ST_LAST} / 2))}
if [[ "$ST_JLINES" -lt 2 ]]; then
    printf 'PLANT SETUP FAILED: %s has %s event(s); the journal plants need two\n' "$ST_JREL" "$ST_JLINES" >&2
    exit 9
fi
ST_OUT=$(st_war check 2>&1); ST_BASE_STATUS=$?

# (a) A partial event appended with no newline.
printf '%s' "$ST_HALF" >> "$ST_J"
ST_OUT=$(st_war check 2>&1); ST_STATUS=$?
ST_TRUNC=$(grep -E 'ERROR +journal\.torn-tail' <<<"$ST_OUT" | grep -oE 'truncate -s [0-9]+ [^` ]+' | head -1)
if [[ $ST_STATUS -ne 0 ]] && grep -qE "ERROR +journal\.torn-tail.*offset $ST_JSIZE" <<<"$ST_OUT" \
    && [[ "$ST_TRUNC" == "truncate -s $ST_JSIZE $ST_JREL" ]] && ! grep -q 'journal.malformed' <<<"$ST_OUT"; then
    st_ok "a torn tail is named" "journal.torn-tail at offset $ST_JSIZE, not journal.malformed"
else
    st_fail "a torn tail is named" "exit $ST_STATUS, truncation [$ST_TRUNC]: $(grep 'journal\.' <<<"$ST_OUT" | head -2 | tr '\n' '|')"
fi
# The tool names the truncation and runs none: check left the bytes as they were.
if [[ $(stat -c %s "$ST_J") -eq $((ST_JSIZE + ${#ST_HALF})) ]]; then
    st_ok "war truncates nothing" "the torn bytes are still there after check"
else
    st_fail "war truncates nothing" "the journal is $(stat -c %s "$ST_J") bytes; war changed it"
fi
# A journal writer reading that journal is refused, by the same name, and
# writes nothing.
ST_OUT=$(st_war journal "$ST_A" --backfill 2>&1); ST_STATUS=$?
if [[ $ST_STATUS -ne 0 ]] && grep -q 'journal.torn-tail' <<<"$ST_OUT" && ! grep -q 'journal.not-empty' <<<"$ST_OUT" \
    && [[ $(stat -c %s "$ST_J") -eq $((ST_JSIZE + ${#ST_HALF})) ]]; then
    st_ok "a writer on a torn journal" "refused naming journal.torn-tail"
else
    st_fail "a writer on a torn journal" "exit $ST_STATUS: $(head -c 300 <<<"$ST_OUT" | tr '\n' '|')"
fi
# The named truncation, run by the plant (a person's act), restores a passing
# check with every parsed event still there.
if [[ -n "$ST_TRUNC" ]]; then (cd "$PLANT_ROOT" && eval "$ST_TRUNC"); fi
ST_OUT=$(st_war check 2>&1); ST_STATUS=$?
if [[ -n "$ST_TRUNC" ]] && [[ $ST_STATUS -eq $ST_BASE_STATUS ]] && [[ "$(st_sha "$ST_J")" == "$ST_JSHA" ]] \
    && ! grep -qE 'journal\.(torn-tail|malformed)' <<<"$ST_OUT" && grep -qE "PASS +journal\.append-only.*$ST_A: $ST_JLINES event" <<<"$ST_OUT"; then
    st_ok "the named truncation repairs it" "$ST_JLINES event(s) kept; check exit $ST_STATUS as before"
else
    st_fail "the named truncation repairs it" "exit $ST_STATUS (baseline $ST_BASE_STATUS), sha $(st_sha "$ST_J"): $(grep 'journal\.' <<<"$ST_OUT" | head -2 | tr '\n' '|')"
fi

# (b) The same bytes between two good lines: malformed, not torn.
{ head -n 1 "$ST_J"; printf '%s\n' "$ST_HALF"; tail -n +2 "$ST_J"; } > "$ST_TMP/middle.jsonl"
cp "$ST_TMP/middle.jsonl" "$ST_J"
ST_OUT=$(st_war check 2>&1); ST_STATUS=$?
if [[ $ST_STATUS -ne 0 ]] && grep -qE 'ERROR +journal\.malformed' <<<"$ST_OUT" && ! grep -q 'journal.torn-tail' <<<"$ST_OUT"; then
    st_ok "a bad middle line is not torn" "journal.malformed, not journal.torn-tail"
else
    st_fail "a bad middle line is not torn" "exit $ST_STATUS: $(grep 'journal\.' <<<"$ST_OUT" | head -2 | tr '\n' '|')"
fi
# (c) The same bytes as a last line WITH its newline: written whole, so
#     malformed too.
{ cat "$ST_TMP/orig.jsonl"; printf '%s\n' "$ST_HALF"; } > "$ST_J"
ST_OUT=$(st_war check 2>&1)
if grep -qE 'ERROR +journal\.malformed' <<<"$ST_OUT" && ! grep -q 'journal.torn-tail' <<<"$ST_OUT"; then
    st_ok "a terminated bad tail is not torn" "journal.malformed, not journal.torn-tail"
else
    st_fail "a terminated bad tail is not torn" "$(grep 'journal\.' <<<"$ST_OUT" | head -2 | tr '\n' '|')"
fi
corpus_reset "$PLANT_ROOT"

# ── OBL-005: every authority-bearing writer goes through the helper ─────────
# Read from this checkout, never written: the six routed files, each up to its
# `#[cfg(test)]` module, must hold no `fs::write(`. The grep is then shown to
# catch one planted back, in a copy.
st_writes() { # dir -> "file:line" for every non-test fs::write( in the six
    local f
    for f in authorize.rs resolution_cmd.rs verify.rs gate_cmd.rs compile.rs sign.rs; do
        awk -v f="$f" '/^#\[cfg\(test\)\]/ { exit } /fs::write\(/ { print f ":" NR }' "$1/$f"
    done
}
ST_SRC="$REPO_ROOT/crates/openwarrant-cli/src"
ST_FOUND=$(st_writes "$ST_SRC")
if [[ -z "$ST_FOUND" ]]; then
    st_ok "no fs::write in the routed writers" "six files, non-test code"
else
    st_fail "no fs::write in the routed writers" "$(tr '\n' ' ' <<<"$ST_FOUND")"
fi
mkdir -p "$ST_TMP/src"
for f in authorize.rs resolution_cmd.rs verify.rs gate_cmd.rs compile.rs sign.rs; do cp "$ST_SRC/$f" "$ST_TMP/src/$f"; done
sed -i '0,/crate::compile::atomic::write_if(&out, body, &before)/s//std::fs::write(\&out, body)/' "$ST_TMP/src/resolution_cmd.rs"
assert_present 'std::fs::write(&out, body)' "$ST_TMP/src/resolution_cmd.rs"
ST_FOUND=$(st_writes "$ST_TMP/src")
if grep -q '^resolution_cmd.rs:' <<<"$ST_FOUND"; then
    st_ok "a planted fs::write is caught" "$(tr '\n' ' ' <<<"$ST_FOUND")"
else
    st_fail "a planted fs::write is caught" "the grep missed the planted write"
fi

# ── R-003: the fault hook is absent from a release build ────────────────────
# Only a release binary built from this source can say so; an older one lacks
# the hook for the wrong reason. No such binary is UNKNOWN, not a pass.
ST_REL_BIN="$REPO_ROOT/target/release/war"
if [[ -x "$ST_REL_BIN" && "$ST_REL_BIN" -nt "$ST_SRC/atomic.rs" ]]; then
    if grep -qa 'OPENWARRANT_FAULT' "$ST_REL_BIN"; then
        st_fail "release build has no fault hook" "OPENWARRANT_FAULT is in $ST_REL_BIN"
    else
        st_ok "release build has no fault hook" "OPENWARRANT_FAULT absent from the release binary"
    fi
else
    printf 'UNKNOWN %-32s no release binary newer than atomic.rs; not counted\n' "release build has no fault hook"
fi

ssh-agent -k >/dev/null 2>&1 || true
if [[ -n "$ST_OLD_SOCK" ]]; then export SSH_AUTH_SOCK="$ST_OLD_SOCK"; else unset SSH_AUTH_SOCK; fi
if [[ -n "$ST_OLD_PID" ]]; then export SSH_AGENT_PID="$ST_OLD_PID"; else unset SSH_AGENT_PID; fi
command rm -rf "$ST_TMP"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
