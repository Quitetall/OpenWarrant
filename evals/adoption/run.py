#!/usr/bin/env python3
"""OpenWarrant adoption study: the harness. PROTOCOL.md is the design; this runs it.

    run.py prepare                       pin and check the tools, fetch crates, warm caches
    run.py validate <repo>               the backlog invariants (BACKLOG-FORMAT.md)
    run.py seed <repo> <arm> <dest> [--done ID ...]
                                         one seeded repository, to read what an arm's agent sees
    run.py parity <repo>                 seed every arm and check each holds every item's words
    run.py plan [--smoke|--pilot] [--out FILE]
                                         the cells of the study, with session counts
    run.py execute <plan> --run-id ID --agent claude|fixture-reference|fixture-noop
               [--jobs N] [--budget-usd X] [--model M]
                                         run the cells (resumable: a finished cell is skipped)
    run.py report <run-id>               results.json and REPORT.md from a run

Every agent session, and everything that touches a repository after an agent
has (snapshot commits, merges, tracker reads, grading), runs inside a
bubblewrap sandbox that sees only that cell's directory, the toolchain and the
pinned tools; never the owner's home, other repositories, this study's hidden
tests or reference solutions (fixture agents excepted, by design).
"""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import threading
import time
import tomllib
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
import arms  # noqa: E402
import jira_arm  # noqa: E402

STUDY = Path(os.environ.get("ADOPTION_STUDY_ROOT", "/mnt/4tb/tmp/openwarrant-adoption"))
KEEP = Path(os.environ.get("ADOPTION_KEEP_ROOT", "/mnt/4tb/artifacts/OpenWarrant/keep/adoption"))
# Where cells' working trees live while they run. Default under STUDY; ADOPTION_RUNS_ROOT=/dev/shm/...
# puts them in memory, away from a contended disk (Beads' embedded database syncs on every write).
RUNS = Path(os.environ.get("ADOPTION_RUNS_ROOT", str(STUDY / "runs")))
HOME = Path.home()

PINS = {
    "claude": "2.1.301",
    "war": "1.0.0-alpha.5",
    "bd": "1.3.1",
    "specify": "v1.2.2",
    "rust": "1.97.1",
    "pytest": "9.0.2",
    "ow_plugin": "v1.0.0-alpha.5",   # the OpenWarrant Claude Code plugin, from this tag
}
OW_REPO = Path(os.environ.get("ADOPTION_OW_REPO", "/mnt/4tb/OpenWarrant"))
PLUGIN_FILES = (".claude-plugin", ".claude/skills", ".claude/hooks", ".mcp.json")
DEFAULT_MODEL = "claude-sonnet-5-5"
SESSION_TURNS = 25          # a session's turn cap (80 in the pilot; tightened so overhead costs something)
INTERRUPT_TURNS = 5         # relay session 2 is cut here, mid-item: the next session must resume it
SESSION_TIMEOUT = 45 * 60   # wall clock per session
SESSION_BUDGET_USD = 8.0    # --max-budget-usd per session (API-equivalent)

PROMPTS = {
    "directed": "Do backlog item {id}: {title}.",
    "relay": ("Continue this repository's planned work: take the next item that is ready, "
              "finish it, and stop when that one item is done."),
    "parallel": ("Continue this repository's planned work: take the next item that is ready, "
                 "finish it, and stop when that one item is done. Other agents are working on "
                 "this repository at the same time, each in its own git worktree."),
}
SCENARIOS = ("directed", "relay", "parallel")


# ------------------------------------------------------------------ tools


def tools() -> dict:
    t = STUDY / "tools"
    return {
        # ADOPTION_WAR: a local build to measure instead of the pinned release.
        "STUDY_WAR": os.environ.get("ADOPTION_WAR")
        or str(HOME / ".local/lib/openwarrant" / PINS["war"] / "bin/war"),
        "STUDY_BD": str(t / "beads/bd"),
        "STUDY_SPECIFY": str(t / "bin/specify"),
        "STUDY_CLAUDE": str(HOME / ".local/share/claude/versions" / PINS["claude"]),
    }


def check_tools() -> list[str]:
    problems = []
    probes = {
        "STUDY_WAR": (["--version"], PINS["war"]),
        "STUDY_BD": (["version"], PINS["bd"]),
        "STUDY_CLAUDE": (["--version"], PINS["claude"]),
    }
    for key, path in tools().items():
        if not Path(path).exists():
            problems.append(f"{key}: {path} is missing")
            continue
        if key in probes:
            argv, want = probes[key]
            if key == "STUDY_WAR" and os.environ.get("ADOPTION_WAR"):
                continue  # a local build, recorded in run.json, not the pin
            out = subprocess.run([path, *argv], text=True, capture_output=True, timeout=60).stdout
            if want not in out:
                problems.append(f"{key}: {path} reports {out.strip()[:80]!r}, not {want}")
    spec = STUDY / "tools/uv-tools/specify-cli"
    if not spec.exists():
        problems.append("specify: not installed (run.py prepare)")
    return problems


# ------------------------------------------------------------------ backlog and source


def repo_def(repo: str) -> Path:
    d = HERE / "repos" / repo
    if not (d / "backlog.toml").exists():
        raise SystemExit(f"no backlog for {repo!r} at {d}")
    return d


def load_backlog(repo: str) -> dict:
    with open(repo_def(repo) / "backlog.toml", "rb") as f:
        b = tomllib.load(f)
    ids = [i["id"] for i in b["item"]]
    if sorted(ids) != sorted(b["order"]):
        raise SystemExit(f"{repo}: order does not list every item exactly once")
    return b


def materialize(repo: str, dest: Path) -> None:
    """The seed tree as a fresh one-commit repository: same history in every arm."""
    d = repo_def(repo)
    with open(d / "source.toml", "rb") as f:
        src = tomllib.load(f)
    dest.mkdir(parents=True)
    if src["kind"] == "git":
        archive = subprocess.run(["git", "-C", src["path"], "archive", src["commit"]],
                                 capture_output=True, check=True, timeout=120).stdout
        subprocess.run(["tar", "-x", "-C", str(dest)], input=archive, check=True, timeout=120)
    elif src["kind"] == "seed-dir":
        shutil.copytree(d / src["path"], dest, dirs_exist_ok=True)
    else:
        raise SystemExit(f"{repo}: unknown source kind {src['kind']!r}")
    env = {**os.environ, **arms.SEED_ENV}
    for argv in (["git", "init", "-q", "-b", "main"], ["git", "add", "-A"],
                 ["git", "commit", "-q", "--no-verify", "-m", f"{repo} at the study's seed"]):
        subprocess.run(argv, cwd=dest, env=env, check=True, capture_output=True, timeout=120)


