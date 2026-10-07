# shellcheck shell=bash
# The timeline and pending projections (slice D2): committed, drift-checked,
# deterministic, and built from tracked inputs only.
#
# On a scratch program, not this repository. These plants used to edit
# docs/warrants/generated/ here and lean on `restore` to put it back, and the
# determinism check ran `war compile` over the live corpus — so the moment this
# repository's committed projections fell behind its records, a recompilation
# legitimately changed the bytes and the check reported the projections as
# non-deterministic. They are not; the fixture was. A scaffolded program's
# committed projections match its records by construction, which is the only
# state in which that question can be asked.
PLANT_ROOT=$(scratch_corpus PJ)
W1=PJ-WAR-0001
GEN=docs/warrants/generated

plant "one byte moved in CORPUS_TIMELINE.json is drift" "corpus-timeline.drift" "CORPUS_TIMELINE.json" 2 \
    "sed -i 's/\"warrants\":/\"warrant_s\":/' $GEN/CORPUS_TIMELINE.json; assert_present 'warrant_s' $GEN/CORPUS_TIMELINE.json" \
    --generated

plant "one byte moved in CORPUS_PENDING.json is drift" "corpus-pending.drift" "CORPUS_PENDING.json" 2 \
    "sed -i 's/\"count\":/\"c0unt\":/' $GEN/CORPUS_PENDING.json; assert_present 'c0unt' $GEN/CORPUS_PENDING.json" \
    --generated

# A journal event added to a Warrant changes the timeline: the committed
# projection is then behind the records, which is drift, not silence.
plant "a new journal event makes the timeline stale" "corpus-timeline.drift" "CORPUS_TIMELINE.json" 2 \
    "printf '{\"v\":1,\"id\":\"01a00000-0000-7000-8000-00000000d2d2\",\"warrant_uuid\":\"%s\",\"type\":\"draft.revised\",\"class\":\"draft_history\",\"actor_ref\":\"agent://plant\",\"occurred_at\":\"2026-09-12T00:00:00Z\",\"payload\":\"{}\",\"idempotency_key\":\"plant-d2\"}\n' \"\$(grep '^uuid' docs/warrants/$W1/manifest.toml | cut -d'\"' -f2)\" >> docs/warrants/$W1/journal.jsonl; assert_present 'plant-d2' docs/warrants/$W1/journal.jsonl" \
    --generated

# Determinism: two compilations of one tree are byte-identical.
PROJ_TMP=$(mktemp -d)
cp "$PLANT_ROOT/$GEN/CORPUS_TIMELINE.json" "$PROJ_TMP/t1.json"
cp "$PLANT_ROOT/$GEN/CORPUS_PENDING.json" "$PROJ_TMP/p1.json"
"$WAR" --root "$PLANT_ROOT" compile >/dev/null 2>&1
if cmp -s "$PLANT_ROOT/$GEN/CORPUS_TIMELINE.json" "$PROJ_TMP/t1.json" \
    && cmp -s "$PLANT_ROOT/$GEN/CORPUS_PENDING.json" "$PROJ_TMP/p1.json"; then
    printf 'ok    %-34s two compilations are byte-identical\n' "the projections are deterministic"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s a recompilation changed the bytes\n' "the projections are deterministic"
    FAILED=$((FAILED + 1))
fi
rm -rf "$PROJ_TMP"

corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
