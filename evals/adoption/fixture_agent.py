#!/usr/bin/env python3
"""A scripted stand-in for a model, for dry runs that cost nothing.

    fixture_agent.py reference   apply the next item's reference patch and record it
                                 done the way the arm's tool records it
    fixture_agent.py noop        change nothing
    fixture_agent.py claim       record the next item done without doing it (a false completion)
    fixture_agent.py claim-broken
                                 add a failing test to the visible suite, then record the target item
                                 done: a tool that tests before it closes must refuse (amendment 8)
    fixture_agent.py spec        read the spec a fixture author wrote, do what its FIXTURE-PATCH line
                                 says (reference, or the natural default), and record the spec done

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
    elif arm == "jira":
        import jira_arm
        j = jira_arm.Jira(env)
        for issue in j.search(f'project = "{j.project}" AND labels = "{env["STUDY_JIRA_LABEL"]}"', "labels"):
            if label in (issue["fields"].get("labels") or []):
                j.transition_to_done(issue["key"])
    elif arm == "openwarrant":
        rows = json.loads(subprocess.run(["war", "view", "tickets", "--json"], capture_output=True,
                                         text=True, env=env).stdout)["result"]["tickets"]
        for t in rows:
            if label in (t.get("labels") or []):
                arms.openwarrant_done(t["id"], Path.cwd(), {**env, "STUDY_WAR": "war",
                                                              "OPENWARRANT_ACTOR": "fixture"}, "fixture")


def spec_marker(arm: str) -> str | None:
    """The FIXTURE-PATCH line, wherever this tool keeps a spec: a stand-in for a worker reading it."""
    found = subprocess.run(["git", "grep", "-h", "FIXTURE-PATCH:"], capture_output=True, text=True).stdout
    if arm == "beads":
        found += subprocess.run(["bd", "list", "--json", "--all"], capture_output=True, text=True).stdout
    for word in found.split("FIXTURE-PATCH:")[1:]:
        return word.split()[0].strip('",')
    return None


def spec_mark_done(arm: str) -> None:
    """Finish every record the tool holds, as a worker that believes the work is done would."""
    if arm == "plans":
        p = Path("PLAN.md")
        p.write_text(p.read_text().replace("- [ ]", "- [x]"))
    elif arm == "speckit":
        for p in Path("specs").glob("*/tasks.md"):
            p.write_text(p.read_text().replace("- [ ]", "- [X]"))
    elif arm == "beads":
        rows = json.loads(subprocess.run(["bd", "list", "--json", "--all"], capture_output=True, text=True).stdout or "[]")
        for issue in rows:
            subprocess.run(["bd", "close", issue["id"], "--reason", "fixture"], check=True, capture_output=True)
    elif arm == "openwarrant":
        rows = json.loads(subprocess.run(["war", "view", "tickets", "--json"], capture_output=True,
                                         text=True).stdout)["result"]["tickets"]
        for t in rows:
            arms.openwarrant_done(t["id"], Path.cwd(), {**os.environ, "STUDY_WAR": "war",
                                                        "OPENWARRANT_ACTOR": "fixture"}, "fixture")


def spec_worker(arm: str, iid: str) -> None:
    marker = spec_marker(arm)
    if marker is None:
        result_line("spec: no FIXTURE-PATCH line, changed nothing")
        return
    name = f"{iid}.patch" if marker == "reference" else f"{iid}.default.patch"
    subprocess.run(["git", "apply", "--whitespace=nowarn", str(DEF / "reference" / name)], check=True)
    spec_mark_done(arm)
    result_line(f"spec: applied {name}")


def main() -> None:
    mode = sys.argv[1]
    if mode == "spec" and os.environ.get("STUDY_AUTHOR") != "backlog":
        spec_worker(os.environ["STUDY_ARM"], os.environ["STUDY_TARGET"])
        return
    if mode == "spec":  # the control has no spec to read: the study's own text, as `reference` reads it
        mode = "reference"
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
    if mode == "claim-broken":
        # tally's layout: pytest collects tests/. The claim is made through the arm's own tool, so a
        # tool that runs the project's tests before closing a ticket refuses it, and says so.
        Path("tests/test_zz_fixture_broken.py").write_text("def test_fixture_broken():\n    assert False\n")
        try:
            mark_done(arm, backlog, todo[0])
        except (RuntimeError, subprocess.CalledProcessError) as e:
            result_line(f"claim-broken: the tool refused to record {todo[0]} done: {str(e)[:300]}")
            return
        result_line(f"claim-broken: recorded {todo[0]} done with the visible tests failing")
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
