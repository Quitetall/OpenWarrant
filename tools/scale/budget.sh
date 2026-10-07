#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# The budget gate (OW-WAR-0120; docs/RETENTION.md).
#
# Times each command in a budget file (tools/scale/budget.toml) against a
# corpus, --runs times each, and compares the median with the row's limit.
#
#   within    the median is at or under the limit
#   over      the median is over the limit, or a run was still going at the
#             cap (--cap-factor times the limit) and was stopped. A stopped
#             run is recorded as "at least <cap> ms", and the row's later
#             runs are skipped, because they cannot bring it back under
#   unknown   a run exited non-zero. A command that failed has no time, and
#             it is never within budget (Law 15)
#
# Exits 1 when any row is over or unknown, naming each one on stderr, and
# 0 only when every row is within. The record says whether the budget's
# `applies_to` (build profile, Warrant count, resolved count) matches what
# was measured. The limits are compared either way.
#
# Every command runs with no ssh-agent reachable (SSH_AUTH_SOCK and
# SSH_AGENT_PID unset), so an interactive row cannot sign anything.
#
# Usage:
#   tools/scale/budget.sh --war <binary> --root <corpus> --budget <file>
#                         [--runs <n>] [--out <file>] [--cap-factor <k>]
#
# --runs defaults to 5. --cap-factor defaults to 10; 0 means no cap.

set -euo pipefail

usage() {
    sed -n '/^# Usage:/,/^# --runs/p' "$0" | sed 's/^# \{0,1\}//' >&2
    exit 2
}

WAR_ARG=""
ROOT=""
BUDGET=""
RUNS=5
OUT=""
CAP=10
while [[ $# -gt 0 ]]; do
    case "$1" in
        --war) WAR_ARG="${2:?}"; shift 2 ;;
        --root) ROOT="${2:?}"; shift 2 ;;
        --budget) BUDGET="${2:?}"; shift 2 ;;
        --runs) RUNS="${2:?}"; shift 2 ;;
        --out) OUT="${2:?}"; shift 2 ;;
        --cap-factor) CAP="${2:?}"; shift 2 ;;
        -h|--help) usage ;;
        *) printf 'budget.sh: unknown argument %s\n' "$1" >&2; usage ;;
    esac
done
[[ -n "$WAR_ARG" && -n "$ROOT" && -n "$BUDGET" ]] || usage
[[ "$RUNS" =~ ^[1-9][0-9]*$ && "$CAP" =~ ^[0-9]+$ ]] || usage
WAR=$(command -v -- "$WAR_ARG" 2>/dev/null || true)
[[ -n "$WAR" && -x "$WAR" ]] || { printf 'budget.sh: no war binary at %s\n' "$WAR_ARG" >&2; exit 2; }
WAR=$(cd "$(dirname "$WAR")" && printf '%s/%s' "$PWD" "$(basename "$WAR")")
[[ -d "$ROOT" ]] || { printf 'budget.sh: no corpus at %s\n' "$ROOT" >&2; exit 2; }
ROOT=$(cd "$ROOT" && pwd)
[[ -f "$BUDGET" ]] || { printf 'budget.sh: no budget at %s\n' "$BUDGET" >&2; exit 2; }
BUDGET=$(cd "$(dirname "$BUDGET")" && printf '%s/%s' "$PWD" "$(basename "$BUDGET")")
if [[ -n "$OUT" ]]; then
    [[ -d "$(dirname -- "$OUT")" ]] || { printf 'budget.sh: %s does not exist\n' "$(dirname -- "$OUT")" >&2; exit 2; }
    OUT="$(cd "$(dirname -- "$OUT")" && pwd)/$(basename -- "$OUT")"
fi

export OPENWARRANT_NO_PROJECTS=1
unset SSH_AUTH_SOCK SSH_AGENT_PID
SCRIPT_REPO=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

# The budget's rows, one per line: id <TAB> limit_ms <TAB> stdin(JSON) <TAB> argv(JSON)
python3 - "$BUDGET" > "$TMP/rows.tsv" <<'PY'
import json, sys, tomllib
b = tomllib.load(open(sys.argv[1], "rb"))
if b.get("schema") != "openwarrant.scale/budget/v1":
    sys.exit(f"budget.sh: {sys.argv[1]} is not an openwarrant.scale/budget/v1 file")
seen = set()
for c in b.get("command", []):
    cid, argv, limit = c["id"], c["argv"], c["limit_ms"]
    if cid in seen or not isinstance(limit, int) or limit < 1 or not argv:
        sys.exit(f"budget.sh: row {cid!r} is malformed or repeated")
    seen.add(cid)
    print("\t".join([cid, str(limit), json.dumps(c.get("stdin", "")), json.dumps(argv)]))
if not seen:
    sys.exit("budget.sh: the budget names no command")
PY

LOAD_START=$(cut -d' ' -f1-3 /proc/loadavg 2>/dev/null || echo unknown)
now_ns() { date +%s%N; }

