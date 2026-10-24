# W148 differential audit — owner direction recorded

Implementation is reported complete under the owner's chosen applicability
behavior, unverified. W148 is not resolved or qualified. Its original work order
and OBL-001 require identical output before M1 and after M2, except the named
state-disagreement fix. M2 also requires types without `stages` to report runtime
receipts as not applicable, never passed. The owner explicitly chose to permit
those applicability changes; adopting that direction in the signed criterion is
still a separate governed act.

The [full historical follow-up](full-scale/README.md) now retains all eight
comparisons and the passing original scale budget. Real-corpus differences,
including derived-view drift and the generated-check exit change, are preserved.
This report does not call those bytes equal or declare the original signed
obligation satisfied. Implementation completion does not award assurance.

## Initial audit observations

The exact historical commits were rebuilt without source changes using Rust
1.97.1. The [observation](observation.json) retains the identities and raw-stream
hashes. Full original output remains in the named durable artifact directory.

On the same frozen corpus, four of seven completed stdout comparisons differ:
`check`, `check --json`, `status --json` and `next`. The attached diffs show
OW-WAR-0071, a decision Warrant, gaining `capability.not-applicable`, a
`not_applicable` entry, removal of runtime receipts from `unmet`, and the derived
diagnostic count change. This is outside the sole allowed state-fix exception.
All seven observed stderr streams and exit codes match. Console JSON, signing
list and resolved pins are byte-identical.

Pre-M1 `check --generated` exceeded the 120-second command deadline: UNKNOWN,
not a successful comparison. At this initial stage, the 1,000-Warrant corpus
and release budget had not yet been reproduced. The full-scale follow-up now
records them. No exception was applied automatically to these raw observations.

## Owner direction — bounded clarification

The owner answered the recorded question: **“Explicitly allow these
applicability changes (Recommended).”** This is a direct design choice in chat;
it is not a signature, an independent verdict, or activation of an amendment.

The proposed differential criterion therefore permits the planned capability
applicability changes: a profile without execution stages may report runtime
receipts as not applicable. Each difference must be listed and bounded. For the
observed corpus, the exception covers only OW-WAR-0071's applicability diagnostic,
its derived count, the `not_applicable` entry, and removal of that requirement
from unmet lists. It does not turn an absent check into a pass. The original
state-disagreement exception and every other byte-comparison requirement remain.

The original observations above remain unchanged: the outputs were not equal.
This clarification does not waive missing observations, the scale budget,
functional controls, independent assessment or human acceptance. Signed atoms
and historical records are unchanged. The changed criterion must be represented
in any governed successor/amendment before claiming the signed obligation met.

The initial shared-cache build failure is retained separately. Revision-specific
caches produced both binaries successfully. Do not alter historical source to
work around stale build artifacts, and do not clean a shared cache while another
checkout is building.

## Functional controls already observed

The protected [CI run 40033704288](https://github.com/Quitetall/OpenWarrant/actions/runs/40033704288/job/120688228332)
passed all 1,711 plant controls at source `1f0fa2e34ef608073ac439653dd723ac9609c7b1`.
The [excerpt](functional-ci-excerpt.log) retains W148's model, capabilities,
records, states, filters, issue-adapter and demonstration observations, including
refusals. The [source comparison](functional-ci-observation.json) checks that
all six named plants are byte-identical here. Committed changes since that
checked head are W147 documentation and progress projections only.

These are observations of functional controls, not independent dispositions.
They support OBL-002–OBL-011's named controls and the doctored-output refusal;
they do not settle OBL-001's historical comparison or performance requirements.

The later shared-preparation implementation also has [observed controls](shared-preparation-ci-excerpt.log)
in that same protected run: shared execution, changed inputs/contracts/gate
identity, Warrant-specific work, failed gates, different orders, an earlier
failure and a second-call timeout. This addresses the implementation that the
original M1 notes explicitly said was still absent. It supplies no historical
byte-equality or performance result.

## Release-profile comparison

Both exact historical commits also built successfully in release mode, with
clean source afterwards. The [release identities and observation](release-base/observation.json)
retain their binary hashes, all command states, timings and raw-stream hashes.
Seven command pairs completed. Both `check --generated` executions reached the
unchanged 120-second command deadline; their comparisons remain UNKNOWN.

The four changed stdout diff bodies match the earlier audit exactly. The
[classification](release-base/exception-classification.json) compares diff bodies
only to identify the same bounded applicability change; it does not rewrite raw
streams, claim byte equality, activate an amendment or establish assurance.
The signing list, console JSON and resolved pins remained byte-identical.

These timings describe this frozen repository, not the required 1,000-Warrant
budget. The synthetic corpus and unchanged budget gate were subsequently
completed in the linked full-scale follow-up; earlier observations stay intact.

## Separate standalone JSON diagnostic

A clean standalone clone of the same `d2171045…` tree produced a completed
**JSON** generated-check report: the old release returned exit 0 after about
248 seconds; the post-M2 release returned exit 2 after about two seconds.
The [raw diff](standalone-diagnostic/generated-json.diff) includes the
applicability diagnostic and three `corpus-status.drift` findings for the
Markdown, JSON and HTML views. The [capture](standalone-diagnostic/pre-capture.json)
retains the longer observation guard and makes no release-budget claim.

This is a diagnostic variant with `--json`, not the exact human-output
`check --generated` command required by OBL-001. Earlier timeouts remain. The
clone also differs in Git layout, local origin and absent ignored build outputs;
these observations do not establish the timeout's cause. No extra compatibility
exception was assumed in this diagnostic. The later full-scale follow-up bounds
the fresh-rendered view differences to the chosen applicability change and
records the exact human-output command comparison. Raw reports remain in the
durable artifact directory named by the audit.
