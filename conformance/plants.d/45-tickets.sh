# shellcheck shell=bash
# OW-WAR-0147 — the ticket loop: create, ready, claim, done.
#
# A scratch program works tickets end to end with no signature and no human
# act (stdin closed, no terminal), and every refusal is by name: a second
# agent's claim, a done nobody claimed, an unknown item, a blocked item, a
# malformed checklist, a Warrant that names the working-form profile. The
# checklist file is the state: a hand edit is honoured and a write moves no
# byte outside the line it touches. Last, each command is timed on THIS
# repository's corpus.

echo "== tickets (OW-WAR-0147) =="
PLANT_ROOT=$(scratch_corpus TK)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
tk_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
tk_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
# Every call: no terminal to ask on, no inherited actor.
tkw() { env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK "$WAR" --root "$PLANT_ROOT" "$@" </dev/null; }
tkj() { tkw --json "$@" 2>/dev/null; }
# field <json> <python expression over v>
tk_field() { python3 -c "import json,sys; v=json.loads(sys.argv[1]); print($2)" "$1" 2>/dev/null; }
TK_CHECK_BEFORE=$("$WAR" --root "$PLANT_ROOT" sign --list 2>/dev/null | grep -cE '^  [A-Z]+-WAR-[0-9]{4} ')

# ---------------------------------------------------------------- OBL-001 --
# create → ready → claim → done, no signature, no human act.
TK_OUT=$(tkj create "Ship the parser" --item "Write the grammar" --item "Wire the CLI" --body "Parsing is slow today.")
TK=$(tk_field "$TK_OUT" 'v["result"]["id"]')
TK_I1=$(tk_field "$TK_OUT" 'v["result"]["items"][0]["id"]')
TK_I2=$(tk_field "$TK_OUT" 'v["result"]["items"][1]["id"]')
TK_READY=$(tkj ready)
TK_R1=$(tk_field "$TK_READY" '",".join(r["item"] for r in v["result"]["ready"])')
tkw claim "$TK/$TK_I1" >/dev/null 2>&1; TK_C=$?
tkw done "$TK/$TK_I1" --note "grammar written" >/dev/null 2>&1; TK_D=$?
tkw claim "$TK_I2" >/dev/null 2>&1 && tkw done "$TK_I2" >/dev/null 2>&1
TK_STATE=$(tk_field "$(tkj tickets)" 'v["result"]["tickets"][0]["state"]')
TK_CL="$PLANT_ROOT/docs/tickets/$TK/atoms/15-checklist.md"
TK_SIGNED=$(find "$PLANT_ROOT/docs/tickets" \( -name 'authorization*' -o -name 'resolution*' -o -name '*.sig' \) | wc -l)
TK_CHECK_AFTER=$("$WAR" --root "$PLANT_ROOT" sign --list 2>/dev/null | grep -cE '^  [A-Z]+-WAR-[0-9]{4} ')
if [[ "$TK" =~ ^t-[0-9a-f]{4,}$ && "$TK_R1" == "$TK_I1,$TK_I2" && $TK_C -eq 0 && $TK_D -eq 0 \
    && "$TK_STATE" == "done" && $TK_SIGNED -eq 0 && "$TK_CHECK_BEFORE" == "$TK_CHECK_AFTER" ]] \
    && grep -q "^- \[x\] Write the grammar ($TK_I1) — done by claude, [0-9-]*: grammar written$" "$TK_CL" \
    && grep -q '"ticket.item_done"' "$PLANT_ROOT/docs/tickets/$TK/journal.jsonl"; then
    tk_ok "create→ready→claim→done, unsigned" "$TK done; nothing signed, the signing queue unchanged ($TK_CHECK_AFTER)"
else
    tk_fail "create→ready→claim→done, unsigned" "id=$TK ready=$TK_R1 claim=$TK_C done=$TK_D state=$TK_STATE signed=$TK_SIGNED queue=$TK_CHECK_BEFORE/$TK_CHECK_AFTER"
