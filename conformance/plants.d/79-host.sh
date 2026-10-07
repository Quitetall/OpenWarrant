# shellcheck shell=bash
# OW-WAR-0148 M8 — `war host` speaks oh.war/liminal-v1 (SAS §11.3, §82.2–82.4;
# docs/LIMINAL_HOST.md). One request on stdin, one response on stdout, built
# by the same `model::build` as `war model`, over a basis held in memory.
#
# Accepted: every case of conformance/host/ matches its response byte for
# byte and its declared observables; compiling this repository and a scratch
# program directly (`war model`) and through their exported requests (`war
# host --export | war host`) gives the same model bytes, and every Warrant's
# views equal the committed ones; a hosted run opens no file of any
# repository, spawns nothing and opens no socket (observed with strace).
# Refused, each by name with nothing written: a path instead of bytes
# (host.path-not-bytes), an unsupported version (host.version), input over
# the limit (host.limit). Refusal controls for the comparison itself: a
# doctored fixture response fails its case, and a doctored observation
# (a signature verdict flipped) moves the hosted model off the standalone one.

echo "== war host: oh.war/liminal-v1 (OW-WAR-0148 M8) =="
H_TMP=$(mktemp -d)
H_WAR="$REPO_ROOT/${WAR#./}"
H_CASES="$REPO_ROOT/conformance/host/cases"
h_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
h_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
h_war() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID "$H_WAR" "$@"; }

# --- the fixtures ---------------------------------------------------------
H_N=$(find "$H_CASES" -mindepth 1 -maxdepth 1 -type d | wc -l)
h_out=$(HOST="$H_WAR host" bash "$REPO_ROOT/conformance/host/run.sh" "$H_CASES" 2>&1)
h_status=$?
if [[ "$h_status" -eq 0 && "$H_N" -ge 17 ]] && grep -q "^$H_N passed, 0 failed$" <<<"$h_out"; then
    h_ok "host fixtures pass" "$H_N cases, bytes and observables"
else
    h_fail "host fixtures pass" "exit $h_status: $(grep -E '^FAIL|passed' <<<"$h_out" | head -3 | tr '\n' ' ')"
fi

# Refusal control: one byte of a fixture response changed is a failed case.
mkdir -p "$H_TMP/doctored"
cp -r "$H_CASES/warrant" "$H_TMP/doctored/"
python3 - "$H_TMP/doctored/warrant/response.json" <<'PY'
import sys
p = sys.argv[1]
s = open(p, "rb").read()
assert s.count(b'"value":"draft"') >= 1
open(p, "wb").write(s.replace(b'"value":"draft"', b'"value":"drafT"', 1))
PY
h_out=$(HOST="$H_WAR host" bash "$REPO_ROOT/conformance/host/run.sh" "$H_TMP/doctored" 2>&1)
h_status=$?
if [[ "$h_status" -ne 0 ]] && grep -q '^FAIL  host case warrant: response bytes differ' <<<"$h_out"; then
    h_ok "a doctored fixture response fails" "host case warrant: response bytes differ"
else
    h_fail "a doctored fixture response fails" "exit $h_status: $h_out"
fi

