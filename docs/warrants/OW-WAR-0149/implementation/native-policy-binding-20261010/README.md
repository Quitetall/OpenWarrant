# Native assessment policy binding — implementation evidence

Unverified work under OW149. No human acceptance or independent verdict.

`NativeAssessment::policy_binding()` now returns an authenticated repository and
collector, the current authority revision, active signed-enrollment identity,
and exact protected host/native configuration byte identities. Deserializing
this object grants nothing. Loading an assessment checks consistent policy
before and after adapter construction; resolving a held verifier rechecks it.
Surrounding checks do not provide atomic writer or launch fencing.

The existing Linux UID-separated namespace fixture observed exact identities
and refusals after protected native-policy, host-account, active-enrollment and
signed authority changes. It restored exact host bytes before replacing the
collector, so that refusal tests enrollment independently. Both actual Katana
and BLUT CPU/software native fixtures passed this same path. Four ordinary
collector signature tests passed; two provisioned namespace entries were ignored
in that ordinary run and exercised separately. These are fixtures, not production
operator, real hardware presence, spend accounting or human qualification.

Format and all-target/all-feature CLI Clippy passed. A direct broader runtime
invocation omitted Cargo's benign `OPENWARRANT_NO_PROJECTS=1` marker; the
non-inheritance test failed with `NotPresent`, and the suite timed out with 26
observed passes. The correctly configured isolated non-inheritance case passed.
The two-thread broad rerun was interrupted after slow archive tests; its overall
result is UNKNOWN. Neither partial run is reported as a passing suite. The first
build also timed out; later builds completed. `observations.json` records exact
statuses and hashes, and retains both failed and interrupted logs.

Tests ran over the source bytes subsequently committed at the recorded
implementation revision, with a base-revision/dirty binary version label.
A full CI gate on the published head remains required before merge.

Native versioned response signing, ingestion, durable record readers, fresh
acceptance reassessment and workbench identity mapping remain work. The v1
signing path is unchanged; native response ingestion still reports unavailable.
OW95 has no participants; no study session is invented.