def apply_reference(repo: str, dest: Path, ids: list[str]) -> None:
    env = {**os.environ, **arms.SEED_ENV}
    for iid in ids:
        patch = repo_def(repo) / "reference" / f"{iid}.patch"
        subprocess.run(["git", "apply", "--whitespace=nowarn", str(patch)], cwd=dest, env=env,
                       check=True, capture_output=True, timeout=120)
    if ids:
        subprocess.run(["git", "add", "-A"], cwd=dest, env=env, check=True, timeout=120)
        subprocess.run(["git", "commit", "-q", "--no-verify", "-m",
                        f"done before the study: {', '.join(ids)}"], cwd=dest, env=env,
                       check=True, capture_output=True, timeout=120)


def seed_repo(repo: str, arm: str, dest: Path, done: list[str], extra: dict | None = None) -> dict:
    backlog = load_backlog(repo)
    materialize(repo, dest)
    apply_reference(repo, dest, done)
    arms.seed(arm, dest, backlog, set(done), {**tools(), **(extra or {})})
    return backlog


# ------------------------------------------------------------------ the sandbox

# How an agent session authenticates. "token": CLAUDE_CODE_OAUTH_TOKEN (from `claude
# setup-token`, the subscription); "api": ANTHROPIC_API_KEY (metered, exact cost);
# "credfile": bind ~/.claude/.credentials.json (fallback: token refresh may fail).
# The variable is passed into the sandbox's environment only, never written to disk.
AUTH_ENV = {"token": "CLAUDE_CODE_OAUTH_TOKEN", "api": "ANTHROPIC_API_KEY"}
_PORT = iter(range(42300, 42999))
_PORT_LOCK = threading.Lock()


def next_port() -> int:
    with _PORT_LOCK:
        return next(_PORT)


def auth_mode() -> str:
    mode = os.environ.get("ADOPTION_AUTH", "token")
    if mode not in ("token", "api", "credfile"):
        raise SystemExit(f"ADOPTION_AUTH={mode!r}: use token, api or credfile")
    return mode


def check_auth() -> str | None:
    mode = auth_mode()
    if mode in AUTH_ENV and not os.environ.get(AUTH_ENV[mode]):
        return f"ADOPTION_AUTH={mode} needs {AUTH_ENV[mode]} in the environment (run under `secrets run --`)"
    if mode == "credfile" and not (HOME / ".claude/.credentials.json").exists():
        return "ADOPTION_AUTH=credfile needs ~/.claude/.credentials.json"
    return None


class Sandbox:
    """One cell's view of the machine. `cell` is mounted at /work, read-write."""

    WORK = "/work"
    AHOME = "/home/agent"

    def __init__(self, cell: Path, repo: str, fixture: bool = False, arm: str = ""):
        self.cell, self.repo, self.fixture, self.arm = cell, repo, fixture, arm

    def _code_binds(self) -> list[str]:
        """The harness's own files and this repository's backlog.toml, never hidden/ or reference/."""
        a = []
        for f in ("run.py", "arms.py", "fixture_agent.py", "jira_arm.py"):
            a += ["--ro-bind", str(HERE / f), f"/opt/study/code/{f}"]
        a += ["--ro-bind", str(repo_def(self.repo) / "backlog.toml"),
              f"/opt/study/code/repos/{self.repo}/backlog.toml"]
        return a

    def argv(self, inner: list[str], cwd: str, net: bool, hidden: Path | None = None,
             config: Path | None = None, extra_env: dict | None = None,
             harness: bool = False) -> list[str]:
        t = tools()
        a = ["bwrap", "--die-with-parent", "--new-session", "--unshare-all"]
        if net:
            a.append("--share-net")
        a += ["--ro-bind", "/usr", "/usr", "--symlink", "usr/bin", "/bin",
              "--symlink", "usr/lib", "/lib", "--symlink", "usr/lib", "/lib64",
              "--symlink", "usr/bin", "/sbin", "--ro-bind", "/etc", "/etc",
              "--proc", "/proc", "--dev", "/dev", "--tmpfs", "/tmp", "--tmpfs", "/run",
              "--tmpfs", "/home", "--dir", self.AHOME,
              "--ro-bind", str(HOME / ".rustup"), f"{self.AHOME}/.rustup",
              "--bind", str(STUDY / "cargo-home"), "/opt/cargo-home",
              "--bind", str(STUDY / "sccache"), "/opt/sccache",
              "--ro-bind", str(STUDY / "py"), "/opt/study/py",
              "--ro-bind", t["STUDY_WAR"], "/opt/study/bin/war",
              "--ro-bind", t["STUDY_BD"], "/opt/study/bin/bd",
              "--ro-bind", t["STUDY_CLAUDE"], "/opt/study/bin/claude",
              "--bind", str(self.cell), self.WORK]
        if self.arm == "openwarrant" and not harness:
            # The plugin QUICKSTART tells a Claude Code user to install: skills, MCP server, hooks.
            a += ["--ro-bind", str(STUDY / "tools" / "ow-plugin"), "/opt/study/ow-plugin"]
        if harness or self.fixture:
            a += self._code_binds()
        if self.fixture and not harness:
            # A fixture agent is a script that applies the reference solution: it must see it.
            a += ["--ro-bind", str(repo_def(self.repo)), "/opt/study/repo-def"]
        if hidden is not None:
            a += ["--ro-bind", str(hidden), "/opt/hidden"]
        if config is not None:
            a += ["--bind", str(config), f"{self.AHOME}/.claude"]
            if net and auth_mode() == "credfile":
                a += ["--bind", str(HOME / ".claude/.credentials.json"),
                      f"{self.AHOME}/.claude/.credentials.json"]
        env = {
            "HOME": self.AHOME, "USER": "agent", "LANG": "C.UTF-8", "TERM": "dumb",
            "PATH": "/opt/study/bin:/opt/study/py/bin:/usr/bin",
            "RUSTUP_HOME": f"{self.AHOME}/.rustup", "RUSTUP_TOOLCHAIN": PINS["rust"],
            "CARGO_HOME": "/opt/cargo-home", "CARGO_NET_OFFLINE": "true",
            "CARGO_TARGET_DIR": f"{self.WORK}/target",
            "RUSTC_WRAPPER": "/usr/bin/sccache", "SCCACHE_DIR": "/opt/sccache",
            "SCCACHE_SERVER_PORT": str(next_port() if net else 4226),
            "GIT_AUTHOR_NAME": "agent", "GIT_AUTHOR_EMAIL": "agent@adoption.invalid",
            "GIT_COMMITTER_NAME": "agent", "GIT_COMMITTER_EMAIL": "agent@adoption.invalid",
            "CLAUDE_CONFIG_DIR": f"{self.AHOME}/.claude",
            "DISABLE_AUTOUPDATER": "1", "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC": "1",
            "PYTHONDONTWRITEBYTECODE": "1",
        }
        mode = auth_mode()
        if net and config is not None and mode in AUTH_ENV:
            env[AUTH_ENV[mode]] = os.environ[AUTH_ENV[mode]]
            # A key not scoped to a workspace needs the workspace named on every request.
            if mode == "api" and os.environ.get("ADOPTION_WORKSPACE_ID"):
                env["ANTHROPIC_CUSTOM_HEADERS"] = f"anthropic-workspace-id: {os.environ['ADOPTION_WORKSPACE_ID']}"
        env.update(extra_env or {})
        a.append("--clearenv")
        for k, v in env.items():
            a += ["--setenv", k, v]
        a += ["--chdir", cwd, *inner]
        return a

    def run(self, inner, cwd, net=False, timeout=600, hidden=None, config=None,
            extra_env=None, stdout=None, stderr=None, harness=False) -> subprocess.CompletedProcess:
        argv = self.argv(inner, cwd, net, hidden, config, extra_env, harness)
        p = subprocess.Popen(argv, stdout=stdout or subprocess.PIPE, stderr=stderr or subprocess.PIPE,
                             text=True, start_new_session=True)
        try:
            out, err = p.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            os.killpg(p.pid, signal.SIGKILL)
            out, err = p.communicate()
            return subprocess.CompletedProcess(argv, -9, out, (err or "") + "\n[harness: timed out]")
        return subprocess.CompletedProcess(argv, p.returncode, out, err)

    def inside(self, sub: list[str], cwd: str, timeout=900, hidden=None, target: str = "target") -> dict:
        """run.py's own in-sandbox subcommands (`_grade`, `_tracker`, `_git`); JSON out, no network."""
        p = self.run(["python3", "/opt/study/code/run.py", *sub], cwd, timeout=timeout, hidden=hidden,
                     harness=True, extra_env={"CARGO_TARGET_DIR": f"{self.WORK}/{target}"})
        try:
            return json.loads(p.stdout.strip().splitlines()[-1])
        except (ValueError, IndexError):
            return {"error": f"exit {p.returncode}: {(p.stderr or '')[-600:]}"}


