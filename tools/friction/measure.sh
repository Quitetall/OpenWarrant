#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# The friction measurement (OW-WAR-0118; docs/FRICTION.md).
#
# The product spec sets two targets: ordinary setup in five to ten minutes,
# routine Warrant administration in at most 60 seconds. This script measures
# the part of both that a script CAN measure — the tool's own time — and
# counts, without timing, the part it cannot: every step a human takes.
#
#   setup    from an empty directory to an authorized first Warrant, following
#            QUICKSTART.md's governed workflow, repeated --runs times, each in a
#            fresh directory
#   routine  the acts in ROUTINE below (OW-WAR-0118 20-basis U-001), each run
#            --runs times on the program the first setup built
#   corpus   with --corpus <repo>: the read-only routine acts against an
#            existing repository, which must be unchanged afterwards
#
# Three rules the record keeps, and the plants in
# conformance/plants.d/50-friction.sh hold:
#
#   - A human step is COUNTED, never timed: what it asks for (files edited,
#     commands typed, dialogs confirmed, requests read) and `time:
#     "not_measured"`. Never a number, never zero.
#   - A failed step is UNKNOWN, never fast (Law 15): its time and every total
#     that includes it read `"unknown"`, and the script exits 1.
#   - It writes nothing outside its temporary directory and --out, kills the
#     ssh-agent it started, and restores SSH_AUTH_SOCK.
#
# The human's key is simulated by a key generated here, in an ssh-agent
# started here without `-c` (OW-WAR-0118 A-001): it signs the bytes a human's
# confirmed key would sign, so the tool's time is right, and the confirmation
# is what is not measured. No other key is ever used: the script refuses to
# go on unless the agent it started lists exactly its own key.
#
# Usage:
#   tools/friction/measure.sh --out <file> [--war <binary>] [--runs <n>] [--corpus <repo>]
#
# --war defaults to `war` on PATH. Exit 0 when every tool step answered; 1
# when any step failed (the record is still written); 2 on a usage error.

set -euo pipefail

usage() {
    sed -n '/^# Usage:/,/^# --war/p' "$0" | sed 's/^# \{0,1\}//' >&2
    exit 2
}

WAR_ARG="war"
RUNS=5
OUT=""
CORPUS=""
while [[ $# -gt 0 ]]; do
    case "$1" in
        --war) WAR_ARG="${2:?--war needs a binary}"; shift 2 ;;
        --runs) RUNS="${2:?--runs needs a number}"; shift 2 ;;
        --out) OUT="${2:?--out needs a file}"; shift 2 ;;
        --corpus) CORPUS="${2:?--corpus needs a repository}"; shift 2 ;;
        -h|--help) usage ;;
        *) printf 'measure.sh: unknown argument %s\n' "$1" >&2; usage ;;
    esac
done
[[ -n "$OUT" ]] || { printf 'measure.sh: --out is required\n' >&2; usage; }
[[ "$RUNS" =~ ^[1-9][0-9]*$ ]] || { printf 'measure.sh: --runs must be a positive integer\n' >&2; exit 2; }

WAR=$(command -v -- "$WAR_ARG" 2>/dev/null || true)
[[ -n "$WAR" && -x "$WAR" ]] || { printf 'measure.sh: no war binary at %s\n' "$WAR_ARG" >&2; exit 2; }
WAR=$(cd "$(dirname "$WAR")" && printf '%s/%s' "$PWD" "$(basename "$WAR")")
if [[ -n "$CORPUS" ]]; then
    CORPUS=$(cd "$CORPUS" && pwd) || { printf 'measure.sh: no corpus at %s\n' "$CORPUS" >&2; exit 2; }
fi
OUT_DIR=$(dirname -- "$OUT")
[[ -d "$OUT_DIR" ]] || { printf 'measure.sh: %s does not exist\n' "$OUT_DIR" >&2; exit 2; }
OUT="$(cd "$OUT_DIR" && pwd)/$(basename -- "$OUT")"

# Every scratch program is opened once and gone; none belongs in the list of
# repositories `war` remembers for whoever runs this (OW-WAR-0115).
export OPENWARRANT_NO_PROJECTS=1

