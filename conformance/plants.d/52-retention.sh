# shellcheck shell=bash
# OW-WAR-0120 — the synthetic corpus, and the budget gate that can fail.
#
# At a small N only (20-basis R-002): the 1,000-Warrant run is a release-time
# act, recorded in docs/scale/baseline-1000.json. Everything below runs in
# temporary directories; the generator's throwaway key lives in an agent it
# starts and kills itself. Each claim is paired with the refusal that shows
# the control can say no.

echo "== retention and the scale budget (OW-WAR-0120) =="

RT_TMP=$(mktemp -d)
RT_CORPUS="$RT_TMP/corpus"
rt_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
rt_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
RT_GIT_BEFORE=$(git -C "$REPO_ROOT" status --porcelain)

# --- OBL-001: real resolved Warrants, only in the scratch directory -------------
RT_OUT=$(cd "$REPO_ROOT" && env -u SSH_AUTH_SOCK -u SSH_AGENT_PID \
    tools/scale/synth-corpus.sh --war "$WAR" --n 12 --resolved 6 --out "$RT_CORPUS" 2>&1)
RT_STATUS=$?
if [[ $RT_STATUS -eq 0 ]] && "$WAR" --root "$RT_CORPUS" check >/dev/null 2>&1; then
    rt_ok "the synthetic program checks clean" "synth-corpus.sh exit 0, war check exit 0"
else
    rt_fail "the synthetic program checks clean" "synth exit $RT_STATUS: $(tail -2 <<<"$RT_OUT" | tr '\n' '|')"