fi

# ---------------------------------------------------------------- OBL-002 --
# A held item: a second agent is refused by name; done needs your own claim.
TK_OUT=$(tkj create "Second ticket" --item "Only item" --item "Other item")
TK2=$(tk_field "$TK_OUT" 'v["result"]["id"]')
TK2_I=$(tk_field "$TK_OUT" 'v["result"]["items"][0]["id"]')
TK2_J=$(tk_field "$TK_OUT" 'v["result"]["items"][1]["id"]')
tkw claim "$TK2/$TK2_I" --as alice >/dev/null 2>&1
TK_ERR=$(tkw claim "$TK2/$TK2_I" --as bob 2>&1 >/dev/null); TK_S=$?
if [[ $TK_S -eq 2 ]] && grep -q 'ticket.claimed-by-other' <<<"$TK_ERR" && grep -q 'claimed by alice since' <<<"$TK_ERR"; then
    tk_ok "a held item refuses a second claim" "bob refused, naming alice and since when"
else
    tk_fail "a held item refuses a second claim" "exit $TK_S: $TK_ERR"
fi
TK_ERR=$(tkw done "$TK2/$TK2_I" --as bob 2>&1 >/dev/null); TK_S=$?
TK_ERR2=$(tkw done "$TK2/$TK2_J" --as bob 2>&1 >/dev/null); TK_S2=$?
TK_ERR3=$(tkw done i-zzzz --as bob 2>&1 >/dev/null); TK_S3=$?
if [[ $TK_S -eq 2 && $TK_S2 -eq 2 && $TK_S3 -eq 2 ]] && grep -q 'claimed-by-other.*alice' <<<"$TK_ERR" \
    && grep -q 'ticket.not-claimed' <<<"$TK_ERR2" && grep -q 'ticket.unknown' <<<"$TK_ERR3" \
    && ! grep -q '\[x\]' "$PLANT_ROOT/docs/tickets/$TK2/atoms/15-checklist.md"; then
    tk_ok "done refuses unclaimed and unknown" "someone else's, unclaimed, unknown: three refusals, nothing ticked"
else
    tk_fail "done refuses unclaimed and unknown" "$TK_S/$TK_S2/$TK_S3: $TK_ERR | $TK_ERR2 | $TK_ERR3"
fi
# Concurrent claims from separate processes: exactly one wins.
TK_OUT=$(tkj create "Race" --item "Contended")
TK3=$(tk_field "$TK_OUT" 'v["result"]["id"]'); TK3_I=$(tk_field "$TK_OUT" 'v["result"]["items"][0]["id"]')
TK_PIDS=()
for n in 1 2 3 4 5 6 7 8; do
    tkw claim "$TK3/$TK3_I" --as "racer-$n" >/dev/null 2>&1 &
    TK_PIDS+=($!)
done
TK_WON=0
for pid in "${TK_PIDS[@]}"; do wait "$pid" && TK_WON=$((TK_WON + 1)); done
TK_JC=$(grep -c '"ticket.claimed"' "$PLANT_ROOT/docs/tickets/$TK3/journal.jsonl")
if [[ $TK_WON -eq 1 && $TK_JC -eq 1 ]]; then
    tk_ok "concurrent claims: exactly one wins" "8 processes, 1 claim, 1 journal event"
else
    tk_fail "concurrent claims: exactly one wins" "$TK_WON winners, $TK_JC claim events"
