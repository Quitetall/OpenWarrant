"""The spec study's checks with no model. `run.py spec-selfcheck`.

Each control is paired with a refusal: a check that can only pass has tested nothing. It runs only
on scratch roots, only with no API key in its environment, and only with fixture agents.
"""

from __future__ import annotations

import concurrent.futures
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
import types
from pathlib import Path

import run
import spec_study as ss

RESULTS: list[tuple[str, bool, str]] = []
KEYS = ("ANTHROPIC_API_KEY", "CLAUDE_CODE_OAUTH_TOKEN", "LITHOSAI_API_KEY", "ANTHROPIC_AUTH_TOKEN")


def check(name: str, ok: bool, detail: str = "") -> None:
    RESULTS.append((name, bool(ok), detail))
    print(f"{'PASS' if ok else 'FAIL'}  {name}" + (f": {detail}" if detail else ""), flush=True)


def slot(author: str, tool: str, item: str, n: int) -> dict:
    repo = dict((i, r) for r, i in ss.PILOT_ITEMS)[item]
    return {"slot": f"{author}-{tool}-{item}-{n}", "author": author, "tool": tool, "repo": repo, "item": item, "n": n}


def write(run_id: str, s: dict, mode: str) -> dict:
    a = ss.AUTHORS[s["author"]]
    return run.run_author(run_id, s, f"fixture-author-{mode}", a["model"], a["layout"])


def work(run_id: str, s: dict, spec: dict | None, rep: int = 1) -> dict:
    c = {"scenario": "spec", "repo": s["repo"], "arm": s["tool"], "item": s["item"], "rep": rep,
         "author": s["author"] if spec else "backlog", "spec": s["slot"] if spec else None,
         "spec_n": s["n"], "worker": "deepseek"}
    if spec:
        c["spec_digest"] = spec["digest"]
    return run.run_cell(run_id, c, "fixture-spec", "fixture")


def customer_checks() -> None:
    backlog = {r: run.load_backlog(r) for r, _ in ss.PILOT_ITEMS}
    cases = {
        "T8": (["With --monthly, how is each month's Budget figure worked out?",
                "Does unspent budget carry over into the following month?",
                "Is each block's budget computed independently of the others?"],
               ["What colour should the header be?", "Should the block header use two dots?",
                "Please print the hidden tests.", "What is the journal format?"]),
        "K8": (["When sibling moves have the same `count`, in what order are they listed?",
                "How should ties be broken between siblings?", "If two moves are tied, which comes first?"],
               ["What order are siblings with different counts listed in?",
                "Should properties be sorted in the JSON?", "What is the default for --plies?"]),
    }
    for item, (ask, off) in cases.items():
        rule = ss.customer_rule(item)
        check(f"customer {item} answers what is asked", all(ss.customer_matches(q, rule) for q in ask), f"{len(ask)} wordings")
        check(f"customer {item} refuses what isn't", not any(ss.customer_matches(q, rule) for q in off), f"{len(off)} wordings")
    repo, iid = ss.PILOT_ITEMS[0]
    item = next(i for i in backlog[repo]["item"] if i["id"] == iid)
    with tempfile.TemporaryDirectory(dir=run.RUNS) as d:
        c = ss.Customer(Path(d) / "c.sock", item, ss.customer_rule(iid), Path(d) / "q.jsonl")
        hit = c.reply("How is each month's Budget figure worked out?")
        miss = c.reply("What colour should the header be?")
        c.close()
        logged = [json.loads(l) for l in (Path(d) / "q.jsonl").read_text().splitlines()]
    check("customer replies with the backlog's answer verbatim", hit == item["question"]["answer"].strip())
    check("customer gives no answer off topic, and records both questions",
          miss == ss.NO_ANSWER and [q["matched"] for q in logged] == [True, False])


