"""The five arms: one tool-neutral backlog written into each tool's own form.

Every arm's CLAUDE.md starts with the same project facts. What differs is only
where the backlog lives and the instruction that points at it, and for Beads and
OpenWarrant that instruction is the text the tool itself installs. Each seeder
also marks the items in `done` as finished, the way that tool records it, so a
directed session starts from the same state in every arm.

Each arm also has a reader (`tracker`) that answers, per item, whether the
tool's own record says the item is done: True, False, or None when the arm has
no record of it (the plain arm). The grader's hidden tests are the truth; the
reader is what lets the study count false completions.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
from pathlib import Path

ARMS = ("plain", "plans", "speckit", "beads", "openwarrant", "jira")

SEED_ENV = {
    "GIT_AUTHOR_NAME": "study-seed",
    "GIT_AUTHOR_EMAIL": "seed@adoption.invalid",
    "GIT_COMMITTER_NAME": "study-seed",
    "GIT_COMMITTER_EMAIL": "seed@adoption.invalid",
    "OPENWARRANT_ACTOR": "planner",
    "BEADS_ACTOR": "planner",
}


def _run(argv, cwd, env, check=True, input_text=None):
    p = subprocess.run(
        argv, cwd=cwd, env=env, text=True, input=input_text,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=300,
    )
    if check and p.returncode != 0:
        raise RuntimeError(f"{argv[0]} {' '.join(argv[1:3])} failed ({p.returncode}): {p.stderr.strip()[-800:]}")
    return p


def _label(item_id: str) -> str:
    return "item-" + item_id.lower()


def render_item(item: dict, heading: str = "###", status: str | None = None) -> str:
    """One item as Markdown: the same words in every arm that shows Markdown."""
    out = [f"{heading} {item['id']} — {item['title']}", ""]
    if status:
        out += [f"Status: {status}", ""]
    out += [item["summary"].strip(), "", item["description"].strip(), ""]
    out += ["**Acceptance**", ""] + [f"- {c}" for c in item["acceptance"]] + [""]
    if item.get("notes", "").strip():
        out += ["**Notes**", "", item["notes"].strip(), ""]
    deps = item.get("depends_on", [])
    out += [f"**Depends on:** {', '.join(deps) if deps else 'nothing'}", ""]
    return "\n".join(out)


def facts_header(backlog: dict) -> str:
    return f"# {backlog['repo']}\n\n{backlog['project']['facts'].strip()}\n"


# ---------------------------------------------------------------- plain


def seed_plain(repo: Path, backlog: dict, done: set[str], env: dict) -> None:
    parts = [facts_header(backlog), "## Backlog", ""]
    for item in backlog["item"]:
        parts.append(render_item(item, status="done" if item["id"] in done else None))
    (repo / "CLAUDE.md").write_text("\n".join(parts))


def tracker_plain(repo: Path, backlog: dict, env: dict) -> dict:
    return {i["id"]: None for i in backlog["item"]}


# ---------------------------------------------------------------- plans

PLANS_POINTER = """## Planned work

The backlog is in `PLAN.md`. Read it, and its handoff log, before starting.
When you finish an item, mark it `[x]` in `PLAN.md` and add a dated line to the
handoff log saying what you did and anything the next session needs to know.
"""


def seed_plans(repo: Path, backlog: dict, done: set[str], env: dict) -> None:
    (repo / "CLAUDE.md").write_text(facts_header(backlog) + "\n" + PLANS_POINTER)
    parts = [f"# Plan: {backlog['repo']}", "", "## Items", ""]
    for item in backlog["item"]:
        box = "x" if item["id"] in done else " "
        body = render_item(item, heading="").split("\n", 2)[2]  # drop the heading line
        parts += [f"- [{box}] **{item['id']} — {item['title']}**", ""]
        parts += ["  " + line if line else "" for line in body.splitlines()] + [""]
    parts += ["## Handoff log", ""]
    if done:
        parts += [f"- (before this session) Done: {', '.join(sorted(done))}.", ""]
    (repo / "PLAN.md").write_text("\n".join(parts))


_PLAN_BOX = re.compile(r"^- \[([ xX])\] \*\*([A-Za-z0-9]+) — ")


def tracker_plans(repo: Path, backlog: dict, env: dict) -> dict:
    state = {i["id"]: False for i in backlog["item"]}
    path = repo / "PLAN.md"
    if not path.exists():
        return {k: None for k in state}
    for line in path.read_text(errors="replace").splitlines():
        m = _PLAN_BOX.match(line)
        if m and m.group(2) in state:
            state[m.group(2)] = m.group(1) in "xX"
    return state


# ---------------------------------------------------------------- speckit

SPECKIT_POINTER = """## Planned work