fi
# A stale claim: refused without --steal, taken with it, journalled.
printf '\n[tickets]\nclaim_ttl_minutes = 0\n' >> "$PLANT_ROOT/openwarrant.toml"
sleep 1
TK_ERR=$(tkw claim "$TK2/$TK2_I" --as bob 2>&1 >/dev/null); TK_S=$?
tkw claim "$TK2/$TK2_I" --as bob --steal >/dev/null 2>&1; TK_S2=$?
git -C "$PLANT_ROOT" checkout -q -- openwarrant.toml
TK_ERR3=$(tkw claim "$TK2/$TK2_I" --as carol --steal 2>&1 >/dev/null); TK_S3=$?
if [[ $TK_S -eq 2 && $TK_S2 -eq 0 && $TK_S3 -eq 2 ]] && grep -q -- '--steal. takes it' <<<"$TK_ERR" \
    && grep -q '"ticket.claim_stolen".*from\\":\\"alice' "$PLANT_ROOT/docs/tickets/$TK2/journal.jsonl" \
    && grep -q 'not stale' <<<"$TK_ERR3"; then
    tk_ok "a stale claim is stolen, journalled" "past the TTL: refused, then --steal took it from alice; fresh: refused"
else
    tk_fail "a stale claim is stolen, journalled" "$TK_S/$TK_S2/$TK_S3: $TK_ERR | $TK_ERR3"
fi

# ---------------------------------------------------------------- OBL-003 --
# The file is the state: a hand edit is honoured, and a write moves only its line.
TK_OUT=$(tkj create "Hand edited" --item "First" --item "Second" --item "Third")
TK4=$(tk_field "$TK_OUT" 'v["result"]["id"]')
TK4_1=$(tk_field "$TK_OUT" 'v["result"]["items"][0]["id"]'); TK4_3=$(tk_field "$TK_OUT" 'v["result"]["items"][2]["id"]')
TK_CL="$PLANT_ROOT/docs/tickets/$TK4/atoms/15-checklist.md"
python3 - "$TK_CL" "$TK4_1" <<'PY'
import sys
p, first = sys.argv[1], sys.argv[2]
lines = open(p).read().splitlines(keepends=True)
items = [l for l in lines if l.startswith("- [")]
head = [l for l in lines if not l.startswith("- [")]
# Reorder (third first), reword the first, add prose and a line with no id.
items = [items[2], items[0].replace("First", "First, reworded by a person"), items[1]]
open(p, "w").write("".join(head) + "Prose a person wrote.\n\n" + "".join(items) + "- [ ] Added by hand\n")
PY
cp "$TK_CL" "$TK_CL.before"
TK_READY=$(tkj ready)
TK_TXT=$(tk_field "$TK_READY" '"|".join(r["text"] for r in v["result"]["ready"] if r["ticket"]=="'"$TK4"'")')
tkw claim "$TK4/$TK4_3" >/dev/null 2>&1 && tkw done "$TK4/$TK4_3" >/dev/null 2>&1
TK_DIFF=$(python3 - "$TK_CL.before" "$TK_CL" "$TK4_3" <<'PY'
import sys
a = open(sys.argv[1]).read().splitlines(keepends=True)
b = open(sys.argv[2]).read().splitlines(keepends=True)
moved = [i for i, (x, y) in enumerate(zip(a, b)) if x != y]
# Exactly: the ticked line, and the unnamed line given an id; nothing else.
ok = len(a) == len(b) and len(moved) == 2 \
    and b[moved[0]].startswith("- [x] Third (" + sys.argv[3] + ")") \
    and b[moved[1]].startswith("- [ ] Added by hand (i-") and a[moved[1]] == "- [ ] Added by hand\n"
print("ok" if ok else f"moved {moved}")
PY
)
command rm -f "$TK_CL.before"
if [[ "$TK_TXT" == "Third|First, reworded by a person|Second|Added by hand" && "$TK_DIFF" == "ok" ]]; then
    tk_ok "a hand-edited checklist is honoured" "reordered, reworded, added: read as written; done moved 2 lines (its own, the new id)"
else
    tk_fail "a hand-edited checklist is honoured" "ready: $TK_TXT; diff: $TK_DIFF"
fi

