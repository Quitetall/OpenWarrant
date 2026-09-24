#!/usr/bin/env bash
# A performer that keeps writing the tree until something kills it, and starts
# a child that does the same (OW-WAR-0131). It is the observation OBL-001 and
# OBL-002 rest on: a marker file that grows while any writer is alive, and
# stops growing once none is.
#
# Every 100 ms it appends `performer <pid> <n>` to $WRITES_UNTIL_KILLED_MARKER,
# and its child appends `child <pid> <n>`. The pids are in the lines so a plant
# can ask whether each writer is still alive, and kill it if the control under
# test did not. It never answers: the only way it stops is a kill.
set -uo pipefail
marker="${WRITES_UNTIL_KILLED_MARKER:?WRITES_UNTIL_KILLED_MARKER names the marker file}"
# Take the Dispatch so `war` is not left writing into a pipe nobody reads, and
# so a writer started by hand with stdin from /dev/null behaves the same.
cat > /dev/null

write_forever() {
    local who="$1" n=0
    while :; do
        n=$((n + 1))
        printf '%s %s %s\n' "$who" "$BASHPID" "$n" >> "$marker"
        sleep 0.1
    done
}

# The child: a separate process in the same process group, the way a model's
# tool call would be. It inherits no stdout, so it does not hold `war`'s pipe.
write_forever child > /dev/null 2>&1 < /dev/null &
write_forever performer
