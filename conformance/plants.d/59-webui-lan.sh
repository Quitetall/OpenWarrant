# shellcheck shell=bash
# OW-WAR-0139 — `war ui --lan`: the web UI beyond this machine, TLS only,
# paired devices only, and no LAN request ever starts a signing act.
#
# CONTINGENT: built against U-001 option A and U-002 option A (B as the
# labelled fallback). Every server here binds 127.0.0.1 or 127.0.0.2 on a
# port the kernel picks, runs with SSH_AUTH_SOCK unset and with a state
# directory of its own, and is stopped by its exact PID. Certificates are
# made here with `openssl req -x509`; nothing touches the network.
#
# The host's terminal is played by a pseudo-terminal (python3's `pty`),
# which answers each pairing prompt from a script: y, n, or no answer. The
# pty is basis R-003's residual, stated rather than hidden: a pty answers
# the TTY guard, and what it buys is a device credential that cannot sign.

echo "== web ui on the LAN (OW-WAR-0139) =="
for WL_NEED in openssl python3 curl ss; do
    command -v "$WL_NEED" >/dev/null 2>&1 || { printf 'PLANT SETUP FAILED: %s is not installed; the LAN controls are UNKNOWN here, not passed\n' "$WL_NEED" >&2; exit 9; }
done
PLANT_ROOT=$(scratch_corpus WL)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
cp "$PLANT_ROOT/docs/authority/roles.toml.example" "$PLANT_ROOT/docs/authority/roles.toml"
cp "$PLANT_ROOT/docs/authority/allowed_signers.example" "$PLANT_ROOT/docs/authority/allowed_signers"
# An error whose remedy is automatic: a child Warrant its parent's committed
# view does not list yet (`relations.child-listed` → `war compile`).
"$WAR" --root "$PLANT_ROOT" new "A child for the remedy" >/dev/null 2>&1
WL_PARENT=$(grep '^uuid' "$PLANT_ROOT/docs/warrants/WL-WAR-0001/manifest.toml" | cut -d'"' -f2)
printf '\n[[parents]]\nref = "war://%s"\ncontract_revision = 1\n' "$WL_PARENT" >>"$PLANT_ROOT/docs/warrants/WL-WAR-0002/manifest.toml"
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "register, a child" >/dev/null 2>&1
grep -q '^ERROR relations.child-listed' <<<"$("$WAR" --root "$PLANT_ROOT" check 2>/dev/null)" \
    || { printf 'PLANT SETUP FAILED: no relations.child-listed error to remedy\n' >&2; exit 9; }

WL_T=$(mktemp -d)
WL_STATE="$WL_T/state"
WL_SRC=crates/openwarrant-cli/src/webui
WL_NAME=war-lan.test
( cd "$WL_T" && openssl req -x509 -newkey ec -pkeyopt ec_paramgen_curve:P-256 -nodes \
    -keyout key.pem -out cert.pem -days 2 -subj "/CN=$WL_NAME" \
    -addext "subjectAltName=DNS:$WL_NAME" >/dev/null 2>&1 ) \
    || { printf 'PLANT SETUP FAILED: openssl could not make a certificate\n' >&2; exit 9; }

# The host terminal: argv on a fresh pty answering prompts from a script, or
# with no terminal at all ("notty"). Its output goes to a log, its pid to a file.
cat >"$WL_T/hostterm.py" <<'PY'
import os, pty, select, signal, sys
log, pidfile, answers, argv = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:]
out = open(log, "ab", buffering=0)
if answers == "notty":
    pid = os.fork()
    if pid == 0:
        os.setsid()
        fd = os.open(log, os.O_WRONLY | os.O_APPEND)
        os.dup2(fd, 1); os.dup2(fd, 2)
        os.dup2(os.open("/dev/null", os.O_RDONLY), 0)
        os.execvp(argv[0], argv)
    master = None
else:
    pid, master = pty.fork()
    if pid == 0:
        os.execvp(argv[0], argv)
open(pidfile, "w").write(str(pid))
def stop(*_):
    try: os.kill(pid, signal.SIGTERM)
    except ProcessLookupError: pass
signal.signal(signal.SIGTERM, stop)
queue = [a for a in answers.split(",") if a] if master is not None else []
seen, buf = 0, b""
while True:
    if master is None:
        try: os.waitpid(pid, 0)
        except InterruptedError: continue
        except ChildProcessError: pass
        break
    try: r, _, _ = select.select([master], [], [], 0.2)
    except InterruptedError: continue
    if not r:
        try:
            if os.waitpid(pid, os.WNOHANG)[0]: break
        except ChildProcessError: break
        continue
    try: data = os.read(master, 4096)
    except OSError: break
    if not data: break
    out.write(data); buf += data
    n = buf.count(b"Pair it? [y/N]")
    while seen < n:
        seen += 1
        a = queue.pop(0) if queue else "-"
        if a in ("y", "n"):
            os.write(master, (a + "\n").encode())
try: os.waitpid(pid, 0)
except ChildProcessError: pass
PY

