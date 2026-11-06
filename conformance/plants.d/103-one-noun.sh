# shellcheck shell=bash
# OW-WAR-0148 M10 — one id space, one list, and both encodings read byte
# for byte.
#
# A scratch program holds a Warrant in each encoding: the directory one
# `war init --program` writes, a light one from `war create`, and a light
# one written by hand as OW-WAR-0147 wrote it (none of M5's or M10's keys).
# Claims: `war warrants` lists all three, the light ones in result.tickets
# and the directory one in result.warrants, typed by its profile, with the
# phase its journal records; `war tickets` and `war ls` are the same list;
# `--type` takes a profile and `--state` a recorded phase; `war show` and
# `war status` route every id to its own encoding; and no read (the list,
# show, status, ready, next, model, check) moves a byte of any ticket or
# Warrant file. Refusals: an id that names nothing, by rule; a --type that
# is neither a type nor a profile (ticket.filter-type-unknown); a --state
# that is no state and no phase (ticket.filter-state-unknown).

echo "== one id space, one list, two encodings (M10) =="
PLANT_ROOT=$(scratch_corpus ON)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
on_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
on_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
onw() { env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK -u SSH_AGENT_PID "$WAR" --root "$PLANT_ROOT" "$@" </dev/null; }
onj() { onw --json "$@" 2>/dev/null; }
on_field() { python3 -c "import json,sys; v=json.loads(sys.argv[1]); print($2)" "$1" 2>/dev/null; }
# Every byte of every ticket and Warrant file, as one digest per file.
on_bytes() { (cd "$PLANT_ROOT" && find docs/tickets docs/warrants -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum); }

ON_T=$(on_field "$(onj create "Fix the parser" --item "Write the grammar")" 'v["result"]["id"]')
ON_OLD="$PLANT_ROOT/docs/tickets/t-0b1d"
mkdir -p "$ON_OLD/atoms"
cat > "$ON_OLD/manifest.toml" <<'TOML'
# A ticket (OW-WAR-0147): the working form of a delivery Warrant. The atoms beside
# this file are the ticket; `war show t-0b1d` renders them. Nothing here is signed.
schema = "oh.war/ticket/v1"
id = "t-0b1d"
uuid = "01a0f3c2-0000-7000-8000-000000000002"
title = "A ticket from before M10"
profile = "ticket"
priority = 1
created_at = "2026-09-26T10:00:00Z"
created_by = "someone"

[[atoms]]
ordinal = 10
role = "intent"
path = "atoms/10-intent.md"

[[atoms]]
ordinal = 15
role = "ticket.checklist"
path = "atoms/15-checklist.md"
TOML
printf '# A ticket from before M10\n\nWritten by hand.\n' > "$ON_OLD/atoms/10-intent.md"
printf '# Checklist\n\n- [ ] Keep working (i-0b01)\n' > "$ON_OLD/atoms/15-checklist.md"
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "two encodings" >/dev/null 2>&1
ON_LS=$(onj warrants)
ON_ALIAS=$(on_field "$ON_LS" '",".join(w["id"] for w in v["result"].get("warrants", []))')
if [[ -z "$ON_T" || "$ON_ALIAS" != ON-WAR-0001 ]]; then
    printf 'PLANT SETUP FAILED: light %s, directory %s\n' "$ON_T" "$ON_ALIAS" >&2
    exit 9
fi

