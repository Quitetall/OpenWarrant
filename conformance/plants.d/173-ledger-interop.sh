# shellcheck shell=bash
# OW-WAR-0148 M17 (docs/LEDGER.md): the ledger reads and writes Agent Trace
# records (Cursor's RFC v0.1.0, agent-trace.dev) and git-ai notes (Git AI
# Standard v3.0.0, refs/notes/ai). On a scratch program (LI), each claim
# paired with a refusal:
# - Agent Trace round-trips: export, delete the atoms, import, and the atoms
#   come back byte for byte; each record carries `version`, `id`,
#   `timestamp`, `files[].conversations[].ranges`, `vcs {git, <sha>}`. A
#   record from another tool reads in as `agent-trace` naming its contributor
#   and lines. Refused: a record whose version is not 0.1, and one with no
#   `vcs` (ledger.agent-trace, exit 2), each writing nothing.
# - git-ai notes round-trip the same way, and a note git-ai already wrote
#   keeps its attestations and prompts beside the ledger's session; its
#   AI-attested lines read in as `git-ai`. Refused: a note with no `---`
#   divider is skipped with a warning (ledger.git-ai), writing nothing.

echo "== the ledger reads and writes Agent Trace and git-ai notes (M17) =="
LI_TMP=$(mktemp -d)
LI_ROOT=$(scratch_corpus LI)
[[ -d "${LI_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
LI_WAR="$REPO_ROOT/${WAR#./}"
li_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
li_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
li_war() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR GIT_AUTHOR_NAME=plant GIT_AUTHOR_EMAIL=plant@invalid \
    GIT_COMMITTER_NAME=plant GIT_COMMITTER_EMAIL=plant@invalid "$LI_WAR" --root "$LI_ROOT" "$@" </dev/null; }
li_git() { git -C "$LI_ROOT" -c user.email=plant@invalid -c user.name=plant -c commit.gpgSign=false "$@"; }
li_sum() { (cd "$LI_ROOT" && find docs/ledger -type f -name '*.md' | LC_ALL=C sort | xargs sha256sum 2>/dev/null | sha256sum | cut -d' ' -f1); }

LI_ID=$(li_war --json create "Parse the header" 2>/dev/null \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["id"])' 2>/dev/null)
mkdir -p "$LI_ROOT/src"
printf 'one\ntwo\nthree\n' > "$LI_ROOT/src/h.rs"
li_git add -A >/dev/null 2>&1
li_git commit -qm "Parse the header" -m "Ledger: src/h.rs: the header is read once" -m "Warrant: $LI_ID" >/dev/null 2>&1
printf 'one\n' > "$LI_ROOT/src/k.rs"
li_git add -A >/dev/null 2>&1
li_git commit -qm "Add k" >/dev/null 2>&1
LI_K=$(git -C "$LI_ROOT" rev-parse HEAD)
li_war admin ledger record HEAD~2..HEAD >/dev/null 2>&1
LI_SUM0=$(li_sum)

# --- Agent Trace -------------------------------------------------------------------------------
li_war admin ledger export --agent-trace --out "$LI_TMP/at.jsonl" >/dev/null 2>&1
LI_AT_SHAPE=$(python3 - "$LI_TMP/at.jsonl" <<'PY' 2>&1
import json, sys
recs = [json.loads(l) for l in open(sys.argv[1])]
ok = len(recs) == 2 and all(
    r["version"] == "0.1.0" and len(r["id"]) == 36 and r["timestamp"].endswith("Z")
    and r["vcs"]["type"] == "git" and len(r["vcs"]["revision"]) == 40
    and all(f["path"] and f["conversations"][0]["ranges"] is not None for f in r["files"])
    and "dev.openwarrant" in r["metadata"]
    for r in recs)
print("shape" if ok else "wrong: %r" % recs[:1])
PY
)
command rm -rf "$LI_ROOT/docs/ledger"
li_war admin ledger import --agent-trace "$LI_TMP/at.jsonl" >/dev/null 2>&1
LI_SUM1=$(li_sum)
cat > "$LI_TMP/cursor.json" <<JSON
{"version": "0.1.0", "id": "6ef2299e-a67f-432b-aa80-3d2fb4d28999", "timestamp": "2026-03-04T05:06:07Z",
 "vcs": {"type": "git", "revision": "$LI_K"}, "tool": {"name": "cursor", "version": "2.0"},
 "files": [{"path": "src/m.rs", "conversations": [{"contributor": {"type": "ai", "model_id": "anthropic/claude-x"},
   "ranges": [{"start_line": 1, "end_line": 12}]}]}]}
JSON
li_war admin ledger import --agent-trace "$LI_TMP/cursor.json" >/dev/null 2>&1
LI_M=$(grep -F '(agent-trace): ' "$LI_ROOT/docs/ledger/src/m.rs.md" 2>/dev/null)
python3 -c 'import json,sys; v=json.load(open(sys.argv[1])); v["version"]="2.0"; v["files"][0]["path"]="src/v.rs"; json.dump(v, open(sys.argv[2],"w"))' "$LI_TMP/cursor.json" "$LI_TMP/v2.json"
python3 -c 'import json,sys; v=json.load(open(sys.argv[1])); del v["vcs"]; v["files"][0]["path"]="src/n.rs"; json.dump(v, open(sys.argv[2],"w"))' "$LI_TMP/cursor.json" "$LI_TMP/novcs.json"
LI_V2=$(li_war admin ledger import --agent-trace "$LI_TMP/v2.json" 2>&1); LI_V2_RC=$?
LI_NV=$(li_war admin ledger import --agent-trace "$LI_TMP/novcs.json" 2>&1); LI_NV_RC=$?
if [[ "$LI_AT_SHAPE" == "shape" && -n "$LI_SUM0" && "$LI_SUM1" == "$LI_SUM0" \
    && "$LI_M" == *"$LI_K - (agent-trace): Agent Trace via cursor: ai (anthropic/claude-x) lines 1-12" ]]; then
    li_ok "Agent Trace round-trips" "2 records exported, atoms back byte for byte; a Cursor record reads in as agent-trace"
else
    li_fail "Agent Trace round-trips" "shape '$LI_AT_SHAPE'; sums $LI_SUM0 -> $LI_SUM1; m '$LI_M'"
fi
if [[ $LI_V2_RC -eq 2 && $LI_NV_RC -eq 2 && ! -e "$LI_ROOT/docs/ledger/src/v.rs.md" && ! -e "$LI_ROOT/docs/ledger/src/n.rs.md" ]] \
    && line_has -F 'ledger.agent-trace' -F 'is not Agent Trace 0.1' <<<"$LI_V2" \
    && line_has -F 'ledger.agent-trace' -F 'no `vcs`' <<<"$LI_NV"; then
    li_ok "a foreign record is refused" "version 2.0, and no vcs: ledger.agent-trace, exit 2, nothing written"
else
    li_fail "a foreign record is refused" "v2 exit $LI_V2_RC: $LI_V2; no vcs exit $LI_NV_RC: $LI_NV"
fi

# --- git-ai notes -----------------------------------------------------------------------------
command rm -f "$LI_ROOT/docs/ledger/src/m.rs.md"
LI_SUM0=$(li_sum)
LI_THEIRS=$(git -C "$LI_ROOT" rev-parse HEAD~1)
cat > "$LI_TMP/theirs.note" <<NOTE
src/h.rs
  abcd1234abcd1234 2-3
---
{"schema_version": "authorship/3.0.0", "git_ai_version": "1.0.23", "base_commit_sha": "$LI_THEIRS",
 "prompts": {"abcd1234abcd1234": {"agent_id": {"tool": "claude", "id": "s1", "model": "opus"},
  "total_additions": 2, "total_deletions": 0, "accepted_lines": 2, "overriden_lines": 0}}}
NOTE
li_git notes --ref=ai add -f -F "$LI_TMP/theirs.note" "$LI_THEIRS" >/dev/null 2>&1
li_war admin ledger export --git-ai >/dev/null 2>&1
LI_NOTE=$(git -C "$LI_ROOT" notes --ref=ai show "$LI_THEIRS" 2>/dev/null)
LI_KEPT=$(python3 -c 'import json,sys
t = sys.stdin.read(); att, meta = t.split("\n---\n", 1); m = json.loads(meta)
print(att.startswith("src/h.rs\n  abcd1234abcd1234 2-3"), "abcd1234abcd1234" in m["prompts"], len(m["sessions"]))' <<<"$LI_NOTE" 2>&1)
command rm -rf "$LI_ROOT/docs/ledger"
li_war admin ledger import --git-ai >/dev/null 2>&1
LI_SUM2=$(li_sum)
# A note git-ai wrote alone (no ledger session) reads in as git-ai.
li_git notes --ref=ai add -f -F "$LI_TMP/theirs.note" "$LI_THEIRS" >/dev/null 2>&1
command rm -rf "$LI_ROOT/docs/ledger"
li_war admin ledger import --git-ai >/dev/null 2>&1
LI_H=$(grep -F '(git-ai): ' "$LI_ROOT/docs/ledger/src/h.rs.md" 2>/dev/null)
printf 'src/h.rs\n  abcd1234abcd1234 1\n{}\n' > "$LI_TMP/broken.note"
li_git notes --ref=ai add -f -F "$LI_TMP/broken.note" "$LI_K" >/dev/null 2>&1
command rm -f "$LI_ROOT/docs/ledger/src/k.rs.md"
LI_BROKEN=$(li_war admin ledger import --git-ai 2>&1)
if [[ "$LI_KEPT" == "True True 1" && -n "$LI_SUM0" && "$LI_SUM2" == "$LI_SUM0" \
    && "$LI_H" == *" $LI_ID (git-ai): AI-authored (git-ai): claude (opus) lines 2-3" ]]; then
    li_ok "git-ai notes round-trip" "export kept git-ai's attestation and prompt beside one session; import gives the atoms back; git-ai's lines read in"
else
    li_fail "git-ai notes round-trip" "kept '$LI_KEPT'; sums $LI_SUM0 -> $LI_SUM2; h '$LI_H'"
fi
if [[ ! -e "$LI_ROOT/docs/ledger/src/k.rs.md" ]] && line_has -F 'ledger.git-ai' -F 'does not read as v3' <<<"$LI_BROKEN"; then
    li_ok "a malformed note is refused" "no --- divider: ledger.git-ai, skipped, nothing written"
else
    li_fail "a malformed note is refused" "$LI_BROKEN"
fi

command rm -rf "$LI_TMP"
