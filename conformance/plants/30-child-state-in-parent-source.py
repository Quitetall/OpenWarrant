"""§91.5 tests 30 and 32 — a child's STATE written into a parent's authored atom."""
import pathlib
p = pathlib.Path("docs/warrants/OW-WAR-0001/atoms/20-basis.md")
p.write_text(p.read_text() + "\nOW-WAR-0002 is resolved, so this basis now depends on it.\n")