# 1. One list: every encoding, each typed, the directory one's phase as
#    its journal records it; the two older names give the same answer.
ON_LIGHT=$(on_field "$ON_LS" '",".join(sorted(t["id"] for t in v["result"]["tickets"]))')
ON_DIR=$(on_field "$ON_LS" '"|".join(w["id"]+":"+w["profile"]+":"+w["encoding"]+":"+w["state"]+":"+w["provenance"] for w in v["result"]["warrants"])')
ON_RES=$(on_field "$ON_LS" 'json.dumps(v["result"], sort_keys=True)')
ON_RES_T=$(on_field "$(onj tickets)" 'json.dumps(v["result"], sort_keys=True)')
ON_RES_L=$(on_field "$(onj ls)" 'json.dumps(v["result"], sort_keys=True)')
ON_HUMAN=$(onw warrants 2>/dev/null)
ON_WANT_LIGHT=$(printf '%s\n' "$ON_T" t-0b1d | LC_ALL=C sort | paste -sd, -)
if [[ "$ON_LIGHT" == "$ON_WANT_LIGHT" && "$ON_DIR" == "ON-WAR-0001:delivery:directory:draft:recorded" \
    && -n "$ON_RES" && "$ON_RES" == "$ON_RES_T" && "$ON_RES" == "$ON_RES_L" ]] \
    && grep -qE '^ON-WAR-0001 +draft .*\[delivery\]$' <<<"$ON_HUMAN" \
    && grep -qE '^t-0b1d +open ' <<<"$ON_HUMAN"; then
    on_ok "one list, every encoding" "$ON_LIGHT light, ON-WAR-0001 directory (delivery, draft as recorded); tickets and ls give the same result"
else
    on_fail "one list, every encoding" "light '$ON_LIGHT' dir '$ON_DIR'; same as tickets/ls: $([[ "$ON_RES" == "$ON_RES_T" && "$ON_RES" == "$ON_RES_L" ]] && echo yes || echo no); $(head -c 300 <<<"$ON_HUMAN")"
fi

# 2. Filters read every encoding: a profile as a type, a recorded phase as
#    a state, the fixed three across both.
on_set() { on_field "$(onj warrants "$@")" '",".join(sorted([t["id"] for t in v["result"]["tickets"]] + [w["id"] for w in v["result"].get("warrants", [])]))'; }
ON_BAD=""
[[ "$(on_set --type delivery)" == "ON-WAR-0001" ]] || ON_BAD="$ON_BAD type-delivery:'$(on_set --type delivery)'"
[[ "$(on_set --type ticket)" == "$ON_WANT_LIGHT" ]] || ON_BAD="$ON_BAD type-ticket:'$(on_set --type ticket)'"
[[ "$(on_set --state draft)" == "ON-WAR-0001" ]] || ON_BAD="$ON_BAD state-draft:'$(on_set --state draft)'"
[[ "$(on_set --state open)" == "$(printf '%s\n' ON-WAR-0001 "$ON_T" t-0b1d | LC_ALL=C sort | paste -sd, -)" ]] \
    || ON_BAD="$ON_BAD state-open:'$(on_set --state open)'"
[[ "$(on_set --search adopt)" == "ON-WAR-0001" ]] || ON_BAD="$ON_BAD search:'$(on_set --search adopt)'"
ON_ERR1=$(onw warrants --type story 2>&1 >/dev/null); ON_S1=$?
ON_ERR2=$(onw warrants --state blocked 2>&1 >/dev/null); ON_S2=$?
if [[ -z "$ON_BAD" && $ON_S1 -eq 2 && $ON_S2 -eq 2 ]] && grep -q 'ticket.filter-type-unknown' <<<"$ON_ERR1" \
    && grep -q 'delivery' <<<"$ON_ERR1" && grep -q 'ticket.filter-state-unknown' <<<"$ON_ERR2" \
    && grep -q 'draft, proposed, authorized' <<<"$ON_ERR2"; then
    on_ok "filters read every encoding" "--type delivery/ticket, --state draft/open, --search; story and blocked refused by rule"
else
    on_fail "filters read every encoding" "mismatch:$ON_BAD; type $ON_S1 '$ON_ERR1'; state $ON_S2 '$ON_ERR2'"
fi

