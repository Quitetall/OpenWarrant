# Packet 1 — Katana

**To:** the Katana team
**From:** OpenWarrant (`Quitetall/OpenWarrant`), 2026-09-03
**Blocks:** OW-WAR-0026 (the runtime seam) and OW-WAR-0045 (the Phase 5 exit)
**Governing text:** SAS §47 (Dispatch), §48 (Katana integration), §51 (Stage
Submission), §52 (attempts), §53 (remedies)

> **Answered 2026-09-03.** Katana replied to this packet. Item 0's premise was
> wrong and is corrected below; items 1–4 stand with real constraints now
> recorded. Nobody at Katana has committed to anything, and nothing here should
> be read as a commitment.

## 1. What we are asking for

We need one stage of one Warrant to be **executed by a stateless Katana agent
that receives nothing but a Dispatch**, and we need Katana to **return a runtime
receipt**. We cannot mint that receipt ourselves: our receipt type has no
constructor that invents the fields §48.4 requires, and a receipt recorded
against a dispatch it does not answer is rejected outright. Until a receipt
arrives from your side, the seam stays "built but never exercised", which is
what our own Warrant currently says about itself.

There is a smaller ask that had to come first — **tell us the executor name** —
and the answer is that there is no name to tell. See §4 item 0, rewritten.

## 2. The governing text

**§48.1 Runtime seam** — "OpenWarrant SHALL invoke Katana through a versioned
Dispatch protocol or subprocess/API adapter."

**§48.2 PromptIR ownership** — "Katana compiles the Dispatch and its runtime
event history into Katana-owned PromptIR. OpenWarrant records the PromptIR
digest from the Katana receipt. It does not compile or reinterpret Katana's
runtime conversation."

This is a boundary we are asking you to *keep*, not one we are asking you to
cross. Requirement RQ-064 forbids us duplicating what Katana owns, and our
Warrant carries a control proving we construct no PromptIR anywhere.

**§48.3 Capabilities** — "Knowledge Fabric and the WAR contract authorize what
may be done. Katana realizes and enforces the low-level capability set."

**§48.4 Runtime receipt** — Katana SHALL return, at minimum: Katana session/run
identity; Dispatch digest; PromptIR digest; provider/model identity; runtime
event-log head; realized capabilities; confinement; usage; artifact refs;
terminal runtime status; receipt digest.

**§48.5 Taint** — taint and influence labels stay Katana-owned runtime facts;
we reference them, we do not compute them.

**§51.2 No self-completion** — the performer may request `continue`, `verify`,
`block`, `amend`, or `cancel`. "It SHALL NOT set or request authoritative
resolution."

## 3. What we have already built

- `war dispatch <alias> <stage>` compiles a §47.1 Dispatch from a Warrant's own
  atoms, selects stage-relevant context, records omitted subgraphs, and digests
  the result deterministically under our §65 Dispatch domain.
- A worked example is attached:
  `attachments/example-dispatch-OW-WAR-0047-STAGE-002.json`. That one targets
  BLUT rather than Katana — it is included to show you the **shape and the
  field set** you will receive, not the job we want run. It is unedited output.

  **Read the `instructions` array as a known defect, not as a sample of good
  output.** Writing this packet found a bug in our Dispatch compiler: the reader
  that lifts a work-order section keeps only lines that begin a bullet, so every
  continuation line is dropped — a wrapped instruction arrives as its first line,
  cut mid-sentence, and an indented command block under a bullet disappears
  entirely. (Once the fix below lands and the example is regenerated, the
  attachment will no longer show this.) A Dispatch is the only packet a stateless actor receives,
  so that is the actor being told less than the contract says, with nothing in
  the output to show that anything was dropped.

  The fix is written and tested but not landed. `crates/openwarrant-compiler`
  is pinned deliverable D-002 of OW-WAR-0056, which is resolved, so changing
  those bytes trips `deliverable.digest-drift`. That control detects the drift
  and makes it answerable; it does not forbid the change. A delivered artifact
  moves through authorization rather than because the performer noticed something
  afterwards, and that authorization is somebody's decision, not the compiler's. The attachment above is
  unedited current output, defect included, because showing you a doctored
  example would be worse than showing you a real one.
