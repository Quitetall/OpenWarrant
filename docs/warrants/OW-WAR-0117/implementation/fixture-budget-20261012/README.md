# Verifier fixture dispatch budgets

CI run 38219821402 at a99f2b9f failed four registered fixture checks after
the CLI correctly refused oversized packets. The same cases reproduced locally:
12 passes, four failures. No production budget enforcement was removed.

The 6000-token splitting fixture copied the entire fixture catalogue into its
source tree, adding unrelated bindings. It now copies only the actual backend
script. Its original 6000-token fit assertions, 100-token refusal and
out-of-bundle verdict refusal remain unchanged. Its positive run still ingests
one response per obligation, two total.

The whole-repository OW63 synthetic ingestion fixture includes exact required
context that can exceed the repository's 48000-token runtime default: three
observed packets had irreducible estimates above 407000 tokens. Its local stub
now gets an explicit bounded 1000000-token fixture budget for ingestion and
self-verifier tests. This is a test input, not a production limit or a claim
that a real model can read these packets. The default and all dedicated low-cap
refusals are unchanged. No model was called.

The registered budget, ingestion and dispatch-admission plants then passed:
17 passes, zero failures. Successful ingestion retained three exact prior
records; self-verifier, outside-scope and over-budget refusals were observed.
This does not qualify actual independent review or complete the parent Warrant.
Full CI of the corrected candidate remains required before merging.
