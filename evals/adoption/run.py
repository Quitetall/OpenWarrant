#!/usr/bin/env python3
"""OpenWarrant adoption study: the harness. PROTOCOL.md is the design; this runs it.

    run.py prepare                       pin and check the tools, fetch crates, warm caches
    run.py validate <repo>               the backlog invariants (BACKLOG-FORMAT.md)
    run.py seed <repo> <arm> <dest> [--done ID ...]
                                         one seeded repository, to read what an arm's agent sees
    run.py parity <repo>                 seed every arm and check each holds every item's words
    run.py plan [--smoke|--pilot] [--out FILE]
                                         the cells of the study, with session counts
    run.py execute <plan> --run-id ID --agent claude|opencode|fixture-reference|fixture-noop
               [--jobs N] [--budget-usd X] [--model M]
                                         run the cells (resumable: a finished cell is skipped)
    run.py report <run-id>               results.json and REPORT.md from a run
    run.py pool <run-id> ...             the win rule over the pooled directed cells of several runs
    run.py a8-check --run-id ID          amendment 8's checks: the paired token rule, the done_test
                                         seed, the brief arm, the hard budget; no model, scratch roots
    run.py isolation-check --run-id ID   amendment 7's leak probe and a reference relay; no model,
                                         scratch roots only

The spec study (PROTOCOL-spec.md), which has an author write the spec a worker then runs:

    run.py spec-plan [--dry-run] [--out FILE]
                                         the pilot's 120 cells, 40 specs and estimated cost; no model
    run.py spec <plan> --run-id ID --budget-usd X [--author opus|deepseek] [--jobs N]
                                         write the specs and store them by digest (costs tokens; each
                                         brief must be frozen). --fixture good|bad runs a script instead
    run.py execute <plan> --run-id ID --agent opencode --model M --specs-run ID
                                         the worker cells, on the specs of run ID
    run.py spec-check                    each brief against its sufficiency table and hidden tests
    run.py spec-selfcheck                the study's checks with no model (scratch roots only)

Every agent session, and everything that touches a repository after an agent
has (snapshot commits, merges, tracker reads, grading), runs inside a
bubblewrap sandbox that sees only that cell's workspace (`<cell>/work`, at /work),
the toolchain and the pinned tools; never the owner's home, other repositories,
this study's hidden tests or reference solutions (fixture agents excepted, by
design), or any session's transcript, grade or tracker reading, which the harness
writes beside the workspace, outside the sandbox (amendment 7).
"""

from __future__ import annotations

import argparse
import concurrent.futures
import itertools
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
import spec_study  # noqa: E402

STUDY = Path(os.environ.get("ADOPTION_STUDY_ROOT", "/mnt/4tb/tmp/openwarrant-adoption"))
KEEP = Path(os.environ.get("ADOPTION_KEEP_ROOT", "/mnt/4tb/artifacts/OpenWarrant/keep/adoption"))
# Where cells' working trees live while they run. Default under STUDY; ADOPTION_RUNS_ROOT=/dev/shm/...
# puts them in memory, away from a contended disk (Beads' embedded database syncs on every write).
RUNS = Path(os.environ.get("ADOPTION_RUNS_ROOT", str(STUDY / "runs")))
HOME = Path.home()

PINS = {
    "claude": "2.1.301",
    "war": "1.0.0-alpha.7",
    "bd": "1.3.1",
    "specify": "v1.2.2",
    "rust": "1.97.1",
    "pytest": "9.0.2",
    "ow_plugin": "v1.0.0-alpha.7",   # the OpenWarrant Claude Code plugin, from this tag
    "opencode": "2.2.1",
}
# Where `prepare` gets Beads and Spec Kit. Beads publishes checksums.txt: the pin below is its
# line for this asset, and the download is checked against both. Spec Kit's v1.2.2 release has no
# assets and no checksum, so it is pinned to the commit its annotated tag points at.
BEADS_URL = "https://github.com/gastownhall/beads/releases/download/v1.3.1"
BEADS_ASSET = "beads_1.3.1_linux_amd64.tar.gz"
BEADS_SHA256 = "3219443a9734b89b93fb16ee8d65844759fa1b3cd3cf139c606b7353cfb0715c"
SPECKIT_GIT = "https://github.com/github/spec-kit.git"
SPECKIT_COMMIT = "74be0a63e93c7087c5b06556b7561810d8f78a45"  # tag v1.2.2
OW_REPO = Path(os.environ.get("ADOPTION_OW_REPO", "/mnt/4tb/OpenWarrant"))
PLUGIN_FILES = (".claude-plugin", ".claude/skills", ".claude/hooks", ".mcp.json")
DEFAULT_MODEL = "claude-sonnet-5-5"

# opencode, the second agent: copied from the owner's install at the pin by `prepare`. Its models
# come from the LithosAI endpoint (`--model lithosai/<id>`), whose key reaches the sandbox on a
# file descriptor, never in an argv or the environment.
OPENCODE_SRC = HOME / ".bun/install/global/node_modules/@opencode/cli/bin/opencode.exe"
OPENCODE_KEY_ENV = "LITHOSAI_API_KEY"
OPENCODE_KEY_PATH = "/run/secrets/lithosai"
_REASONING = {"tools": True, "reasoning": {"supported": True}, "input": ["text"], "output": ["text"]}
OPENCODE_PROVIDERS = {
    "lithosai": {
        "npm": "@ai-sdk/openai-compatible", "name": "LithosAI",
        "options": {"baseURL": "https://api.lithosai.cloud/v1", "apiKey": "{file:" + OPENCODE_KEY_PATH + "}"},
        "models": {
            "deepseek-ai/DeepSeek-V4.1-Flash": {
                "name": "DeepSeek V4.1 Flash", "limit": {"context": 1000000, "output": 384000},
                "capabilities": _REASONING, "compatibility": {"reasoningField": "reasoning_content"}},
            "zai-org/GLM-5.3-Flash": {
                "name": "GLM 5.3 Flash", "limit": {"context": 1000000, "output": 131072},
                "capabilities": _REASONING, "compatibility": {"reasoningField": "reasoning_content"}},
        },
    },
}
# opencode 2.2.1 reads AGENTS.md and never CLAUDE.md, so under it every arm keeps the project's
# instructions in AGENTS.md, and Spec Kit installs its opencode commands (arms.py).
AGENT_SEED = {"opencode": {"STUDY_INSTRUCTIONS": "AGENTS.md", "STUDY_SPECKIT_INTEGRATION": "opencode"}}
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
    "spec": "Do backlog item {id}.",   # PROTOCOL-spec.md: every arm, and no title an author didn't write
}
SCENARIOS = ("directed", "relay", "parallel")


# ------------------------------------------------------------------ tools


# Amendment 5: OpenWarrant versions as arms. A published version names its installed release and
# the tag its plugin comes from; `next` names a local build (ADOPTION_WAR_NEXT, main as alpha.8
# develops) and the commit it was built from (ADOPTION_OW_PLUGIN_REF_NEXT). Nothing else differs.
OW_VERSIONS = {
    "openwarrant@a6": {"war": str(HOME / ".local/lib/openwarrant/1.0.0-alpha.6/bin/war"),
                       "version": "1.0.0-alpha.6", "plugin_ref": "v1.0.0-alpha.6"},
    "openwarrant@a7": {"war": str(HOME / ".local/lib/openwarrant/1.0.0-alpha.7/bin/war"),
                       "version": "1.0.0-alpha.7", "plugin_ref": "v1.0.0-alpha.7"},
    "openwarrant@next": {"war_env": "ADOPTION_WAR_NEXT", "plugin_env": "ADOPTION_OW_PLUGIN_REF_NEXT"},
}


def arm_war(arm: str = "") -> str:
    """The `war` an arm's sandbox gets. Unversioned arms keep the run-wide choice: ADOPTION_WAR, a
    local build to measure instead of the pinned release, or the pin."""
    v = OW_VERSIONS.get(arms.version_arm(arm))
    if v:
        return v["war"] if "war" in v else os.environ.get(v["war_env"], "")
    return os.environ.get("ADOPTION_WAR") or str(HOME / ".local/lib/openwarrant" / PINS["war"] / "bin/war")


def arm_plugin_ref(arm: str = "") -> str:
    v = OW_VERSIONS.get(arms.version_arm(arm))
    if v:
        return v["plugin_ref"] if "plugin_ref" in v else os.environ.get(v["plugin_env"], "")
    return os.environ.get("ADOPTION_OW_PLUGIN_REF", PINS["ow_plugin"])


def plugin_dir(arm: str = "") -> Path:
    """Where the arm's Claude Code plugin is unpacked. The unversioned arm keeps `prepare`'s copy."""
    if arms.version_arm(arm) in OW_VERSIONS:
        return STUDY / "tools" / f"ow-plugin-{arm_plugin_ref(arm)}"
    return STUDY / "tools" / "ow-plugin"


def unpack_plugin(dest: Path, ref: str) -> None:
    dest.mkdir(parents=True)
    archive = subprocess.run(["git", "-C", str(OW_REPO), "archive", ref, *PLUGIN_FILES],
                             capture_output=True, check=True, timeout=120).stdout
    subprocess.run(["tar", "-x", "-C", str(dest)], input=archive, check=True, timeout=120)


_BUILDS: dict = {}


def arm_build(arm: str) -> dict:
    """What an arm ran: its `war` (path, version string, sha256) and its plugin ref. Recorded in
    run.json and in every cell's result."""
    if arm not in _BUILDS:
        war = arm_war(arm)
        out = ""
        if war and Path(war).exists():
            out = subprocess.run([war, "--version"], capture_output=True, text=True, timeout=60).stdout.strip()
        _BUILDS[arm] = {"war": war, "war_version": out,
                        "war_sha256": _sha256(Path(war)) if war and Path(war).exists() else None,
                        "plugin_ref": arm_plugin_ref(arm)}
    return _BUILDS[arm]


def tools(arm: str = "") -> dict:
    t = STUDY / "tools"
    return {
        "STUDY_WAR": arm_war(arm),
        "STUDY_BD": str(t / "beads/bd"),
        "STUDY_SPECIFY": str(t / "bin/specify"),
        "STUDY_CLAUDE": str(HOME / ".local/share/claude/versions" / PINS["claude"]),
        "STUDY_OPENCODE": str(t / "opencode" / PINS["opencode"] / "opencode"),
    }


def check_tools(agent: str = "claude", arm_list: list[str] | None = None) -> list[str]:
    problems = []
    for arm in sorted(a for a in set(arm_list or []) if arms.version_arm(a) in OW_VERSIONS):
        v, war, ref = OW_VERSIONS[arms.version_arm(arm)], arm_war(arm), arm_plugin_ref(arm)
        if not war or not Path(war).exists():
            problems.append(f"{arm}: no war at {war!r}" + (f" (set {v['war_env']})" if "war_env" in v else ""))
            continue
        out = subprocess.run([war, "--version"], text=True, capture_output=True, timeout=60).stdout
        if "version" in v and v["version"] not in out:
            problems.append(f"{arm}: {war} reports {out.strip()[:80]!r}, not {v['version']}")
        if not ref:
            problems.append(f"{arm}: no plugin ref (set {v['plugin_env']} to the commit the build came from)")
        elif subprocess.run(["git", "-C", str(OW_REPO), "rev-parse", "--verify", "-q", f"{ref}^{{commit}}"],
                            capture_output=True, timeout=60).returncode != 0:
            problems.append(f"{arm}: plugin ref {ref!r} is not a commit in {OW_REPO}")
    # Amendment 4: a re-measure of one arm need not have every other arm's tools.
    only_ow = os.environ.get("ADOPTION_ONLY_OPENWARRANT") == "1"
    probes = {
        "STUDY_WAR": (["--version"], PINS["war"]),
        "STUDY_BD": (["version"], PINS["bd"]),
        "STUDY_CLAUDE": (["--version"], PINS["claude"]),
        "STUDY_OPENCODE": (["--version"], PINS["opencode"]),
    }
    for key, path in tools().items():
        if key == "STUDY_OPENCODE" and agent != "opencode":
            continue
        if only_ow and key in ("STUDY_BD", "STUDY_SPECIFY"):
            continue
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
    if not only_ow and not spec.exists():
        problems.append("specify: not installed (run.py prepare)")
    elif not only_ow and not speckit_at_pin():
        problems.append(f"specify: {spec} is not Spec Kit {PINS['specify']} at {SPECKIT_COMMIT[:12]}")
    return problems


def speckit_at_pin() -> bool:
    receipt = STUDY / "tools/uv-tools/specify-cli/uv-receipt.toml"
    return receipt.exists() and SPECKIT_COMMIT in receipt.read_text()


def harness_commit() -> dict:
    """The harness's own commit, and whether its files differ from it: the run names the code it ran."""
    head = subprocess.run(["git", "-C", str(HERE), "rev-parse", "HEAD"], capture_output=True, text=True,
                          timeout=30).stdout.strip()
    dirty = subprocess.run(["git", "-C", str(HERE), "status", "--porcelain", "--", "."], capture_output=True,
                           text=True, timeout=30).stdout.strip()
    return {"commit": head, "dirty": bool(dirty)}


def _sha256(path: Path) -> str:
    import hashlib
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for block in iter(lambda: f.read(1 << 20), b""):
            h.update(block)
    return h.hexdigest()


def install_beads() -> None:
    """Beads at its pin, from the release tarball. Everything downloaded stays in its own new
    directory until its digest matches; only the `bd` binary leaves it."""
    import tarfile
    import tempfile
    bd = Path(tools()["STUDY_BD"])
    if bd.exists():
        out = subprocess.run([str(bd), "version"], capture_output=True, text=True, timeout=60).stdout
        if PINS["bd"] in out:
            print(f"beads {PINS['bd']}: already installed")
            return
    (STUDY / "tools").mkdir(parents=True, exist_ok=True)
    dl = Path(tempfile.mkdtemp(prefix="dl-beads-", dir=STUDY / "tools"))
    try:
        for name in (BEADS_ASSET, "checksums.txt"):
            subprocess.run(["curl", "-fsSL", "--max-time", "300", "-o", str(dl / name), f"{BEADS_URL}/{name}"],
                           check=True, timeout=330)
        got = _sha256(dl / BEADS_ASSET)
        published = next((l.split()[0] for l in (dl / "checksums.txt").read_text().splitlines()
                          if l.strip().endswith(BEADS_ASSET)), None)
        if got != BEADS_SHA256 or got != published:
            raise SystemExit(f"beads: {BEADS_ASSET} is {got}, pinned {BEADS_SHA256}, published {published}: refused")
        with tarfile.open(dl / BEADS_ASSET) as t:
            member = next(m for m in t.getmembers() if m.isfile() and Path(m.name).name == "bd")
            t.extract(member, dl / "x", filter="data")
        new = dl / "x" / member.name
        new.chmod(0o755)
        bd.parent.mkdir(parents=True, exist_ok=True)
        os.replace(new, bd)
    finally:
        shutil.rmtree(dl, ignore_errors=True)
    print(f"beads {PINS['bd']}: installed, sha256 of the tarball matches checksums.txt")


