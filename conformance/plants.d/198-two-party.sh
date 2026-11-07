# shellcheck shell=bash
# A breaking change takes both owners (t-3c16c W6; docs/INTERFACES.md).
#
# A scratch repository, two throwaway keys each in its own throwaway ssh-agent,
# and a register naming two people. Nothing here reaches the caller's agent: the
# plant's own environment has no SSH_AUTH_SOCK, and each signature names the
# socket it may use. Each claim beside the input it refuses:
#   an agent cannot close a breaking change, and is told what a person signs;
#   one party's signature is not enough;
#   the same person cannot sign for both parties;
#   a key that is not the signer's does not make that person's signature;
#   an agent signing is refused by the register;
#   both owners, each with their own key, close it.

echo "== two-party acceptance of a breaking change =="
TW_ROOT=$(scratch_corpus TW)
[[ -d "${TW_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
TW_TMP=$(mktemp -d)
tw_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
tw_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
# No key is reachable from this: signing goes through tw_sign alone.
tww() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID OPENWARRANT_ACTOR=hpr-agent "$WAR" --root "$TW_ROOT" "$@"; }
# The change request's state as `view tickets` reads it: `war done` hands an
# Order to a person and exits 0, so "still open" is read here.
tw_state() { tww --json view tickets 2>/dev/null | python3 -c 'import json,sys
print(next((t["state"] for t in json.load(sys.stdin)["result"]["tickets"] if t["id"]==sys.argv[1]), "missing"))' "$TW_T"; }
tw_sign() { local sock=$1 who=$2 what=$3; env -u SSH_AGENT_PID SSH_AUTH_SOCK="$sock" OPENWARRANT_ACTOR=hpr-agent "$WAR" --root "$TW_ROOT" sign "$what" --ssh-sign --as "$who" 2>&1; }
tw_id() { sed -n 's/.*"id": "\(t-[0-9a-f]*\)".*/\1/p' | head -1; }

tw_agent() {  # <name>: a key and an agent holding it, and nothing else
    ssh-keygen -q -t ed25519 -N "" -C "$1" -f "$TW_TMP/id_$1" || return 1
    ssh-agent -a "$TW_TMP/$1.sock" -s > "$TW_TMP/$1.env" || return 1
    SSH_AUTH_SOCK="$TW_TMP/$1.sock" ssh-add -q "$TW_TMP/id_$1" 2>/dev/null || return 1
    [[ $(SSH_AUTH_SOCK="$TW_TMP/$1.sock" ssh-add -l 2>/dev/null | grep -c .) == 1 ]]
}
tw_agents_gone() {
    local f pid
    for f in "$TW_TMP"/*.env; do
        [[ -f "$f" ]] || continue
        pid=$(sed -n 's/^SSH_AGENT_PID=\([0-9]*\);.*/\1/p' "$f")
        [[ -n "$pid" ]] && kill "$pid" 2>/dev/null
    done
}
tw_agent alice && tw_agent bob || { tw_agents_gone; printf 'PLANT SETUP FAILED: the throwaway agents\n' >&2; exit 9; }
tww plan types add interfaces >/dev/null 2>&1 || { tw_agents_gone; printf 'PLANT SETUP FAILED: types add\n' >&2; exit 9; }
mkdir -p "$TW_ROOT/docs/records/interfaces" "$TW_ROOT/docs/authority"
cat > "$TW_ROOT/docs/records/interfaces/physics.md" <<'DOC'
---
format: openwarrant/1.0
profile: interface
version: 1.0.0
owner: hpr
parties: hpr, wmd
summary: "What the engine relies on"
---
# Physics interface

## CL-step · clause — A step advances the world by exactly dt
status: agreed
verify: inspection

A step advances simulated time by `dt`.
DOC
for tw_who in alice bob; do
    printf '%s namespaces="oh.war/response,oh.war/dsse" %s\n' "$tw_who" "$(cut -d' ' -f1,2 "$TW_TMP/id_$tw_who.pub")"
done >> "$TW_ROOT/docs/authority/allowed_signers"
cat >> "$TW_ROOT/docs/authority/roles.toml" <<'ROLES'

[[assignment]]
actor = "Alice Owner"
actor_kind = "human"
roles = ["authorizer", "resolver"]
assigned_by = "conformance/plants.d/198-two-party.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while this plant runs."
ssh_principal = "alice"

[[assignment]]
actor = "Bob Owner"
actor_kind = "human"
roles = ["authorizer", "resolver"]
assigned_by = "conformance/plants.d/198-two-party.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "Exists only while this plant runs."
ssh_principal = "bob"
ROLES

TW_T=$(tww --json create "Change what a step means" --change breaking --clause CL-step 2>/dev/null | tw_id)
[[ -n "$TW_T" ]] || { tw_agents_gone; printf 'PLANT SETUP FAILED: no change request\n' >&2; exit 9; }
TW_SHOW=$(tww --json view show "$TW_T" 2>/dev/null)
TW_ITEMS=$(python3 -c 'import json,sys; print(" ".join(i["id"] for i in json.load(sys.stdin)["result"]["items"]))' <<<"$TW_SHOW")
read -r TW_I1 TW_I2 <<<"$TW_ITEMS"
if [[ -n "$TW_I1" && -n "$TW_I2" ]] && grep -q 'accept: hpr' <<<"$TW_SHOW" && grep -q 'accept: wmd' <<<"$TW_SHOW" \
    && grep -q '"level": "order"' <<<"$(tww --json view tickets 2>/dev/null)"; then
    tw_ok "a breaking change is an Order" "one item per party: accept: hpr, accept: wmd"
else
    tw_fail "a breaking change is an Order" "$TW_ITEMS: $(head -c 300 <<<"$TW_SHOW")"
fi

tww start "$TW_T" >/dev/null 2>&1
TW_DONE=$(tww done "$TW_T" 2>&1); TW_DONE_RC=$?
if [[ "$(tw_state)" != done ]] && grep -q 'waits on a person' <<<"$TW_DONE" && grep -q 'war sign' <<<"$TW_DONE"; then
    tw_ok "an agent cannot close it" "handed to a person, still open; it names what they sign"
else
    tw_fail "an agent cannot close it" "$TW_DONE_RC: $(head -c 300 <<<"$TW_DONE")"
fi

TW_AGENT=$(tw_sign "$TW_TMP/alice.sock" hpr-agent "$TW_T/$TW_I1"); TW_AGENT_RC=$?
TW_WRONG=$(tw_sign "$TW_TMP/alice.sock" "Bob Owner" "$TW_T/$TW_I1"); TW_WRONG_RC=$?
TW_ANY=$(find "$TW_ROOT/docs/authority/responses" -name "$TW_T*" 2>/dev/null | grep -c .)
if [[ $TW_AGENT_RC -ne 0 && $TW_WRONG_RC -ne 0 && "$TW_ANY" == 0 ]] \
    && grep -q 'sign.no-eligible-signer' <<<"$TW_AGENT" && grep -q 'sign.ssh-refused' <<<"$TW_WRONG"; then
    tw_ok "the wrong signer is refused" "an agent by the register; another's key; nothing recorded"
else
    tw_fail "the wrong signer is refused" "$TW_AGENT_RC $TW_WRONG_RC files $TW_ANY: $TW_AGENT | $TW_WRONG"
fi

TW_ONE=$(tw_sign "$TW_TMP/alice.sock" "Alice Owner" "$TW_T/$TW_I1"); TW_ONE_RC=$?
TW_HALF=$(tww done "$TW_T" 2>&1); TW_HALF_RC=$?
if [[ $TW_ONE_RC -eq 0 && "$(tw_state)" != done ]] && grep -q 'signed off by Alice Owner' <<<"$TW_ONE" \
    && grep -q "war sign $TW_T/$TW_I2" <<<"$TW_HALF"; then
    tw_ok "one party is not enough" "Alice signed; the change stays open"
else
    tw_fail "one party is not enough" "$TW_ONE_RC $TW_HALF_RC: $TW_ONE | $TW_HALF"
fi

TW_BOTH=$(tw_sign "$TW_TMP/alice.sock" "Alice Owner" "$TW_T/$TW_I2"); TW_BOTH_RC=$?
if [[ $TW_BOTH_RC -ne 0 ]] && grep -q 'sign.one-party' <<<"$TW_BOTH"; then
    tw_ok "one person cannot be both parties" "sign.one-party; nothing recorded"
else
    tw_fail "one person cannot be both parties" "$TW_BOTH_RC: $TW_BOTH"
fi

TW_TWO=$(tw_sign "$TW_TMP/bob.sock" "Bob Owner" "$TW_T/$TW_I2"); TW_TWO_RC=$?
TW_END=$(tww done "$TW_T" 2>&1); TW_END_RC=$?
TW_FINAL=$(tww --json view tickets 2>/dev/null | python3 -c 'import json,sys
t=[x for x in json.load(sys.stdin)["result"]["tickets"] if x["id"]==sys.argv[1]][0]
print(t["state"], t["done"], t["total"])' "$TW_T")
if [[ $TW_TWO_RC -eq 0 ]] && grep -q 'signed off by Bob Owner' <<<"$TW_TWO" && [[ "$TW_FINAL" == "done 2 2" ]]; then
    tw_ok "both owners close it" "two people, two keys, both signatures verify"
else
    tw_fail "both owners close it" "$TW_TWO_RC $TW_END_RC $TW_FINAL: $TW_TWO | $TW_END"
fi

tw_agents_gone
/usr/bin/rm -rf "$TW_TMP"
unset TW_ROOT TW_TMP TW_T TW_SHOW TW_ITEMS TW_I1 TW_I2 TW_DONE TW_DONE_RC TW_AGENT TW_AGENT_RC TW_WRONG TW_WRONG_RC
unset TW_ANY TW_ONE TW_ONE_RC TW_HALF TW_HALF_RC TW_BOTH TW_BOTH_RC TW_TWO TW_TWO_RC TW_END TW_END_RC TW_FINAL tw_who
