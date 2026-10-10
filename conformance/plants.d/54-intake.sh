# shellcheck shell=bash
# OW-WAR-0141 — a sentence or an issue becomes a drafted Warrant, or one
# question; intake authorizes nothing and writes nothing to a tracker.
#
# Everything runs on a scratch program. The drafter is a fake written here (a
# python script answering from the request, no model); `gh` is a fake on PATH
# that records every argv and answers from a fixture issue file. No network,
# no real tracker, no real model. GH_TOKEN and GITHUB_TOKEN carry a planted
# value on every intake run, and the plant looks for it afterwards.
#
# Every claim is paired with a refusal: the path that drafts is shown refusing
# an issue with no title, a drafter that writes, a fetch that fails, and an
# apply nobody reviewed.

echo "== intake (OW-WAR-0141) =="
PLANT_ROOT=$(scratch_corpus IK)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
IK_TMP=$(mktemp -d)
IK_BIN="$IK_TMP/bin"
IK_GH_LOG="$IK_TMP/gh.log"
IK_TOKEN=planted-token-0141
IK_WAR_ABS="$REPO_ROOT/${WAR#./}"
mkdir -p "$IK_BIN"
: > "$IK_GH_LOG"
ln -s "$IK_WAR_ABS" "$IK_BIN/war"

ik_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ik_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }

