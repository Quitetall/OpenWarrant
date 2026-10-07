# Profiles

A profile says which atoms a Warrant must have (SAS §16.3). OpenWarrant knows
two, `delivery` and `decision`. A program can add more as data, without
touching the technical core (§2.2: adding a profile "SHALL NOT require
redesign"). OW-WAR-0140 built this.

## What a profile is

A profile is a name, the core profile it extends, and the namespaced roles it
requires. Profiles live in `profiles/<name>.toml`, schema `oh.war/profile/v1`.
The repository reads every file in that directory when it opens. The core
crate reads no files (§79.1): the CLI hands it each definition's bytes, and
`ProfileRegistry` in `openwarrant-core/src/role.rs` validates them.

`Profile` is a name some registry has resolved, not an enum. `"delivery"` and
`"decision"` resolve everywhere. Any other name resolves only through the
registry of a program that defines it. Everywhere else it is refused as
`unknown profile`, as before.

## The two core profiles

`profiles/delivery.toml` and `profiles/decision.toml` restate §16.3:

```toml
schema = "oh.war/profile/v1"
name = "delivery"
core = true
required_roles = ["control", "intent", "basis", "work_order", "milestones",
                  "assurance", "relations_and_integrity"]
```

A program cannot change them. A copy whose `required_roles` differ from
§16.3's, or that extends something, is refused. So is one that is unapproved
or requires a namespaced role. The registry refuses the repository and names
the file. The files add nothing a Warrant composes against: every committed IR
compiled to the same bytes before and after they existed.

## Adding a profile

A new profile extends a core profile:

```toml
schema = "oh.war/profile/v1"
name = "lab"
extends = "delivery"
approved = false            # required: §22.3's "until that profile is approved"
note = "prose for a reader"
acceptance_role = "lab.acceptance"          # optional: whose header names the acceptance authority
reference_roles = ["lab.protocol"]          # optional: atoms that link, never copy

[[requires]]
role = "lab.protocol"       # must be in the profile's own namespace, `lab.*`
ordinal = 71                # must not be a core role's ordinal (§16.1)
file = "71-lab-protocol.md" # what `war new --profile lab` names the atom
stub = """# Protocol
<!-- required -->
<!-- The question the author answers. -->
"""
```

The registry refuses a definition in these cases, and names the file each
time:

- its `name` is not its file stem, or not a lowercase word;
- it extends anything but `delivery` or `decision`;
- it does not state `approved`;
- it requires a role outside its own namespace, or a role twice, or at a core
  role's ordinal;
- its acceptance or reference roles are not among the roles it requires;
- it has a field this schema does not define.

Then `war new --profile lab "..."` writes the extended profile's preset atoms
(`feature` for `delivery`, `decision` for `decision`), plus one stub per
required role from the definition's `stub` text. No code names the profile.

## What a profile may require, and what it may not change

The manifest validation rule, in `manifest.rs` `validate_in`:

- A namespaced role the profile requires is admitted when it is declared, and
  its absence is refused: `profile lab requires a lab.protocol atom`.
- A required namespaced role that no profile names is still
  `UnknownRequiredRole` (§16.4). That includes a role another profile names:
  `contractor.terms` required on a `delivery` Warrant is refused in a program
  that defines `contractor`. An optional namespaced role is preserved, as it
  always was.

A profile may NOT change:

- **The core types.** The roles a profile adds are composition entries,
  carried with their digests like any other atom. No core struct gains a
  field. The work order of OW-WAR-0140 freezes these modules:
  - in `openwarrant-core`: `lifecycle`, `state`, `contract`, `obligation`,
    `verification`, `independence`, `resolution`, `authority`,
    `deliverable`, `gate` and `gate_run`;
  - in `openwarrant-compiler`: `ir`, `canonical` and `digest`.

  `56-contractor.sh` diffs them from the seam commit to HEAD.
- **Who may act.** No profile adds an actor role. Acceptance is the existing
  resolution act. An `acceptance_role` atom names the acceptance authority in
  its header (`acceptance_authority: <actor>`). `war check` requires that
  actor to be a human in `docs/authority/roles.toml`, holding `resolver`, who
  is not the performer. `war resolve` refuses an agent whatever the atom says.
- **What counts as resolved.** Only the core decides that, and the profile
  has no way into it.
- **What it can prove about its references.** A reference role must cite at
  least one `scheme://` record. A `war://` Warrant in this repository
  resolves. A `kf://` record reads UNKNOWN: no Knowledge Fabric is reachable
  here, and Law 15 forbids reading an unasked question as answered. The
  record proves the reference was made. It cannot prove what the referenced
  contract says (R-001).

## A working form: tickets (OW-WAR-0147)

A profile may declare `form = "working"`. A record of a working-form profile is
the working state of a Warrant before anyone asked for a contract: it carries
only the core roles the definition names in `core_roles`, plus its own
namespaced roles, and it is not a Warrant of the contract corpus.

```toml
schema = "oh.war/profile/v1"
name = "ticket"
extends = "delivery"
approved = false
form = "working"
core_roles = ["intent"]

[[requires]]
role = "ticket.checklist"
ordinal = 15
file = "15-checklist.md"
stub = "# Checklist\n\n"
```

`profiles/ticket.toml` is the one this repository ships; `war create` uses a
built-in copy of it in a repository that has none. What the form may and may
not do:

- A working record lives outside `docs/warrants/` (tickets live in
  `docs/tickets/`). It is never compiled, authorized, verified or resolved.
  `Manifest::validate_in` refuses a Warrant manifest naming a working-form
  profile (`WorkingFormProfile`), so nothing that reads the contract corpus
  ever reads a two-atom record.
- The way into the contract is promotion into the core profile it extends
  (`war promote`), which then requires every core role, exactly as `war new`.
  The working form loosens nothing a signature, verification or resolution
  reads.
- The registry refuses a working form that names no core role, a core role the
  extended profile does not author (a compiler-produced one, or `adr` on
  `delivery`), a role twice, an unknown `form`, `core_roles` without
  `form = "working"`, or an acceptance or reference role (nothing accepts a
  working record). A core profile has no working form.

## Capabilities: what applies to a kind (OW-ADR-0031)

A profile selects `capabilities` from a closed set the kernel implements:
`structure, links, claims, acceptance, evidence, verification,
authorization, resolution, stages`. It cannot define one, and a capability
never supplies an act: `verification` means "nothing of this reads verified
without an independent response".

| capability | needs | what it makes apply |
|---|---|---|
| `structure` | — | parse, roles, every structural rule |
| `links` | `structure` | traceability, roadmap placement (`roadmap.unassigned`), SAS sections, parents |
| `claims` | `structure` | claim and done on work items |
| `acceptance` | `structure` | a revision in force once a human accepts it |
| `evidence` | `structure` | gate citations, `war evidence record`, §40 records |
| `verification` | `evidence` | obligations, the adequacy review, `war verify` |
| `authorization` | `structure` | `war authorize`, `war sign`'s authorize act, the authorization's checks |
| `resolution` | `verification`, `authorization` | §56.1, `war resolve`, deliverable pins, corrections |
| `stages` | `structure` | the milestone graph, `war next`'s execute acts, §56.1 requirement 12 |

- `delivery` selects all nine; `decision` all but `stages`, so requirement 12
  reads "not applicable: no `stages` capability" for a decision; a working
  form (`form = "working"`) selects `structure, links, claims`. A core file
  restates its kind data and may not change it.
- An extension defaults to its core's set (the working form's set for a
  working form) and may narrow it, never widen it.
