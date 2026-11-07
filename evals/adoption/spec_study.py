"""The spec study (PROTOCOL-spec.md): who writes the spec, and in which tool.

The adoption study's seeder writes one expert backlog into every tool. Here an author writes the
spec, so this module holds what that adds: the frozen brief, the author's prompt, the scripted
customer, a spec's capture in its tool's own form, its store by digest, laying a stored spec over a
fresh repository for a worker, the per-tool "is it done" reader for a spec nobody labelled, and the
pilot's plan and cost estimate. run.py runs it; nothing here starts a model.

A spec is stored as files under neutral paths, so that a spec written under Claude Code (CLAUDE.md,
skills) can be read by a worker under opencode (AGENTS.md, commands):

    INSTRUCTIONS.md                         plain: the whole instruction file
    PLAN.md                                 plans
    specs/**, .specify/memory/constitution.md   speckit
    beads.jsonl                             beads: `bd export`
    docs/tickets/**, docs/warrants/**       openwarrant
"""

from __future__ import annotations

import hashlib
import json
import re
import shutil
import socket
import subprocess
import threading
import time
import tomllib
from pathlib import Path

import arms

HERE = Path(__file__).resolve().parent
SPEC_DIR = HERE / "spec"

TOOLS = ("plain", "plans", "speckit", "beads", "openwarrant")
PILOT_ITEMS = (("tally", "T8"), ("knightlm", "K8"))
MODEL_AUTHORS = ("opus", "deepseek")
SPECS_PER_COMBINATION = 2
WORKERS_PER_SPEC = 2
WORKERS_PER_CONTROL = 4     # one fixed text, so 4 workers: the same 40 cells as a model author

# The harness each author runs under decides its instruction file and tool integration (`layout`).
AUTHORS = {
    "opus": {"agent": "claude", "model": "claude-opus-5-5", "layout": "claude"},
    "deepseek": {"agent": "opencode", "model": "lithosai/deepseek-ai/DeepSeek-V4.1-Flash", "layout": "opencode"},
}
WORKERS = {"deepseek": {"agent": "opencode", "model": "lithosai/deepseek-ai/DeepSeek-V4.1-Flash"}}

WORKER_TURNS = 60           # this study's cap (PROTOCOL-spec.md); the adoption study keeps its own
AUTHOR_TURNS = 60
AUTHOR_BUDGET_USD = 3.0     # --max-budget-usd per Opus author session
PILOT_CAP_USD = 30.0        # the owner's hard cap on API spend for the pilot

MAX_FILE_BYTES = 1 << 20
MAX_FILES = 500
CUSTOMER_PATH = "/opt/study/customer.sock"
NO_ANSWER = "The customer has nothing to add on that. Use your own judgement."

# USD per million tokens at list price, peak where there are peak hours. A price not known is None,
# so the estimate says UNKNOWN instead of inventing one. Opus 5.5: Anthropic's list, 2026-10-06.
# DeepSeek V4.1 Flash: DeepSeek's own endpoint, api-docs.deepseek.com/quick_start/pricing, 2026-11-06.
LIST_PRICES = {
    "opus": {"input": 4.0, "cache_read": 0.20, "output": 20.0},
    "deepseek": {"input": 0.30, "cache_read": 0.006, "output": 1.20},
    "glm": None,
}
# Measured: median of 30 DeepSeek directed sessions in oc-base-{1,2,3} and oc-rem-3, all at the
# 24-turn cap. At 60 turns input and output scale by 2.5, and cache reads by its square, since each
# turn re-reads a context that grows. This is an upper-end guess: it assumes a session uses all 60.
MEASURED_24_TURNS = {"input": 43000, "cache_read": 716000, "output": 9200}
OPUS_SPEC_GUESS_USD = (0.50, 1.50)  # PROTOCOL-spec.md's guess, until the pilot measures it


def session_at_60() -> dict:
    m = MEASURED_24_TURNS
    return {"input": m["input"] * 2.5, "cache_read": m["cache_read"] * 2.5 ** 2, "output": m["output"] * 2.5}


def list_price(who: str, tokens: dict) -> float | None:
    p = LIST_PRICES.get(who)
    if p is None:
        return None
    return sum(tokens.get(k, 0) * p[k] for k in p) / 1e6


# ------------------------------------------------------------------ briefs


def brief_path(item: str) -> Path:
    return SPEC_DIR / item / "BRIEF.md"


