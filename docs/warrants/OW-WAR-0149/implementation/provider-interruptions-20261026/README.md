# Provider interruption follow-up

Katana's budget-refusal fix is merged in PR17 at 9851b8d. Cancellation and
finalization support is merged in PR18 at ced1d4a. Both candidates passed all
19 local checks and all five hosted jobs. The event/receipt formats, digest
domains, broker limits, unknown cost and assurance meaning are unchanged.

The kernel now publishes receipts for requested tools discarded before dispatch:
budget refusal requires a halted outcome and exhaustion evidence; cancellation
requires a cancelled outcome and cancellation evidence; finalization requires
the runtime phase record. None adds permissions or artifact effects. A final
response finishing its native turn does not establish that discarded work or
Warrant outcomes were completed. Unknown ids, invented success/effects, missing
stop evidence and unfinished requests remain refusals. The focused follow-up
suite passed 18 tests, with one instrumentation child ignored.

The new reader validated four retained real-model receipts: three completed
turns and one halted turn. The fifth real-model capture had no published receipt
and remains unvalidated. False assurance refused. These are historical-reader
compatibility observations, not fresh model execution with the patched producer.
Original producer source identity remains unestablished; the expected heads are
test instrumentation, not protected production custody. Global harness installs
and LAMU routing were not changed. No model or paid API call was used here.

Earlier failed runs and observations remain intact. The old publication defect
is fixed in provider source; current operator execution, protected collection,
confinement, mandatory cost accounting, BLUT agreement and independent/human
qualification remain separate work. W149 remains in progress and unqualified.

Provider source and binary hashes, original failed and passing tests, full local
gate results and compatibility bytes are retained under
`/mnt/4tb/artifacts/OpenWarrant/keep/katana-native-interruptions-20261024/`.