# ------------------------------------------------------------------ inside the sandbox


def _inside_grade(repo: str) -> dict:
    backlog = load_backlog(repo)
    out = {"prepare": None, "items": {}}
    prep = backlog["project"].get("prepare")
    if prep:
        p = subprocess.run(prep, capture_output=True, text=True, timeout=1800)
        out["prepare"] = {"exit": p.returncode, "tail": (p.stderr or p.stdout)[-1500:]}
    for item in backlog["item"]:
        hidden = Path("/opt/hidden") / item["id"]
        argv = [a.replace("{hidden}", str(hidden)).replace("{repo}", os.getcwd()) for a in item["check"]]
        t0 = time.time()
        try:
            p = subprocess.run(argv, capture_output=True, text=True, timeout=item.get("timeout_secs", 300))
            res = {"pass": p.returncode == 0, "exit": p.returncode, "tail": (p.stdout + p.stderr)[-1200:]}
        except subprocess.TimeoutExpired:
            res = {"pass": False, "exit": None, "tail": "timed out"}
        res["secs"] = round(time.time() - t0, 1)
        out["items"][item["id"]] = res
    return out


def _inside_git(action: str, arg: str | None) -> dict:
    g = ["git", "-c", "core.hooksPath=/dev/null", "-c", "core.fsmonitor=false"]
    if action in ("merge", "bundle", "worktrees") and not arg:
        raise SystemExit(f"_git {action} needs an argument")
    arg = arg or ""
    if action == "snapshot":
        subprocess.run([*g, "add", "-A"], check=False, capture_output=True, timeout=300)
        p = subprocess.run([*g, "commit", "-q", "--no-verify", "--allow-empty", "-m", arg or "session"],
                           capture_output=True, text=True, timeout=300)
        head = subprocess.run([*g, "rev-parse", "HEAD"], capture_output=True, text=True).stdout.strip()
        return {"commit": head, "exit": p.returncode}
    if action == "diffstat":
        p = subprocess.run([*g, "diff", "--shortstat", arg or "HEAD~1", "HEAD"], capture_output=True, text=True)
        return {"shortstat": p.stdout.strip()}
    if action == "merge":
        p = subprocess.run([*g, "merge", "--no-edit", "--no-verify", arg], capture_output=True, text=True, timeout=300)
        if p.returncode != 0:
            files = subprocess.run([*g, "diff", "--name-only", "--diff-filter=U"],
                                   capture_output=True, text=True).stdout.split()
            subprocess.run([*g, "merge", "--abort"], capture_output=True)
            return {"merged": False, "conflicts": files}
        return {"merged": True, "conflicts": []}
    if action == "bundle":
        p = subprocess.run([*g, "bundle", "create", arg, "--all"], capture_output=True, text=True, timeout=300)
        return {"bundle": arg, "exit": p.returncode}
    if action == "worktrees":
        made = []
        for k in range(1, int(arg) + 1):
            subprocess.run([*g, "worktree", "add", "-q", f"../w{k}", "-b", f"agent{k}"],
                           check=True, capture_output=True, timeout=300)
            made.append(f"w{k}")
        return {"worktrees": made}
    raise SystemExit(f"unknown git action {action}")


def _inside_answers(repo: str, arm: str) -> dict:
    env = {**os.environ, **arms.SEED_ENV, "STUDY_WAR": "/opt/study/bin/war", "STUDY_BD": "/opt/study/bin/bd",
           "OPENWARRANT_ACTOR": "owner", "BEADS_ACTOR": "owner"}
    delivered = arms.deliver_answers(arm, Path.cwd(), load_backlog(repo), env)
    g = ["git", "-c", "core.hooksPath=/dev/null", "-c", "core.fsmonitor=false"]
    subprocess.run([*g, "add", "-A"], capture_output=True, timeout=300)
    subprocess.run([*g, "commit", "-q", "--no-verify", "-m", "owner: answers to open questions"],
                   capture_output=True, timeout=300, env={**env, "GIT_AUTHOR_NAME": "owner", "GIT_COMMITTER_NAME": "owner"})
    return {"answered": delivered}


def _inside_tracker(repo: str, arm: str) -> dict:
    env_tools = {"STUDY_WAR": "/opt/study/bin/war", "STUDY_BD": "/opt/study/bin/bd"}
    return arms.tracker(arm, Path.cwd(), load_backlog(repo), env_tools)


# ------------------------------------------------------------------ one agent session


ARM_MCP = {
    # Each arm's MCP servers as its own setup installs them: only the OpenWarrant plugin has one.
    "openwarrant": {"openwarrant": {"command": "war", "args": ["mcp"]}},
}


def agent_argv(agent: str, prompt: str, model: str, max_turns: int, arm: str = "") -> list[str]:
    if agent == "claude":
        argv = ["/opt/study/bin/claude", "-p", prompt, "--model", model,
                "--output-format", "stream-json", "--verbose",
                "--permission-mode", "bypassPermissions",
                "--strict-mcp-config", "--mcp-config", json.dumps({"mcpServers": ARM_MCP.get(arm, {})}),
                "--setting-sources", "project,local", "--no-session-persistence",
                "--max-turns", str(max_turns), "--max-budget-usd", str(SESSION_BUDGET_USD)]
        if arm == "openwarrant":
            argv += ["--plugin-dir", "/opt/study/ow-plugin"]
        return argv
    if agent in ("fixture-reference", "fixture-noop", "fixture-claim"):
        return ["python3", "/opt/study/code/fixture_agent.py", agent.split("-", 1)[1]]
    raise SystemExit(f"unknown agent {agent!r}")