# 3. One id space: show and status route every id to its encoding; an id
#    that names nothing is refused, by rule, in both.
ON_SH_T=$(on_field "$(onj show "$ON_T")" 'v["result"]["schema"]')
ON_SH_OLD=$(on_field "$(onj show t-0b1d)" 'v["result"]["ticket"]["title"]')
ON_SH_W=$(on_field "$(onj show ON-WAR-0001)" 'v["result"]["alias"]')
ON_ST_T=$(on_field "$(onj status "$ON_T")" 'v["command"] + ":" + v["result"]["schema"]')
ON_ST_W=$(on_field "$(onj status ON-WAR-0001)" 'v["result"]["view"]')
ON_ERR3=$(onw show t-zzzz 2>&1 >/dev/null); ON_S3=$?
ON_ERR4=$(onw status t-zzzz 2>&1 >/dev/null); ON_S4=$?
ON_ERR5=$(onw show ON-WAR-0999 2>&1 >/dev/null); ON_S5=$?
if [[ "$ON_SH_T" == oh.war/ticket-show/v1 && "$ON_SH_OLD" == "A ticket from before M10" \
    && "$ON_SH_W" == ON-WAR-0001 && "$ON_ST_T" == "status:oh.war/ticket-show/v1" && "$ON_ST_W" == status \
    && $ON_S3 -eq 2 && $ON_S4 -eq 2 && $ON_S5 -ne 0 ]] && grep -q 'ticket.unknown' <<<"$ON_ERR3" \
    && grep -q 'ticket.unknown' <<<"$ON_ERR4" && grep -q 'ON-WAR-0999' <<<"$ON_ERR5"; then
    on_ok "show and status route every id" "$ON_T, t-0b1d (hand-written), ON-WAR-0001; t-zzzz refused (ticket.unknown), ON-WAR-0999 refused"
else
    on_fail "show and status route every id" "show $ON_SH_T/$ON_SH_OLD/$ON_SH_W status $ON_ST_T/$ON_ST_W; refusals $ON_S3 '$ON_ERR3' $ON_S4 $ON_S5 '$ON_ERR5'"
fi

# 4. Reading moves no byte: every read command over both encodings, then
#    every ticket and Warrant file compared with its digest before. The
#    refusal: a write in the same place is seen (so the comparison is not
#    vacuous).
ON_B0=$(on_bytes)
for args in "warrants" "tickets --search parser" "ls --state draft" "show $ON_T" "show t-0b1d" \
    "show ON-WAR-0001" "status $ON_T" "status ON-WAR-0001" "status" "ready" "next" "model" \
    "check $ON_T" "check t-0b1d" "check"; do
    # shellcheck disable=SC2086
    onw $args >/dev/null 2>&1
done
ON_B1=$(on_bytes)
ON_GIT=$(git -C "$PLANT_ROOT" status --porcelain -- docs/tickets docs/warrants)
onw note t-0b1d "a write, to see the comparison move" >/dev/null 2>&1
ON_B2=$(on_bytes)
if [[ -n "$ON_B0" && "$ON_B0" == "$ON_B1" && -z "$ON_GIT" && "$ON_B2" != "$ON_B1" ]]; then
    on_ok "reads move no byte" "$(wc -l <<<"$ON_B0") ticket and Warrant files byte-identical after 15 reads; a note moves them"
else
    on_fail "reads move no byte" "changed: $(diff <(echo "$ON_B0") <(echo "$ON_B1") | head -3 | tr '\n' ' ') git: $ON_GIT; write seen: $([[ "$ON_B2" != "$ON_B1" ]] && echo yes || echo no)"
fi

corpus_gone "$PLANT_ROOT"
unset ON_T ON_OLD ON_LS ON_ALIAS ON_LIGHT ON_DIR ON_RES ON_RES_T ON_RES_L ON_HUMAN ON_WANT_LIGHT ON_BAD \
    ON_ERR1 ON_ERR2 ON_ERR3 ON_ERR4 ON_ERR5 ON_S1 ON_S2 ON_S3 ON_S4 ON_S5 ON_SH_T ON_SH_OLD ON_SH_W \
    ON_ST_T ON_ST_W ON_B0 ON_B1 ON_B2 ON_GIT