This project uses Spec Kit. The backlog is specified in `specs/001-backlog/`
(`spec.md`, `plan.md`, `tasks.md`). Implement it with `/speckit-implement`, which
checks tasks off in `tasks.md` as they are done.
"""


def _speckit_tasks(backlog: dict) -> dict[str, list[str]]:
    """Task ids per item: one task per acceptance criterion, numbered T001.. in order."""
    n, out = 0, {}
    for item in backlog["item"]:
        out[item["id"]] = []
        for _ in item["acceptance"]:
            n += 1
            out[item["id"]].append(f"T{n:03d}")
    return out


def seed_speckit(repo: Path, backlog: dict, done: set[str], env: dict) -> None:
    _run([env["STUDY_SPECIFY"], "init", "--here", "--force", "--non-interactive",
          "--integration", "claude", "--script", "sh", "--ignore-agent-tools"], repo, env)
    (repo / "CLAUDE.md").write_text(facts_header(backlog) + "\n" + SPECKIT_POINTER)
    # What `/speckit-constitution` leaves in a set-up project, holding only the facts every arm gets.
    (repo / ".specify" / "memory" / "constitution.md").write_text(
        f"# {backlog['repo']} Constitution\n\n## Core Principles\n\n### I. The project as it is\n\n"
        + backlog["project"]["facts"].strip()
        + "\n\n## Governance\n\nThis constitution records the project's existing facts; it adds no rule"
        " beyond them.\n\n**Version**: 1.0.0 | **Ratified**: 2026-10-22 | **Last Amended**: 2026-10-22\n")
    feature = repo / "specs" / "001-backlog"
    feature.mkdir(parents=True, exist_ok=True)
    stories = {item["id"]: n + 1 for n, item in enumerate(backlog["item"])}

    spec = [f"# Feature Specification: {backlog['repo']} backlog", "",
            "**Feature Branch**: `001-backlog`", "**Status**: Ready for implementation", "",
            "## User Scenarios & Testing *(mandatory)*", ""]
    for item in backlog["item"]:
        us = stories[item["id"]]
        spec += [f"### User Story {us} - {item['id']}: {item['title']} (Priority: P{us})", "",
                 item["summary"].strip(), "", item["description"].strip(), "",
                 "**Independent Test**: the acceptance scenarios below.", "",
                 "**Acceptance Scenarios**:", ""]
        spec += [f"{k}. {c}" for k, c in enumerate(item["acceptance"], 1)] + [""]
        if item.get("notes", "").strip():
            spec += ["**Design decisions**:", "", item["notes"].strip(), ""]
    (feature / "spec.md").write_text("\n".join(spec))

    plan = [f"# Implementation Plan: {backlog['repo']} backlog", "",
            "**Spec**: [spec.md](spec.md)", "", "## Technical Context", "",
            backlog["project"]["facts"].strip(), "", "## Constitution Check", "",
            "No conflict: the constitution records the same project facts.", "",
            "## Project Structure", "", "The existing layout described above; no new top-level directories.", "",
            "## Story order and dependencies", ""]
    for item in backlog["item"]:
        deps = item.get("depends_on", [])
        plan.append(f"- User Story {stories[item['id']]} ({item['id']}): "
                    + (f"after {', '.join(deps)}" if deps else "no prerequisite"))
    (feature / "plan.md").write_text("\n".join(plan) + "\n")

    task_ids = _speckit_tasks(backlog)
    tasks = [f"# Tasks: {backlog['repo']} backlog", "",
             "**Input**: Design documents from `/specs/001-backlog/`", "",
             "## Format: `[ID] [P?] [Story] Description`", ""]
    for item in backlog["item"]:
        us = stories[item["id"]]
        deps = item.get("depends_on", [])
        tasks += [f"## Phase {us}: User Story {us} - {item['id']}: {item['title']} (Priority: P{us})", "",
                  f"**Goal**: {item['summary'].strip()}", "",
                  "**Depends on**: " + (", ".join(deps) if deps else "nothing"), ""]
        mark = "X" if item["id"] in done else " "
        for tid, crit in zip(task_ids[item["id"]], item["acceptance"]):
            tasks.append(f"- [{mark}] {tid} [US{us}] {crit}")
        tasks.append("")
    (feature / "tasks.md").write_text("\n".join(tasks))
    (repo / ".specify" / "feature.json").write_text(
        json.dumps({"feature_directory": "specs/001-backlog"}, indent=2) + "\n")


_SK_TASK = re.compile(r"^- \[([ xX])\] (T\d{3})\b")


def tracker_speckit(repo: Path, backlog: dict, env: dict) -> dict:
    path = repo / "specs" / "001-backlog" / "tasks.md"
    if not path.exists():
        return {i["id"]: None for i in backlog["item"]}
    checked = {}
    for line in path.read_text(errors="replace").splitlines():
        m = _SK_TASK.match(line)
        if m:
            checked[m.group(2)] = m.group(1) in "xX"
    return {iid: bool(tids) and all(checked.get(t, False) for t in tids)
            for iid, tids in _speckit_tasks(backlog).items()}


# ---------------------------------------------------------------- beads


def seed_beads(repo: Path, backlog: dict, done: set[str], env: dict) -> None:
    bd = env["STUDY_BD"]
    _run([bd, "init", "--non-interactive"], repo, env)
    claude = repo / "CLAUDE.md"
    text = claude.read_text()
    end = "<!-- END BEADS INTEGRATION -->"
    if end not in text:
        raise RuntimeError("bd init wrote no Beads block into CLAUDE.md")
    # Keep the block Beads installs verbatim; replace its placeholder sections with the facts.
    head, _ = text.split(end, 1)
    head = head.replace("# Project Instructions for AI Agents", facts_header(backlog).rstrip(), 1)
    claude.write_text(head + end + "\n")
    ids = {}
    for item in backlog["item"]:
        argv = [bd, "create", f"{item['id']}: {item['title']}",
                "-d", item["summary"].strip() + "\n\n" + item["description"].strip(),
                "--acceptance", "\n".join(f"- {c}" for c in item["acceptance"]),
                "-l", _label(item["id"]), "--json"]
        if item.get("notes", "").strip():
            argv += ["--design", item["notes"].strip()]
        for dep in item.get("depends_on", []):
            argv += ["--deps", ids[dep]]
        ids[item["id"]] = json.loads(_run(argv, repo, env).stdout)["id"]
    for iid in backlog["order"]:
        if iid in done:
            _run([bd, "close", ids[iid], "--reason", "done before this session"], repo, env)


def tracker_beads(repo: Path, backlog: dict, env: dict) -> dict:
    p = _run([env["STUDY_BD"], "list", "--json", "--all"], repo, env, check=False)
    state: dict[str, bool | None] = {i["id"]: None for i in backlog["item"]}
    if p.returncode != 0:
        return state
    for issue in json.loads(p.stdout or "[]"):
        for label in issue.get("labels") or []:
            for iid in state:
                if label == _label(iid):
                    state[iid] = issue.get("status") == "closed"
    return state


# ---------------------------------------------------------------- openwarrant


def _war(argv, repo, env):
    p = _run([env["STUDY_WAR"], *argv, "--json"], repo, env, check=False)
    report = json.loads(p.stdout)
    if report.get("exit_code", 1) != 0:
        raise RuntimeError(f"war {argv[0]}: {[d.get('message') for d in report.get('diagnostics', [])][:3]}")
    return report["result"]


def seed_openwarrant(repo: Path, backlog: dict, done: set[str], env: dict) -> None:
    war = env["STUDY_WAR"]
    # QUICKSTART's order: the project's CLAUDE.md exists, and `war init` appends its managed block.
    (repo / "CLAUDE.md").write_text(facts_header(backlog))
    _run([war, "init", "--root", "."], repo, env)
    if "<!-- openwarrant:begin -->" not in (repo / "CLAUDE.md").read_text():
        raise RuntimeError("war init did not add its block to CLAUDE.md")
    if env.get("ADOPTION_OW_SESSION_START") == "1":
        # The opt-in session hook (`[tickets] session_start`), when the run enables it.
        with open(repo / "openwarrant.toml", "a") as f:
            f.write("\n[tickets]\nsession_start = true\n")
    tickets = {}
    for item in backlog["item"]:
        body = item["description"].strip()
        if item.get("notes", "").strip():
            # Not `## Notes`: that heading is where `war note` appends dated notes,
            # and OpenWarrant's views read what is under it as those, not as
            # part of the description.
            body += "\n\n## Design notes\n\n" + item["notes"].strip()
        deps = [tickets[d] for d in item.get("depends_on", [])]
        argv = ["create", f"{item['id']}: {item['title']}", "--summary", item["summary"].strip(),
                "--body", body, "-l", _label(item["id"])]
        if not deps:
            for crit in item["acceptance"]:
                argv += ["--item", crit]
        tickets[item["id"]] = _war(argv, repo, env)["id"]
        if deps:
            for crit in item["acceptance"]:
                add = ["add", tickets[item["id"]], crit]
                for t in deps:
                    add += ["--after", t]
                _war(add, repo, env)
    for iid in backlog["order"]:
        if iid in done:
            openwarrant_done(tickets[iid], repo, env, "done before this session")


def openwarrant_done(ticket: str, repo: Path, env: dict, note: str) -> None:
    """A ticket is done when each of its items is: `war done` per item, as an agent would."""
    shown = _war(["show", ticket], repo, env)
    for item in shown["items"]:
        if not item["done"]:
            _war(["done", f"{ticket}/{item['id']}", "--note", note], repo, env)


def tracker_openwarrant(repo: Path, backlog: dict, env: dict) -> dict:
    state: dict[str, bool | None] = {i["id"]: None for i in backlog["item"]}
    p = _run([env["STUDY_WAR"], "view", "tickets", "--json"], repo, env, check=False)
    try:
        rows = json.loads(p.stdout)["result"]
    except (ValueError, KeyError, TypeError):
        return state
    rows = rows.get("tickets", rows) if isinstance(rows, dict) else rows
    for t in rows or []:
        for label in t.get("labels") or []:
            for iid in state:
                if label == _label(iid):
                    total, finished = t.get("total"), t.get("done")
                    state[iid] = bool(total) and finished == total
    return state


# ---------------------------------------------------------------- jira


def seed_jira(repo: Path, backlog: dict, done: set[str], env: dict) -> None:
    import jira_arm
    label = env["STUDY_JIRA_LABEL"]
    pointer = jira_arm.POINTER.format(project=env["JIRA_PROJECT"], site=env["JIRA_SITE"], label=label)
    (repo / "CLAUDE.md").write_text(facts_header(backlog) + "\n" + pointer)
    jira_arm.seed(backlog, done, label, env)


def tracker_jira(repo: Path, backlog: dict, env: dict) -> dict:
    import jira_arm
    return jira_arm.tracker(backlog, env["STUDY_JIRA_LABEL"], env)


# ---------------------------------------------------------------- answers to open questions


def _answer_text(item: dict) -> str:
    return f"Answer to the open question: {item['question']['answer'].strip()}"


def deliver_answer(arm: str, repo: Path, backlog: dict, item: dict, env: dict) -> None:
    """The owner's reply to an item's open question, through the arm's own update channel."""
    text = _answer_text(item)
    iid = item["id"]
    if arm == "plain":
        p = repo / "CLAUDE.md"
        lines = p.read_text().splitlines()
        head = next(n for n, l in enumerate(lines) if l.startswith(f"### {iid} — "))
        nxt = next((n for n in range(head + 1, len(lines)) if lines[n].startswith("### ")), len(lines))
        lines[nxt:nxt] = [text, ""]
        p.write_text("\n".join(lines) + "\n")
    elif arm == "plans":
        p = repo / "PLAN.md"
        lines = p.read_text().splitlines()
        head = next(n for n, l in enumerate(lines) if re.match(rf"^- \[[ xX]\] \*\*{iid} — ", l))
        nxt = next((n for n in range(head + 1, len(lines)) if lines[n].startswith("- [") or lines[n].startswith("## ")),
                   len(lines))
        lines[nxt:nxt] = [f"  {text}", ""]
        lines.append(f"- (owner) {iid}: {text}")
        p.write_text("\n".join(lines) + "\n")
    elif arm == "speckit":
        p = repo / "specs" / "001-backlog" / "spec.md"
        s = p.read_text()
        entry = f"- Q: ({iid}) {item['question']['asked'].strip()} → A: {item['question']['answer'].strip()}"
        if "## Clarifications" not in s:
            s = s.replace("## User Scenarios & Testing", "## Clarifications\n\n### Session 2026-10-22\n\n"
                          + entry + "\n\n## User Scenarios & Testing", 1)
        else:
            s = s.replace("### Session 2026-10-22\n\n", "### Session 2026-10-22\n\n" + entry + "\n", 1)
        p.write_text(s)
    elif arm == "beads":
        rows = json.loads(_run([env["STUDY_BD"], "list", "--json", "--all"], repo, env).stdout or "[]")
        bid = next(r["id"] for r in rows if _label(iid) in (r.get("labels") or []))
        _run([env["STUDY_BD"], "comments", "add", bid, text], repo, env)
    elif arm == "openwarrant":
        rows = _war(["view", "tickets"], repo, env)["tickets"]
        tid = next(t["id"] for t in rows if _label(iid) in (t.get("labels") or []))
        _war(["note", tid, text], repo, env)
    elif arm == "jira":
        import jira_arm
        j = jira_arm.Jira(env)
        key = next(i["key"] for i in j.search(f'project = "{j.project}" AND labels = "{env["STUDY_JIRA_LABEL"]}"', "labels")
                   if _label(iid) in (i["fields"].get("labels") or []))
        j.call("POST", f"/rest/api/3/issue/{key}/comment", {"body": jira_arm.adf(text)})


