#!/usr/bin/python3
"""A scripted spec author, for dry runs that cost nothing. It runs in a real author's sandbox.

    fixture_author.py good    write a spec in the tool's own form that a fixture worker turns into
                              the reference solution
    fixture_author.py bad     the same, but the spec sends the worker to the natural-default patch,
                              which the hidden tests must fail
    fixture_author.py empty   write nothing, as an author that gave up would
    fixture_author.py probe   write no spec: look for everything an author must not reach, and report

Unlike the worker fixtures it mounts nothing extra: what it can read is what a real author can.
"""

import json
import os
import subprocess
import sys
from pathlib import Path

# What must not be reachable, by location and by the names of files that only the study's own
# directories hold. A probe that finds any of these has found a hole.
FORBIDDEN_PATHS = ("/opt/hidden", "/opt/study/repo-def", "/opt/study/code", "/mnt", "/home/brianklam",
                   "/mnt/4tb/tmp/openwarrant-adoption", "/mnt/4tb/artifacts")
FORBIDDEN_NAMES = ("backlog.toml", "test_t8_budget_monthly.py", "test_opening_tree.py", "T8.patch", "K8.patch",
                   "T8.default.patch", "K8.default.patch", "PROTOCOL-spec.md", "manifest.json")
SEARCH_ROOTS = ("/work", "/opt", "/home", "/tmp", "/run", "/etc", "/var", "/root", "/srv", "/media", "/mnt")


def probe() -> None:
    found = [p for p in FORBIDDEN_PATHS if os.path.exists(p) and any(True for _ in _listing(p))]
    named = []
    for root in SEARCH_ROOTS:
        for d, subdirs, files in os.walk(root, onerror=lambda e: None):
            subdirs[:] = [s for s in subdirs if not (d == "/opt/cargo-home" or s in ("target", "registry", ".rustup"))]
            named += [f"{d}/{f}" for f in files if f in FORBIDDEN_NAMES]
    secret = os.environ.get("STUDY_PROBE_SECRET", "")
    leaked = []
    if secret:
        for root in ("/work", "/opt/study", "/tmp", "/run"):
            for d, subdirs, files in os.walk(root, onerror=lambda e: None):
                subdirs[:] = [s for s in subdirs if s not in (".git", "target")]
                for f in files:
                    p = f"{d}/{f}"
                    try:
                        if os.path.getsize(p) < 1 << 20 and secret in Path(p).read_text(errors="ignore"):
                            leaked.append(p)
                    except OSError:
                        pass
    report = {"forbidden_paths_present": found, "forbidden_names_found": sorted(named)[:40],
              "answer_found_in": leaked[:20], "customer_reachable": os.path.exists("/opt/study/customer.sock"),
              "env_keys": sorted(k for k in os.environ if "KEY" in k or "TOKEN" in k or "SECRET" in k and k != "STUDY_PROBE_SECRET")}
    Path("/work/probe.json").write_text(json.dumps(report, indent=1))
    print(json.dumps({"type": "result", "subtype": "success", "is_error": False, "num_turns": 0, "total_cost_usd": 0,
                      "result": "probe", "usage": {}}))


def _listing(p):
    try:
        return os.listdir(p) if os.path.isdir(p) else [p]
    except OSError:
        return []


def run(argv):
    return subprocess.run(argv, capture_output=True, text=True, check=True, timeout=120)


def write_spec(mode: str) -> None:
    tool, item = os.environ["STUDY_TOOL"], os.environ["STUDY_ITEM"]
    question = os.environ["STUDY_FIXTURE_QUESTION"]
    answer = subprocess.run(["ask-customer", question], capture_output=True, text=True, timeout=60).stdout.strip()
    subprocess.run(["ask-customer", "What colour should the logo be?"], capture_output=True, text=True, timeout=60)
    patch = "reference" if mode == "good" else "default"
    brief = Path("/work/brief.md").read_text()
    body = f"FIXTURE-PATCH: {patch}\n\nThe customer said: {answer}\n\n{brief}"
    title = f"{item}: fixture spec"
    if tool == "plain":
        instr = Path(os.environ.get("STUDY_INSTRUCTIONS", "CLAUDE.md"))
        instr.write_text(instr.read_text() + f"\n## Backlog\n\n### {item} — fixture spec\n\n{body}\n")
    elif tool == "plans":
        Path("PLAN.md").write_text(f"# Plan\n\n## Items\n\n- [ ] **{item} — fixture spec**\n\n{body}\n\n## Handoff log\n")
    elif tool == "speckit":
        d = Path(f"specs/001-{item.lower()}-fixture")
        d.mkdir(parents=True)
        (d / "spec.md").write_text(f"# {title}\n\n{body}\n")
        (d / "plan.md").write_text(f"# Plan: {title}\n")
        (d / "tasks.md").write_text(f"# Tasks: {title}\n\n- [ ] T001 [US1] do {item}\n- [ ] T002 [US1] test {item}\n")
    elif tool == "beads":
        run(["bd", "create", title, "-d", body, "--acceptance", "- as the brief says", "--json"])
    elif tool == "openwarrant":
        run(["war", "create", title, "--summary", "fixture", "--body", body, "--item", "as the brief says", "--json"])
    print(json.dumps({"type": "result", "subtype": "success", "is_error": False, "num_turns": 0, "total_cost_usd": 0,
                      "result": f"wrote {tool}", "usage": {}}))


if __name__ == "__main__":
    if sys.argv[1] == "probe":
        probe()
    elif sys.argv[1] == "empty":
        print(json.dumps({"type": "result", "subtype": "success", "is_error": False, "num_turns": 0,
                          "total_cost_usd": 0, "result": "wrote nothing", "usage": {}}))
    else:
        write_spec(sys.argv[1])
