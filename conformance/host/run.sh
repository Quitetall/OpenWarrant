#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# The `oh.war/liminal-v1` parity suite (OW-WAR-0148 M8; Liminal SAS §100,
# LIM-SAS-RQ-015). README.md beside this file declares the observables.
#
#   bash conformance/host/run.sh [CASES_DIR]
#
# HOST is the command that answers one request on stdin with one response on
# stdout; it defaults to this repository's `target/debug/war host`. A Liminal
# host sets HOST to its own entry point. Every case must match on BOTH its
# byte observable (the response, byte for byte) and its semantic observables
# (observables.json); a difference in either fails the case, by name.
set -uo pipefail
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cases="${1:-$here/cases}"
HOST="${HOST:-$here/../../target/debug/war host}"
pass=0 fail=0
out=$(mktemp)
trap 'rm -f "$out"' EXIT
for dir in "$cases"/*/; do
    name=$(basename "$dir")
    code=0
    # shellcheck disable=SC2086 # HOST is a command line
    $HOST < "$dir/request.json" > "$out" || code=$?
    why=$(python3 - "$dir" "$out" "$code" <<'PY'
import json, sys, hashlib
d, out, code = sys.argv[1], sys.argv[2], int(sys.argv[3])
want = json.load(open(d + "/observables.json"))
got_bytes = open(out, "rb").read()
problems = []
if got_bytes != open(d + "/response.json", "rb").read():
    problems.append("response bytes differ")
try:
    r = json.loads(got_bytes)
except ValueError:
    print("response is not JSON"); sys.exit()
m = r.get("model")
got = {
    "exit": code,
    "outcome": r.get("outcome"),
    "refusal": (r.get("refusal") or {}).get("rule"),
    "basis_digest": (r.get("basis") or {}).get("digest"),
    "model_digest": r.get("model_digest"),
    "records": None if m is None else len(m["records"]),
    "relations": None if m is None else len(m["relations"]),
    "states": None if m is None else len(m["states"]),
    "host_rules": sorted({x["rule"] for x in r.get("diagnostics", [])}),
    "projections": [[p["path"], p["digest"]] for p in r.get("projections", []) if p.get("status") == "rendered"],
}
for k, v in want.items():
    if got.get(k) != v:
        problems.append(f"{k}: {got.get(k)!r}, declared {v!r}")
# The model digest is over the model's compact bytes: recompute it.
if m is not None:
    b = json.dumps(m, ensure_ascii=False, separators=(",", ":")).encode()
    if "sha256:" + hashlib.sha256(b).hexdigest() != r.get("model_digest"):
        problems.append("model_digest is not the digest of the model's bytes")
print("; ".join(problems))
PY
)
    if [[ -z "$why" ]]; then
        printf 'ok    host case %s\n' "$name"
        pass=$((pass + 1))
    else
        printf 'FAIL  host case %s: %s\n' "$name" "$why"
        fail=$((fail + 1))
    fi
done
echo "$pass passed, $fail failed"
[[ "$fail" -eq 0 && "$pass" -gt 0 ]]
