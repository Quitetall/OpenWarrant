# shellcheck shell=bash
# SPDX-License-Identifier: Apache-2.0
# Context presets and `war apply` under FORMAT-1.0 §10.4 and §10.5 (draft 3),
# on a scratch program (FA); nothing in this repository moves.
#
# Accepted: the human package of a Format 1.0 ticket, edited between its
# markers and given back, replaces that block's text; a box ticked in it is
# done by the applier (`— done by <actor>, <date>`, a `work.item_done` in the
# journal); `--preset resume` names a live claim's holder; the header names
# its preset and walk.
# Refused, by name: the same package given back after the document moved
# (apply.stale), with the document byte-for-byte as it was; a block whose
# heading names another record (apply.identity); an unknown preset
# (context.preset-unknown), with nothing printed.

echo "== context presets and war apply (FORMAT-1.0 §10.4, §10.5) =="
FA_ROOT=$(scratch_corpus FA)
[[ -d "${FA_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
FA_TMP=$(mktemp -d)
SCRATCH_CORPORA+=("$FA_ROOT" "$FA_TMP")
FA_WAR="$REPO_ROOT/${WAR#./}"
fa_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
fa_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
fa_war() { env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK -u SSH_AGENT_PID "$FA_WAR" --root "$FA_ROOT" "$@" </dev/null; }
FA_DOC="$FA_ROOT/docs/tickets/t-fa01.md"
mkdir -p "$FA_ROOT/docs/tickets"
printf -- '---\nformat: openwarrant/1.0\nprofile: ticket\nalias: t-fa01\nuuid: 01a0d342-ee5e-7b71-9623-f2d8cd5afa01\nsummary: Plant package work.\n---\n# Plant package work\n\n## Intent\nThe plant edits this sentence.\n\n## Checklist\n- [ ] ticked by hand (i-fa11)\n- [ ] left open (i-fa12)\n' > "$FA_DOC"
git -C "$FA_ROOT" add -A >/dev/null 2>&1 && git -C "$FA_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "a Format 1.0 ticket" >/dev/null 2>&1
FA_AT=2026-10-14T10:00:00Z

# --- a package edited and given back ------------------------------------------------
fa_war context t-fa01 --preset human >"$FA_TMP/pkg.md" 2>"$FA_TMP/err"
cp "$FA_TMP/pkg.md" "$FA_TMP/old.md"
python3 - "$FA_TMP/pkg.md" <<'PY'
import sys
p = sys.argv[1]; s = open(p).read()
s = s.replace("The plant edits this sentence.", "The plant edited this sentence.", 1)
s = s.replace("- [ ] ticked by hand (i-fa11)", "- [x] ticked by hand (i-fa11)", 1)
open(p, "w").write(s)
PY
FA_OUT=$(fa_war apply "$FA_TMP/pkg.md" --as plant-alice --at "$FA_AT" 2>&1); FA_RC=$?
FA_J="$FA_ROOT/docs/tickets/t-fa01/journal.jsonl"
if [[ $FA_RC -eq 0 ]] && grep -qx 'The plant edited this sentence.' "$FA_DOC" \
    && grep -qx -- '- \[x\] ticked by hand (i-fa11) — done by plant-alice, 2026-10-14' "$FA_DOC" \
    && grep -qx -- '- \[ \] left open (i-fa12)' "$FA_DOC" \
    && grep -q '"type":"work.item_done"' "$FA_J" 2>/dev/null; then
    fa_ok "apply writes the edited package" "text replaced; box done by plant-alice; work.item_done journalled"
else
    fa_fail "apply writes the edited package" "exit $FA_RC: $FA_OUT; $(head -c 300 "$FA_TMP/err")"
fi

# --- the package made before that write: stale -------------------------------------
python3 - "$FA_TMP/old.md" <<'PY'
import sys
p = sys.argv[1]; s = open(p).read()
s = s.replace("The plant edits this sentence.", "A stale package rewrote this.", 1)
open(p, "w").write(s)
PY
FA_BEFORE=$(sha256sum "$FA_DOC" | cut -c1-64)
FA_OUT=$(fa_war apply "$FA_TMP/old.md" --as plant-bob --at "$FA_AT" 2>&1); FA_RC=$?
if [[ $FA_RC -ne 0 && "$FA_OUT" == *"apply.stale"* ]] \
    && [[ "$(sha256sum "$FA_DOC" | cut -c1-64)" == "$FA_BEFORE" ]]; then
    fa_ok "a stale package is refused" "apply.stale; the document is unchanged"
else
    fa_fail "a stale package is refused" "exit $FA_RC: $FA_OUT"
fi

# --- a block that names another record: identity ------------------------------------
fa_war context t-fa01 --preset human >"$FA_TMP/id.md" 2>/dev/null
python3 - "$FA_TMP/id.md" <<'PY'
import sys
p = sys.argv[1]; s = open(p).read()
s = s.replace("## Intent\n", "## Purpose\n", 1)
open(p, "w").write(s)
PY
FA_OUT=$(fa_war apply "$FA_TMP/id.md" --as plant-bob --at "$FA_AT" 2>&1); FA_RC=$?
if [[ $FA_RC -ne 0 && "$FA_OUT" == *"apply.identity"* ]] \
    && [[ "$(sha256sum "$FA_DOC" | cut -c1-64)" == "$FA_BEFORE" ]]; then
    fa_ok "a renamed block is refused" "apply.identity; the document is unchanged"
else
    fa_fail "a renamed block is refused" "exit $FA_RC: $FA_OUT"
fi

# --- presets: resume names the holder; an unknown preset prints nothing --------------
fa_war claim t-fa01/i-fa12 --as plant-carol >/dev/null 2>&1
FA_RES=$(fa_war context t-fa01 --preset resume 2>&1)
if [[ "$FA_RES" == *"preset resume (walk core)"* && "$FA_RES" == *"state claimed · holder plant-carol · lease "* ]]; then
    fa_ok "resume shows the live claim" "holder plant-carol, with its lease"
else
    fa_fail "resume shows the live claim" "$(printf '%s' "$FA_RES" | head -c 400)"
fi
FA_BAD=$(fa_war context t-fa01 --preset everything 2>"$FA_TMP/bad.err"); FA_RC=$?
if [[ $FA_RC -ne 0 && -z "$FA_BAD" ]] && grep -q 'context.preset-unknown' "$FA_TMP/bad.err"; then
    fa_ok "an unknown preset is refused" "context.preset-unknown; nothing printed"
else
    fa_fail "an unknown preset is refused" "exit $FA_RC: $FA_BAD $(head -c 200 "$FA_TMP/bad.err")"
fi
