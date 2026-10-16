# Independent verifier commands

`war verify <alias> --run` calls the command in `[verify].verifier_argv` once
per generated bundle. A command reads its final argument, the bundle path,
and prints one `oh.war/verification-response/v2` document. OpenWarrant checks
the returned subject and packet bindings before recording any verdict.

`claude-verifier.sh` uses Claude Code with no tools or MCP access. It does not
enforce a dollar cap. Do not run it when paid calls require reliable spend
tracking unless the surrounding execution service supplies that control.

## Local LAMU verifier

`local-verifier.py` sends one blind bundle to an operator-controlled LAMU
HTTP server. It requires Python 3.11 or later, an explicit model, and a log
directory. It does not start a server, change routing, unload another model,
retry, or fall back to another provider.

Keep the log directory outside the repository. The client refuses a directory
inside its repository working directory before writing or contacting the
provider: creating review logs among checked inputs would stale its own review.

After checking service and GPU ownership, start the local service with
`lamu serve --port 8020`. Configure the verifier, using your actual local model:

```toml
[verify]
verifier_argv = ["python3", "tools/verifier/local-verifier.py", "--model", "YOUR-LOCAL-MODEL", "--log-root", "/absolute/path/to/private-review-runs"]
verifier_timeout_secs = 900
```

Then run `war verify <alias> --performer <actual-performer> --run --json`.
The command receives the exact packet references through
`OPENWARRANT_REVIEWED_PACKETS`, supplied by OpenWarrant. Do not invent them
for a production review.

To select the local adapter for one preparation run, without editing repository
configuration, pass one command argument per `--verifier` flag:

```bash
war prepare OW-WAR-0003 --compact \
  --verifier python3 --verifier tools/verifier/local-verifier.py \
  --verifier=--model --verifier YOUR-LOCAL-MODEL \
  --verifier=--log-root --verifier /absolute/path/to/private-review-runs
```

The same `--verifier` arguments work with `war verify <alias> --run`.
`--compact` selects the experimental v3 transport. Arguments are passed directly
to the process, without a shell. The repository's configured timeout still
applies. Without these options the configured command and v2 transport remain
the defaults.

Add `--dry-run` to `prepare` to display the chosen transport and explicit
command selection without starting the adapter. A forced review that returns
`UNKNOWN` stops preparation as `UNKNOWN`, even when older verdicts are still
current. Existing records remain intact; they cannot stand in for the requested
new observation. Command selection grants no authority and leaves the source,
packet, view, independence and human-acceptance checks in place. Establish the
local runtime and its resource capacity before executing this command.

The client requires valid compiler budget metadata and refuses packets marked
over budget, or whose estimate exceeds the budget, before any backend call or
review-log write. Exact required context stays intact. This checks the supplied
compiler metadata; it does not substitute a new tokenizer.

Pass `--thinking on` or `--thinking off` to request LAMU's supported
`enable_thinking` setting. The default, `--thinking auto`, omits that field and
keeps the provider default. The run log records `thinking_requested`; that is
the requested mode, not proof that a model honored it. This option does not
change the exact review context, system instructions, output limit or verdict
checks. An incomplete answer remains unusable in every mode. No retry or mode
switch happens automatically.

### Inspect an oversized packet offline

The experimental preview command separates reading context from the lossless
machine bindings and raw evidence archive:

```bash
war verify OW-WAR-0003 --performer agent://codex --bundle --portable-preview /absolute/path/outside/the/repository --json
```

Copy the emitted `preview-*.json` and its sibling `bundle-*.json` together to
read them offline. The preview keeps exact governing sources, fixtures and
contract atoms. It lists archived raw evidence and retains the existing bounded
gate-output excerpts. A reference does not mean that an agent saw the archived
bytes. Claims needing missing observations remain unestablished.

The preview has its own token account. Required rules are kept even if they
exceed the budget. Publication refuses output links and different bytes at a
retained name. Outputs stay outside the repository so they do not change the
reviewed source inputs.

**Preview only:** `oh.war/verification-view-preview/v1` is not accepted by the
local verifier or response ingestion. It cannot be used with `--run`, cannot
award assurance and does not unblock a production review yet. The accompanying
v2 archive remains subject to its original budget refusal. Executable compact
review transport and its ingestion checks use a separate experimental protocol
under proposed ADR0030; preview files remain ineligible.

Before introducing that transport, inspect a retained preview against its
original repository and actual performer:

```bash
war verify OW-WAR-0003 --performer agent://codex --inspect-preview /absolute/path/to/preview-HASH.json --json
```

This read-only check recomputes the view from the exact sibling archive, checks
its raw and canonical hashes, and applies the existing packet validation to the
current task, source snapshot, governing context and displayed evidence. It
refuses altered displays even after someone recomputes the view filename hash.
Changed source inputs make the preview stale. Neither file is imported, no
verifier runs and no verdict or assurance is created. Inspection needs the
original source repository; the package itself remains readable offline.
Inputs are limited to 64 MiB each and are read without following path links.

### Emit a compact model input

```bash
war verify OW-WAR-0003 --performer agent://codex --bundle --compact --json
```

