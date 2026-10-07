#!/usr/bin/env bash
# Stop diagnostic: report corpus problems without confusing an agent/harness
# stop with completed work. A global legacy corpus error cannot require another
# approval loop or block a read-only/prototype turn. Explicit action gates and
# the immutable-history edit guard remain enforced at their actual boundaries.
set -uo pipefail

input=$(cat)
read -r active cwd < <(printf '%s' "$input" | python3 -c 'import sys, json
try:
    v = json.load(sys.stdin)
    print(str(v.get("stop_hook_active", False)).lower(), v.get("cwd", ""))
except Exception:
    print("false", "")')
[[ "$active" == "true" ]] && exit 0
if [[ -n "$cwd" ]] && ! cd "$cwd" 2>/dev/null; then
    printf 'openwarrant stop-check: cannot enter %s; the check did not run\n' "$cwd" >&2
    exit 0
fi
[[ -f openwarrant.toml ]] || exit 0
if [[ -x ./target/debug/war ]]; then
    war_cmd=./target/debug/war
elif command -v war >/dev/null 2>&1; then
    war_cmd=war
else
    printf 'OpenWarrant: war unavailable; corpus check UNKNOWN.\n' >&2
    exit 0
fi

reason=$("$war_cmd" --json check 2>/dev/null | python3 -c 'import sys, json
try:
    v = json.load(sys.stdin)
except Exception:
    print("OpenWarrant: invalid check output; corpus check UNKNOWN.")
    sys.exit(0)
if not isinstance(v, dict) or not isinstance(v.get("exit_code"), int):
    print("OpenWarrant: missing check result; corpus check UNKNOWN.")
    sys.exit(0)
if v["exit_code"] == 0:
    sys.exit(0)
errs = [d for d in v.get("diagnostics", []) if d.get("severity") in ("error", "unknown")]
first = "; ".join(d.get("rule", "?") + ": " + d.get("message", "")[:120] for d in errs[:3])
print("OpenWarrant: `war check` is %s with %d error(s); this does not hold the turn. If you change a record, fix the cause rather than the checker. First: %s" % (v.get("verdict_line", "not ready"), len(errs), first))')
if [[ -n "$reason" ]]; then
    printf '%s\n' "$reason" >&2
fi

# OW-WAR-0148 M14: the end-of-session summary. Under a repo preset, what
# waits on a person (`war next` lists it) is shown to the person when the
# agent ends its turn: a message, not a block. Nothing without a preset.
if grep -q '^\[preset\]' openwarrant.toml 2>/dev/null; then
    waiting=$("$war_cmd" --json next 2>/dev/null | python3 -c 'import sys, json
try:
    v = json.load(sys.stdin)
except Exception:
    sys.exit(0)
acts = [a for a in ((v.get("result") or {}).get("actions") or []) if a.get("actor") == "human"]
if acts:
    first = "; ".join("%s %s" % (a.get("action", "?"), a.get("warrant", "?")) for a in acts[:3])
    more = "" if len(acts) <= 3 else " and %d more" % (len(acts) - 3)
    print(json.dumps({"systemMessage": "OpenWarrant: %d act(s) wait on a person (war next lists them): %s%s" % (len(acts), first, more)}))')
    if [[ -n "$waiting" ]]; then
        printf '%s\n' "$waiting"
    fi
fi
exit 0
