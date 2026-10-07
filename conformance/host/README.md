# conformance/host — the `oh.war/liminal-v1` parity suite

Request/response pairs for `war host` (OW-WAR-0148 M8; SAS §82.2–82.4;
docs/LIMINAL_HOST.md). They are the fixtures a Liminal host of the WAR domain
must pass (Liminal SAS §100, LIM-SAS-RQ-015), and the ones the standalone
compiler passes now: standalone and hosted runs are checked against the same
cases (SAS §11.3).

## Layout

```text
cases/<name>/request.json      one oh.war/liminal-v1 request, as sent on stdin
cases/<name>/response.json     the response, exact bytes (stdout, newline-terminated)
cases/<name>/observables.json  the declared semantic observables
run.sh [CASES_DIR]             runs every case; exit 0 only when all match
regen.sh [WAR]                 rewrites responses and observables from the requests
make-cases.sh [WAR]            builds the cases from a scratch program
```

## Observables, declared before any comparison

**Byte observable.** The whole response, byte for byte. A host passes a case
only when its stdout equals `response.json` exactly. Semantic agreement never
replaces this.

**Semantic observables** (`observables.json`), each compared exactly:

| key | what it is |
|---|---|
| `exit` | the process exit code: 0 compiled, 1 refused, 2 compiled and not established |
| `outcome` | `compiled`, `not_established` or `refused` |
| `refusal` | the refusal's rule (`host.version`, …), or null |
| `basis_digest` | the basis digest the host recomputed and echoed |
| `model_digest` | sha256 of the model's compact JSON bytes |
| `records`, `relations`, `states` | counts in the compiled `oh.war/model/v1` |
| `host_rules` | the sorted set of host diagnostic rules |
| `projections` | `[path, digest]` of every rendered projection |

`run.sh` also recomputes `model_digest` from the response's model, so a model
edited without its digest is caught.

## The cases

| case | exercises |
|---|---|
| `records-password-reset` | the password-reset record atom (docs/records/) under the delivery profile |
| `ticket` | a ticket whose first item `implements REQ-pr1` |
| `warrant` | a second, unsigned Warrant; every Warrant's views requested and rendered |
| `nodes-differ` | a Node held at a revision the basis does not compile to: `host.node-differs`, exit 2 |
| `member-withheld` | a member the readers need, held by digest only: `host.member-withheld`, exit 2 |
| `no-repository` | a basis with no `openwarrant.toml`: `host.not-compiled`, exit 2 |
| `refuse-path-not-bytes` | a member naming a `file` instead of bytes |
| `refuse-member-without-bytes` | a member with neither bytes nor `withheld` |
| `refuse-basis-locator` | a basis naming a `root` |
| `refuse-version` | version 2 |
| `refuse-protocol` | another protocol id |
| `refuse-limit` | a declared `members` limit the basis exceeds |
| `refuse-member-digest` | bytes that do not digest to the declared digest |
| `refuse-basis-digest` | a basis digest that is not the members' |
| `refuse-member-path` | a member path with `..` |
| `refuse-unknown-field` | a field the protocol does not define |
| `refuse-malformed` | a request that is not JSON |

Every refusal is named before any work, and nothing is written.

## Running it against another host

```sh
HOST="liminal host-war" bash conformance/host/run.sh
```

`HOST` is any command that reads one request on stdin and writes one response
on stdout. The default is this repository's `target/debug/war host`.

`conformance/plants.d/79-host.sh` runs the suite, shows a doctored response
failing its case, and compares `war model` with `war host --export | war
host` on this repository and on a scratch program, byte for byte.

## When the compiler changes

A response is an expectation of the pinned compiler. When the model changes
on purpose, `bash conformance/host/regen.sh` rewrites the responses and
observables from the committed requests; review the diff like code.
`make-cases.sh` replaces the requests too (its program is new each run).