def deliver_answers(arm: str, repo: Path, backlog: dict, env: dict) -> list[str]:
    done = []
    for item in backlog["item"]:
        if item.get("question"):
            deliver_answer(arm, repo, backlog, item, env)
            done.append(item["id"])
    return done


SEEDERS = {"plain": seed_plain, "plans": seed_plans, "speckit": seed_speckit,
           "beads": seed_beads, "openwarrant": seed_openwarrant, "jira": seed_jira}
TRACKERS = {"plain": tracker_plain, "plans": tracker_plans, "speckit": tracker_speckit,
            "beads": tracker_beads, "openwarrant": tracker_openwarrant, "jira": tracker_jira}


def seed(arm: str, repo: Path, backlog: dict, done: set[str], tools: dict) -> None:
    env = {**os.environ, **SEED_ENV, **tools}
    env.pop("SSH_AUTH_SOCK", None)
    env.pop("SSH_AGENT_PID", None)
    SEEDERS[arm](repo, backlog, done, env)
    _run(["git", "add", "-A"], repo, env)
    _run(["git", "commit", "-q", "--no-verify", "-m", f"seed: {arm} backlog"], repo, env, check=False)


def tracker(arm: str, repo: Path, backlog: dict, tools: dict) -> dict:
    env = {**os.environ, **SEED_ENV, **tools}
    env.pop("SSH_AUTH_SOCK", None)
    return TRACKERS[arm](repo, backlog, env)