- Stage Submission, the four attempt kinds (§52), and the four remedies (§53)
  are implemented and refuse a submission that tries to resolve anything.
- Receipt *consumption* is implemented: `KatanaReceipt::validate` refuses a
  receipt missing any of §48.4's minimum fields, and refuses one whose Dispatch
  digest does not match the Dispatch it was recorded against — "a receipt for a
  different dispatch is evidence about a different run."
- Receipt *minting* is not implemented, deliberately: no non-test code path
  constructs a `KatanaReceipt`. Being exact, because it changes what you can
  rely on — the struct's fields are public, so this is a discipline held by
  review, not a barrier held by the type system. That is precisely why
  OW-WAR-0026's OBL-002 asks for a **plant** proving fabrication is refused, and
  that plant is not written yet. The obligation is stated, not established. We
  are not claiming otherwise.

## 4. What we need returned — artifacts, not assertions

**0. ~~An executor name and its argument schema.~~ ANSWERED — the premise was
wrong, and the name is ours to assign.** Katana has no executor registry, no
`executor_ref` concept, and no lookup that would resolve one. There is nothing on
that side to name, so declining to invent a name would leave the field empty
forever. What goes there will be a name **OpenWarrant** assigns to one of
Katana's invocation surfaces, pinned to a Katana version.

Today `war dispatch OW-WAR-0045 STAGE-002` still refuses:

> stage "STAGE-002" declares no executor_ref, so nothing says what runs it.
> Refused rather than dispatched under the WAR id