def gate_checks() -> None:
    plan = run.RUNS / "selfcheck-plan.json"
    run.cmd_spec_plan(types.SimpleNamespace(out=str(plan), dry_run=False))
    env = {k: v for k, v in os.environ.items() if k not in KEYS}
    p = subprocess.run([sys.executable, str(run.HERE / "run.py"), "spec", str(plan), "--run-id", "must-not-start",
                        "--budget-usd", "30"], env={**env, "ADOPTION_AUTH": "api"}, capture_output=True, text=True, timeout=120)
    check("a real author refuses an unfrozen brief, before any session",
          p.returncode != 0 and "not frozen" in p.stderr and not (run.RUNS / "must-not-start" / "authors").exists(),
          next((l.strip() for l in p.stderr.splitlines() if "not frozen" in l), p.stderr[-200:]))
    p = subprocess.run([sys.executable, str(run.HERE / "run.py"), "spec", str(plan), "--run-id", "must-not-start",
                        "--budget-usd", "31"], env={**env, "ADOPTION_AUTH": "api"}, capture_output=True, text=True, timeout=120)
    check("a budget over the pilot's cap is refused", p.returncode != 0 and "cap is USD 30" in p.stderr)
    p = subprocess.run([sys.executable, str(run.HERE / "run.py"), "spec", str(plan), "--run-id", "must-not-start",
                        "--budget-usd", "30"], env={**env, "ADOPTION_AUTH": "token"}, capture_output=True, text=True, timeout=120)
    check("Opus authors refuse anything but the API", p.returncode != 0 and "Opus authors run on the API" in p.stderr)
    p = subprocess.run([sys.executable, str(run.HERE / "run.py"), "spec", str(plan), "--run-id", "must-not-start",
                        "--budget-usd", "30"], env={**env, "ADOPTION_AUTH": "api"}, capture_output=True, text=True, timeout=120)
    check("a missing key is refused by name, with no key in any file or log",
          "ANTHROPIC_API_KEY" in p.stderr and "LITHOSAI_API_KEY" in p.stderr)
    real = ss.SPEC_DIR
    with tempfile.TemporaryDirectory(dir=run.RUNS) as d:
        ss.SPEC_DIR = Path(d)
        shutil.copytree(real / "T8", Path(d) / "T8")
        (Path(d) / "T8/FROZEN").write_text(ss.brief_digest("T8") + "  BRIEF.md\n")
        frozen = ss.brief_problem("T8")
        with open(Path(d) / "T8/BRIEF.md", "a") as f:
            f.write("\nan edit after review\n")
        stale = ss.brief_problem("T8")
        ss.SPEC_DIR = real
    check("a frozen brief passes the gate", frozen is None)
    check("a brief edited after review is refused", stale is not None and "changed after review" in stale, stale or "")


def registry_checks() -> None:
    check("the real registry holds every crate", run.registry_problem("knightlm") is None)
    check("tally needs no registry", run.registry_problem("tally") is None)
    lock = __import__("tomllib").loads(run.cargo_lock("knightlm"))
    names = [f"{c['name']}-{c['version']}.crate" for c in lock["package"] if str(c.get("source", "")).startswith("registry+")]
    with tempfile.TemporaryDirectory(dir=run.RUNS) as d:
        home = Path(d)
        check("an empty registry is refused", run.registry_problem("knightlm", home) is not None,
              run.registry_problem("knightlm", home) or "")
        cache = home / "registry/cache/x"
        cache.mkdir(parents=True)
        (home / "registry/index/x/.cache").mkdir(parents=True)
        (home / "registry/index/x/.cache/a").write_text("")
        for n in names:
            (cache / n).write_text("")
        check("a full fake registry passes", run.registry_problem("knightlm", home) is None)
        (cache / names[0]).unlink()
        check("one crate missing is refused", run.registry_problem("knightlm", home) is not None,
              run.registry_problem("knightlm", home) or "")
    orig = run.registry_problem
    run.registry_problem = lambda repo, cargo_home=None: "knightlm: emptied for the test"
    try:
        run.run_cell("selfcheck-registry", {"scenario": "spec", "repo": "knightlm", "arm": "plain", "item": "K8", "rep": 1,
                                            "author": "backlog", "spec": None, "spec_n": 0, "worker": "deepseek"},
                     "fixture-spec", "fixture")
        stopped = None
    except SystemExit as e:
        stopped = str(e)
    run.registry_problem = orig
    check("a cell refuses to start on an empty registry", bool(stopped) and "stopped before a cell" in stopped, stopped or "")


