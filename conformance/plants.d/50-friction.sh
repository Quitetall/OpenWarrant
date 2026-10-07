# shellcheck shell=bash
# OW-WAR-0118 — the friction measurement keeps its three honesty rules.
#
# tools/friction/measure.sh runs on scratch directories it makes itself, with
# a key and an ssh-agent it starts itself. Nothing here reads or writes this
# repository's docs, and nothing here reaches the owner's agent: the script
# is run with SSH_AUTH_SOCK pointed at a sentinel path that is not a socket,
# which it must put back exactly as it found it.
#
# Every claim is paired with a refusal: the checker that accepts the real
# record is shown rejecting a doctored one, and the script that measures a
# working `war` is shown reporting `unknown` for a failing one.

echo "== friction measurement (OW-WAR-0118) =="

FR_TMP=$(mktemp -d)
FR_SENTINEL="$FR_TMP/not-an-agent.sock"
mkdir -p "$FR_TMP/home" "$FR_TMP/wrap-authorize" "$FR_TMP/wrap-resolve"
FR_WAR_ABS="$REPO_ROOT/${WAR#./}"

fr_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
fr_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }

# The record checker (OBL-001), as a function so it can be shown refusing.
# Prints "ok" or the first violation.
fr_check_record() {
    python3 - "$1" <<'PY'
import json, sys
SETUP = ["git-init", "init-program", "human-say-who-may-sign", "sas-propose", "human-accept-sas",
         "sas-sign", "check", "compile", "authorize", "human-authorize-first-warrant", "authorize-sign"]
ROUTINE = ["new", "check-alias", "compile", "authorize", "human-confirm-routine-signature",
           "sign", "next", "status", "resolve-dry-run"]
def number_in(v):
    if isinstance(v, bool):
        return False
    if isinstance(v, (int, float)):
        return True
    if isinstance(v, dict):
        return any(number_in(x) for x in v.values())
    if isinstance(v, list):
        return any(number_in(x) for x in v)
    return False
try:
    r = json.load(open(sys.argv[1]))
    for name, want, steps in (("setup", SETUP, r["setup"]["steps"]),
                              ("routine", ROUTINE, r["routine"]["program"]["steps"])):
        got = [s["id"] for s in steps]
        if got != want:
            raise SystemExit(f"{name} steps are {got}, wanted {want}")
        for s in steps:
            if s["kind"] == "human":
                if s.get("time") != "not_measured":
                    raise SystemExit(f"{name}/{s['id']} is human and its time is {s.get('time')!r}")
                if number_in(s):
                    raise SystemExit(f"{name}/{s['id']} is human and carries a number")
                if not any(s["asks"].get(k) for k in ("files_edited", "commands", "dialogs")):
                    raise SystemExit(f"{name}/{s['id']} is human and asks for nothing")
            elif s["kind"] == "tool":
                if any(e not in s["accepted_exits"] for e in s["exit_codes"]):
                    raise SystemExit(f"{name}/{s['id']} exited {s['exit_codes']}")
                if s["exit_code"] != 0 and s["accepted_exits"] != [0, 2]:
                    raise SystemExit(f"{name}/{s['id']} exit {s['exit_code']}")
                if not isinstance(s["median_ms"], int) or not isinstance(s["max_ms"], int):
                    raise SystemExit(f"{name}/{s['id']} has no time in ms")
            else:
                raise SystemExit(f"{name}/{s['id']} has kind {s['kind']!r}")
    if r["status"] != "complete":
        raise SystemExit(f"status is {r['status']}")
    print("ok")
except SystemExit as e:
    print(e)
except Exception as e:
    print(f"unreadable record: {e}")
PY
}

# --- OBL-001 and OBL-003: one real run, from the repository root ----------------
FR_GIT_BEFORE=$(git -C "$REPO_ROOT" status --porcelain)
FR_OUT=$(cd "$REPO_ROOT" && HOME="$FR_TMP/home" SSH_AUTH_SOCK="$FR_SENTINEL" \
    tools/friction/measure.sh --war "$WAR" --runs 2 --out "$FR_TMP/record.json" 2>&1)
FR_STATUS=$?
FR_GIT_AFTER=$(git -C "$REPO_ROOT" status --porcelain)

FR_VERDICT=$(fr_check_record "$FR_TMP/record.json")
if [[ $FR_STATUS -eq 0 && "$FR_VERDICT" == "ok" ]]; then
    fr_ok "every step timed or counted" "tool steps in ms, human steps not_measured"
else
    fr_fail "every step timed or counted" "exit $FR_STATUS: $FR_VERDICT $(tail -2 <<<"$FR_OUT" | tr '\n' '|')"
fi

# The checker refuses a human step with a number, and one with zero.
for FR_PLANT in '"time": 0' '"time": 4200'; do
    python3 - "$FR_TMP/record.json" "$FR_TMP/doctored.json" "$FR_PLANT" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
k, v = sys.argv[3].split(": ")
for s in r["setup"]["steps"]:
    if s["kind"] == "human":
        s["time"] = int(v)
        break
