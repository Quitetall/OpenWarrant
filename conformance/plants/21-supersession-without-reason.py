"""§21.1 — `reason` is part of the relation's SHAPE, so its absence is a parse refusal."""
import pathlib, sys
parent_uuid = sys.argv[1]
p = pathlib.Path("docs/warrants/OW-WAR-0002/manifest.toml")
p.write_text(p.read_text() + f'\n[[supersedes]]\nref = "war://{parent_uuid}"\n')
