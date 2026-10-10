# shellcheck shell=bash
# OW-WAR-0148 M15 — external executors: dispatch out, submission back, and
# the same refusals `war evidence submit` applies.
#
# One scratch program (GX). A fake `gh` and a fake `bd` on PATH record every
# argv and stdin; neither reaches a network, and no model runs. Routed by
# label: `hq` → `[go.executors.hq] adapter = "agent-hq"` (its preset argvs:
# `gh issue create ... --assignee @copilot`, then `gh issue view <ref> --json
# comments --jq ...`); `gt` → `adapter = "gastown"` with a configured argv,
# handed the node as Beads issue JSONL.
#
# Accepted: an honest Agent HQ round trip (pending once, then a submission
# carrying the packet's bindings, asking to be verified) is ingested and the
# item lands and is ticked `claimed`; the fake saw the preset argv and the
# brief on stdin. A Gas Town round trip: the fake `bd` was handed a Beads
# line for the item (`<warrant>.1`, its text), and its answer lands it.
# Refused, nothing ticked, each by rule: an adapter answering `{"status":
# "done"}` with no submission (go.adapter-answer); a submission asking to be
# resolved (submission.self-completion); a submission for a dispatch nobody
# compiled (submission.unknown-dispatch). Each is set aside for a person.

echo "== war go: external executors through the same refusals (M15) =="
GX_ROOT=$(scratch_corpus GX)
[[ -d "${GX_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
GX_TMP=$(mktemp -d)
GX_WAR="$REPO_ROOT/${WAR#./}"
gx_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
gx_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
gxw() { env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK -u SSH_AGENT_PID PATH="$GX_TMP/bin:$PATH" \
    FAKE_DIR="$GX_TMP/fake" "$GX_WAR" --root "$GX_ROOT" "$@" </dev/null; }
gx_field() { python3 -c "import json,sys; v=json.loads(sys.argv[1]); print($2)" "$1" 2>/dev/null; }
gx_new() { gx_field "$(gxw --json create "$@" 2>/dev/null)" 'v["result"]["id"] + "/" + v["result"]["items"][0]["id"]'; }

mkdir -p "$GX_TMP/bin" "$GX_TMP/fake"
# The fake gh: `issue create` keeps the brief and prints the issue URL;
# `issue view` answers pending once, then as the issue's title says.
cat > "$GX_TMP/bin/gh" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$*" >> "$FAKE_DIR/gh.argv"
cat > "$FAKE_DIR/stdin.$$"
GH_STDIN="$FAKE_DIR/stdin.$$" exec python3 - "$@" <<'PY'
import json, os, re, sys
d = os.environ["FAKE_DIR"]
a = sys.argv[1:]
if a[:2] == ["issue", "create"]:
    title = a[a.index("--title") + 1]
    n = len([f for f in os.listdir(d) if f.endswith(".brief")]) + 1
    open(f"{d}/{n}.brief", "w").write(open(os.environ["GH_STDIN"]).read())
    open(f"{d}/{n}.title", "w").write(title)
    print(f"https://github.com/o/r/issues/{n}")
elif a[:2] == ["issue", "view"]:
    n = a[2].rsplit("/", 1)[1]
    polls = f"{d}/{n}.polls"
    seen = int(open(polls).read()) if os.path.exists(polls) else 0
    open(polls, "w").write(str(seen + 1))
    if seen == 0:
        print(json.dumps({"status": "pending"}))
        sys.exit(0)
    title = open(f"{d}/{n}.title").read()
    brief = open(f"{d}/{n}.brief").read()
    answer = json.loads(re.search(r"```json war-submission\n(.*?)```", brief, re.S).group(1))
    if "says done" in title:
        print(json.dumps({"status": "done"}))
    elif "asks resolve" in title:
        answer["requested_next_action"] = "resolve"
        print(json.dumps({"status": "submitted", "submission": answer}))
    elif "forged" in title:
        answer["dispatch_id"] = "01a00000-0000-7000-8000-000000000000"
        print(json.dumps({"status": "submitted", "submission": answer}))
    else:
        print(json.dumps({"status": "submitted", "submission": answer}))
else:
    sys.exit(3)
PY
SH
# The fake bd: `create` keeps the Beads line and the packet, `answer` answers.
cat > "$GX_TMP/bin/bd" <<'SH'
#!/usr/bin/env bash
printf '%s\n' "$*" >> "$FAKE_DIR/bd.argv"
case "$1" in
  create) cat > "$FAKE_DIR/bd.stdin"; cp "$WAR_GO_PACKET" "$FAKE_DIR/bd.packet"; echo "bd-7" ;;
  answer) python3 -c 'import json,sys; p=json.load(open(sys.argv[1])); print(json.dumps({"status":"submitted","submission":p["answer"] | {"requested_next_action":"verify"}}))' "$FAKE_DIR/bd.packet" ;;
  *) exit 3 ;;
esac
SH
chmod +x "$GX_TMP/bin/gh" "$GX_TMP/bin/bd"