# --- refusals by name, nothing written -------------------------------------
# h_refused <name> <rule> <request-file>: exit 1, the rule named, the working
# directory left empty and this repository's status unmoved.
h_refused() {
    local name="$1" rule="$2" req="$3" cwd before after code=0
    cwd=$(mktemp -d "$H_TMP/cwd.XXXX")
    before=$(git -C "$REPO_ROOT" status --porcelain=v1 --untracked-files=all)
    (cd "$cwd" && h_war host < "$req" > "$H_TMP/refused.json") || code=$?
    after=$(git -C "$REPO_ROOT" status --porcelain=v1 --untracked-files=all)
    local got
    got=$(python3 -c 'import json,sys; r=json.load(open(sys.argv[1])); print(r["outcome"], (r.get("refusal") or {}).get("rule"), r["model"] is None)' "$H_TMP/refused.json" 2>&1)
    if [[ "$code" -eq 1 && "$got" == "refused $rule True" && -z "$(ls -A "$cwd")" && "$before" == "$after" ]]; then
        h_ok "$name" "refused by $rule, nothing written"
    else
        h_fail "$name" "exit $code, $got, cwd [$(ls -A "$cwd")]"
    fi
}
h_refused "a path instead of bytes is refused" host.path-not-bytes "$H_CASES/refuse-path-not-bytes/request.json"
h_refused "an unsupported version is refused" host.version "$H_CASES/refuse-version/request.json"
h_refused "a declared limit is held to" host.limit "$H_CASES/refuse-limit/request.json"
head -c $((256 * 1024 * 1024 + 1)) /dev/zero > "$H_TMP/huge"
h_refused "input over the limit is refused" host.limit "$H_TMP/huge"
command rm -f "$H_TMP/huge"

# --- purity, observed ------------------------------------------------------
# h_pure <name> <request-file>: under strace, the run executes nothing but
# itself, opens no socket, and touches no path under this repository, the
# virtual basis or the scratch directory (the binary's own exec aside).
h_pure() {
    local name="$1" req="$2" log="$H_TMP/strace.log" bad
    if ! command -v strace >/dev/null 2>&1; then
        printf 'UNKNOWN %-32s strace is not installed; purity is not observed\n' "$name"
        return
    fi
    (cd "$H_TMP" && env -u SSH_AUTH_SOCK -u SSH_AGENT_PID strace -f -qq -o "$log" \
        -e trace=%file,%process,%network "$H_WAR" host < "$req" > /dev/null) || true
    bad=$(python3 - "$log" "$REPO_ROOT" "$H_TMP" <<'PY'
import re, sys
log, repo, tmp = sys.argv[1:]
bad = []
execs = 0
for line in open(log):
    if re.search(r"\b(socket|connect|bind|sendto)\(", line):
        bad.append(line.strip())
    if re.search(r"\b(execve|execveat)\(", line):
        execs += 1
        if execs > 1:
            bad.append(line.strip())
        continue
    if re.search(r"\b(clone3?|fork|vfork)\(", line):
        bad.append(line.strip())
    for path in re.findall(r'"(/[^"]*)"', line):
        if path.startswith(("/liminal-basis", repo + "/", tmp)) or path == repo:
            if path.startswith(repo + "/target/") or path == repo + "/target":
                continue  # the binary itself, loaded by the dynamic linker
            bad.append(line.strip())
print("\n".join(bad[:3]))
PY
)
    if [[ -z "$bad" ]]; then
        h_ok "$name" "no file of the basis, no process, no socket"
    else
        h_fail "$name" "$bad"
    fi
}
h_pure "a hosted compile is pure" "$H_CASES/warrant/request.json"
h_pure "a refusal is pure" "$H_CASES/refuse-version/request.json"

