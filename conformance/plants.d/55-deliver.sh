# shellcheck shell=bash
# t-39dc — `war deliver`: a deliverable is declared delivered by command.
#
# Delivery is §37.2 provenance on each declared deliverable — the sha256 of
# the file now, how it was made, the build of `war` that recorded it — and
# `content_addressed = true`. It was written by a throwaway script, and a
# wave of Warrants reached the blind verifier with none. Accepted: one
# deliverable recorded by id, its sibling left alone, every comment kept,
# `war check` clean; again, and nothing moves. Refused, each with nothing
# written: a missing file (and the whole command with it), an id the Warrant
# does not declare, and a path a LATER authorized Warrant governs
# (OW-ADR-0021) — while that later Warrant records the same path. The
# resolved-Warrant refusal is in 55-prepare.sh, which resolves one.
#
# A scratch program (DL), a key generated here, and a throwaway ssh-agent
# asserted to hold only that key, killed before any `war deliver` runs.
# Nothing here reaches the owner's agent or this repository's corpus.

echo "== war deliver (t-39dc) =="
PLANT_ROOT=$(scratch_corpus DL)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
DL_TMP=$(mktemp -d)
DL_A=DL-WAR-0001
DL_B=DL-WAR-0002
DL_M="$PLANT_ROOT/docs/warrants/$DL_A/deliverables.toml"

if ! WAR="$REPO_ROOT/${WAR#./}" D="$PLANT_ROOT" T="$DL_TMP" A="$DL_A" \
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID bash -euo pipefail > "$DL_TMP/setup.log" 2>&1 <<'SETUP'
cd "$D"
g() { git -c user.email=plant@invalid -c user.name=plant "$@"; }
mkdir -p src docs/memo
printf 'alpha\n' > src/a.txt
printf '# A memo\n' > docs/memo/b.md
cat > "docs/warrants/$A/deliverables.toml" <<'DELIV'
schema = "oh.war/deliverables/v1"

# Declared by the performer. This comment is part of the record.

[[deliverable]]
id = "D-001"
title = "a source file"
kind = "file"
target_ref = "src/a.txt"
required = true
content_addressed = false   # delivered by `war deliver`
provenance_required = true
obligation_refs = ["OBL-001"]

# D-002's own comment.
[[deliverable]]
id = "D-002"
title = "a memo"
kind = "document"
target_ref = "docs/memo/b.md"
required = true
content_addressed = false
provenance_required = false
obligation_refs = ["OBL-001"]

[[deliverable]]
id = "D-003"
title = "a file nobody wrote"
kind = "file"
target_ref = "src/missing.txt"
required = false
content_addressed = false
provenance_required = false
obligation_refs = ["OBL-001"]
DELIV
"$WAR" --root . new "A later Warrant over the same file" >/dev/null
B=$(ls docs/warrants | grep -v "^$A$" | grep WAR | head -1)
test -n "$B"
cat > "docs/warrants/$B/deliverables.toml" <<'DELIV'
schema = "oh.war/deliverables/v1"

[[deliverable]]
id = "D-001"
title = "the same source file, later"
kind = "file"
target_ref = "src/a.txt"
required = true
content_addressed = false
provenance_required = false
DELIV
ssh-keygen -q -t ed25519 -N "" -C plant -f "$T/id_plant"
printf 'plant namespaces="oh.war/response,oh.war/dsse" %s\n' "$(cut -d' ' -f1,2 "$T/id_plant.pub")" > docs/authority/allowed_signers
cat > docs/authority/roles.toml <<'ROLES'
[[assignment]]
actor = "Plant Signer"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "conformance/plants.d/55-deliver.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while this plant runs."
ssh_principal = "plant"
ROLES
"$WAR" --root . compile >/dev/null
g add -A; g commit -qm "two Warrants declaring one file"
test -z "$(git status --porcelain)"
SETUP
then
    printf 'PLANT SETUP FAILED: could not build the scratch corpus:\n' >&2
    tail -15 "$DL_TMP/setup.log" >&2
    exit 9
fi

