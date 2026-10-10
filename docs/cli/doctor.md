# Repository doctor

`war doctor [WARRANT] [--generated] [--json]` runs read-only diagnostics. It reuses
record checks and the legacy stage frontier, reports authority parse failures and
missing CLI performer/verifier configuration, and supplies diagnostic commands.
It never runs configured backends, executes fixes, signs, or changes records.

JSON uses the report envelope with an `oh.war/doctor/v1` result. `remedies` contains
argv arrays and purposes, not shell scripts. Review the cited source; a repair to a
signed document needs its normal amendment route. Nonzero exit means diagnostic
errors or unavailable required observations, not permission denial or failed work.

Scope is explicit: unified admission, protected authority, signer custody and
backend availability remain UNKNOWN. No execution permission or assurance is issued.
Missing optional professional setup is a warning, not a new prototype start gate.
Configured executable names do not prove availability, independence or isolation.

It also reports the managed pointer block in the root `AGENTS.md` and `CLAUDE.md`
(M16): `doctor.agents-block` when a file's block is the one this `war` writes,
`doctor.agents-block-missing`, `doctor.agents-block-stale` (an older `war`'s, or
edited between the markers) and `doctor.agents-block-malformed`, each a warning that
`war agents-md --block` resolves. A block a newer `war` wrote is
`install.version-skew`, not stale.

`--generated` adds drift checks. Reports do not run acceptance gates or regenerate
files. Doctor-level unavailable reads are UNKNOWN; malformed parsed records are ERROR.
Inherited check diagnostics remain unchanged: some legacy file-read checks classify
unavailable reads as errors. Full I/O classification convergence remains open.
The existing frontier is a milestone projection, not proof of complete eligibility.
