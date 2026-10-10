# Collector integration with alpha.3

Main `f35c8e777aa8fd1105efa250e5f45d0037f6b800` contains the merged
alpha.3 ergonomics work. This candidate merges that main into the collector and
prospective authority-admission implementation. Generated conflicts were resolved
by compiling authored sources with an alpha.3 executable snapshot.

The first regeneration accidentally selected an alpha.2 executable overwritten
by a concurrent shared-cache build. It refused the valid new profile `encoding`
field. The record was not changed. A fresh alpha.3 executable was captured before
regeneration; shared build outputs are not stable source identities.

Alpha.3 consolidates CLI integration tests into `tests/all.rs`. The two new
collector test files are now explicitly registered there. Namespace callback
names follow the module path; the parent requires both actual UID observations,
so a subprocess running zero tests cannot report success.

Three authority CLI unit tests, seven collector integration tests, seven JSON
envelope tests, the explicit separate-UID namespace roundtrip and strict
all-target/all-feature CLI Clippy pass. Two namespace entries remain deliberately
ignored in the default suite and are exercised explicitly. Their software-signed
fixture is not human approval or a qualified deployment. Full hosted CI on the
combined candidate is required; collector activation, native wiring, launch
fencing and operator/independent qualification remain open.