# ---------------------------------------------------------------- OBL-004 --
# Blockers keep items out of `ready`, and a blocked claim is refused.
TK_OUT=$(tkj create "Ordered work" --item "Foundation")
TK5=$(tk_field "$TK_OUT" 'v["result"]["id"]'); TK5_F=$(tk_field "$TK_OUT" 'v["result"]["items"][0]["id"]')
TK5_W=$(tk_field "$(tkj add "$TK5" "Walls" --after "$TK5_F")" 'v["result"]["item"]')
TK5_R=$(tk_field "$(tkj add "$TK5" "Roof" --after "$TK5/$TK5_W")" 'v["result"]["item"]')
TK5_OTHER=$(tk_field "$(tkj create "Paint" --item "Paint the walls")" 'v["result"]["id"]')
TK5_SC=$(tk_field "$(tkj add "$TK5_OTHER" "Second coat" --after "$TK5")" 'v["result"]["item"]')
TK_IDS=$(tk_field "$(tkj ready)" '",".join(r["ticket"]+"/"+str(r["item"]) for r in v["result"]["ready"])')
TK_ERR=$(tkw claim "$TK5/$TK5_W" 2>&1 >/dev/null); TK_S=$?
tkw claim "$TK5/$TK5_F" >/dev/null 2>&1 && tkw done "$TK5/$TK5_F" >/dev/null 2>&1
TK_IDS2=$(tk_field "$(tkj ready)" '",".join(r["ticket"]+"/"+str(r["item"]) for r in v["result"]["ready"])')
if grep -q "$TK5/$TK5_F" <<<"$TK_IDS" && ! grep -q "$TK5_W\|$TK5_R\|$TK5_SC" <<<"$TK_IDS" && [[ -n "$TK5_SC" ]] \
    && [[ $TK_S -eq 2 ]] && grep -q "ticket.blocked.*waits on $TK5_F" <<<"$TK_ERR" \
    && grep -q "$TK5/$TK5_W" <<<"$TK_IDS2" && ! grep -q "$TK5_R" <<<"$TK_IDS2"; then
    tk_ok "blockers keep items out of ready" "walls waits on the foundation, roof on walls, a ticket on a ticket; claim refused"
else
    tk_fail "blockers keep items out of ready" "before: $TK_IDS; claim $TK_S: $TK_ERR; after: $TK_IDS2"
fi

# ---------------------------------------------------------------- OBL-005 --
# `war prime`: undone items only, notes, and old done tickets compacted.
tkw note "$TK5" "Chose concrete over timber: the site floods." >/dev/null 2>&1
TK_OUT=$(tkj create "An old finished job" --item "Old work")
TK6=$(tk_field "$TK_OUT" 'v["result"]["id"]')
sed -i 's/^- \[ \] Old work (\(i-[0-9a-f]*\))$/- [x] Old work (\1) — done by alice, 2020-01-01/' "$PLANT_ROOT/docs/tickets/$TK6/atoms/15-checklist.md"
TK_PRIME=$(tkw prime 2>/dev/null)
TK_PRIME_ONE=$(tkw prime "$TK5" 2>/dev/null)
TK_OLD=$(awk '/^## Done earlier/{f=1} f' <<<"$TK_PRIME" | grep -c "$TK6")
TK_RECENT=$(awk '/^## Done in the last/{f=1} /^## Done earlier/{f=0} f' <<<"$TK_PRIME" | grep -c "$TK")
if grep -q "^- \[ \] Walls ($TK5_W)" <<<"$TK_PRIME" && ! grep -q "Foundation" <<<"$TK_PRIME" \
    && ! grep -q '^- \[x\]' <<<"$TK_PRIME" && grep -q "concrete over timber" <<<"$TK_PRIME" \
    && [[ $TK_OLD -eq 1 && $TK_RECENT -ge 1 ]] \
    && grep -q "^- $TK6 — An old finished job (done 2020-01-01, 1 item(s))$" <<<"$TK_PRIME" \
    && grep -q "1 item(s) done" <<<"$TK_PRIME_ONE" && ! grep -q Foundation <<<"$TK_PRIME_ONE"; then
    tk_ok "prime: undone items, old done compacted" "remaining items and notes; $TK6 (2020) is one line; $TK recent"
