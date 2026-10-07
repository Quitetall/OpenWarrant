# shellcheck shell=bash
# t-26ca — a refusal reaches the client, every time, under load.
#
# Its own file, beside 63-webui.sh rather than inside it: OW-WAR-0139's
# OBL-005 pins 63-webui.sh's bytes. The loopback server stops reading an
# oversized request at its bound; it used to close with the rest unread,
# the kernel answered RST, and a client that had not yet read the 431 lost
# it (curl status 000, or the 431 then "connection reset by peer"). Under
# load that happened to about one request in eight.
#
# Accepted: every one of RF_N oversized header requests is answered 431 and
# closed cleanly (curl exit 0), and likewise every oversized body is 413,
# with RF_PAR clients at once and a CPU burner per core running. Refused:
# the same requests are refusals — nothing is served, and the server is
# still up and answering a normal request after them. And (t-5f49) a
# socket that sends nothing is answered 408 when the time bound runs out,
# never 431, while the page is served beside it.

echo "== web ui refusals under load (t-26ca) =="
PLANT_ROOT=$(scratch_corpus RF)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
RF_N=50
RF_PAR=16
RF_OUT=$(mktemp); RF_ERR=$(mktemp); RF_BODY=$(mktemp)
env -u SSH_AUTH_SOCK "$WAR" --root "$PLANT_ROOT" ui --port 0 --as refusal-plant >"$RF_OUT" 2>"$RF_ERR" &
RF_PID=$!
for _ in $(seq 1 60); do grep -q 'http://' "$RF_OUT" && break; sleep 0.25; done
RF_URL=$(grep -o 'http://[^ ]*' "$RF_OUT" | head -1)
RF_HOST=${RF_URL#http://}; RF_HOST=${RF_HOST%%/*}
RF_B="http://$RF_HOST"
head -c 8000 /dev/zero | tr '\0' a >"$RF_BODY"

# rf_many <curl args...>  ->  "<count> <status> <curl exit>;" per outcome,
# RF_N requests, RF_PAR at a time.
rf_many() {
    seq 1 "$RF_N" | xargs -P "$RF_PAR" -I{} \
        curl -s -o /dev/null -w '%{http_code} %{exitcode}\n' "$@" 2>/dev/null \
        | sort | uniq -c | awk '{print $1, $2, $3}' | tr '\n' ';'
}

if [[ -z "$RF_URL" ]]; then
    printf 'FAIL  %-34s the server printed no URL: %s\n' "war ui starts" "$(tail -2 "$RF_ERR")"
    FAILED=$((FAILED + 1))
else
    # Load: one busy loop per core, each bounded by `timeout` so none can
    # outlive the plant, and killed by PID when the requests are done.
    RF_BURN=()
    for _ in $(seq 1 "$(nproc 2>/dev/null || echo 4)"); do
        timeout 120 sh -c 'while :; do :; done' &
        RF_BURN+=($!)
    done
    RF_BIG=$(head -c 9000 /dev/zero | tr '\0' a)
    RF_HDR=$(rf_many -H "X-Big: $RF_BIG" "$RF_B/")
    RF_BDY=$(rf_many -H 'Expect:' -H 'Content-Type: application/json' --data-binary "@$RF_BODY" "$RF_B/api/act")
    kill "${RF_BURN[@]}" 2>/dev/null; wait "${RF_BURN[@]}" 2>/dev/null

    if [[ "$RF_HDR" == "$RF_N 431 0;" ]]; then
        printf 'ok    %-34s %s of %s answered 431, closed cleanly\n' "oversized headers under load" "$RF_N" "$RF_N"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s count status exit: %s\n' "oversized headers under load" "$RF_HDR"; FAILED=$((FAILED + 1))
    fi
    if [[ "$RF_BDY" == "$RF_N 413 0;" ]]; then
        printf 'ok    %-34s %s of %s answered 413, closed cleanly\n' "oversized bodies under load" "$RF_N" "$RF_N"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s count status exit: %s\n' "oversized bodies under load" "$RF_BDY"; FAILED=$((FAILED + 1))
    fi
    # t-5f49: a socket that sends nothing does not hold the page — it is
    # read on its own thread — and when the time bound runs out it is
    # answered 408, not 431: it sent nothing too large. (Under load the
    # bound once ran out on a client starved of CPU, whose ordinary headers
    # and oversized body were answered 431.)
    RF_PORT=${RF_HOST##*:}
    if exec {RF_FD}<>"/dev/tcp/127.0.0.1/$RF_PORT"; then
        # Order, not a stopwatch: a client under load can be starved for
        # seconds, so the page's wall time proves nothing. The page must be
        # answered while the silent socket is still open and unanswered; a
        # server that read connections one after another would answer the
        # silent socket (at its time bound) before it got to the page.
        RF_BESIDE=$(curl -s -m 10 -o /dev/null -w '%{http_code}' "$RF_B/")
        RF_EARLY=$(timeout 0.5 head -c 1 <&"$RF_FD" | od -An -c | tr -d ' \n')
        if [[ "$RF_BESIDE" == 200 && -z "$RF_EARLY" ]]; then
            printf 'ok    %-34s the page is 200 while it is still unanswered\n' "a silent socket holds nothing"; PASSED=$((PASSED + 1))
        else
            printf 'FAIL  %-34s page %s; the silent socket had already been answered: %s\n' "a silent socket holds nothing" "$RF_BESIDE" "${RF_EARLY:-no}"; FAILED=$((FAILED + 1))
        fi
        RF_SILENT=$(timeout 30 head -c 200 <&"$RF_FD" | head -1 | tr -d '\r')
        exec {RF_FD}<&-
        if [[ "$RF_SILENT" == "HTTP/1.1 408 "* ]]; then
            printf 'ok    %-34s %s\n' "a silent socket is 408, not 431" "$RF_SILENT"; PASSED=$((PASSED + 1))
        else
            printf 'FAIL  %-34s got %q\n' "a silent socket is 408, not 431" "$RF_SILENT"; FAILED=$((FAILED + 1))
        fi
    else
        printf 'FAIL  %-34s could not open a socket to %s\n' "a silent socket holds nothing" "$RF_PORT"; FAILED=$((FAILED + 1))
    fi
    # The server survived them: the page is still served.
    RF_AFTER=$(curl -s -o /dev/null -w '%{http_code}' "$RF_B/")
    if [[ "$RF_AFTER" == 200 ]] && kill -0 "$RF_PID" 2>/dev/null; then
        printf 'ok    %-34s the page is still 200\n' "the server outlives the refusals"; PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s got %s\n' "the server outlives the refusals" "$RF_AFTER"; FAILED=$((FAILED + 1))
    fi
fi
kill "$RF_PID" 2>/dev/null; wait "$RF_PID" 2>/dev/null
command rm -f "$RF_OUT" "$RF_ERR" "$RF_BODY"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