fi
RT_COUNTS=$("$WAR" --root "$RT_CORPUS" --json status 2>/dev/null | python3 -c '
import collections, json, sys
ws = json.load(sys.stdin)["result"]["warrants"]
p = collections.Counter(w["state"]["phase"] for w in ws)
print(p.get("resolved", 0), len(ws) - p.get("resolved", 0))' 2>/dev/null)
RT_RESOLVED_FILES=$(ls "$RT_CORPUS"/docs/warrants/*/resolution.toml 2>/dev/null | wc -l)
if [[ "$RT_COUNTS" == "6 6" && "$RT_RESOLVED_FILES" -eq 6 ]]; then
    rt_ok "war status counts 6 resolved, 6 not" "and 6 resolution records on disk"
else
    rt_fail "war status counts 6 resolved, 6 not" "status says '$RT_COUNTS', $RT_RESOLVED_FILES resolution.toml"
fi

# Refusal: a non-empty --out, and nothing written into it.
RT_FULL="$RT_TMP/full"
mkdir -p "$RT_FULL"
printf 'keep me\n' > "$RT_FULL/someone-elses.txt"
RT_BEFORE=$(cd "$RT_FULL" && find . -print0 | sort -z | xargs -0 sha256sum 2>/dev/null; ls -A)
RT_OUT=$(cd "$REPO_ROOT" && env -u SSH_AUTH_SOCK -u SSH_AGENT_PID \
    tools/scale/synth-corpus.sh --war "$WAR" --n 3 --resolved 1 --out "$RT_FULL" 2>&1)
RT_STATUS=$?
RT_AFTER=$(cd "$RT_FULL" && find . -print0 | sort -z | xargs -0 sha256sum 2>/dev/null; ls -A)
if [[ $RT_STATUS -eq 2 ]] && grep -q 'is not empty; refusing' <<<"$RT_OUT" && [[ "$RT_BEFORE" == "$RT_AFTER" ]]; then
    rt_ok "a non-empty --out is refused" "exit 2, its contents untouched"
else
    rt_fail "a non-empty --out is refused" "exit $RT_STATUS, changed: $([[ "$RT_BEFORE" == "$RT_AFTER" ]] && echo no || echo yes)"
fi

# --- OBL-002: the gate passes the real budget, and refuses a 1 ms one ------------
RT_OUT=$(cd "$REPO_ROOT" && tools/scale/budget.sh --war "$WAR" --root "$RT_CORPUS" \
    --budget tools/scale/budget.toml --runs 1 --out "$RT_TMP/real.json" 2>&1)
RT_STATUS=$?
if [[ $RT_STATUS -eq 0 ]] && python3 -c 'import json,sys; r=json.load(open(sys.argv[1])); sys.exit(0 if r["held"] and all(x["verdict"]=="within" for x in r["rows"]) else 1)' "$RT_TMP/real.json"; then
    rt_ok "the real budget passes at N=12" "every row within, exit 0"
else
    rt_fail "the real budget passes at N=12" "exit $RT_STATUS: $(grep -E 'OVER|UNKNOWN' <<<"$RT_OUT" | head -3 | tr '\n' '|')"
fi

sed -E 's/^limit_ms = [0-9]+/limit_ms = 1/' "$REPO_ROOT/tools/scale/budget.toml" > "$RT_TMP/tiny.toml"
RT_OUT=$(cd "$REPO_ROOT" && tools/scale/budget.sh --war "$WAR" --root "$RT_CORPUS" \
    --budget "$RT_TMP/tiny.toml" --runs 1 --cap-factor 0 --out "$RT_TMP/tiny.json" 2>&1)
RT_STATUS=$?
RT_MISSING=$(python3 - "$REPO_ROOT/tools/scale/budget.toml" "$RT_TMP/tiny.json" "$RT_OUT" <<'PY'
import json, sys, tomllib
ids = [c["id"] for c in tomllib.load(open(sys.argv[1], "rb"))["command"]]
r = json.load(open(sys.argv[2]))
named = sys.argv[3]
missing = [i for i in ids if i not in r["over"] or f"OVER BUDGET: {i}" not in named]
if any(x["limit_ms"] != 1 for x in r["rows"]):
    missing.append("a limit that is not 1 ms")
print(" ".join(missing))
PY
)
if [[ $RT_STATUS -eq 1 && -z "$RT_MISSING" ]]; then
    rt_ok "a 1 ms budget names every command" "exit 1, each row OVER BUDGET by name"
else
    rt_fail "a 1 ms budget names every command" "exit $RT_STATUS, not named: $RT_MISSING"
fi

# --- OBL-003: a command that fails is unknown, never within ----------------------
RT_MANIFEST="$RT_CORPUS/docs/warrants/SC-WAR-0003/manifest.toml"
printf 'schema = "oh.war/manifest/v1"\nthis is [not toml\n' > "$RT_MANIFEST"
RT_OUT=$(cd "$REPO_ROOT" && tools/scale/budget.sh --war "$WAR" --root "$RT_CORPUS" \
    --budget tools/scale/budget.toml --runs 1 --out "$RT_TMP/broken.json" 2>&1)
RT_STATUS=$?
git -C "$RT_CORPUS" checkout -q -- docs/warrants/SC-WAR-0003/manifest.toml
RT_V=$(python3 - "$RT_TMP/broken.json" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
bad = []
failed = [x for x in r["rows"] if any(e != 0 for e in x["exit_codes"])]
for x in failed:
    if x["verdict"] != "unknown" or x["median_ms"] != "unknown" or x["id"] not in r["unknown"]:
        bad.append(f"{x['id']} exited {x['exit_codes']} but reads {x['verdict']}")
for x in r["rows"]:
    if x["verdict"] == "within" and any(e != 0 for e in x["exit_codes"]):
        bad.append(f"{x['id']} failed and reads within")
if not failed:
    bad.append("no command failed on a corrupt manifest; the plant exercised nothing")
if r["held"]:
    bad.append("held with a failed command")
print("ok " + ",".join(x["id"] for x in failed) if not bad else "; ".join(bad))
PY
)
if [[ $RT_STATUS -eq 1 && "$RT_V" == ok* ]]; then
    rt_ok "a failing command is unknown" "${RT_V#ok } unknown, exit 1"
else
    rt_fail "a failing command is unknown" "exit $RT_STATUS: $RT_V"
fi

# --- nothing left behind in this repository ----------------------------------------
RT_GIT_AFTER=$(git -C "$REPO_ROOT" status --porcelain)
if [[ "$RT_GIT_BEFORE" == "$RT_GIT_AFTER" ]]; then
    rt_ok "the repository tree is unchanged" "git status --porcelain identical"
else
    rt_fail "the repository tree is unchanged" "$(diff <(echo "$RT_GIT_BEFORE") <(echo "$RT_GIT_AFTER") | head -3 | tr '\n' '|')"
fi

# --- OBL-004: the committed 1,000 measurement, and the doc that cites it -----------
RT_BASE="$REPO_ROOT/docs/scale/baseline-1000.json"
RT_V=$(python3 - "$RT_BASE" <<'PY'
import json, sys
try:
    r = json.load(open(sys.argv[1]))
except Exception as e:
    print(f"unreadable: {e}"); sys.exit()
errs = []
if r.get("schema") != "openwarrant.scale/budget-run/v1": errs.append("not a budget.sh record")
if r["war"]["build_profile"] != "release": errs.append(f"profile {r['war']['build_profile']}")
if not r["war"]["version"].startswith("war "): errs.append("no war --version")
if len(r["repository"]["commit"]) != 40: errs.append("no commit")
if not r["machine"].get("cpu_model") or not r["machine"].get("os"): errs.append("no CPU or OS")
c = r["corpus"]
if c.get("counted_by") != "war status --json": errs.append("corpus not counted by war status --json")
if c.get("warrants") != 1000: errs.append(f"{c.get('warrants')} Warrants")
if not isinstance(c.get("resolved"), int) or c["resolved"] < 500: errs.append(f"{c.get('resolved')} resolved")
print("ok" if not errs else "; ".join(errs))
PY
)
if [[ "$RT_V" == "ok" ]]; then
    rt_ok "the 1,000 record is a release record" "1000 Warrants, >=500 resolved, by war status"
else
    rt_fail "the 1,000 record is a release record" "$RT_V"
fi

# RETENTION.md's budget table against the record, and a doctored copy refused.
rt_check_doc() { # <doc> <record>
    python3 - "$1" "$2" <<'PY'
import json, re, sys
doc = open(sys.argv[1], encoding="utf-8").read()
r = json.load(open(sys.argv[2]))
want = {}
for x in r["rows"]:
    med = str(x["median_ms"]) if x["median_ms"] != "unknown" else (f">= {x['at_least_ms']}" if "at_least_ms" in x else "unknown")
    want[x["id"]] = (str(x["limit_ms"]), med, x["verdict"])
got = {}
for line in doc.splitlines():
    m = re.match(r"^\|\s*`([a-z0-9-]+)`\s*\|[^|]*\|\s*([^|]+?)\s*\|\s*([^|]+?)\s*\|\s*\**(within|over|unknown)\**\s*\|\s*$", line)
    if m:
        got[m.group(1)] = (m.group(2).replace(",", ""), m.group(3).replace(",", ""), m.group(4))
errs = [f"{k}: doc {got.get(k)} vs record {v}" for k, v in want.items() if got.get(k) != v]
errs += [f"{k}: in the doc, not the record" for k in got if k not in want]
held = "held" if r["held"] else "did not hold"
if f"The budget {held}" not in doc:
    errs.append(f"the doc does not say 'The budget {held}'")
for n in (str(r["corpus"]["warrants"]), str(r["corpus"]["resolved"])):
    if n not in doc.replace(",", ""):
        errs.append(f"the doc never states {n}")
print("ok" if not errs else "; ".join(errs[:3]))
PY
}
RT_V=$(rt_check_doc "$REPO_ROOT/docs/RETENTION.md" "$RT_BASE")
if [[ "$RT_V" == "ok" ]]; then
    rt_ok "RETENTION.md matches the record" "every budget row, and the verdict"
else
    rt_fail "RETENTION.md matches the record" "$RT_V"
fi
python3 - "$REPO_ROOT/docs/RETENTION.md" "$RT_TMP/RETENTION.md" <<'PY'
import re, sys
s = open(sys.argv[1], encoding="utf-8").read()
s2 = re.sub(r"^(\|\s*`check`\s*\|[^|]*\|[^|]*\|\s*)([^|]+?)(\s*\|)", lambda m: m.group(1) + "1" + m.group(3), s, count=1, flags=re.M)
open(sys.argv[2], "w", encoding="utf-8").write(s2)
PY
RT_V=$(rt_check_doc "$RT_TMP/RETENTION.md" "$RT_BASE")
if [[ "$RT_V" == *"check: doc"* ]]; then
    rt_ok "a doctored budget row is refused" "check named"
else
    rt_fail "a doctored budget row is refused" "$RT_V"
fi

command rm -rf "$RT_TMP"
unset RT_TMP RT_CORPUS RT_OUT RT_STATUS RT_COUNTS RT_RESOLVED_FILES RT_FULL RT_BEFORE RT_AFTER RT_MISSING RT_MANIFEST RT_V RT_BASE RT_GIT_BEFORE RT_GIT_AFTER