TRACKER_PATHS = ("CLAUDE.md", "PLAN.md", "specs/", ".specify/", ".beads/", "docs/tickets/",
                 "openwarrant.toml", "AGENTS.md")

_REDIRECT = re.compile(r"(?<![0-9&>])>{1,2}\s*(?!&|/dev/null)[\w./~-]")
_WRITERS = re.compile(r"\bsed\s+-i|\btee\b|\bgit\s+apply\b|\bpatch\b|write_text\(|\.write\(|open\([^)]*['\"][wa]['\"]")
_SOURCE = re.compile(r"\.(py|rs|toml|ts|js|go|c|h|json|md)\b|\b(src|tests?|crates|tally|docs)/")


def bash_writes_code(cmd: str) -> bool:
    """A Bash call that writes a file other than the arm's own tracker files (heuristic).

    Agents often edit through `sed -i`, heredoc scripts or redirects instead of Edit/Write.
    A command counts when it has a writing form and names a source path that is not only
    a tracker file."""
    if not (_REDIRECT.search(cmd) or _WRITERS.search(cmd)):
        return False
    stripped = cmd
    for t in TRACKER_PATHS:
        stripped = stripped.replace(t, "")
    return bool(_SOURCE.search(stripped))


def parse_stream(path: Path) -> dict:
    """Metrics from a `claude -p --output-format stream-json` transcript."""
    seen, msgs, result = set(), [], None
    tools_used: dict[str, int] = {}
    first_edit_ctx = None
    processed_before_edit = 0
    bash_tracker_calls = 0
    if not path.exists():
        return {"error": "no transcript"}
    for line in path.read_text(errors="replace").splitlines():
        try:
            ev = json.loads(line)
        except ValueError:
            continue
        if ev.get("type") == "result":
            result = ev
        if ev.get("type") != "assistant":
            continue
        m = ev.get("message", {})
        mid = m.get("id")
        # Claude Code streams each content block of one message as its own event under the same
        # message id: count the message's tokens once, but read every block.
        first_of_message = mid not in seen
        seen.add(mid)
        u = m.get("usage", {})
        ctx = (u.get("input_tokens", 0) + u.get("cache_read_input_tokens", 0)
               + u.get("cache_creation_input_tokens", 0))
        if first_of_message:
            msgs.append(ctx)
        edits_code = False
        for block in m.get("content", []):
            if block.get("type") != "tool_use":
                continue
            name = block.get("name", "?")
            tools_used[name] = tools_used.get(name, 0) + 1
            inp = block.get("input", {})
            if name in ("Edit", "Write", "MultiEdit", "NotebookEdit"):
                fp = str(inp.get("file_path", ""))
                if not any(t in fp for t in TRACKER_PATHS):
                    edits_code = True
            if name == "Bash":
                cmd = str(inp.get("command", ""))
                if bash_writes_code(cmd):
                    edits_code = True
                if any(cmd.lstrip().startswith(w) or f" {w} " in cmd for w in ("war", "bd")) \
                        or "PLAN.md" in cmd or "tasks.md" in cmd:
                    bash_tracker_calls += 1
        if first_edit_ctx is None:
            if first_of_message:
                processed_before_edit += ctx + u.get("output_tokens", 0)
            if edits_code:
                first_edit_ctx = ctx
    out = {
        "assistant_messages": len(msgs),  # distinct message ids
        "tools": tools_used,
        "tracker_bash_calls": bash_tracker_calls,
        "context_at_first_code_edit": first_edit_ctx,
        "tokens_processed_before_first_code_edit": processed_before_edit if first_edit_ctx is not None else None,
        "peak_context": max(msgs) if msgs else 0,
    }
    if result and result.get("is_error") and str(result.get("result", "")).startswith("API Error"):
        out["api_error"] = str(result.get("result"))[:400]
    if result:
        u = result.get("usage", {})
        out.update({
            "subtype": result.get("subtype"), "is_error": result.get("is_error"),
            "num_turns": result.get("num_turns"), "duration_ms": result.get("duration_ms"),
            "cost_usd": result.get("total_cost_usd"),
            "input_tokens": u.get("input_tokens"), "output_tokens": u.get("output_tokens"),
            "cache_read_tokens": u.get("cache_read_input_tokens"),
            "cache_creation_tokens": u.get("cache_creation_input_tokens"),
        })
    return out


def run_session(sb: Sandbox, sdir: Path, cwd: str, prompt: str, agent: str, model: str,
                max_turns: int, extra_env: dict | None = None, arm: str = "") -> dict:
    sdir.mkdir(parents=True, exist_ok=True)
    config = sdir / "claude-config"
    config.mkdir(exist_ok=True)
    (sdir / "prompt.txt").write_text(prompt + "\n")
    t0 = time.time()
    with open(sdir / "stream.jsonl", "w") as out, open(sdir / "stderr.txt", "w") as err:
        env = {**(extra_env or {}), **(jira_env(sb.cell) if arm == "jira" else {})}
        p = sb.run(agent_argv(agent, prompt, model, max_turns, arm), cwd, net=(agent == "claude" or arm == "jira"),
                   timeout=SESSION_TIMEOUT, config=config, extra_env=env, stdout=out, stderr=err)
    metrics = parse_stream(sdir / "stream.jsonl")
    if metrics.get("api_error"):
        # The model never ran: this is the harness's problem, not the arm's. Stop, score nothing.
        raise SystemExit(f"stopped: the API refused the session ({sdir}): {metrics['api_error']}")
    metrics.update({"exit": p.returncode, "wall_secs": round(time.time() - t0, 1), "max_turns": max_turns})
    shutil.rmtree(config, ignore_errors=True)  # holds nothing the study keeps; never the credentials
    (sdir / "metrics.json").write_text(json.dumps(metrics, indent=1))
    return metrics


def read_tracker(sb: Sandbox, cwd: str, repo: str, arm: str, target: str = "target") -> dict:
    """The arm's own record of what is done. Jira's lives on Jira's servers, so it is read
    from the host over the API (nothing from the repository runs); every other arm's is read
    inside the sandbox, without network."""
    if arm == "jira":
        return arms.tracker("jira", Path("."), load_backlog(repo), jira_env(sb.cell))
    return sb.inside(["_tracker", repo, arm], cwd, timeout=300, target=target)


def deliver(sb: "Sandbox", cwd: str, repo: str, arm: str) -> dict:
    """The owner's answers to open questions, in the arm's own channel; Jira's from the host."""
    if not any(i.get("question") for i in load_backlog(repo)["item"]):
        return {"answered": []}
    if arm == "jira":
        return {"answered": arms.deliver_answers("jira", Path("."), load_backlog(repo), jira_env(sb.cell))}
    return sb.inside(["_answers", repo, arm], cwd, timeout=300)


