"""§91.5 test 34 — a retired Warrant whose source was removed (§21.4 forbids deletion)."""
import pathlib
m = pathlib.Path("docs/warrants/OW-WAR-0002/manifest.toml")
m.write_text(m.read_text().replace("profile =", 'currency = "superseded"\nprofile =', 1))
for atom in pathlib.Path("docs/warrants/OW-WAR-0002/atoms").glob("*.md"):
    atom.write_text("")