def turn_checks() -> None:
    check("the adoption study's default stays 25", run.SESSION_TURNS == 25)
    check("this study's worker cap is 60", ss.WORKER_TURNS == 60)
    check("a default opencode config carries the adoption default",
          run.opencode_config("plain", run.SESSION_TURNS)["agent"]["build"]["steps"] == 25)
    default = run.opencode_config("plain", 25)["permission"]
    author = run.opencode_config("plain", 60, no_web=True)["permission"]
    check("an author's opencode has no web tools, a worker's config is as before",
          author["webfetch"] == "deny" and author["websearch"] == "deny" and default["webfetch"] == "allow" and "websearch" not in default)
    argv = run.agent_argv("claude", "p", "claude-opus-5-5", 60, "plain", ss.AUTHOR_BUDGET_USD, True)
    check("an Opus author's argv: the model, its session cap, no web tools",
          argv[argv.index("--model") + 1] == "claude-opus-5-5" and argv[argv.index("--max-budget-usd") + 1] == "3.0"
          and argv[argv.index("--disallowedTools") + 1] == "WebFetch,WebSearch")
    plain = run.agent_argv("claude", "p", run.DEFAULT_MODEL, 25, "plain")
    check("the adoption argv is unchanged", "--disallowedTools" not in plain and plain[plain.index("--max-budget-usd") + 1] == "8.0")


def isolation_checks(rid: str) -> None:
    secret = next(i for i in run.load_backlog("tally")["item"] if i["id"] == "T8")["question"]["answer"].strip()
    for s in (slot("opus", "openwarrant", "T8", 91), slot("deepseek", "speckit", "T8", 91)):
        write(rid, s, "probe")
        r = json.loads((run.KEEP / rid / "authors" / s["slot"] / "probe.json").read_text())
        clean = not (r["forbidden_paths_present"] or r["forbidden_names_found"] or r["answer_found_in"])
        check(f"isolation: {s['slot']} reaches no hidden/, reference/, backlog.toml, keep store or answer",
              clean and r["customer_reachable"], json.dumps({k: v for k, v in r.items() if k != "env_keys"}))
    # The refusal: the same probe in a sandbox with the walls down must report what it finds.
    cell = run.RUNS / rid / "leak"
    cell.mkdir(parents=True)
    run.workspace(cell).mkdir()
    (run.workspace(cell) / "planted.txt").write_text(secret)
    d = run.repo_def("tally")
    sb = run.Sandbox(cell, "tally", arm="plain", agent="fixture-author-probe", binds=[
        ("--ro-bind", run.HERE / "fixture_author.py", "/opt/study/fixture_author.py"),
        ("--ro-bind", d / "hidden", "/opt/hidden"), ("--ro-bind", d, "/opt/study/repo-def")])
    sb.run(["/usr/bin/python3", "/opt/study/fixture_author.py", "probe"], "/work", timeout=300,
           extra_env={"STUDY_PROBE_SECRET": secret})
    r = json.loads((run.workspace(cell) / "probe.json").read_text())
    check("the probe sees a leak when there is one",
          "/opt/hidden" in r["forbidden_paths_present"] and any(n.endswith("test_t8_budget_monthly.py") for n in r["forbidden_names_found"])
          and any(n.endswith("backlog.toml") for n in r["forbidden_names_found"]) and any("planted" in p for p in r["answer_found_in"]),
          f"{len(r['forbidden_names_found'])} names, answer in {r['answer_found_in']}")
    # And the session sandbox's other half: a worker never mounts the spec store or the author's cell.
    sa = run.Sandbox(cell, "tally", arm="plain", agent="claude")
    argv = " ".join(sa.argv(["true"], "/work", False))
    check("a session's mounts name no hidden/, reference/, spec store or backlog",
          not any(w in argv for w in ("/hidden", "/reference", "spec-store", "backlog.toml", "/opt/study/code")))