This emits `oh.war/review-context/v3`: small packet/view references and a pointer
to the retained subject. The model input is `oh.war/verification-view/v1`,
distinct from a preview. Its required rules, fixtures and contracts stay exact;
raw omitted evidence remains archived and does not count as evidence shown.
Budget pressure retains an explicitly ineligible input instead of removing rules.

New executable views withhold prior verifier conclusions from the blind input.
They retain an empty `prior_verifications` list and an explicit archive pointer
with the historical count. The lossless archive retains those judgments;
emission does not rewrite them. Required atoms, rules and contracts remain exact.
The legacy v2 transport and non-executable preview retain their existing form.

The local adapter accepts this input only with exact compiler-produced
`OPENWARRANT_REVIEWED_PACKETS` and `OPENWARRANT_REVIEWED_VIEWS` metadata. The
harness supplies these from the index. It selects the actual pair, checks raw
view and archive identities and task agreement before any provider call or log
write, and sends only the view to the model. The Rust-computed archive digest
is carried unchanged. Full byte/source/actor checks remain the ingester's job.

The returned response uses v3 and retains exact view/archive references. It
does not mean the model saw all archived bytes or that its claims establish
assurance. The old v2 path and refusal of previews remain in place. No backend
is started or configured by input emission. To dispatch each compact input through
the configured v3-aware verifier and ingest its answer, use:

```bash
war verify OW-WAR-0003 --performer agent://codex --run --compact --json
```

The runner supplies only the actual packet/view pair to each call. Oversized
views stay inspectable but are never dispatched. Protocol downgrade or substituted
response references are refused before ingestion; the normal ingest still checks
current source, bytes, coverage and independence. Adapter failure or timeout
records no verdict for that call. The default `--run` continues to use v2.

Ingest the actual independent response with the existing `--response` command.
The v3 path preflights the whole envelope and all referenced views before any
obligation record or journal write. It checks current source, canonical
archives, exact canonical views and sibling archive bytes, request/performer
and obligation coverage, reading budget, assignment and independence policy.
It does not accept previews or infer v3 from optional fields in a v2 response.
New records use a distinct v3 envelope and journal binding; prior raw records
remain in history, and exact replay preserves their bytes. Current-work and
immutable-candidate checks use the same binding predicate. Human acceptance,
authorization and resolution remain separate acts.

### Candidate compact storage codec

The SDK exposes an opt-in `compact_verification_record` module for the proposed
v3 stored envelope. `decode` parses that envelope; `StoredCompactVerification::binds`
compares the subject, archive references and exact view references, requiring
nonempty distinct files and valid relative paths and hashes. A match checks
metadata only. It does not read the files, prove their contents, authenticate
the reviewer, or establish independent assurance.

`decode_response` separately selects the exact compact response protocol and
refuses legacy or future versions and unknown fields. Its `prepare_records`
method rejects empty observations, repeated obligation ids and invalid reference
sets before returning a lazy record iterator. It preserves the supplied verdicts
and independence claims; it does not improve or authenticate them. A caller
must validate current source, declared obligations, actual view/archive bytes
and verifier admissibility before writing or using any result for assurance.

The frozen v1/v2 SDK decoder continues to refuse this envelope. The CLI uses
the explicit v3 decoder and validates the additional bindings. The candidate
JSON schema is published beside v1/v2, without changing the active schema pack.
Regenerate that candidate with:

```bash
cargo run --locked -p openwarrant-core --features schema --example compact_verification_schema
```

The client permits HTTP to a literal loopback IP and port only. It ignores
proxy settings, refuses redirects, requests no tools, and rejects a substituted
model, tool call or incomplete answer. It retains the exact input bytes, raw
provider response, returned TOML, timing and reported token usage in a unique
run directory. A malformed or incomplete verdict set yields `not_established`
for every requested obligation.

**The operator must establish that the server and model are local.** A
loopback address or a `service: lamu` response does not authenticate a server
or prove that it cannot proxy to a paid service. Inspect the actual executable,
version and local backend before use. The client reports monetary cost as
unknown; token usage is not a dollar meter. It cannot enforce a mandatory hard
spend cap. No cost is silently reported as zero.

The model receives only the generated input (a v2 bundle or v1 executable view)
in a new two-message context.
The client runs in a fresh temporary directory and provides no filesystem
tools. This isolates the review conversation; it is not an operating-system
sandbox against a hostile provider process or another process under the same
account. Do not infer stronger custody or independence from the transport.
Do not award assurance merely because the client returned a response. Inspect
the actual evidence and preserve independent and human acceptance gates.

Run the synthetic transport controls without a model:

```bash
python3 -B tools/verifier/test_local_verifier.py -v
```

These controls prove client behavior with a test server. They do not qualify
a model, provider integration, or any production Warrant.

After independently establishing the local runtime and available resources,
run the explicit live control with a fresh output directory:

```bash
python3 -B conformance/fixtures/verifier/local-refresh.py --war /absolute/path/to/war --output /absolute/path/to/new-control-run --model YOUR-LOCAL-MODEL
```

It uses a scratch corpus and two real model calls. It observes a visible text
claim, a missing execution claim, rejection of an old response after an artifact
change, and a fresh review with exact prior record bytes retained. The output
directory holds both the corpus and raw run history. It does not authorize,
resolve, or accept any production Warrant. It never runs in the default gate.