else
    tk_fail "prime: undone items, old done compacted" "$(head -c 600 <<<"$TK_PRIME")"
fi

# ---------------------------------------------------------------- OBL-006 --
# `war check`: tickets add no authority finding; a malformed checklist is named.
TK_CHK=$(tkw check 2>&1); TK_S=$?
if [[ $TK_S -eq 0 ]] && grep -q '^PASS ticket.well-formed' <<<"$TK_CHK" \
    && ! grep -E '^ERROR' <<<"$TK_CHK" | grep -qi 'ticket'; then
    tk_ok "check: an unsigned ticket is sound" "exit 0; no ticket finding about authorization, evidence or verification"
else
    tk_fail "check: an unsigned ticket is sound" "exit $TK_S: $(grep -E '^(ERROR|WARN)' <<<"$TK_CHK" | head -3)"
fi
TK_CL="$PLANT_ROOT/docs/tickets/$TK2/atoms/15-checklist.md"
cp "$TK_CL" "$TK_CL.good"
printf -- '- [y] odd box\n- [ ] duplicate (%s)\n- [ ] waits on nothing (i-0bad, after i-nope)\n- [ ] across (i-0bae, after t-nope)\n' "$TK2_I" >> "$TK_CL"
TK_CHK=$(tkw check "$TK2" 2>&1); TK_S=$?
TK_ALL=$(tkw check 2>&1); TK_S2=$?
command mv "$TK_CL.good" "$TK_CL"
if [[ $TK_S -eq 2 && $TK_S2 -eq 2 ]] && grep -q '^ERROR ticket.checklist-malformed' <<<"$TK_CHK" \
    && grep -q '^ERROR ticket.item-duplicate' <<<"$TK_CHK" \
    && [[ $(grep -c '^ERROR ticket.blocker-unknown' <<<"$TK_CHK") -eq 2 ]] \
    && grep -q '^ERROR ticket.checklist-malformed' <<<"$TK_ALL"; then
    tk_ok "check: a malformed checklist is named" "malformed box, duplicate id, two unknown blockers; in the corpus check too"
else
    tk_fail "check: a malformed checklist is named" "exit $TK_S/$TK_S2: $(grep -E '^ERROR' <<<"$TK_CHK" | head -4)"
