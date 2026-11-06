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

    # The live version moves when a record changes.
    WU_V1=$(curl -s -H "Authorization: Bearer $WU_TOK" "$WU_B/api/version")
    touch "$PLANT_ROOT"/docs/warrants/*/journal.jsonl
    sleep 1
    WU_V2=$(curl -s -H "Authorization: Bearer $WU_TOK" "$WU_B/api/version")
    if [[ -n "$WU_V1" && "$WU_V1" != "$WU_V2" ]]; then
        printf 'ok    %-34s the version moved\n' "a record change is seen"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s version did not move\n' "a record change is seen"; FAILED=$((FAILED + 1))
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
    else
        printf 'FAIL  %-34s the queue offered no act: %s\n' "an allowlisted act" "$(head -c 200 <<<"$WU_Q")"; FAILED=$((FAILED + 1))
    fi
fi
kill "$WU_PID" 2>/dev/null; wait "$WU_PID" 2>/dev/null
command rm -f "$WU_OUT" "$WU_ERR"

# No key, no socket, no in-process signing under webui/.
if ! grep -rn 'ssh-keygen\|SSH_AUTH_SOCK\|sign::run(repo, Some\|ssh_sign_file\|authorize::ingest\|resolution_cmd::ingest' "$WU_SRC" \
    | grep -v ':\s*//' | grep -q .; then
    printf 'ok    %-34s no key, socket or signing call\n' "the web UI holds no authority"; PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s webui/ names a key, socket or signing seam\n' "the web UI holds no authority"; FAILED=$((FAILED + 1))
fi

corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
