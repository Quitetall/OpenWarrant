# Warrant corpus conventions

Conventions this repository follows where the SAS permits more than one reading.
Recorded because an external review of the corpus asked all of these, which means
a reader cannot infer them from the files.

## `[[parents]]` is authorization lineage, not execution order

SAS §20.1: a parent WAR "preserves the originating context, rationale, and
**authorization** from which child work is decomposed." It says nothing about
sequencing.

So OW-WAR-0002 through OW-WAR-0005 each declare exactly one parent, OW-WAR-0001,
because 0001 is the Warrant that authorized this repository to exist and from
which all four decompose. They do **not** declare each other.

Execution prerequisites are a different relation and live in the `basis` atom
under *Prerequisites*. OW-WAR-0005 therefore declares one parent (0001) and three
prerequisites (0002, 0003, 0004), and that is not a contradiction — it cannot be
started before those three resolve, and it derives its authorization from none of
them.

The distinction matters for a reader deciding what a child may assume. §20.2's
`inherited_context_selectors` inherit from the *parent*; nothing is inherited
from a prerequisite.

## Known gap: children do not yet carry `contract_digest`

§20.2 requires a child to reference its parent's `warrant_ref`,
`contract_revision`, **and** `contract_digest`, plus
`inherited_context_selectors`. These manifests carry the first two only.

`contract_digest` cannot be computed until OW-WAR-0003 delivers canonical IR and
digesting, and writing a placeholder digest would be worse than omitting the
field — a wrong digest that looks right is exactly the class of claim this
protocol exists to prevent. `inherited_context_selectors` are omitted for the
same reason they are cheap to add later: nothing consumes them yet, and guessing
the selector set before the compiler can resolve one would freeze a guess.

Both are added in OW-WAR-0003, and until then these manifests are knowingly
non-conformant to §20.2. Stated here rather than discovered by the first parser
that enforces it.

## The `decision` profile is unexercised

All five Warrants use the `delivery` profile of §16.3. The `decision` profile —
control, intent, basis, one or more ADRs, assurance, relations_and_integrity —
has no instance in this corpus, so the parser and checker paths for it are
untested by the acceptance corpus itself. Named in the residual risk of
OW-WAR-0003 and OW-WAR-0005.

The first ADR this repository writes will be a `decision` Warrant, which closes
this gap as a side effect of OW-WAR-0003's library selection rather than as
make-work.

## Ordinals follow §16.1, with gaps left deliberately

Atom ordinals are the canonical role order of §16.1: control 00, intent 10,
basis 20, decisions 30, work_order 40, milestones 45, execution 50, assurance 60,
resolution 70, validation 80, relations_and_integrity 90.

Gaps in a manifest's ordinal sequence are therefore meaningful, not sloppy: a
Warrant with 10, 20, 40, 45, 60 has no `decisions` atom because it transcludes no
ADR, and no `execution` atom because execution has not begun. §16.1 requires the
human parent to omit inapplicable optional roles rather than render empty
headings, and the manifest is where that omission is declared.