def jira_label(cell: Path) -> str:
    import hashlib
    return "study-" + hashlib.sha256(str(cell).encode()).hexdigest()[:12]


def jira_env(cell: Path) -> dict:
    return {**{k: os.environ[k] for k in jira_arm.ENV}, "STUDY_JIRA_LABEL": jira_label(cell)}


def observe(sb: Sandbox, sdir: Path, cwd: str, repo: str, arm: str, target: str = "target") -> dict:
    """After a session: grade (no network, hidden tests mounted read-only) and read the tracker."""
    sdir.mkdir(parents=True, exist_ok=True)
    grade = sb.inside(["_grade", repo], cwd, timeout=3600, hidden=repo_def(repo) / "hidden", target=target)
    tracker = read_tracker(sb, cwd, repo, arm, target)
    (sdir / "grade.json").write_text(json.dumps(grade, indent=1))
    (sdir / "tracker.json").write_text(json.dumps(tracker, indent=1))
    passing = {k: v["pass"] for k, v in grade.get("items", {}).items()}
    return {"pass": passing, "tracker": tracker,
            "false_completions": sorted(k for k, v in tracker.items() if v is True and not passing.get(k)),
            "unrecorded_completions": sorted(k for k, v in tracker.items() if v is False and passing.get(k))}


# ------------------------------------------------------------------ scenarios


def cell_dir(run_id: str, c: dict) -> Path:
    return RUNS / run_id / c["scenario"] / c["repo"] / c["arm"] / c.get("item", "-") / f"r{c['rep']}"


def run_cell(run_id: str, c: dict, agent: str, model: str) -> dict:
    cell = cell_dir(run_id, c)
    if (cell / "result.json").exists():
        return json.loads((cell / "result.json").read_text())
    if cell.exists():
        shutil.rmtree(cell)
    cell.mkdir(parents=True)
    backlog = load_backlog(c["repo"])
    sb = Sandbox(cell, c["repo"], fixture=agent.startswith("fixture"), arm=c["arm"])
    fixture_env = {"STUDY_ARM": c["arm"], "STUDY_REPO": c["repo"]}
    result: dict = {"cell": c, "agent": agent, "model": model, "sessions": []}
    t0 = time.time()
    extra = jira_env(cell) if c["arm"] == "jira" else None
    try:
        _run_cell_body(c, cell, sb, backlog, agent, model, fixture_env, result, extra)
    finally:
        if c["arm"] == "jira":
            result["jira_issues_deleted"] = jira_arm.cleanup(jira_label(cell), jira_env(cell))
    result["wall_secs"] = round(time.time() - t0, 1)
    result["cost_usd"] = round(sum((s.get("metrics") or {}).get("cost_usd") or 0 for s in result["sessions"]), 4)
    (cell / "result.json").write_text(json.dumps(result, indent=1))
    keep_cell(run_id, c, cell)
    return result


def _run_cell_body(c: dict, cell: Path, sb: "Sandbox", backlog: dict, agent: str, model: str,
                   fixture_env: dict, result: dict, extra: dict | None) -> None:

    if c["scenario"] == "directed":
        idx = backlog["order"].index(c["item"])
        seed_repo(c["repo"], c["arm"], cell / "repo", backlog["order"][:idx], extra)
        item = next(i for i in backlog["item"] if i["id"] == c["item"])
        result["answers"] = deliver(sb, f"{sb.WORK}/repo", c["repo"], c["arm"])
        prompt = PROMPTS["directed"].format(id=item["id"], title=item["title"])
        m = run_session(sb, cell / "s01", f"{sb.WORK}/repo", prompt, agent, model, SESSION_TURNS,
                        {**fixture_env, "STUDY_TARGET": item["id"]}, arm=c["arm"])
        sb.inside(["_git", "snapshot", "s01"], f"{sb.WORK}/repo")
        obs = observe(sb, cell / "s01", f"{sb.WORK}/repo", c["repo"], c["arm"])
        earlier = backlog["order"][:idx]
        result["sessions"].append({"metrics": m, **obs})
        result.update({
            "success": bool(obs["pass"].get(item["id"])),
            "regressions": sorted(k for k in earlier if not obs["pass"].get(k)),
            "tracker_says_done": obs["tracker"].get(item["id"]),
        })

    elif c["scenario"] == "relay":
        seed_repo(c["repo"], c["arm"], cell / "repo", [], extra)
        n_max = len(backlog["item"]) + 2
        stalled = 0
        prev_pass: dict = {}
        prev_tracker: dict = read_tracker(sb, f"{sb.WORK}/repo", c["repo"], c["arm"])
        for s in range(1, n_max + 1):
            sdir = cell / f"s{s:02d}"
            turns = INTERRUPT_TURNS if s == 2 else SESSION_TURNS
            m = run_session(sb, sdir, f"{sb.WORK}/repo", PROMPTS["relay"], agent, model, turns, fixture_env,
                            arm=c["arm"])
            snap = sb.inside(["_git", "snapshot", f"s{s:02d}"], f"{sb.WORK}/repo")
            stat = sb.inside(["_git", "diffstat", "HEAD~1"], f"{sb.WORK}/repo")
            obs = observe(sb, sdir, f"{sb.WORK}/repo", c["repo"], c["arm"])
            newly = sorted(k for k, v in obs["pass"].items() if v and not prev_pass.get(k))
            lost = sorted(k for k, v in obs["pass"].items() if not v and prev_pass.get(k))
            result["sessions"].append({"n": s, "interrupted_by_design": s == 2, "metrics": m,
                                       "snapshot": snap, "diff": stat, "newly_passing": newly,
                                       "regressed": lost, **obs})
            # Idle means nothing moved: no file changed, no tracker record changed, nothing newly
            # passes. The tracker is compared too because Beads keeps its records outside git.
            moved = bool(stat.get("shortstat")) or obs["tracker"] != prev_tracker or bool(newly)
            prev_pass, prev_tracker = obs["pass"], obs["tracker"]
            stalled = 0 if moved else stalled + 1
            if s == 1:
                result["answers"] = deliver(sb, f"{sb.WORK}/repo", c["repo"], c["arm"])
                prev_tracker = read_tracker(sb, f"{sb.WORK}/repo", c["repo"], c["arm"])
            if all(prev_pass.get(i["id"]) for i in backlog["item"]) or stalled >= 2:
                break
        result.update({
            "items_passing": sorted(k for k, v in prev_pass.items() if v),
            "sessions_used": len(result["sessions"]),
            "stopped": "all_pass" if all(prev_pass.get(i["id"]) for i in backlog["item"])
                       else ("stalled" if stalled >= 2 else "session_cap"),
            "false_completions_at_end": result["sessions"][-1]["false_completions"],
        })

    elif c["scenario"] == "parallel":
        k = c["k"]
        seed_repo(c["repo"], c["arm"], cell / "repo", [], extra)
        result["answers"] = deliver(sb, f"{sb.WORK}/repo", c["repo"], c["arm"])
        sb.inside(["_git", "worktrees", str(k)], f"{sb.WORK}/repo")
        finished: list[tuple[float, int, dict]] = []
        lock = threading.Lock()

        def one(j: int):
            m = run_session(sb, cell / f"a{j}", f"{sb.WORK}/w{j}", PROMPTS["parallel"], agent, model,
                            SESSION_TURNS, {**fixture_env, "CARGO_TARGET_DIR": f"{sb.WORK}/target-{j}"},
                            arm=c["arm"])
            sb.inside(["_git", "snapshot", f"agent{j}"], f"{sb.WORK}/w{j}")
            with lock:
                finished.append((time.time(), j, m))

        with concurrent.futures.ThreadPoolExecutor(max_workers=k) as ex:
            list(ex.map(one, range(1, k + 1)))
        per_agent = {}
        for _, j, m in sorted(finished):
            obs = observe(sb, cell / f"a{j}", f"{sb.WORK}/w{j}", c["repo"], c["arm"], target=f"target-{j}")
            per_agent[j] = {"metrics": m, **obs}
        merges = []
        for _, j, _ in sorted(finished):
            merges.append({"agent": j, **sb.inside(["_git", "merge", f"agent{j}"], f"{sb.WORK}/repo")})
        final = observe(sb, cell / "merged", f"{sb.WORK}/repo", c["repo"], c["arm"])
        solved_by: dict[str, list[int]] = {}
        for j, a in per_agent.items():
            for iid, ok in a["pass"].items():
                if ok:
                    solved_by.setdefault(iid, []).append(j)
        result["sessions"] = [{"agent": j, **a} for j, a in per_agent.items()]
        result.update({
            "merges": merges,
            "merge_conflicts": sum(1 for m in merges if not m.get("merged")),
            "items_passing_merged": sorted(k for k, v in final["pass"].items() if v),
            "duplicated_items": sorted(k for k, v in solved_by.items() if len(v) > 1),
            "false_completions_merged": final["false_completions"],
        })