def brief_text(item: str) -> str:
    return brief_path(item).read_text()


def brief_digest(item: str) -> str:
    return hashlib.sha256(brief_path(item).read_bytes()).hexdigest()


def brief_problem(item: str) -> str | None:
    """None when the owner froze this exact brief. A real author never runs on anything else."""
    frozen = SPEC_DIR / item / "FROZEN"
    if not brief_path(item).exists():
        return f"{item}: no brief at {brief_path(item)}"
    if not frozen.exists():
        return f"{item}: the brief is not frozen: the owner reviews it, then records its sha256 in {frozen}"
    if frozen.read_text().split()[:1] != [brief_digest(item)]:
        return f"{item}: FROZEN does not match BRIEF.md ({brief_digest(item)[:12]}): the brief changed after review"
    return None


def contract_terms(item: str) -> list[str]:
    p = SPEC_DIR / item / "terms.txt"
    return [l.strip() for l in p.read_text().splitlines() if l.strip()] if p.exists() else []


def contract_carried(item: str, text: str) -> dict:
    """How much of the contract reached the spec: its exact terms, found verbatim."""
    flat = " ".join(text.split())
    terms = contract_terms(item)
    found = [t for t in terms if " ".join(t.split()) in flat]
    return {"terms": len(terms), "found": len(found), "missing": [t for t in terms if t not in found]}


# ------------------------------------------------------------------ the author's prompt


_TOOL_STEPS = {
    "plain": "Write the description in `{instr}`, under a heading `## Backlog`, as a section headed "
             "`### {id} — <a short title>`.",
    "plans": "Write it in `PLAN.md`: a section `## Items` holding an entry that starts "
             "`- [ ] **{id} — <a short title>**` with the plan under it, then an empty section `## Handoff log`.",
    "beads": "Record it as Beads issues with `bd create`: the title starts with `{id}:`, with a description, "
             "`--acceptance`, and `--design` where it helps. If you create more than one issue, set their "
             "dependencies.",
    "openwarrant": "Use OpenWarrant's own path: shape the work with `war plan`, or the `/war-spec` flow if you "
                   "have it, and end with a ticket: `war create`, with a title starting `{id}:`, `--summary`, "
                   "`--body`, and one `--item` per acceptance criterion. Nothing here needs a signature.",
}
_SPECKIT_STEPS = {
    "claude": "Use Spec Kit's own steps, in order: `/speckit-specify` with the work described above, whose feature "
              "title starts with `{id}`, then `/speckit-plan`, then `/speckit-tasks`.",
    "opencode": "Use Spec Kit's own steps, in order. An agent can't invoke a slash command, so do what each command "
                "file says, with the work described above as its input: `.opencode/commands/speckit.specify.md`, "
                "with a feature title starting `{id}`, then `speckit.plan.md`, then `speckit.tasks.md`.",
}


def author_prompt(tool: str, item: str, brief: str, instr: str, layout: str) -> str:
    step = (_SPECKIT_STEPS[layout] if tool == "speckit" else _TOOL_STEPS[tool]).format(id=item, instr=instr)
    return f"""You are writing the specification for one piece of work on this repository. You are not implementing it: don't change the project's code, tests or documentation.

Another agent will implement it later. That agent gets this repository and what you write, and nothing else: it won't see the brief below. Its whole instruction will be: `Do backlog item {item}.`

There is nobody at this terminal. The customer who asked for this answers questions through a shell command, one question per call: `ask-customer "your question"`. It answers only what is asked.

## The brief

{brief.strip()}

## How to write it down

{step}

Stop when the specification is written."""


# ------------------------------------------------------------------ the scripted customer


def customer_rule(item: str) -> dict:
    with open(SPEC_DIR / item / "customer.toml", "rb") as f:
        return tomllib.load(f)


def customer_matches(question: str, rule: dict) -> bool:
    """Scripted, so every author meets the same customer: each group needs one of its words."""
    q = " ".join(question.lower().replace("`", "").split())
    return all(any(w in q for w in group) for group in rule["all_of"])


