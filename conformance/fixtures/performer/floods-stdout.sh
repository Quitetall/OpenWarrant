#!/usr/bin/env bash
# A performer in a loop. `war perform` caps the read and refuses the answer,
# rather than growing until the box runs out of memory.
set -uo pipefail
cat > /dev/null
exec python3 -c '
import sys
line = "x" * 4095 + "\n"
for _ in range(3000):          # ~12 MiB, past the 8 MiB cap
    sys.stdout.write(line)
'