def install_speckit() -> None:
    """Spec Kit at its pin, as a uv tool under tools/, with uv's own cache in a new directory."""
    import tempfile
    if speckit_at_pin() and Path(tools()["STUDY_SPECIFY"]).is_symlink():
        print(f"spec kit {PINS['specify']}: already installed")
        return
    t = STUDY / "tools"
    (t / "uv-tools").mkdir(parents=True, exist_ok=True)
    (t / "bin").mkdir(parents=True, exist_ok=True)
    cache = Path(tempfile.mkdtemp(prefix="dl-speckit-", dir=t))
    env = {**os.environ, "UV_TOOL_DIR": str(t / "uv-tools"), "UV_TOOL_BIN_DIR": str(t / "bin"),
           "UV_CACHE_DIR": str(cache)}
    try:
        p = subprocess.run(["uv", "tool", "install", "--force", "specify-cli", "--from",
                            f"git+{SPECKIT_GIT}@{SPECKIT_COMMIT}"], env=env, capture_output=True,
                           text=True, timeout=600)
    finally:
        shutil.rmtree(cache, ignore_errors=True)
    if p.returncode != 0:
        raise SystemExit(f"spec kit: uv tool install failed: {p.stderr[-800:]}")
    print(f"spec kit {PINS['specify']}: installed from {SPECKIT_COMMIT[:12]}")


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


def ow_done_test(arm: str) -> bool:
    """Amendment 8: whether an arm's seed carries `[tickets] done_test`. Only `@next` arms can: the key
    is part of what `next` is, and alpha.7 and older refuse a key they don't know. On unless
    ADOPTION_OW_DONE_TEST=0; run.json records it."""
    return (arms.tool_of(arm) == "openwarrant" and arm.endswith("@next")
            and os.environ.get("ADOPTION_OW_DONE_TEST", "1") != "0")


def seed_extras(arm: str) -> dict:
    return {"STUDY_OW_DONE_TEST": "1"} if ow_done_test(arm) else {}


def war_knows_done_test(war: str) -> bool:
    """Whether a `war` reads `[tickets] done_test`."""
    return war_reads_tickets_key(war, 'done_test = "true"')


def war_reads_tickets_key(war: str, line: str) -> bool:
    """Whether a `war` reads a `[tickets]` key: a freshly set up repository with the key in it still
    lists its tickets. A `war` without the key refuses the table (`unknown field`)."""
    import tempfile
    env = arms.seed_env({})
    RUNS.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=RUNS) as d:
        subprocess.run(["git", "init", "-q", "-b", "main"], cwd=d, env=env, check=True, capture_output=True, timeout=60)
        if subprocess.run([war, "init", "--root", "."], cwd=d, env=env, capture_output=True, timeout=120).returncode != 0:
            return False
        arms.tickets_config(Path(d), line)
        p = subprocess.run([war, "view", "tickets", "--json"], cwd=d, env=env, capture_output=True, text=True, timeout=120)
        try:
            return json.loads(p.stdout).get("exit_code") == 0
        except ValueError:
            return False


def feature_problems(arm_list: list[str], agent: str, repos: list[str] | None = None) -> list[str]:
    """Amendment 8: what a plan's arms need of `next` that the build at hand may not have yet. A run
    stops before its first cell instead of measuring an arm that is silently its own twin."""
    problems = []
    for arm in sorted(set(arm_list)):
        war = arm_war(arm)
        if not war or not Path(war).exists():
            continue  # check_tools reports it
        keyed = repos is None or any(r in arms.DONE_TEST for r in repos)   # a repository with no command gets no key
        if keyed and ow_done_test(arm) and not war_knows_done_test(war):
            problems.append(f"{arm}: {war} does not read `[tickets] done_test`. Build `next` from a main that "
                            "has it, or set ADOPTION_OW_DONE_TEST=0 to run without the key (run.json records which)")
        if arms.with_brief(arm) and agent == "claude":
            problems.append(f"{arm}: the brief arm is opencode's (its `instructions` config); not run under {agent}")
    return problems


def seed_repo(repo: str, arm: str, dest: Path, done: list[str], extra: dict | None = None) -> dict:
    backlog = load_backlog(repo)
    materialize(repo, dest)
    apply_reference(repo, dest, done)
    arms.seed(arm, dest, backlog, set(done), {**tools(arm), **seed_extras(arm), **(extra or {})})
    if arms.with_brief(arm) and not (dest / arms.BRIEF_FILE).is_file():
        # The arm is its twin plus this file. Without it the cell would be the twin under another name.
        raise SystemExit(f"stopped: {arm}'s `war` wrote no {arms.BRIEF_FILE} into the seeded repository; "
                         "the brief arm waits for a `next` that keeps that file")
    return backlog


def seed_scaffold(repo: str, arm: str, dest: Path, done: list[str], extra: dict | None = None) -> dict:
    """The seed and earlier items' solutions, the tool set up, and no backlog anywhere in the tree."""
    backlog = load_backlog(repo)
    materialize(repo, dest)
    apply_reference(repo, dest, done)
    arms.scaffold(arm, dest, backlog, {**tools(arm), **seed_extras(arm), **(extra or {})})
    return backlog


def spec_store() -> Path:
    return KEEP / "spec-store"


def seed_spec(sb: "Sandbox", repo: str, arm: str, dest: Path, done: list[str], digest: str,
              extra: dict | None = None) -> dict:
    """A stored spec in place of backlog.toml. The store is checked against the digest first, and the
    spec is laid over the scaffold in the sandbox, since an author wrote it."""
    spec_study.verify_stored(spec_store(), digest)
    backlog = seed_scaffold(repo, arm, dest, done, extra)
    out = sb.inside(["_spec_apply", arm], f"{sb.WORK}/{dest.name}",
                    binds=[("--ro-bind", spec_store() / digest, "/opt/spec")])
    if "error" in out:
        raise SystemExit(f"stopped: spec {digest[:12]} could not be seeded: {out['error']}")
    return backlog


def cargo_lock(repo: str) -> str | None:
    """Cargo.lock at the repository's pin, or None for a repository that is not built with cargo."""
    with open(repo_def(repo) / "source.toml", "rb") as f:
        src = tomllib.load(f)
    if src["kind"] != "git":
        return None
    p = subprocess.run(["git", "-C", src["path"], "show", f"{src['commit']}:Cargo.lock"],
                       capture_output=True, text=True, timeout=60)
    return p.stdout if p.returncode == 0 else None


def registry_problem(repo: str, cargo_home: Path | None = None) -> str | None:
    """Is the offline cargo registry there? Run `rem-c4` found it emptied during a run (cause not
    found), and every cell then failed on `no matching package`, which reads like the agent's fault.
    Checked per crate from the lockfile, in the index and the download cache both."""
    lock = cargo_lock(repo)
    if lock is None:
        return None
    home = cargo_home or STUDY / "cargo-home"
    crates = [f"{c['name']}-{c['version']}.crate" for c in tomllib.loads(lock).get("package", [])
              if str(c.get("source", "")).startswith("registry+")]
    cached = {f.name for d in (home / "registry/cache").glob("*") for f in d.iterdir()} \
        if (home / "registry/cache").is_dir() else set()
    missing = [c for c in crates if c not in cached]
    indexed = any((home / "registry/index").glob("*/.cache/*")) if (home / "registry/index").is_dir() else False
    if missing or not indexed:
        return (f"{repo}: the cargo registry at {home} is missing {len(missing)} of {len(crates)} crates"
                f"{'' if indexed else ' and has no index'} (run.py prepare restores it)")
    return None


# ------------------------------------------------------------------ the sandbox

# How an agent session authenticates. "token": CLAUDE_CODE_OAUTH_TOKEN (from `claude
# setup-token`, the subscription); "api": ANTHROPIC_API_KEY (metered, exact cost);
# "credfile": bind ~/.claude/.credentials.json (fallback: token refresh may fail).
# The variable is passed into the sandbox's environment only, never written to disk.
AUTH_ENV = {"token": "CLAUDE_CODE_OAUTH_TOKEN", "api": "ANTHROPIC_API_KEY"}
# Each networked session's sccache server gets its own port, so sandboxes never share one. The ports
# cycle (a full study has more sessions than the range); ADOPTION_PORT_BASE keeps a second harness
# running beside a study off the study's ports.
_PORT_BASE = int(os.environ.get("ADOPTION_PORT_BASE", "42300"))
_PORT = itertools.cycle(range(_PORT_BASE, _PORT_BASE + 699))
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


def workspace(cell: Path) -> Path:
    """The only part of a cell a sandbox mounts (at /work): the repository, worktrees and build
    output. The harness's records of each session (`sNN/`, `aN/`, `merged/`: transcripts, grades,
    tracker readings) and `result.json` sit beside it in the cell and are never mounted, so no
    session reads another's grade, or its hidden-test output (amendment 7)."""
    return cell / "work"


class Sandbox:
    """One cell's view of the machine. Only `workspace(cell)` is mounted, at /work, read-write."""

    WORK = "/work"
    AHOME = "/home/agent"

    def __init__(self, cell: Path, repo: str, fixture: bool = False, arm: str = "", agent: str = "claude",
                 layout: str = "", binds: list | None = None, work: Path | None = None):
        self.cell, self.repo, self.fixture, self.arm, self.agent = cell, repo, fixture, arm, agent
        # `work`: the directory mounted at /work, when it isn't the cell's own workspace (`run.py seed`).
        self.ws = work or workspace(cell)
        # `layout`: whose files the repository was seeded with, when that isn't `agent`'s own: the
        # spec study's workers are opencode's, and a fixture stands in for them.
        # `binds`: (flag, source, destination), for a session's own extra mounts.
        self.layout, self.binds = layout, binds or []

    def _code_binds(self) -> list[str]:
        """The harness's own files and this repository's backlog.toml, never hidden/ or reference/."""
        a = []
        for f in ("run.py", "arms.py", "fixture_agent.py", "jira_arm.py", "spec_study.py"):
            a += ["--ro-bind", str(HERE / f), f"/opt/study/code/{f}"]
        a += ["--ro-bind", str(repo_def(self.repo) / "backlog.toml"),
              f"/opt/study/code/repos/{self.repo}/backlog.toml"]
        return a

    def argv(self, inner: list[str], cwd: str, net: bool, hidden: Path | None = None,
             config: Path | None = None, extra_env: dict | None = None,
             harness: bool = False, key_fd: int | None = None, binds: list | None = None) -> list[str]:
        t = tools(self.arm)
        self.ws.mkdir(parents=True, exist_ok=True)
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
              "--bind", str(self.ws), self.WORK]
        if arms.tool_of(self.arm) == "openwarrant" and not harness and self.agent == "claude":
            # The plugin QUICKSTART tells a Claude Code user to install: skills, MCP server, hooks.
            a += ["--ro-bind", str(plugin_dir(self.arm)), "/opt/study/ow-plugin"]
        if self.agent == "opencode" and not harness:
            a += ["--ro-bind", t["STUDY_OPENCODE"], "/opt/study/bin/opencode"]
        if key_fd is not None:
            a += ["--dir", str(Path(OPENCODE_KEY_PATH).parent), "--ro-bind-data", str(key_fd), OPENCODE_KEY_PATH]
        if harness or self.fixture:
            a += self._code_binds()
        if self.fixture and not harness:
            # A fixture agent is a script that applies the reference solution: it must see it.
            a += ["--ro-bind", str(repo_def(self.repo)), "/opt/study/repo-def"]
        if hidden is not None:
            a += ["--ro-bind", str(hidden), "/opt/hidden"]
        for flag, src, dst in (binds or []) + ([] if harness else self.binds):
            a += [flag, str(src), dst]
        if config is not None and self.agent == "opencode":
            a += ["--bind", str(config), f"{self.AHOME}/.config/opencode"]
        elif config is not None:
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
        if self.agent == "opencode":
            env.update({"OPENCODE_DISABLE_AUTOUPDATE": "1", "OPENCODE_DISABLE_MODELS_FETCH": "1"})
        if harness:
            env.update(AGENT_SEED.get(self.layout or self.agent, {}))  # `_answers` writes the agent's instruction file
        mode = auth_mode()
        if net and config is not None and self.agent == "claude" and mode in AUTH_ENV:
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
            extra_env=None, stdout=None, stderr=None, harness=False, binds=None) -> subprocess.CompletedProcess:
        key_fd = None
        if self.agent == "opencode" and net and config is not None and not harness:
            # The provider key, on a pipe bwrap copies into a file only the sandbox sees.
            key_fd, w = os.pipe()
            os.write(w, os.environ[OPENCODE_KEY_ENV].encode())
            os.close(w)
        argv = self.argv(inner, cwd, net, hidden, config, extra_env, harness, key_fd, binds)
        try:
            p = subprocess.Popen(argv, stdout=stdout or subprocess.PIPE, stderr=stderr or subprocess.PIPE,
                                 text=True, start_new_session=True, pass_fds=(key_fd,) if key_fd is not None else ())
        finally:
            if key_fd is not None:
                os.close(key_fd)
        try:
            out, err = p.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            os.killpg(p.pid, signal.SIGKILL)
            out, err = p.communicate()
            return subprocess.CompletedProcess(argv, -9, out, (err or "") + "\n[harness: timed out]")
        return subprocess.CompletedProcess(argv, p.returncode, out, err)

    def inside(self, sub: list[str], cwd: str, timeout=900, hidden=None, target: str = "target",
               binds=None) -> dict:
        """run.py's own in-sandbox subcommands (`_grade`, `_tracker`, `_git`); JSON out, no network."""
        p = self.run(["python3", "/opt/study/code/run.py", *sub], cwd, timeout=timeout, hidden=hidden,
                     harness=True, extra_env={"CARGO_TARGET_DIR": f"{self.WORK}/{target}"}, binds=binds)
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