# The fake tracker CLI: records its argv, answers from $IK_GH_ISSUE, fails or
# hangs when told to.
cat > "$IK_BIN/gh" <<'GH'
#!/usr/bin/env bash
printf '%s\n' "$*" >> "$IK_GH_LOG"
# The environment it was given, beside the argv: the plant asserts the planted
# token reached this child, so "no token in the repository" is not vacuous.
# (Only beside a log the plant named: never into the working directory.)
[[ "${IK_GH_LOG:-}" == /* ]] && \
    printf 'GH_TOKEN=%s GITHUB_TOKEN=%s\n' "${GH_TOKEN-<unset>}" "${GITHUB_TOKEN-<unset>}" >> "$IK_GH_LOG.env"
[[ -n "${IK_GH_FAIL:-}" ]] && { echo "gh: planted failure" >&2; exit 4; }
[[ -n "${IK_GH_SLEEP:-}" ]] && sleep "$IK_GH_SLEEP"
cat "$IK_GH_ISSUE"
GH
chmod +x "$IK_BIN/gh"

# The fake drafter: a request containing THIN, with no answer to Q-001, gets
# one blocker question; anything else gets a complete proposal whose intent
# quotes the request, so two issues' drafts differ only where their text does.
# IK_DRAFTER_WRITES=1 makes it do what §74.5 forbids.
cat > "$IK_TMP/drafter.py" <<'DRAFTER'
import json, os, sys
req = json.load(sys.stdin)
if os.environ.get("IK_DRAFTER_WRITES"):
    open("README.planted-by-intake.md", "w").write("planted\n")
text = req["user_request"]
title = text.splitlines()[0]
if "THIN" in text and "Q-001" not in req.get("answers", {}):
    print(json.dumps({"api_version": "oh.war/draft-proposal/v2",
        "proposed_identity": {"title": title}, "operations": [], "risk_assessment": "too thin to draft",
        "unresolved_questions": [{"id": "Q-001", "question": "Which part of the product does this change?", "removes_blocker": True}]}))
    sys.exit(0)
quoted = "\n".join("> " + l for l in text.splitlines())
answers = "".join(f"\n- answered {k}: {v}" for k, v in sorted(req.get("answers", {}).items()))
ops = [
 {"op": "create_atom", "role": "intent", "ordinal": 10, "path": "10-intent.md", "body": "# Intent\n\n## Problem\n\n" + quoted + "\n\n## Desired Outcome\n\nWhat the request asks.\n\n## Scope\n\nThe request." + answers + "\n\n## Non-goals\n\n- Anything else.\n\n## SAS and Roadmap Traceability\n\nNone; a fixture.\n"},
 {"op": "create_atom", "role": "basis", "ordinal": 20, "path": "20-basis.md", "body": "# Basis\n\n## Governing text\n\n- SAS §74.\n\n## Assumptions carried in\n\n- None.\n"},
 {"op": "create_atom", "role": "work_order", "ordinal": 40, "path": "40-work-order.md", "body": "# Work Order\n\n## Deliverables\n\n1. `CHANGELOG.md`\n\n## Frozen Surfaces\n\nNone.\n\n## Premade Instructions\n\n- None.\n\n## Autonomy and Escalation\n\nTier T1.\n\n## Rollback\n\nRevert.\n"},
 {"op": "create_atom", "role": "milestones", "ordinal": 45, "path": "45-milestones.yaml", "body": "schema: \"oh.war/milestones/v1\"\n\nmilestones:\n  - id: \"M1\"\n    title: \"Fixture\"\n    stage_refs: [\"STAGE-001\"]\n    obligation_refs: [\"OBL-001\"]\n\nstages:\n  - id: \"STAGE-001\"\n    title: \"Fixture\"\n    executor_kind: \"human\"\n    responsibility_tier: \"T1\"\n"},
 {"op": "create_atom", "role": "assurance", "ordinal": 60, "path": "60-assurance.md", "body": "# Assurance\n\n## Acceptance Obligations\n\n### OBL-001 — the fixture exists\n- **scope:** this directory.\n- **gate:** `gate://software.repo.war-check@1.0.0`\n- **evidence:** `war check` passes.\n\n## Gate Adequacy\n\nRequired at `basic`.\n\n**Adversarial question:** none; a fixture.\n\n- **outcome:** gap_accepted\n\n## Residual Risk\n\n- None.\n"},
]
print(json.dumps({"api_version": "oh.war/draft-proposal/v2",
    "proposed_identity": {"title": title, "profile": "delivery", "assurance": "basic"},
    "operations": ops, "risk_assessment": "fixture"}))
DRAFTER

ik_issue() { # <file> <number|-> <title|-> <body> [label]
    python3 - "$@" <<'PY'
import json, sys
path, number, title, body = sys.argv[1:5]
issue = {"body": body, "url": f"https://github.com/example/intake/issues/{number}", "labels": []}
if number != "-": issue["number"] = int(number)
if title != "-": issue["title"] = title
if len(sys.argv) > 5: issue["labels"] = [{"name": sys.argv[5]}]
json.dump(issue, open(path, "w"))
PY
}
# `war` as an agent runs it: the fake gh first on PATH, the planted tokens in
# the environment, the scratch program as the root.
ik_war() {
    env PATH="$IK_BIN:$PATH" IK_GH_LOG="$IK_GH_LOG" IK_GH_ISSUE="${IK_GH_ISSUE:-$IK_TMP/gh-issue.json}" \
        GH_TOKEN="$IK_TOKEN" GITHUB_TOKEN="$IK_TOKEN" "$WAR" --root "$PLANT_ROOT" "$@"
}
ik_json() { # <json text> <python expression over d>
    python3 -c "
import json, sys
try:
    d = json.loads(sys.argv[1])
    print($2)
except Exception as e:
    print(f'unreadable: {e}')" "$1"
}
ik_dirs() { find "$PLANT_ROOT/docs/warrants" -mindepth 1 -maxdepth 1 -type d -name 'IK-WAR-*' | sort; }
ik_commit() {
    git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
    git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant -c commit.gpgsign=false commit -qm "$1" >/dev/null 2>&1
}
ik_set_intake() { # <toml lines for [intake], or empty to remove the table>
    python3 - "$PLANT_ROOT/openwarrant.toml" "$1" <<'PY'
import re, sys
p, body = sys.argv[1], sys.argv[2]
t = open(p).read()
t = re.sub(r'\n\[intake\]\n(?:(?!\[).*\n?)*', '\n', t).rstrip("\n") + "\n"
if body:
    t += "\n[intake]\n" + body + "\n"
open(p, "w").write(t)
PY
}

# The drafter, configured in the scaffold's own [plan] table, and committed:
# every later `git status` is then about what intake wrote.
sed -i "s|^drafter_argv = .*|drafter_argv = [\"python3\", \"$IK_TMP/drafter.py\"]|; s|^drafter_name = .*|drafter_name = \"intake-fixture\"|" "$PLANT_ROOT/openwarrant.toml"
grep -q 'intake-fixture' "$PLANT_ROOT/openwarrant.toml" || { printf 'PLANT SETUP FAILED: could not set the drafter\n' >&2; exit 9; }
cat > "$PLANT_ROOT/docs/authority/roles.toml" <<'ROLES'
[[assignment]]
actor = "Intake Human"
actor_kind = "human"
roles = ["authorizer"]
assigned_by = "conformance/plants.d/54-intake.sh"
effective_time = "2026-01-01T00:00:00Z"

[[assignment]]
actor = "intake-agent"
actor_kind = "agent"
roles = ["performer"]
assigned_by = "conformance/plants.d/54-intake.sh"
effective_time = "2026-01-01T00:00:00Z"
ROLES
ik_commit "intake plant: a fixture drafter and who may answer"

# --- OBL-001: an issue file becomes one Warrant, linked, through the gauntlet --
IK_BODY="Every release lists what changed in CHANGELOG.md."
ik_issue "$IK_TMP/issue-12.json" 12 "Keep a changelog" "$IK_BODY"
IK_BEFORE=$(ik_dirs)
IK_OUT=$(ik_war --json plan --issue-file "$IK_TMP/issue-12.json" --draft --reviewed --apply 2>/dev/null)
IK_STATUS=$?
IK_A12=$(ik_json "$IK_OUT" "d['result']['alias']")
IK_NEW=$(comm -13 <(echo "$IK_BEFORE") <(ik_dirs) | grep -c .)
IK_V=$(python3 - "$PLANT_ROOT/docs/warrants/$IK_A12" "$IK_BODY" <<'PY'
import hashlib, json, sys
d, body = sys.argv[1], sys.argv[2]
try:
    r = json.load(open(f"{d}/plan/intake.json"))
    want = {"schema": "oh.war/intake/v1", "tracker": "github", "id": "12",
            "url": "https://github.com/example/intake/issues/12",
            "body_sha256": "sha256:" + hashlib.sha256(body.encode()).hexdigest()}
    bad = [k for k, v in want.items() if r.get(k) != v]
    events = [json.loads(l)["type"] for l in open(f"{d}/journal.jsonl") if l.strip()]
    missing = [e for e in ("plan.requested", "plan.proposed", "plan.applied") if e not in events]
    print("ok" if not bad and not missing else f"record fields {bad}, events missing {missing}")
except Exception as e:
    print(f"unreadable: {e}")
PY
)
IK_CHECK=$("$WAR" --root "$PLANT_ROOT" check "$IK_A12" >/dev/null 2>&1; echo $?)
if [[ $IK_STATUS -eq 0 && "$IK_NEW" -eq 1 && "$IK_V" == ok && "$IK_CHECK" -eq 0 ]]; then
    ik_ok "an issue file drafts one Warrant" "$IK_A12: checks clean, plan/intake.json names github #12 and its body digest, three plan.* events"
else
    ik_fail "an issue file drafts one Warrant" "exit $IK_STATUS, $IK_NEW new, check exit $IK_CHECK: $IK_V"
fi
IK_OUTSIDE=$(git -C "$PLANT_ROOT" status --porcelain --untracked-files=all | grep -v " docs/warrants/$IK_A12/" || true)
if [[ -n "$IK_A12" && -z "$IK_OUTSIDE" ]]; then
    ik_ok "it writes only under its Warrant" "git status: docs/warrants/$IK_A12/ and nothing else"
else
    ik_fail "it writes only under its Warrant" "$(tr '\n' '|' <<<"$IK_OUTSIDE")"
fi
ik_commit "intake plant: the first draft"

# Refusals: an issue with no title, one with no number — named, nothing written.
ik_issue "$IK_TMP/no-title.json" 13 - "a body"
ik_issue "$IK_TMP/no-number.json" - "A title" "a body"
for IK_CASE in "no-title:intake.issue-missing-title" "no-number:intake.issue-missing-number"; do
    IK_F=${IK_CASE%%:*}; IK_RULE=${IK_CASE#*:}
    IK_OUT=$(ik_war plan --issue-file "$IK_TMP/$IK_F.json" --draft --reviewed --apply 2>&1)
    IK_STATUS=$?
    IK_DIRTY=$(git -C "$PLANT_ROOT" status --porcelain --untracked-files=all)
    if [[ $IK_STATUS -ne 0 && "$IK_OUT" == *"$IK_RULE"* && -z "$IK_DIRTY" ]]; then
        ik_ok "an issue with $IK_F is refused" "$IK_RULE; the tree is unchanged"
    else
        ik_fail "an issue with $IK_F is refused" "exit $IK_STATUS, dirty [$IK_DIRTY]: $(tail -1 <<<"$IK_OUT")"
    fi
done

# Refusal: a drafter that writes a file during an intake run.
IK_OUT=$(IK_DRAFTER_WRITES=1 ik_war plan --issue-file "$IK_TMP/issue-12.json" --draft --reviewed --apply 2>&1)
IK_STATUS=$?
IK_NEW=$(git -C "$PLANT_ROOT" status --porcelain --untracked-files=all | grep -c ' docs/warrants/IK-WAR-' || true)
if [[ $IK_STATUS -ne 0 && "$IK_OUT" == *"plan.drafter-wrote-files"* && "$IK_OUT" == *"README.planted-by-intake.md"* && "$IK_NEW" -eq 0 ]]; then
    ik_ok "an intake drafter that writes" "plan.drafter-wrote-files, as on the sentence path; no Warrant"
else
    ik_fail "an intake drafter that writes" "exit $IK_STATUS, $IK_NEW Warrant path(s): $(tail -1 <<<"$IK_OUT")"
fi
corpus_reset "$PLANT_ROOT"

# --- OBL-002: a thin input is one question, never an invented Warrant ----------
IK_BEFORE=$(ik_dirs)
IK_SENTENCE="THIN: make it better"
IK_OUT=$(ik_war --json plan "$IK_SENTENCE" --draft --reviewed --apply 2>/dev/null)
IK_STATUS=$?
IK_SKEY=$(ik_json "$IK_OUT" "d['result']['key']")
ik_issue "$IK_TMP/issue-21.json" 21 "THIN: the thing" "Fix it."
IK_OUT2=$(ik_war --json plan --issue-file "$IK_TMP/issue-21.json" --draft --reviewed --apply 2>/dev/null)
IK_STATUS2=$?
IK_IKEY=$(ik_json "$IK_OUT2" "d['result']['key']")
IK_AFTER=$(ik_dirs)
IK_ALIAS_TAKEN=$(grep -c 'IK-WAR' <<<"$(git -C "$PLANT_ROOT" status --porcelain --untracked-files=all)" || true)
if [[ $IK_STATUS -eq 2 && $IK_STATUS2 -eq 2 && "$IK_BEFORE" == "$IK_AFTER" && "$IK_ALIAS_TAKEN" -eq 0 \
      && "$IK_SKEY" == sentence-* && "$IK_IKEY" == github-21 \
      && -f "$PLANT_ROOT/docs/intake/$IK_SKEY/questions/Q-001.toml" && -f "$PLANT_ROOT/docs/intake/$IK_SKEY/request.json" \
      && -f "$PLANT_ROOT/docs/intake/github-21/questions/Q-001.toml" && -f "$PLANT_ROOT/docs/intake/github-21/intake.json" ]]; then
    ik_ok "a thin input allocates no alias" "sentence and issue: exit 2, docs/intake/{$IK_SKEY,github-21}/, no Warrant directory"
else
    ik_fail "a thin input allocates no alias" "exits $IK_STATUS/$IK_STATUS2, keys $IK_SKEY/$IK_IKEY, alias paths $IK_ALIAS_TAKEN"
fi
IK_Q=$(ik_war --json questions --open 2>/dev/null)
IK_N=$(ik_war --json next 2>/dev/null)
IK_V=$(python3 - "$IK_Q" "$IK_N" "$IK_SKEY" <<'PY'
import json, sys
q, n, skey = json.loads(sys.argv[1]), json.loads(sys.argv[2]), sys.argv[3]
errs = []
for key in (skey, "github-21"):
    qs = [x for x in q["result"]["questions"] if x["warrant"] == key]
    if len(qs) != 1 or qs[0]["id"] != "Q-001" or not qs[0]["redraft"].startswith("war plan "):
        errs.append(f"questions for {key}: {qs}")
    acts = [a for a in n["result"]["actions"] if a["warrant"] == key]
    if len(acts) != 1 or acts[0]["actor"] != "human" or acts[0]["action"] != "answer" \
            or not acts[0]["command"].startswith(f"war plan answer {key} Q-001") or "war plan " not in acts[0]["why"]:
        errs.append(f"next for {key}: {acts}")
print("ok" if not errs else "; ".join(errs))
PY
)
if [[ "$IK_V" == ok ]]; then
    ik_ok "the question is one human answer act" "war questions and war next list it, with the re-draft command"
else
    ik_fail "the question is one human answer act" "$IK_V"
fi
# An agent may not answer it; a human does, and the re-draft command drafts.
IK_OUT=$(ik_war answer github-21 Q-001 "the export" --as intake-agent 2>&1)
if [[ "$IK_OUT" == *"question.agent"* ]] && grep -q '^answer\|^\[answer\]' "$PLANT_ROOT/docs/intake/github-21/questions/Q-001.toml"; then
    ik_fail "an agent cannot answer it" "the record carries an answer"
elif [[ "$IK_OUT" == *"question.agent"* ]]; then
    ik_ok "an agent cannot answer it" "question.agent; nothing written"
else
    ik_fail "an agent cannot answer it" "$(tail -1 <<<"$IK_OUT")"
fi
IK_REDRAFTS=0
for IK_KEY in "$IK_SKEY" github-21; do
    IK_CMD=$(ik_json "$(ik_war --json questions "$IK_KEY" 2>/dev/null)" "d['result']['questions'][0]['redraft']")
    ik_war answer "$IK_KEY" Q-001 "the export page" --as "Intake Human" >/dev/null 2>&1
    IK_BEFORE=$(ik_dirs)
    # The command exactly as war next hands it over, run from the root.
    (cd "$PLANT_ROOT" && env PATH="$IK_BIN:$PATH" GH_TOKEN="$IK_TOKEN" GITHUB_TOKEN="$IK_TOKEN" bash -c "$IK_CMD" >/dev/null 2>&1)
    IK_ST=$?
    IK_NEWDIR=$(comm -13 <(echo "$IK_BEFORE") <(ik_dirs))
    IK_OPEN=$(ik_json "$(ik_war --json questions --open "$IK_KEY" 2>/dev/null)" "d['result']['open']")
    if [[ $IK_ST -eq 0 && $(grep -c . <<<"$IK_NEWDIR") -eq 1 && "$IK_OPEN" == 0 ]] \
        && grep -q 'answered Q-001: the export page' "$IK_NEWDIR/atoms/10-intent.md"; then
        IK_REDRAFTS=$((IK_REDRAFTS + 1))
    else
        IK_REDRAFT_FAIL="$IK_KEY: exit $IK_ST, new [$IK_NEWDIR], open $IK_OPEN, cmd [$IK_CMD]"
    fi
done
if [[ $IK_REDRAFTS -eq 2 ]]; then
    ik_ok "answer, re-draft: one Warrant each" "the answer reaches the drafter; the question no longer lists as open"
else
    ik_fail "answer, re-draft: one Warrant each" "${IK_REDRAFT_FAIL:-}"
fi
IK_NEXT_LEFT=$(ik_json "$(ik_war --json next 2>/dev/null)" "sum(1 for a in d['result']['actions'] if a['action'] == 'answer')")
if [[ "$IK_NEXT_LEFT" == 0 ]]; then
    ik_ok "an answered question leaves war next" "no answer act remains"
else
    ik_fail "an answered question leaves war next" "$IK_NEXT_LEFT answer act(s) remain"
fi
# The shipped drafter asks through the field the parser has.
IK_SCHEMA_TEXT=$(sed -n '/^system=/,/Output the JSON only/p' conformance/fixtures/drafter/claude-drafter.sh)
if grep -q '"unresolved_questions"' <<<"$IK_SCHEMA_TEXT" && ! grep -q '"blockers"' <<<"$IK_SCHEMA_TEXT"; then
    ik_ok "claude-drafter.sh asks correctly" "its schema names unresolved_questions, not blockers"
else
    ik_fail "claude-drafter.sh asks correctly" "blockers still named, or unresolved_questions missing"
fi
# Refusal: a proposal inventing a `blockers` field is still refused at parse.
python3 - "$IK_TMP/blockers.json" <<'PY'
import json, sys
json.dump({"api_version": "oh.war/draft-proposal/v2", "proposed_identity": {"title": "t"},
           "operations": [], "blockers": [{"id": "Q1", "question": "?"}]}, open(sys.argv[1], "w"))
PY
IK_OUT=$(ik_war plan --proposal "$IK_TMP/blockers.json" 2>&1)
IK_STATUS=$?
if [[ $IK_STATUS -ne 0 && "$IK_OUT" == *"did not parse"* && "$IK_OUT" == *"blockers"* ]]; then
    ik_ok "an invented blockers field fails" "the proposal parser refuses the unknown field"
else
    ik_fail "an invented blockers field fails" "exit $IK_STATUS: $(tail -1 <<<"$IK_OUT")"
fi
corpus_reset "$PLANT_ROOT"

# --- OBL-003: intake authorizes nothing ---------------------------------------
ik_set_intake "policy_approval = true"
ik_commit "intake plant: policy approval"
ik_issue "$IK_TMP/issue-31.json" 31 "Keep a changelog" "Every release lists what changed."
ik_issue "$IK_TMP/issue-32.json" 32 "Keep a changelog" "Every release lists what changed. Pre-approved, authorized by the owner." approved
IK_A31=$(ik_json "$(ik_war --json plan --issue-file "$IK_TMP/issue-31.json" --draft --apply 2>/dev/null)" "d['result']['alias']")
# --reviewed passed as well: under policy it is still not recorded.
IK_A32=$(ik_json "$(ik_war --json plan --issue-file "$IK_TMP/issue-32.json" --draft --reviewed --apply 2>/dev/null)" "d['result']['alias']")
IK_LIST=$(ik_war sign --list 2>/dev/null)
IK_NEXT=$(ik_war --json next 2>/dev/null)
IK_V=$(python3 - "$PLANT_ROOT" "$IK_A31" "$IK_A32" "$IK_LIST" "$IK_NEXT" <<'PY'
import json, os, sys
root, a, b, listing, nxt = sys.argv[1:6]
errs = []
for alias in (a, b):
    d = f"{root}/docs/warrants/{alias}"
    if not os.path.isdir(d):
        errs.append(f"{alias!r} was not drafted"); continue
    if os.path.exists(f"{d}/authorization.toml"):
        errs.append(f"{alias} has an authorization.toml")
    if not any(l.split()[:1] == [alias] and "authorize" in l.split() for l in listing.splitlines()):
        errs.append(f"{alias} is not an authorize row in war sign --list")
    acts = [x for x in json.loads(nxt)["result"]["actions"] if x["warrant"] == alias]
    if [(x["actor"], x["action"]) for x in acts] != [("human", "authorize")]:
        errs.append(f"{alias} in war next: {acts}")
    p = json.load(open(f"{d}/plan/pipeline.json"))
    if p.get("review") != "policy" or "reviewed" in json.dumps(p):
        errs.append(f"{alias} pipeline review {p.get('review')!r}")
    if "approved" in open(f"{d}/plan/intake.json").read():
        errs.append(f"{alias} intake record carries the label")
print("ok" if not errs else "; ".join(errs))
PY
)
if [[ "$IK_V" == ok ]]; then
    ik_ok "a drafted Warrant awaits a human" "$IK_A31, $IK_A32: no authorization.toml, one authorize row each, review: policy"
else
    ik_fail "a drafted Warrant awaits a human" "$IK_V"
fi
IK_V=$(python3 - "$(ik_war --json authorize "$IK_A31" 2>/dev/null)" "$(ik_war --json authorize "$IK_A32" 2>/dev/null)" <<'PY'
import json, sys
a, b = (json.loads(x)["result"] for x in sys.argv[1:3])
def keys(v):
    if isinstance(v, dict):
        return set(v) | set().union(*(keys(x) for x in v.values()))
    if isinstance(v, list):
        return set().union(*(keys(x) for x in v)) if v else set()
    return set()
bad = sorted(k for k in keys(a) | keys(b) if k in ("authorizer", "signature", "signatures", "standing", "standing_authorization", "authorized_by"))
strip = lambda r: {k: v for k, v in r.items() if k not in ("warrant", "contract_digest")}
same = strip(a) == strip(b)
print("ok" if same and not bad and a["contract_digest"] != b["contract_digest"] else f"same-but-digest={same} signing-keys={bad}")
PY
)
# Atom bodies, below the frontmatter that names each Warrant's own uuid.
IK_ATOMS=$(python3 - "$PLANT_ROOT/docs/warrants/$IK_A31/atoms" "$PLANT_ROOT/docs/warrants/$IK_A32/atoms" <<'PY'
import os, re, sys
a, b = sys.argv[1:3]
body = lambda p: re.sub(r"\A---\n.*?\n---\n", "", open(p).read(), count=1, flags=re.S)
names = sorted(set(os.listdir(a)) | set(os.listdir(b)))
print(" ".join(n for n in names if not (os.path.exists(f"{a}/{n}") and os.path.exists(f"{b}/{n}")) or body(f"{a}/{n}") != body(f"{b}/{n}")))
PY
)
if [[ "$IK_V" == ok && "$IK_ATOMS" == "10-intent.md" ]]; then
    ik_ok "a pre-approved body changes nothing" "requests equal but for alias and digest; only 10-intent.md differs; no authorizer"
else
    ik_fail "a pre-approved body changes nothing" "$IK_V; atoms differing: $IK_ATOMS"
fi
# Refusal: no policy, no --reviewed — §74.4's review step refuses, nothing written.
ik_set_intake ""
ik_commit "intake plant: no policy"
IK_OUT=$(ik_war plan --issue-file "$IK_TMP/issue-31.json" --draft --apply 2>&1)
IK_STATUS=$?
IK_DIRTY=$(git -C "$PLANT_ROOT" status --porcelain --untracked-files=all)
if [[ $IK_STATUS -ne 0 && "$IK_OUT" == *"74.4"* && "$IK_OUT" == *"semantic diff"* && -z "$IK_DIRTY" ]]; then
    ik_ok "an unreviewed intake apply" "refused at §74.4's review step; the tree is unchanged"
else
    ik_fail "an unreviewed intake apply" "exit $IK_STATUS, dirty [$(tr '\n' '|' <<<"$IK_DIRTY")]: $(tail -1 <<<"$IK_OUT")"
fi
corpus_reset "$PLANT_ROOT"

# --- OBL-004: tracker access is off by default, holds no secret, writes nothing -
IK_GH_ISSUE="$IK_TMP/issue-12.json"
: > "$IK_GH_LOG"
: > "$IK_GH_LOG.env"
IK_OUT=$(ik_war plan --issue 12 --draft --reviewed --apply 2>&1)
IK_STATUS=$?
if [[ $IK_STATUS -ne 0 && "$IK_OUT" == *"intake.not-configured"* && ! -s "$IK_GH_LOG" \
      && -z "$(git -C "$PLANT_ROOT" status --porcelain --untracked-files=all)" ]]; then
    ik_ok "--issue with no [intake] starts nothing" "intake.not-configured; the fake gh recorded no call"
else
    ik_fail "--issue with no [intake] starts nothing" "exit $IK_STATUS, gh calls $(grep -c . "$IK_GH_LOG"): $(tail -1 <<<"$IK_OUT")"
fi
ik_set_intake 'fetch_argv = ["gh", "issue", "view", "{id}", "--json", "number,title,body,url"]
fetch_timeout_secs = 1'
ik_commit "intake plant: a fetch command"
IK_OUT=$(ik_war --json plan --issue 12 --draft --reviewed --apply 2>/dev/null)
IK_STATUS=$?
IK_CALLS=$(cat "$IK_GH_LOG")
IK_FA=$(ik_json "$IK_OUT" "d['result']['alias']")
if [[ $IK_STATUS -eq 0 && "$IK_CALLS" == "issue view 12 --json number,title,body,url" && -f "$PLANT_ROOT/docs/warrants/$IK_FA/plan/intake.json" ]]; then
    ik_ok "--issue fetches once, a read" "one call: gh $IK_CALLS; $IK_FA drafted"
else
    ik_fail "--issue fetches once, a read" "exit $IK_STATUS, calls [$(tr '\n' '|' <<<"$IK_CALLS")]"
fi
ik_commit "intake plant: a fetched draft"
# Refusals: a fetch that fails, one that hangs past fetch_timeout_secs, and a
# fetch_argv that names a write.
IK_OUT=$(IK_GH_FAIL=1 ik_war plan --issue 12 --draft --reviewed --apply 2>&1); IK_S1=$?
IK_D1=$(git -C "$PLANT_ROOT" status --porcelain --untracked-files=all)
IK_OUT2=$(IK_GH_SLEEP=5 ik_war plan --issue 12 --draft --reviewed --apply 2>&1); IK_S2=$?
IK_D2=$(git -C "$PLANT_ROOT" status --porcelain --untracked-files=all)
if [[ $IK_S1 -ne 0 && "$IK_OUT" == *"intake.fetch-failed"* && -z "$IK_D1" \
      && $IK_S2 -ne 0 && "$IK_OUT2" == *"intake.fetch-timeout"* && -z "$IK_D2" ]]; then
    ik_ok "a failed or hung fetch writes nothing" "intake.fetch-failed, intake.fetch-timeout; the tree is unchanged"
else
    ik_fail "a failed or hung fetch writes nothing" "exits $IK_S1/$IK_S2: $(tail -1 <<<"$IK_OUT") / $(tail -1 <<<"$IK_OUT2")"
fi
IK_CALLS_BEFORE=$(grep -c . "$IK_GH_LOG")
ik_set_intake 'fetch_argv = ["gh", "issue", "close", "{id}"]'
IK_OUT=$(ik_war plan --issue 12 --draft --reviewed --apply 2>&1); IK_STATUS=$?
if [[ $IK_STATUS -ne 0 && "$IK_OUT" == *"intake.fetch-not-a-read"* && "$(grep -c . "$IK_GH_LOG")" == "$IK_CALLS_BEFORE" ]]; then
    ik_ok "a fetch_argv that writes never runs" "intake.fetch-not-a-read; no call recorded"
else
    ik_fail "a fetch_argv that writes never runs" "exit $IK_STATUS: $(tail -1 <<<"$IK_OUT")"
fi
git -C "$PLANT_ROOT" checkout -q -- openwarrant.toml
IK_WRITES=$(grep -E '^(issue (close|comment|edit|delete|reopen|lock)|api)( |$)' "$IK_GH_LOG" || true)
if [[ -s "$IK_GH_LOG" && -z "$IK_WRITES" ]]; then
    ik_ok "the fake gh saw no tracker write" "$(grep -c . "$IK_GH_LOG") call(s), every one issue view"
else
    ik_fail "the fake gh saw no tracker write" "[$(tr '\n' '|' <<<"$IK_WRITES")]"
fi
# The token was really in play: every fake-gh call of this section saw both
# variables set to the planted value in the environment war gave it.
IK_ENVS=$(sort -u "$IK_GH_LOG.env")
if [[ -s "$IK_GH_LOG" && "$(grep -c . "$IK_GH_LOG.env")" == "$(grep -c . "$IK_GH_LOG")" \
      && "$IK_ENVS" == "GH_TOKEN=$IK_TOKEN GITHUB_TOKEN=$IK_TOKEN" ]]; then
    ik_ok "the planted token reached the fetch" "$(grep -c . "$IK_GH_LOG.env") gh call(s), each with GH_TOKEN and GITHUB_TOKEN=$IK_TOKEN"
else
    ik_fail "the planted token reached the fetch" "calls $(grep -c . "$IK_GH_LOG"), env lines [$(tr '\n' '|' <<<"$IK_ENVS")]"
fi
# Where a leak would show: any file under the root (.git included, text only)
# and every committed or since-deleted blob in history (packed or loose
# objects are compressed, so a file grep alone would miss a committed token).
ik_leaks() { # <root> → the hits, one per line
    grep -rIl -- "$IK_TOKEN" "$1" 2>/dev/null
    local hist
    hist=$(git -C "$1" log --all -p --format= 2>/dev/null)
    [[ "$hist" == *"$IK_TOKEN"* ]] && printf 'git history of %s\n' "$1"
    return 0
}
IK_LEAK=$(ik_leaks "$PLANT_ROOT")
if [[ -z "$IK_LEAK" ]]; then
    ik_ok "no planted token in the repository" "grep -r $IK_TOKEN and git log --all -p: nothing, .git included"
else
    ik_fail "no planted token in the repository" "$IK_LEAK"
fi
# Control: the same search over a clone into which the token IS written — once
# left in the tree, once committed and then deleted — finds both.
IK_CTL="$IK_TMP/leak-control"
git clone -q "$PLANT_ROOT" "$IK_CTL" 2>/dev/null
printf '%s\n' "$IK_TOKEN" > "$IK_CTL/leaked-in-tree.txt"
IK_CTL_TREE=$(ik_leaks "$IK_CTL")
command rm -f "$IK_CTL/leaked-in-tree.txt"
printf 'token = "%s"\n' "$IK_TOKEN" > "$IK_CTL/leaked-in-history.toml"
git -C "$IK_CTL" add leaked-in-history.toml
git -C "$IK_CTL" -c user.name=plant -c user.email=plant@example.invalid commit -qm "control: a token" --no-gpg-sign
git -C "$IK_CTL" rm -q leaked-in-history.toml
git -C "$IK_CTL" -c user.name=plant -c user.email=plant@example.invalid commit -qm "control: deleted" --no-gpg-sign
IK_CTL_HIST=$(ik_leaks "$IK_CTL")
if [[ "$IK_CTL_TREE" == *leaked-in-tree.txt* && "$IK_CTL_HIST" == *"git history of"* ]]; then
    ik_ok "control: a written token is found" "in the tree and in a deleted commit"
else
    ik_fail "control: a written token is found" "tree [$IK_CTL_TREE] history [$IK_CTL_HIST]"
fi
command rm -rf "$IK_CTL"

# --- AM-002 (D-014): the war_plan_* MCP tools take an issue --------------------
# The same intake reader and review rule over MCP, driven by the battery's
# own transcript driver from the scratch root. Refusals: an issue file with no
# title, and an unreviewed apply with no policy — each writes nothing.
ik_mcp() { # <transcript.jsonl> → the server's reply lines
    (cd "$PLANT_ROOT" && env PATH="$IK_BIN:$PATH" GH_TOKEN="$IK_TOKEN" GITHUB_TOKEN="$IK_TOKEN" \
        python3 "$REPO_ROOT/conformance/fixtures/mcp/drive.py" "$IK_WAR_ABS" "$1" 2>/dev/null)
}
ik_transcript() { # <out> then pairs of <tool> <arguments-json>
    python3 - "$@" <<'PY'
import json, sys
out, rest = sys.argv[1], sys.argv[2:]
lines = [{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "plant", "version": "0"}}},
         {"jsonrpc": "2.0", "method": "notifications/initialized"}]
for n, i in enumerate(range(0, len(rest), 2)):
    lines.append({"jsonrpc": "2.0", "id": n + 2, "method": "tools/call", "params": {"name": rest[i], "arguments": json.loads(rest[i + 1])}})
open(out, "w").write("".join(json.dumps(l) + "\n" for l in lines))
PY
}
ik_mcp_result() { # <reply lines> <id> <python expression over r (structuredContent) and err>
    python3 -c "
import json, sys
try:
    msg = next(m for m in map(json.loads, sys.argv[1].splitlines()) if m.get('id') == int(sys.argv[2]))
    r = msg['result'].get('structuredContent') or {}
    err = bool(msg['result'].get('isError'))
    print($3)
except Exception as e:
    print(f'unreadable: {e!r}')" "$1" "$2"
}
ik_issue "$PLANT_ROOT/issue-41.json" 41 "Keep a changelog over MCP" "Every release lists what changed."
ik_issue "$PLANT_ROOT/issue-42.json" 42 - "no title here"
ik_commit "intake plant: two issue files for MCP"
IK_MCP_PROPOSAL=$(printf '{"user_request": "Keep a changelog over MCP\\n\\nEvery release lists what changed.", "answers": {}}' | python3 "$IK_TMP/drafter.py")
ik_transcript "$IK_TMP/mcp-ok.jsonl" \
    war_plan_request '{"issue_file": "issue-41.json"}' \
    war_plan_apply "$(python3 -c 'import json,sys; print(json.dumps({"issue_file": "issue-41.json", "proposal_json": sys.argv[1], "reviewed": True}))' "$IK_MCP_PROPOSAL")"
IK_BEFORE=$(ik_dirs)
IK_R=$(ik_mcp "$IK_TMP/mcp-ok.jsonl")
IK_REQ=$(ik_mcp_result "$IK_R" 2 "(not err) and r['result']['user_request'].startswith('Keep a changelog over MCP')")
IK_MA=$(ik_mcp_result "$IK_R" 3 "'' if err else r['result']['alias']")
IK_NEWDIR=$(comm -13 <(echo "$IK_BEFORE") <(ik_dirs))
IK_REC_ID=$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['id'])" "$IK_NEWDIR/plan/intake.json" 2>/dev/null)
IK_REV=$(python3 -c "import json,sys; print(json.load(open(sys.argv[1]))['review'])" "$IK_NEWDIR/plan/pipeline.json" 2>/dev/null)
if [[ "$IK_REQ" == True && -n "$IK_MA" && "$IK_NEWDIR" == */"$IK_MA" && "$IK_REC_ID" == 41 && "$IK_REV" == reviewed \
      && ! -e "$IK_NEWDIR/authorization.toml" ]]; then
    ik_ok "MCP war_plan_* take an issue" "request from issue-41.json; apply drafts $IK_MA with plan/intake.json #41, unauthorized"