# --- standalone versus hosted ----------------------------------------------
# h_parity <name> <root>: `war model` there, and its exported request through
# `war host`: the hosted model's own bytes (model_digest) are the digest of
# the standalone model's compact bytes, the two compact forms are equal, and
# every Warrant view the host rendered is the committed file.
h_parity() {
    local name="$1" root="$2" code=0
    h_war --root "$root" --json model > "$H_TMP/standalone.json" 2>/dev/null
    h_war --root "$root" host --export --projection 'warrant:*' > "$H_TMP/request.json" 2>"$H_TMP/export.err" \
        || { h_fail "$name" "export: $(head -c 300 "$H_TMP/export.err")"; return; }
    h_war host < "$H_TMP/request.json" > "$H_TMP/hosted.json" || code=$?
    local got
    got=$(python3 - "$H_TMP/standalone.json" "$H_TMP/hosted.json" "$root" "$code" <<'PY'
import hashlib, json, sys
s, h, root, code = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
compact = lambda v: json.dumps(v, ensure_ascii=False, separators=(",", ":")).encode()
a = compact(json.load(open(s))["result"])
r = json.load(open(h))
if code != 0 or r["model"] is None:
    print(f"host exit {code}: {r['outcome']} {[d['rule'] for d in r['diagnostics']][:3]} {r.get('refusal')}")
    sys.exit()
b = compact(r["model"])
problems = []
if a != b:
    problems.append("model bytes differ")
if "sha256:" + hashlib.sha256(a).hexdigest() != r["model_digest"]:
    problems.append("standalone bytes are not the hosted model_digest")
views = [p for p in r["projections"] if p["status"] == "rendered"]
same = sum(open(f"{root}/{p['path']}", "rb").read() == p["utf8"].encode() for p in views)
if not views or same != len(views):
    problems.append(f"{same}/{len(views)} views equal the committed files")
print("; ".join(problems) or f"OK {len(r['model']['records'])} records, {same} views")
PY
)
    if [[ "$got" == OK* ]]; then
        h_ok "$name" "byte-identical: ${got#OK }"
    else
        h_fail "$name" "$got"
    fi
}
h_parity "standalone = hosted, this repository" "$REPO_ROOT"

H_ROOT=$(scratch_corpus HP)
[[ -d "${H_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
mkdir -p "$H_ROOT/profiles" "$H_ROOT/docs/records/password-reset"
cp "$REPO_ROOT/profiles/delivery.toml" "$REPO_ROOT/profiles/decision.toml" "$REPO_ROOT/profiles/ticket.toml" "$H_ROOT/profiles/"
cp "$REPO_ROOT/docs/records/password-reset/10-records.md" "$H_ROOT/docs/records/password-reset/"
h_war --root "$H_ROOT" create "Add password reset" -i "Expire tokens (implements REQ-pr1)" >/dev/null 2>&1
h_war --root "$H_ROOT" new "Send the reset email" >/dev/null 2>&1
h_war --root "$H_ROOT" compile >/dev/null 2>&1
h_parity "standalone = hosted, a scratch program" "$H_ROOT"

# Refusal control for the comparison: flip one supplied signature verdict in
# this repository's request and the hosted model is no longer the standalone
# one — the comparison sees an observation, not just the bytes.
h_war host --export > "$H_TMP/request.json" 2>/dev/null
h_war --json model > "$H_TMP/standalone.json" 2>/dev/null
h_flip=$(python3 - "$H_TMP/request.json" "$H_TMP/flipped.json" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
v = [s for s in r.get("observations", {}).get("signatures", []) if s["verdict"] == "verified"]
if not v:
    print("none"); sys.exit()
for s in v:
    s["verdict"], s["reason"] = "rejected", "doctored by 79-host.sh"
json.dump(r, open(sys.argv[2], "w"), ensure_ascii=False, separators=(",", ":"))
print(len(v))
PY
)
if [[ "$h_flip" == none ]]; then
    h_fail "a doctored observation moves the model" "this repository's request carries no verified signature"
else
    h_war host < "$H_TMP/flipped.json" > "$H_TMP/hosted.json"
    h_cmp=$(python3 - "$H_TMP/standalone.json" "$H_TMP/hosted.json" <<'PY'
import json, sys
c = lambda v: json.dumps(v, ensure_ascii=False, separators=(",", ":"))
a = c(json.load(open(sys.argv[1]))["result"])
r = json.load(open(sys.argv[2]))
print("same" if r["model"] is not None and c(r["model"]) == a else "differs", r["outcome"])
PY
)
    if [[ "$h_cmp" == "differs not_established" ]]; then
        h_ok "a doctored observation moves the model" "$h_flip verdicts flipped: model differs, not established"
    else
        h_fail "a doctored observation moves the model" "$h_cmp"
    fi
fi

command rm -rf "$H_TMP"
unset H_TMP H_WAR H_CASES H_N H_ROOT h_out h_status h_flip h_cmp