dl_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
dl_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
dl_war()  { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID "$WAR" --root "$PLANT_ROOT" "$@" </dev/null; }
dl_sha()  { sha256sum "$1" | cut -d' ' -f1; }
dl_line() { grep -m1 -E 'deliver\.|ERROR|error' <<<"$1"; }
# The recorded provenance of one deliverable, as `war pins` reads it.
dl_pin()  { dl_war --json pins | python3 -c "import json,sys; ps=[p for p in json.load(sys.stdin)['result']['pins'] if p['warrant']==sys.argv[1] and p['deliverable_id']==sys.argv[2]]; p=ps[0] if ps else {}; print(p.get('content_addressed'), p.get('content_digest'))" "$1" "$2"; }

# 1. Refused: a missing file refuses the whole command, and nothing is written.
DL_BEFORE=$(dl_sha "$DL_M")
DL_OUT=$(dl_war deliver "$DL_A" 2>&1); DL_STATUS=$?
if [[ $DL_STATUS -eq 2 ]] && grep -q 'deliver.missing .*D-003 → src/missing.txt' <<<"$DL_OUT" \
    && [[ "$(dl_sha "$DL_M")" == "$DL_BEFORE" ]]; then
    dl_ok "a missing file refuses all" "deliver.missing names D-003; deliverables.toml unchanged"
else
    dl_fail "a missing file refuses all" "exit $DL_STATUS: $(dl_line "$DL_OUT")"
fi

# ... and an id the Warrant does not declare.
DL_OUT=$(dl_war deliver "$DL_A" D-009 2>&1); DL_STATUS=$?
if [[ $DL_STATUS -eq 2 ]] && grep -q 'deliver.unknown-id .*D-009' <<<"$DL_OUT" \
    && [[ "$(dl_sha "$DL_M")" == "$DL_BEFORE" ]]; then
    dl_ok "an undeclared id is refused" "deliver.unknown-id names D-009; nothing written"
else
    dl_fail "an undeclared id is refused" "exit $DL_STATUS: $(dl_line "$DL_OUT")"
fi

# 2. Accepted: D-001 by id — its digest is the file's, D-002 is untouched,
#    every comment survives, and `war check` finds nothing to say about it.
DL_OUT=$(dl_war deliver "$DL_A" D-001 2>&1); DL_STATUS=$?
DL_WANT="True sha256:$(dl_sha "$PLANT_ROOT/src/a.txt")"
DL_CHECK=$(dl_war check "$DL_A" 2>&1)
if [[ $DL_STATUS -eq 0 ]] && grep -q 'deliver.recorded .*D-001 → src/a.txt' <<<"$DL_OUT" \
    && [[ "$(dl_pin "$DL_A" D-001)" == "$DL_WANT" ]] \
    && [[ "$(dl_pin "$DL_A" D-002)" == "False None" ]] \
    && grep -q '^# Declared by the performer. This comment is part of the record.$' "$DL_M" \
    && grep -q '^content_addressed = true   # delivered by `war deliver`$' "$DL_M" \
    && grep -q "^# D-002's own comment.$" "$DL_M" \
    && grep -q '^tool_or_runtime_identity = "war ' "$DL_M" \
    && ! grep -qE '^ERROR +deliverable\.' <<<"$DL_CHECK"; then
    dl_ok "one deliverable is delivered" "D-001 content addressed at its bytes; D-002 untouched; comments kept"
else
    dl_fail "one deliverable is delivered" "exit $DL_STATUS: $(dl_line "$DL_OUT") pin=$(dl_pin "$DL_A" D-001)"
fi

# 3. Idempotent: D-001 is current, D-002 is recorded; once more and nothing moves.
dl_war deliver "$DL_A" D-001 D-002 >/dev/null 2>&1
DL_BEFORE=$(dl_sha "$DL_M")
DL_OUT=$(dl_war deliver "$DL_A" D-001 D-002 2>&1); DL_STATUS=$?
if [[ $DL_STATUS -eq 0 && "$(dl_sha "$DL_M")" == "$DL_BEFORE" ]] \
    && [[ $(grep -c 'deliver.current' <<<"$DL_OUT") -eq 2 ]] && ! grep -q 'deliver.recorded' <<<"$DL_OUT"; then
    dl_ok "a second delivery writes nothing" "deliver.current twice; deliverables.toml byte-identical"