SCRIPT_REPO=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
TMP=$(mktemp -d)
LOGS="$TMP/logs"
mkdir -p "$LOGS"
STEPS="$TMP/steps.tsv"
: > "$STEPS"

# SSH_AUTH_SOCK as it was, set or unset, so it can be put back exactly.
if [[ -n "${SSH_AUTH_SOCK+x}" ]]; then OLD_SOCK_SET=1; OLD_SOCK="$SSH_AUTH_SOCK"; else OLD_SOCK_SET=0; OLD_SOCK=""; fi
if [[ -n "${SSH_AGENT_PID+x}" ]]; then OLD_PID_SET=1; OLD_PID="$SSH_AGENT_PID"; else OLD_PID_SET=0; OLD_PID=""; fi
AGENT_PID=""
AGENT_STOPPED=0

restore_agent() {
    [[ "$AGENT_STOPPED" == 0 ]] || return 0
    AGENT_STOPPED=1
    if [[ -n "$AGENT_PID" ]]; then
        kill "$AGENT_PID" 2>/dev/null || true
        # Wait for it to go, so a caller that checks right after sees it gone.
        local i
        for i in 1 2 3 4 5 6 7 8 9 10; do
            kill -0 "$AGENT_PID" 2>/dev/null || break
            sleep 0.1
        done
    fi
    if [[ "$OLD_SOCK_SET" == 1 ]]; then export SSH_AUTH_SOCK="$OLD_SOCK"; else unset SSH_AUTH_SOCK; fi
    if [[ "$OLD_PID_SET" == 1 ]]; then export SSH_AGENT_PID="$OLD_PID"; else unset SSH_AGENT_PID; fi
}
cleanup() {
    restore_agent
    rm -rf "$TMP"
}
trap cleanup EXIT

now_ms() { local n; n=$(date +%s%N); printf '%s' $((n / 1000000)); }

# The load at start, before anything here adds to it.
LOAD_START=$(cut -d' ' -f1-3 /proc/loadavg 2>/dev/null || echo "unknown")

# --- the simulated human key -------------------------------------------------

KEYDIR="$TMP/key"
mkdir -p "$KEYDIR"
ssh-keygen -q -t ed25519 -N "" -C friction -f "$KEYDIR/id_friction"
KEY_PUB=$(cut -d' ' -f1,2 "$KEYDIR/id_friction.pub")
KEY_FP=$(ssh-keygen -lf "$KEYDIR/id_friction.pub" | awk '{print $2}')
unset SSH_AUTH_SOCK SSH_AGENT_PID
eval "$(ssh-agent -s -a "$TMP/agent.sock")" >/dev/null
AGENT_PID="$SSH_AGENT_PID"
ssh-add -q "$KEYDIR/id_friction" 2>/dev/null
# The agent must hold exactly this key and nothing else: a measurement that
# could reach anyone's real key is not one this script will take.
AGENT_KEYS=$(ssh-add -l 2>/dev/null || true)
if [[ "$(printf '%s\n' "$AGENT_KEYS" | grep -c .)" != 1 ]] || ! grep -qF -- "$KEY_FP" <<<"$AGENT_KEYS"; then
    printf 'measure.sh: the throwaway agent holds more or other than its own key; refusing\n' >&2
    exit 2
fi

write_authority() { # <program root>
    printf 'friction namespaces="oh.war/response,oh.war/dsse" %s\n' "$KEY_PUB" > "$1/docs/authority/allowed_signers"
    cat > "$1/docs/authority/roles.toml" <<'ROLES'
[[assignment]]
actor = "Friction Signer"
actor_kind = "human"
roles = ["authorizer", "resolver", "risk_acceptor", "judge"]
assigned_by = "tools/friction/measure.sh"
effective_time = "2026-01-01T00:00:00Z"
note = "A throwaway identity that exists only while the friction measurement runs."
ssh_principal = "friction"
ROLES
}

# --- one timed step ------------------------------------------------------------

