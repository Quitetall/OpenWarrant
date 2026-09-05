# Integration packets — what OpenWarrant needs from its peers, and what it will refuse

Three Warrants' worth of work in this repository cannot be finished by this
repository. Each is blocked on an artifact only another system can produce: a
runtime receipt Katana mints, an IR a Liminal profile emits, an enterprise
identifier Knowledge Fabric allocates.

Each packet here is one outbound request. Every packet has the same five parts:

1. **What we are asking for**, in one paragraph.
2. **The governing text**, quoted from the SAS rather than paraphrased, with
   section numbers so the recipient can check us.
3. **What we have already built** on our side of the seam.
4. **What we need returned — as artifacts, not assertions.** This is the part
   that matters. An email saying "yes, that works" closes nothing here.
5. **What we will refuse**, with the control that does the refusing. These are
   not threats; they are already-passing plants in `conformance/plant.sh`. We
   list them so nobody spends effort on a return we are structurally unable to
   accept.

| Packet | Recipient | Unblocks |
|---|---|---|
| [katana.md](katana.md) | Katana | OW-WAR-0026, OW-WAR-0045 (Phase 5 exit) |
| [liminal.md](liminal.md) | Liminal | OW-WAR-0040, OW-WAR-0048 (Phase 8 exit) |
| [knowledge-fabric.md](knowledge-fabric.md) | Knowledge Fabric | OW-WAR-0028, OW-WAR-0029, OW-WAR-0044 (Phase 4 exit) |

## One thing to read first, whichever packet you got

A **SAS** and a **Warrant** are the same class of artifact at two levels of
importance. Each is a controlled contract with intent, basis, deliverables,
acceptance obligations, gates, and immutable revisions. They differ in scope and
in what traces to them, not in kind. A program has exactly one SAS; every
Warrant in that program traces to it.

Starting a program? Write its SAS. Doing work inside a program? Write a Warrant
against that program's SAS. See `docs/DEFINITIONS.md` and SAS §6.10.

You do **not** need to adopt any of this to answer a packet. The packets ask for
artifacts your system already knows how to produce; the WAR framing is here so
you can see why the artifact and not a summary of it is what closes the item.
