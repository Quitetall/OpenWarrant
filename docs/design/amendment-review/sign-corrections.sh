#!/usr/bin/env bash
set -euo pipefail

if [[ ! -t 0 || ! -t 1 ]]; then
  echo 'Run this script in a human terminal; each correction requires your confirmation.' >&2
  exit 2
fi

review_repo=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)
cd -- "$review_repo"
python3 - <<'PY'
from pathlib import Path
import hashlib
import sys

expected = {
    'xtask/src/main.rs': '952bc7b0aaf62e12d3210afca07a5d5390c59b4780a662e5f78a0e4014d30ee6',
    'README.md': '468c44364589226235012fa7be7c434b0602ac03d47189f70b92dddb79aba450',
}
for path, digest in expected.items():
    if hashlib.sha256(Path(path).read_bytes()).hexdigest() != digest:
        sys.exit(f'{path} changed since review. Refresh the review and signing preview first.')
PY
./target/debug/war sign OW-WAR-0060/D-002 --kind behaviour-change
./target/debug/war sign OW-WAR-0062/D-003 --kind behaviour-change