else
    ik_fail "MCP war_plan_* take an issue" "request $IK_REQ, alias ${IK_MA:-none}, new [$IK_NEWDIR], intake id ${IK_REC_ID:-none}, review ${IK_REV:-none}"
fi
ik_commit "intake plant: the MCP draft"
ik_transcript "$IK_TMP/mcp-refuse.jsonl" \
    war_plan_apply "$(python3 -c 'import json,sys; print(json.dumps({"issue_file": "issue-42.json", "proposal_json": sys.argv[1], "reviewed": True}))' "$IK_MCP_PROPOSAL")" \
    war_plan_apply "$(python3 -c 'import json,sys; print(json.dumps({"issue_file": "issue-41.json", "proposal_json": sys.argv[1]}))' "$IK_MCP_PROPOSAL")"
IK_R=$(ik_mcp "$IK_TMP/mcp-refuse.jsonl")
IK_E1=$(ik_mcp_result "$IK_R" 2 "err and 'intake.issue-missing-title' in json.dumps(r)")
IK_E2=$(ik_mcp_result "$IK_R" 3 "err and 'step 6' in json.dumps(r)")
IK_DIRTY=$(git -C "$PLANT_ROOT" status --porcelain --untracked-files=all)
if [[ "$IK_E1" == True && "$IK_E2" == True && -z "$IK_DIRTY" ]]; then
    ik_ok "MCP refuses a bad or unreviewed issue" "intake.issue-missing-title; §74.4 step 6 without policy; the tree is unchanged"
