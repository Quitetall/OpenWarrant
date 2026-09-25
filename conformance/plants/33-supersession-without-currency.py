"""§91.5 test 33, read under OW-ADR-0022 (OW-WAR-0113 AM-002).

§21.2: the superseded Warrant's canonical currency becomes `superseded`.
OW-ADR-0022 makes that a fact DERIVED from the successor's `supersedes`
relation, never a field written into the predecessor, whose bytes are under a
signature. So a supersession with no written currency is the correct state,
and a written one is the error.

    33-supersession-without-currency.py <parent-uuid>            the derived case
    33-supersession-without-currency.py <parent-uuid> --written  the refusal

Both declare OW-WAR-0002 `supersedes` → OW-WAR-0001. `--written` also writes
`currency = "superseded"` into OW-WAR-0001's manifest, which `war check` must
refuse (`relations.currency-authored`).
"""
import pathlib, sys

parent_uuid = sys.argv[1]
written = sys.argv[2:] == ["--written"]
if sys.argv[2:] and not written:
    sys.exit(f"unknown arguments: {sys.argv[2:]}")

p = pathlib.Path("docs/warrants/OW-WAR-0002/manifest.toml")
p.write_text(
    p.read_text()
    + f'\n[[supersedes]]\nref = "war://{parent_uuid}"\nreason = "planted"\nadopts = ["OW-WAR-0003"]\n'
)

if written:
    q = pathlib.Path("docs/warrants/OW-WAR-0001/manifest.toml")
    s = q.read_text()
    anchor = '\nprofile = "delivery"\n'
    if anchor not in s:
        sys.exit("OW-WAR-0001's manifest has no top-level profile line to plant beside")
    q.write_text(s.replace(anchor, anchor + 'currency = "superseded"\n', 1))
