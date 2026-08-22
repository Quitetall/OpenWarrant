"""§91.5 test 35 — a superseding Warrant that adopts nothing while the superseded has children."""
import pathlib, sys
parent_uuid = sys.argv[1]
p = pathlib.Path("docs/warrants/OW-WAR-0002/manifest.toml")
p.write_text(p.read_text() + f'\n[[supersedes]]\nref = "war://{parent_uuid}"\nreason = "planted"\n')
m = pathlib.Path("docs/warrants/OW-WAR-0001/manifest.toml")
m.write_text(m.read_text().replace("profile =", 'currency = "superseded"\nprofile =', 1))
