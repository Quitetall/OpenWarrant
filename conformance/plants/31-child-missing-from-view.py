"""§91.5 test 31 — a child exists but the parent's generated view omits it."""
import pathlib, re
p = pathlib.Path("docs/warrants/OW-WAR-0001/generated/WAR.md")
s = p.read_text()
out = re.sub(r"^- `war://[^`]*` — OW-WAR-0005 .*\n", "", s, flags=re.M)
assert out != s, "plant did not fire: OW-WAR-0005 was not in the Children list"
p.write_text(out)