GX_H=$(gx_new "Honest work" -l hq -i h1)
GX_D=$(gx_new "Adapter says done" -l hq -i d1)
GX_R=$(gx_new "Answer asks resolve" -l hq -i r1)
GX_F=$(gx_new "Answer forged" -l hq -i f1)
GX_G=$(gx_new "Gas Town work" -l gt -i g1)
cat >> "$GX_ROOT/openwarrant.toml" <<'EOF'

[go]
max_parallel = 3
max_attempts = 1
poll_secs = 1

[go.route]
hq = "hq"
gt = "gt"

[go.executors.hq]
adapter = "agent-hq"

[go.executors.gt]
adapter = "gastown"
dispatch_argv = ["bd", "create"]
poll_argv = ["bd", "answer", "{ref}"]
EOF
git -C "$GX_ROOT" add -A >/dev/null 2>&1
git -C "$GX_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "five Warrants" >/dev/null 2>&1

GX_OUT=$(gxw --json evidence go 2>/dev/null); GX_RC=$?
gx_sum() { gx_field "$GX_OUT" "$1"; }
GX_LANDED=$(gx_sum '",".join(sorted(x["node"] + ":" + x["level"] for x in v["result"]["landed"]))')
GX_TICKED=$(cat "$GX_ROOT"/docs/tickets/*/atoms/15-checklist.md | grep -c '^- \[x\]')
GX_CREATE=$(grep -c '^issue create --title Honest work: h1 --body-file - --assignee @copilot$' "$GX_TMP/fake/gh.argv")
GX_VIEW=$(grep -c '^issue view https://github.com/o/r/issues/[0-9]* --json comments --jq ' "$GX_TMP/fake/gh.argv")
GX_BRIEF=$(grep -l '^## Answer' "$GX_TMP"/fake/*.brief 2>/dev/null | wc -l)
# `landed` is read sorted by node id, and ticket ids are random: the two
# expected nodes are sorted the same way, never assumed to fall G before H.
GX_WANT=$(printf '%s\n' "$GX_G:claimed" "$GX_H:claimed" | LC_ALL=C sort | paste -sd, -)
if [[ -n "$GX_G" && -n "$GX_H" && "$GX_LANDED" == "$GX_WANT" &&"$GX_TICKED" == "2" && "$GX_CREATE" == "1" \
    && "$GX_VIEW" -ge 5 && "$GX_BRIEF" == "4" ]]; then
    gx_ok "an Agent HQ round trip lands" "issue created with the preset argv, pending once, answered; $GX_H ticked claimed"
else
    gx_fail "an Agent HQ round trip lands" "exit $GX_RC; landed '$GX_LANDED', $GX_TICKED ticked, create $GX_CREATE, view $GX_VIEW, briefs $GX_BRIEF"
fi
GX_BEADS=$(python3 -c 'import json,sys; v=json.loads(open(sys.argv[1]).readline()); print(v.get("id","") + " " + v.get("title",""))' "$GX_TMP/fake/bd.stdin" 2>/dev/null)
if [[ "$GX_BEADS" == "${GX_G%/*}.1 g1" ]] && grep -q '^create$' "$GX_TMP/fake/bd.argv" && grep -q '^answer bd-7$' "$GX_TMP/fake/bd.argv"; then
    gx_ok "a Gas Town round trip lands" "bd was handed the Beads issue \"$GX_BEADS\" and polled as bd-7; $GX_G landed"
else
    gx_fail "a Gas Town round trip lands" "beads title '$GX_BEADS'"
fi
GX_RULES=$(python3 - "$GX_OUT" "$GX_D" "$GX_R" "$GX_F" <<'PY'
import json, sys
v = json.loads(sys.argv[1])
blocked = {x["node"] for x in v["result"]["blocked"]}
out = []
for node, want in zip(sys.argv[2:], ["go.adapter-answer", "submission.self-completion", "submission.unknown-dispatch"]):
    detail = " ".join(x["detail"] for x in v["result"]["requeued"] + v["result"]["blocked"] if x["node"] == node)
    out.append("ok" if want in detail and node in blocked else f"{node}:{want}?")
print(" ".join(out))
PY
)
if [[ "$GX_RULES" == "ok ok ok" ]] && ! grep -q '^- \[x\] [dfr]1' "$GX_ROOT"/docs/tickets/*/atoms/15-checklist.md; then
    gx_ok "an adapter cannot declare done" "status done (go.adapter-answer), resolve (self-completion), forged dispatch (unknown-dispatch): none ticked"
else
    gx_fail "an adapter cannot declare done" "$GX_RULES"
fi

command rm -rf "$GX_TMP"
unset GX_ROOT GX_TMP GX_WAR GX_H GX_D GX_R GX_F GX_G GX_OUT GX_RC GX_LANDED GX_TICKED GX_CREATE GX_VIEW \
    GX_BRIEF GX_BEADS GX_RULES
unset -f gx_ok gx_fail gxw gx_field gx_new gx_sum
