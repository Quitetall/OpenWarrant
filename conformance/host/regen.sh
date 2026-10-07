#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Rewrite every case's response.json and observables.json from its committed
# request.json, after a deliberate change to what the compiler answers.
# A regenerated response is a new expectation: review its diff like code.
#
#   bash conformance/host/regen.sh [WAR]
set -euo pipefail
unset SSH_AUTH_SOCK SSH_AGENT_PID
here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WAR="${1:-$here/../../target/debug/war}"
for dir in "$here"/cases/*/; do
    code=0
    "$WAR" host < "$dir/request.json" > "$dir/response.json" || code=$?
    python3 - "$dir" "$code" <<'PY'
import json, sys
d, code = sys.argv[1], int(sys.argv[2])
r = json.load(open(d + "/response.json"))
m = r.get("model")
obs = {
    "exit": code,
    "outcome": r["outcome"],
    "refusal": (r.get("refusal") or {}).get("rule"),
    "basis_digest": (r.get("basis") or {}).get("digest"),
    "model_digest": r.get("model_digest"),
    "records": None if m is None else len(m["records"]),
    "relations": None if m is None else len(m["relations"]),
    "states": None if m is None else len(m["states"]),
    "host_rules": sorted({x["rule"] for x in r["diagnostics"]}),
    "projections": [[p["path"], p["digest"]] for p in r["projections"] if p["status"] == "rendered"],
}
json.dump(obs, open(d + "/observables.json", "w"), indent=2)
open(d + "/observables.json", "a").write("\n")
PY
    echo "$(basename "$dir"): exit $code"
done