class Customer:
    """Serves one author session over a unix socket bound into its sandbox. The answer stays on the
    host: an author reaches it by asking, and never by reading a file. What comes down the socket
    is data: it is matched, logged and otherwise ignored."""

    def __init__(self, sock: Path, item: dict, rule: dict, log: Path):
        self.sock, self.item, self.rule, self.log = sock, item, rule, log
        self.asked: list[dict] = []
        self._stop = threading.Event()
        sock.parent.mkdir(parents=True, exist_ok=True)
        self.server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        self.server.bind(str(sock))
        self.server.listen(8)
        self.server.settimeout(0.25)
        self.thread = threading.Thread(target=self._serve, daemon=True)
        self.thread.start()

    def _serve(self) -> None:
        while not self._stop.is_set():
            try:
                conn, _ = self.server.accept()
            except TimeoutError:
                continue
            except OSError:
                return
            with conn:
                conn.settimeout(5)
                try:
                    data = b""
                    while b"\n" not in data and len(data) < 8192:
                        chunk = conn.recv(4096)
                        if not chunk:
                            break
                        data += chunk
                    conn.sendall(self.reply(data.decode("utf-8", "replace").strip()).encode() + b"\n")
                except OSError:
                    pass

    def reply(self, question: str) -> str:
        matched = customer_matches(question, self.rule)
        entry = {"n": len(self.asked) + 1, "asked": question[:2000], "matched": matched}
        self.asked.append(entry)
        with open(self.log, "a") as f:
            f.write(json.dumps(entry) + "\n")
        return self.item["question"]["answer"].strip() if matched else NO_ANSWER

    def close(self) -> None:
        self._stop.set()
        self.thread.join(timeout=2)
        self.server.close()
        self.sock.unlink(missing_ok=True)


# ------------------------------------------------------------------ capture (inside the sandbox)


def capture_spec(tool: str, repo: Path, out: Path, env: dict) -> dict:
    """Copy the spec out of `repo` under its neutral paths. Runs in the sandbox, with no network.
    Only regular files are taken, so a planted symlink can't carry anything else out."""
    out.mkdir(parents=True, exist_ok=True)
    skipped: list[str] = []

    def take(src: Path, rel: str) -> None:
        if src.is_symlink() or not src.is_file():
            skipped.append(f"{rel}: not a regular file")
        elif src.stat().st_size > MAX_FILE_BYTES:
            skipped.append(f"{rel}: over {MAX_FILE_BYTES} bytes")
        else:
            dest = out / rel
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(src, dest, follow_symlinks=False)

    def tree(rel: str) -> None:
        root = repo / rel
        if root.is_symlink():
            skipped.append(f"{rel}: symlink")
        elif root.is_dir():
            for f in sorted(root.rglob("*")):
                if f.is_symlink() or f.is_file():
                    take(f, str(f.relative_to(repo)))

    if tool == "plain":
        instr = arms.instructions(repo, env)
        if instr.exists() or instr.is_symlink():
            take(instr, "INSTRUCTIONS.md")
    elif tool == "plans":
        if (repo / "PLAN.md").exists() or (repo / "PLAN.md").is_symlink():
            take(repo / "PLAN.md", "PLAN.md")
    elif tool == "speckit":
        tree("specs")
        if (repo / ".specify/memory/constitution.md").exists():
            take(repo / ".specify/memory/constitution.md", ".specify/memory/constitution.md")
    elif tool == "beads":
        p = subprocess.run([env["STUDY_BD"], "export", "-o", str(out / "beads.jsonl")], cwd=repo, env=env,
                           capture_output=True, text=True, timeout=120)
        if p.returncode != 0:
            skipped.append(f"bd export: exit {p.returncode}: {p.stderr[-300:]}")
    elif tool == "openwarrant":
        tree("docs/tickets")
        tree("docs/warrants")
    return {"skipped": skipped}


def changed_outside_spec(tool: str, repo: Path, base: str) -> list[str]:
    """What the author changed that is not its spec: an author is told not to, and this says if it did."""
    g = ["git", "-c", "core.hooksPath=/dev/null", "-c", "core.fsmonitor=false"]
    a = subprocess.run([*g, "diff", "--name-only", base], cwd=repo, capture_output=True, text=True, timeout=120)
    b = subprocess.run([*g, "ls-files", "-o", "--exclude-standard"], cwd=repo, capture_output=True, text=True, timeout=120)
    keep = {"plain": ("CLAUDE.md", "AGENTS.md"), "plans": ("PLAN.md",), "speckit": ("specs/", ".specify/"),
            "beads": (".beads/",), "openwarrant": ("docs/tickets/", "docs/warrants/", "docs/plan", ".openwarrant")}[tool]
    return sorted(f for f in set(a.stdout.split("\n")) | set(b.stdout.split("\n"))
                  if f and not f.startswith(keep))