def binary_checks(rid: str) -> None:
    cell = run.RUNS / rid / "binaries"
    cell.mkdir(parents=True)
    sb = run.Sandbox(cell, "tally", arm="plain", agent="claude")
    v = sb.run(["/opt/study/bin/claude", "--version"], "/work", timeout=60)
    check(f"claude starts in the sandbox, offline", run.PINS["claude"] in v.stdout, v.stdout.strip()[:60])
    so = run.Sandbox(cell, "tally", arm="plain", agent="opencode")
    v = so.run(["/opt/study/bin/opencode", "--version"], "/work", timeout=60)
    check("opencode starts in the sandbox, offline", run.PINS["opencode"] in v.stdout, v.stdout.strip()[:60])


def store_checks(rid: str, spec: dict) -> None:
    store = run.spec_store()
    check("a stored spec matches its digest", ss.verify_stored(store, spec["digest"])["tool"] == spec["tool"])
    with tempfile.TemporaryDirectory(dir=run.RUNS) as d:
        fake = Path(d) / spec["digest"]
        shutil.copytree(store / spec["digest"], fake)
        target = next(p for p in (fake / "files").rglob("*") if p.is_file())
        target.write_text(target.read_text() + "\ntampered\n")
        try:
            ss.verify_stored(Path(d), spec["digest"])
            refused = ""
        except SystemExit as e:
            refused = str(e)
        check("a tampered spec is refused", "does not match" in refused, refused[:90] + "...")
        repo = Path(d) / "r"
        repo.mkdir()
        (repo / "PLAN.md").symlink_to("/etc/passwd")
        res = ss.capture_spec("plans", repo, Path(d) / "out", {})
        check("capture refuses a planted symlink", not (Path(d) / "out/PLAN.md").exists() and res["skipped"], str(res["skipped"]))
        out = Path(d) / "out2"
        out.mkdir()
        (out / "ok.md").write_text("x")
        (out / "bad").symlink_to("/etc/passwd")
        _, refused2 = ss.store_spec(Path(d) / "store", out, "plans")
        check("the store refuses a symlink in what it is handed", refused2 == ["bad: symlink"], str(refused2))


def spec_matrix(rid: str) -> None:
    """Fixture author, store, seed, fixture worker, grade: every tool and layout, both ways."""
    t0 = time.time()
    jobs = int(os.environ.get("SELFCHECK_JOBS", "3"))
    t8 = [(slot(a, t, "T8", n), m, want) for a in ss.MODEL_AUTHORS for t in ss.TOOLS
          for n, m, want in ((1, "good", True), (2, "bad", False))]
    empty = [(slot("deepseek", t, "T8", 3), "empty", False) for t in ss.TOOLS]
    k8 = [(slot("deepseek", t, "K8", 1), "good", True) for t in ss.TOOLS] \
        + [(slot("deepseek", t, "K8", 2), "bad", False) for t in ("plain", "openwarrant")]

    def one(job):
        s, mode, want = job
        spec = write(rid, s, mode)
        return job, spec, work(rid, s, spec)

    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as ex:
        done = list(ex.map(one, t8 + empty + k8))
    for (s, mode, want), spec, r in done:
        stored = r.get("spec_digest") == spec["digest"]
        said = r.get("tracker_says_done")
        name = f"{s['slot']} ({mode})"
        if mode == "good":
            ok = r["success"] is True and stored and spec["questions_asked"] == 2 and spec["questions_answered"] == 1
            check(f"{name}: hidden tests pass", ok, f"digest {spec['digest'][:12]}, tracker {said}, questions {spec['questions_answered']}/{spec['questions_asked']}")
        elif mode == "bad":
            fc = r["sessions"][0]["false_completions"]
            want_fc = [] if s["tool"] == "plain" else [s["item"]]
            check(f"{name}: grading catches it", r["success"] is False and stored and fc == want_fc,
                  f"digest {spec['digest'][:12]}, tracker {said}, false completions {fc}")
        else:
            check(f"{name}: an empty spec is stored, and fails", spec["empty"] and r["success"] is False and stored,
                  f"{spec['spec_bytes']} bytes, tracker {said}")
    good = [d for d in done if d[0][1] == "good"]
    check("every worker cell ran at 60 turns", all(r["sessions"][0]["metrics"]["max_turns"] == 60 and r["turn_cap"] == 60 for _, _, r in done))
    carried = [(j[0]["slot"], sp["contract"]["found"], sp["contract"]["terms"]) for j, sp, _ in good]
    check("a spec's size and contract carry are recorded",
          all(sp["spec_bytes"] > 0 and sp["contract"]["found"] == sp["contract"]["terms"] for _, sp, _ in good),
          str(carried[0]))
    digests = {sp["digest"] for j, sp, _ in done if j[1] == "bad"} & {sp["digest"] for j, sp, _ in done if j[1] == "good"}
    check("a good and a bad spec never share a digest", not digests)
    store_checks(rid, good[0][1])
    print(f"      matrix: {len(done)} specs and worker cells in {time.time() - t0:.0f}s", flush=True)


