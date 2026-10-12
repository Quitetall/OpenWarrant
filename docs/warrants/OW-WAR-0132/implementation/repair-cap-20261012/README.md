# Repair caps during recovery

With `max_repairs = 3`, fixture performances produced `answered, answered, failed`. After the configured cap was lowered to 1, the old admission logic checked only the recovery limit: it started another actor and accepted a third answer. The before observation retains the actor marker and packet/journal count increase.

The fixed admission checks the number of accepted submissions against the current repair cap even when the attempt is classified as recovery. The same call now reports `perform.repair-limit`, starts no actor, compiles no Dispatch and adds no journal claim or ending. Raising the cap to 2 admits the pending recovery. Attempt labels, default limits and frozen record formats retain their existing meanings.

All 20 selected Linux hotline/default controls pass on the final rebuilt binary. This includes human-only answers, no-responder UNKNOWN, affected-stage blocking, spend refusal, eleven endings with unknown spend, separate repair/recovery counts and the new lowered/raised-cap pair. All seven performer unit tests and CLI all-targets Clippy with warnings denied pass. The first Clippy run refused a nested conditional; its output is retained separately from the passing final run.

The standalone helper omits the full-corpus positive fixture. These observations are not the full registered gate, independent verification, macOS qualification or human acceptance. The before observation used a retained binary whose dirty build identity is recorded; do not treat it as a clean committed build. The final binary was rebuilt from the dirty source candidate and carries exact source/binary hashes.