The three real surfaces, of which §48.1 already anticipates two ("Dispatch
protocol or subprocess/API adapter"):

1. **CLI subprocess** — `katana [--result|--json] --policy <reader|builder|trusted|muramasa>
   --sandbox <auto|bubblewrap|podman|podman-session|landlock|bubblewrap-egress|none>
   --cwd <dir> --runtime-profile <profile> --wall-timeout-secs <n>
   --new-session-path <path> --max-turns <n> --max-tool-calls <n> "<prompt>"`.
   Stateless by construction against a fresh cwd and session path.
2. **ACP over stdio** (`katana acp`) — a genuinely versioned protocol, and the
   closest thing to §48.1's "versioned Dispatch protocol".
3. **MCP server** (`katana mcp`), exposing `katana.run`.

Version pinning for item 4 is already solved on their side: `katana --version`
plus source commit plus binary sha256, the same triple their SWE-bench harness
emits as build provenance.

**1. One execution.** The agent receives the Dispatch JSON and nothing else — no
repository checkout handed over the side, no prior conversation, no operator
context. That is the word "stateless" in the Phase 5 exit criterion, and it is
the part we cannot verify from the outside if the run is set up loosely.

**2. A Stage Submission** back, requesting one of the five §51.2 actions. A
submission that requests resolution is a finding for us, not a failure for you —
but we would rather it not happen by accident, so: the agent should be able to
say "verify" and stop.

**3. A runtime receipt** carrying the eleven §48.4 fields. The two that do the
most work for us are the **Dispatch digest** (proving the agent ran the packet we
sent, unmodified) and the **realized capabilities** (so we can check realized
never exceeded authorized).

**4. A version pin.** The Katana adapter identity and exact version, so the run
is reproducible and the receipt is attributable.

## 4b. What exists today, what does not, and the part that is larger than it looks

Reported by Katana 2026-09-03, recorded here because it changes what
OW-WAR-0045 can honestly claim.

**Exists.** Stateless execution is not theoretical — a sibling project is driving
Katana in exactly the shape item 1 describes (cleared environment, per-run HOME,
fresh session log per item, no repository handed over the side), 150 runs in
flight. Most of §48.4's eleven fields already exist as recorded runtime facts:
session identity and usage and terminal status in the `--result` digest;
`katana::compiler::PromptIR` with `canonical_bytes()` and a `prompt_hash` on
every `model.request` event; a hash-chained event log (`prev_hash` per event);
provider/model identity on session creation and per request; realized
capabilities as `capability.granted` events; confinement on `session.created`.
Our §48.2 boundary is not aspirational on their side — it is what their replay
gate already asserts.

**Does not exist.** No receipt emitter and no receipt digest: assembling the
eleven fields is real work there, not a flag. No Dispatch parser and no Dispatch
digest — Katana takes a prompt string. The Dispatch JSON *can* be that string,
which satisfies the letter of item 1, but Katana would not validate it, would not
digest it, and **would not enforce our `resource_envelope` or
`capability_authorization` from it**; its confinement and capability limits come
from its own flags. "Realized never exceeded authorized" is therefore checkable
after the fact from the event log, not enforced from the Dispatch. And no Stage
Submission: `--result` emits `{status, session, answer, tool_calls, usage}`; the
five §51.2 actions are not modelled.

**The part that is larger than it looks.** Our §51.2 worry was that an agent
might request resolution *by accident*. Katana's correction is sharper: in a
prompt-only integration, accident is not the risk. If the Submission exists
because the prompt asked for one, then conformance to §51.2 — including the
refusal to resolve — is **model behaviour, not a Katana guarantee**. Nothing
structural prevents a model from emitting a submission that resolves, and a
well-formed one would look exactly like a compliant one.

OW-WAR-0045's exit says "without authority confusion". If that is meant to rest
on a structural property, a prompt-shaped integration does not carry it. Better
to have this before a passing run is recorded than after. It is unscoped, and it
may be the largest of the three pieces.

## 4c. Digest algorithms — checked, and no collision

Katana flagged that its digests are BLAKE3 with a `b3:` prefix (both
`prompt_hash` and the event-log chain), and asked whether `KatanaReceipt::validate`
assumes SHA-256 hex. It does not: every §48.4 field is checked only for being
non-empty, so a `b3:`-prefixed `prompt_ir_digest` or `runtime_event_log_head`
passes unchanged. §48.5 taint labels are likewise referenced, never recomputed.

The one field that is not opaque is `dispatch_digest`. It must be **our** digest
of the Dispatch, echoed back verbatim, not Katana's hash of the packet — that
equality is what proves the agent ran the bytes we sent. Everything else can be
in whatever algorithm Katana natively emits.

## 5. What we will refuse

Each of these is an existing control, not a hypothetical:

- **A receipt we generated.** If you send us field values in prose and ask us to
  assemble the receipt, we have to decline — the receipt has to be minted on your
  side to be worth anything. (OW-WAR-0026 OBL-002 wants this backed by a plant;
  writing that plant is on us, and is tracked.)
- **A receipt recorded against the wrong Dispatch.** Enforced today by
  `KatanaReceipt::validate`.
- **A PromptIR we constructed.** OBL-001 is proved by absence: no PromptIR
  construction anywhere in the crate. Our receipt type carries a
  `prompt_ir_digest` and no PromptIR, which is the shape of the boundary.
- **A submission that resolves.** §51.2. Our resolution seam refuses any
  non-human resolver outright (§27.2).
- **"It ran and it worked."** OW-WAR-0045's exit names three separate claims —
  *stateless*, *executed*, *without authority confusion* — and each needs its own
  evidence. A single summary sentence establishes none of them.

## 6. How to send it back

Any of: a PR to this repository adding the receipt and submission under
`docs/warrants/OW-WAR-0045/evidence/`; a tarball; or plain files. Canonical JSON
preferred, but we will take whatever Katana natively emits and record the
conversion. Tell us the exact bytes' digest either way.

## 7. If the answer is "not yet"

That is a usable answer. Say which of §4's items are far off and we will record
OW-WAR-0026 as delivered-but-unexercised (which is what its OBL-003 already
says) and stop counting OW-WAR-0045 as near-term. What we cannot do is leave it
ambiguous — an unexercised seam that *looks* finished is the failure mode this
whole repository exists to prevent.
