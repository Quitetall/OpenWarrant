# W122 grammar implementation audit

The grammar document had source-line pointers that moved as the implementation
grew. Source links now name their symbols and pin exact paths and lines to
revision `23070e52da9d6153e136883ec1f1406da5151f39`. Each link was checked
against that Git object and the identical current file. The structured-atom
section also states the existing stages-capability condition. No parser,
accepted input, schema or normative SAS text was changed.

The public battery helpers ran the registered `57-grammar.sh`, not a copied
test. The initial run recorded 49 grammar passes and one initial corpus failure.
The raw corpus report names only four stale generated views after the document
edit. Public compilation refreshed those projections. The same registered run
then returned exit 0: 50 passes, zero failures, including the positive corpus.
Both logs and all four initial error findings remain here. The full initial
report remains in the durable artifact named and hashed by the observation.

## Bounded observations

- OBL-001: all four header-substitution controls refused with `atom.header`,
  naming the key and mismatched values or missing key.
- OBL-002: the rerun's initial whole-corpus check passed, including absence of
  a new atom-header error.
- OBL-003: the reader refusals, verbatim nested-holder example, extension
  preservation and document-drift refusal controls passed. The example without
  `holder` is accepted by the reader; its example UUID is still rejected by the
  separate atom-header check. That distinction remains explicit in the table
  and test. No full-CLI acceptance of another Warrant's UUID is claimed.
- OBL-004: every standard table cell quotes its source or says unspecified;
  every divergence remains a proposal. Positive and planted-negative controls
  for missing quotations, missing proposals and invented citations passed.
  Performer inspection of the document is not an independent document verdict.

The sealed tool's version and SHA-256 are retained. All nine relevant grammar
and loader source files are byte-identical to its recorded source revision.
This is a focused grammar run with the whole-corpus positive check; it is not a
new full Cargo gate or release qualification.

Only the updated grammar document, D-001, was recorded through public
`war deliver`. The prior document and delivery manifest remain in Git at the
pinned base revision. No signed contract atom, authority, judgment, independent
verification or resolution was changed. Implementation is reported complete,
unverified; independent assessment and human acceptance remain separate.