def _spec_env() -> dict:
    return {**os.environ, **arms.SEED_ENV, "STUDY_WAR": "/opt/study/bin/war", "STUDY_BD": "/opt/study/bin/bd"}


def _inside_spec_capture(tool: str, out: str, base: str | None) -> dict:
    res = spec_study.capture_spec(tool, Path.cwd(), Path(out), _spec_env())
    if base:
        res["changed_outside_spec"] = spec_study.changed_outside_spec(tool, Path.cwd(), base)
    return res


def _inside_spec_apply(arm: str) -> dict:
    env = _spec_env()
    res = spec_study.apply_spec(arm, Path.cwd(), Path("/opt/spec/files"), env)
    g = ["git", "-c", "core.hooksPath=/dev/null", "-c", "core.fsmonitor=false"]
    subprocess.run([*g, "add", "-A"], capture_output=True, timeout=300, env=env)
    subprocess.run([*g, "commit", "-q", "--no-verify", "-m", f"seed: {arm} spec"], capture_output=True,
                   timeout=300, env=env)
    return res


def _inside_spec_tracker(arm: str, item: str) -> dict:
    return {item: spec_study.spec_done(arm, Path.cwd(), _spec_env())}


# ------------------------------------------------------------------ one agent session


ARM_MCP = {
    # Each arm's MCP servers as its own setup installs them: only the OpenWarrant plugin has one.
    "openwarrant": {"openwarrant": {"command": "war", "args": ["mcp"]}},
}


# Under opencode, each arm's MCP servers as its documentation sets them up for a harness with a
# shell. OpenWarrant's QUICKSTART said "any other harness: war admin mcp" until 2026-10-30, when it
# changed to "a harness with a shell needs nothing more" (amendment 3); runs from then use none.
OPENCODE_ARM_MCP = {} if os.environ.get("ADOPTION_OC_OW_MCP") != "1" else ARM_MCP


def agent_argv(agent: str, prompt: str, model: str, max_turns: int, arm: str = "",
               budget: float | None = None, no_web: bool = False) -> list[str]:
    arm = arms.tool_of(arm)
    budget = SESSION_BUDGET_USD if budget is None else budget   # `execute --session-budget-usd` sets the global
    if agent == "claude":
        argv = ["/opt/study/bin/claude", "-p", prompt, "--model", model,
                "--output-format", "stream-json", "--verbose",
                "--permission-mode", "bypassPermissions",
                "--strict-mcp-config", "--mcp-config", json.dumps({"mcpServers": ARM_MCP.get(arm, {})}),
                "--setting-sources", "project,local", "--no-session-persistence",
                "--max-turns", str(max_turns), "--max-budget-usd", str(budget)]
        if arm == "openwarrant":
            argv += ["--plugin-dir", "/opt/study/ow-plugin"]
        if no_web:
            argv += ["--disallowedTools", "WebFetch,WebSearch"]
        return argv
    if agent == "opencode":
        # The turn cap is the build agent's `steps` in the session's config (opencode_config).
        return ["/opt/study/bin/opencode", "run", "--standalone", "--format", "json", "-m", model, prompt]
    if agent in ("fixture-reference", "fixture-noop", "fixture-claim", "fixture-claim-broken", "fixture-spec"):
        return ["python3", "/opt/study/code/fixture_agent.py", agent.split("-", 1)[1]]
    if agent == "fixture-probe":
        return ["/usr/bin/python3", "/opt/study/fixture_probe.py"]
    if agent in ("fixture-author-good", "fixture-author-bad", "fixture-author-empty", "fixture-author-probe"):
        return ["/usr/bin/python3", "/opt/study/fixture_author.py", agent.rsplit("-", 1)[1]]
    raise SystemExit(f"unknown agent {agent!r}")


