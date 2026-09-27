# shellcheck shell=bash
# OW-WAR-0116 — what the blind verifier could not see in 63-webui.sh, observed.
#
# 63-webui.sh is held byte-for-byte (OW-WAR-0139 OBL-005, and the quiet-grep
# exemption names its sha), so these checks live beside it (t-1de2):
#   OBL-001: GET /api/act is the act's state — it needs the token, answers the
#            state, and starts nothing; every method but GET and POST is 405.
#   OBL-002: a record change is seen within four seconds; /api/progress on this
#            corpus is `war roadmap`'s phases with `war status`'s rungs, and
#            nothing under webui/ reads view.json.
#   OBL-003: the Queue row carries its dry-run verdict beside its allowlist
#            id; an id not on the allowlist is 403 and starts no process (no
#            act line, no child of the server; a started act is the control
#            that the same observation sees one); the act's command, whole,
#            from the server's log; and one act at a time (of two POSTs sent
#            at once, one is 202 and one 409).
# Every server runs with SSH_AUTH_SOCK unset: no act can reach an ssh agent.

echo "== web ui, what the verifier asked (OW-WAR-0116) =="
PLANT_ROOT=$(scratch_corpus WV)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
cp "$PLANT_ROOT/docs/authority/roles.toml.example" "$PLANT_ROOT/docs/authority/roles.toml"
cp "$PLANT_ROOT/docs/authority/allowed_signers.example" "$PLANT_ROOT/docs/authority/allowed_signers"
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "register" >/dev/null 2>&1

