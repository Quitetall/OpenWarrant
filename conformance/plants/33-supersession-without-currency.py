"""§91.5 test 33 — supersession declared, the superseded Warrant's currency left alone."""
import pathlib, sys
parent_uuid = sys.argv[1]
p = pathlib.Path("docs/warrants/OW-WAR-0002/manifest.toml")
p.write_text(
    p.read_text()
    + f'\n[[supersedes]]\nref = "war://{parent_uuid}"\nreason = "planted"\nadopts = ["OW-WAR-0003"]\n'
)