- A kind lacking a capability is refused the act by name
  (`capability.absent`), `war next` offers it nothing for it, and `war check`
  names the rule families that do not apply (`capability.not-applicable`).
  A §56.1 requirement whose capability is absent reads "not applicable",
  never met, and a kind without `resolution` stays `draft`.
- Refused: a name outside the set (`profile.capability-unknown`), a
  capability without its prerequisite (`profile.capability-prerequisite`),
  none, one twice, or a set wider than the core's
  (`profile.capabilities`).

What was once chosen by the profile's name is data too:

| field | default | read by |
|---|---|---|
| `satisfied_outcome` | `delivered` for `delivery`, none otherwise | the `profile_outcome` `war sign` drafts for `satisfied` |
| `falsifiable_claims` | `false` | §56.3: whether `falsified` may be recorded |
| `standing_coverage` | `true` for `delivery`, `false` otherwise | whether a standing class may name the profile |
| `extends` | — | the profile `war promote` turns a working record into |

`satisfied_outcome` and `falsifiable_claims` need `resolution`;
`standing_coverage` needs `authorization`.

## The pin: a signature covers the type

`war new` writes `profile_digest = "sha256:…"`, the profile file's digest,
into a new manifest when the program has that file. The manifest's bytes are
inside the contract digest, so a signature covers the type. `war check`
reports `profile.pinned` while the file is unchanged, and `profile.pin-drift`,
naming the profile and both digests, when it moved: a warning on a draft, an
error once authorized. A manifest without the field keeps its bytes, its
digest and its checks.

## What `war check` reports for an extended profile

| rule | when |
|---|---|
| `profile.registered` | PASS, with the `sha256:` of the definition the Warrant composed against |
| `profile.unapproved` | WARN while `approved = false` |
| `profile.acceptance-authority` | PASS or ERROR, as above. Naming nobody is a WARN on a draft and an ERROR once authorized, like `atom.preset-unanswered` |
| `profile.reference` | PASS (resolved) or UNKNOWN (cannot be answered here), per reference |
| `profile.reference-missing` | a reference role cites nothing. WARN on a draft, ERROR once authorized |

## Open

- **U-003: pinning in the IR.** A new manifest pins its profile file
  (above); the digest is not in the IR, which would move every existing
  contract digest. Profile definitions are not in the schema pack. That is
  escalated, not decided.
- **The contractor profile** (`profiles/contractor.toml`) is a non-binding
  technical mechanism, `approved = false`. It is not a legal instrument, and
  it is not legal, financial or quality advice. SAS §98 requires separate
  legal, finance and QMS decisions before a contractor profile is delivered.
  None exists yet. See `docs/EXAMPLES/04-contractor.md`.
- **Callers that validate outside the repository**, such as the MCP
  `war_new`, `war plan --apply`, a preservation archive and a stage's
  runtime basis, still resolve profiles against the two core ones. A
  program profile is refused there as unknown. That fails closed, and none
  of those paths was in this Warrant's set.