json.dump(r, open(sys.argv[2], "w"))
PY
    FR_V=$(fr_check_record "$FR_TMP/doctored.json")
    if [[ "$FR_V" == *"is human and its time is"* ]]; then
        fr_ok "a timed human step is refused" "$FR_PLANT → refused"
    else
        fr_fail "a timed human step is refused" "$FR_PLANT → $FR_V"
    fi
done

# OBL-003: nothing left behind.
if [[ "$FR_GIT_BEFORE" == "$FR_GIT_AFTER" ]]; then
    fr_ok "the repository tree is unchanged" "git status --porcelain identical"
else
    fr_fail "the repository tree is unchanged" "$(diff <(echo "$FR_GIT_BEFORE") <(echo "$FR_GIT_AFTER") | head -3 | tr '\n' '|')"
fi
FR_AGENT=$(python3 -c 'import json,sys; a=json.load(open(sys.argv[1]))["ssh_agent"]; print(a["pid"] or "", a["ssh_auth_sock_restored"], a["stopped"])' "$FR_TMP/record.json" 2>/dev/null)
read -r FR_PID FR_RESTORED FR_STOPPED <<<"$FR_AGENT"
if [[ -n "$FR_PID" ]] && ! kill -0 "$FR_PID" 2>/dev/null && [[ "$FR_RESTORED" == "True" && "$FR_STOPPED" == "True" ]]; then
    fr_ok "its ssh-agent is gone, sock restored" "agent $FR_PID not running; SSH_AUTH_SOCK back to the sentinel"
else
    fr_fail "its ssh-agent is gone, sock restored" "record says: ${FR_AGENT:-nothing}"
fi
if [[ -z "$(ls -A "$FR_TMP/home")" ]]; then
    fr_ok "HOME holds nothing afterwards" "no ~/.ssh, no config written"
else
    fr_fail "HOME holds nothing afterwards" "$(cd "$FR_TMP/home" && find . | head -5 | tr '\n' ' ')"
fi

# --- OBL-002: a failed step is unknown, and the run fails -----------------------
cat > "$FR_TMP/wrap-authorize/war" <<WRAP
#!/usr/bin/env bash
for a in "\$@"; do [[ "\$a" == authorize ]] && { echo "planted: authorize fails" >&2; exit 1; }; done
exec "$FR_WAR_ABS" "\$@"
WRAP
chmod +x "$FR_TMP/wrap-authorize/war"
FR_OUT=$(cd "$REPO_ROOT" && HOME="$FR_TMP/home" SSH_AUTH_SOCK="$FR_SENTINEL" PATH="$FR_TMP/wrap-authorize:$PATH" \
    tools/friction/measure.sh --runs 1 --out "$FR_TMP/failed.json" 2>&1)
FR_STATUS=$?
FR_V=$(python3 - "$FR_TMP/failed.json" <<'PY'
import json, sys
r = json.load(open(sys.argv[1]))
auth = next(s for s in r["setup"]["steps"] if s["id"] == "authorize")
ok = (auth["median_ms"] == "unknown" and auth["times_ms"] == "unknown" and auth["exit_code"] == 1
      and r["setup"]["tool_total"] == "unknown" and r["status"] == "unknown"
      and "setup/authorize" in r["failed_steps"]
      and "unknown" in r["setup"]["summary"])
# The steps that did answer still carry their times: unknown is per step.
check = next(s for s in r["setup"]["steps"] if s["id"] == "check")
ok = ok and isinstance(check["median_ms"], int)
print("ok" if ok else f"authorize={auth.get('median_ms')} total={r['setup']['tool_total']} status={r['status']}")
PY
)
if [[ $FR_STATUS -eq 1 && "$FR_V" == "ok" ]]; then
    fr_ok "a failed authorize is unknown" "step and setup total unknown, exit 1"
else
    fr_fail "a failed authorize is unknown" "exit $FR_STATUS: $FR_V"
fi

# `resolve --dry-run` answers "not ready" with exit 2 and that is accepted as
# its answer; exit 1 is an error and must still be unknown.
cat > "$FR_TMP/wrap-resolve/war" <<WRAP
#!/usr/bin/env bash
for a in "\$@"; do [[ "\$a" == resolve ]] && { echo "planted: resolve errors" >&2; exit 1; }; done
exec "$FR_WAR_ABS" "\$@"
WRAP
chmod +x "$FR_TMP/wrap-resolve/war"
FR_OUT=$(cd "$REPO_ROOT" && HOME="$FR_TMP/home" SSH_AUTH_SOCK="$FR_SENTINEL" PATH="$FR_TMP/wrap-resolve:$PATH" \
    tools/friction/measure.sh --runs 1 --out "$FR_TMP/failed2.json" 2>&1)
