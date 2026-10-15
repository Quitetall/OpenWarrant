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

The client requires valid compiler budget metadata and refuses packets marked
over budget, or whose estimate exceeds the budget, before any backend call or
review-log write. Exact required context stays intact. This checks the supplied
compiler metadata; it does not substitute a new tokenizer.

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
review transport and its ingestion checks are separate unfinished work under
proposed ADR0030.

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

The model receives only the generated bundle in a new two-message context.
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