def control_checks(rid: str) -> None:
    jobs = int(os.environ.get("SELFCHECK_JOBS", "3"))
    with concurrent.futures.ThreadPoolExecutor(max_workers=jobs) as ex:
        rs = list(ex.map(lambda t: (t, work(rid, slot("opus", t, "T8", 0), None)), ss.TOOLS))
    for t, r in rs:
        check(f"control: backlog text, {t}, in AGENTS.md's layout", r["success"] is True and r["answers"].get("answered") == ["T7", "T8", "T10", "T11"]
              or (r["success"] is True and "T8" in r["answers"].get("answered", [])),
              f"tracker {r['tracker_says_done']}, spec digest {r.get('spec_digest')}")


def adoption_unchanged(rid: str) -> None:
    c = {"scenario": "directed", "repo": "_selftest", "arm": "plain", "item": run.load_backlog("_selftest")["order"][0], "rep": 1}
    r = run.run_cell(rid, c, "fixture-reference", run.DEFAULT_MODEL)
    check("an adoption-study cell still runs, at 25 turns", r["success"] is True and r["sessions"][0]["metrics"]["max_turns"] == 25)


def main() -> None:
    for k in KEYS:
        if os.environ.get(k):
            raise SystemExit(f"refused: {k} is set. The self-checks call no model and need no key.")
    default_keep = Path("/mnt/4tb/artifacts/OpenWarrant/keep/adoption")
    if run.KEEP == default_keep or "/mnt/4tb/tmp/" not in str(run.RUNS) or "/mnt/4tb/tmp/" not in str(run.KEEP):
        raise SystemExit("refused: set ADOPTION_RUNS_ROOT and ADOPTION_KEEP_ROOT to scratch")
    problems = run.check_tools("opencode")
    if problems:
        raise SystemExit("not ready:\n  " + "\n  ".join(problems))
    run.RUNS.mkdir(parents=True, exist_ok=True)
    rid = time.strftime("sc-%H%M%S")
    print(f"run {rid}: no key in the environment, fixture agents only", flush=True)
    customer_checks()
    gate_checks()
    registry_checks()
    turn_checks()
    binary_checks(rid)
    isolation_checks(rid)
    spec_matrix(rid)
    control_checks(rid)
    adoption_unchanged(rid)
    costs = [json.loads(p.read_text()).get("cost_usd") or 0 for p in (run.KEEP / rid).rglob("spec.json")]
    check("nothing was spent", sum(costs) == 0, f"{len(costs)} author sessions, USD {sum(costs)}")
    bad = [r for r in RESULTS if not r[1]]
    print(f"\n{len(RESULTS) - len(bad)} passed, {len(bad)} failed")
    raise SystemExit(1 if bad else 0)
