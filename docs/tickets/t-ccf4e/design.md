# Derived measurement candidate

This implements remaining measurement work under OW-WAR-0041. It does not replace its signed contract, reconstruct a missing pre-tuning observation, or establish an improvement. Default legacy telemetry artifacts remain unchanged. The opt-in `war telemetry --derived` report uses a separate candidate schema and never silently upgrades a retained baseline.

## Exact SDK inputs

`openwarrant_core::telemetry_metrics::derive_metric` takes one of the eight SAS names and two caller-observed terms. Each measured term supplies an integer value, measurement unit, bounded population and source reference. Missing observation supplies a reason. Labels disclose the caller's input; they do not authenticate it or prove that the input was actually measured. This pure operation performs no I/O, event discovery, authorization, tuning or assurance.

The caller must choose one consistent cohort and observation window. Missing follow-up cannot be relabeled as a completed observation. The output retains raw numerator and denominator counts and their sources, not a rounded percentage. `denominator_scale` records exact unit conversion. The value is `numerator / (denominator_scale * denominator)`; the implementation does not multiply counts or convert to floating point. Zero denominator is undefined and remains `not_measurable_yet`. A measured zero numerator over a nonempty population remains a measured zero.

| SAS metric | Numerator population | Denominator population | Units and bound |
| --- | --- | --- | --- |
| human control minutes per accepted WAR | Instrumented human control time for the same accepted cohort | Accepted Warrants in that cohort | Milliseconds / Warrant; scale 60,000; may exceed one minute |
| amendments per WAR | Recorded amendment records belonging to the cohort | Warrants examined | Counts; may exceed one |
| safe auto-amendment fraction | Amendments independently assessed eligible for safe automation under the effective policy | Amendments actually assessed under that same policy | Subset fraction, at most one; a permission alone is not an assessment |
| gate-failure-to-repair success rate | Failed gate cases with an observed successful repair | Failed gate cases with complete repair follow-up | Subset fraction, at most one |
| post-resolution escape rate | Resolved Warrants with a confirmed escaped defect in the observed window | Resolved Warrants examined through that same window | Subset fraction, at most one; count affected Warrants, not defect events |
| untracked-work rate | Commit candidates lacking a tracking identifier | All examined commits in the same history range | Subset fraction, at most one; candidates are not automatic attribution |
| adequacy-review catch rate | Recorded reviews reporting a counterexample | Recorded adequacy reviews with outcomes | Subset fraction, at most one; reported outcomes do not establish independent assurance |
| gate-library reuse rate | Recorded gate applications/citations in the examined cohort | Gate definitions in the stated registry | Applications per definition; may exceed one; not the fraction of gates reused |

Wrong units, undeclared names, empty population/source labels, missing-reason omissions and impossible subset counts are refused. Caller-supplied valid counts still require actual event evidence before any empirical claim.

## Collector boundary

The opt-in CLI report reads an exact retained Git subject, not simply label mutable checkout data with `--commit`. Its history numerator and denominator use one frozen endpoint and the same adoption baseline. It discloses measurable native populations and preserve specific unknown reasons for missing instrumentation. An absent directory, malformed source or failed history read must not become a measured zero. Existing baseline bytes must not be overwritten by candidate output.

The candidate CLI and public refusal controls are implemented. Actual repository observations and complete batch gates remain to be recorded. Historical pre-tuning baseline reconciliation and missing human/event instrumentation remain separate unfinished Warrant scope. Full release gates and independent/human acts remain separate.