FR_STATUS=$?
FR_V=$(python3 -c '
import json, sys
r = json.load(open(sys.argv[1]))
s = next(s for s in r["routine"]["program"]["steps"] if s["id"] == "resolve-dry-run")
ok = s["median_ms"] == "unknown" and r["routine"]["program"]["sequence_total"] == "unknown" and r["setup"]["tool_total"] != "unknown"
print("ok" if ok else json.dumps(s))' "$FR_TMP/failed2.json" 2>&1)
if [[ $FR_STATUS -eq 1 && "$FR_V" == "ok" ]]; then
    fr_ok "an erroring resolve --dry-run" "exit 1 is unknown, not a not-ready answer"
else
    fr_fail "an erroring resolve --dry-run" "exit $FR_STATUS: $FR_V"
fi

# --- OBL-004: the baseline is the script's own output, and the doc matches it ---
FR_V=$(python3 - "$REPO_ROOT/docs/friction/baseline-1.json" "$FR_TMP/record.json" <<'PY'
import json, sys
b = json.load(open(sys.argv[1])); n = json.load(open(sys.argv[2]))
errs = []
if b.get("schema") != n.get("schema"): errs.append("schema differs")
if b["war"]["build_profile"] != "release": errs.append(f"profile {b['war']['build_profile']}")
if not b["war"]["version"].startswith("war "): errs.append("no war --version")
if len(b["repository"]["commit"]) != 40: errs.append("no commit")
for path in (("setup",), ("routine", "program")):
    bs, ns = b, n
    for p in path: bs, ns = bs[p], ns[p]
    if [s["id"] for s in bs["steps"]] != [s["id"] for s in ns["steps"]]:
        errs.append(f"{'/'.join(path)} steps differ from a re-run")
if b["status"] != "complete": errs.append(f"status {b['status']}")
print("ok" if not errs else "; ".join(errs))
PY
)
if [[ "$FR_V" == "ok" ]]; then
    fr_ok "the baseline is a release record" "same steps, same order as a re-run"
else
    fr_fail "the baseline is a release record" "$FR_V"
fi

# The doc's table against the baseline: every row, both directions.
fr_check_doc() { # <doc> <baseline>
    python3 - "$1" "$2" <<'PY'
import json, re, sys
doc = open(sys.argv[1], encoding="utf-8").read()
b = json.load(open(sys.argv[2]))
want = {}
sections = [("setup", b["setup"]["steps"]), ("routine", b["routine"]["program"]["steps"])]
if b["routine"].get("corpus"):
    sections.append(("corpus", b["routine"]["corpus"]["steps"]))
for sec, steps in sections:
    for s in steps:
        want[(sec, s["id"])] = ("not measured", "not measured") if s["kind"] == "human" \
            else (str(s["median_ms"]), str(s["max_ms"]))
got = {}
for line in doc.splitlines():
    m = re.match(r"^\|\s*(setup|routine|corpus)\s*\|\s*`?([a-z0-9-]+)`?\s*\|[^|]*\|\s*([^|]+?)\s*\|\s*([^|]+?)\s*\|\s*$", line)
    if m:
        got[(m.group(1), m.group(2))] = (m.group(3), m.group(4))
errs = [f"{k}: doc {got.get(k)} vs baseline {v}" for k, v in want.items() if got.get(k) != v]
errs += [f"{k}: in the doc, not the baseline" for k in got if k not in want]
if "neither met nor missed" not in doc:
    errs.append("the doc does not say the setup target is neither met nor missed")
print("ok" if not errs else "; ".join(errs[:3]))
PY
}
FR_V=$(fr_check_doc "$REPO_ROOT/docs/FRICTION.md" "$REPO_ROOT/docs/friction/baseline-1.json")
if [[ "$FR_V" == "ok" ]]; then
    fr_ok "FRICTION.md matches the baseline" "every row, human steps not measured"
else
    fr_fail "FRICTION.md matches the baseline" "$FR_V"
fi
# Refusal: one median moved by a millisecond in a copy of the doc.
python3 - "$REPO_ROOT/docs/FRICTION.md" "$FR_TMP/FRICTION.md" <<'PY'
import re, sys
s = open(sys.argv[1], encoding="utf-8").read()
def bump(m): return f"{m.group(1)}{int(m.group(2)) + 1}{m.group(3)}"
s2 = re.sub(r"^(\|\s*setup\s*\|\s*`?check`?\s*\|[^|]*\|\s*)(\d+)(\s*\|)", bump, s, count=1, flags=re.M)
open(sys.argv[2], "w", encoding="utf-8").write(s2)
PY
FR_V=$(fr_check_doc "$FR_TMP/FRICTION.md" "$REPO_ROOT/docs/friction/baseline-1.json")
if [[ "$FR_V" == *"'setup', 'check'"* ]]; then
    fr_ok "a doc row off by 1 ms is refused" "setup/check named"
else
    fr_fail "a doc row off by 1 ms is refused" "$FR_V"
fi

command rm -rf "$FR_TMP"
unset FR_TMP FR_SENTINEL FR_OUT FR_STATUS FR_V FR_VERDICT FR_PID FR_RESTORED FR_STOPPED FR_AGENT FR_PLANT FR_GIT_BEFORE FR_GIT_AFTER FR_WAR_ABS