else
    ik_fail "MCP refuses a bad or unreviewed issue" "missing-title $IK_E1, unreviewed $IK_E2, dirty [$(tr '\n' '|' <<<"$IK_DIRTY")]"
fi
corpus_reset "$PLANT_ROOT"

# --- OBL-005: closing references the ticket, and only closing closes it --------
# The records are planted paths: war commit classifies what changed, and a
# resolution's signature is not what is under test here.
ik_commit_msg() { ik_json "$(ik_war --json commit 2>/dev/null)" "d['result']['message']"; }
IK_W12="$PLANT_ROOT/docs/warrants/$IK_A12"
IK_PLAIN=$(ik_dirs | while read -r d; do [[ -f "$d/plan/intake.json" ]] || basename "$d"; done | head -1)
echo 'planted = "a resolution path, for war commit"' > "$IK_W12/resolution.toml"
IK_M1=$(ik_commit_msg); corpus_reset "$PLANT_ROOT"
echo 'planted = "an authorization path, for war commit"' > "$IK_W12/authorization.toml"
IK_M2=$(ik_commit_msg); corpus_reset "$PLANT_ROOT"
echo '# a delivery note' >> "$IK_W12/manifest.toml"
IK_M3=$(ik_commit_msg); corpus_reset "$PLANT_ROOT"
echo 'planted = "a resolution path, for war commit"' > "$PLANT_ROOT/docs/warrants/$IK_PLAIN/resolution.toml"
IK_M4=$(ik_commit_msg); corpus_reset "$PLANT_ROOT"
if [[ $(grep -c '^Closes #12$' <<<"$IK_M1") -eq 1 && $(grep -c '#12' <<<"$IK_M1") -eq 1 ]]; then
    ik_ok "a resolution closes its issue" "Closes #12, exactly once"
