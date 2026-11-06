# shellcheck shell=bash
# The daily verbs' speed budget at 10,000 Warrants (tools/scale/budget-10k.toml;
# CONTRIBUTING.md, "The speed budget").
#
# The 10,000 run itself is `cargo scale-budget`: minutes, a release
# build, so not a battery step. Here the same tooling runs at a small size on
# the battery's debug binary, in temporary directories only, and each claim
# is paired with the refusal that shows the control can say no:
# - the generator builds a ticket corpus with nothing signed (`--tickets`,
#   `--unsigned`), counted by the tool; and refuses `--unsigned` with a
#   resolved count, writing nothing;
# - the budget file names ready, next and claim at 50 ms, release, for 200
#   directory Warrants and 9,800 tickets;
# - with every limit raised past any run, budget.sh holds and each `claim`
#   run takes the item `war view ready` lists first (`{ready}`), so the ready
#   set shrinks by one per run; with every limit at 1 ms it exits 1 naming
#   each row; and with nothing ready, `{ready}` names nothing and the claim
#   row is unknown, never within (Law 15).

echo "== the speed budget at 10,000 Warrants: the tooling, small =="

SB_TMP=$(mktemp -d)
SB_CORPUS="$SB_TMP/corpus"
SB_WAR="$REPO_ROOT/${WAR#./}"
SB_BUDGET="$REPO_ROOT/tools/scale/budget-10k.toml"
sb_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
sb_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
sb_count() { # <root>: "<directory Warrants> <tickets> <ready rows>" as the tool counts them
    python3 - "$SB_WAR" "$1" <<'PY'
import json, subprocess, sys
war, root = sys.argv[1], sys.argv[2]
def result(*argv):
    out = subprocess.run([war, "--root", root, "--json", *argv], capture_output=True, text=True).stdout
    return json.loads(out)["result"]
try:
    print(len(result("status")["warrants"]), len(result("view", "tickets")["tickets"]),
          len(result("view", "ready")["ready"]))
except Exception as e:
    print(f"unreadable: {e}")
PY
}
SB_GIT_BEFORE=$(git -C "$REPO_ROOT" status --porcelain)

# --- the generator: tickets, nothing signed ----------------------------------------
SB_OUT=$(cd "$REPO_ROOT" && env -u SSH_AUTH_SOCK -u SSH_AGENT_PID \
    tools/scale/synth-corpus.sh --war "$SB_WAR" --n 3 --resolved 0 --tickets 60 --unsigned \
    --out "$SB_CORPUS" 2>&1)
SB_STATUS=$?
SB_COUNTS=$(sb_count "$SB_CORPUS")
SB_SIGNED=$(find "$SB_CORPUS/docs/authority" -name '*.response.toml' 2>/dev/null | wc -l)
if [[ $SB_STATUS -eq 0 && "$SB_COUNTS" == "3 60 "* && "$SB_SIGNED" -eq 0 \
    && ! -e "$SB_CORPUS/docs/authority/allowed_signers" ]]; then
    sb_ok "an unsigned ticket corpus" "3 drafts, 60 tickets by the tool; no key, no response"
else
    sb_fail "an unsigned ticket corpus" "exit $SB_STATUS, counts '$SB_COUNTS', responses $SB_SIGNED: $(tail -2 <<<"$SB_OUT" | tr '\n' '|')"
fi

SB_REFUSED="$SB_TMP/refused"
SB_OUT=$(cd "$REPO_ROOT" && env -u SSH_AUTH_SOCK -u SSH_AGENT_PID \
    tools/scale/synth-corpus.sh --war "$SB_WAR" --n 3 --resolved 1 --unsigned --out "$SB_REFUSED" 2>&1)
SB_STATUS=$?
if [[ $SB_STATUS -eq 2 && ! -e "$SB_REFUSED" ]] && grep -q -- '--unsigned needs --resolved 0' <<<"$SB_OUT"; then
    sb_ok "unsigned and resolved is refused" "exit 2, nothing written"
else
    sb_fail "unsigned and resolved is refused" "exit $SB_STATUS, wrote: $([[ -e "$SB_REFUSED" ]] && echo yes || echo no)"
fi

# --- the budget file ------------------------------------------------------------------
SB_V=$(python3 - "$SB_BUDGET" <<'PY'
import sys, tomllib
b = tomllib.load(open(sys.argv[1], "rb"))
errs = []
if b.get("schema") != "openwarrant.scale/budget/v1": errs.append("schema")
a = b.get("applies_to", {})
if a.get("build_profile") != "release": errs.append(f"profile {a.get('build_profile')}")
if a.get("warrants", 0) + a.get("tickets", 0) != 10000: errs.append(f"{a.get('warrants')} + {a.get('tickets')} Warrants")
rows = {c["id"]: c for c in b.get("command", [])}
if sorted(rows) != ["claim", "next", "ready"]: errs.append(f"rows {sorted(rows)}")
for r in rows.values():
    if r["limit_ms"] != 50: errs.append(f"{r['id']} at {r['limit_ms']} ms")
if rows.get("claim", {}).get("argv") != ["claim", "{ready}"]: errs.append("claim takes no {ready}")
print("ok" if not errs else "; ".join(errs))
PY
)
if [[ "$SB_V" == ok ]]; then
    sb_ok "the 10,000 budget names its rows" "ready, next, claim at 50 ms; release; 10,000 Warrants"
else
    sb_fail "the 10,000 budget names its rows" "$SB_V"
