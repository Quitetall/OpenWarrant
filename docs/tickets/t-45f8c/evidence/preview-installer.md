# Preview installer assets

Scope: OW-WAR-0143 publishing workflow. This fixes future previews; the published alpha.4 installer assets were repaired separately.

The regression executes the actual `Publish only the checked alpha preview` shell from `.github/workflows/release.yml`. Candidate validation, checksum calculation and asset selection run for real. Git and GitHub are stubs: this test makes no publication or native-execution claim.

Observed failure before the fix: the upload contained only the six fixture assets under `dist/`, without `install.sh`. The fixed workflow hashes this checkout's installer and uploads it and `install.sh.sha256` before publishing the existing draft. The refusal case removes the installer while retaining the earlier checksum; the workflow stops without upload or publication.

Validation: 29 release tests passed, including both capsule versions, native receipt checks, source limits, corruption and false-qualification refusals. Generated-record validation: 2,030 PASS, 394 WARN, 0 UNKNOWN, 0 ERROR. Record validation is separate from execution and release qualification.

No human authority, Warrant resolution or assurance mark is supplied here. Protected CI and integration remain pending.
