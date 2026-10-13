# Portable verifier package — proposal

Status: proposed direction, not an accepted standard or implemented verifier
protocol. The owner is choosing the direction. Current v2 packet and admission
rules remain in force. This proposal does not accept Format draft 3, change the
SAS, award assurance, or authorize a paid call.

## The observed problem

At source `38c7dbed`, the largest OW-WAR-0063 review packet estimates 407450
tokens against the unchanged default budget of 48000. Its request is 1072971
canonical JSON bytes; required sources are 540814 bytes. The request carries
7485 gate input bindings. Exact gate stdout contributes 382075 bytes and the
pinned SAS 149780 bytes. These are historical observations, not a measurement
of every current Warrant. See [retained profile](../warrants/OW-WAR-0117/implementation/budget-profile-20261012/packet-profile.json).

Splitting by obligation does not remove this fixed reading floor. Raising a
synthetic fixture limit proves ingestion, not that a real backend can read the
packet. No measured savings or full review is claimed here.

## Three routes

| Route | What it requires | Remaining limit |
|---|---|---|
| Portable package and controlled review reads (recommended) | Exact offline sources, a bounded entry view, and an independently run read-only verifier adapter | New protocol, adapter isolation, coverage controls and qualification |
| Entire packet in the model request | A backend with sufficient actual capacity and reliable cost accounting under the configured cap | Cannot assume availability, cheapness or comprehension |
| Manual oversized review | An independent authorized reviewer inspects the exact retained material | Human availability; no unattended full-wave review |

## Separate package integrity from review coverage

The package retains the full reviewed subject, contract, exact applicable rules,
required dependencies, fixture/evidence bytes and source inventory. It can be
used offline. A manifest's digest binds those bytes; it does not say anybody
read them. The package must never acquire authority merely by containing a
signed-looking field or an agent's summary.

The entry view gives the task, obligations, source identities and exact required
rules. The view must name what remains to be read. A required rule must not be
silently reduced to a pointer or paraphrase. If the rules alone exceed the
reading budget, this route still refuses readiness unless the protocol provides
and proves complete required-rule delivery. A smaller entry is not a complete
review merely because the larger package exists.

Machine checks can compare inventory digests without putting every checksum in
model prose. That separation must be explicit in the new protocol. It cannot
reinterpret an existing v2 response as though it reviewed a smaller view.

## The module interfaces

The OpenWarrant SDK owns a small pure interface: validate package integrity and
bindings; describe missing or altered material; check a proposed result against
the exact package and the requested obligations. It takes bytes and explicit
inputs. It does not retrieve context, run a model, require a database or grant
execution rights. Reuse the existing packet/source primitives where their
semantics fit; do not treat their F7 schema as interchangeable with the legacy
verification-bundle schema.

LAMU or another context-provider adapter selects and builds views. It retains
exact source revisions and required dependencies. Background can be summarized;
required rules cannot be weakened. Its output remains checked data.

The verifier workflow owns the read interface, isolation, actual backend limits,
cost policy and truthful coverage observations. A first concrete adapter must
exercise that interface; do not introduce speculative adapters for every app.
The existing no-tools Claude wrapper is not silently converted to this route.
Its approved isolation constraints need explicit reconciliation.

## Controlled reads

Reads are confined to one immutable, validated package. No repository writes,
network fetches, arbitrary process execution, performer conversation, settings
or host instructions enter through the read interface. Path traversal, links,
undeclared sources and changed bytes are refused. The adapter uses an isolated
workspace and never the performer's writable workspace.

Any read observation names the package digest, view, obligation, source/range,
actual bytes and adapter identity. An arbitrary agent-supplied receipt is not
trusted evidence. Reading is not comprehension: only the independent verifier
supplies the verdict, and human acceptance remains its own act. Missing required
reads leave affected obligations unestablished. Native caller authentication and
custody limitations in OW149 remain open, not fixed by this proposal.

## Compatibility and tests before use

Introduce an explicit new protocol identifier if this direction is selected.
Do not overwrite v1/v2 history, their digest domains, retained packets or current
admission behavior. An old adapter must receive an old packet or a named
unsupported result, never a disguised pointer-only packet.

Prove through the public interface:

1. A complete immutable offline package works with the original repository
   unavailable; deleting a required source refuses affected readiness.
2. Altered content, a stale source revision, forged/omitted coverage, a source
   outside the package, traversal and links cannot clear an obligation.
3. Oversized exact required rules still refuse; omitted optional background is
   disclosed. A checksum-only inventory is not mistaken for rule delivery.
4. Package/view replacement during a run and stale returned results refuse;
   earlier exact history remains available.
5. The declared no-write, no-network and independent-workspace controls are
   actually observed rejecting attempts, not asserted from adapter names.
6. Unknown spend refuses when a hard cap is mandatory. Lower entry bytes do not
   become a dollar-saving claim without measured complete-task receipts.
7. Compare a real whole-packet and package-read review on the same bounded task,
   including a planted defect. Both must assess the approved requirements;
   report missing observations as UNKNOWN, not qualification.

Selecting a direction starts protocol drafting. Formal amendments, native
adapter qualification, independent verdicts and secure human acts remain required
where the governing work specifies them. It does not complete t-b9610 or OW117.
