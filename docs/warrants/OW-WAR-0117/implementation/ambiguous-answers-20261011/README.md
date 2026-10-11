# OW117 ambiguous-answer refusals

The blind-verifier wrapper previously accepted the last duplicate JSON field or
verdict for an obligation. A conflicting answer could therefore yield an
`established` result. The wrapper now treats either ambiguity as unusable and
returns `not_established`. Existing response schemas, identity flags, signatures,
and partial-answer handling are unchanged.

The final selected suite passed **15 controls, zero failures**: the positive
compiled corpus and 14 fake-backend controls, including both new refusals. Shell
syntax and diff checks also passed. These are performer observations, not an
independent verification or real-model judgment. No model was called.

[Exact source and log identities](observation.json), [initial failures](red.log.gz),
[first corrected controls](green.log.gz), and [final run](final.log.gz) are retained.
Initial runs also failed the positive corpus check because the new ticket had not
yet been compiled. After regeneration it passed. The final rerun waited on a
filesystem journal commit, then completed normally within its 480-second limit.

Independent review, human resolution and the previously measured oversized review
presentation remain pending. Existing historical verification records stay intact.

The follow-up progress refresh also met disk I/O pressure: the first compile did
not produce a report within its 120-second limit. Its subsequent check correctly
reported JSON/HTML corpus drift for the changed work report. A serialized compile
retry completed; this did not change source claims or suppress a checker rule.