def keep_cell(run_id: str, c: dict, cell: Path) -> None:
    """Raw evidence to the keep store; the working trees and build output are deleted."""
    dest = KEEP / run_id / cell.relative_to(RUNS / run_id)
    if (cell / "repo").exists():
        Sandbox(cell, c["repo"]).inside(["_git", "bundle", "/work/repo.bundle"], "/work/repo")
    dest.mkdir(parents=True, exist_ok=True)
    for f in cell.rglob("*"):
        if f.is_file() and f.name in ("stream.jsonl", "stderr.txt", "metrics.json", "grade.json",
                                       "tracker.json", "prompt.txt", "result.json"):
            rel = f.relative_to(cell)
            (dest / rel.parent).mkdir(parents=True, exist_ok=True)
            shutil.copy2(f, dest / rel)
    if (cell / "repo.bundle").exists():
        shutil.copy2(cell / "repo.bundle", dest / "repo.bundle")
    for sub in ("repo", "target", "repo.bundle") + tuple(f"w{k}" for k in range(1, 11)) \
            + tuple(f"target-{k}" for k in range(1, 11)):
        p = cell / sub
        if p.is_dir():
            shutil.rmtree(p, ignore_errors=True)
        elif p.exists():
            p.unlink()


# ------------------------------------------------------------------ plan, execute, report


FIRST_RUN_ARMS = ("plain", "plans", "speckit", "beads", "openwarrant")  # jira: a later run


def plan_cells(pilot: bool, reps: int, k: int, arm_list: list[str] | None = None) -> list[dict]:
    repos = ["tally"] if pilot else sorted(p.name for p in (HERE / "repos").iterdir()
                                          if (p / "backlog.toml").exists() and not p.name.startswith("_"))
    arm_list = ["plain", "openwarrant"] if pilot else (arm_list or list(FIRST_RUN_ARMS))
    reps = 1 if pilot else reps
    cells = []
    for repo in repos:
        backlog = load_backlog(repo)
        directed_items = backlog["order"][:2] if pilot else backlog["order"]
        for arm in arm_list:
            for rep in range(1, reps + 1):
                for item in directed_items:
                    cells.append({"scenario": "directed", "repo": repo, "arm": arm, "item": item, "rep": rep})
                cells.append({"scenario": "relay", "repo": repo, "arm": arm, "rep": rep})
                if not pilot:
                    cells.append({"scenario": "parallel", "repo": repo, "arm": arm, "rep": rep, "k": k})
    # Replicate-major, arms interleaved: a budget stop leaves every arm with the same cells.
    scen = {name: n for n, name in enumerate(SCENARIOS)}
    arm_n = {a: n for n, a in enumerate(arm_list)}
    cells.sort(key=lambda c: (c["rep"], scen[c["scenario"]], c["repo"], c.get("item", ""), arm_n[c["arm"]]))
    return cells


def sessions_in(c: dict, backlog: dict) -> int:
    return {"directed": 1, "relay": len(backlog["item"]) + 2, "parallel": c.get("k", 0)}[c["scenario"]]


def cmd_execute(args) -> None:
    cells = json.loads(Path(args.plan).read_text())["cells"]
    problems = check_tools()
    if any(c["arm"] == "jira" for c in cells) and jira_arm.missing():
        problems.append("jira cells need " + ", ".join(jira_arm.missing()) + " in the environment")
    if args.agent == "claude" and check_auth():
        problems.append(check_auth())
    if problems:
        raise SystemExit("not ready:\n  " + "\n  ".join(problems))
    run_root = RUNS / args.run_id
    run_root.mkdir(parents=True, exist_ok=True)
    (run_root / "run.json").write_text(json.dumps({
        "run_id": args.run_id, "agent": args.agent, "model": args.model, "pins": PINS,
        "war_build": os.environ.get("ADOPTION_WAR"), "plugin_ref": os.environ.get("ADOPTION_OW_PLUGIN_REF"),
        "ow_session_start": os.environ.get("ADOPTION_OW_SESSION_START") == "1",
        "auth": auth_mode(), "budget_usd": args.budget_usd,
        "started": time.strftime("%Y-%m-%dT%H:%M:%S%z"), "cells": len(cells)}, indent=1))
    spent = 0.0
    lock = threading.Lock()
    stop = threading.Event()

    def go(c):
        nonlocal spent
        if stop.is_set():
            return None
        r = run_cell(args.run_id, c, args.agent, args.model)
        with lock:
            spent += r.get("cost_usd") or 0
            done_line = f"[{spent:8.2f} USD] {c['scenario']:8} {c['repo']:9} {c['arm']:11} {c.get('item','-'):3} r{c['rep']}"
            print(done_line, flush=True)
            if args.budget_usd and spent >= args.budget_usd:
                stop.set()
                print(f"budget reached ({spent:.2f} >= {args.budget_usd}); no new cells start", flush=True)
        return r

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as ex:
        list(ex.map(go, cells))
    print(f"spent {spent:.2f} USD (API-equivalent, as reported by the agent)")


