# Actual native receipt preservation observation

Unverified continuation of OW-WAR-0111. The exact candidate is
`7c92af39e761a3da6c1ffaea20f014073b8e9e36` (Rust 1.97.1).
A new signature-free prototype was created through public `war init --vibe`,
`war plan new` and `war admin dispatch --prototype`. The retained BLUT fixture
producer then performed an actual CPU `connector_object` reference declaration.
It did not fetch an object, call a model, spend money, authorize a Warrant or
establish confinement. Its signing key is disposable public test material;
matching that key does not authenticate a real operator.

The native verifier returned `validated` with `assurance: not-established`.
It rejected an altered signature and a same-length change to an actual output.
The output was restored after the refusal control. Local capture import retained
exact receipt bytes against the recorded Dispatch and compile event. Capture
native standing remained UNKNOWN because no operator-enrolled verification
interface was supplied. The separately observed native verdict does not change
that standing.

The source and its two local commits were exported with `--history`, then the
source was moved out of its original location. Querying the unchanged archive
reconnected the current capture to exact Dispatch, compile event, contract and
stage. The receipt decoded from that archive is byte-identical to the actual
native receipt and passed native verification again. Retained provider inputs,
outputs and exact producer executable are separate dependencies of that native
check; the archive query itself does not authenticate a provider.

## Import still refuses

Whole-archive import returned `preservation: required coverage unavailable`.
`runtime receipt refs` requires a BLUT-specific preservation resolver; the
current whole-coverage path supports service stages. `schema and compiler
identity` is unavailable because the fresh public init did not install a schema
pack. The historical capture's exact contract/stage also remains unresolved:
its historical commit contains no generated IR. Its Dispatch bytes reconnect,
but this is not permission to infer the missing historical IR from current data.
No coverage declarations were weakened, fabricated or hand-edited.

The older actual fixture archive also remains unchanged. It contains two
conflicting contract digests for source revision 1: the fixture's pre-existing
conformance authorization differs from the later prototype contract. The KF
binding reader correctly refuses this conflict. The new public prototype avoids
that conflict; it does not rewrite the older fixture's history.

These observations prove bounded native receipt retention and refusal controls,
not an import/re-export roundtrip, a native KF roundtrip, independent verification,
human acceptance or release qualification. OW111 remains in progress. Next work:
retain native provider references through a provider-specific resolver, supply
checked schema identity during onboarding, preserve exact historical IR, then
exercise a real empty-instance KF roundtrip without treating test trust as human
approval.

## Evidence and reproduction

`observation.json` identifies raw evidence and archive/receipt digests.
`fresh-native-history.archive.json.gz` retains exact exported bytes;
`fresh-detached-native-basis.json.gz` records the query, including historical gaps.
`fresh-native-import.json` records the explicit refusal. Native verdicts and
control results are beside this file. The older conflicting archive and its
query are retained separately as `native-capture.archive.json.gz` and
`native-basis.json.gz`.

Decompress the fresh archive to a new file and run:

```sh
war archive runtime-basis /absolute/path/fresh-native-history.archive.json --json
```

Do not expect `war archive import` to accept this deliberately incomplete archive.
The raw evidence directory retains native public key, plan, binding, receipt and
job files. The exact producer and verifier identified in
`native-binary-identities.json` remain in the earlier
`blut-native-roundtrip-20261009` evidence directory. No private key is exported.