# ------------------------------------------------------------------ the store


def _file_digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def manifest(root: Path, tool: str) -> tuple[dict, list[str]]:
    """Regular files under `root`, as the manifest that defines a spec's digest. The caller's `root`
    was written by a sandbox: nothing in it is followed, opened as anything but bytes, or run."""
    files, refused = {}, []
    for f in sorted(root.rglob("*")):
        rel = str(f.relative_to(root))
        if f.is_symlink():
            refused.append(f"{rel}: symlink")
        elif f.is_dir():
            continue
        elif not f.is_file():
            refused.append(f"{rel}: not a regular file")
        elif f.stat().st_size > MAX_FILE_BYTES:
            refused.append(f"{rel}: too large")
        elif len(files) >= MAX_FILES:
            refused.append(f"{rel}: over {MAX_FILES} files")
        else:
            files[rel] = _file_digest(f)
    return {"schema": "ow-adoption/spec/v1", "tool": tool, "files": files}, refused


def digest_of(m: dict) -> str:
    return hashlib.sha256(json.dumps(m, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def store_spec(store: Path, captured: Path, tool: str) -> tuple[str, list[str]]:
    m, refused = manifest(captured, tool)
    digest = digest_of(m)
    dest = store / digest
    if dest.exists():
        verify_stored(store, digest)
        return digest, refused
    tmp = store / f".tmp-{digest}-{threading.get_ident()}"
    for rel in m["files"]:
        (tmp / "files" / rel).parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(captured / rel, tmp / "files" / rel, follow_symlinks=False)
    (tmp / "files").mkdir(parents=True, exist_ok=True)
    (tmp / "manifest.json").write_text(json.dumps(m, indent=1, sort_keys=True) + "\n")
    try:
        tmp.rename(dest)
    except OSError:     # another session stored the same bytes first
        shutil.rmtree(tmp, ignore_errors=True)
    return digest, refused


def verify_stored(store: Path, digest: str) -> dict:
    """A worker runs what the digest names, or nothing."""
    dest = store / digest
    if not (dest / "manifest.json").exists():
        raise SystemExit(f"no spec {digest} in {store}")
    stored = json.loads((dest / "manifest.json").read_text())
    m, refused = manifest(dest / "files", stored["tool"])
    if refused or digest_of(m) != digest:
        raise SystemExit(f"spec {digest} in {store} does not match its digest: refused")
    return stored


def spec_stats(item: str, root: Path, baseline: Path | None) -> dict:
    """Spec size (bytes the author added) and how much of the contract it carried. `root` holds
    captured files; a journal is bookkeeping, not spec. Read as bytes, and decoded as text only."""
    texts, size = [], 0
    for f in sorted(p for p in root.rglob("*") if p.is_file() and not p.is_symlink()):
        if f.name == "journal.jsonl":
            continue
        rel = f.relative_to(root)
        data = f.read_bytes()
        was = (baseline / rel).read_bytes() if baseline and (baseline / rel).is_file() else b""
        if data == was:
            continue
        size += max(0, len(data) - len(was))
        text = data.decode("utf-8", "replace")
        if f.name == "beads.jsonl":
            rows = []
            for line in text.splitlines():
                try:
                    rows.append(json.loads(line))
                except ValueError:
                    continue
            text = "\n".join(str(r.get(k) or "") for r in rows if isinstance(r, dict)
                             for k in ("title", "description", "design", "acceptance_criteria", "notes"))
        texts.append(text)
    joined = "\n".join(texts)
    return {"spec_bytes": size, "empty": size == 0, "contract": contract_carried(item, joined),
            "names_a_model": sorted(set(m.lower() for m in re.findall(r"(?i)\b(claude|opus|deepseek|sonnet)\b", joined)))}


# ------------------------------------------------------------------ laying a spec over a repository


def apply_spec(tool: str, repo: Path, files: Path, env: dict) -> dict:
    """A stored spec in the worker's repository. Runs in the sandbox, over `arms.scaffold`'s result."""

    def lay(rel: str) -> None:
        src = files / rel
        if src.is_dir():
            for f in sorted(src.rglob("*")):
                if f.is_file() and not f.is_symlink():
                    lay(str(f.relative_to(files)))
        elif src.is_file() and not src.is_symlink():
            dest = repo / rel
            dest.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(src, dest, follow_symlinks=False)

    out: dict = {"tool": tool}
    instr = arms.instructions(repo, env)
    if tool == "plain":
        if (files / "INSTRUCTIONS.md").is_file():
            shutil.copyfile(files / "INSTRUCTIONS.md", instr, follow_symlinks=False)
    elif tool == "plans":
        lay("PLAN.md")
        with open(instr, "a") as f:
            f.write("\n" + arms.PLANS_POINTER)
    elif tool == "speckit":
        lay("specs")
        lay(".specify/memory/constitution.md")
        dirs = sorted(p.name for p in (repo / "specs").iterdir() if p.is_dir()) if (repo / "specs").is_dir() else []
        where = f"specs/{dirs[0]}/" if len(dirs) == 1 else "specs/"
        with open(instr, "a") as f:
            f.write("\n" + arms.speckit_pointer(env).replace("specs/001-backlog/", where))
        if len(dirs) == 1:
            (repo / ".specify" / "feature.json").write_text(
                json.dumps({"feature_directory": f"specs/{dirs[0]}"}, indent=2) + "\n")
        out["feature_dirs"] = dirs
    elif tool == "beads":
        if (files / "beads.jsonl").is_file() and (files / "beads.jsonl").stat().st_size:
            p = subprocess.run([env["STUDY_BD"], "import", str(files / "beads.jsonl")], cwd=repo, env=env,
                               capture_output=True, text=True, timeout=120)
            if p.returncode != 0:
                raise RuntimeError(f"bd import failed: {p.stderr[-400:]}")
    elif tool == "openwarrant":
        lay("docs/tickets")
        lay("docs/warrants")
    return out


# ------------------------------------------------------------------ "done", for a spec nobody labelled


_BOX = re.compile(r"^\s*- \[([ xX])\]")
_TASK = re.compile(r"^- \[([ xX])\] T\d+\b")


def spec_done(tool: str, repo: Path, env: dict) -> bool | None:
    """Does the tool's own record say the work is done? The adoption study's readers find items by the
    labels its seeder wrote. An author writes its own records, so here done means: the tool holds at
    least one record, and all of them are finished. None when the tool has no such record."""
    if tool == "plain":
        return None
    if tool == "plans":
        p = repo / "PLAN.md"
        boxes = [m.group(1) for l in (p.read_text(errors="replace").splitlines() if p.exists() else [])
                 if (m := _BOX.match(l))]
    elif tool == "speckit":
        boxes = [m.group(1) for f in sorted((repo / "specs").glob("*/tasks.md"))
                 for l in f.read_text(errors="replace").splitlines() if (m := _TASK.match(l))]
    elif tool == "beads":
        p = subprocess.run([env["STUDY_BD"], "list", "--json", "--all"], cwd=repo, env=env,
                           capture_output=True, text=True, timeout=120)
        if p.returncode != 0:
            return None
        rows = json.loads(p.stdout or "[]")
        return bool(rows) and all(r.get("status") == "closed" for r in rows)
    elif tool == "openwarrant":
        p = subprocess.run([env["STUDY_WAR"], "view", "tickets", "--json"], cwd=repo, env=env,
                           capture_output=True, text=True, timeout=120)
        try:
            rows = json.loads(p.stdout)["result"]
        except (ValueError, KeyError, TypeError):
            return None
        rows = (rows.get("tickets", rows) if isinstance(rows, dict) else rows) or []
        return bool(rows) and all(t.get("total") and t.get("done") == t.get("total") for t in rows)
    else:
        return None
    return bool(boxes) and all(b in "xX" for b in boxes)


# ------------------------------------------------------------------ the sufficiency check


def hidden_assertions(hidden: Path) -> dict[str, int]:
    """Each top-level function of the hidden tests that asserts, and how many `assert` statements."""
    import ast
    out: dict[str, int] = {}
    for f in sorted(hidden.glob("*.py")):
        for node in ast.parse(f.read_text()).body:
            if isinstance(node, ast.FunctionDef):
                n = sum(isinstance(x, ast.Assert) for x in ast.walk(node))
                if n:
                    out[node.name] = n
    return out


_ROW = re.compile(r"^\|\s*`?([A-Za-z_][\w]*)\.(\d+)`?\s*\|(.*)\|\s*$")


def check_sufficiency(item: str, hidden: Path) -> dict:
    """Mechanical half of the check. Every assertion has a row, every row cites clauses that exist,
    every clause is needed by some row, and the brief doesn't give away what the customer holds.
    Whether a clause really determines its assertion is the reader's half: this can't say."""
    brief = brief_text(item)
    table = (SPEC_DIR / item / "SUFFICIENCY.md").read_text()
    clauses = set(re.findall(r"\*\*(C\d+)\.\*\*", brief))
    want = hidden_assertions(hidden)
    rows: dict[str, list] = {}
    counts = {"contract": 0, "customer": 0, "exclusion": 0}
    problems, used = [], set()
    for line in table.splitlines():
        m = _ROW.match(line)
        if not m:
            continue
        name, n, cols = m.group(1), m.group(2), [c.strip() for c in m.group(3).split("|")]
        status = cols[-1].strip("` ")
        cited = set(re.findall(r"C\d+", cols[-2]))
        rows.setdefault(name, []).append(n)
        if status not in counts:
            problems.append(f"{name}.{n}: status {status!r}")
            continue
        counts[status] += 1
        used |= cited
        if status != "exclusion" and not cited:
            problems.append(f"{name}.{n}: {status} but cites no clause")
        if cited - clauses:
            problems.append(f"{name}.{n}: cites {sorted(cited - clauses)}, not in the brief")
    for name, n in want.items():
        if len(rows.get(name, [])) != n:
            problems.append(f"{name}: {n} assertions, {len(rows.get(name, []))} rows")
    for name in set(rows) - set(want):
        problems.append(f"{name}: in the table, no such assertion")
    for c in sorted(clauses - used):
        problems.append(f"{c}: no assertion needs it, so the contract isn't minimal")
    rule = customer_rule(item)
    low = brief.lower()
    for phrase in rule.get("must_not_say", []):
        if phrase.lower() in low:
            problems.append(f"the brief says {phrase!r}, which only the customer may give")
    return {"rows": sum(len(v) for v in rows.values()), "assertions": sum(want.values()), "counts": counts,
            "clauses": sorted(clauses), "problems": problems}


# ------------------------------------------------------------------ the pilot's plan


def spec_slots() -> list[dict]:
    """The specs to write. Replicate first, so a budget stop leaves every author and tool even."""
    slots = []
    for n in range(1, SPECS_PER_COMBINATION + 1):
        for repo, item in PILOT_ITEMS:
            for tool in TOOLS:
                for author in MODEL_AUTHORS:
                    slots.append({"slot": f"{author}-{tool}-{item}-{n}", "author": author, "tool": tool,
                                  "repo": repo, "item": item, "n": n})
    return slots


def worker_cells(worker: str = "deepseek") -> list[dict]:
    cells = []
    for s in spec_slots():
        for rep in range(1, WORKERS_PER_SPEC + 1):
            cells.append({"scenario": "spec", "repo": s["repo"], "arm": s["tool"], "item": s["item"], "rep": rep,
                          "author": s["author"], "spec": s["slot"], "spec_n": s["n"], "worker": worker})
    for repo, item in PILOT_ITEMS:
        for tool in TOOLS:
            for rep in range(1, WORKERS_PER_CONTROL + 1):
                cells.append({"scenario": "spec", "repo": repo, "arm": tool, "item": item, "rep": rep,
                              "author": "backlog", "spec": None, "spec_n": 0, "worker": worker})
    cells.sort(key=lambda c: (c["rep"], c["spec_n"], c["item"], c["author"], TOOLS.index(c["arm"])))
    return cells


def estimate(slots: list[dict], cells: list[dict]) -> dict:
    opus = sum(1 for s in slots if s["author"] == "opus")
    deepseek = sum(1 for s in slots if s["author"] == "deepseek")
    session = list_price("deepseek", session_at_60())
    low, high = OPUS_SPEC_GUESS_USD
    return {
        "opus_specs": opus, "deepseek_specs": deepseek, "worker_cells": len(cells),
        "cash_usd": [round(opus * low, 2), round(opus * high, 2)],
        "cap_usd": PILOT_CAP_USD,
        "author_session_cap_usd": AUTHOR_BUDGET_USD,
        "worst_case_cash_usd": PILOT_CAP_USD,
        "list_price_deepseek_session_usd": round(session, 3),
        "list_price_deepseek_author_usd": round(deepseek * session, 2),
        "list_price_workers_usd": round(len(cells) * session, 2),
        "list_price_total_usd": [round(opus * low + (deepseek + len(cells)) * session, 2),
                                 round(opus * high + (deepseek + len(cells)) * session, 2)],
        "worker_turn_cap": WORKER_TURNS,
    }
