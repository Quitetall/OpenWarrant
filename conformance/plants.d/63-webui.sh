# shellcheck shell=bash
# OW-WAR-0116 — `war ui`: the hardened loopback web UI.
#
# Every plant runs against a scratch program with SSH_AUTH_SOCK unset, so an
# act the page starts can never reach an ssh agent: the plants prove which
# act the server ran and that nothing else ran, never that anyone signed.

echo "== web ui (OW-WAR-0116) =="
PLANT_ROOT=$(scratch_corpus WU)
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
    wu_expect "the page is served" "$(wu_code "$WU_B/")" 200
    wu_expect "an API call without the token" "$(wu_code "$WU_B/api/version")" 401
    wu_expect "an API call with a wrong token" "$(wu_code -H 'Authorization: Bearer 00' "$WU_B/api/version")" 401
    wu_expect "a foreign Host (DNS rebinding)" "$(wu_code -H 'Host: evil.example' -H "Authorization: Bearer $WU_TOK" "$WU_B/api/version")" 400
    wu_expect "a foreign Origin" "$(wu_code -H 'Origin: http://evil.example' -H "Authorization: Bearer $WU_TOK" "$WU_B/api/version")" 403
    wu_expect "an act by a non-POST method" "$(wu_code -X PUT -H "Authorization: Bearer $WU_TOK" "$WU_B/api/act")" 405
    wu_expect "an act by DELETE" "$(wu_code -X DELETE -H "Authorization: Bearer $WU_TOK" "$WU_B/api/act")" 405
    # GET /api/act is the act's state, which app.js's watchAct() polls: a
    # read that needs the token like every API read, and starts nothing.
    # Before any POST it says idle, and still says idle after being asked.
    wu_expect "the act state without the token" "$(wu_code "$WU_B/api/act")" 401
    WU_S1=$(curl -s -w ' %{http_code}' -H "Authorization: Bearer $WU_TOK" "$WU_B/api/act")
    WU_S2=$(curl -s -w ' %{http_code}' -H "Authorization: Bearer $WU_TOK" "$WU_B/api/act")
    if [[ "$WU_S1" == '{"state":"idle"} 200' && "$WU_S2" == '{"state":"idle"} 200' ]] \
        && ! grep -q '^war ui: act ' "$WU_ERR"; then
        printf 'ok    %-34s %s, twice; no act started\n' "a GET on /api/act only reads" "$WU_S1"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s %s then %s\n' "a GET on /api/act only reads" "$WU_S1" "$WU_S2"; FAILED=$((FAILED + 1))
    fi
    wu_expect "an act with no Origin" "$(wu_code -X POST -H "Authorization: Bearer $WU_TOK" -H 'Content-Type: application/json' -d '{"id":"x"}' "$WU_B/api/act")" 403
    wu_expect "an act not on the allowlist" "$(wu_code -X POST -H "Origin: $WU_B" -H "Authorization: Bearer $WU_TOK" -H 'Content-Type: application/json' -d '{"id":"sign-0000000000000000"}' "$WU_B/api/act")" 403
    wu_expect "an act that carries an argv" "$(wu_code -X POST -H "Origin: $WU_B" -H "Authorization: Bearer $WU_TOK" -H 'Content-Type: application/json' -d '{"id":"x","argv":["rm","-rf","/"]}' "$WU_B/api/act")" 400
    wu_expect "an oversized request" "$(wu_code -H "X-Big: $(head -c 9000 /dev/zero | tr '\0' a)" "$WU_B/")" 431

    WU_CSP=$(curl -s -D - -o /dev/null "$WU_B/" | tr -d '\r' | grep -i '^content-security-policy:')
    if grep -q "frame-ancestors 'none'" <<<"$WU_CSP" && ! grep -q 'unsafe-inline' <<<"$WU_CSP"; then
        printf 'ok    %-34s no unsafe-inline, frame-ancestors none\n' "the CSP"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s %s\n' "the CSP" "$WU_CSP"; FAILED=$((FAILED + 1))
    fi

    WU_PORT=${WU_HOST##*:}
    if ss -ltnH 2>/dev/null | awk '{print $4}' | grep -qx "127.0.0.1:$WU_PORT" \
        && ! ss -ltnH 2>/dev/null | awk '{print $4}' | grep -qE "^(0\.0\.0\.0|\*|\[::\]):$WU_PORT$"; then
        printf 'ok    %-34s 127.0.0.1:%s only\n' "bound to loopback" "$WU_PORT"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s not loopback-only\n' "bound to loopback"; FAILED=$((FAILED + 1))
    fi

    # Progress is the canonical roadmap when the program has one; this
    # scaffold has none, so the page says so rather than inventing phases.
    WU_PROG=$(curl -s -H "Authorization: Bearer $WU_TOK" "$WU_B/api/progress")
    if python3 -c 'import sys,json; d=json.load(sys.stdin); assert "roadmap" in d and "phases" in d and "ladder" in d' <<<"$WU_PROG" 2>/dev/null; then
        printf 'ok    %-34s roadmap, phases, ladder\n' "the progress view"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s %s\n' "the progress view" "$(head -c 160 <<<"$WU_PROG")"; FAILED=$((FAILED + 1))
    fi

    # The live version moves when a record changes, within four seconds
    # (OBL-002): asked every quarter second, the first answer that differs.
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

    # The scaffold's adopt Warrant awaits authorization. The example register
    # names two signers, so the server runs `--as your-name-here`; the queue offers it
    # with an act id when its dry run would record. Starting it runs exactly
    # `war --root <root> sign <alias> --ssh-sign` — and with no agent socket
    # the signature fails, so nothing is signed.
    WU_Q=$(curl -s -H "Authorization: Bearer $WU_TOK" "$WU_B/api/queue")
    WU_ID=$(python3 -c 'import sys,json; d=json.load(sys.stdin); print(next((a["act_id"] for a in d["acts"] if "act_id" in a), ""))' <<<"$WU_Q" 2>/dev/null)
    if [[ -n "$WU_ID" ]]; then
        WU_POST=$(wu_code -X POST -H "Origin: $WU_B" -H "Authorization: Bearer $WU_TOK" -H 'Content-Type: application/json' -d "{\"id\":\"$WU_ID\"}" "$WU_B/api/act")
        WU_AGAIN=$(wu_code -X POST -H "Origin: $WU_B" -H "Authorization: Bearer $WU_TOK" -H 'Content-Type: application/json' -d "{\"id\":\"$WU_ID\"}" "$WU_B/api/act")
        for _ in $(seq 1 120); do
            grep -q '"state":"done"' <(curl -s -H "Authorization: Bearer $WU_TOK" "$WU_B/api/act") && break
            sleep 0.5
        done
        if [[ "$WU_POST" == 202 ]] && grep -q "act $WU_ID: war --root $PLANT_ROOT sign WU-WAR-0001 --as your-name-here --ssh-sign" "$WU_ERR" \
            && [[ "$WU_AGAIN" == 409 || "$WU_AGAIN" == 202 ]] \
            && [[ ! -f "$PLANT_ROOT/docs/warrants/WU-WAR-0001/authorization.toml" ]]; then
            printf 'ok    %-34s ran sign WU-WAR-0001 --ssh-sign; nothing signed\n' "an allowlisted act"; PASSED=$((PASSED + 1))
        else
            printf 'FAIL  %-34s post %s, again %s; %s\n' "an allowlisted act" "$WU_POST" "$WU_AGAIN" "$(tail -1 "$WU_ERR")"; FAILED=$((FAILED + 1))
        fi
        # OBL-003, the command itself: the server's act log line, whole, is
        # `war --root <root> sign <target> --ssh-sign` with the `--as` this
        # server was started with, and it is the only act line.
        WU_LINES=$(grep -c '^war ui: act ' "$WU_ERR")
        WU_LINE=$(grep -m1 '^war ui: act ' "$WU_ERR")
        WU_WANT="war ui: act $WU_ID: war --root $PLANT_ROOT sign WU-WAR-0001 --as your-name-here --ssh-sign"
        if [[ "$WU_LINES" == 1 && "$WU_LINE" == "$WU_WANT" ]]; then
            printf 'ok    %-34s %s\n' "the act's command, from its log" "${WU_LINE/$PLANT_ROOT/<root>}"; PASSED=$((PASSED + 1))
        else
            printf 'FAIL  %-34s %s line(s); first %s\n' "the act's command, from its log" "$WU_LINES" "$WU_LINE"; FAILED=$((FAILED + 1))
        fi
        # One act at a time: the second POST, sent while the first is still
        # dry-running, is refused 409 and adds no act line.
        wu_expect "a second act while one runs" "$WU_AGAIN" 409
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

# No key, no socket, no in-process signing under webui/.
if ! grep -rn 'ssh-keygen\|SSH_AUTH_SOCK\|sign::run(repo, Some\|ssh_sign_file\|authorize::ingest\|resolution_cmd::ingest' "$WU_SRC" \
    | grep -v ':\s*//' | grep -q .; then
    printf 'ok    %-34s no key, socket or signing call\n' "the web UI holds no authority"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s webui/ names a key, socket or signing seam\n' "the web UI holds no authority"; FAILED=$((FAILED + 1))
fi

corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