# The corpus as the tool counts it, before anything is timed.
"$WAR" --root "$ROOT" --json status > "$TMP/status.json" 2>/dev/null </dev/null || true
CORPUS_BEFORE=$(git -C "$ROOT" status --porcelain 2>/dev/null | sha256sum | cut -d' ' -f1)

: > "$TMP/runs.tsv"
while IFS=$'\t' read -r id limit stdin_json argv_json; do
    mapfile -t ARGV < <(python3 -c 'import json,sys; [print(a) for a in json.loads(sys.argv[1])]' "$argv_json")
    python3 -c 'import json,sys; sys.stdout.write(json.loads(sys.argv[1]))' "$stdin_json" > "$TMP/stdin"
    cap_ms=0
    if [[ "$CAP" != 0 ]]; then cap_ms=$(( limit * CAP )); fi
    for run in $(seq 1 "$RUNS"); do
        rc=0
        t0=$(now_ns)
        if [[ "$cap_ms" != 0 ]]; then
            timeout --kill-after=5 "$(printf '%d.%03d' $((cap_ms / 1000)) $((cap_ms % 1000)))" \
                "$WAR" --root "$ROOT" "${ARGV[@]}" < "$TMP/stdin" > "$TMP/out.log" 2>&1 || rc=$?
        else
            "$WAR" --root "$ROOT" "${ARGV[@]}" < "$TMP/stdin" > "$TMP/out.log" 2>&1 || rc=$?
        fi
        t1=$(now_ns)
        ms=$(( (t1 - t0) / 1000000 ))
        if [[ "$cap_ms" != 0 && ( "$rc" == 124 || "$rc" == 137 ) ]]; then
            printf '%s\t%s\tcapped\t%s\t%s\n' "$id" "$run" "$cap_ms" "$rc" >> "$TMP/runs.tsv"
            break
        fi
        printf '%s\t%s\tdone\t%s\t%s\n' "$id" "$run" "$ms" "$rc" >> "$TMP/runs.tsv"
        if [[ "$rc" != 0 ]]; then
            tail -3 "$TMP/out.log" | sed "s/^/budget.sh: $id: /" >&2
            break
        fi
    done
done < "$TMP/rows.tsv"

LOAD_END=$(cut -d' ' -f1-3 /proc/loadavg 2>/dev/null || echo unknown)
CORPUS_AFTER=$(git -C "$ROOT" status --porcelain 2>/dev/null | sha256sum | cut -d' ' -f1)

case "$WAR" in
    */target/release/*|*/release/war) PROFILE=release; PROFILE_BY="binary path (target/release)" ;;
    */target/debug/*|*/debug/war) PROFILE=debug; PROFILE_BY="binary path (target/debug)" ;;
    *)
        if readelf -S "$WAR" 2>/dev/null | grep -q '\.debug_info'; then PROFILE=debug; PROFILE_BY="ELF .debug_info section present"
        else PROFILE=unknown; PROFILE_BY="not determined from the binary"; fi ;;
esac

export TMP RUNS CAP WAR ROOT BUDGET OUT PROFILE PROFILE_BY LOAD_START LOAD_END CORPUS_BEFORE CORPUS_AFTER SCRIPT_REPO
export WAR_VERSION; WAR_VERSION=$("$WAR" --version 2>/dev/null | head -1 || true)
export WAR_SHA; WAR_SHA=$(sha256sum "$WAR" | cut -d' ' -f1)
export REPO_COMMIT; REPO_COMMIT=$(git -C "$SCRIPT_REPO" rev-parse HEAD 2>/dev/null || echo unknown)
export BUDGET_SHA; BUDGET_SHA=$(sha256sum "$BUDGET" | cut -d' ' -f1)
export CPU; CPU=$(awk -F': ' '/^model name/{print $2; exit}' /proc/cpuinfo 2>/dev/null || echo unknown)

python3 - <<'PY'
import collections, datetime, json, os, platform, statistics, sys, tomllib

E = os.environ
T = E["TMP"]
budget = tomllib.load(open(E["BUDGET"], "rb"))
applies_to = budget.get("applies_to", {})

runs = collections.defaultdict(list)
for line in open(f"{T}/runs.tsv", encoding="utf-8"):
    cid, run, kind, value, rc = line.rstrip("\n").split("\t")
    runs[cid].append((kind, int(value), int(rc)))

# The corpus, as `war status --json` counted it.
corpus = {"root": E["ROOT"], "counted_by": "war status --json"}
try:
    ws = json.load(open(f"{T}/status.json"))["result"]["warrants"]
    phases = collections.Counter(w["state"]["phase"] for w in ws)
    corpus.update(warrants=len(ws), by_phase=dict(sorted(phases.items())), resolved=phases.get("resolved", 0))
except Exception as e:
    corpus.update(warrants="unknown", by_phase="unknown", resolved="unknown", error=f"war status --json unreadable: {e}")
