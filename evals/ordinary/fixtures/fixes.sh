#!/usr/bin/env bash
# Fixture agent for `war eval ordinary`: does the ordinary work. Reads the
# prompt on stdin, fixes the off-by-one in calc.py, says so. No model.
set -euo pipefail
cat > /dev/null
python3 - <<'PY'
import pathlib
p = pathlib.Path("calc.py")
p.write_text(p.read_text().replace("range(1, len(numbers))", "range(len(numbers))"))
PY
echo "Fixed the off-by-one in total(): the loop now starts at index 0."
