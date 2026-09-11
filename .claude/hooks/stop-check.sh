#!/usr/bin/env bash
# Stop hook: in an OpenWarrant repository, the turn does not end on a corpus
# `war check` reports errors for. The block names the first rules so the agent
# fixes the cause, never the checker (AGENTS.md rule 5).
#
# `stop_hook_active` is true when this hook already blocked once this turn;
# then it lets the stop through, so a red corpus never loops forever.
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
command -v war >/dev/null 2>&1 || exit 0

reason=$(war --json check 2>/dev/null | python3 -c 'import sys, json
try:
    v = json.load(sys.stdin)
except Exception:
    sys.exit(0)
if v.get("exit_code", 0) == 0:
    sys.exit(0)
errs = [d for d in v.get("diagnostics", []) if d.get("severity") in ("error", "unknown")]
first = "; ".join(d.get("rule", "?") + ": " + d.get("message", "")[:120] for d in errs[:3])
print("OpenWarrant: `war check` is %s with %d error(s). Fix the cause, never the checker. First: %s" % (v.get("verdict_line", "not ready"), len(errs), first))')
if [[ -n "$reason" ]]; then
    printf '{"decision":"block","reason":%s}\n' \
        "$(printf '%s' "$reason" | python3 -c 'import sys, json; print(json.dumps(sys.stdin.read()))')"
fi
exit 0