def wilson(k: int, n: int, z: float = 1.96) -> tuple[float, float]:
    if n == 0:
        return (0.0, 0.0)
    p = k / n
    d = 1 + z * z / n
    c = (p + z * z / (2 * n)) / d
    h = z * ((p * (1 - p) / n + z * z / (4 * n * n)) ** 0.5) / d
    return (round(c - h, 3), round(c + h, 3))


def bootstrap_ci(diffs: list[float], n: int = 10000, seed: int = 20261022) -> tuple[float, float]:
    import random
    rng = random.Random(seed)
    k = len(diffs)
    means = sorted(sum(rng.choice(diffs) for _ in range(k)) / k for _ in range(n))
    return (round(means[int(0.025 * n)], 3), round(means[int(0.975 * n) - 1], 3))


def paired_differences(results: list[dict]) -> list[dict]:
    """OpenWarrant minus each other arm on the primary measures, over cells both arms ran."""
    measures = {
        "directed success": ("directed", lambda c: (c["repo"], c["item"], c["rep"]),
                             lambda r: 1.0 if r.get("success") else 0.0),
        "relay items passing": ("relay", lambda c: (c["repo"], c["rep"]),
                                lambda r: float(len(r.get("items_passing", [])))),
        "relay cost per session (USD)": ("relay", lambda c: (c["repo"], c["rep"]),
                                         lambda r: (r.get("cost_usd") or 0) / max(1, r.get("sessions_used", 1))),
    }
    out = []
    for name, (scenario, key, value) in measures.items():
        cells: dict[str, dict] = {}
        for r in results:
            c = r["cell"]
            if c["scenario"] == scenario:
                cells.setdefault(c["arm"], {})[key(c)] = value(r)
        ow = cells.get("openwarrant", {})
        for arm, vals in sorted(cells.items()):
            if arm == "openwarrant":
                continue
            common = sorted(set(ow) & set(vals))
            if len(common) < 2:
                continue
            diffs = [ow[k] - vals[k] for k in common]
            out.append({"measure": name, "arm": arm, "cells": len(common),
                        "mean": round(sum(diffs) / len(diffs), 3), "ci95": bootstrap_ci(diffs)})
    return out