TRACKER_PATHS = ("CLAUDE.md", "PLAN.md", "specs/", ".specify/", ".beads/", "docs/tickets/",
                 "openwarrant.toml", "AGENTS.md", ".openwarrant/")

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
    first_edit_turn = None
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
                first_edit_turn = len(msgs) - 1  # assistant messages before the one that edits
    out = {
        "assistant_messages": len(msgs),  # distinct message ids
        "tools": tools_used,
        "tracker_bash_calls": bash_tracker_calls,
        "context_at_first_code_edit": first_edit_ctx,
        "tokens_processed_before_first_code_edit": processed_before_edit if first_edit_ctx is not None else None,
        "turns_before_first_code_edit": first_edit_turn,
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


_OC_EDITS = ("edit", "write", "patch", "multiedit")


def parse_opencode(path: Path, max_turns: int) -> dict:
    """Metrics from an `opencode run --format json` stream, in parse_stream's terms.

    Each step (one model call) ends in a `step_finish` event carrying that call's tokens; its
    tool calls arrive before it as `tool_use` events with the same messageID. A step's context
    is its input plus cache reads and writes, as for Claude Code."""
    if not path.exists():
        return {"error": "no transcript"}
    steps: list[dict] = []
    tools_used: dict[str, int] = {}
    edit_msgs: set = set()
    bash_tracker_calls = 0
    errors: list[str] = []
    stamps: list[int] = []
    for line in path.read_text(errors="replace").splitlines():
        try:
            ev = json.loads(line)
        except ValueError:
            continue
        if isinstance(ev.get("timestamp"), int):
            stamps.append(ev["timestamp"])
        kind, part = ev.get("type"), ev.get("part") or {}
        if kind == "tool_use":
            name = str(part.get("tool", "?"))
            tools_used[name] = tools_used.get(name, 0) + 1
            inp = (part.get("state") or {}).get("input") or {}
            if name in _OC_EDITS:
                target = str(inp.get("path") or inp.get("filePath") or inp.get("file_path") or "")
                text = str(inp.get("patchText", "")) if name == "patch" else ""
                for t in TRACKER_PATHS:
                    text = text.replace(t, "")
                if (target and not any(t in target for t in TRACKER_PATHS)) or _SOURCE.search(text):
                    edit_msgs.add(part.get("messageID"))
            if name in ("shell", "bash"):
                cmd = str(inp.get("command", ""))
                if bash_writes_code(cmd):
                    edit_msgs.add(part.get("messageID"))
                if any(cmd.lstrip().startswith(w) or f" {w} " in cmd for w in ("war", "bd")) \
                        or "PLAN.md" in cmd or "tasks.md" in cmd:
                    bash_tracker_calls += 1
        elif kind == "step_finish":
            tok = part.get("tokens") or {}
            cache = tok.get("cache") or {}
            steps.append({"msg": part.get("messageID"), "reason": part.get("reason"),
                          "input": tok.get("input") or 0, "read": cache.get("read") or 0,
                          "write": cache.get("write") or 0,
                          "output": (tok.get("output") or 0) + (tok.get("reasoning") or 0),
                          "cost": part.get("cost") or 0})
        elif kind == "error":
            errors.append(json.dumps(ev.get("error") or part)[:400])
    ctxs = [st["input"] + st["read"] + st["write"] for st in steps]
    first_edit_ctx, first_edit_turn, processed = None, None, 0
    for n, (st, ctx) in enumerate(zip(steps, ctxs)):
        processed += ctx + st["output"]
        if st["msg"] in edit_msgs:
            first_edit_ctx, first_edit_turn = ctx, n  # steps before the one that edits
            break
    out = {
        "assistant_messages": len({st["msg"] for st in steps}),
        "tools": tools_used,
        "tracker_bash_calls": bash_tracker_calls,
        "context_at_first_code_edit": first_edit_ctx,
        "tokens_processed_before_first_code_edit": processed if first_edit_ctx is not None else None,
        "turns_before_first_code_edit": first_edit_turn,
        "peak_context": max(ctxs) if ctxs else 0,
        "num_turns": len(steps),
        "subtype": ("error_max_turns" if len(steps) >= max_turns else "success") if steps and not errors
                   else "error_during_execution",
        "is_error": bool(errors),
        "duration_ms": (max(stamps) - min(stamps)) if stamps else None,
        "cost_usd": round(sum(st["cost"] for st in steps), 6),  # 0 when the provider reports no price
        "input_tokens": sum(st["input"] for st in steps),
        "output_tokens": sum(st["output"] for st in steps),
        "cache_read_tokens": sum(st["read"] for st in steps),
        "cache_creation_tokens": sum(st["write"] for st in steps),
    }
    if errors:
        out["errors"] = errors[:5]
        if not steps:
            out["api_error"] = errors[0]  # the model never ran: the harness's problem, not the arm's
    return out


def opencode_config(arm: str, max_turns: int, no_web: bool = False) -> dict:
    """A session's opencode config: the provider, every tool allowed (the sandbox is the boundary,
    as with Claude Code's bypassPermissions), the turn cap, and the arm's MCP server."""
    mcp, full_arm = arms.with_mcp(arm), arm
    arm = arms.tool_of(arm)
    cfg = {
        "$schema": "https://opencode.ai/config.json",
        "autoupdate": False, "share": "disabled",
        "permission": {k: "allow" for k in ("edit", "bash", "webfetch", "external_directory", "doom_loop")},
        "agent": {"build": {"steps": max_turns}},
        "provider": OPENCODE_PROVIDERS,
    }
    if no_web:
        cfg["permission"].update({"webfetch": "deny", "websearch": "deny"})
    # Amendment 6: an `openwarrant-mcp@X` arm registers `war mcp` (its default tool set) as a local,
    # stdio MCP server, the way opencode's `mcp` config section takes one; opencode starts it in the
    # session's directory, and `war mcp` finds the repository from there.
    servers = {name: {"type": "local", "command": [d["command"], *d["args"]], "enabled": True}
               for name, d in (ARM_MCP if mcp else OPENCODE_ARM_MCP).get(arm, {}).items()}
    if servers:
        cfg["mcp"] = servers
    if arms.with_brief(full_arm):
        # Amendment 8: opencode's `instructions` takes files to load beside AGENTS.md at startup. A
        # relative path is looked up from the session's directory, the repository (or its worktree).
        cfg["instructions"] = [arms.BRIEF_FILE]
    return cfg


def run_session(sb: Sandbox, sdir: Path, cwd: str, prompt: str, agent: str, model: str,
                max_turns: int, extra_env: dict | None = None, arm: str = "",
                budget: float | None = None, no_web: bool = False) -> dict:
    sdir.mkdir(parents=True, exist_ok=True)
    config = sdir / ("opencode-config" if agent == "opencode" else "claude-config")
    config.mkdir(exist_ok=True)
    if agent == "opencode":
        (config / "opencode.json").write_text(json.dumps(opencode_config(arm, max_turns, no_web), indent=1))
    (sdir / "prompt.txt").write_text(prompt + "\n")
    t0 = time.time()
    with open(sdir / "stream.jsonl", "w") as out, open(sdir / "stderr.txt", "w") as err:
        env = {**(extra_env or {}), **(jira_env(sb.cell) if arm == "jira" else {})}
        p = sb.run(agent_argv(agent, prompt, model, max_turns, arm, budget, no_web), cwd,
                   net=(agent in ("claude", "opencode") or arm == "jira"),
                   timeout=SESSION_TIMEOUT, config=config, extra_env=env, stdout=out, stderr=err)
    metrics = (parse_opencode(sdir / "stream.jsonl", max_turns) if agent == "opencode"
               else parse_stream(sdir / "stream.jsonl"))
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


def observe(sb: Sandbox, sdir: Path, cwd: str, repo: str, arm: str, target: str = "target",
            spec_item: str | None = None) -> dict:
    """After a session: grade (no network, hidden tests mounted read-only) and read the tracker."""
    sdir.mkdir(parents=True, exist_ok=True)
    grade = sb.inside(["_grade", repo], cwd, timeout=3600, hidden=repo_def(repo) / "hidden", target=target)
    tracker = (sb.inside(["_spec_tracker", arm, spec_item], cwd, timeout=300, target=target) if spec_item
               else read_tracker(sb, cwd, repo, arm, target))
    (sdir / "grade.json").write_text(json.dumps(grade, indent=1))
    (sdir / "tracker.json").write_text(json.dumps(tracker, indent=1))
    passing = {k: v["pass"] for k, v in grade.get("items", {}).items()}
    return {"pass": passing, "tracker": tracker,
            "false_completions": sorted(k for k, v in tracker.items() if v is True and not passing.get(k)),
            "unrecorded_completions": sorted(k for k, v in tracker.items() if v is False and passing.get(k))}


# ------------------------------------------------------------------ scenarios


def cell_dir(run_id: str, c: dict) -> Path:
    if c["scenario"] == "spec":
        return RUNS / run_id / "spec" / c["repo"] / c["arm"] / c["author"] / c["item"] / f"s{c['spec_n']}r{c['rep']}"
    return RUNS / run_id / c["scenario"] / c["repo"] / c["arm"] / c.get("item", "-") / f"r{c['rep']}"


def run_cell(run_id: str, c: dict, agent: str, model: str, turns: int = SESSION_TURNS) -> dict:
    cell = cell_dir(run_id, c)
    if (cell / "result.json").exists():
        return json.loads((cell / "result.json").read_text())
    if cell.exists():
        shutil.rmtree(cell)
    problem = registry_problem(c["repo"])
    if problem:
        raise SystemExit(f"stopped before a cell: {problem}")
    cell.mkdir(parents=True)
    workspace(cell).mkdir()
    backlog = load_backlog(c["repo"])
    layout = "opencode" if c["scenario"] == "spec" else ""
    fixture_env = {"STUDY_ARM": arms.tool_of(c["arm"]), "STUDY_REPO": c["repo"]}
    if agent == "fixture-probe":
        # The probe sees exactly what a real agent sees, plus its own script; it is told where the
        # study's files are on the host, and their names, only so it can look for them.
        sb = Sandbox(cell, c["repo"], arm=c["arm"], agent=agent, layout=layout,
                     binds=[("--ro-bind", HERE / "fixture_probe.py", "/opt/study/fixture_probe.py")])
        fixture_env.update(probe_env(c["repo"], cell))
    else:
        sb = Sandbox(cell, c["repo"], fixture=agent.startswith("fixture"), arm=c["arm"], agent=agent,
                     layout=layout)
    result: dict = {"cell": c, "agent": agent, "model": model, "turn_cap": turns,
                    "build": arm_build(c["arm"]), "sessions": [],
                    "done_test": arms.done_test_command(backlog) if ow_done_test(c["arm"]) else None,
                    "brief_file": arms.BRIEF_FILE if arms.with_brief(c["arm"]) else None}
    t0 = time.time()
    extra = {**(jira_env(cell) if c["arm"] == "jira" else {}), **AGENT_SEED.get(layout or agent, {})} or None
    try:
        _run_cell_body(c, cell, sb, backlog, agent, model, fixture_env, result, extra, turns)
    finally:
        if c["arm"] == "jira":
            result["jira_issues_deleted"] = jira_arm.cleanup(jira_label(cell), jira_env(cell))
    problem = registry_problem(c["repo"])
    if problem:  # emptied while this cell ran: its grade says nothing about the agent
        raise SystemExit(f"stopped during a cell, nothing scored: {problem}")
    result["wall_secs"] = round(time.time() - t0, 1)
    result["cost_usd"] = round(sum((s.get("metrics") or {}).get("cost_usd") or 0 for s in result["sessions"]), 4)
    (cell / "result.json").write_text(json.dumps(result, indent=1))
    keep_cell(run_id, c, cell)
    return result


def _run_cell_body(c: dict, cell: Path, sb: "Sandbox", backlog: dict, agent: str, model: str,
                   fixture_env: dict, result: dict, extra: dict | None, turns: int = SESSION_TURNS) -> None:

    if c["scenario"] == "spec":
        idx = backlog["order"].index(c["item"])
        earlier = backlog["order"][:idx]
        item = next(i for i in backlog["item"] if i["id"] == c["item"])
        result.update({"author": c["author"], "worker": c["worker"], "spec_slot": c["spec"],
                       "spec_digest": c.get("spec_digest"), "turn_cap": spec_study.WORKER_TURNS})
        if c["author"] == "backlog":    # the control: the study's own text, as in the adoption study
            seed_repo(c["repo"], c["arm"], sb.ws / "repo", earlier, extra)
            result["answers"] = deliver(sb, f"{sb.WORK}/repo", c["repo"], c["arm"])
        else:
            seed_spec(sb, c["repo"], c["arm"], sb.ws / "repo", earlier, c["spec_digest"], extra)
        m = run_session(sb, cell / "s01", f"{sb.WORK}/repo", PROMPTS["spec"].format(id=item["id"]), agent, model,
                        spec_study.WORKER_TURNS, {**fixture_env, "STUDY_TARGET": item["id"],
                                                  "STUDY_AUTHOR": c["author"]}, arm=c["arm"])
        sb.inside(["_git", "snapshot", "s01"], f"{sb.WORK}/repo")
        obs = observe(sb, cell / "s01", f"{sb.WORK}/repo", c["repo"], c["arm"],
                      spec_item=None if c["author"] == "backlog" else item["id"])
        result["sessions"].append({"metrics": m, **obs})
        result.update({
            "success": bool(obs["pass"].get(item["id"])),
            "regressions": sorted(k for k in earlier if not obs["pass"].get(k)),
            "tracker_says_done": obs["tracker"].get(item["id"]),
        })

    elif c["scenario"] == "directed":
        idx = backlog["order"].index(c["item"])
        seed_repo(c["repo"], c["arm"], sb.ws / "repo", backlog["order"][:idx], extra)
        item = next(i for i in backlog["item"] if i["id"] == c["item"])
        result["answers"] = deliver(sb, f"{sb.WORK}/repo", c["repo"], c["arm"])
        prompt = PROMPTS["directed"].format(id=item["id"], title=item["title"])
        m = run_session(sb, cell / "s01", f"{sb.WORK}/repo", prompt, agent, model, turns,
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
        seed_repo(c["repo"], c["arm"], sb.ws / "repo", [], extra)
        n_max = len(backlog["item"]) + 2
        stalled = 0
        prev_pass: dict = {}
        prev_tracker: dict = read_tracker(sb, f"{sb.WORK}/repo", c["repo"], c["arm"])
        for s in range(1, n_max + 1):
            sdir = cell / f"s{s:02d}"
            cap = INTERRUPT_TURNS if s == 2 else turns
            m = run_session(sb, sdir, f"{sb.WORK}/repo", PROMPTS["relay"], agent, model, cap, fixture_env,
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
        seed_repo(c["repo"], c["arm"], sb.ws / "repo", [], extra)
        result["answers"] = deliver(sb, f"{sb.WORK}/repo", c["repo"], c["arm"])
        sb.inside(["_git", "worktrees", str(k)], f"{sb.WORK}/repo")
        finished: list[tuple[float, int, dict]] = []
        lock = threading.Lock()

        def one(j: int):
            m = run_session(sb, cell / f"a{j}", f"{sb.WORK}/w{j}", PROMPTS["parallel"], agent, model,
                            turns, {**fixture_env, "CARGO_TARGET_DIR": f"{sb.WORK}/target-{j}"},
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
    ws = workspace(cell)
    if (ws / "repo").exists():
        Sandbox(cell, c["repo"]).inside(["_git", "bundle", "/work/repo.bundle"], "/work/repo")
    dest.mkdir(parents=True, exist_ok=True)
    for f in cell.rglob("*"):
        if ws in f.parents:
            continue  # what an agent wrote is kept as the bundle, never by file name
        if f.is_file() and f.name in ("stream.jsonl", "stderr.txt", "metrics.json", "grade.json",
                                       "tracker.json", "prompt.txt", "result.json", "spec.json",
                                       "questions.jsonl", "probe.json"):
            rel = f.relative_to(cell)
            (dest / rel.parent).mkdir(parents=True, exist_ok=True)
            shutil.copy2(f, dest / rel)
    for f in ("repo.bundle", "probe.json"):  # the bundle, and a fixture author's probe report
        if (ws / f).exists():
            shutil.copy2(ws / f, dest / f)
    if ws.name == "work" and ws.parent == cell and RUNS in cell.parents:
        shutil.rmtree(ws, ignore_errors=True)


def probe_env(repo: str, cell: Path) -> dict:
    """What `fixture_probe.py` tries to reach, and the names it looks for (amendment 7)."""
    d = repo_def(repo)
    paths = [f"/work/s{n:02d}/{f}" for n in (1, 2, 3) for f in ("grade.json", "tracker.json", "stream.jsonl")]
    paths += [f"{cell}/s01/grade.json", str(cell), str(d / "hidden"), str(d / "reference"), str(d / "backlog.toml"),
              str(KEEP), str(RUNS), str(HERE), "/opt/hidden", "/opt/study/repo-def", "/opt/study/code",
              "/mnt", str(HOME)]
    names = {f.name for sub in ("hidden", "reference") for f in (d / sub).rglob("*")
             if f.is_file() and f.name not in ("conftest.py", "__init__.py")} | {"backlog.toml"}
    return {"STUDY_PROBE_PATHS": "\n".join(paths), "STUDY_PROBE_NAMES": "\n".join(sorted(names))}


def probe_report(stream: Path) -> dict:
    for line in stream.read_text(errors="replace").splitlines():
        try:
            ev = json.loads(line)
        except ValueError:
            continue
        if ev.get("type") == "probe":
            return ev
    return {}


def cmd_isolation_check(args) -> None:
    """Amendment 7's check, no model: a probe relay must reach none of the study's files, the same
    probe with the old mount must reach them (the refusal), and a reference relay must still grade."""
    if "ADOPTION_RUNS_ROOT" not in os.environ or "ADOPTION_KEEP_ROOT" not in os.environ:
        raise SystemExit("refused: set ADOPTION_RUNS_ROOT and ADOPTION_KEEP_ROOT to scratch")
    rid, repo, arm = args.run_id, args.repo, args.arm
    results = []

    def check(name: str, ok: bool, detail: str = "") -> None:
        results.append(ok)
        print(f"{'PASS' if ok else 'FAIL'}  {name}" + (f": {detail}" if detail else ""), flush=True)

    probe = {"scenario": "relay", "repo": repo, "arm": arm, "rep": 1}
    r = run_cell(f"{rid}-probe", probe, "fixture-probe", "fixture")
    cell = cell_dir(f"{rid}-probe", probe)
    check("the probe relay ran two sessions and graded each",
          r["sessions_used"] == 2 and all((cell / f"s{n:02d}" / "grade.json").exists() for n in (1, 2)),
          f"{r['sessions_used']} sessions, stopped {r['stopped']}")
    rep2 = probe_report(cell / "s02" / "stream.jsonl")
    for a in rep2.get("attempts", []):
        print(f"      s02 tried {a['path']}: " + (a.get("got") or a.get("error")))
    print(f"      s02 sees /work as {rep2.get('work_entries')}")
    check("session 2 reaches no earlier grade, record, hidden/, reference/ or study path",
          bool(rep2) and not rep2["reachable"], f"reachable: {rep2.get('reachable')}")
    check("session 2 finds no file named as a hidden test, reference patch, backlog or session record",
          bool(rep2) and not rep2["forbidden_names_found"], f"found: {rep2.get('forbidden_names_found')}")
    check("session 2 finds no file holding a grade", bool(rep2) and not rep2["grade_like_files"],
          f"found: {rep2.get('grade_like_files')}")
    # The refusal: the same probe with the mount the harness had before (the cell at /work) and the
    # grader's and a fixture's mounts must report what it reaches.
    d = repo_def(repo)
    sb = Sandbox(cell, repo, arm=arm, agent="fixture-probe", work=cell, binds=[
        ("--ro-bind", HERE / "fixture_probe.py", "/opt/study/fixture_probe.py"),
        ("--ro-bind", d / "hidden", "/opt/hidden"), ("--ro-bind", d, "/opt/study/repo-def")])
    p = sb.run(["/usr/bin/python3", "/opt/study/fixture_probe.py"], "/work", timeout=600, extra_env=probe_env(repo, cell))
    down = next((json.loads(l) for l in p.stdout.splitlines() if l.startswith('{"type": "probe"')), {})
    print(f"      walls down, reachable: {down.get('reachable')}")
    print(f"      walls down, {len(down.get('forbidden_names_found', []))} names found, e.g. "
          f"{down.get('forbidden_names_found', [])[:4]}; grades in {down.get('grade_like_files', [])[:3]}")
    check("the probe sees a leak when there is one",
          {"/work/s01/grade.json", "/opt/hidden", "/opt/study/repo-def"} <= set(down.get("reachable", []))
          and any(n.endswith(".patch") for n in down.get("forbidden_names_found", []))
          and any(n.startswith("/opt/hidden/") for n in down.get("forbidden_names_found", []))
          and "/work/s01/grade.json" in down.get("grade_like_files", []))
    ref = {"scenario": "relay", "repo": repo, "arm": arm, "rep": 1}
    r = run_cell(f"{rid}-reference", ref, "fixture-reference", "fixture")
    kept = KEEP / f"{rid}-reference" / cell_dir(f"{rid}-reference", ref).relative_to(RUNS / f"{rid}-reference")
    graded = [json.loads((kept / f"s{s['n']:02d}" / "grade.json").read_text()) for s in r["sessions"]]
    n_items = len(load_backlog(repo)["item"])
    check("a reference relay still grades: every item passes, no false completion, every grade kept",
          r["stopped"] == "all_pass" and len(r["items_passing"]) == n_items and not r["false_completions_at_end"]
          and all(len(g.get("items", {})) == n_items for g in graded),
          f"{len(r['items_passing'])} of {n_items} in {r['sessions_used']} sessions, stopped {r['stopped']}, "
          f"false completions {r['false_completions_at_end']}")
    print(f"{sum(results)} of {len(results)} checks passed")
    if not all(results):
        raise SystemExit(1)


# ------------------------------------------------------------------ amendment 8's checks


def _token_cell(arm: str, item: str, tokens: int, success: bool = True, says_done: bool | None = True) -> dict:
    return {"cell": {"scenario": "directed", "repo": "tally", "arm": arm, "item": item, "rep": 1},
            "success": success, "tracker_says_done": says_done,
            "sessions": [{"metrics": {"input_tokens": tokens}}]}


def cmd_a8_check(args) -> None:
    """Amendment 8's checks, no model. Each control is paired with a refusal. A check that needs
    something `next` does not have yet is SKIP with the reason: neither a pass nor a failure."""
    if "ADOPTION_RUNS_ROOT" not in os.environ or "ADOPTION_KEEP_ROOT" not in os.environ:
        raise SystemExit("refused: set ADOPTION_RUNS_ROOT and ADOPTION_KEEP_ROOT to scratch")
    rid = args.run_id
    results: list[str] = []

    def check(name: str, ok: bool, detail: str = "") -> None:
        results.append("PASS" if ok else "FAIL")
        print(f"{results[-1]}  {name}" + (f": {detail}" if detail else ""), flush=True)

    def skip(name: str, why: str) -> None:
        results.append("SKIP")
        print(f"SKIP  {name}: {why}", flush=True)

    def env(**kv):
        import contextlib

        @contextlib.contextmanager
        def cm():
            old = {k: os.environ.get(k) for k in kv}
            os.environ.update(kv)
            try:
                yield
            finally:
                for k, v in old.items():
                    os.environ.pop(k, None) if v is None else os.environ.__setitem__(k, v)
        return cm()

    # ---- the token step
    new, old = "openwarrant@next", "openwarrant@a7"
    items = [f"T{n}" for n in range(1, 13)]

    def pair(new_tokens: list[int], **kw) -> list[dict]:
        rs = [_token_cell(old, i, 1000) for i in items[:len(new_tokens)]]
        return rs + [_token_cell(new, i, t) for i, t in zip(items, new_tokens)] + kw.get("extra", [])

    w = win_rule(pair([700, 750, 800, 850, 800, 760, 820, 790, 810, 780, 770, 830]), new, old)
    check("tokens: every cell cheaper, the whole interval below 1.0, the new version wins",
          w["result"] == f"{new} wins on tokens" and w["paired_tokens"]["ci95"][1] < 1.0, ratio_text(w["paired_tokens"]))
    w = win_rule(pair([1300, 1250, 1200, 1350, 1280, 1220, 1310, 1270, 1240, 1290, 1260, 1330]), new, old)
    check("tokens: every cell dearer, the whole interval above 1.0, the new version loses",
          w["result"] == f"{old} wins on tokens" and w["paired_tokens"]["ci95"][0] > 1.0, ratio_text(w["paired_tokens"]))
    w = win_rule(pair([500, 500, 500, 2000, 2000, 2000]), new, old)
    check("tokens: an interval that holds 1.0 is a tie, where the median rule called a winner",
          w["result"] == "tie: no win" and w["result_amendment5"] == f"{old} wins on tokens",
          f"{ratio_text(w['paired_tokens'])}; amendment 5's step: {w['result_amendment5']}")
    rs = pair([500] * 6)
    rs[-1] = _token_cell(new, items[5], 500, success=False, says_done=True)
    w = win_rule(rs, new, old)
    check("false completions still come first: cheaper everywhere, one false completion, it loses",
          w["result"] == f"{old} wins on false completions" and w["result_amendment5"] == w["result"], w["result"])
    rs = pair([500] * 6)
    for n in (4, 5):
        rs[6 + n] = _token_cell(new, items[n], 500, success=False, says_done=False)
    w = win_rule(rs, new, old)
    check("passes still come before tokens: cheaper everywhere, two fewer passes, it loses",
          w["result"] == f"{old} wins on passes", w["result"])
    rows = against_plain(pair([900] * 6, extra=[_token_cell("plain", i, 1000) for i in items[:6]]))
    by = {r["arm"]: r for r in rows}
    check("against plain: each OpenWarrant arm has a row, and the release criterion reads next's",
          set(by) == {new, old} and by[new]["release_criterion"] == "met" and "release_criterion" not in by[old]
          and against_plain(pair([1100] * 6, extra=[_token_cell("plain", i, 1000) for i in items[:6]]))[1]
          ["release_criterion"] == "not met", f"next over plain {by[new]['tokens']['ratio']}")
    recorded = Path("/mnt/4tb/artifacts/OpenWarrant/keep/adoption")
    real = [{**json.loads(f.read_text()), "_run": r} for r in ("oc-v5-2", "oc-v5-3")
            for f in sorted((recorded / r).rglob("result.json"))]
    if real:
        pr = paired_ratio(real, new, old)
        # Amendment 6's diagnosis: +0.8%, 95% interval -21.7% to +28.5%, from 10,000 resamples of another seed.
        check("the recorded runs: oc-v5-2 and oc-v5-3 give the diagnosis's +0.8%, and its interval within resampling noise",
              pr["pairs"] == 12 and abs(pr["ratio"] - 1.008) < 0.0006 and abs(pr["ci95"][0] - 0.783) < 0.006
              and abs(pr["ci95"][1] - 1.285) < 0.006, ratio_text(pr))
    else:
        skip("the recorded runs reproduce the diagnosis", f"no oc-v5-2 and oc-v5-3 under {recorded}")

    # ---- the hard budget
    purse = Purse(2.0)
    took = [purse.take(1.0), purse.take(1.0), purse.take(1.0)]
    purse.settle(1.0, 0.3)
    after_one = purse.take(1.0)      # 0.3 spent and 1.0 held: another 1.0 does not fit
    purse.settle(1.0, 0.2)
    after_two = purse.take(1.0)      # 0.5 spent, nothing held: it fits
    check("a hard budget holds each cell's cap before it starts, and refuses the cell that doesn't fit",
          took == [True, True, False] and after_one is False and after_two is True, f"{took}, {after_one}, {after_two}")

    # ---- done_test in the seed
    tally, knight = load_backlog("tally"), load_backlog("knightlm")
    cmds = {b["repo"]: arms.done_test_command(b) for b in (tally, knight)}
    check("each repository's done_test command is a line of the facts every arm carries",
          cmds == {"tally": "python3 -m pytest -q", "knightlm": "cargo test --workspace --locked -j 2"}, str(cmds))
    planted = {"repo": "tally", "project": {"facts": tally["project"]["facts"].replace("python3 -m pytest -q", "make test")}}
    try:
        arms.done_test_command(planted)
        check("a command the facts don't hold is refused", False, "accepted")
    except RuntimeError as e:
        check("a command the facts don't hold is refused", True, str(e)[:90])
    scratch = RUNS / rid
    shutil.rmtree(scratch, ignore_errors=True)
    scratch.mkdir(parents=True)
    (scratch / "cfg").mkdir()
    (scratch / "cfg" / "openwarrant.toml").write_text('schema = "x"\n')
    arms.tickets_config(scratch / "cfg", "session_start = true")
    arms.tickets_config(scratch / "cfg", 'done_test = "python3 -m pytest -q"')
    cfg = tomllib.loads((scratch / "cfg" / "openwarrant.toml").read_text())
    check("two [tickets] keys land in one table", cfg["tickets"] == {"session_start": True,
                                                                      "done_test": "python3 -m pytest -q"}, str(cfg["tickets"]))
    done = tally["order"][:tally["order"].index("T8")]
    layout = AGENT_SEED["opencode"]
    a7 = "openwarrant@a7"
    check("only @next arms carry the key, and ADOPTION_OW_DONE_TEST=0 turns it off",
          ow_done_test(new) and ow_done_test("openwarrant-brief@next") and ow_done_test("openwarrant-mcp@next")
          and not ow_done_test(a7) and not ow_done_test("openwarrant-mcp@a7") and not ow_done_test("openwarrant")
          and not ow_done_test("plain") and seed_extras(a7) == {})
    with env(ADOPTION_OW_DONE_TEST="0"):
        check("the switch off: no arm carries the key", not ow_done_test(new) and seed_extras(new) == {})
    seed_repo("tally", a7, scratch / "a7", done, layout)
    check("alpha.7's seed has no done_test, and its `war` still reads its tickets",
          "done_test" not in (scratch / "a7" / "openwarrant.toml").read_text()
          and all(v is not None for v in arms.tracker(a7, scratch / "a7", tally, tools(a7)).values()))
    check("the probe passes a key alpha.7 has (session_start) and refuses the one it lacks (done_test)",
          war_reads_tickets_key(arm_war(a7), "session_start = true") and not war_knows_done_test(arm_war(a7)))
    c = {"scenario": "directed", "repo": "tally", "arm": a7, "item": "T8", "rep": 1}
    r = run_cell(f"{rid}-a7-broken", c, "fixture-claim-broken", "fixture")
    check("the control: with no done_test, alpha.7 records an item done over a failing visible suite",
          r["tracker_says_done"] is True and r["success"] is False and r["done_test"] is None,
          f"tracker {r['tracker_says_done']}, hidden tests pass {r['success']}")
    nxt = arm_war(new)
    if not nxt or not Path(nxt).exists():
        for name in ("next seeds done_test", "`war done` refuses on a failing done_test", "the brief file is in the seed"):
            skip(name, "no `next` build (set ADOPTION_WAR_NEXT and ADOPTION_OW_PLUGIN_REF_NEXT)")
        knows = None
    else:
        knows = war_knows_done_test(nxt)
        build = arm_build(new)["war_version"]
    if knows is False:
        problems = feature_problems([new], "opencode")
        check("a run whose `next` lacks the key stops before its first cell", len(problems) == 1 and "done_test" in problems[0],
              (problems or ["no problem reported"])[0][:110])
        with env(ADOPTION_OW_DONE_TEST="0"):
            seed_repo("tally", new, scratch / "next-off", done, layout)
            check("the switch off: `next` seeds as before, with no key, and nothing stops the run",
                  feature_problems([new], "opencode") == []
                  and "done_test" not in (scratch / "next-off" / "openwarrant.toml").read_text())
        for name in ("next's seed carries done_test and its `war` reads it",
                     "with done_test, the reference solution is recorded done",
                     "with done_test, `war done` refuses over a failing visible suite"):
            skip(name, f"this `next` ({build}) does not read `[tickets] done_test`; owed when the product has the key")
    elif knows:
        seed_repo("tally", new, scratch / "next", done, layout)
        text = (scratch / "next" / "openwarrant.toml").read_text()
        cfg = tomllib.loads(text)
        check("next's seed carries done_test and its `war` reads it",
              cfg.get("tickets", {}).get("done_test") == cmds["tally"] and feature_problems([new], "opencode") == []
              and all(v is not None for v in arms.tracker(new, scratch / "next", tally, tools(new)).values()), build)
        # `war init` on this build records the test it finds. The seed must hold one key, the facts'
        # command, and closing the earlier items must have run nothing.
        materialize("tally", scratch / "init")
        subprocess.run([nxt, "init", "--root", "."], cwd=scratch / "init", env=arms.seed_env({}), check=True,
                       capture_output=True, timeout=120)
        found = arms.strip_done_test(scratch / "init")
        tickets_text = "".join(f.read_text() for f in sorted((scratch / "next" / "docs" / "tickets").rglob("*")) if f.is_file())
        check("`war init` finds a test of its own; the seed holds one key, the facts' command, and seeding ran no test",
              bool(found) and text.count("done_test") == 1 and "observed" not in tickets_text
              and not list((scratch / "next").rglob("test_zz*")),
              f"init found {found!r}; the seed holds {cfg['tickets']['done_test']!r}")
        with env(ADOPTION_OW_DONE_TEST="0"):
            seed_repo("tally", new, scratch / "next-off", done, layout)
            check("the switch off: `next`'s seed has no done_test, though `war init` wrote one",
                  "done_test" not in (scratch / "next-off" / "openwarrant.toml").read_text()
                  and all(v is not None for v in arms.tracker(new, scratch / "next-off", tally, tools(new)).values()))
        c = {"scenario": "directed", "repo": "tally", "arm": new, "item": "T8", "rep": 1}
        r = run_cell(f"{rid}-next-reference", c, "fixture-reference", "fixture")
        check("with done_test, the reference solution is recorded done",
              r["success"] is True and r["tracker_says_done"] is True and r["done_test"] == cmds["tally"],
              f"tracker {r['tracker_says_done']}, hidden tests pass {r['success']}")
        r = run_cell(f"{rid}-next-broken", c, "fixture-claim-broken", "fixture")
        check("with done_test, `war done` refuses over a failing visible suite",
              r["tracker_says_done"] is False and r["success"] is False,
              f"tracker {r['tracker_says_done']}, hidden tests pass {r['success']}")

    # ---- the brief arm
    brief = "openwarrant-brief@next"
    cb, cn = opencode_config(brief, 60), opencode_config(new, 60)
    check("the brief arm's opencode config is its twin's plus the instruction file, and nothing else",
          cb.get("instructions") == [arms.BRIEF_FILE] and "instructions" not in cn
          and {k: v for k, v in cb.items() if k != "instructions"} == cn and "mcp" not in cb, str(cb.get("instructions")))
    check("the brief arm is the openwarrant arm on next's build",
          arms.tool_of(brief) == "openwarrant" and arms.version_arm(brief) == new and arm_war(brief) == arm_war(new)
          and brief in arms.ALL_ARMS)
    with env(ADOPTION_OW_DONE_TEST="0"):
        check("the brief arm is refused under Claude Code", any("brief" in p for p in feature_problems([brief], "claude"))
              and feature_problems([brief], "opencode") == [])
    if knows is not None:
        with env(ADOPTION_OW_DONE_TEST="1" if knows else "0"):
            try:
                seed_repo("tally", brief, scratch / "brief", done, layout)
                text = (scratch / "brief" / arms.BRIEF_FILE).read_text()
                check("the brief file is in the seed, and not empty", bool(text.strip()), f"{len(text)} bytes")
            except SystemExit as e:
                check("a brief cell stops when `war` wrote no brief file", "wrote no" in str(e), str(e)[:100])
                skip("the brief file is in the seed", f"this `next` ({build}) writes no {arms.BRIEF_FILE}; owed when the product keeps it")
    skip("opencode loads the brief file at startup", "needs one model session to see (as amendment 6 proved `war mcp`); "
         "owed before the first brief cell, not a study cell")

    # ---- the plans
    for path in sorted((HERE / "plans").glob("*.json")):
        doc = json.loads(path.read_text())
        cells = doc["cells"]
        bad = [c for c in cells if c["arm"] not in arms.ALL_ARMS or c["scenario"] != "directed"
               or c["item"] not in load_backlog(c["repo"])["order"]]
        per_arm: dict[str, set] = {}
        for c in cells:
            per_arm.setdefault(c["arm"], set()).add((c["repo"], c["item"], c["rep"]))
        even = len({frozenset(v) for v in per_arm.values()}) == 1
        ex = doc["execute"]
        ok = not bad and even
        detail = f"{len(cells)} cells, {len(per_arm)} arms, {len(next(iter(per_arm.values())))} items each"
        if ex["agent"] == "claude":
            worst = len(cells) * ex["session_budget_usd"]
            ok = ok and ex["budget_usd"] <= 30 and worst <= 30
            detail += f"; every session at its cap: USD {worst:.2f}, budget {ex['budget_usd']}"
        check(f"plan {path.name}: directed cells only, the same cells for every arm"
              + (", and no more than USD 30 even at every cap" if ex["agent"] == "claude" else ""), ok, detail)
    shutil.rmtree(scratch, ignore_errors=True)
    n = {k: results.count(k) for k in ("PASS", "SKIP", "FAIL")}
    print(f"{n['PASS']} passed, {n['SKIP']} skipped (owed, see each), {n['FAIL']} failed")
    if n["FAIL"]:
        raise SystemExit(1)


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
    if any(c["scenario"] == "spec" for c in cells):
        cells = resolve_specs(cells, args.specs_run, args.skip_unwritten)
    arm_list = sorted({c["arm"] for c in cells})
    problems = check_tools(args.agent, arm_list)
    if args.agent == "opencode":
        if not os.environ.get(OPENCODE_KEY_ENV):
            problems.append(f"opencode needs {OPENCODE_KEY_ENV} in the environment (run under `secrets run --`)")
        provider, _, mid = args.model.partition("/")
        if mid not in OPENCODE_PROVIDERS.get(provider, {}).get("models", {}):
            problems.append(f"--model {args.model!r}: opencode runs take lithosai/<model>, one of "
                            + ", ".join(OPENCODE_PROVIDERS["lithosai"]["models"]))
    if any(c["arm"] == "jira" for c in cells) and jira_arm.missing():
        problems.append("jira cells need " + ", ".join(jira_arm.missing()) + " in the environment")
    if args.agent == "claude" and check_auth():
        problems.append(check_auth())
    problems += [p for p in (registry_problem(r) for r in sorted({c["repo"] for c in cells})) if p]
    problems += feature_problems(arm_list, args.agent, sorted({c["repo"] for c in cells}))
    hard = args.session_budget_usd is not None
    if hard:
        global SESSION_BUDGET_USD
        if not args.budget_usd or args.session_budget_usd <= 0:
            problems.append("--session-budget-usd makes --budget-usd a hard cap: give both, above zero")
        SESSION_BUDGET_USD = args.session_budget_usd
    if problems:
        raise SystemExit("not ready:\n  " + "\n  ".join(problems))
    if args.agent == "claude":  # only Claude Code loads the plugin
        for arm in arm_list:
            if arms.version_arm(arm) in OW_VERSIONS and not plugin_dir(arm).exists():
                unpack_plugin(plugin_dir(arm), arm_plugin_ref(arm))
    run_root = RUNS / args.run_id
    run_root.mkdir(parents=True, exist_ok=True)
    run_doc = json.dumps({
        "run_id": args.run_id, "agent": args.agent, "model": args.model, "pins": PINS,
        "war_build": os.environ.get("ADOPTION_WAR"), "plugin_ref": os.environ.get("ADOPTION_OW_PLUGIN_REF"),
        "arm_builds": {arm: arm_build(arm) for arm in arm_list}, "turn_cap": args.turns,
        "plan": str(Path(args.plan).resolve()), "harness_commit": harness_commit(),
        "ow_session_start": os.environ.get("ADOPTION_OW_SESSION_START") == "1",
        "opencode_ow_mcp": args.agent == "opencode" and os.environ.get("ADOPTION_OC_OW_MCP") == "1",
        # Amendment 8: the switch, the arms whose seed carries `[tickets] done_test`, and each
        # repository's command; and the arms whose opencode config loads the brief file.
        "ow_done_test": {"switch": os.environ.get("ADOPTION_OW_DONE_TEST", "1") != "0",
                         "arms": [a for a in arm_list if ow_done_test(a)],
                         "commands": {r: arms.done_test_command(load_backlog(r))
                                      for r in sorted({c["repo"] for c in cells})}
                                     if any(ow_done_test(a) for a in arm_list) else {}},
        "ow_brief": {"file": arms.BRIEF_FILE, "arms": [a for a in arm_list if arms.with_brief(a)]},
        "session_budget_usd": SESSION_BUDGET_USD, "budget_hard": hard,
        "auth": auth_mode(), "budget_usd": args.budget_usd, "specs_run": args.specs_run,
        "started": time.strftime("%Y-%m-%dT%H:%M:%S%z"), "cells": len(cells)}, indent=1)
    (run_root / "run.json").write_text(run_doc)
    (KEEP / args.run_id).mkdir(parents=True, exist_ok=True)  # the keep store outlives a run root in /dev/shm
    if not (KEEP / args.run_id / "run.json").exists():
        (KEEP / args.run_id / "run.json").write_text(run_doc)
        (KEEP / args.run_id / "plan.json").write_text(Path(args.plan).read_text())
    spent = 0.0
    lock = threading.Lock()
    stop = threading.Event()
    purse = Purse(args.budget_usd) if hard else None

    def go(c):
        nonlocal spent
        if stop.is_set():
            return None
        hold = 0.0
        if purse and not (cell_dir(args.run_id, c) / "result.json").exists():
            # The cap is hard: a cell starts only if every session it may run, each at its own cap,
            # still fits under the budget beside what is spent and what running cells hold.
            hold = SESSION_BUDGET_USD * sessions_in(c, load_backlog(c["repo"]))
            if not purse.take(hold):
                stop.set()
                print(f"budget: {c['scenario']} {c['repo']} {c['arm']} {c.get('item', '-')} not started "
                      f"({purse.spent:.2f} spent, {purse.held:.2f} held, {hold:.2f} needed, "
                      f"{args.budget_usd} the cap); no new cells start", flush=True)
                return None
        r = None
        try:
            r = run_cell(args.run_id, c, args.agent, args.model, args.turns)
        finally:
            if purse:
                purse.settle(hold, (r or {}).get("cost_usd") or 0)
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


class Purse:
    """A hard budget (amendment 8, `execute --session-budget-usd`): money is held for a cell before it
    starts, at the most its sessions can cost, and a cell that doesn't fit never starts."""

    def __init__(self, total: float):
        self.total, self.spent, self.held = total, 0.0, 0.0
        self._lock = threading.Lock()

    def take(self, hold: float) -> bool:
        with self._lock:
            if self.spent + self.held + hold > self.total + 1e-9:
                return False
            self.held += hold
            return True

    def settle(self, hold: float, cost: float) -> None:
        with self._lock:
            self.held -= hold
            self.spent += cost


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


def median(xs: list) -> float | None:
    xs = sorted(xs)
    if not xs:
        return None
    n = len(xs)
    return xs[n // 2] if n % 2 else (xs[n // 2 - 1] + xs[n // 2]) / 2


def cell_tokens(r: dict) -> int:
    """Every token a cell's sessions processed: input, cache reads and writes, and output."""
    return sum((m.get("input_tokens") or 0) + (m.get("cache_read_tokens") or 0)
               + (m.get("cache_creation_tokens") or 0) + (m.get("output_tokens") or 0)
               for m in ((s.get("metrics") or {}) for s in r["sessions"]))


# Amendment 5: which versions are compared, newer first.
VERSION_PAIRS = (("openwarrant@a7", "openwarrant@a6"), ("openwarrant@next", "openwarrant@a7"),
                 ("openwarrant-mcp@next", "openwarrant-mcp@a7"))
# Amendment 6: compared descriptively only, no win rule. Amendment 8 adds the brief arm beside its twin.
TWINS = (("openwarrant-mcp@a7", "openwarrant@a7"), ("openwarrant-mcp@next", "openwarrant@next"),
         ("openwarrant-brief@next", "openwarrant@next"))
RELEASE_ARM = "openwarrant@next"   # amendment 8: the owner's release criterion is this arm's ratio against plain


def directed_pairs(results: list[dict], new: str, old: str) -> tuple[list[dict], list[dict]]:
    """The directed cells both arms ran, in the same order. Pooled results carry `_run`, so cells of
    different runs stay apart: a pair is one item, in one replicate, of one run."""
    cells: dict[str, dict] = {}
    for r in results:
        c = r["cell"]
        if c["scenario"] == "directed" and c["arm"] in (new, old):
            cells.setdefault(c["arm"], {})[(r.get("_run", ""), c["repo"], c["item"], c["rep"])] = r
    common = sorted(set(cells.get(new, {})) & set(cells.get(old, {})))
    return [cells[new][k] for k in common], [cells[old][k] for k in common]


def paired_ratio(results: list[dict], new: str, old: str, n: int = 100000, seed: int = 20261022) -> dict | None:
    """Amendment 8's token statistic: the geometric mean, over the directed cells both arms ran, of
    each cell's tokens under `new` over the same cell's under `old`, with a percentile bootstrap 95%
    interval (pairs resampled with replacement, `n` times, a fixed seed). Below 1.0, `new` used fewer.
    A pair where either cell reports no tokens has no ratio and is left out, and counted."""
    import math
    import random
    a, b = directed_pairs(results, new, old)
    toks = [(cell_tokens(x), cell_tokens(y)) for x, y in zip(a, b)]
    logs = [math.log(x / y) for x, y in toks if x and y]
    if not logs:
        return None
    k = len(logs)
    out = {"new": new, "old": old, "pairs": k, "left_out": len(toks) - k,
           "ratio": round(math.exp(sum(logs) / k), 4), "ci95": None, "verdict": "tie"}
    if k >= 2:
        rng = random.Random(seed)
        means = sorted(sum(rng.choices(logs, k=k)) / k for _ in range(n))
        lo, hi = math.exp(means[int(0.025 * n)]), math.exp(means[int(0.975 * n) - 1])
        out["ci95"] = [round(lo, 4), round(hi, 4)]
        # The whole interval below 1.0: `new` wins on tokens. The whole interval above: it loses.
        out["verdict"] = "new" if hi < 1.0 else ("old" if lo > 1.0 else "tie")
    return out


def ratio_text(pr: dict | None) -> str:
    if not pr:
        return "not available"
    pct = lambda x: f"{100 * (x - 1):+.1f}%"
    ci = f"95% interval {pct(pr['ci95'][0])} to {pct(pr['ci95'][1])}" if pr["ci95"] else "one pair, no interval"
    left = f", {pr['left_out']} left out for no tokens" if pr["left_out"] else ""
    return f"{pr['ratio']:.3f} ({pct(pr['ratio'])}, {ci}; {pr['pairs']} pairs{left})"


def win_rule(results: list[dict], new: str, old: str, tie: int = 1) -> dict:
    """The rule for a pair of versions, on the directed cells both arms ran. False completions first
    (more loses), then passes (a difference of `tie` cells or less is a tie: one per replicate,
    amendment 6), then tokens. A tie overall is no win.

    The token step is amendment 8's: the paired ratio (`paired_ratio`), a win only when its whole 95%
    interval is on one side of 1.0. `result` follows it. Amendment 5's token step (the median of
    totals, 10% fewer wins) decided every verdict recorded before amendment 8; those stand, and its
    figure and verdict are still worked out and printed beside the new one (`result_amendment5`)."""
    a, b = directed_pairs(results, new, old)
    out = {"new": new, "old": old, "cells": len(a), "paired_tokens": paired_ratio(results, new, old)}
    steps = [f"cells both ran: {len(a)}"]

    def done(result: str, result_a5: str | None = None) -> dict:
        return {**out, "steps": steps, "result": result, "result_amendment5": result_a5 or result}

    fa = sum(1 for r in a if r.get("tracker_says_done") and not r.get("success"))
    fb = sum(1 for r in b if r.get("tracker_says_done") and not r.get("success"))
    steps.append(f"false completions: {new} {fa}, {old} {fb}")
    if fa != fb:
        return done(f"{new if fa < fb else old} wins on false completions")
    pa, pb = sum(1 for r in a if r.get("success")), sum(1 for r in b if r.get("success"))
    steps.append(f"passes: {new} {pa}, {old} {pb} (difference {pa - pb}; "
                 + ("one cell or less is a tie)" if tie == 1 else f"{tie} cells or less is a tie)"))
    if abs(pa - pb) > tie:
        return done(f"{new if pa > pb else old} wins on passes")
    # Amendment 5's token step, as it was.
    ta, tb = median([cell_tokens(r) for r in a]), median([cell_tokens(r) for r in b])
    a5 = "tie: no win"
    if not ta or not tb:
        steps.append("median tokens per cell (amendment 5's step): not available")
    else:
        steps.append(f"median tokens per cell (amendment 5's step): {new} {ta:,.0f}, {old} {tb:,.0f} "
                     f"({new} is {100 * (ta - tb) / tb:+.1f}% against {old}; 10% fewer wins)")
        a5 = f"{new} wins on tokens" if ta <= 0.9 * tb else (f"{old} wins on tokens" if tb <= 0.9 * ta else a5)
    # Amendment 8's token step.
    pr = out["paired_tokens"]
    steps.append(f"paired token ratio, {new} over {old} (amendment 8's step): {ratio_text(pr)}; "
                 "a win needs the whole interval on one side of 1.0")
    a8 = {"new": f"{new} wins on tokens", "old": f"{old} wins on tokens"}.get((pr or {}).get("verdict"), "tie: no win")
    return done(a8, a5)


def rule_lines(w: dict) -> list[str]:
    """A pair's steps and both verdicts, for a report."""
    return [f"- {step}" for step in w["steps"]] + [
        "", f"**Result (amendment 8's token step): {w['result']}**", "",
        f"By amendment 5's token step, which decided every verdict recorded before amendment 8: "
        f"{w['result_amendment5']}.", ""]


def against_plain(results: list[dict]) -> list[dict]:
    """Amendment 8: each OpenWarrant arm against `plain` on the directed cells both ran, by the same
    paired token ratio. Descriptive: no win rule. The owner's release criterion reads one row of it."""
    arm_names = sorted({r["cell"]["arm"] for r in results if r["cell"]["scenario"] == "directed"})
    rows = []
    for arm in arm_names:
        if arms.tool_of(arm) != "openwarrant" or "plain" not in arm_names:
            continue
        a, b = directed_pairs(results, arm, "plain")
        pr = paired_ratio(results, arm, "plain")
        if not a or not pr:
            continue
        row = {"arm": arm, "cells": len(a), "tokens": pr,
               "passes": [sum(1 for r in a if r.get("success")), sum(1 for r in b if r.get("success"))],
               "false_completions": [sum(1 for r in a if r.get("tracker_says_done") and not r.get("success")),
                                     sum(1 for r in b if r.get("tracker_says_done") and not r.get("success"))]}
        if arm == RELEASE_ARM:
            row["release_criterion"] = "met" if pr["ratio"] <= 1.0 else "not met"
        rows.append(row)
    return rows


def plain_lines(rows: list[dict], over: str = "") -> list[str]:
    if not rows:
        return []
    lines = [f"## Each OpenWarrant arm against plain{over} (amendment 8: paired token ratio, descriptive, no win rule)",
             "", "| arm | cells both ran | tokens, arm over plain | passes, arm and plain | false completions, arm and plain |",
             "|---|---|---|---|---|"]
    lines += [f"| {r['arm']} | {r['cells']} | {ratio_text(r['tokens'])} | {r['passes'][0]} and {r['passes'][1]} | "
              f"{r['false_completions'][0]} and {r['false_completions'][1]} |" for r in rows]
    lines.append("")
    for r in rows:
        if "release_criterion" in r:
            lines += [f"The owner's release criterion, `{RELEASE_ARM}`'s ratio against `plain` at or below 1.0: "
                      f"{r['tokens']['ratio']:.3f}, **{r['release_criterion']}** on these cells.", ""]
    return lines


def describe(results: list[dict], arm: str) -> dict:
    """An arm's directed cells in the terms of the win rule, for a side-by-side comparison."""
    rs = [r for r in results if r["cell"]["scenario"] == "directed" and r["cell"]["arm"] == arm]
    ms = [s.get("metrics") or {} for r in rs for s in r["sessions"]]
    return {"arm": arm, "cells": len(rs), "passes": sum(1 for r in rs if r.get("success")),
            "false_completions": sum(1 for r in rs if r.get("tracker_says_done") and not r.get("success")),
            "median_tokens_per_cell": median([cell_tokens(r) for r in rs]),
            "turns_per_session": round(sum(m.get("num_turns") or 0 for m in ms) / len(ms), 1) if ms else None,
            "median_turns_before_first_edit": median([m["turns_before_first_code_edit"] for m in ms
                                                      if m.get("turns_before_first_code_edit") is not None])}


def excluded(results: list[dict], specs: list[str]) -> tuple[list[dict], int]:
    """Amendment 5's rule for a harness error: a cell rerun or excluded for every arm. `RUN/REPO/ITEM`
    leaves that item of that run out, in every arm; the report says so. Nothing in the keep store moves."""
    drop = {tuple(s.split("/")) for s in specs or []}
    for d in drop:
        if len(d) != 3:
            raise SystemExit(f"--exclude {'/'.join(d)!r}: give RUN/REPO/ITEM")
    kept = [r for r in results if (r.get("_run", ""), r["cell"]["repo"], r["cell"].get("item", "-")) not in drop]
    return kept, len(results) - len(kept)


def cmd_pool(args) -> None:
    """Amendment 6: the win rule over the pooled directed cells of several runs, per pair, with a tie
    of one cell per replicate the pair ran in; and each MCP arm beside its twin, descriptively."""
    results = []
    for run_id in args.run_ids:
        for p in sorted((KEEP / run_id).rglob("result.json")):
            results.append({**json.loads(p.read_text()), "_run": run_id})
    results, dropped = excluded(results, args.exclude)
    lines = [f"# Pooled: {', '.join(args.run_ids)}", ""]
    if args.exclude:
        lines += [f"Excluded for every arm (harness errors, amendment 5's rule): {', '.join(args.exclude)}; "
                  f"{dropped} cells left out.", ""]
    out = {"runs": args.run_ids, "pairs": [], "twins": [], "excluded": args.exclude, "cells_excluded": dropped}
    for new, old in VERSION_PAIRS:
        runs = [rid for rid in args.run_ids
                if {new, old} <= {r["cell"]["arm"] for r in results if r["_run"] == rid}]
        if not runs:
            continue
        w = win_rule([r for r in results if r["_run"] in runs], new, old, tie=len(runs))
        w["runs"] = runs
        out["pairs"].append(w)
        lines += [f"## {new} against {old}, pooled over {', '.join(runs)} (tie: {len(runs)} cells or less)", ""]
        lines += rule_lines(w)
    plain_runs = [rid for rid in args.run_ids
                  if "plain" in {r["cell"]["arm"] for r in results if r["_run"] == rid}]
    out["against_plain"] = against_plain([r for r in results if r["_run"] in plain_runs])
    lines += plain_lines(out["against_plain"], ", pooled over every run holding both")
    for mcp_arm, twin in TWINS:
        runs = [rid for rid in args.run_ids
                if {mcp_arm, twin} <= {r["cell"]["arm"] for r in results if r["_run"] == rid}]
        if not runs:
            continue
        rows = [describe([r for r in results if r["_run"] in runs], a) for a in (mcp_arm, twin)]
        pr = paired_ratio([r for r in results if r["_run"] in runs], mcp_arm, twin)
        out["twins"].append({"runs": runs, "rows": rows, "paired_tokens": pr})
        keys = list(rows[0])
        lines += [f"## {mcp_arm} beside {twin}, pooled over {', '.join(runs)} (descriptive, no win rule)", "",
                  "| " + " | ".join(keys) + " |", "|" + "---|" * len(keys)]
        lines += ["| " + " | ".join(str(r[k]) for k in keys) + " |" for r in rows] + [""]
        lines += [f"Paired token ratio, {mcp_arm} over {twin}: {ratio_text(pr)}.", ""]
    name = "pool-" + "-".join(args.run_ids) + ("-excl" if args.exclude else "")
    (KEEP / f"{name}.json").write_text(json.dumps(out, indent=1))
    (KEEP / f"{name}.md").write_text("\n".join(lines))
    print("\n".join(lines))


def paired_differences(results: list[dict]) -> list[dict]:
    """Each OpenWarrant arm minus each non-OpenWarrant arm on the primary measures, over cells both
    arms ran. Versioned arms (amendment 5) are compared with each other by win_rule instead."""
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
        for ow_arm in sorted(a for a in cells if arms.tool_of(a) == "openwarrant"):
            ow = cells[ow_arm]
            for arm, vals in sorted(cells.items()):
                if arms.tool_of(arm) == "openwarrant":
                    continue
                common = sorted(set(ow) & set(vals))
                if len(common) < 2:
                    continue
                diffs = [ow[k] - vals[k] for k in common]
                row = {"measure": name, "arm": arm, "cells": len(common),
                       "mean": round(sum(diffs) / len(diffs), 3), "ci95": bootstrap_ci(diffs)}
                if ow_arm != "openwarrant":
                    row["openwarrant_arm"] = ow_arm
                out.append(row)
    return out


def resolve_specs(cells: list[dict], specs_run: str | None, skip_unwritten: bool) -> list[dict]:
    """Each worker cell's spec digest, from the run that wrote it. No cell starts on a spec not stored."""
    out, missing = [], []
    for c in cells:
        if c["author"] != "backlog":
            if not specs_run:
                raise SystemExit("spec cells need --specs-run <the run that wrote the specs>")
            f = KEEP / specs_run / "authors" / c["spec"] / "spec.json"
            if not f.exists():
                missing.append(c["spec"])
                continue
            c = {**c, "spec_run": specs_run, "spec_digest": json.loads(f.read_text())["digest"]}
            spec_study.verify_stored(spec_store(), c["spec_digest"])
        out.append(c)
    if missing and not skip_unwritten:
        raise SystemExit(f"{len(set(missing))} specs are not stored in {specs_run}, for example {missing[0]} "
                         "(--skip-unwritten runs the cells that have one)")
    return out


def network_commands(stream: Path) -> list[str]:
    """Shell commands in a transcript that reach for the network. A sandbox with a model's API can't
    forbid them, so an author's are recorded: the web tools are off, and this is what's left."""
    pat = re.compile(r'"command":\s*"([^"]*\b(?:curl|wget|git clone|git fetch|pip install|npm install)\b[^"]{0,120})')
    return sorted(set(pat.findall(stream.read_text(errors="replace"))))[:20] if stream.exists() else []


def run_author(run_id: str, slot: dict, agent: str, model: str, layout: str) -> dict:
    """One author: the brief, the seed repository and its tool, then what it wrote, stored by digest.
    It sees the sandbox every agent sees, with no backlog.toml in it or in the tree it starts from."""
    cell = RUNS / run_id / "authors" / slot["slot"]
    if (cell / "spec.json").exists():
        return json.loads((cell / "spec.json").read_text())
    if cell.exists():
        shutil.rmtree(cell)
    cell.mkdir(parents=True)
    repo, tool, iid = slot["repo"], slot["tool"], slot["item"]
    fixture = agent.startswith("fixture")
    problem = registry_problem(repo)
    if problem:
        raise SystemExit(f"stopped before an author: {problem}")
    extra = AGENT_SEED.get(layout, {})
    ws = workspace(cell)
    ws.mkdir()
    backlog = seed_scaffold(repo, tool, ws / "repo", backlog_earlier(repo, iid), extra)
    item = next(i for i in backlog["item"] if i["id"] == iid)
    (cell / "seed-head").write_text(subprocess.run(["git", "-C", str(ws / "repo"), "rev-parse", "HEAD"],
                                                   capture_output=True, text=True, timeout=30).stdout.strip())
    brief = spec_study.brief_text(iid)
    (ws / "brief.md").write_text(brief)  # the author's own brief, which its prompt also holds
    instr = extra.get("STUDY_INSTRUCTIONS", "CLAUDE.md")
    prompt = spec_study.author_prompt(tool, iid, brief, instr, layout)
    sock = STUDY / "sock" / os.urandom(4).hex() / "c.sock"
    customer = spec_study.Customer(sock, item, spec_study.customer_rule(iid), cell / "questions.jsonl")
    binds = [("--bind", sock, spec_study.CUSTOMER_PATH),
             ("--ro-bind", HERE / "ask_customer.py", "/opt/study/bin/ask-customer")]
    env = {"OPENWARRANT_ACTOR": "planner", "BEADS_ACTOR": "planner"}  # the same in every author's records
    if fixture:
        binds.append(("--ro-bind", HERE / "fixture_author.py", "/opt/study/fixture_author.py"))
        env.update({"STUDY_TOOL": tool, "STUDY_ITEM": iid, **extra,
                    "STUDY_FIXTURE_QUESTION": item["question"]["asked"],
                    "STUDY_PROBE_SECRET": item["question"]["answer"].strip()})
    sb = Sandbox(cell, repo, arm=tool, agent=agent, layout=layout, binds=binds)
    base = sb.inside(["_spec_capture", tool, "/work/spec-base"], "/work/repo", timeout=300)
    if "error" in base:
        customer.close()
        raise SystemExit(f"stopped: could not read the scaffold of {slot['slot']}: {base['error']}")
    try:
        m = run_session(sb, cell / "s01", "/work/repo", prompt, agent, model, spec_study.AUTHOR_TURNS, env,
                        arm=tool, budget=spec_study.AUTHOR_BUDGET_USD, no_web=True)
    finally:
        customer.close()
    cap = sb.inside(["_spec_capture", tool, "/work/spec-out", (cell / "seed-head").read_text()], "/work/repo",
                    timeout=300)
    if "error" in cap:
        raise SystemExit(f"stopped: could not capture {slot['slot']}: {cap['error']}")
    sb.inside(["_git", "snapshot", "author"], "/work/repo")
    (ws / "spec-out").mkdir(exist_ok=True)
    digest, refused = spec_study.store_spec(spec_store(), ws / "spec-out", tool)
    stats = spec_study.spec_stats(iid, ws / "spec-out", ws / "spec-base")
    spec = {**slot, "digest": digest, "agent": agent, "model": model, "layout": layout,
            "brief_sha256": spec_study.brief_digest(iid), "turn_cap": spec_study.AUTHOR_TURNS,
            "cost_usd": m.get("cost_usd") or 0, "metrics": m, "questions": customer.asked,
            "questions_asked": len(customer.asked), "questions_answered": sum(q["matched"] for q in customer.asked),
            "capture_skipped": cap.get("skipped", []) + refused,
            "changed_outside_spec": cap.get("changed_outside_spec", []),
            "network_commands": network_commands(cell / "s01" / "stream.jsonl"), **stats}
    (cell / "spec.json").write_text(json.dumps(spec, indent=1))
    keep_cell(run_id, {"repo": repo}, cell)
    return spec


def backlog_earlier(repo: str, iid: str) -> list[str]:
    order = load_backlog(repo)["order"]
    return order[:order.index(iid)]


def spec_spend_problems(args, slots: list[dict]) -> list[str]:
    """Everything that must hold before a real author runs. Checked in full before any session."""
    problems = [p for p in (spec_study.brief_problem(i) for i in sorted({s["item"] for s in slots})) if p]
    if any(s["author"] == "opus" for s in slots):
        if auth_mode() != "api":
            problems.append(f"Opus authors run on the API: ADOPTION_AUTH=api, not {auth_mode()}")
        elif check_auth():
            problems.append(check_auth())
        if not 0 < args.budget_usd <= spec_study.PILOT_CAP_USD:
            problems.append(f"--budget-usd {args.budget_usd}: the pilot's cap is USD {spec_study.PILOT_CAP_USD}, "
                            "and a real run must state its budget")
    if any(s["author"] == "deepseek" for s in slots) and not os.environ.get(OPENCODE_KEY_ENV):
        problems.append(f"deepseek authors need {OPENCODE_KEY_ENV} in the environment (run under `secrets run --`)")
    for agent in sorted({spec_study.AUTHORS[s["author"]]["agent"] for s in slots}):
        problems += check_tools(agent)
    problems += [p for p in (registry_problem(r) for r in sorted({s["repo"] for s in slots})) if p]
    return problems


def cmd_spec(args) -> None:
    slots = json.loads(Path(args.plan).read_text())["specs"]
    if args.author:
        slots = [s for s in slots if s["author"] == args.author]
    if args.only:
        slots = [s for s in slots if s["slot"] in args.only]
    if not args.fixture:
        problems = spec_spend_problems(args, slots)
        if problems:
            raise SystemExit("not ready:\n  " + "\n  ".join(problems))
    (RUNS / args.run_id).mkdir(parents=True, exist_ok=True)
    spent, reserved = 0.0, 0.0
    lock = threading.Lock()

    def go(slot):
        nonlocal spent, reserved
        author = spec_study.AUTHORS[slot["author"]]
        hold = spec_study.AUTHOR_BUDGET_USD if slot["author"] == "opus" and not args.fixture else 0.0
        with lock:  # the cap is hard: a session starts only if its own cap still fits under it
            if hold and spent + reserved + hold > args.budget_usd:
                print(f"budget: {slot['slot']} not started ({spent:.2f} spent, {reserved:.2f} held)", flush=True)
                return None
            reserved += hold
        agent = f"fixture-author-{args.fixture}" if args.fixture else author["agent"]
        s = None
        try:
            s = run_author(args.run_id, slot, agent, author["model"], author["layout"])
        finally:
            with lock:
                reserved -= hold
                if s:
                    spent += s.get("cost_usd") or 0 if slot["author"] == "opus" else 0
        print(f"[{spent:7.2f} USD] {slot['slot']:32} {s['digest'][:12]} {s['spec_bytes']:7} bytes, "
              f"{s['questions_asked']} asked, {s['questions_answered']} answered", flush=True)
        return s

    with concurrent.futures.ThreadPoolExecutor(max_workers=args.jobs) as ex:
        done = [s for s in ex.map(go, slots) if s]
    print(f"{len(done)} of {len(slots)} specs written; spent {spent:.2f} USD on the API")


def cmd_spec_plan(args) -> None:
    slots, cells = spec_study.spec_slots(), spec_study.worker_cells()
    doc = {"study": "spec", "stage": "pilot", "specs": slots, "cells": cells, "max_sessions": len(slots) + len(cells)}
    if args.out:
        Path(args.out).write_text(json.dumps(doc, indent=1))
    if args.dry_run:
        for n, c in enumerate(cells, 1):
            print(f"{n:3}  {c['repo']:8} {c['item']:3} {c['arm']:11} author={c['author']:8} "
                  f"spec={c['spec_n']} worker={c['worker']} rep={c['rep']}")
        print()
    est = spec_study.estimate(slots, cells)
    by_author = {a: sum(1 for c in cells if c["author"] == a) for a in ("opus", "deepseek", "backlog")}
    print(f"{len(cells)} worker cells {by_author}; {len(slots)} specs to write "
          f"({est['opus_specs']} opus, {est['deepseek_specs']} deepseek); turn cap {est['worker_turn_cap']}")
    print(f"cash: USD {est['cash_usd'][0]} to {est['cash_usd'][1]} on Opus authoring (the protocol's guess per spec), "
          f"workers and DeepSeek authors free on the endpoint; hard cap USD {est['cap_usd']:.2f}, "
          f"USD {est['author_session_cap_usd']:.2f} per author session")
    print(f"at list price: USD {est['list_price_deepseek_session_usd']} per DeepSeek session at 60 turns, "
          f"so {est['list_price_workers_usd']} for workers and {est['list_price_deepseek_author_usd']} for DeepSeek "
          f"authors; total USD {est['list_price_total_usd'][0]} to {est['list_price_total_usd'][1]}")
    print("no model was called")


def cmd_spec_check(args) -> None:
    bad = 0
    for repo, item in spec_study.PILOT_ITEMS:
        res = spec_study.check_sufficiency(item, repo_def(repo) / "hidden" / item)
        print(f"{item}: {res['rows']} rows for {res['assertions']} assertions: {res['counts']}; "
              f"{len(res['clauses'])} clauses")
        for e in res["problems"]:
            print("  PROBLEM", e)
        bad += len(res["problems"])
    raise SystemExit(1 if bad else 0)


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
        ms = [s.get("metrics") or {} for r in rs for s in r["sessions"]]
        per_session = {
            "turns_per_session": round(sum(m.get("num_turns") or 0 for m in ms) / len(ms), 1) if ms else None,
            "ktokens_per_session": round(sum((m.get("input_tokens") or 0) + (m.get("cache_read_tokens") or 0)
                                             + (m.get("cache_creation_tokens") or 0) + (m.get("output_tokens") or 0)
                                             for m in ms) / len(ms) / 1000, 1) if ms else None,
        }
        if scenario == "directed":
            k = sum(1 for r in rs if r.get("success"))
            ctx = [s["metrics"].get("context_at_first_code_edit") for r in rs for s in r["sessions"]
                   if s["metrics"].get("context_at_first_code_edit") is not None]
            tbe = sorted(s["metrics"].get("turns_before_first_code_edit") for r in rs for s in r["sessions"]
                         if s["metrics"].get("turns_before_first_code_edit") is not None)
            per_session["median_turns_before_first_edit"] = median(tbe)
            per_session["no_code_edit"] = sum(1 for r in rs for s in r["sessions"]
                                              if s["metrics"].get("context_at_first_code_edit") is None)
            per_session["median_ktokens_per_cell"] = (round(median([cell_tokens(r) for r in rs]) / 1000, 1)
                                                      if rs else None)
            rows.append({"scenario": scenario, "arm": arm, "cells": len(rs), "success": k,
                         "success_ci95": wilson(k, len(rs)), "cost_usd": round(cost, 2),
                         "cost_per_success": round(cost / k, 2) if k else None,
                         "median_context_at_first_edit": sorted(ctx)[len(ctx) // 2] if ctx else None,
                         "false_completions": sum(1 for r in rs if r.get("tracker_says_done") and not r.get("success")),
                         "regressions": sum(len(r.get("regressions", [])) for r in rs), **per_session})
        elif scenario == "relay":
            passed = sum(len(r.get("items_passing", [])) for r in rs)
            cut = [r for r in rs if len(r["sessions"]) >= 3
                   and (r["sessions"][1]["metrics"].get("subtype") == "error_max_turns")]
            resumed = sum(1 for r in cut if r["sessions"][2].get("newly_passing"))
            rows.append({"scenario": scenario, "arm": arm, "cells": len(rs), "items_passing": passed,
                         "sessions": sum(r.get("sessions_used", 0) for r in rs), "cost_usd": round(cost, 2),
                         "cost_per_item": round(cost / passed, 2) if passed else None,
                         "false_completions_at_end": sum(len(r.get("false_completions_at_end", [])) for r in rs),
                         "session2_cut": len(cut), "resumed_in_session3": resumed, **per_session})
        else:
            rows.append({"scenario": scenario, "arm": arm, "cells": len(rs),
                         "items_passing_merged": sum(len(r.get("items_passing_merged", [])) for r in rs),
                         "duplicated_items": sum(len(r.get("duplicated_items", [])) for r in rs),
                         "merge_conflicts": sum(r.get("merge_conflicts", 0) for r in rs),
                         "cost_usd": round(cost, 2), **per_session})
    paired = paired_differences(results)
    wins = [win_rule(results, new, old) for new, old in VERSION_PAIRS
            if any(r["cell"]["arm"] == new for r in results) and any(r["cell"]["arm"] == old for r in results)]
    plain_rows = against_plain(results)
    (root / "results.json").write_text(json.dumps({"run_id": args.run_id, "rows": rows,
                                                   "paired_vs_openwarrant": paired, "version_pairs": wins,
                                                   "against_plain": plain_rows}, indent=1))
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
                  "| measure | openwarrant arm | other arm | cells | mean difference | interval |",
                  "|---|---|---|---|---|---|"]
        lines += [f"| {d['measure']} | {d.get('openwarrant_arm', 'openwarrant')} | {d['arm']} | {d['cells']} | "
                  f"{d['mean']} | {d['ci95']} |" for d in paired]
        lines.append("")
    for w in wins:
        lines += [f"## {w['new']} against {w['old']} (the win rule, directed cells both ran)", ""]
        lines += rule_lines(w)
    lines += plain_lines(plain_rows)
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
    install_beads()
    install_speckit()
    py = STUDY / "py"
    if not (py / "bin/python3").exists():
        subprocess.run(["python3", "-m", "venv", str(py)], check=True, timeout=300)
    subprocess.run([str(py / "bin/python3"), "-m", "pip", "install", "-q", f"pytest=={PINS['pytest']}"],
                   check=True, timeout=600)
    oc = Path(tools()["STUDY_OPENCODE"])
    if not oc.exists() and OPENCODE_SRC.exists():
        out = subprocess.run([str(OPENCODE_SRC), "--version"], capture_output=True, text=True, timeout=60).stdout
        if PINS["opencode"] in out:
            oc.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(OPENCODE_SRC, oc)
        else:
            print(f"opencode: {OPENCODE_SRC} is {out.strip()!r}, not the pinned {PINS['opencode']}")
    problems = check_tools("opencode")
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
    s = sub.add_parser("seed"); s.add_argument("repo"); s.add_argument("arm", choices=arms.ALL_ARMS)
    s.add_argument("dest"); s.add_argument("--done", nargs="*", default=[])
    s.add_argument("--spec", help="a stored spec's digest, in place of backlog.toml")
    s.add_argument("--layout", choices=("claude", "opencode"),
                   help="whose files: with --spec the default is opencode, without it claude")
    pa = sub.add_parser("parity"); pa.add_argument("repo"); pa.add_argument("--keep", action="store_true")
    pl = sub.add_parser("plan"); pl.add_argument("--pilot", action="store_true")
    pl.add_argument("--smoke", action="store_true", help="one directed tally T6 session, plain arm")
    pl.add_argument("--reps", type=int, default=3); pl.add_argument("--k", type=int, default=3)
    pl.add_argument("--out")
    pl.add_argument("--arms", nargs="*", choices=arms.ALL_ARMS, help="arms for the full plan (default: all but jira)")
    pl.add_argument("--scenarios", nargs="*", choices=SCENARIOS, help="only these scenarios (default: all)")
    ex = sub.add_parser("execute"); ex.add_argument("plan"); ex.add_argument("--run-id", required=True)
    ex.add_argument("--agent", required=True); ex.add_argument("--model", default=DEFAULT_MODEL)
    ex.add_argument("--jobs", type=int, default=2); ex.add_argument("--budget-usd", type=float, default=0)
    ex.add_argument("--turns", type=int, default=SESSION_TURNS, help="a session's turn cap (amendment 5: 60)")
    ex.add_argument("--session-budget-usd", type=float,
                    help="a session's own cap, in place of USD 8; with it --budget-usd is hard: a cell "
                         "starts only if its sessions' caps still fit under the budget (amendment 8)")
    ex.add_argument("--specs-run", help="the run whose specs a spec plan's worker cells use")
    ex.add_argument("--skip-unwritten", action="store_true", help="leave out cells whose spec was never written")
    sp = sub.add_parser("spec"); sp.add_argument("plan"); sp.add_argument("--run-id", required=True)
    sp.add_argument("--author", choices=spec_study.MODEL_AUTHORS); sp.add_argument("--only", nargs="*")
    sp.add_argument("--jobs", type=int, default=1); sp.add_argument("--budget-usd", type=float, default=0)
    sp.add_argument("--fixture", choices=("good", "bad", "empty", "probe"), help="a scripted author: no model, no brief freeze")
    pp = sub.add_parser("spec-plan"); pp.add_argument("--dry-run", action="store_true"); pp.add_argument("--out")
    sub.add_parser("spec-check")
    sub.add_parser("spec-selfcheck")
    ic = sub.add_parser("isolation-check", help="amendment 7: the leak probe and a reference relay, no model")
    ic.add_argument("--run-id", required=True); ic.add_argument("--repo", default="tally")
    ic.add_argument("--arm", default="openwarrant@a7")
    rp = sub.add_parser("report"); rp.add_argument("run_id")
    po = sub.add_parser("pool"); po.add_argument("run_ids", nargs="+")
    po.add_argument("--exclude", nargs="*", default=[], metavar="RUN/REPO/ITEM",
                    help="an item of a run left out for every arm: a harness error (amendment 5's rule)")
    a8 = sub.add_parser("a8-check", help="amendment 8's checks, no model"); a8.add_argument("--run-id", required=True)
    # inside the sandbox only
    g = sub.add_parser("_grade"); g.add_argument("repo")
    t = sub.add_parser("_tracker"); t.add_argument("repo"); t.add_argument("arm")
    an = sub.add_parser("_answers"); an.add_argument("repo"); an.add_argument("arm")
    gi = sub.add_parser("_git"); gi.add_argument("action"); gi.add_argument("arg", nargs="?")
    sc = sub.add_parser("_spec_capture"); sc.add_argument("tool"); sc.add_argument("out"); sc.add_argument("base", nargs="?")
    sa = sub.add_parser("_spec_apply"); sa.add_argument("arm")
    st = sub.add_parser("_spec_tracker"); st.add_argument("arm"); st.add_argument("item")
    args = ap.parse_args()

    if args.cmd == "prepare":
        cmd_prepare(args)
    elif args.cmd == "validate":
        cmd_validate(args)
    elif args.cmd == "seed":
        if args.spec:
            dest = Path(args.dest).resolve()
            dest.parent.mkdir(parents=True, exist_ok=True)
            sb = Sandbox(dest.parent, args.repo, arm=args.arm, agent="fixture", layout=args.layout, work=dest.parent)
            seed_spec(sb, args.repo, args.arm, dest, args.done, args.spec, AGENT_SEED.get(args.layout or "opencode", {}))
        else:
            seed_repo(args.repo, args.arm, Path(args.dest), args.done, AGENT_SEED.get(args.layout or "claude"))
        print(args.dest)
    elif args.cmd == "parity":
        cmd_parity(args)
    elif args.cmd == "plan":
        cells = ([{"scenario": "directed", "repo": "tally", "arm": "plain", "item": "T6", "rep": 1}]
                 if args.smoke else plan_cells(args.pilot, args.reps, args.k, args.arms))
        if args.scenarios:
            cells = [c for c in cells if c["scenario"] in args.scenarios]
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
    elif args.cmd == "spec":
        cmd_spec(args)
    elif args.cmd == "spec-plan":
        cmd_spec_plan(args)
    elif args.cmd == "spec-check":
        cmd_spec_check(args)
    elif args.cmd == "spec-selfcheck":
        import spec_selfcheck
        spec_selfcheck.main()
    elif args.cmd == "isolation-check":
        cmd_isolation_check(args)
    elif args.cmd == "report":
        cmd_report(args)
    elif args.cmd == "pool":
        cmd_pool(args)
    elif args.cmd == "a8-check":
        cmd_a8_check(args)
    elif args.cmd == "_grade":
        print(json.dumps(_inside_grade(args.repo)))
    elif args.cmd == "_answers":
        print(json.dumps(_inside_answers(args.repo, args.arm)))
    elif args.cmd == "_tracker":
        print(json.dumps(_inside_tracker(args.repo, args.arm)))
    elif args.cmd == "_git":
        print(json.dumps(_inside_git(args.action, args.arg)))
    elif args.cmd == "_spec_capture":
        print(json.dumps(_inside_spec_capture(args.tool, args.out, args.base)))
    elif args.cmd == "_spec_apply":
        print(json.dumps(_inside_spec_apply(args.arm)))
    elif args.cmd == "_spec_tracker":
        print(json.dumps(_inside_spec_tracker(args.arm, args.item)))


if __name__ == "__main__":
    main()