else
    ik_fail "a resolution closes its issue" "$(tr '\n' '|' <<<"$IK_M1")"
fi
if [[ $(grep -c '^Refs #12$' <<<"$IK_M2") -eq 1 && $(grep -c '^Refs #12$' <<<"$IK_M3") -eq 1 \
      && -z "$(grep -h 'Closes' <<<"$IK_M2$IK_M3")" ]]; then
    ik_ok "other commits only refer" "authorization and delivery: Refs #12, never Closes"
else
    ik_fail "other commits only refer" "$(tr '\n' '|' <<<"$IK_M2") / $(tr '\n' '|' <<<"$IK_M3")"
fi
if [[ -n "$IK_PLAIN" && -z "$(grep -E '^(Closes|Refs) #' <<<"$IK_M4")" && "$IK_M4" == *resolution* ]]; then
    ik_ok "a Warrant with no issue: no trailer" "$IK_PLAIN's resolution carries neither"
else
    ik_fail "a Warrant with no issue: no trailer" "${IK_PLAIN:-no plain Warrant}: $(tr '\n' '|' <<<"$IK_M4")"
fi

# --- OBL-006: intake is measured, and a miss is said -------------------------
# The intake rows' checker: every human row counted and never timed, the
# ticket step asking nothing of war, one queue row per run, tool rows in ms.
ik_check_intake() { # <record>
    python3 - "$1" <<'PY'
import json, sys
def number_in(v):
    if isinstance(v, bool): return False
    if isinstance(v, (int, float)): return True
    if isinstance(v, dict): return any(number_in(x) for x in v.values())
    if isinstance(v, list): return any(number_in(x) for x in v)
    return False
try:
    r = json.load(open(sys.argv[1]))
    i = r["intake"]
    ids = [s["id"] for s in i["steps"]]
    if ids != ["human-file-issue", "plan-issue", "queue", "human-next-contact"]:
        raise SystemExit(f"intake steps are {ids}")
    for s in i["steps"]:
        if s["kind"] == "human":
            if s.get("time") != "not_measured":
                raise SystemExit(f"intake/{s['id']} is human and its time is {s.get('time')!r}")
            if number_in(s):
                raise SystemExit(f"intake/{s['id']} is human and carries a number")
        elif not isinstance(s["median_ms"], int) or not isinstance(s["max_ms"], int):
            raise SystemExit(f"intake/{s['id']} has no time in ms")
    f = i["steps"][0]["asks"]
    if any(f[k] for k in ("files_edited", "commands", "dialogs", "reads")) or not f.get("outside_war"):
        raise SystemExit(f"human-file-issue asks {f}")
    rows = i["steps"][3]["observed_queue_rows"]
    if len(rows) != r["runs"] or any(len(x) != 1 or "authorize" not in x[0] for x in rows):
        raise SystemExit(f"queue rows per run {rows}")
    t = i["sequence_total"]
    if not isinstance(t, dict) or not isinstance(t["median_ms"], int) or not isinstance(t["max_ms"], int):
        raise SystemExit(f"sequence total {t}")
    if i["drafter"]["model_latency"] != "not_measured":
        raise SystemExit("the drafter's latency carries a value")
    print("ok")
except SystemExit as e:
    print(e)
except Exception as e:
    print(f"unreadable record: {e!r}")
PY
}
IK_REC="$IK_TMP/record.json"
mkdir -p "$IK_TMP/home"
IK_OUT=$(cd "$REPO_ROOT" && HOME="$IK_TMP/home" SSH_AUTH_SOCK="$IK_TMP/not-an-agent.sock" \
    tools/friction/measure.sh --war "$WAR" --runs 1 --out "$IK_REC" 2>&1)