wl_ok()   { printf 'ok    %-40s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
wl_fail() { printf 'FAIL  %-40s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
wl_expect() { # name, got, want
    if [[ "$2" == "$3" ]]; then wl_ok "$1" "$2"; else wl_fail "$1" "got $2, wanted $3"; fi
}
# wl_start <tag> <answers|notty> <war ui args...>: a host with its own log.
wl_start() {
    local tag=$1 answers=$2; shift 2
    : >"$WL_T/$tag.log"
    python3 "$WL_T/hostterm.py" "$WL_T/$tag.log" "$WL_T/$tag.pid" "$answers" \
        env -u SSH_AUTH_SOCK XDG_STATE_HOME="$WL_STATE" "$WAR" --root "$PLANT_ROOT" ui --port 0 --as your-name-here "$@" &
    printf '%s' $! >"$WL_T/$tag.driver"
    for _ in $(seq 1 80); do grep -aq '#pair=' "$WL_T/$tag.log" 2>/dev/null && break; sleep 0.25; done
}
wl_stop() { # by exact pid: the server, then its driver
    local p d
    p=$(cat "$WL_T/$1.pid" 2>/dev/null); d=$(cat "$WL_T/$1.driver" 2>/dev/null)
    [[ -n "$p" ]] && kill "$p" 2>/dev/null
    [[ -n "$d" ]] && { kill "$d" 2>/dev/null; wait "$d" 2>/dev/null; }
}
wl_port() { local u; u=$(grep -ao "https://$WL_NAME:[0-9]*" "$WL_T/$1.log" | head -1); printf '%s' "${u##*:}"; }
wl_code() { local c; c=$(grep -ao '#pair=[0-9a-f]*' "$WL_T/$1.log" | tail -1); printf '%s' "${c#\#pair=}"; }
# wl_next_code <tag> <previous>: wait for the link printed after an attempt.
wl_next_code() {
    local c
    for _ in $(seq 1 40); do c=$(wl_code "$1"); [[ -n "$c" && "$c" != "$2" ]] && break; sleep 0.1; done
    printf '%s' "$c"
}
# wl_curl <port> <curl args...>: TLS to 127.0.0.2, verified against the plant's cert.
wl_curl() {
    local p=$1; shift
    curl -s --max-time 10 --resolve "$WL_NAME:$p:127.0.0.2" --cacert "$WL_T/cert.pem" "$@"
}
# wl_pair <tag> <code> <cookie jar> <user agent>: prints "<status> <body>".
wl_pair() {
    local p; p=$(wl_port "$1")
    wl_curl "$p" -c "$3" -A "$4" -H "Origin: https://$WL_NAME:$p" -H 'Content-Type: application/json' \
        -d "{\"code\":\"$2\"}" -w '\n%{http_code}' "https://$WL_NAME:$p/pair" | tr '\n' ' ' \
        | awk '{print $NF, $0}' | sed 's/ [0-9]*$//'
}
wl_cred() { awk '$6 == "__Host-war_device" {print $7}' "$1" 2>/dev/null | tail -1; }
wl_nonce() { # <port> <jar>
    wl_curl "$1" -b "$2" "https://$WL_NAME:$1/api/nonce" \
        | python3 -c 'import sys,json; print(json.load(sys.stdin)["nonce"])' 2>/dev/null
}
# The loopback page's responses, headers only, over thirteen requests whose
# bodies OW-WAR-0139 does not change (app.js is a deliverable, so it is not
# among them; /api/version's body carries the root path, so its 200 is not).
wl_loopback_headers() { # <host:port> <token>
    local b="http://$1" t="Authorization: Bearer $2"
    {
        curl -s -D - -o /dev/null "$b/"
        curl -s -D - -o /dev/null "$b/assets/app.css"
        curl -s -D - -o /dev/null "$b/api/version"
        curl -s -D - -o /dev/null "$b/nope"
        curl -s -D - -o /dev/null -H "$t" "$b/api/nope"
        curl -s -D - -o /dev/null -H 'Host: x' "$b/"
        curl -s -D - -o /dev/null -H 'Origin: http://evil' "$b/"
        curl -s -D - -o /dev/null -X PUT -H "$t" "$b/api/act"
        curl -s -D - -o /dev/null -X POST -H "Origin: $b" -H "$t" -d '{"id":"x","argv":[]}' "$b/api/act"
        curl -s -D - -o /dev/null -X POST -H "Origin: $b" -H "$t" -d '{"id":"sign-0000"}' "$b/api/act"
        curl -s -D - -o /dev/null -X POST -H "Origin: $b" -H "$t" -d '{"id":"x"}' "$b/api/request"
        curl -s -D - -o /dev/null -H "$t" "$b/api/nonce"
        curl -s -D - -o /dev/null -X POST -H "$t" -d '{"id":"x"}' "$b/api/act"
    } | tr -d '\r' | sha256sum | cut -d' ' -f1
}
# Captured with the same function from the build of 269d3324, before this
# Warrant touched webui/ (OW-WAR-0116's loopback server).
WL_LOOPBACK_DIGEST=532d4d64d802abe3aa6b26f68215cad571cd81d366d9e1b7b6e9fa7fd75f3dcd

# ---- OBL-001: LAN mode serves only TLS, only to the configured name --------

WL_FREE=$((20000 + RANDOM % 20000))
WL_OUT=$(env -u SSH_AUTH_SOCK XDG_STATE_HOME="$WL_STATE" "$WAR" --root "$PLANT_ROOT" ui --port 0 --lan "127.0.0.2:$WL_FREE" 2>&1 & WLP=$!; sleep 1.5; if kill -0 "$WLP" 2>/dev/null; then echo STILL-RUNNING; awk '{print $4}' <<<"$(ss -ltnH 2>/dev/null)" | grep -x "127.0.0.2:$WL_FREE"; kill "$WLP"; fi; wait "$WLP"; echo "exit=$?")
if grep -q 'ui.lan-needs-tls' <<<"$WL_OUT" && grep -q 'exit=1' <<<"$WL_OUT" && ! grep -q 'STILL-RUNNING' <<<"$WL_OUT" \
    && ! grep -qx "127.0.0.2:$WL_FREE" <<<"$(awk '{print $4}' <<<"$(ss -ltnH 2>/dev/null)")"; then
    wl_ok "OBL-001 --lan with no certificate" "ui.lan-needs-tls, exit 1, nothing bound"
else
    wl_fail "OBL-001 --lan with no certificate" "$(tr '\n' ' ' <<<"$WL_OUT" | head -c 200)"
fi
WL_OUT=$(env -u SSH_AUTH_SOCK XDG_STATE_HOME="$WL_STATE" timeout 5 "$WAR" --root "$PLANT_ROOT" ui --port 0 --lan "127.0.0.2:$WL_FREE" --cert "$WL_T/cert.pem" 2>&1; echo "exit=$?")
if grep -q 'ui.lan-needs-tls' <<<"$WL_OUT" && grep -q 'exit=1' <<<"$WL_OUT"; then
    wl_ok "OBL-001 --lan with a certificate, no key" "ui.lan-needs-tls, exit 1"
else
    wl_fail "OBL-001 --lan with a certificate, no key" "$(tr '\n' ' ' <<<"$WL_OUT" | head -c 200)"
fi

wl_start A "y,y,n,-" --lan 127.0.0.2:0 --name "$WL_NAME" --cert "$WL_T/cert.pem" --key "$WL_T/key.pem" --pair-answer-secs 2
WL_AP=$(wl_port A)
WL_AB="https://$WL_NAME:$WL_AP"
WL_APID=$(cat "$WL_T/A.pid" 2>/dev/null)
WL_LURL=$(grep -ao 'http://[^ ]*' "$WL_T/A.log" | head -1)
WL_LHOST=${WL_LURL#http://}; WL_LHOST=${WL_LHOST%%/*}
WL_TOK=${WL_LURL#*#t=}; WL_TOK=${WL_TOK%%&*}
if [[ -z "$WL_AP" || -z "$WL_TOK" ]]; then
    wl_fail "war ui --lan starts" "$(grep -av '^  [█▀▄ ]*$' "$WL_T/A.log" | tail -3 | tr '\n' ' ')"
else
    wl_expect "OBL-001 the page over verified TLS" "$(wl_curl "$WL_AP" -o /dev/null -w '%{http_code}' "$WL_AB/")" 200
    WL_RAW=$(python3 - "$WL_AP" <<'PY'
import socket, sys
s = socket.create_connection(("127.0.0.2", int(sys.argv[1])), timeout=5)
s.sendall(b"GET / HTTP/1.1\r\nHost: x\r\n\r\n")
got = b""
try:
    while True:
        d = s.recv(4096)
        if not d: break
        got += d
except OSError: pass
print("HTTP" if b"HTTP/" in got or b"<html" in got.lower() else "no-http", len(got))
PY
)
    WL_PLAIN=$(curl -s --max-time 5 -o /dev/null -w '%{http_code}' "http://127.0.0.2:$WL_AP/")
    if [[ "$WL_RAW" == no-http* && "$WL_PLAIN" == 000 ]]; then
        wl_ok "OBL-001 plain HTTP to the LAN port" "no HTTP response, no body ($WL_RAW bytes)"
    else
        wl_fail "OBL-001 plain HTTP to the LAN port" "raw: $WL_RAW, curl: $WL_PLAIN"
    fi
    wl_expect "OBL-001 a wrong Host over TLS" "$(wl_curl "$WL_AP" -o /dev/null -w '%{http_code}' -H "Host: evil.example:$WL_AP" "$WL_AB/")" 421
    wl_expect "OBL-001 the loopback Host over TLS" "$(wl_curl "$WL_AP" -o /dev/null -w '%{http_code}' -H "Host: 127.0.0.2:$WL_AP" "$WL_AB/")" 421
    wl_expect "OBL-001 a foreign Origin over TLS" "$(wl_curl "$WL_AP" -o /dev/null -w '%{http_code}' -H 'Origin: https://evil.example' "$WL_AB/")" 403
    wl_expect "OBL-001 an http:// Origin over TLS" "$(wl_curl "$WL_AP" -o /dev/null -w '%{http_code}' -H "Origin: http://$WL_NAME:$WL_AP" "$WL_AB/")" 403
    WL_HDRS_OK=1
    for WL_REQ in "/" "/api/version" "/nope" "/api/act"; do
        WL_H=$(wl_curl "$WL_AP" -D - -o /dev/null "$WL_AB$WL_REQ" | tr -d '\r')
        grep -qx 'Strict-Transport-Security: max-age=31536000' <<<"$WL_H" \
            && grep -qx "Content-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'" <<<"$WL_H" \
            || WL_HDRS_OK=0
    done
    WL_H=$(wl_curl "$WL_AP" -D - -o /dev/null -H "Host: evil.example:$WL_AP" "$WL_AB/" | tr -d '\r')
    grep -q '^Strict-Transport-Security:' <<<"$WL_H" || WL_HDRS_OK=0
    if [[ $WL_HDRS_OK == 1 ]]; then
        wl_ok "OBL-001 HSTS and the CSP on every response" "200, 401, 404, 421"
    else
        wl_fail "OBL-001 HSTS and the CSP on every response" "a response lacks one"
    fi
    # This server's listeners only: a battery running beside this one may hold
    # the same port number on another address, and that is not this server's.
    WL_MINE=$(ss -ltnpH 2>/dev/null | awk -v p="pid=$WL_APID," 'index($0, p) { print $4 }')
    if [[ -n "$WL_APID" ]] && grep -qx "127.0.0.2:$WL_AP" <<<"$WL_MINE" \
        && ! grep -qE "^(0\.0\.0\.0|\*|\[::\]|127\.0\.0\.1):$WL_AP$" <<<"$WL_MINE"; then
        wl_ok "OBL-001 bound only to the given address" "127.0.0.2:$WL_AP only, of pid $WL_APID's listeners"
    else
        wl_fail "OBL-001 bound only to the given address" "pid ${WL_APID:-none}: $(tr '\n' ' ' <<<"$WL_MINE")"
    fi

    # ---- OBL-004 and OBL-002: pairing, credentials ------------------------
    WL_C1=$(wl_code A)
    WL_R=$(wl_pair A "$WL_C1" "$WL_T/jar1" plant-phone/1.0)
    WL_CRED1=$(wl_cred "$WL_T/jar1")
    if [[ "$WL_R" == 200* && ${#WL_CRED1} == 64 ]] && grep -aq "pair a device from 127.0.0.1 (plant-phone/1.0)" "$WL_T/A.log"; then
        wl_ok "OBL-004 y at the host pairs; names the device" "200, a credential, prompt names 127.0.0.1 and its agent"
    else
        wl_fail "OBL-004 y at the host pairs; names the device" "$WL_R / cred ${#WL_CRED1} chars"
    fi
    WL_C2=$(wl_next_code A "$WL_C1")
    WL_R=$(wl_pair A "$WL_C1" "$WL_T/jar-reuse" plant-thief/1.0)
    if [[ "$WL_R" == 403*pair.code-used* && -z "$(wl_cred "$WL_T/jar-reuse")" ]]; then
        wl_ok "OBL-002 a pairing code used twice" "403 pair.code-used, no credential"
    else
        wl_fail "OBL-002 a pairing code used twice" "$WL_R"
    fi
    WL_R=$(wl_pair A "$WL_C2" "$WL_T/jar2" plant-tablet/1.0)
    WL_CRED2=$(wl_cred "$WL_T/jar2")
    WL_DEV2=$(sed -n 's/.*"device":"\([0-9a-f]*\)".*/\1/p' <<<"$WL_R")
    WL_C3=$(wl_next_code A "$WL_C2")
    WL_DEVS_BEFORE=$(python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))["devices"]))' "$WL_STATE/openwarrant/ui-devices.json" 2>/dev/null)
    WL_R=$(wl_pair A "$WL_C3" "$WL_T/jar3" plant-declined/1.0)
    WL_DEVS_N=$(python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))["devices"]))' "$WL_STATE/openwarrant/ui-devices.json" 2>/dev/null)
    if [[ "$WL_R" == 403*pair.declined* && -z "$(wl_cred "$WL_T/jar3")" && "$WL_DEVS_N" == "$WL_DEVS_BEFORE" ]]; then
        wl_ok "OBL-004 n at the host" "403 pair.declined, no credential, no device recorded"
    else
        wl_fail "OBL-004 n at the host" "$WL_R ($WL_DEVS_BEFORE -> $WL_DEVS_N devices)"
    fi
    WL_C4=$(wl_next_code A "$WL_C3")
    WL_R=$(wl_pair A "$WL_C4" "$WL_T/jar4" plant-unanswered/1.0)
    WL_DEVS_N=$(python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))["devices"]))' "$WL_STATE/openwarrant/ui-devices.json" 2>/dev/null)
    if [[ "$WL_R" == 403*pair.timed-out* && -z "$(wl_cred "$WL_T/jar4")" && "$WL_DEVS_N" == "$WL_DEVS_BEFORE" ]]; then
        wl_ok "OBL-004 no answer at the host" "403 pair.timed-out, no credential"
    else
        wl_fail "OBL-004 no answer at the host" "$WL_R"
    fi

    wl_expect "OBL-002 a paired device reads" "$(wl_curl "$WL_AP" -b "$WL_T/jar1" -o /dev/null -w '%{http_code}' "$WL_AB/api/version")" 200
    wl_expect "OBL-002 /api/ with no credential" "$(wl_curl "$WL_AP" -o /dev/null -w '%{http_code}' "$WL_AB/api/queue")" 401
    wl_expect "OBL-002 the loopback token on the LAN" "$(wl_curl "$WL_AP" -H "Authorization: Bearer $WL_TOK" -o /dev/null -w '%{http_code}' "$WL_AB/api/queue")" 401
    wl_expect "OBL-002 a forged credential" "$(wl_curl "$WL_AP" -H "Cookie: __Host-war_device=$(printf '0%.0s' $(seq 1 64))" -o /dev/null -w '%{http_code}' "$WL_AB/api/queue")" 401
    env XDG_STATE_HOME="$WL_STATE" "$WAR" --root "$PLANT_ROOT" ui devices --revoke "$WL_DEV2" >/dev/null 2>&1
    WL_REV=$(wl_curl "$WL_AP" -b "$WL_T/jar2" -w ' %{http_code}' "$WL_AB/api/version")
    if [[ -n "$WL_DEV2" && "$WL_REV" == *ui.device-revoked*401 ]] \
        && [[ "$(wl_curl "$WL_AP" -b "$WL_T/jar1" -o /dev/null -w '%{http_code}' "$WL_AB/api/version")" == 200 ]]; then
        wl_ok "OBL-002 a revoked credential" "401 ui.device-revoked; the other device still 200"
    else
        wl_fail "OBL-002 a revoked credential" "device '$WL_DEV2': $WL_REV"
    fi
    WL_DF="$WL_STATE/openwarrant/ui-devices.json"
    WL_MODE=$(stat -c '%a' "$WL_DF" 2>/dev/null)
    if [[ "$WL_MODE" == 600 && "$WL_DF" != "$PLANT_ROOT"* && -n "$WL_CRED1" && -n "$WL_CRED2" ]] \
        && ! grep -q "$WL_CRED1" "$WL_DF" && ! grep -q "$WL_CRED2" "$WL_DF" \
        && ! grep -rq "$WL_CRED1" "$PLANT_ROOT" \
        && grep -q "$(printf '%s' "$WL_CRED1" | sha256sum | cut -d' ' -f1)" "$WL_DF"; then
        wl_ok "OBL-002 the device file" "0600, outside the repository, hashes only"
    else
        wl_fail "OBL-002 the device file" "mode $WL_MODE at $WL_DF"
    fi

    # ---- OBL-003: no LAN request starts a signing act ---------------------
    WL_LQ=$(curl -s -H "Authorization: Bearer $WL_TOK" "http://$WL_LHOST/api/queue")
    WL_SID=$(python3 -c 'import sys,json; d=json.load(sys.stdin); print(next((a["act_id"] for a in d["acts"] if "act_id" in a), ""))' <<<"$WL_LQ" 2>/dev/null)
    WL_DQ=$(wl_curl "$WL_AP" -b "$WL_T/jar1" "$WL_AB/api/queue")
    if [[ -n "$WL_SID" ]] && python3 -c '