else
    dl_fail "a second delivery writes nothing" "exit $DL_STATUS: $(dl_line "$DL_OUT")"
fi
git -C "$PLANT_ROOT" checkout -q -- "docs/warrants/$DL_A/deliverables.toml"

# 4. Not recorded: a path a LATER authorized Warrant governs is named and
#    left alone (its pin is historical, OW-ADR-0021). Authorize A, then B,
#    one batch each, a second apart, with a throwaway agent killed after.
DL_OLD_SOCK=${SSH_AUTH_SOCK:-}
unset SSH_AUTH_SOCK SSH_AGENT_PID
eval "$(ssh-agent -s)" >/dev/null
ssh-add -q "$DL_TMP/id_plant" 2>/dev/null
DL_KEY=$(ssh-keygen -lf "$DL_TMP/id_plant.pub" | awk '{print $2}')
DL_HELD=$(ssh-add -l 2>/dev/null)
if [[ $(grep -c . <<<"$DL_HELD") -ne 1 || "$(awk '{print $2}' <<<"$DL_HELD")" != "$DL_KEY" ]]; then
    printf 'PLANT SETUP FAILED: the agent holds another key\n' >&2
    ssh-agent -k >/dev/null 2>&1
    exit 9
fi
DL_SIGN1=$("$WAR" --root "$PLANT_ROOT" sign --batch "$DL_A" --ssh-sign --as "Plant Signer" </dev/null 2>&1)
sleep 1
DL_SIGN2=$("$WAR" --root "$PLANT_ROOT" sign --batch "$DL_B" --ssh-sign --as "Plant Signer" </dev/null 2>&1)
ssh-agent -k >/dev/null 2>&1 || true
if [[ -n "$DL_OLD_SOCK" ]]; then export SSH_AUTH_SOCK="$DL_OLD_SOCK"; else unset SSH_AUTH_SOCK; fi
unset SSH_AGENT_PID
if [[ ! -f "$PLANT_ROOT/docs/warrants/$DL_A/authorization.toml" || ! -f "$PLANT_ROOT/docs/warrants/$DL_B/authorization.toml" ]]; then
    printf 'PLANT SETUP FAILED: the two authorizations were not recorded:\n%s\n%s\n' "$(tail -3 <<<"$DL_SIGN1")" "$(tail -3 <<<"$DL_SIGN2")" >&2
    exit 9
fi
DL_BEFORE=$(dl_sha "$DL_M")
DL_OUT=$(dl_war deliver "$DL_A" D-001 2>&1); DL_STATUS=$?
if [[ $DL_STATUS -eq 0 ]] && grep -qE "^PASS +deliver\.governed .*D-001 → src/a.txt is governed by $DL_B/D-001" <<<"$DL_OUT" \
    && ! grep -q 'deliver.recorded' <<<"$DL_OUT" && [[ "$(dl_sha "$DL_M")" == "$DL_BEFORE" ]]; then
    dl_ok "a later Warrant's path is not recorded" "deliver.governed names $DL_B/D-001; nothing written"
else
    dl_fail "a later Warrant's path is not recorded" "exit $DL_STATUS: $(dl_line "$DL_OUT")"
fi
# ... and the Warrant that governs it records it (the control).
DL_OUT=$(dl_war deliver "$DL_B" D-001 2>&1); DL_STATUS=$?
if [[ $DL_STATUS -eq 0 ]] && [[ "$(dl_pin "$DL_B" D-001)" == "$DL_WANT" ]]; then
    dl_ok "the governing Warrant delivers it" "$DL_B/D-001 content addressed at src/a.txt's bytes"
else
    dl_fail "the governing Warrant delivers it" "exit $DL_STATUS: $(dl_line "$DL_OUT")"
fi

command rm -rf "$DL_TMP"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
