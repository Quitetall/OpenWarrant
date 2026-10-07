#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Build conformance/host/cases/ from a scratch program (OW-WAR-0148 M8).
#
#   bash conformance/host/make-cases.sh [WAR]
#
# WAR defaults to ./target/debug/war. The program is new each run (its Warrant
# UUIDs and ticket ids are random), so the requests this writes are new bytes
# each time: run it only to add or replace cases, and commit what it wrote.
# Each response is what `war host` answers to its request; `regen.sh`
# rewrites only the responses, from the committed requests.
#
# Nothing here signs: no key, no agent (SSH_AUTH_SOCK is unset below).
set -euo pipefail
unset SSH_AUTH_SOCK SSH_AGENT_PID OPENWARRANT_ACTOR
export OPENWARRANT_NO_PROJECTS=1 OPENWARRANT_NO_UPDATE_CHECK=1
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
R="$PWD"
WAR="$(cd "$(dirname "${1:-./target/debug/war}")" && pwd)/$(basename "${1:-./target/debug/war}")"
CASES="$R/conformance/host/cases"
D=$(mktemp -d)
trap 'rm -rf "$D"' EXIT
g() { git -C "$D" -c user.email=host@invalid -c user.name=host "$@"; }
w() { "$WAR" --root "$D" "$@"; }

git -C "$D" init -q .
w init --program "Host Corpus" --namespace HC >/dev/null
w compile >/dev/null
g add -A && g commit -qm baseline

# python3 helpers: write a case from a request (and its declared observables)
case_out() { # <name> <request-file> ; answer it, record exit and observables
    # ONLY_CASE=<name> writes that case alone: the scratch program is new on
    # every run, so a full run rewrites every case's bytes.
    [[ -n "${ONLY_CASE:-}" && "$1" != "$ONLY_CASE" ]] && return 0
    local name="$1" req="$2" dir="$CASES/$1" code=0
    mkdir -p "$dir"
    cp "$req" "$dir/request.json"
    "$WAR" host < "$dir/request.json" > "$dir/response.json" || code=$?
    python3 - "$dir" "$code" <<'PY'
import json, sys, hashlib
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
    echo "case $name: exit $code"
}

# 1. The password-reset records under the delivery profile.
mkdir -p "$D/profiles" "$D/docs/records/password-reset"
cp "$R/profiles/delivery.toml" "$R/profiles/decision.toml" "$R/profiles/ticket.toml" "$D/profiles/"
cp "$R/docs/records/password-reset/10-records.md" "$D/docs/records/password-reset/"
g add -A && g commit -qm records
w host --export > "$D/req"
case_out records-password-reset "$D/req"

# 2. A ticket, whose first item implements REQ-pr1.
w create "Add password reset" -i "Expire tokens after issue (implements REQ-pr1)" -i "Write the reset page" >/dev/null
g add -A && g commit -qm ticket
w host --export > "$D/req"
case_out ticket "$D/req"

# 3. A second Warrant (unsigned, so a draft), every Warrant's views requested.
w new "Send the reset email" >/dev/null
w compile >/dev/null
g add -A && g commit -qm warrant
w host --export --projection 'warrant:*' > "$D/req"
case_out warrant "$D/req"

# 3b. The four document types (OW-WAR-0148 M6) and their declared
#     projections over the password-reset records: a hosted rendering is the
#     one `war compile` writes.
cp "$R/profiles/prd.toml" "$R/profiles/architecture.toml" "$R/profiles/test-plan.toml" "$R/profiles/agent-packet.toml" "$D/profiles/"
cp "$R/docs/records/password-reset/20-product.md" "$R/docs/records/password-reset/30-architecture.md" "$R/docs/records/password-reset/documents.toml" "$D/docs/records/password-reset/"
w compile >/dev/null
g add -A && g commit -qm documents
w host --export --projection 'document:*' > "$D/req"
case_out documents "$D/req"

# 4. Compiled, not established: the request holds a Node the basis does not
#    compile to (REQ-pr1 at another revision).
python3 - "$D/req" > "$D/req2" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
n = [x for x in r["nodes"] if x["id"] == "REQ-pr1"][0]
n["revision"] = "sha256:" + "0" * 64
r["options"].pop("projections", None)
if not r["options"]:
    del r["options"]
print(json.dumps(r, ensure_ascii=False, separators=(",", ":")))
PY
case_out nodes-differ "$D/req2"

# 5. Compiled, not established: a member the readers need is withheld.
python3 - "$D/req" > "$D/req2" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
m = [x for x in r["basis"]["members"] if x["path"] == "docs/records/password-reset/10-records.md"][0]
m.pop("utf8"); m["withheld"] = True
r.pop("options", None)
print(json.dumps(r, ensure_ascii=False, separators=(",", ":")))
PY
case_out member-withheld "$D/req2"

# 6. Not compiled: a basis that is no repository (no openwarrant.toml).
python3 - > "$D/req2" <<'PY'
import json, hashlib
b = "# notes\n"
d = "sha256:" + hashlib.sha256(b.encode()).hexdigest()
pairs = json.dumps([["notes.md", d]], separators=(",", ":"))
r = {"protocol": "oh.war/liminal-v1", "version": 1,
     "basis": {"id": "liminal-basis://no-repository",
               "digest": "sha256:" + hashlib.sha256(pairs.encode()).hexdigest(),
               "members": [{"path": "notes.md", "digest": d, "utf8": b}]},
     "profiles": [], "nodes": [], "relations": []}
print(json.dumps(r, separators=(",", ":")))
PY
case_out no-repository "$D/req2"

# Refusals: each by name, before any work. Built from the ticket request.
w host --export > "$D/base"
refusal() { # <name> <python expression over r, mutating it>
    python3 - "$D/base" "$2" > "$D/req2" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
exec(sys.argv[2])
print(json.dumps(r, ensure_ascii=False, separators=(",", ":")))
PY
    case_out "$1" "$D/req2"
}
refusal refuse-path-not-bytes 'm = r["basis"]["members"][0]; m.pop("utf8", None); m.pop("hex", None); m["file"] = "/etc/passwd"'
refusal refuse-member-without-bytes 'm = r["basis"]["members"][0]; m.pop("utf8", None); m.pop("hex", None)'
refusal refuse-basis-locator 'r["basis"]["root"] = "/srv/repository"'
refusal refuse-version 'r["version"] = 2'
refusal refuse-protocol 'r["protocol"] = "oh.war/liminal-v0"'
refusal refuse-limit 'r["options"] = {"limits": {"members": 3}}'
refusal refuse-member-digest 'm = r["basis"]["members"][0]; m["utf8"] = m["utf8"] + " "'
refusal refuse-basis-digest 'r["basis"]["digest"] = "sha256:" + "0" * 64'
refusal refuse-member-path 'r["basis"]["members"][0]["path"] = "../outside.md"'
refusal refuse-unknown-field 'r["repository"] = "/srv/repository"'
printf '{"protocol":"oh.war/liminal-v1","version":1,' > "$D/req2"
case_out refuse-malformed "$D/req2"