import sys, json
d = json.load(sys.stdin)
acts = d["acts"]
assert acts, "no acts"
assert "act_id" not in json.dumps(d), "an act id reached the device"
for a in acts:
    assert a.get("verdict") and a.get("command", "").startswith("war sign ") and a.get("host_only") is True, a
' <<<"$WL_DQ" 2>/dev/null; then
        wl_ok "OBL-003 the device's queue" "verdict + host command, no act id (host has $WL_SID)"
    else
        wl_fail "OBL-003 the device's queue" "$(head -c 200 <<<"$WL_DQ")"
    fi
    WL_N=$(wl_nonce "$WL_AP" "$WL_T/jar1")
    WL_R=$(wl_curl "$WL_AP" -b "$WL_T/jar1" -H "Origin: $WL_AB" -H 'Content-Type: application/json' \
        -d "{\"id\":\"$WL_SID\",\"nonce\":\"$WL_N\"}" -w ' %{http_code}' "$WL_AB/api/act")
    sleep 1
    WL_STATE_ACT=$(curl -s -H "Authorization: Bearer $WL_TOK" "http://$WL_LHOST/api/act")
    WL_KIDS=$(ps --ppid "$WL_APID" -o args= 2>/dev/null | grep -c ' sign ')
    if [[ "$WL_R" == *act.host-only*403 && "$WL_STATE_ACT" == '{"state":"idle"}' && "$WL_KIDS" == 0 ]] \
        && ! grep -aq "war ui: act $WL_SID" "$WL_T/A.log" \
        && [[ ! -f "$PLANT_ROOT/docs/warrants/WL-WAR-0001/authorization.toml" ]]; then
        wl_ok "OBL-003 a device POSTs the signing id" "403 act.host-only; no act, no child, nothing signed"
    else
        wl_fail "OBL-003 a device POSTs the signing id" "$WL_R; act $WL_STATE_ACT; sign children $WL_KIDS"
    fi
    WL_RID=$(python3 -c 'import sys,json; print(json.load(sys.stdin)["acts"][0]["request_id"])' <<<"$WL_DQ" 2>/dev/null)
    WL_N=$(wl_nonce "$WL_AP" "$WL_T/jar1")
    WL_R=$(wl_curl "$WL_AP" -b "$WL_T/jar1" -H "Origin: $WL_AB" -H 'Content-Type: application/json' \
        -d "{\"id\":\"$WL_RID\",\"nonce\":\"$WL_N\"}" -w ' %{http_code}' "$WL_AB/api/request")
    WL_LQ=$(curl -s -H "Authorization: Bearer $WL_TOK" "http://$WL_LHOST/api/queue")
    if [[ "$WL_R" == *200 ]] && grep -q '"requested_from":"device ' <<<"$WL_LQ" \
        && [[ "$(curl -s -H "Authorization: Bearer $WL_TOK" "http://$WL_LHOST/api/act")" == '{"state":"idle"}' ]] \
        && [[ ! -f "$PLANT_ROOT/docs/warrants/WL-WAR-0001/authorization.toml" ]]; then
        wl_ok "OBL-003 a device asks for a signature" "marked 'requested from' at the host; nothing ran"
    else
        wl_fail "OBL-003 a device asks for a signature" "$WL_R"
    fi
    WL_REM=$(wl_curl "$WL_AP" -b "$WL_T/jar1" "$WL_AB/api/help" \
        | python3 -c 'import sys,json; print(next(r["act_id"] for r in json.load(sys.stdin)["remedies"] if r.get("act_id") and r["rule"]=="relations.child-listed"))' 2>/dev/null)
    WL_N=$(wl_nonce "$WL_AP" "$WL_T/jar1")
    WL_R1=$(wl_curl "$WL_AP" -b "$WL_T/jar1" -H "Origin: $WL_AB" -H 'Content-Type: application/json' \
        -d "{\"id\":\"$WL_REM\",\"nonce\":\"$WL_N\"}" -o /dev/null -w '%{http_code}' "$WL_AB/api/act")
    # t-470e: the contract is 202, then the act. The server logs the act and
    # sets the state to running before it answers 202; the remedy (`war
    # compile`, a child process) runs after, and GET /api/act says `done`
    # with its id and exit when it has finished. So wait for THIS act's
    # `done` — not any `done` — bounded in wall-clock time, not in polls
    # (a poll is slow on a loaded machine), require exit 0, and only then
    # ask `war check`. A failure names which of these did not hold.
    WL_WAIT_END=$((SECONDS + 180))
    WL_DONE=""
    while ((SECONDS < WL_WAIT_END)); do
        WL_DONE=$(curl -s -H "Authorization: Bearer $WL_TOK" "http://$WL_LHOST/api/act" | python3 -c '
import sys, json
s = json.load(sys.stdin)
if s.get("state") == "done" and s.get("id") == sys.argv[1]:
    x = s.get("exit")
    print("exit", x if x == 0 else "%s: %s" % (x, " ".join(str(s.get("output", "")).split())[:160]))' "$WL_REM" 2>/dev/null)
        [[ -n "$WL_DONE" ]] && break
        sleep 0.5
    done
    WL_R2=$(wl_curl "$WL_AP" -b "$WL_T/jar1" -H "Origin: $WL_AB" -H 'Content-Type: application/json' \
        -d "{\"id\":\"$WL_REM\",\"nonce\":\"$WL_N\"}" -w ' %{http_code}' "$WL_AB/api/act")
    sleep 1
    WL_LINES=$(grep -ac "war ui: act $WL_REM: war --root .* compile — from device" "$WL_T/A.log")
    WL_LEFT=$(grep -c '^ERROR relations.child-listed' <<<"$("$WAR" --root "$PLANT_ROOT" check 2>/dev/null)")
    if [[ -n "$WL_REM" && "$WL_R1" == 202 && "$WL_LINES" == 1 && "$WL_DONE" == "exit 0" && "$WL_LEFT" == 0 ]]; then
        wl_ok "OBL-003 an auto remedy with a fresh nonce" "202; ran war compile once; the error is gone"
    else
        wl_fail "OBL-003 an auto remedy with a fresh nonce" "id '$WL_REM' → $WL_R1; act log lines $WL_LINES; act ${WL_DONE:-not done in 180 s}; child-listed errors left $WL_LEFT"
    fi
    if [[ "$WL_R2" == *act.nonce-used*409 && "$WL_LINES" == 1 ]]; then
        wl_ok "OBL-002 an act replayed with its nonce" "409 act.nonce-used; the act log has one line"
    else
        wl_fail "OBL-002 an act replayed with its nonce" "$WL_R2; act log lines $WL_LINES"
    fi

    # ---- t-67ed: tickets on the LAN are read-only --------------------------
    # A device reads /api/tickets; its ticket act is 403 act.host-only and
    # writes nothing. The same act from the host's loopback page records.
    "$WAR" --root "$PLANT_ROOT" create "LAN ticket" --item "LAN item" >/dev/null 2>&1
    WL_TT=$("$WAR" --root "$PLANT_ROOT" --json ready 2>/dev/null | python3 -c '
import sys, json
r = json.load(sys.stdin)["result"]["ready"]
print(next(x["ticket"] + "/" + x["item"] for x in r if x["text"] == "LAN item"))' 2>/dev/null)
    WL_TV=$(wl_curl "$WL_AP" -b "$WL_T/jar1" -w ' %{http_code}' "$WL_AB/api/tickets")
    WL_TR=$(wl_curl "$WL_AP" -b "$WL_T/jar1" -H "Origin: $WL_AB" -H 'Content-Type: application/json' \
        -d "{\"act\":\"claim\",\"target\":\"$WL_TT\"}" -w ' %{http_code}' "$WL_AB/api/ticket")
    WL_TC=$("$WAR" --root "$PLANT_ROOT" --json show "${WL_TT%%/*}" 2>/dev/null | python3 -c '
import sys, json
print(len(json.load(sys.stdin)["result"]["ticket"]["claims"]))' 2>/dev/null)
    if [[ -n "$WL_TT" && "$WL_TV" == *'LAN item'*200 && "$WL_TR" == *act.host-only*403 && "$WL_TC" == 0 ]]; then
        wl_ok "t-67ed a device's ticket act" "reads 200; claim 403 act.host-only; no claim written"
    else
        wl_fail "t-67ed a device's ticket act" "target '$WL_TT'; view ${WL_TV: -3}; act $WL_TR; claims $WL_TC"
    fi
    WL_TL=$(curl -s -H "Authorization: Bearer $WL_TOK" -H "Origin: http://$WL_LHOST" -H 'Content-Type: application/json' \
        -d "{\"act\":\"claim\",\"target\":\"$WL_TT\"}" -w ' %{http_code}' "http://$WL_LHOST/api/ticket")
    WL_TC=$("$WAR" --root "$PLANT_ROOT" --json show "${WL_TT%%/*}" 2>/dev/null | python3 -c '
import sys, json
print(len(json.load(sys.stdin)["result"]["ticket"]["claims"]))' 2>/dev/null)
    if [[ "$WL_TL" == *'"ok":true'*200 && "$WL_TC" == 1 ]]; then
        wl_ok "t-67ed the same act on loopback" "200; the claim is recorded"
    else
        wl_fail "t-67ed the same act on loopback" "$WL_TL; claims $WL_TC"
    fi

    # ---- OBL-005: the loopback page of a --lan server is unchanged --------
    WL_GOT=$(wl_loopback_headers "$WL_LHOST" "$WL_TOK")
    if [[ "$WL_GOT" == "$WL_LOOPBACK_DIGEST" ]]; then
        wl_ok "OBL-005 loopback headers beside --lan" "byte-identical to 269d3324"
    else
        wl_fail "OBL-005 loopback headers beside --lan" "sha256 $WL_GOT"
    fi
    WL_LAN_H=$(wl_curl "$WL_AP" -D - -o /dev/null "$WL_AB/" | tr -d '\r')
    WL_LOOP_H=$(curl -s -D - -o /dev/null "http://$WL_LHOST/" | tr -d '\r')
    if [[ "$WL_LAN_H" != "$WL_LOOP_H" && "$(grep -v '^Strict-Transport-Security:' <<<"$WL_LAN_H")" == "$WL_LOOP_H" ]]; then
        wl_ok "OBL-005 the comparison can tell" "the LAN page differs by exactly HSTS"
    else
        wl_fail "OBL-005 the comparison can tell" "LAN and loopback headers are not HSTS apart"
    fi
fi
wl_stop A

# ---- OBL-002: an expired code, an expired credential ------------------------
wl_start B "y" --lan 127.0.0.2:0 --name "$WL_NAME" --cert "$WL_T/cert.pem" --key "$WL_T/key.pem" --pair-answer-secs 5 --pair-code-secs 3 --device-secs 3
WL_BP=$(wl_port B)
WL_B1=$(wl_code B)
WL_R=$(wl_pair B "$WL_B1" "$WL_T/jarB" plant-short/1.0)
WL_B2=$(wl_next_code B "$WL_B1")
WL_BFRESH=$(wl_curl "$WL_BP" -b "$WL_T/jarB" -o /dev/null -w '%{http_code}' "https://$WL_NAME:$WL_BP/api/version")
if [[ "$WL_R" == 200* && "$WL_BFRESH" == 200 ]]; then
    sleep 4
    WL_R=$(wl_pair B "$WL_B2" "$WL_T/jarB2" plant-late/1.0)
    if [[ "$WL_R" == 403*pair.code-expired* && -z "$(wl_cred "$WL_T/jarB2")" ]]; then
        wl_ok "OBL-002 a pairing code after expiry" "403 pair.code-expired, no credential"
    else
        wl_fail "OBL-002 a pairing code after expiry" "$WL_R"
    fi
    # Sent by hand: curl, like a browser, drops a cookie past its Max-Age,
    # and the question is what the server does with one presented anyway.
    WL_R=$(wl_curl "$WL_BP" -H "Cookie: __Host-war_device=$(wl_cred "$WL_T/jarB")" -w ' %{http_code}' "https://$WL_NAME:$WL_BP/api/version")
    if [[ "$WL_R" == *ui.device-expired*401 ]]; then
        wl_ok "OBL-002 an expired credential" "200 while fresh; 401 ui.device-expired after"
    else
        wl_fail "OBL-002 an expired credential" "$WL_R"
    fi
else
    wl_fail "OBL-002 expiry (server B paired)" "$WL_R; fresh read $WL_BFRESH"
fi
wl_stop B

# ---- OBL-004: no terminal at the host --------------------------------------
wl_start D notty --lan 127.0.0.2:0 --name "$WL_NAME" --cert "$WL_T/cert.pem" --key "$WL_T/key.pem" --pair-answer-secs 2
WL_DEVS_BEFORE=$(python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))["devices"]))' "$WL_STATE/openwarrant/ui-devices.json" 2>/dev/null)
WL_R=$(wl_pair D "$(wl_code D)" "$WL_T/jarD" plant-notty/1.0)
WL_DEVS_N=$(python3 -c 'import json,sys; print(len(json.load(open(sys.argv[1]))["devices"]))' "$WL_STATE/openwarrant/ui-devices.json" 2>/dev/null)
if [[ "$WL_R" == 403*pair.no-tty* && -z "$(wl_cred "$WL_T/jarD")" && "$WL_DEVS_N" == "$WL_DEVS_BEFORE" ]]; then
    wl_ok "OBL-004 no terminal at the host" "403 pair.no-tty, no credential"
else
    wl_fail "OBL-004 no terminal at the host" "$WL_R"
fi
wl_stop D

# ---- OBL-001, U-002 B: the labelled self-signed fallback --------------------
wl_start S "-" --lan 127.0.0.2:0 --name "$WL_NAME" --self-signed
WL_SP=$(wl_port S)
WL_FP=$(grep -ao 'SHA-256 fingerprint: [0-9A-F:]*' "$WL_T/S.log" | head -1 | awk '{print $3}')
WL_SEEN=$(openssl s_client -connect "127.0.0.2:$WL_SP" -servername "$WL_NAME" </dev/null 2>/dev/null \
    | openssl x509 -outform der 2>/dev/null | sha256sum | cut -d' ' -f1 | tr 'a-f' 'A-F' | sed 's/../&:/g; s/:$//')
WL_SCODE=$(curl -s --max-time 10 -k --resolve "$WL_NAME:$WL_SP:127.0.0.2" -o /dev/null -w '%{http_code}' "https://$WL_NAME:$WL_SP/api/version")
WL_SMODE=$(stat -c '%a' "$WL_STATE/openwarrant/ui-tls/$WL_NAME.key.der" 2>/dev/null)
if grep -aq 'U-002 option B, the fallback' "$WL_T/S.log" && [[ -n "$WL_FP" && "$WL_FP" == "$WL_SEEN" && "$WL_SCODE" == 401 && "$WL_SMODE" == 600 ]]; then
    wl_ok "OBL-001 --self-signed (U-002 B fallback)" "labelled; served cert = printed fingerprint; key 0600; /api/ 401"
else
    wl_fail "OBL-001 --self-signed (U-002 B fallback)" "fp '$WL_FP' vs '$WL_SEEN'; api $WL_SCODE; key mode $WL_SMODE"
fi
WL_SVERIFY=$(curl -s --max-time 10 --resolve "$WL_NAME:$WL_SP:127.0.0.2" --cacert "$WL_T/cert.pem" -o /dev/null -w '%{http_code}' "https://$WL_NAME:$WL_SP/"; echo " exit=$?")
if [[ "$WL_SVERIFY" == "000 exit=60" ]]; then
    wl_ok "OBL-001 --self-signed is not the operator cert" "a client trusting only the operator CA refuses it (curl 60)"
else
    wl_fail "OBL-001 --self-signed is not the operator cert" "$WL_SVERIFY"
fi
wl_stop S

# ---- OBL-005: loopback without --lan ----------------------------------------
WL_LO="$WL_T/loop.log"
env -u SSH_AUTH_SOCK XDG_STATE_HOME="$WL_STATE" "$WAR" --root "$PLANT_ROOT" ui --port 0 >"$WL_LO" 2>/dev/null &
WL_LPID=$!
for _ in $(seq 1 60); do grep -q 'http://' "$WL_LO" && break; sleep 0.25; done
WL_U=$(grep -o 'http://[^ ]*' "$WL_LO" | head -1); WL_H=${WL_U#http://}; WL_H=${WL_H%%/*}; WL_K=${WL_U#*#t=}; WL_K=${WL_K%%&*}
WL_GOT=$(wl_loopback_headers "$WL_H" "$WL_K")
if [[ "$WL_GOT" == "$WL_LOOPBACK_DIGEST" ]]; then
    wl_ok "OBL-005 loopback headers, no --lan" "byte-identical to 269d3324"
else
    wl_fail "OBL-005 loopback headers, no --lan" "sha256 $WL_GOT"
fi
kill "$WL_LPID" 2>/dev/null; wait "$WL_LPID" 2>/dev/null
if [[ "$(sha256sum conformance/plants.d/63-webui.sh | cut -d' ' -f1)" == 0397209b30e60b7aab726f1742ba7556c844e671f2df1a81692cd0d9ac376a79 ]]; then
    wl_ok "OBL-005 63-webui.sh unmodified" "its 269d3324 bytes; the battery runs it"
else
    wl_fail "OBL-005 63-webui.sh unmodified" "63-webui.sh moved"
fi

# ---- OBL-003: no key, socket or signing call under webui/ -------------------
# 63-webui.sh's pattern over every file, and the agent's tools over the Rust
# (the page's own text tells the owner to load the key with `ssh-add -c`).
#
# wl_signing_seams <dir>: true only when both greps RAN over <dir> (exit 0 or
# 1) and neither found a line that is not a comment; the lines found go to
# $WL_HITS. Each grep's status is its own: a hit from either is a refusal, and
# a grep that could not read <dir> (exit 2) is no answer (t-5134).
wl_signing_seams() {
    local a b sa sb
    [[ -d "$1" ]] || { WL_HITS="$1: not a directory"; return 1; }
    a=$(grep -rn 'ssh-keygen\|SSH_AUTH_SOCK\|sign::run(repo, Some\|ssh_sign_file\|authorize::ingest\|resolution_cmd::ingest' "$1")
    sa=$?
    b=$(grep -rn --include='*.rs' 'ssh-add\|SSH_AGENT\|ssh_agent' "$1")
    sb=$?
    WL_HITS=$(grep -v ':\s*//' <<<"$a"$'\n'"$b")
    (( sa <= 1 && sb <= 1 )) || { WL_HITS="grep exited $sa/$sb over $1"; return 1; }
    [[ -z "${WL_HITS//$'\n'/}" ]]
}
if wl_signing_seams "$WL_SRC"; then
    wl_ok "OBL-003 webui/ holds no signing authority" "no ssh key, agent socket or signing call"
else
    wl_fail "OBL-003 webui/ holds no signing authority" "webui/ names a key, socket or signing seam: $(head -2 <<<"$WL_HITS")"
fi
# The refusing case: a copy of webui/ with one planted signing call, which
# only the FIRST grep names. The copy's doc comment naming `ssh-add` is
# reworded so the second grep finds nothing at all: that is the case the old
# group hid, its status being the second grep's 1.
WL_PLANTED=$(mktemp -d)
cp -r "$WL_SRC" "$WL_PLANTED/webui"
sed -i 's/ssh-add/ssh add/g; s/SSH_AGENT/SSH AGENT/g; s/ssh_agent/ssh agent/g' "$WL_PLANTED/webui/"*.rs
printf '\nfn planted(repo: &Repository) { let _ = crate::sign::run(repo, Some(1)); }\n' >> "$WL_PLANTED/webui/mod.rs"
if ! wl_signing_seams "$WL_PLANTED/webui" && line_has -F 'mod.rs' -F 'sign::run(repo, Some' <<<"$WL_HITS"; then
    wl_ok "OBL-003 a planted signing call" "refused: $(head -c 80 <<<"$WL_HITS")"
else
    wl_fail "OBL-003 a planted signing call" "a sign::run call in webui/ was not found"
fi
# And a grep that could not read what it was pointed at (exit 2) is no
# answer: a clean copy of webui/ with one unreadable file, and no webui/ at all.
mkdir "$WL_PLANTED/unreadable"
cp -r "$WL_SRC/." "$WL_PLANTED/unreadable/"
printf 'fn nothing() {}\n' > "$WL_PLANTED/unreadable/closed.rs"
chmod 000 "$WL_PLANTED/unreadable/closed.rs"
if [[ $(id -u) -eq 0 ]] || ! wl_signing_seams "$WL_PLANTED/unreadable"; then
    wl_ok "OBL-003 a grep that cannot read" "refused: $WL_HITS"
else
    wl_fail "OBL-003 a grep that cannot read" "an unreadable file under webui/ passed"
fi
chmod 600 "$WL_PLANTED/unreadable/closed.rs"
if ! wl_signing_seams "$WL_PLANTED/absent"; then
    wl_ok "OBL-003 no webui/ to read" "refused: $WL_HITS"
else
    wl_fail "OBL-003 no webui/ to read" "an absent webui/ passed"
fi
command rm -rf "$WL_PLANTED"

command rm -rf "$WL_T"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
