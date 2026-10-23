#!/usr/bin/env python3
"""A scripted stand-in for a model, for dry runs that cost nothing.

    fixture_agent.py reference   apply the next item's reference patch and record it
                                 done the way the arm's tool records it
    fixture_agent.py noop        change nothing
    fixture_agent.py claim       record the next item done without doing it (a false completion)

It runs inside the same sandbox as a real agent session, with the repository
definition mounted at /opt/study/repo-def (a real agent never sees it). Its
stdout is one stream-json `result` line, so the metrics parser runs on it too.
A dry run with `reference` should pass every item it reaches and record no false
completion; with `noop` it should pass nothing and stall. Both outcomes are the
harness's own refusal checks: if either comes out otherwise, the harness is wrong.
"""

import json
import os
import subprocess
import sys
import tomllib
from pathlib import Path

sys.path.insert(0, "/opt/study/code")
import arms  # noqa: E402

DEF = Path("/opt/study/repo-def")


def result_line(note: str) -> None:
    print(json.dumps({"type": "result", "subtype": "success", "is_error": False, "num_turns": 0,
                      "duration_ms": 0, "total_cost_usd": 0, "result": note,
                      "usage": {"input_tokens": 0, "output_tokens": 0, "cache_read_input_tokens": 0,
                                "cache_creation_input_tokens": 0}}))


def ledger() -> Path:
    """What this fixture has applied in this checkout, kept in the cell directory, outside the
    repository: a reverse-apply check is wrong once a later patch edits the same lines."""
    return Path("/work") / f".fixture-applied-{Path.cwd().name}"


def applied_ids() -> set[str]:
    return set(ledger().read_text().split()) if ledger().exists() else set()


def mark_done(arm: str, backlog: dict, iid: str) -> None:
    env = {**os.environ}
    label = "item-" + iid.lower()
    if arm == "plans":
        p = Path("PLAN.md")
        text = p.read_text().replace(f"- [ ] **{iid} — ", f"- [x] **{iid} — ", 1)
        p.write_text(text.rstrip("\n") + f"\n- (fixture) Done: {iid}.\n")
    elif arm == "speckit":
        p = Path("specs/001-backlog/tasks.md")
        text = p.read_text()
        for tid in arms._speckit_tasks(backlog)[iid]:
            text = text.replace(f"- [ ] {tid} ", f"- [X] {tid} ")
        p.write_text(text)
    elif arm == "beads":
        rows = json.loads(subprocess.run(["bd", "list", "--json", "--all"], capture_output=True,
                                         text=True, env=env, check=True).stdout or "[]")
        for issue in rows:
            if label in (issue.get("labels") or []):
                subprocess.run(["bd", "close", issue["id"], "--reason", "fixture"], env=env, check=True,
                               capture_output=True)
    elif arm == "openwarrant":
        rows = json.loads(subprocess.run(["war", "view", "tickets", "--json"], capture_output=True,
                                         text=True, env=env).stdout)["result"]["tickets"]
        for t in rows:
            if label in (t.get("labels") or []):
                arms.openwarrant_done(t["id"], Path.cwd(), {**env, "STUDY_WAR": "war",
                                                              "OPENWARRANT_ACTOR": "fixture"}, "fixture")


def main() -> None:
    mode = sys.argv[1]
    if mode == "noop":
        result_line("noop: changed nothing")
        return
    with open(DEF / "backlog.toml", "rb") as f:
        backlog = tomllib.load(f)
    arm = os.environ["STUDY_ARM"]
    target = os.environ.get("STUDY_TARGET")
    order = backlog["order"]
    done = applied_ids()
    todo = [target] if target else [i for i in order if i not in done]
    if not todo:
        result_line("reference: nothing left")
        return
    if mode == "claim":
        # The next item the tracker does not yet record as done, ticked without any work.
        state = arms.TRACKERS[arm](Path.cwd(), backlog, {**os.environ, "STUDY_WAR": "war", "STUDY_BD": "bd"})
        todo = [target] if target else [i for i in order if state.get(i) is False]
        if not todo:
            result_line("claim: nothing to claim")
            return
        mark_done(arm, backlog, todo[0])
        result_line(f"claim: recorded {todo[0]} done, did nothing")
        return
    iid = todo[0]
    subprocess.run(["git", "apply", "--whitespace=nowarn", str(DEF / "reference" / f"{iid}.patch")], check=True)
    with open(ledger(), "a") as f:
        f.write(iid + "\n")
    mark_done(arm, backlog, iid)
    result_line(f"reference: applied {iid}")


if __name__ == "__main__":
    main()
