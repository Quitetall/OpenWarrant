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
WATCH_OUT=$( ( sleep 4; "$WAR" sas propose 0.1.0-draft.9 >/dev/null 2>&1 ) & "$WAR" watch --interval 200 --ticks 40 2>/dev/null )
WATCH_STATUS=$?
restore
if [[ $WATCH_STATUS -eq 0 ]] && grep -Fq -- '+ ' <<< "$WATCH_OUT" && grep -Fq -- '0.1.0-draft.9' <<< "$WATCH_OUT"; then
    printf 'ok    %-34s the loop printed the new act and exited\n' "watch prints what appeared"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s; output:\n%s\n' "watch prints what appeared" "$WATCH_STATUS" "$(head -6 <<< "$WATCH_OUT")"
    FAILED=$((FAILED + 1))
fi