# step <section> <id> <run> <accept> <cwd> -- <argv...>
#
# <accept> is the exit codes that are the command's ANSWER, comma-separated.
# For every act but one it is `0`. `war resolve --dry-run` answers "not ready"
# with exit 2 by design (its verdict `not_ready`, §56.1); exit 1 is an error
# and stays one. Any exit outside <accept> makes the step unknown.
STEP_FAILED=0
step() {
    local section="$1" id="$2" run="$3" accept="$4" cwd="$5"
    shift 6
    local log="$LOGS/$section.$id.$run.log" t0 t1 rc=0 argv
    argv=$(printf '%s\x1f' "$@")
    t0=$(now_ms)
    (cd "$cwd" && "$@") </dev/null >"$log" 2>&1 || rc=$?
    t1=$(now_ms)
    if [[ ",$accept," != *",$rc,"* ]]; then
        STEP_FAILED=1
        printf 'measure.sh: %s %s (run %s) exited %s: %s\n' "$section" "$id" "$run" "$rc" "$(tail -1 "$log")" >&2
    fi
    printf '%s\t%s\t%s\t%s\t%s\t%s\t%s\n' "$section" "$id" "$run" "$rc" "$((t1 - t0))" "$accept" "${argv%$'\x1f'}" >> "$STEPS"
}

# `war` as the user types it: the name, resolved to the binary under test.
BIN="$TMP/bin"
mkdir -p "$BIN"
ln -s "$WAR" "$BIN/war"
export PATH="$BIN:$PATH"

GIT_ID=(-c user.email=friction@invalid -c user.name=friction -c commit.gpgsign=false)

# --- setup: QUICKSTART's governed workflow, --runs times -------------------------

NS="FR"
FIRST="$NS-WAR-0001"
for run in $(seq 1 "$RUNS"); do
    P="$TMP/setup-$run"
    mkdir -p "$P"
    step setup git-init "$run" 0 "$P" -- git init -q .
    step setup init-program "$run" 0 "$P" -- war init --program "Friction" --namespace "$NS"
    # HUMAN (simulated, not timed): say who may sign.
    if [[ -d "$P/docs/authority" ]]; then write_authority "$P"; fi
    step setup sas-propose "$run" 0 "$P" -- war sas propose 0.1.0
    step setup sas-sign "$run" 0 "$P" -- war sign 0.1.0 --ssh-sign
    step setup check "$run" 0 "$P" -- war check
    step setup compile "$run" 0 "$P" -- war compile
    step setup authorize "$run" 0 "$P" -- war authorize "$FIRST"
    step setup authorize-sign "$run" 0 "$P" -- war sign "$FIRST" --ssh-sign
done

# --- routine acts: OW-WAR-0118 20-basis U-001, one table -------------------------
#
# id | argv after `war` ({A} is the Warrant this run created) | accepted exits
ROUTINE=(
    "new|new|Routine act {K}|0"
    "check-alias|check|{A}|0"
    "compile|compile|0"
    "authorize|authorize|{A}|0"
    "sign|sign|{A}|--ssh-sign|0"
    "next|next|0"
    "status|status|0"
    "resolve-dry-run|resolve|--dry-run|{A}|0,2"
)

P="$TMP/setup-1"
git -C "$P" add -A >/dev/null 2>&1 || true
git -C "$P" "${GIT_ID[@]}" commit -qm "setup" >/dev/null 2>&1 || true
for run in $(seq 1 "$RUNS"); do
    alias=$(printf '%s-WAR-%04d' "$NS" $((run + 1)))
    for row in "${ROUTINE[@]}"; do
        IFS='|' read -r -a f <<<"$row"
        id="${f[0]}"; accept="${f[-1]}"
        args=()
        for a in "${f[@]:1:${#f[@]}-2}"; do
            a="${a//\{A\}/$alias}"; a="${a//\{K\}/$run}"
            args+=("$a")
        done
        step program "$id" "$run" "$accept" "$P" -- war "${args[@]}"
    done
done