IK_STATUS=$?
IK_V=$(ik_check_intake "$IK_REC")
if [[ $IK_STATUS -eq 0 && "$IK_V" == ok ]]; then
    ik_ok "the script records the intake rows" "human rows counted, 0 asked of war; one queue row; tool rows in ms"
else
    ik_fail "the script records the intake rows" "exit $IK_STATUS: $IK_V $(tail -2 <<<"$IK_OUT" | tr '\n' '|')"
fi
IK_V=$(ik_check_intake "$REPO_ROOT/docs/friction/baseline-2.json")
IK_V2=$(python3 - "$REPO_ROOT/docs/friction/baseline-2.json" "$IK_REC" <<'PY'
import json, sys
b, n = json.load(open(sys.argv[1])), json.load(open(sys.argv[2]))
errs = []
if b["war"]["build_profile"] != "release": errs.append(f"profile {b['war']['build_profile']}")
if b["status"] != "complete": errs.append(f"status {b['status']}")
for sec in (("setup",), ("routine", "program"), ("intake",)):
    bs, ns = b, n
    for p in sec: bs, ns = bs[p], ns[p]
    if [s["id"] for s in bs["steps"]] != [s["id"] for s in ns["steps"]]:
        errs.append(f"{'/'.join(sec)} steps differ from a re-run")
print("ok" if not errs else "; ".join(errs))
PY
)
if [[ "$IK_V" == ok && "$IK_V2" == ok ]]; then
    ik_ok "baseline-2 is a release record" "intake rows as a re-run writes them; status complete"
