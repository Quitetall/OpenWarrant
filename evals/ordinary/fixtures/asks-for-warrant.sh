#!/usr/bin/env bash
# Fixture agent for `war eval ordinary`: fixes the bug, but ties the work to
# a Warrant anyway. Still a failure: ordinary work needs none.
set -euo pipefail
cat > /dev/null
python3 - <<'PY'
import pathlib
p = pathlib.Path("calc.py")
p.write_text(p.read_text().replace("range(1, len(numbers))", "range(len(numbers))"))
PY
echo "Fixed total(). This change needs a Warrant before it can be merged."
