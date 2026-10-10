# Operator-selected collector activation

Prototype implementation under OW-WAR-0149. No independent verdict, human act,
participant agreement or release qualification is recorded.

`war authority activate-collector` validates exact signed enrollment against
current protected authority under the operator lock. It installs the selection
atomically, retains earlier signed objects and appends selection observations.
Exact replay does not change state. Current presence policy remains binding.
Observed UID and timestamps do not establish a human or trusted clock.

`LoadedEnrollment::load_active` selects from protected state, checks actual
execution UID and signatures, and rechecks authority and selection after crypto.
Each use rechecks current authority and selected content. The old `load` API
remains authentication-only. The host still must choose trusted roots, authenticate
the collector caller, protect/observe executable identity and fence native launch.

The explicit user/mount namespace fixture runs real normal CLI bootstrap and
activation. It observes: execution-user CLI activation refusal; invalid-signature
refusal with unchanged bytes; unchanged exact replay; inactive signed enrollment
remaining unavailable; retained replacement history; changed-selection refusal;
fresh reload; execution-UID mismatch refusal; and signed authority revocation.
Software fixture keys live only in memory; public signatures are fixture data.
No protected host deployment or human presence is established by this experiment.

Full CLI regression suite: 441 unit and 213 integration tests pass; four integration
tests remain explicitly ignored, including provisioned namespace entry points.
The explicit namespace roundtrip is separately observed passing. Strict CLI
all-target/all-feature Clippy passes after the retained initial style failures.
The initial command-missing refusal is retained. Full hosted candidate gates and
independent/participant qualification remain open.
