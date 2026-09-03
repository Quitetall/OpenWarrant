# Packet 1 — Katana

**To:** the Katana team
**From:** OpenWarrant (`Quitetall/OpenWarrant`), 2026-09-03
**Blocks:** OW-WAR-0026 (the runtime seam) and OW-WAR-0045 (the Phase 5 exit)
**Governing text:** SAS §47 (Dispatch), §48 (Katana integration), §51 (Stage
Submission), §52 (attempts), §53 (remedies)

## 1. What we are asking for

We need one stage of one Warrant to be **executed by a stateless Katana agent
that receives nothing but a Dispatch**, and we need Katana to **return a runtime
receipt**. We cannot mint that receipt ourselves: our receipt type has no
constructor that invents the fields §48.4 requires, and a receipt recorded
against a dispatch it does not answer is rejected outright. Until a receipt
arrives from your side, the seam stays "built but never exercised", which is
what our own Warrant currently says about itself.

There is a smaller ask that has to come first: **tell us the executor name.**
See §4, item 0.

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
  field set** you will receive, not the job we want run.
- Stage Submission, the four attempt kinds (§52), and the four remedies (§53)
  are implemented and refuse a submission that tries to resolve anything.
- Receipt *consumption* is implemented: `KatanaReceipt::validate` refuses a
  receipt missing any of §48.4's minimum fields, and refuses one whose Dispatch
  digest does not match the Dispatch it was recorded against — "a receipt for a
  different dispatch is evidence about a different run."
- Receipt *minting* is not implemented, deliberately. There is no code path that
  produces one, which is the honest form of OW-WAR-0026's OBL-002. Note that
  OBL-002 asks for a **plant** proving fabrication is refused, and that plant is
  not written yet — the obligation is stated, not established. We are not
  claiming otherwise.

## 4. What we need returned — artifacts, not assertions

**0. An executor name and its argument schema.** Today
`war dispatch OW-WAR-0045 STAGE-002` refuses with:

> stage "STAGE-002" declares no executor_ref, so nothing says what runs it.
> Refused rather than dispatched under the WAR id

We deliberately will not invent a name for something that runs in your system.
Send us the executor identifier Katana resolves (and the shape of its args) and
we will pin it in the stage, recompile, and send you the real Dispatch.

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
