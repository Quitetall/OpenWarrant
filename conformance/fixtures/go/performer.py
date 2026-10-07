#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""A fixture performer for `war evidence go` (OW-WAR-0148 M15). No model.

It reads the packet (`oh.war/go-packet/v1`) on stdin, records what it saw,
does a token piece of work in its worktree, and answers with a Stage
Submission carrying the packet's bindings. What it records lets a plant check
the scheduler from the outside:

  GO_LOG      one line per event: `start <node> <attempt> <t_ns> running=<n>`,
              `end <node> <attempt> <t_ns>`, and `VIOLATION <node> ...` when a
              node it was handed waits on one whose checklist line is not
              ticked in the checkout the run started from.
  GO_SCRIPT   optional; lines `<node> <attempt|*> <action> [arg...]`:
                kill            SIGKILL itself before answering
                sleep <secs>    sleep (past a time budget)
                write <path> <text>   write <text> to <path> in the worktree
                answer <action> answer with this requested_next_action
                silent          exit 0 printing nothing
                usage <n>       report <n> tokens of usage
  GO_SLEEP    seconds of "work" per node (default 0.2)

Concurrency is counted under an flock on GO_LOG + ".lock".
"""
import fcntl
import json
import os
import signal
import sys
import time

packet = json.load(sys.stdin)
node = packet["node"]
attempt = packet["attempt"]
log = os.environ.get("GO_LOG", os.devnull)
lockpath = log + ".lock"
countpath = log + ".running"


def locked(fn):
    with open(lockpath, "a+") as lk:
        fcntl.flock(lk, fcntl.LOCK_EX)
        try:
            return fn()
        finally:
            fcntl.flock(lk, fcntl.LOCK_UN)


def bump(delta):
    def go():
        try:
            n = int(open(countpath).read().strip() or "0")
        except (OSError, ValueError):
            n = 0
        n += delta
        open(countpath, "w").write(str(n))
        return n
    return locked(go)


def record(line):
    locked(lambda: open(log, "a").write(line + "\n"))


running = bump(+1)
record(f"start {node} {attempt} {time.time_ns()} running={running}")

# Every node it waits on must already be ticked where the run started.
root = packet["root"]
for dep in packet.get("depends_on", []):
    ticket, _, item = dep.partition("/")
    path = os.path.join(root, "docs", "tickets", ticket, "atoms", "15-checklist.md")
    try:
        lines = open(path, encoding="utf-8").read().splitlines()
    except OSError:
        lines = []
    if item:
        ticked = any(l.startswith("- [x]") and f"({item}" in l for l in lines)
    else:
        items = [l for l in lines if l.startswith("- [")]
        ticked = bool(items) and all(l.startswith("- [x]") for l in items)
    if not ticked:
        record(f"VIOLATION {node} started before {dep} was done")

actions = []
script = os.environ.get("GO_SCRIPT")
if script and os.path.exists(script):
    for raw in open(script, encoding="utf-8"):
        parts = raw.split()
        if len(parts) >= 3 and parts[0] == node and parts[1] in ("*", str(attempt)):
            actions.append(parts[2:])

answer = "verify"
usage = None
wrote_custom = False
time.sleep(float(os.environ.get("GO_SLEEP", "0.2")))
for a in actions:
    if a[0] == "kill":
        bump(-1)
        record(f"killed {node} {attempt} {time.time_ns()}")
        os.kill(os.getpid(), signal.SIGKILL)
    elif a[0] == "sleep":
        time.sleep(float(a[1]))
    elif a[0] == "write":
        path = os.path.join(packet["worktree"], a[1])
        os.makedirs(os.path.dirname(path) or ".", exist_ok=True)
        with open(path, "a", encoding="utf-8") as f:
            f.write(" ".join(a[2:]) + "\n")
        wrote_custom = True
    elif a[0] == "answer":
        answer = a[1]
    elif a[0] == "usage":
        usage = int(a[1])
    elif a[0] == "silent":
        bump(-1)
        record(f"end {node} {attempt} {time.time_ns()}")
        sys.exit(0)

if not wrote_custom:
    work = os.path.join(packet["worktree"], "work")
    os.makedirs(work, exist_ok=True)
    with open(os.path.join(work, node.replace("/", "--") + ".txt"), "w") as f:
        f.write(f"{node} attempt {attempt}\n")

bump(-1)
record(f"end {node} {attempt} {time.time_ns()}")
out = dict(packet["answer"])
out["requested_next_action"] = answer
out["claims"] = [{"id": "C-001", "statement": "The fixture performer wrote one file."}]
if usage is not None:
    out["usage"] = {"tokens": usage}
print(json.dumps(out, indent=2))
