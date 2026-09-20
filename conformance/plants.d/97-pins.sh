# shellcheck shell=bash
# A pin is a promise only once something binds it.
#
# `deliverable.digest-drift` used to be an ERROR for every Warrant in every
# state. In a repository whose Warrants are written at the START of the work —
# which is what a durable record of intent is for — that means ten ERRORs and
# `NOT READY` on every commit, forever, guarding nothing: no resolution had
# accepted those bytes, so there was no promise for the drift to contradict.
# An operator who sees the same red on every run stops reading it, and the same
# rule does matter for a resolved record.
#
# The line is the resolution: its §56.2 record carries
# `artifact_manifest_digest` = sha256(deliverables.toml). Below it, drift is a
# WARN naming `war pins --refresh`. At it, drift is an ERROR and only
# `war correct` moves the pin.

PIN_RESOLVED_ALIAS="OW-WAR-0061"
PIN_RESOLVED_FILE="docs/roadmap/PHASE1_EXIT.md"
PIN_DRAFT_ALIAS="OW-WAR-0068"
PIN_DRAFT_FILE="docs/SKILLS.md"

# A resolved Warrant's pinned file, moved: the wall is where it always was.
plant "a resolved pin moved" "deliverable.digest-drift" "$PIN_RESOLVED_ALIAS" 2 \
    "printf '\n<!-- planted -->\n' >> $PIN_RESOLVED_FILE; \
     assert_present 'planted' $PIN_RESOLVED_FILE"

# The same edit under a Warrant nothing has resolved: reported, not refused,
# and the remedy is one command rather than a human signature.
plant "an unresolved pin moved" "deliverable.pin-stale" "war pins --refresh" 0 \
    "printf '\n<!-- planted -->\n' >> $PIN_DRAFT_FILE; \
     assert_present 'planted' $PIN_DRAFT_FILE"

# `war pins --refresh` re-records the unresolved pin and the check goes quiet.
plant_cmd "refreshing an unresolved pin" "pins.refreshed" "$PIN_DRAFT_ALIAS" 0 \
    "printf '\n<!-- planted -->\n' >> $PIN_DRAFT_FILE; \
     assert_present 'planted' $PIN_DRAFT_FILE" \
    pins --refresh --alias "$PIN_DRAFT_ALIAS"

# It refuses the resolved one by name, so the escape cannot become the laundry.
plant_cmd "refreshing a resolved pin" "pins.signed" "war correct" 2 \
    "printf '\n<!-- planted -->\n' >> $PIN_RESOLVED_FILE; \
     assert_present 'planted' $PIN_RESOLVED_FILE" \
    pins --refresh --alias "$PIN_RESOLVED_ALIAS"
