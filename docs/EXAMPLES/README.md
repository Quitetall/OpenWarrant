# Examples

Each example is a Warrant of one work kind, walked from `war plan new` to a
resolution, with the records it left. Where an example points into this
repository's corpus, the records are real: nothing here is a mock-up.

| kind | example | status |
|---|---|---|
| code | [01-code.md](01-code.md) — OW-WAR-0046: a receipt, five verifications, a resolution in one directory | walked from the committed records |
| documents / research / decisions | [02-document.md](02-document.md) — OW-WAR-0065: a research memo under the `document.review` gate | walked from the committed records; awaits its independent review |
| ops / runs / experiments | [03-run.md](03-run.md) — OW-WAR-0066: the conformance battery as a service stage under `war run` | walked from the committed records |

The QUICKSTART at the repository root is the same loop on an empty directory.

These walk the governed path. Most work is lighter: a ticket is made with
`war create "..."`, closed with `war start` and `war done`, and becomes a Warrant
(`--test "<command>"`) or an Order (`--sign`) only when someone asks, in place and
with the same id (`war plan promote <id>`). [docs/TICKETS.md](../TICKETS.md) walks it.
