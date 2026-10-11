# Governance of the OpenWarrant Format

## The editor

The repository owner is the editor. The editor accepts, rejects or defers
every change to the standard, and alone makes a version normative. An
agent or contributor may draft and propose a change. A proposal is not
accepted until the editor accepts it.

## Proposing a change

1. **Issue.** Open an issue `format: <one line>`. Say which section it
   touches and why the current text fails a reader, writer, checker or
   context compiler.
2. **Proposal.** Open a pull request with:
   - the edit to `FORMAT-<version>.md`;
   - an ADR atom under `docs/adr/atoms/`, `status: proposed`, giving the
     decision, the options considered and the consequences;
   - at least one case under `conformance/format/` that the change makes
     pass, and one that it makes refuse.
3. **Review.** The proposal stays open at least 7 days for comment, unless
   it only fixes wording (a patch).
4. **Decision.** The editor accepts the ADR (its `status` becomes
   `accepted`, with a dated line under its Status heading) and merges, or
   records why not. A rejected ADR keeps its file with `status: rejected`.

## Versions

- **Patch** (`1.0.1`): wording, examples and typos. No rule changes. It
  needs no ADR.
- **Minor** (`1.1`): additions only, as FORMAT-1.0 §6 defines. A valid
  `1.0` document keeps its meaning, model and digests. It needs an ADR.
- **Major** (`2.0`): any other change, under a new `format` value. Every
  earlier major stays readable forever. It needs an ADR and a migration
  note that migrates nothing in place.

A version becomes normative when the editor accepts it. Its file is then
frozen: later corrections are patch releases in a new file.

## Extensions

Anyone may define a dotted extension (`com.example.*`, or `x.*` for
private use) without asking. To list a short prefix in the registry
(FORMAT-1.0 Appendix C), open a pull request that adds it with a link to its
definition. An extension enters the core only by a minor version.

## Licence

Specification text: CC-BY-4.0 (`LICENSE` in this directory). Grammar,
schemas, conformance cases and code: Apache-2.0. A contribution is made
under both, as the files it touches require.