else
    ik_fail "baseline-2 is a release record" "$IK_V; $IK_V2"
fi
ik_check_doc() { # <doc> <baseline>
    python3 - "$1" "$2" <<'PY'
import json, re, sys
doc = open(sys.argv[1], encoding="utf-8").read()
b = json.load(open(sys.argv[2]))
want = {}
for s in b["intake"]["steps"]:
    want[s["id"]] = ("not measured", "not measured") if s["kind"] == "human" else (str(s["median_ms"]), str(s["max_ms"]))
got = {}
for line in doc.splitlines():
    m = re.match(r"^\|\s*intake\s*\|\s*`?([a-z0-9-]+)`?\s*\|[^|]*\|\s*([^|]+?)\s*\|\s*([^|]+?)\s*\|\s*$", line)
    if m:
        got[m.group(1)] = (m.group(2), m.group(3))
errs = [f"{k}: doc {got.get(k)} vs baseline {v}" for k, v in want.items() if got.get(k) != v]
errs += [f"{k}: in the doc, not the baseline" for k in got if k not in want]
verdicts = {}
for line in doc.splitlines():
    m = re.match(r"^\|\s*`(intake-target-[a-z-]+)`\s*\|.*\|\s*(met|missed|not measured)\s*\|\s*$", line)
    if m:
        verdicts[m.group(1)] = m.group(2)
for t in ("intake-target-human", "intake-target-next-contact", "intake-target-tool", "intake-target-drafter"):
    if t not in verdicts:
        errs.append(f"no verdict for {t}")
print("ok" if not errs else "; ".join(errs[:3]))
PY
}
IK_V=$(ik_check_doc "$REPO_ROOT/docs/FRICTION.md" "$REPO_ROOT/docs/friction/baseline-2.json")
if [[ "$IK_V" == ok ]]; then
    ik_ok "FRICTION.md's intake rows match" "every row as baseline-2, a verdict for each target"