marker = os.path.join(E["ROOT"], "SYNTH.json")
if os.path.isfile(marker):
    try:
        corpus["generator"] = json.load(open(marker))
    except Exception:
        corpus["generator"] = "SYNTH.json unreadable"
corpus["tree_changed_by_the_run"] = E["CORPUS_BEFORE"] != E["CORPUS_AFTER"]

rows, over, unknown = [], [], []
for c in budget["command"]:
    cid, limit = c["id"], c["limit_ms"]
    rs = runs.get(cid, [])
    row = {"id": cid, "argv": ["war", *c["argv"]], "limit_ms": limit}
    if c.get("stdin"):
        row["stdin"] = c["stdin"]
    row["exit_codes"] = [rc for kind, _, rc in rs if kind == "done"]
    failed = any(kind == "done" and rc != 0 for kind, _, rc in rs)
    capped = [v for kind, v, _ in rs if kind == "capped"]
    done_ms = [v for kind, v, rc in rs if kind == "done" and rc == 0]
    if not rs or failed:
        # Law 15: a command that failed has no time, and is not within budget.
        row.update(verdict="unknown", median_ms="unknown", max_ms="unknown", times_ms="unknown",
                   why="exited non-zero" if rs else "never ran")
        unknown.append(cid)
    elif capped:
        row.update(verdict="over", median_ms="unknown", max_ms="unknown", times_ms=done_ms,
                   at_least_ms=capped[0],
                   why=f"run {len(done_ms) + 1} was still going at the cap of {capped[0]} ms "
                       f"({E['CAP']}x the limit) and was stopped; later runs skipped")
        over.append(cid)
    else:
        med = statistics.median_low(done_ms)
        row.update(times_ms=done_ms, median_ms=med, max_ms=max(done_ms),
                   verdict="within" if med <= limit else "over")
        if med > limit:
            over.append(cid)
    rows.append(row)

reasons = []
if applies_to.get("build_profile") and applies_to["build_profile"] != E["PROFILE"]:
    reasons.append(f"build profile {E['PROFILE']}, budget is for {applies_to['build_profile']}")
if isinstance(corpus["warrants"], int):
    if applies_to.get("warrants") and corpus["warrants"] != applies_to["warrants"]:
        reasons.append(f"{corpus['warrants']} Warrants, budget is for {applies_to['warrants']}")
    if applies_to.get("resolved_at_least") and corpus["resolved"] < applies_to["resolved_at_least"]:
        reasons.append(f"{corpus['resolved']} resolved, budget wants at least {applies_to['resolved_at_least']}")
else:
    reasons.append("corpus shape unknown")

held = not over and not unknown
record = {
    "schema": "openwarrant.scale/budget-run/v1",
    "recorded_at": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "procedure": "tools/scale/budget.sh (OW-WAR-0120); docs/RETENTION.md",
    "budget": {"file": os.path.relpath(E["BUDGET"], E["SCRIPT_REPO"]) if E["BUDGET"].startswith(E["SCRIPT_REPO"] + "/") else E["BUDGET"],
               "sha256": E["BUDGET_SHA"], "applies_to": applies_to,
               "applies": not reasons, "does_not_apply_because": reasons},
    "runs": int(E["RUNS"]),
    "cap_factor": int(E["CAP"]),
    "war": {"path": E["WAR"], "version": E["WAR_VERSION"], "build_profile": E["PROFILE"],
            "build_profile_determined_by": E["PROFILE_BY"], "sha256": E["WAR_SHA"]},
    "repository": {"commit": E["REPO_COMMIT"]},
    "machine": {"os": platform.system(), "kernel": platform.release(), "arch": platform.machine(),
                "cpu_model": E["CPU"], "nproc": os.cpu_count(),
                "load_average_start": E["LOAD_START"].split(), "load_average_end": E["LOAD_END"].split()},
    "timing": "wall clock per run, milliseconds, `date +%s%N` around the command; no ssh-agent reachable",
    "corpus": corpus,
    "rows": rows,
    "over": over,
    "unknown": unknown,
    "held": held,
}
if E["OUT"]:
    with open(E["OUT"], "w", encoding="utf-8") as f:
        json.dump(record, f, indent=2, ensure_ascii=False)
        f.write("\n")
for r in rows:
    shown = r["median_ms"] if r["median_ms"] != "unknown" else (f">={r['at_least_ms']}" if "at_least_ms" in r else "unknown")
    print(f"{r['verdict']:8} {r['id']:22} median {shown} ms, limit {r['limit_ms']} ms")
for cid in over:
    print(f"budget.sh: OVER BUDGET: {cid}", file=sys.stderr)
for cid in unknown:
    print(f"budget.sh: UNKNOWN (a run exited non-zero): {cid}", file=sys.stderr)
if not record["budget"]["applies"]:
    print("budget.sh: note: the budget's applies_to does not match: " + "; ".join(reasons), file=sys.stderr)
sys.exit(0 if held else 1)
PY