# --- the same acts, read-only, against an existing repository ------------------
CORPUS_ALIAS=""
CORPUS_CHANGED=0
if [[ -n "$CORPUS" ]]; then
    CORPUS_BEFORE=$(git -C "$CORPUS" status --porcelain 2>/dev/null || echo "not-a-git-repository")
    CORPUS_ALIAS=$(war --root "$CORPUS" --json status 2>/dev/null | python3 -c '
import json, sys
try:
    ws = json.load(sys.stdin)["result"]["warrants"]
    print(sorted(w["alias"] for w in ws)[-1])
except Exception:
    print("")
' || true)
    CORPUS_ALIAS="${CORPUS_ALIAS:-NO-WARRANT}"
    CROWS=(
        "check-alias|check|{A}|0"
        "next|next|0"
        "status|status|0"
        "resolve-dry-run|resolve|--dry-run|{A}|0,2"
    )
    for run in $(seq 1 "$RUNS"); do
        for row in "${CROWS[@]}"; do
            IFS='|' read -r -a f <<<"$row"
            id="${f[0]}"; accept="${f[-1]}"
            args=()
            for a in "${f[@]:1:${#f[@]}-2}"; do args+=("${a//\{A\}/$CORPUS_ALIAS}"); done
            step corpus "$id" "$run" "$accept" "$CORPUS" -- war --root "$CORPUS" "${args[@]}"
        done
    done
    CORPUS_AFTER=$(git -C "$CORPUS" status --porcelain 2>/dev/null || echo "not-a-git-repository")
    if [[ "$CORPUS_BEFORE" != "$CORPUS_AFTER" ]]; then
        CORPUS_CHANGED=1
        STEP_FAILED=1
        printf 'measure.sh: the read-only acts changed %s; its rows are unknown\n' "$CORPUS" >&2
    fi
fi

LOAD_END=$(cut -d' ' -f1-3 /proc/loadavg 2>/dev/null || echo "unknown")

# --- machine and binary facts ---------------------------------------------------

WAR_VERSION=$("$WAR" --version 2>/dev/null | head -1 || true)
WAR_SHA=$(sha256sum "$WAR" | cut -d' ' -f1)
case "$WAR" in
    */target/release/*|*/release/war) PROFILE=release; PROFILE_BY="binary path (target/release)" ;;
    */target/debug/*|*/debug/war) PROFILE=debug; PROFILE_BY="binary path (target/debug)" ;;
    *)
        if readelf -S "$WAR" 2>/dev/null | grep -q '\.debug_info'; then
            PROFILE=debug; PROFILE_BY="ELF .debug_info section present"
        elif readelf -h "$WAR" >/dev/null 2>&1; then
            PROFILE=unknown; PROFILE_BY="an ELF binary outside target/, without .debug_info: release-like, not established"
        else
            PROFILE=unknown; PROFILE_BY="not an ELF binary (a wrapper script?)"
        fi
        ;;
esac
REPO_COMMIT=$(git -C "$SCRIPT_REPO" rev-parse HEAD 2>/dev/null || echo unknown)
if [[ -n "$(git -C "$SCRIPT_REPO" status --porcelain --untracked-files=no 2>/dev/null)" ]]; then REPO_DIRTY=true; else REPO_DIRTY=false; fi
CPU=$(awk -F': ' '/^model name/{print $2; exit}' /proc/cpuinfo 2>/dev/null || echo unknown)

# The agent goes before the record is written, so the record can say it went.
restore_agent
if [[ -n "$AGENT_PID" ]] && kill -0 "$AGENT_PID" 2>/dev/null; then AGENT_GONE=false; else AGENT_GONE=true; fi
if [[ "$OLD_SOCK_SET" == 1 && "${SSH_AUTH_SOCK:-}" == "$OLD_SOCK" ]] || [[ "$OLD_SOCK_SET" == 0 && -z "${SSH_AUTH_SOCK+x}" ]]; then SOCK_RESTORED=true; else SOCK_RESTORED=false; fi

export STEPS RUNS WAR WAR_VERSION WAR_SHA PROFILE PROFILE_BY REPO_COMMIT REPO_DIRTY CPU LOAD_START LOAD_END \
    CORPUS CORPUS_ALIAS CORPUS_CHANGED AGENT_PID AGENT_GONE SOCK_RESTORED STEP_FAILED OUT

python3 - <<'PY'
import datetime, json, os, platform, statistics

E = os.environ
runs = int(E["RUNS"])

rows = {}
order = {"setup": [], "program": [], "corpus": []}
for line in open(E["STEPS"], encoding="utf-8"):
    section, sid, run, rc, ms, accept, argv = line.rstrip("\n").split("\t")
    key = (section, sid)
    if key not in rows:
        rows[key] = {"argv": argv.split("\x1f"), "accept": [int(a) for a in accept.split(",")], "exits": [], "ms": []}
        order[section].append(sid)
    rows[key]["exits"].append(int(rc))
    rows[key]["ms"].append(int(ms))

