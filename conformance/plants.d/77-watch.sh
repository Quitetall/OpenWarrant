# shellcheck shell=bash
# `war watch` (slice D6): the pending set, once and as a diff.

# Once: the current pending acts, as an envelope with its own schema.
plant_cmd "watch --once lists the pending acts" "oh.war/watch/v1" "pending" 0 \
    "true" \
    --json watch --once

# A throwaway SAS revision appears in the set as soon as it is proposed.
plant_cmd "a proposed revision appears in the pending set" "0.1.0-draft.9" "accept" 0 \
    "\"$WAR\" sas propose 0.1.0-draft.9 >/dev/null 2>&1 || { echo 'sas propose failed' >&2; exit 9; }" \
    watch --once

# The loop: a bounded run exits on its own, and a change made while it runs
# is printed as a `+` line. The revision is proposed after the first poll.
# The first snapshot builds every pending request (a second or two on this
# corpus); the proposal lands after it, and the loop runs long enough to see it.
#
# The proposal waits for the watch's first snapshot to be PRINTED, not for a
# fixed four seconds: on a corpus with forty pending acts the first snapshot
# takes longer than that, the proposal landed inside it, and the loop — right
# not to call an act "new" that was there when it looked — printed no `+`.
WATCH_FILE=$(mktemp)
"$WAR" watch --interval 200 --ticks 60 > "$WATCH_FILE" 2>/dev/null &
WATCH_PID=$!
for _ in $(seq 1 200); do [[ -s "$WATCH_FILE" ]] && break; sleep 0.1; done
"$WAR" sas propose 0.1.0-draft.9 >/dev/null 2>&1
wait "$WATCH_PID"
WATCH_STATUS=$?
WATCH_OUT=$(cat "$WATCH_FILE")
command rm -f "$WATCH_FILE"
restore
if [[ $WATCH_STATUS -eq 0 ]] && grep -Fq -- '+ ' <<< "$WATCH_OUT" && grep -Fq -- '0.1.0-draft.9' <<< "$WATCH_OUT"; then
    printf 'ok    %-34s the loop printed the new act and exited\n' "watch prints what appeared"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s; output:\n%s\n' "watch prints what appeared" "$WATCH_STATUS" "$(head -6 <<< "$WATCH_OUT")"
    FAILED=$((FAILED + 1))
fi