def cmd_report(args) -> None:
    root = KEEP / args.run_id
    results = [json.loads(p.read_text()) for p in sorted(root.rglob("result.json"))]
    by: dict[tuple, list] = {}
    for r in results:
        c = r["cell"]
        by.setdefault((c["scenario"], c["arm"]), []).append(r)
    rows = []
    for (scenario, arm), rs in sorted(by.items()):
        cost = sum(r.get("cost_usd") or 0 for r in rs)
        if scenario == "directed":
            k = sum(1 for r in rs if r.get("success"))
            ctx = [s["metrics"].get("context_at_first_code_edit") for r in rs for s in r["sessions"]
                   if s["metrics"].get("context_at_first_code_edit") is not None]
            rows.append({"scenario": scenario, "arm": arm, "cells": len(rs), "success": k,
                         "success_ci95": wilson(k, len(rs)), "cost_usd": round(cost, 2),
                         "cost_per_success": round(cost / k, 2) if k else None,
                         "median_context_at_first_edit": sorted(ctx)[len(ctx) // 2] if ctx else None,
                         "false_completions": sum(1 for r in rs if r.get("tracker_says_done") and not r.get("success")),
                         "regressions": sum(len(r.get("regressions", [])) for r in rs)})
        elif scenario == "relay":
            passed = sum(len(r.get("items_passing", [])) for r in rs)
            cut = [r for r in rs if len(r["sessions"]) >= 3
                   and (r["sessions"][1]["metrics"].get("subtype") == "error_max_turns")]
            resumed = sum(1 for r in cut if r["sessions"][2].get("newly_passing"))
            rows.append({"scenario": scenario, "arm": arm, "cells": len(rs), "items_passing": passed,
                         "sessions": sum(r.get("sessions_used", 0) for r in rs), "cost_usd": round(cost, 2),
                         "cost_per_item": round(cost / passed, 2) if passed else None,
                         "false_completions_at_end": sum(len(r.get("false_completions_at_end", [])) for r in rs),
                         "session2_cut": len(cut), "resumed_in_session3": resumed})
        else:
            rows.append({"scenario": scenario, "arm": arm, "cells": len(rs),
                         "items_passing_merged": sum(len(r.get("items_passing_merged", [])) for r in rs),
                         "duplicated_items": sum(len(r.get("duplicated_items", [])) for r in rs),
                         "merge_conflicts": sum(r.get("merge_conflicts", 0) for r in rs),
                         "cost_usd": round(cost, 2)})
    paired = paired_differences(results)
    (root / "results.json").write_text(json.dumps({"run_id": args.run_id, "rows": rows,
                                                   "paired_vs_openwarrant": paired}, indent=1))
    lines = [f"# Adoption study run {args.run_id}", ""]
    for scenario in SCENARIOS:
        sub = [r for r in rows if r["scenario"] == scenario]
        if not sub:
            continue
        keys = [k for k in sub[0] if k != "scenario"]
        lines += [f"## {scenario}", "", "| " + " | ".join(keys) + " |", "|" + "---|" * len(keys)]
        lines += ["| " + " | ".join(str(r.get(k)) for k in keys) + " |" for r in sub] + [""]
    if paired:
        lines += ["## OpenWarrant minus each arm (paired cells, bootstrap 95% interval)", "",
                  "| measure | other arm | cells | mean difference | interval |", "|---|---|---|---|---|"]
        lines += [f"| {d['measure']} | {d['arm']} | {d['cells']} | {d['mean']} | {d['ci95']} |" for d in paired]
        lines.append("")
    (root / "REPORT.md").write_text("\n".join(lines))
    print("\n".join(lines))


# ------------------------------------------------------------------ setup commands


def cmd_prepare(args) -> None:
    for d in ("cargo-home", "sccache", "runs"):
        (STUDY / d).mkdir(parents=True, exist_ok=True)
    plugin = STUDY / "tools" / "ow-plugin"
    shutil.rmtree(plugin, ignore_errors=True)
    plugin.mkdir(parents=True)
    ref = os.environ.get("ADOPTION_OW_PLUGIN_REF", PINS["ow_plugin"])
    archive = subprocess.run(["git", "-C", str(OW_REPO), "archive", ref, *PLUGIN_FILES],
                             capture_output=True, check=True, timeout=120).stdout
    subprocess.run(["tar", "-x", "-C", str(plugin)], input=archive, check=True, timeout=120)
    print(f"openwarrant plugin {ref}: {sum(1 for _ in plugin.rglob('*') if _.is_file())} files")
    py = STUDY / "py"
    if not (py / "bin/python3").exists():
        subprocess.run(["python3", "-m", "venv", str(py)], check=True, timeout=300)
    subprocess.run([str(py / "bin/python3"), "-m", "pip", "install", "-q", f"pytest=={PINS['pytest']}"],
                   check=True, timeout=600)
    problems = check_tools()
    for p in problems:
        print("tool:", p)
    for repo in sorted(p.name for p in (HERE / "repos").iterdir()
                       if (p / "source.toml").exists() and not p.name.startswith("_")):
        with open(repo_def(repo) / "source.toml", "rb") as f:
            src = tomllib.load(f)
        if src["kind"] != "git":
            continue
        tmp = STUDY / "fetch" / repo
        shutil.rmtree(tmp, ignore_errors=True)
        materialize(repo, tmp)
        env = {**os.environ, "CARGO_HOME": str(STUDY / "cargo-home"), "RUSTUP_TOOLCHAIN": PINS["rust"]}
        p = subprocess.run(["cargo", "fetch", "--locked"], cwd=tmp, env=env, capture_output=True, text=True, timeout=900)
        print(f"{repo}: cargo fetch exit {p.returncode}")
        shutil.rmtree(tmp, ignore_errors=True)
    print("ready" if not problems else "not ready")


def cmd_validate(args) -> None:
    script = repo_def(args.repo) / "validate.sh"
    raise SystemExit(subprocess.run(["bash", str(script)], timeout=7200).returncode)


def cmd_parity(args) -> None:
    """Every item's title, acceptance criteria and notes reach every arm's files."""
    backlog = load_backlog(args.repo)
    base = RUNS / "_parity" / args.repo
    shutil.rmtree(base, ignore_errors=True)
    report = {}
    for arm in arms.ARMS:
        dest = base / arm
        if arm == "jira" and jira_arm.missing():
            print(f"{arm:12} skipped: needs {', '.join(jira_arm.missing())}")
            continue
        extra = jira_env(dest) if arm == "jira" else None
        seed_repo(args.repo, arm, dest, [], extra)
        text = ""
        for f in dest.rglob("*"):
            if f.is_file() and ".git" not in f.parts and f.suffix in (".md", ".jsonl", ".toml", ""):
                try:
                    text += f.read_text(errors="ignore")
                except OSError:
                    pass
        if arm == "beads":
            bd = tools()["STUDY_BD"]
            rows = json.loads(subprocess.run([bd, "list", "--json", "--all"], cwd=dest, capture_output=True,
                                             text=True, timeout=120).stdout or "[]")
            for issue in rows:
                shown = json.loads(subprocess.run([bd, "show", "--json", issue["id"]], cwd=dest,
                                                  capture_output=True, text=True, timeout=120).stdout)
                for d in shown if isinstance(shown, list) else [shown]:
                    text += "\n".join(str(d.get(k) or "") for k in
                                      ("title", "description", "acceptance_criteria", "design")) + "\n"
        if arm == "jira":
            text += jira_arm.text(jira_label(dest), jira_env(dest))
        flat = " ".join(text.split())

        def norm(piece: str) -> str:
            piece = piece.strip()
            piece = piece[2:] if arm == "jira" and piece.startswith("- ") else piece
            return " ".join(piece.split())

        missing = []
        for item in backlog["item"]:
            for piece in [item["title"], *item["acceptance"], *[l for l in item.get("notes", "").splitlines() if l.strip()]]:
                if norm(piece) not in flat:
                    missing.append(f"{item['id']}: {piece.strip()[:70]}")
        trk = arms.tracker(arm, dest, backlog, {**tools(), **(extra or {})})
        if arm == "jira":
            jira_arm.cleanup(jira_label(dest), jira_env(dest))
        report[arm] = {"missing": missing, "tracker": trk}
        print(f"{arm:12} missing {len(missing):3}  tracker {trk}")
        for m in missing[:5]:
            print("   ", m)
    if not args.keep:
        shutil.rmtree(base, ignore_errors=True)
    raise SystemExit(1 if any(r["missing"] for r in report.values()) else 0)


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    sub = ap.add_subparsers(dest="cmd", required=True)
    sub.add_parser("prepare")
    v = sub.add_parser("validate"); v.add_argument("repo")
    s = sub.add_parser("seed"); s.add_argument("repo"); s.add_argument("arm", choices=arms.ARMS)
    s.add_argument("dest"); s.add_argument("--done", nargs="*", default=[])
    pa = sub.add_parser("parity"); pa.add_argument("repo"); pa.add_argument("--keep", action="store_true")
    pl = sub.add_parser("plan"); pl.add_argument("--pilot", action="store_true")
    pl.add_argument("--smoke", action="store_true", help="one directed tally T6 session, plain arm")
    pl.add_argument("--reps", type=int, default=3); pl.add_argument("--k", type=int, default=3)
    pl.add_argument("--out")
    pl.add_argument("--arms", nargs="*", choices=arms.ARMS, help="arms for the full plan (default: all but jira)")
    ex = sub.add_parser("execute"); ex.add_argument("plan"); ex.add_argument("--run-id", required=True)
    ex.add_argument("--agent", required=True); ex.add_argument("--model", default=DEFAULT_MODEL)
    ex.add_argument("--jobs", type=int, default=2); ex.add_argument("--budget-usd", type=float, default=0)
    rp = sub.add_parser("report"); rp.add_argument("run_id")
    # inside the sandbox only
    g = sub.add_parser("_grade"); g.add_argument("repo")
    t = sub.add_parser("_tracker"); t.add_argument("repo"); t.add_argument("arm")
    an = sub.add_parser("_answers"); an.add_argument("repo"); an.add_argument("arm")
    gi = sub.add_parser("_git"); gi.add_argument("action"); gi.add_argument("arg", nargs="?")
    args = ap.parse_args()

    if args.cmd == "prepare":
        cmd_prepare(args)
    elif args.cmd == "validate":
        cmd_validate(args)
    elif args.cmd == "seed":
        seed_repo(args.repo, args.arm, Path(args.dest), args.done)
        print(args.dest)
    elif args.cmd == "parity":
        cmd_parity(args)
    elif args.cmd == "plan":
        cells = ([{"scenario": "directed", "repo": "tally", "arm": "plain", "item": "T6", "rep": 1}]
                 if args.smoke else plan_cells(args.pilot, args.reps, args.k, args.arms))
        n = sum(sessions_in(c, load_backlog(c["repo"])) for c in cells)
        doc = {"pilot": args.pilot, "cells": cells, "max_sessions": n}
        text = json.dumps(doc, indent=1)
        if args.out:
            Path(args.out).write_text(text)
        by = {}
        for c in cells:
            by[c["scenario"]] = by.get(c["scenario"], 0) + 1
        print(f"{len(cells)} cells {by}; at most {n} agent sessions")
    elif args.cmd == "execute":
        cmd_execute(args)
    elif args.cmd == "report":
        cmd_report(args)
    elif args.cmd == "_grade":
        print(json.dumps(_inside_grade(args.repo)))
    elif args.cmd == "_answers":
        print(json.dumps(_inside_answers(args.repo, args.arm)))
    elif args.cmd == "_tracker":
        print(json.dumps(_inside_tracker(args.repo, args.arm)))
    elif args.cmd == "_git":
        print(json.dumps(_inside_git(args.action, args.arg)))


if __name__ == "__main__":
    main()