UNKNOWN = "unknown"

def tool_step(section, sid):
    r = rows[(section, sid)]
    ok = all(e in r["accept"] for e in r["exits"])
    bad = [e for e in r["exits"] if e not in r["accept"]]
    step = {
        "id": sid,
        "kind": "tool",
        "argv": r["argv"],
        "accepted_exits": r["accept"],
        "exit_codes": r["exits"],
        "exit_code": bad[0] if bad else r["exits"][0],
    }
    if ok:
        step["times_ms"] = r["ms"]
        step["median_ms"] = statistics.median_low(r["ms"])
        step["max_ms"] = max(r["ms"])
    else:
        # Law 15: a step that did not answer has no time. Not its wall
        # clock, not zero: unknown.
        step["times_ms"] = UNKNOWN
        step["median_ms"] = UNKNOWN
        step["max_ms"] = UNKNOWN
    if 2 in r["accept"] and ok:
        step["answer"] = ["ready" if e == 0 else "not_ready" for e in r["exits"]]
    return step

def total(section, ids):
    """Per-run sum over ids, then median and max. Unknown if any step is."""
    if any(not all(e in rows[(section, i)]["accept"] for e in rows[(section, i)]["exits"]) for i in ids):
        return UNKNOWN
    per_run = [sum(rows[(section, i)]["ms"][k] for i in ids) for k in range(runs)]
    return {"median_ms": statistics.median_low(per_run), "max_ms": max(per_run), "per_run_ms": per_run}

H_WHO = {
    "id": "human-say-who-may-sign",
    "kind": "human",
    "what": "Write who may sign and load the key (QUICKSTART step 2)",
    "asks": {
        "files_edited": ["docs/authority/roles.toml", "docs/authority/allowed_signers"],
        "commands": [
            "cp docs/authority/roles.toml.example docs/authority/roles.toml",
            "cp docs/authority/allowed_signers.example docs/authority/allowed_signers",
            "ssh-add -c ~/.ssh/id_ed25519",
        ],
        "dialogs": [],
        "reads": ["the comments in roles.toml.example and allowed_signers.example"],
    },
    "simulated_by": "a key generated by this script and both files written from it",
    "time": "not_measured",
}
H_SAS = {
    "id": "human-accept-sas",
    "kind": "human",
    "what": "Read what accepting the SAS means and confirm the signature (QUICKSTART step 3)",
    "asks": {
        "files_edited": [],
        "commands": ["war sign 0.1.0 --ssh-sign"],
        "dialogs": ["the ssh-agent confirmation for the SAS acceptance"],
        "reads": ["the acceptance request `war sign` shows", "docs/sas/<name>_SAS.md"],
    },
    "simulated_by": "an ssh-agent without -c, which signs without asking",
    "time": "not_measured",
}
H_AUTH = {
    "id": "human-authorize-first-warrant",
    "kind": "human",
    "what": "Read the authorization request and confirm the signature (QUICKSTART step 4)",
    "asks": {
        "files_edited": [],
        "commands": ["war sign <alias> --ssh-sign"],
        "dialogs": ["the ssh-agent confirmation for the authorization"],
        "reads": ["the authorization request `war sign` shows: intent, obligations, residual risks"],
    },
    "simulated_by": "an ssh-agent without -c, which signs without asking",
    "time": "not_measured",
}
H_ROUTINE_SIGN = {
    "id": "human-confirm-routine-signature",
    "kind": "human",
    "what": "Confirm the routine act's signature",
    "asks": {
        "files_edited": [],
        "commands": ["war sign <alias> --ssh-sign"],
        "dialogs": ["the ssh-agent confirmation"],
        "reads": ["the request `war sign` shows"],
    },
    "simulated_by": "an ssh-agent without -c, which signs without asking",
    "time": "not_measured",
}

setup_steps = []
for sid in order["setup"]:
    setup_steps.append(tool_step("setup", sid))
    if sid == "init-program":
        setup_steps.append(H_WHO)
    elif sid == "sas-propose":
        setup_steps.append(H_SAS)
    elif sid == "authorize":
        setup_steps.append(H_AUTH)