fi
# A Warrant cannot name the working-form profile: the contract layer refuses it.
mkdir -p "$PLANT_ROOT/profiles"
command cp profiles/ticket.toml "$PLANT_ROOT/profiles/"
TK_M=$(ls "$PLANT_ROOT"/docs/warrants/*/manifest.toml | head -1)
sed -i 's/^profile = "delivery"$/profile = "ticket"/' "$TK_M"
TK_CHK=$(tkw check 2>&1); TK_S=$?
git -C "$PLANT_ROOT" checkout -q -- "$TK_M"; command rm -rf "$PLANT_ROOT/profiles"
if [[ $TK_S -ne 0 ]] && grep -q 'profile ticket is a working form' <<<"$TK_CHK"; then
    tk_ok "a Warrant naming profile ticket" "refused: a working form is not a Warrant of the contract corpus"
else
    tk_fail "a Warrant naming profile ticket" "exit $TK_S: $(grep -E '^ERROR' <<<"$TK_CHK" | head -2)"
fi

# ---------------------------------------------------------------- OBL-007 --
# MCP: the same loop over `war mcp`; promote; --draft with and without a drafter.
TK_OUT=$(tkj create "Over MCP" --item "Via a tool")
TK7=$(tk_field "$TK_OUT" 'v["result"]["id"]'); TK7_I=$(tk_field "$TK_OUT" 'v["result"]["items"][0]["id"]')
TK_TX=$(mktemp)
{
    printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"plant","version":"0"}}}'
    printf '%s\n' '{"jsonrpc":"2.0","method":"notifications/initialized"}'
    printf '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"war_claim","arguments":{"target":"%s/%s","actor":"mcp-agent"}}}\n' "$TK7" "$TK7_I"
    printf '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"war_claim","arguments":{"target":"%s/%s","actor":"intruder"}}}\n' "$TK7" "$TK7_I"
    printf '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"war_done","arguments":{"target":"%s/%s","actor":"mcp-agent","note":"via mcp"}}}\n' "$TK7" "$TK7_I"
    printf '%s\n' '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"war_prime","arguments":{}}}'
} > "$TK_TX"
TK_MCP=$(cd "$PLANT_ROOT" && env -u OPENWARRANT_ACTOR python3 "$REPO_ROOT/conformance/fixtures/mcp/drive.py" "$REPO_ROOT/$WAR" "$TK_TX" 2>/dev/null)
command rm -f "$TK_TX"
if grep -q '"id":2.*"exit_code":0' <<<"$TK_MCP" && grep -q '"id":3.*claimed by mcp-agent' <<<"$TK_MCP" \
    && grep -q '"id":4.*"ticket_state":"done"' <<<"$TK_MCP" \
    && grep -q "Via a tool ($TK7_I) — done by mcp-agent" "$PLANT_ROOT/docs/tickets/$TK7/atoms/15-checklist.md"; then
    tk_ok "the loop over war mcp" "claim, a second agent refused by name, done, prime"
else
    tk_fail "the loop over war mcp" "$(head -c 400 <<<"$TK_MCP")"
fi
TK_P=$(tkj promote "$TK")
TK_W=$(tk_field "$TK_P" 'v["result"]["warrant"]')
TK_ERR=$(tkw promote "$TK" 2>&1 >/dev/null); TK_S=$?
if [[ "$TK_W" =~ ^TK-WAR-[0-9]{4}$ && -f "$PLANT_ROOT/docs/warrants/$TK_W/manifest.toml" && $TK_S -eq 2 ]] \
    && grep -q "promoted_to = \"$TK_W\"" "$PLANT_ROOT/docs/tickets/$TK/manifest.toml" \
    && grep -q "From ticket \[$TK\]" "$PLANT_ROOT/docs/warrants/$TK_W/atoms/10-intent.md" \
    && grep -q 'ticket.already-promoted' <<<"$TK_ERR" \
    && ! grep -q "^authorization" <(ls "$PLANT_ROOT/docs/warrants/$TK_W"); then
    tk_ok "promote drafts a Warrant, opt-in" "$TK → $TK_W, a draft awaiting its human; twice refused"
else
    tk_fail "promote drafts a Warrant, opt-in" "warrant=$TK_W exit=$TK_S: $TK_ERR"
fi
TK_N=$(ls "$PLANT_ROOT/docs/tickets" | wc -l)
TK_ERR=$(tkw create "Drafted work" --draft 2>&1 >/dev/null); TK_S=$?
TK_N2=$(ls "$PLANT_ROOT/docs/tickets" | wc -l)
cat > "$PLANT_ROOT/drafter.sh" <<'SH'
#!/usr/bin/env bash
cat >/dev/null
printf '%s' '{"api_version":"oh.war/draft-proposal/v2","proposed_identity":{"title":"x"},"operations":[{"op":"create_atom","role":"work_order","ordinal":40,"path":"40-work-order.md","body":"# Work Order\n\n## Deliverables\n\n1. `src/a.rs` parses\n2. `src/b.rs` prints\n\n## Rollback\n\n1. not an item\n"}],"risk_assessment":"low"}'
SH
chmod +x "$PLANT_ROOT/drafter.sh"
printf '\n[plan]\ndrafter_argv = ["./drafter.sh"]\n' >> "$PLANT_ROOT/openwarrant.toml"
git -C "$PLANT_ROOT" add drafter.sh openwarrant.toml >/dev/null 2>&1
TK_D=$(tkj create "Drafted work" --draft)
git -C "$PLANT_ROOT" rm -q --cached drafter.sh >/dev/null 2>&1; command rm -f "$PLANT_ROOT/drafter.sh"
git -C "$PLANT_ROOT" checkout -q HEAD -- openwarrant.toml
TK_DI=$(tk_field "$TK_D" '"|".join(i["text"] for i in v["result"]["items"])')
if [[ $TK_S -eq 2 && "$TK_N" == "$TK_N2" ]] && grep -q 'ticket.no-drafter' <<<"$TK_ERR" \
    && [[ "$TK_DI" == '`src/a.rs` parses|`src/b.rs` prints' ]]; then
    tk_ok "--draft: the drafter's items, or none" "no drafter: refused, nothing created; a drafter: its two deliverables"
else
    tk_fail "--draft: the drafter's items, or none" "exit $TK_S ($TK_N/$TK_N2): $TK_ERR; drafted: $TK_DI $(head -c 300 <<<"$TK_D")"
fi

# ---------------------------------------------------------------- OBL-008 --
# Timing, on THIS repository's corpus: every ticket command well under a
# second (asserted < 2000 ms, generous for a loaded machine). The ticket made
# here is removed afterwards; the corpus is untouched.
TK_REAL_DIR="$REPO_ROOT/docs/tickets"
TK_HAD_DIR=0; [[ -d "$TK_REAL_DIR" ]] && TK_HAD_DIR=1
TK_HAD_CLAIMS=0; [[ -d "$REPO_ROOT/.openwarrant/state/claims" ]] && TK_HAD_CLAIMS=1
rw() { env -u OPENWARRANT_ACTOR "$WAR" "$@" </dev/null; }
tk_ms() { local s e; s=$(date +%s%N); "$@" >/dev/null 2>&1; local rc=$?; e=$(date +%s%N); echo "$(( (e - s) / 1000000 )) $rc"; }
TK_T=$(rw --json create "Timing probe (plant 45)" --item one --item two 2>/dev/null)
TKR=$(tk_field "$TK_T" 'v["result"]["id"]'); TKR_I=$(tk_field "$TK_T" 'v["result"]["items"][0]["id"]')
TK_TIMES=""; TK_SLOW=""
for cmd in "create Timing-probe-2 --item x" "ready" "claim $TKR/$TKR_I" "done $TKR/$TKR_I --note timed" \
    "add $TKR three" "note $TKR timed" "prime" "prime $TKR" "show $TKR" "tickets" "check $TKR"; do
    # shellcheck disable=SC2086
    read -r ms rc < <(tk_ms rw $cmd)
    TK_TIMES="$TK_TIMES ${cmd%% *}=${ms}ms"
    [[ $rc -eq 0 && $ms -lt 2000 ]] || TK_SLOW="$TK_SLOW ${cmd%% *}(${ms}ms,exit $rc)"
done
# Remove every ticket this block made; nothing else under docs/tickets moves.
for d in "$TK_REAL_DIR"/t-*; do
    [[ -f "$d/manifest.toml" ]] && grep -q '^title = "Timing\(-probe-2\| probe (plant 45)\)"$' "$d/manifest.toml" && command rm -rf "$d"
done
[[ $TK_HAD_DIR -eq 0 ]] && rmdir "$TK_REAL_DIR" 2>/dev/null
[[ $TK_HAD_CLAIMS -eq 0 ]] && command rm -rf "$REPO_ROOT/.openwarrant/state/claims"
if [[ -n "$TKR" && -z "$TK_SLOW" ]]; then
    tk_ok "each ticket command < 2 s here" "$TK_TIMES"
else
    tk_fail "each ticket command < 2 s here" "slow or failed:$TK_SLOW; all:$TK_TIMES"
fi