WU_SRC=crates/openwarrant-cli/src/webui
WU_OUT=$(mktemp); WU_ERR=$(mktemp)
env -u SSH_AUTH_SOCK "$WAR" --root "$PLANT_ROOT" ui --port 0 --as your-name-here >"$WU_OUT" 2>"$WU_ERR" &
WU_PID=$!
for _ in $(seq 1 60); do grep -q 'http://' "$WU_OUT" && break; sleep 0.25; done
WU_URL=$(grep -o 'http://[^ ]*' "$WU_OUT" | head -1)
WU_HOST=${WU_URL#http://}; WU_HOST=${WU_HOST%%/*}
WU_TOK=${WU_URL#*#t=}; WU_TOK=${WU_TOK%%&*}
WU_B="http://$WU_HOST"
wu_code() { curl -s -o /dev/null -w '%{http_code}' "$@"; }
wu_expect() { # name, got, want
    if [[ "$2" == "$3" ]]; then
        printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s got %s, wanted %s\n' "$1" "$2" "$3"; FAILED=$((FAILED + 1))
    fi
}

if [[ -z "$WU_URL" ]]; then
    printf 'FAIL  %-34s the server printed no URL: %s\n' "war ui starts" "$(tail -2 "$WU_ERR")"
    FAILED=$((FAILED + 1))
else
    # OBL-001: the act route by method.
    wu_expect "an act by PUT" "$(wu_code -X PUT -H "Authorization: Bearer $WU_TOK" "$WU_B/api/act")" 405
    wu_expect "an act by DELETE" "$(wu_code -X DELETE -H "Authorization: Bearer $WU_TOK" "$WU_B/api/act")" 405
    # GET /api/act is the act's state, which app.js's watchAct() polls: a
    # read that needs the token like every API read, and starts nothing.
    wu_expect "the act state without the token" "$(wu_code "$WU_B/api/act")" 401
    WU_S1=$(curl -s -w ' %{http_code}' -H "Authorization: Bearer $WU_TOK" "$WU_B/api/act")
    WU_S2=$(curl -s -w ' %{http_code}' -H "Authorization: Bearer $WU_TOK" "$WU_B/api/act")
    WU_ACTS=$(grep -c '^war ui: act ' "$WU_ERR")
    if [[ "$WU_S1" == '{"state":"idle"} 200' && "$WU_S2" == '{"state":"idle"} 200' && "$WU_ACTS" == 0 ]]; then
        printf 'ok    %-34s %s, twice; no act started\n' "a GET on /api/act only reads" "$WU_S1"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s %s then %s; %s act line(s)\n' "a GET on /api/act only reads" "$WU_S1" "$WU_S2" "$WU_ACTS"; FAILED=$((FAILED + 1))
    fi

    # OBL-002: the live version moves when a record changes, within four
    # seconds: asked every quarter second, the first answer that differs.
    WU_V1=$(curl -s -H "Authorization: Bearer $WU_TOK" "$WU_B/api/version")
    touch "$PLANT_ROOT"/docs/warrants/*/journal.jsonl
    WU_T0=$(date +%s%N); WU_V2=$WU_V1
    while [[ "$WU_V2" == "$WU_V1" && $(( ($(date +%s%N) - WU_T0) / 1000000 )) -lt 4000 ]]; do
        sleep 0.25
        WU_V2=$(curl -s -H "Authorization: Bearer $WU_TOK" "$WU_B/api/version")
    done
    WU_MS=$(( ($(date +%s%N) - WU_T0) / 1000000 ))
    if [[ -n "$WU_V1" && "$WU_V1" != "$WU_V2" && $WU_MS -lt 4000 ]]; then
        printf 'ok    %-34s the version moved in %s ms (< 4000)\n' "a record change is seen" "$WU_MS"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s version did not move within 4000 ms\n' "a record change is seen"; FAILED=$((FAILED + 1))
    fi

    # OBL-003: the act's command, whole, and one act at a time.
    WU_Q=$(curl -s -H "Authorization: Bearer $WU_TOK" "$WU_B/api/queue")
    WU_ID=$(python3 -c 'import sys,json; d=json.load(sys.stdin); print(next((a["act_id"] for a in d["acts"] if "act_id" in a), ""))' <<<"$WU_Q" 2>/dev/null)
    # The Queue row itself: its dry-run verdict beside the allowlist id, and
    # an id is on a row only when its verdict is "would record".
    WU_ROW=$(python3 -c '
import sys, json
d = json.load(sys.stdin)
rows = [a for a in d["acts"] if a.get("target") == "WV-WAR-0001"]
assert len(rows) == 1, f"{len(rows)} rows for WV-WAR-0001"
r = rows[0]
assert r.get("verdict") == "would record", "verdict %r" % r.get("verdict")
assert r.get("act_id", "").startswith("sign-"), "act_id %r" % r.get("act_id")
assert all(a.get("verdict") == "would record" for a in d["acts"] if "act_id" in a), "an id on a row whose verdict is not would record"
print("%s WV-WAR-0001: verdict %r, act_id %s" % (r["act"], r["verdict"], r["act_id"]))
' <<<"$WU_Q" 2>&1)
    if [[ $? -eq 0 ]]; then
        printf 'ok    %-34s %s\n' "the Queue row: verdict and id" "$WU_ROW"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s %s\n' "the Queue row: verdict and id" "$(tail -1 <<<"$WU_ROW" | head -c 300)"; FAILED=$((FAILED + 1))
    fi

    # An id not on the allowlist: 403 with the allowlist's refusal (not the
    # Origin's), and no process: no `war ui: act` line in the server's log,
    # the act state still idle (a started act is never idle again), and no
    # act among the server's children. The tree is read by a sampler that
    # runs from before the POST to a second after it, reading every thread's
    # /proc children list and each child's command line in a loop without
    # forking — an act's children live well under a second, too briefly for
    # a ps afterwards. The server has other children (its record watcher runs
    # git), so an act is a child whose command line carries `sign`, the verb
    # every signing act runs, dry or not. The started act below, watched by
    # the same sampler, is the control that this sees one.
    wu_sample() { # out-file, stop-file
        local f p a
        local -a pids args
        while [[ ! -e "$2" ]]; do
            for f in /proc/"$WU_PID"/task/*/children; do
                pids=(); read -r -a pids <"$f" 2>/dev/null
                for p in "${pids[@]}"; do
                    args=(); mapfile -d '' -t args <"/proc/$p/cmdline" 2>/dev/null
                    a="${args[*]}"
                    [[ -n "$a" ]] && printf '%s: %s\n' "$p" "${a:0:160}"
                done
            done
        done >"$1"
    }
    wu_acts_in() { grep -E '^[0-9]+: [^ ]*war( .*)? sign( |$)' "$1" | sort -u; }
    WU_KF=$(mktemp); WU_KSTOP=$(mktemp -u)
    wu_sample "$WU_KF" "$WU_KSTOP" & WU_KPID=$!
    WU_A0=$(grep -c '^war ui: act ' "$WU_ERR")
    WU_R=$(curl -s -w ' %{http_code}' -X POST -H "Origin: $WU_B" -H "Authorization: Bearer $WU_TOK" -H 'Content-Type: application/json' -d '{"id":"sign-0000000000000000"}' "$WU_B/api/act")
    sleep 1
    : >"$WU_KSTOP"; wait "$WU_KPID"
    WU_KIDS=$(wu_acts_in "$WU_KF")
    WU_OTHER=$(cut -d' ' -f2- "$WU_KF" | cut -d' ' -f1-3 | sort -u | tr '\n' ';')
    command rm -f "$WU_KF" "$WU_KSTOP"
    WU_A1=$(grep -c '^war ui: act ' "$WU_ERR")
    WU_S3=$(curl -s -H "Authorization: Bearer $WU_TOK" "$WU_B/api/act")
    if [[ "$WU_R" == 'not an act this page may start 403' && "$WU_A0" == 0 && "$WU_A1" == 0 && "$WU_S3" == '{"state":"idle"}' && -z "$WU_KIDS" ]]; then
        printf 'ok    %-34s %s; no act line, state idle, no act among its children (others seen: %s)\n' "an act not on the allowlist" "$WU_R" "${WU_OTHER:-none}"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s %s; act lines %s -> %s; state %s; children [%s]\n' "an act not on the allowlist" "$WU_R" "$WU_A0" "$WU_A1" "$WU_S3" "$WU_KIDS"; FAILED=$((FAILED + 1))
    fi

    if [[ -n "$WU_ID" ]]; then
        # Two POSTs at once: an act takes a process spawn to start, far longer
        # than the gap between two concurrent requests, so exactly one is
        # accepted and the other finds it running. (Sent one after the other,
        # a fast act can finish first, and a second act is then correct.)
        WU_P1=$(mktemp); WU_P2=$(mktemp)
        # The control for the 403's observation: the same sampler, watching
        # a started act, sees its child (the dry run, then the sign).
        WU_KF=$(mktemp); WU_KSTOP=$(mktemp -u)
        wu_sample "$WU_KF" "$WU_KSTOP" & WU_KPID=$!
        WU_CURLS=()
        for f in "$WU_P1" "$WU_P2"; do
            wu_code -X POST -H "Origin: $WU_B" -H "Authorization: Bearer $WU_TOK" -H 'Content-Type: application/json' -d "{\"id\":\"$WU_ID\"}" "$WU_B/api/act" >"$f" &
            WU_CURLS+=($!)
        done
        wait "${WU_CURLS[@]}"   # the two POSTs only; the server runs on
        WU_BOTH=$(sort "$WU_P1" "$WU_P2" | tr '\n' ' '); command rm -f "$WU_P1" "$WU_P2"
        for _ in $(seq 1 120); do
            WU_ST=$(curl -s -H "Authorization: Bearer $WU_TOK" "$WU_B/api/act")
            [[ "$WU_ST" == *'"state":"done"'* ]] && break
            sleep 0.5
        done
        : >"$WU_KSTOP"; wait "$WU_KPID"
        WU_CKIDS=$(wu_acts_in "$WU_KF" | head -2 | sed "s|$PLANT_ROOT|<root>|" | tr '\n' ';'); command rm -f "$WU_KF" "$WU_KSTOP"
        wu_expect "two acts at once: one runs" "$WU_BOTH" "202 409 "
        if [[ -n "$WU_CKIDS" && "$WU_ST" != '{"state":"idle"}' ]]; then
            printf 'ok    %-34s a started act is seen: %s state %s\n' "control: the same observation" "$WU_CKIDS" "${WU_ST:0:15}"; PASSED=$((PASSED + 1))
        else
            printf 'FAIL  %-34s a started act showed children [%s], state %s: the 403 check would miss one\n' "control: the same observation" "$WU_CKIDS" "${WU_ST:0:40}"; FAILED=$((FAILED + 1))
        fi
        WU_LINES=$(grep -c '^war ui: act ' "$WU_ERR")
        WU_LINE=$(grep -m1 '^war ui: act ' "$WU_ERR")
        WU_WANT="war ui: act $WU_ID: war --root $PLANT_ROOT sign WU-WAR-0001 --as your-name-here --ssh-sign"
        WU_WANT=${WU_WANT/WU-WAR-0001/WV-WAR-0001}
        if [[ "$WU_LINES" == 1 && "$WU_LINE" == "$WU_WANT" && ! -f "$PLANT_ROOT/docs/warrants/WV-WAR-0001/authorization.toml" ]]; then
            printf 'ok    %-34s %s; nothing signed\n' "the act's command, from its log" "${WU_LINE/$PLANT_ROOT/<root>}"; PASSED=$((PASSED + 1))
        else
            printf 'FAIL  %-34s %s line(s); first %s\n' "the act's command, from its log" "$WU_LINES" "$WU_LINE"; FAILED=$((FAILED + 1))
        fi
    else
        printf 'FAIL  %-34s the queue offered no act: %s\n' "an allowlisted act" "$(head -c 200 <<<"$WU_Q")"; FAILED=$((FAILED + 1))
    fi
fi
kill "$WU_PID" 2>/dev/null; wait "$WU_PID" 2>/dev/null
command rm -f "$WU_OUT" "$WU_ERR"

# OBL-002 on this corpus: /api/progress is `war roadmap`'s phases, in its
# order, with its exits, tiers, open slugs and members; each member's rung is
# `war status --json`'s; the unassigned group is status's Warrants whose
# objective names no roadmap phase. Read-only: GETs and two read commands.
# Slow in a debug build (each side reads the whole corpus), so the roadmap
# is read while the server answers.
WU_OUT=$(mktemp); WU_ERR=$(mktemp); WU_RM=$(mktemp); WU_ST=$(mktemp); WU_PG=$(mktemp)
"$WAR" --root "$REPO_ROOT" roadmap --json >"$WU_RM" 2>/dev/null &
WU_RMPID=$!
"$WAR" --root "$REPO_ROOT" --json status >"$WU_ST" 2>/dev/null
env -u SSH_AUTH_SOCK "$WAR" --root "$REPO_ROOT" ui --port 0 >"$WU_OUT" 2>"$WU_ERR" &
WU_PID=$!
for _ in $(seq 1 60); do grep -q 'http://' "$WU_OUT" && break; sleep 0.25; done
WU_URL=$(grep -o 'http://[^ ]*' "$WU_OUT" | head -1)
WU_HOST=${WU_URL#http://}; WU_HOST=${WU_HOST%%/*}
WU_TOK=${WU_URL#*#t=}; WU_TOK=${WU_TOK%%&*}
curl -s --max-time 900 -H "Authorization: Bearer $WU_TOK" "http://$WU_HOST/api/progress" >"$WU_PG"
kill "$WU_PID" 2>/dev/null; wait "$WU_PID" 2>/dev/null
wait "$WU_RMPID" 2>/dev/null
WU_CMP=$(python3 - "$WU_PG" "$WU_RM" "$WU_ST" <<'PY' 2>&1
import json, sys
pg = json.load(open(sys.argv[1]))
rm = json.load(open(sys.argv[2]))["result"]
st = json.load(open(sys.argv[3]))["result"]
rung = {w["alias"]: w["rung"] for w in st["warrants"]}
want = [p["id"] for p in rm["phases"]]
got = [p["id"] for p in pg["phases"]]
assert got == want, f"phase order {got} != war roadmap's {want}"
assert got, "war roadmap lists no phases: nothing compared"
n = 0
for g, r in zip(pg["phases"], rm["phases"]):
    for k in ("title", "tier", "exit", "depends_on", "achieved", "exit_warrant", "open"):
        assert g[k] == r[k], f"{g['id']} {k}: {g[k]!r} != war roadmap's {r[k]!r}"
    assert [m["alias"] for m in g["members"]] == r["members"], f"{g['id']} members differ"
    for m in g["members"]:
        assert m.get("rung") == rung.get(m["alias"]), f"{m['alias']} rung {m.get('rung')} != status {rung.get(m['alias'])}"
        n += 1
assert n, "no phase has a member: no rung compared"
un = [a for o in st["objectives"] if o.get("roadmap_ref") is None for a in o["warrants"]]
assert [m["alias"] for m in pg["unassigned"]] == un, "unassigned differs from status"
for m in pg["unassigned"]:
    assert m.get("rung") == rung.get(m["alias"]), f"unassigned {m['alias']} rung differs"
opened = sum(len(p["open"]) for p in pg["phases"])
print(f"{len(got)} phases in war roadmap's order ({got[0]} .. {got[-1]}); {n} members' rungs = status; "
      f"{opened} open slug(s); {len(un)} unassigned; accepted revision {pg['roadmap'].get('accepted_revision')}")
PY
)
if [[ $? -eq 0 && -n "$WU_URL" ]]; then
    printf 'ok    %-34s %s\n' "progress on this corpus" "$WU_CMP"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "progress on this corpus" "$(tail -1 <<<"$WU_CMP" | head -c 300)"; FAILED=$((FAILED + 1))
fi
command rm -f "$WU_OUT" "$WU_ERR" "$WU_RM" "$WU_ST" "$WU_PG"

# The Progress view is built from roadmap_cmd::view_with and status::build;
# nothing under webui/ names view.json (progress_viewer's Corpus snapshot
# falls back to it only for a program with no roadmap record).
if ! grep -rn 'view\.json' "$WU_SRC" >/dev/null 2>&1; then
    printf 'ok    %-34s no view.json under webui/\n' "progress is not read from view.json"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "progress is not read from view.json" "$(grep -rn -m1 'view\.json' "$WU_SRC")"; FAILED=$((FAILED + 1))
fi

corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