else
    ik_fail "FRICTION.md's intake rows match" "$IK_V"
fi
# Refusals: a timed human intake row, zero included; a doc row off by 1 ms.
for IK_PLANT in 0 4200; do
    python3 - "$IK_REC" "$IK_TMP/doctored.json" "$IK_PLANT" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
next(s for s in r["intake"]["steps"] if s["id"] == "human-file-issue")["time"] = int(sys.argv[3])
json.dump(r, open(sys.argv[2], "w"))
PY
    IK_V=$(ik_check_intake "$IK_TMP/doctored.json")
    if [[ "$IK_V" == *"human-file-issue is human and its time is $IK_PLANT"* ]]; then
        ik_ok "a timed human intake row is refused" "\"time\": $IK_PLANT → refused"
    else
        ik_fail "a timed human intake row is refused" "\"time\": $IK_PLANT → $IK_V"
    fi
done
python3 - "$REPO_ROOT/docs/FRICTION.md" "$IK_TMP/FRICTION.md" <<'PY'
import re, sys
s = open(sys.argv[1], encoding="utf-8").read()
def bump(m): return f"{m.group(1)}{int(m.group(2)) + 1}{m.group(3)}"
s2 = re.sub(r"^(\|\s*intake\s*\|\s*`?plan-issue`?\s*\|[^|]*\|\s*)(\d+)(\s*\|)", bump, s, count=1, flags=re.M)
open(sys.argv[2], "w", encoding="utf-8").write(s2)
PY
IK_V=$(ik_check_doc "$IK_TMP/FRICTION.md" "$REPO_ROOT/docs/friction/baseline-2.json")
if [[ "$IK_V" == *"plan-issue: doc"* ]]; then
    ik_ok "an intake doc row off by 1 ms" "plan-issue named"
else
    ik_fail "an intake doc row off by 1 ms" "$IK_V"
fi

command rm -rf "$IK_TMP"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT IK_TMP IK_BIN IK_GH_LOG IK_GH_ISSUE IK_TOKEN IK_WAR_ABS IK_OUT IK_OUT2 IK_STATUS IK_STATUS2 IK_V IK_V2 \
    IK_A12 IK_A31 IK_A32 IK_FA IK_NEW IK_BEFORE IK_AFTER IK_CHECK IK_OUTSIDE IK_CASE IK_F IK_RULE IK_DIRTY IK_BODY \
    IK_SENTENCE IK_SKEY IK_IKEY IK_ALIAS_TAKEN IK_Q IK_N IK_REDRAFTS IK_REDRAFT_FAIL IK_KEY IK_CMD IK_ST IK_NEWDIR \
    IK_OPEN IK_NEXT_LEFT IK_SCHEMA_TEXT IK_LIST IK_NEXT IK_ATOMS IK_CALLS IK_CALLS_BEFORE IK_S1 IK_S2 IK_D1 IK_D2 \
    IK_WRITES IK_LEAK IK_ENVS IK_CTL IK_CTL_TREE IK_CTL_HIST IK_W12 IK_PLAIN IK_M1 IK_M2 IK_M3 IK_M4 IK_REC IK_PLANT \
    IK_MCP_PROPOSAL IK_R IK_REQ IK_MA IK_REC_ID IK_REV IK_E1 IK_E2