fi

# --- the gate holds when it should, and claim takes the next ready item ---------------
sed -E 's/^limit_ms = [0-9]+/limit_ms = 600000/' "$SB_BUDGET" > "$SB_TMP/wide.toml"
SB_BEFORE=$(sb_count "$SB_CORPUS")
SB_OUT=$(cd "$REPO_ROOT" && tools/scale/budget.sh --war "$SB_WAR" --root "$SB_CORPUS" \
    --budget "$SB_TMP/wide.toml" --runs 2 --out "$SB_TMP/wide.json" 2>&1)
SB_STATUS=$?
SB_AFTER=$(sb_count "$SB_CORPUS")
SB_V=$(python3 - "$SB_TMP/wide.json" "$SB_BEFORE" "$SB_AFTER" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
before, after = int(sys.argv[2].split()[2]), int(sys.argv[3].split()[2])
errs = []
if not r["held"]: errs.append("not held")
for x in r["rows"]:
    if x["verdict"] != "within" or x["exit_codes"] != [0, 0]: errs.append(f"{x['id']} {x['verdict']} {x['exit_codes']}")
if r["corpus"].get("tickets") != 60: errs.append(f"tickets counted {r['corpus'].get('tickets')}")
if r["budget"]["applies"]: errs.append("applies at 60 tickets")
if before - after != 2: errs.append(f"ready went {before} -> {after}, not down by the 2 claim runs")
print("ok" if not errs else "; ".join(errs))
PY
)
if [[ $SB_STATUS -eq 0 && "$SB_V" == ok ]]; then
    sb_ok "the budget holds under wide limits" "every row within; each claim took the next ready item"
else
    sb_fail "the budget holds under wide limits" "exit $SB_STATUS: $SB_V"
fi

sed -E 's/^limit_ms = [0-9]+/limit_ms = 1/' "$SB_BUDGET" > "$SB_TMP/tiny.toml"
SB_OUT=$(cd "$REPO_ROOT" && tools/scale/budget.sh --war "$SB_WAR" --root "$SB_CORPUS" \
    --budget "$SB_TMP/tiny.toml" --runs 1 --cap-factor 0 --out "$SB_TMP/tiny.json" 2>&1)
SB_STATUS=$?
SB_MISSING=$(python3 - "$SB_TMP/tiny.json" "$SB_OUT" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
named = sys.argv[2]
print(" ".join(i for i in ("ready", "next", "claim")
               if i not in r["over"] or f"OVER BUDGET: {i}" not in named))
PY
)
if [[ $SB_STATUS -eq 1 && -z "$SB_MISSING" ]]; then
    sb_ok "a 1 ms budget names every row" "exit 1, ready, next and claim OVER BUDGET by name"
else
    sb_fail "a 1 ms budget names every row" "exit $SB_STATUS, not named: $SB_MISSING"
fi

# --- nothing ready: the claim row is unknown, never within --------------------------
SB_EMPTY="$SB_TMP/empty"
mkdir -p "$SB_EMPTY"
git -C "$SB_EMPTY" init -q
(cd "$SB_EMPTY" && env -u SSH_AUTH_SOCK -u SSH_AGENT_PID "$SB_WAR" init --program "Empty" --namespace EM \
    >/dev/null 2>&1 </dev/null)
SB_OUT=$(cd "$REPO_ROOT" && tools/scale/budget.sh --war "$SB_WAR" --root "$SB_EMPTY" \
    --budget "$SB_TMP/wide.toml" --runs 1 --out "$SB_TMP/empty.json" 2>&1)
SB_STATUS=$?
SB_V=$(python3 - "$SB_TMP/empty.json" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
rows = {x["id"]: x for x in r["rows"]}
c = rows.get("claim", {})
errs = []
if c.get("verdict") != "unknown" or "claim" not in r["unknown"]: errs.append(f"claim reads {c.get('verdict')}")
if r["held"]: errs.append("held with nothing to claim")
for i in ("ready", "next"):
    if rows.get(i, {}).get("verdict") != "within": errs.append(f"{i} reads {rows.get(i, {}).get('verdict')}")
print("ok" if not errs else "; ".join(errs))
PY
)
if [[ $SB_STATUS -eq 1 && "$SB_V" == ok ]] && grep -q 'nothing is ready' <<<"$SB_OUT"; then
    sb_ok "nothing ready: claim is unknown" "exit 1; ready and next within, claim unknown"
else
    sb_fail "nothing ready: claim is unknown" "exit $SB_STATUS: $SB_V"
fi

# --- nothing left behind in this repository ----------------------------------------
SB_GIT_AFTER=$(git -C "$REPO_ROOT" status --porcelain)
if [[ "$SB_GIT_BEFORE" == "$SB_GIT_AFTER" ]]; then
    sb_ok "the repository tree is unchanged" "git status --porcelain identical"
else
    sb_fail "the repository tree is unchanged" "$(diff <(echo "$SB_GIT_BEFORE") <(echo "$SB_GIT_AFTER") | head -3 | tr '\n' '|')"
fi
command rm -rf "$SB_TMP"
unset SB_TMP SB_CORPUS SB_WAR SB_BUDGET SB_GIT_BEFORE SB_GIT_AFTER SB_OUT SB_STATUS SB_COUNTS SB_SIGNED \
    SB_REFUSED SB_V SB_BEFORE SB_AFTER SB_MISSING SB_EMPTY