program_steps = []
for sid in order["program"]:
    program_steps.append(tool_step("program", sid))
    if sid == "authorize":
        program_steps.append(H_ROUTINE_SIGN)

def humans(steps):
    return sum(1 for s in steps if s["kind"] == "human")

setup_total = total("setup", order["setup"])
program_total = total("program", order["program"])

def summary(tot, steps, what):
    n = humans(steps)
    t = "unknown (a step failed)" if tot == UNKNOWN else f"{tot['median_ms']} ms median of the tool's time"
    return f"{what}: {t}, plus {n} human step(s) counted and not measured"

record = {
    "schema": "openwarrant.friction/record/v1",
    "recorded_at": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "procedure": "tools/friction/measure.sh (OW-WAR-0118); docs/FRICTION.md",
    "runs": runs,
    "war": {
        "path": E["WAR"],
        "version": E["WAR_VERSION"],
        "build_profile": E["PROFILE"],
        "build_profile_determined_by": E["PROFILE_BY"],
        "sha256": E["WAR_SHA"],
    },
    "repository": {"commit": E["REPO_COMMIT"], "tracked_changes": E["REPO_DIRTY"] == "true"},
    "machine": {
        "os": platform.system(),
        "kernel": platform.release(),
        "arch": platform.machine(),
        "cpu_model": E["CPU"],
        "nproc": os.cpu_count(),
        "load_average_start": E["LOAD_START"].split(),
        "load_average_end": E["LOAD_END"].split(),
    },
    "timing": "wall clock of each command, milliseconds, from `date +%s%N` around it (includes process start)",
    "setup": {
        "from": "an empty directory",
        "to": "an authorized first Warrant (QUICKSTART.md, Governed legacy workflow, steps 1-4)",
        "steps": setup_steps,
        "tool_total": setup_total,
        "human_steps": humans(setup_steps),
        "human_time": "not_measured",
        "summary": summary(setup_total, setup_steps, "setup"),
    },
    "routine": {
        "acts": "OW-WAR-0118 20-basis U-001",
        "program": {
            "where": "the program the first setup run built; each run creates its own Warrant",
            "steps": program_steps,
            "sequence_total": program_total,
            "human_steps": humans(program_steps),
            "human_time": "not_measured",
            "summary": summary(program_total, program_steps, "one routine sequence"),
        },
        "corpus": None,
    },
    "not_measured": [
        "every human step: its time is not_measured, never zero (docs/FRICTION.md gives the manual protocol)",
        "any OS but the one named under machine",
        "authoring a Warrant's atoms: the routine Warrants keep `war new`'s TODO atoms",
    ],
    "ssh_agent": {
        "pid": int(E["AGENT_PID"]) if E["AGENT_PID"] else None,
        "stopped": E["AGENT_GONE"] == "true",
        "ssh_auth_sock_restored": E["SOCK_RESTORED"] == "true",
    },
}

if E["CORPUS"]:
    csteps = [tool_step("corpus", sid) for sid in order["corpus"]]
    changed = E["CORPUS_CHANGED"] == "1"
    if changed:
        for s in csteps:
            s["times_ms"] = s["median_ms"] = s["max_ms"] = UNKNOWN
    record["routine"]["corpus"] = {
        "root": E["CORPUS"],
        "alias": E["CORPUS_ALIAS"],
        "read_only": not changed,
        "steps": csteps,
    }

failed = []
for sec, steps in (("setup", setup_steps), ("program", program_steps),
                   ("corpus", (record["routine"]["corpus"] or {}).get("steps", []))):
    for s in steps:
        if s["kind"] == "tool" and s["median_ms"] == UNKNOWN:
            failed.append(f"{sec}/{s['id']}")
record["failed_steps"] = failed
record["status"] = "unknown" if (failed or E["STEP_FAILED"] == "1") else "complete"

with open(E["OUT"], "w", encoding="utf-8") as f:
    json.dump(record, f, indent=2, ensure_ascii=False)
    f.write("\n")
print(record["setup"]["summary"])
print(record["routine"]["program"]["summary"])
if failed:
    print("unknown: " + ", ".join(failed))
PY

[[ "$STEP_FAILED" == 0 ]] || exit 1
exit 0
