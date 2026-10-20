#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Run the Format 1.0 conformance corpus (conformance/format/) against an implementation.

    run.py --reader war    [--war PATH] [--known FILE] [CASE...]
    run.py --reader python [CASE...]
    run.py --bless CASE...     (write the expected result from the python modules;
                                review it against the specification before committing)

Four kinds of case (README.md, "Layout"): model cases (the reader), context
cases (`args`, no `at`: the context compiler), tracking cases (`at`: the
tracker and evaluator) and apply cases (`at` and `package.md`).

A case passes when the reader's normalized output equals the expected bytes
(README.md, "What is compared"). Exit 0 when every case passes, 1 when one
fails, 2 on a setup failure. With `--known`, a case listed there is expected
to fail (a recorded divergence) and is reported `known`; a listed case that
passes fails the run, so the list never outlives the divergence.
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import tomllib

HERE = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.dirname(os.path.dirname(HERE))

REFUSALS = {
    "format.encoding", "format.frontmatter", "format.key-unknown", "format.heading",
    "format.title-missing", "format.attribute", "format.field-duplicate",
    "format.field-missing", "format.authored-state", "format.relation-kind",
    "format.relation-target", "format.item", "format.id-duplicate",
    "format.fence-unclosed", "format.section-missing", "format.document-duplicate",
    "format.manifest-restates", "format.unrecognised", "context.target-unknown",
    "context.over-budget", "format.field-shape", "profile.invalid", "apply.stale",
    "apply.identity", "work.stale-revision", "work.claimed-by-other", "work.journal-rewritten",
    "work.journal-malformed", "work.journal-torn", "work.idempotency-conflict",
}
UNKNOWNS = {"format.newer-major", "format.newer-minor", "format.requires-unknown",
            "format.profile-unknown", "work.claim-remote-unreachable", "work.journal-newer",
            "check.unknown"}
WARNINGS = {"format.target-unknown", "format.refusal-missing", "format.item-id-missing",
            "format.legacy-obligation", "context.summary-shape", "context.statement-shape",
            "context.section-ref", "work.tick-unbacked", "format.prefix-unregistered"}
# `context.span-unknown` (10.3) is reported for a legacy record whose span the
# model does not give. The corpus's documents are Format 1.0 and the python
# reader reads no legacy encoding, so no case expects it yet.
UNCOVERABLE = {"context.span-unknown"}
SPEC_RULES = REFUSALS | UNKNOWNS | WARNINGS

ENV = {k: v for k, v in os.environ.items()
       if k not in ("SSH_AUTH_SOCK", "SSH_AGENT_PID", "OPENWARRANT_ACTOR")
       and not k.startswith(("GIT_", "CLAUDE_"))}
ENV.update(OPENWARRANT_NO_PROJECTS="1", OPENWARRANT_NO_UPDATE_CHECK="1")


def jcs(value) -> bytes:
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def cases(selected):
    out = []
    for name in sorted(os.listdir(HERE)):
        d = os.path.join(HERE, name)
        if os.path.isfile(os.path.join(d, "case.toml")) and (not selected or name in selected
                                                              or name.split("-")[0] in selected):
            out.append(d)
    return out


def input_files(case):
    base = os.path.join(case, "input")
    out = set()
    for dp, _, fs in os.walk(base):
        for f in fs:
            out.add(os.path.relpath(os.path.join(dp, f), base).replace(os.sep, "/"))
    return out


# ---------------------------------------------------------------- readers --

def case_kind(case):
    """model, context, tracking or apply (README.md, "Layout")."""
    if os.path.isfile(os.path.join(case, "at")):
        return "apply" if os.path.isfile(os.path.join(case, "package.md")) else "tracking"
    if os.path.isfile(os.path.join(case, "args")):
        return "context"
    return "model"


def read_args(case):
    import shlex
    with open(os.path.join(case, "args")) as f:
        return shlex.split(f.read())


def touch_claims(store):
    """A claim file's modification time is its `since_unix` (README.md, "Tracking cases")."""
    if not os.path.isdir(store):
        return
    for dp, _, fs in os.walk(store):
        for f in fs:
            if f.endswith(".lock"):
                p = os.path.join(dp, f)
                with open(p) as fh:
                    c = json.load(fh)
                os.utime(p, (c["since_unix"], c["since_unix"]))


def evidence_journal(tree, alias):
    for place in ("warrants", "tickets"):
        if (os.path.isfile(os.path.join(tree, "docs", place, alias + ".md"))
                or os.path.isdir(os.path.join(tree, "docs", place, alias))):
            return os.path.join(tree, "docs", place, alias, "journal.jsonl")
    raise RuntimeError(f"journal/{alias}.jsonl: no work document {alias} in input/")


def tracking_tree(case, work, prepare=None):
    """input/ as a git working tree; journal/ at the evidence directories; claims/ in the store.

    `prepare(tree)`, when given, runs after input/ is copied and before the
    first commit: a reader that needs files of its own beside the case's
    (war's `openwarrant.toml`) writes them there.
    """
    tree = os.path.join(work, "repo")
    shutil.copytree(os.path.join(case, "input"), tree)
    if prepare is not None:
        prepare(tree)
    jdir = os.path.join(case, "journal")
    git = ["git", "-c", "user.name=corpus", "-c", "user.email=corpus@invalid",
           "-c", "commit.gpgsign=false", "-c", "tag.gpgsign=false", "-c", "init.defaultBranch=main"]
    subprocess.run(git + ["init", "-q", "."], cwd=tree, env=ENV, check=True, timeout=60,
                   stdin=subprocess.DEVNULL)
    committed = []
    if os.path.isdir(jdir):
        for name in sorted(os.listdir(jdir)):
            if name.endswith(".committed.jsonl"):
                dst = evidence_journal(tree, name[: -len(".committed.jsonl")])
                os.makedirs(os.path.dirname(dst), exist_ok=True)
                shutil.copyfile(os.path.join(jdir, name), dst)
                committed.append(dst)
    subprocess.run(git + ["add", "-A"], cwd=tree, env=ENV, check=True, timeout=60)
    subprocess.run(git + ["commit", "-q", "--allow-empty", "-m", "corpus input"], cwd=tree, env=ENV,
                   check=True, timeout=60, stdin=subprocess.DEVNULL)
    if os.path.isdir(jdir):
        for name in sorted(os.listdir(jdir)):
            if name.endswith(".jsonl") and not name.endswith(".committed.jsonl"):
                dst = evidence_journal(tree, name[: -len(".jsonl")])
                os.makedirs(os.path.dirname(dst), exist_ok=True)
                shutil.copyfile(os.path.join(jdir, name), dst)
    cdir = os.path.join(case, "claims")
    if os.path.isdir(cdir):
        store = os.path.join(tree, ".git", "openwarrant", "claims")
        shutil.copytree(cdir, store)
        touch_claims(store)
    return tree


def run_json(argv):
    p = subprocess.run([sys.executable, "-I", *argv], capture_output=True, text=True, timeout=120)
    if p.returncode != 0:
        raise RuntimeError(p.stderr.strip()[-400:])
    return json.loads(p.stdout)


def python_reader(case, work):
    kind = case_kind(case)
    if kind == "model":
        tree = os.path.join(work, "repo")
        shutil.copytree(os.path.join(case, "input"), tree)
        return run_json([os.path.join(HERE, "reader.py"), tree])
    if kind == "context":
        tree = os.path.join(work, "repo")
        shutil.copytree(os.path.join(case, "input"), tree)
        touch_claims(os.path.join(tree, ".openwarrant", "state", "claims"))
        out = run_json([os.path.join(HERE, "context.py"), tree, *read_args(case)])
        return {"records": [], "relations": [], "findings": out["findings"], "output": out["output"]}
    tree = tracking_tree(case, work)
    with open(os.path.join(case, "at")) as f:
        at = f.read().strip()
    argv = [os.path.join(HERE, "tracker.py"), tree, "--at", at]
    if os.path.isdir(os.path.join(case, "acts")):
        argv += ["--acts", os.path.join(case, "acts")]
    args = read_args(case)
    if kind == "apply":
        args = [args[0], os.path.join(case, "package.md"), *args[1:]]
    out = run_json([*argv, *args])
    return {"records": [], "relations": [], "findings": out["findings"],
            "states": out.get("states", []), "next": out.get("next"), "tree": tree}


class War:
    def __init__(self, war, work):
        self.war = os.path.abspath(war)
        self.template = os.path.join(work, "template")
        os.makedirs(self.template)
        self._run(["git", "init", "-q", "."], self.template)
        # A program for the documents to live in. Its own namespace (ZQ) is one
        # no case uses, and nothing it writes is compared (only the case's files are).
        self._run([self.war, "init", "--program", "Format corpus", "--namespace", "ZQ",
                   "--root", "."], self.template)
        # A case marked `bare = true` runs where `war init` alone was run: no
        # program and no Warrant, as ordinary work keeps documents.
        self.bare = os.path.join(work, "bare")
        os.makedirs(self.bare)
        self._run(["git", "init", "-q", "."], self.bare)
        self._run([self.war, "init", "--root", "."], self.bare)

    def template_for(self, case):
        with open(os.path.join(case, "case.toml"), "rb") as f:
            return self.bare if tomllib.load(f).get("bare") else self.template

    @staticmethod
    def _run(argv, cwd):
        p = subprocess.run(argv, cwd=cwd, env=ENV, capture_output=True, text=True,
                           stdin=subprocess.DEVNULL, timeout=120)
        if p.returncode != 0:
            raise RuntimeError(f"{argv[0]} {argv[1]}: exit {p.returncode}: {p.stderr.strip()[-400:]}")
        return p

    def apply(self, case, work):
        """An apply case (10.5): `war apply package.md --as A --at T` over input/
        committed in the scratch program; the documents after are compared."""
        tree = os.path.join(work, "repo")
        shutil.copytree(self.template, tree, symlinks=True)
        shutil.copytree(os.path.join(case, "input"), tree, dirs_exist_ok=True)
        git = ["git", "-c", "user.name=corpus", "-c", "user.email=corpus@invalid",
               "-c", "commit.gpgsign=false"]
        self._run(git + ["add", "-A"], tree)
        self._run(git + ["commit", "-q", "--allow-empty", "-m", "corpus input"], tree)
        with open(os.path.join(case, "at")) as f:
            at = f.read().strip()
        args = read_args(case)
        if args[:1] != ["apply"]:
            raise RuntimeError(f"an apply case's args start with `apply`: {args}")
        p = subprocess.run([self.war, "--json", "apply", os.path.join(case, "package.md"),
                            *args[1:], "--at", at, "--root", tree], cwd=tree, env=ENV,
                           capture_output=True, text=True, stdin=subprocess.DEVNULL, timeout=120)
        try:
            env = json.loads(p.stdout)
        except json.JSONDecodeError:
            raise RuntimeError(f"war apply: exit {p.returncode}: {p.stderr.strip()[-400:]}")
        findings = sorted(f"{d['rule']} {d['file']}" for d in env.get("diagnostics", [])
                          if d.get("rule") in SPEC_RULES and d.get("file"))
        return {"records": [], "relations": [], "findings": findings, "tree": tree}

    # ------------------------------------------------------------ tracking --
    #
    # A tracking case (README.md, "Tracking cases") is one operation over
    # input/ as a committed git tree with its journals and claim store
    # (`tracking_tree`), under the minimal config war needs (a case's own
    # `openwarrant.toml`, its `[claims]` or `[next]`, appended). Every war
    # command runs `--at` the case's time, so a lease or a date is judged
    # then, not now. The states of 11.1 are `war plan states` (`war_states`).

    CONFIG = ('schema = "oh.war/repository-config/v1"\n\n'
              '[project]\nname = "Format corpus"\nnamespace = "ZQ"\n')

    def _prepare(self, tree):
        own = os.path.join(tree, "openwarrant.toml")
        extra = open(own).read() if os.path.isfile(own) else ""
        with open(own, "w") as f:
            f.write(self.CONFIG + ("\n" + extra if extra else ""))

    def _war(self, tree, *argv):
        """`war --json <argv> --root <tree>`: the report envelope, and its exit code."""
        env = dict(ENV, **self.acts_env)
        p = subprocess.run([self.war, "--json", *argv, "--root", tree], cwd=tree, env=env,
                           capture_output=True, text=True, stdin=subprocess.DEVNULL, timeout=120)
        try:
            return json.loads(p.stdout), p.returncode
        except json.JSONDecodeError:
            raise RuntimeError(f"war {argv[0]}: exit {p.returncode}: {p.stderr.strip()[-400:]}")

    def _track(self, case, work, op):
        """Set up the case's tree and run `op(tree, rest of args)`."""
        tree = tracking_tree(case, work, self._prepare)
        with open(os.path.join(case, "at")) as f:
            self.at = f.read().strip()
        self.acts_env = war_acts_env(case)
        out = {"records": [], "relations": [], "findings": [], "states": [], "next": None,
               "tree": tree}
        out.update(op(tree, read_args(case)[1:]))
        out["findings"] = sorted(out["findings"])
        return out

    def _states(self, tree, findings=()):
        """`war plan states`: the states of 11.1, and the findings of reading them."""
        report, _ = self._war(tree, "plan", "states", "--at", self.at)
        found = set(findings) | war_findings(report)
        states = (report.get("result") or {}).get("states", [])
        return {"states": states, "findings": found}

    def _write(self, tree, argv):
        """A write; its refusals, or else the states after it."""
        report, code = self._war(tree, *argv, "--at", self.at)
        found = war_findings(report)
        if code != 0 or any(f.split(" ")[0] in REFUSALS | UNKNOWNS for f in found):
            if not found:
                msgs = [d.get("message", "")[:200] for d in report.get("diagnostics", [])]
                raise RuntimeError(f"war {argv[0]}: exit {code}: {msgs}")
            return {"findings": found}
        return self._states(tree, found)

    @staticmethod
    def _flags(rest, *names):
        """`--actor A --note T` and bare `--steal` into a dict; the first word is the target."""
        target, opts, i = rest[0], {}, 1
        while i < len(rest):
            key = rest[i].lstrip("-")
            if key in names and i + 1 < len(rest) and not rest[i + 1].startswith("--"):
                opts[key], i = rest[i + 1], i + 2
            else:
                opts[key], i = True, i + 1
        return target, opts

    def track_states(self, case, work):
        """`states`: `war plan states --at <at>`."""
        return self._track(case, work, lambda tree, rest: self._states(tree))

    def track_next(self, case, work):
        """`next`: the candidates of `war next --at <at>`, in order."""
        def op(tree, rest):
            report, _ = self._war(tree, "next", "--at", self.at)
            ready = (report.get("result") or {}).get("ready", [])
            cands = [r["ticket"] + ("/" + r["item"] if r.get("item") else "") for r in ready]
            # `next`'s warnings are those of the states it is computed from.
            found = war_findings(report) | self._states(tree)["findings"]
            return {"next": cands, "findings": found}
        return self._track(case, work, op)

    def track_claim(self, case, work):
        """`claim <target> --actor A [--steal]`: `war claim`, then the states."""
        def op(tree, rest):
            target, o = self._flags(rest, "actor")
            steal = ["--steal"] if o.get("steal") else []
            return self._write(tree, ["claim", target, "--as", o["actor"], *steal])
        return self._track(case, work, op)

    def track_done(self, case, work):
        """`done <target> --actor A --base R`: `war done --if-rev R`, then the states."""
        def op(tree, rest):
            target, o = self._flags(rest, "actor", "base")
            return self._write(tree, ["done", target, "--as", o["actor"], "--if-rev", o["base"]])
        return self._track(case, work, op)

    def track_note(self, case, work):
        """`note <target> --actor A --note T`: `war note`, then the states."""
        def op(tree, rest):
            target, o = self._flags(rest, "actor", "note")
            return self._write(tree, ["note", target, o["note"], "--as", o["actor"]])
        return self._track(case, work, op)

    def read(self, case, work):
        kind = case_kind(case)
        if kind == "tracking" and read_args(case)[:1] == ["evaluate"]:
            return war_evaluate(self.war, case, work)
        if kind == "apply":
            return self.apply(case, work)
        if kind in ("tracking", "apply"):
            # A tracking case names one operation (README.md, "Tracking
            # cases"); each has a method `track_<op>`. An operation with none
            # is listed in known-war.txt.
            op = read_args(case)[0]
            handler = getattr(self, "track_" + op.replace("-", "_"), None)
            if kind == "apply" or handler is None:
                raise RuntimeError(f"no war adapter for {kind} operation {op!r}")
            return handler(case, work)
        tree = os.path.join(work, "repo")
        template = self.template_for(case)
        shutil.copytree(template, tree, symlinks=True)
        shutil.copytree(os.path.join(case, "input"), tree, dirs_exist_ok=True)
        project_name_over_template(case, template, tree)
        files = input_files(case)
        argv_file = os.path.join(case, "args")
        if os.path.isfile(argv_file):
            args = read_args(case)
            # As for the python reader: a claim file's modification time is its since_unix.
            touch_claims(os.path.join(tree, ".openwarrant", "state", "claims"))
            # A preset that reads state digests every journal (10.4): the case's
            # own, not the scratch program's.
            for dp, _, fs in os.walk(os.path.join(tree, "docs")):
                for f in fs:
                    rel = os.path.relpath(os.path.join(dp, f), tree).replace(os.sep, "/")
                    if f == "journal.jsonl" and rel not in files:
                        os.remove(os.path.join(dp, f))
            if os.path.isfile(os.path.join(case, "expected", "context.md")):
                p = subprocess.run([self.war, "context", *args, "--root", tree], cwd=tree,
                                   env=ENV, capture_output=True, text=True,
                                   stdin=subprocess.DEVNULL, timeout=120)
                output = p.stdout
                if "--index" in args:
                    # The index (10.2) of the case's own files: the scratch
                    # program's documents are not the case's (as for model cases).
                    head, *blocks = output.split("\n## ")
                    output = head + "".join(
                        "\n## " + b for b in blocks if b.split("\n", 1)[0] in files)
                return {"records": [], "relations": [], "findings": [], "output": output,
                        "context_exit": p.returncode}
            p = subprocess.run([self.war, "--json", "context", *args, "--root", tree], cwd=tree,
                               env=ENV, capture_output=True, text=True,
                               stdin=subprocess.DEVNULL, timeout=120)
            try:
                env = json.loads(p.stdout)
            except json.JSONDecodeError:
                return {"records": [], "relations": [], "findings": [],
                        "context_exit": p.returncode, "stderr": p.stderr[-300:]}
            rules = sorted({d["rule"] for d in env.get("diagnostics", [])
                            if d.get("rule") in SPEC_RULES})
            return {"records": [], "relations": [], "findings": rules}
        p = subprocess.run([self.war, "model", "--json", "--root", tree], cwd=tree, env=ENV,
                           capture_output=True, text=True, stdin=subprocess.DEVNULL, timeout=120)
        try:
            model = json.loads(p.stdout)["result"]
        except (json.JSONDecodeError, KeyError, TypeError):
            raise RuntimeError(f"war model: exit {p.returncode}: {p.stderr.strip()[-400:]}")
        recs = [r for r in model["records"] if r.get("source") in files]
        # A record's container: its work document, or else its file's own
        # `doc:<path>` record (section 4).
        doc_of = {r["id"]: r.get("document") or f"doc:{r['source']}" for r in recs}
        rels = []
        for r in model["relations"]:
            if r["from"] not in doc_of:
                continue
            # Containment is carried by the record's `document`; a reader that
            # also emits it as `part_of <document>` says nothing more.
            if r["kind"] == "part_of" and r["to"] == doc_of[r["from"]] != r["from"]:
                continue
            rels.append({k: r[k] for k in ("from", "kind", "to", "to_revision") if k in r})
        findings = {f"{d['rule']} {d['record']}" for d in model["diagnostics"]
                    if d.get("rule") in SPEC_RULES}
        # `war check` is the checker over the same reading: it adds the rules
        # the model does not carry (format.target-unknown). It also runs the
        # tracker of section 11, whose `work.*` findings a model case does not
        # compare: tracking cases compare them in `states.json` (402).
        p = subprocess.run([self.war, "--json", "check", "--root", tree], cwd=tree, env=ENV,
                           capture_output=True, text=True, stdin=subprocess.DEVNULL, timeout=120)
        try:
            check = json.loads(p.stdout)
        except json.JSONDecodeError:
            raise RuntimeError(f"war check: exit {p.returncode}: {p.stderr.strip()[-400:]}")
        findings |= {f"{d['rule']} {d['file']}" for d in check.get("diagnostics", [])
                     if d.get("rule") in SPEC_RULES and d.get("file")
                     and not d["rule"].startswith("work.")}
        findings = sorted(findings)
        return {
            "records": [{k: r.get(k) for k in ("id", "type", "title", "fields", "ext",
                                               "content_digest")} for r in recs],
            "relations": rels, "findings": findings,
        }


# ------------------------------------------------- war: evaluate (12) --

def war_run(war, tree, argv):
    """`war <argv> --root <tree>` in `tree`: (exit code, stdout, stderr)."""
    p = subprocess.run([war, *argv, "--root", tree], cwd=tree, env=ENV, capture_output=True,
                       text=True, stdin=subprocess.DEVNULL, timeout=120)
    return p.returncode, p.stdout, p.stderr


def war_findings(report):
    """`<rule id> <path>:<line>` for each spec rule a war report names. A claim
    file is named from the claim store (`claims/<file>:1`, README.md)."""
    out = set()
    for d in report.get("diagnostics", []):
        rule = d.get("rule")
        if rule not in SPEC_RULES:
            continue
        where = d.get("file") or ""
        marker = "openwarrant/claims/"
        if marker in where:
            # A claim file is one JSON object: its finding is at line 1.
            where = "claims/" + where.split(marker, 1)[1]
            where += "" if ":" in where else ":1"
        out.add(f"{rule} {where}".rstrip())
    return out


def war_states(war, tree, records, at=None, env=None):
    """The states of 11.1 of `records` (aliases, and their items), as `war plan
    states` reports them: `{record, kind, value, provenance, refines?, lapsed?}`,
    `kind` the family. The one reading of war's states every adapter shares."""
    argv = [war, "--json", "plan", "states", "--root", tree] + (["--at", at] if at else [])
    p = subprocess.run(argv, cwd=tree, env=env or ENV, capture_output=True, text=True,
                       stdin=subprocess.DEVNULL, timeout=120)
    try:
        states = json.loads(p.stdout)["result"]["states"]
    except (json.JSONDecodeError, KeyError, TypeError):
        raise RuntimeError(f"war plan states: exit {p.returncode}: {p.stderr.strip()[-300:]}")
    return [st for st in states if st["record"].split("/")[0] in records]


def war_evaluate(war, case, work):
    """`evaluate --actor A` (README, "Tracking cases") as `war check evaluate --as A --at <at>`."""
    import shlex
    tree = tracking_tree(case, work)
    with open(os.path.join(case, "at")) as f:
        at = f.read().strip()
    # A program for the documents to live in, as for a model case; nothing
    # it writes is compared.
    code, _, err = war_run(war, tree, ["init", "--program", "Format corpus", "--namespace", "ZQ"])
    if code != 0:
        raise RuntimeError(f"war init: exit {code}: {err.strip()[-300:]}")
    args = read_args(case)
    sub = argparse.ArgumentParser(prog="evaluate")
    sub.add_argument("--actor", required=True)
    a = sub.parse_args(args[1:])
    code, out, err = war_run(war, tree, ["--json", "check", "evaluate", "--as", a.actor,
                                         "--at", at])
    try:
        env = json.loads(out)
    except json.JSONDecodeError:
        raise RuntimeError(f"war check evaluate: exit {code}: {err.strip()[-300:]} {shlex.join(args)}")
    findings = {d["rule"] for d in env.get("diagnostics", []) if d.get("rule") in SPEC_RULES}
    records = set()
    for dp, _, fs in os.walk(os.path.join(case, "input")):
        for f in fs:
            if f.endswith(".md"):
                records.add(f[:-3] if f != "ticket.md" else os.path.basename(dp))
    # 11.4's warnings over the documents after the evaluation.
    _, out, _ = war_run(war, tree, ["--json", "check"])
    try:
        diags = json.loads(out).get("diagnostics", [])
    except json.JSONDecodeError:
        diags = []
    findings |= {f"{d['rule']} {d['file']}" for d in diags
                 if d.get("rule") in WARNINGS and d.get("file")
                 and d["file"].split(":")[0].startswith("docs/")}
    return {"records": [], "relations": [], "findings": sorted(findings),
            "states": war_states(war, tree, records, at), "tree": tree}


def project_name_over_template(case, template, tree):
    """A case's `openwarrant.toml` names its project (10.2); war's scratch
    program keeps its own configuration with that name in place of its own."""
    import tomllib
    given = os.path.join(case, "input", "openwarrant.toml")
    if not os.path.isfile(given):
        return
    with open(given, "rb") as f:
        name = tomllib.load(f)["project"]["name"]
    with open(os.path.join(template, "openwarrant.toml")) as f:
        text = f.read()
    text = re.sub(r'(?m)^name = ".*"$', "name = " + json.dumps(name), text, count=1)
    with open(os.path.join(tree, "openwarrant.toml"), "w") as f:
        f.write(text)


def war_acts_env(case):
    """Tracking cases: `acts/*.json` are acceptance acts taken as verified (README.md).

    The corpus carries no signature. A debug build of war reads such a
    directory from OPENWARRANT_CORPUS_ACTS, and says so on standard error; a
    release build has no such path, so it believes only signed acts.
    """
    acts = os.path.join(case, "acts")
    return {"OPENWARRANT_CORPUS_ACTS": acts} if os.path.isdir(acts) else {}


# ---------------------------------------------------------------- compare --

def normalize(out):
    recs = sorted(out["records"], key=lambda r: r["id"].encode())
    for r in recs:
        if r.get("title") is None:
            r.pop("title", None)
    rels = sorted(out["relations"],
                  key=lambda r: (r["from"], r["kind"], r["to"], r.get("to_revision", "")))
    return recs, rels, out["findings"]


def after_files(case):
    base = os.path.join(case, "expected", "after")
    out = []
    for dp, _, fs in os.walk(base):
        for f in fs:
            out.append(os.path.relpath(os.path.join(dp, f), base).replace(os.sep, "/"))
    return sorted(out)


def verdict(case, out):
    """(passed, detail)"""
    exp = os.path.join(case, "expected")
    recs, rels, findings = normalize(out)
    rule_of = [f.split(" ")[0] for f in findings]
    refusals = sorted(f for f, r in zip(findings, rule_of) if r in REFUSALS)
    unknowns = sorted({r for r in rule_of if r in UNKNOWNS})
    warnings = sorted(f for f, r in zip(findings, rule_of) if r in WARNINGS)
    if os.path.isfile(os.path.join(exp, "refusal")):
        want = open(os.path.join(exp, "refusal")).read().strip()
        got = refusals
        return got == [want], f"refusals {got or 'none'}, want [{want}]"
    if os.path.isfile(os.path.join(exp, "unknown")):
        want = open(os.path.join(exp, "unknown")).read().strip()
        ok = unknowns == [want] and not refusals
        return ok, f"unknown {unknowns or 'none'}, refusals {refusals or 'none'}, want [{want}]"
    if refusals or unknowns:
        return False, f"refused or UNKNOWN: {refusals + unknowns}"
    if os.path.isfile(os.path.join(exp, "context.md")):
        want = open(os.path.join(exp, "context.md"), "rb").read()
        got = out.get("output", "").encode()
        if got == want:
            return True, ""
        gl, wl = got.decode().split("\n"), want.decode().split("\n")
        for i, (g, w) in enumerate(zip(gl, wl)):
            if g != w:
                return False, f"context.md line {i + 1}: got {g[:100]!r}, want {w[:100]!r}"
        return False, f"context.md: {len(gl)} lines, want {len(wl)}"
    if os.path.isfile(os.path.join(exp, "states.json")):
        want = json.load(open(os.path.join(exp, "states.json")))
        got = {"states": out.get("states", []), "warnings": warnings}
        if jcs(got) == jcs(want):
            return True, ""
        w = {json.dumps(x, sort_keys=True) for x in want["states"]}
        g = {json.dumps(x, sort_keys=True) for x in got["states"]}
        return False, (f"states: missing {sorted(w - g)[:3]} unexpected {sorted(g - w)[:3]}; "
                       f"warnings {warnings}, want {want['warnings']}")
    if os.path.isfile(os.path.join(exp, "next.json")):
        want = json.load(open(os.path.join(exp, "next.json")))
        got = {"candidates": out.get("next") or [], "warnings": warnings}
        return jcs(got) == jcs(want), f"next {got}, want {want}"
    if os.path.isdir(os.path.join(exp, "after")):
        for rel in after_files(case):
            p = os.path.join(out["tree"], rel)
            want = open(os.path.join(exp, "after", rel), "rb").read()
            got = open(p, "rb").read() if os.path.isfile(p) else None
            if got != want:
                return False, f"after/{rel} differs" if got is not None else f"after/{rel} is missing"
        return True, ""
    with open(os.path.join(exp, "model.json"), "rb") as f:
        want_bytes = f.read()
    got = {"records": recs, "relations": rels, "warnings": warnings}
    got_bytes = jcs(got)
    if got_bytes == jcs(json.loads(want_bytes)):
        return True, ""
    want = json.loads(want_bytes)
    diffs = []
    for key in ("records", "relations", "warnings"):
        if jcs(want.get(key)) != jcs(got.get(key)):
            w = {json.dumps(x, sort_keys=True, ensure_ascii=False) for x in want.get(key, [])}
            g = {json.dumps(x, sort_keys=True, ensure_ascii=False) for x in got.get(key, [])}
            diffs.append(f"{key}: missing {sorted(w - g)[:3]} unexpected {sorted(g - w)[:3]}")
    return False, "; ".join(diffs)


def bless(case, out):
    """Write the accept result the python modules give. Refuse and unknown cases are written by hand."""
    name = os.path.basename(case)
    recs, rels, findings = normalize(out)
    warnings = [f for f in findings if f.split(" ")[0] in WARNINGS]
    bad = [f for f in findings if f.split(" ")[0] not in WARNINGS]
    if bad:
        print(f"{name}: not an accept case: {bad}", file=sys.stderr)
        return
    exp = os.path.join(case, "expected")
    os.makedirs(exp, exist_ok=True)
    kind = case_kind(case)
    if kind == "model":
        with open(os.path.join(exp, "model.json"), "wb") as f:
            f.write(jcs({"records": recs, "relations": rels, "warnings": warnings}) + b"\n")
    elif kind == "context":
        with open(os.path.join(exp, "context.md"), "wb") as f:
            f.write(out["output"].encode())
    elif os.path.isfile(os.path.join(exp, "next.json")) or read_args(case)[0] == "next":
        with open(os.path.join(exp, "next.json"), "wb") as f:
            f.write(jcs({"candidates": out["next"], "warnings": warnings}) + b"\n")
    elif os.path.isfile(os.path.join(exp, "states.json")) or read_args(case)[0] in ("states", "claim"):
        with open(os.path.join(exp, "states.json"), "wb") as f:
            f.write(jcs({"states": out["states"], "warnings": warnings}) + b"\n")
    else:
        # after/**: every document of input/, and every journal byte-identical to its input
        tree = out["tree"]
        shutil.rmtree(os.path.join(exp, "after"), ignore_errors=True)
        for rel in sorted(input_files(case)):
            if rel.endswith(".md"):
                dst = os.path.join(exp, "after", rel)
                os.makedirs(os.path.dirname(dst), exist_ok=True)
                shutil.copyfile(os.path.join(tree, rel), dst)
        jdir = os.path.join(case, "journal")
        if os.path.isdir(jdir):
            for name_ in sorted(os.listdir(jdir)):
                if name_.endswith(".jsonl") and not name_.endswith(".committed.jsonl"):
                    src = evidence_journal(tree, name_[:-6])
                    if open(src, "rb").read() == open(os.path.join(jdir, name_), "rb").read():
                        rel = os.path.relpath(src, tree)
                        dst = os.path.join(exp, "after", rel)
                        os.makedirs(os.path.dirname(dst), exist_ok=True)
                        shutil.copyfile(src, dst)
    print(f"blessed {name}")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--reader", choices=("war", "python"))
    ap.add_argument("--war", default=os.environ.get("WAR", os.path.join(REPO, "target/debug/war")))
    ap.add_argument("--known", help="file of case names (one per line, # comments) expected to fail")
    ap.add_argument("--bless", action="store_true")
    ap.add_argument("-v", "--verbose", action="store_true")
    ap.add_argument("case", nargs="*")
    a = ap.parse_args()
    selected = cases(a.case)
    if not selected:
        print("no cases", file=sys.stderr)
        return 2
    if a.bless:
        for case in selected:
            with tempfile.TemporaryDirectory(prefix="ow-format-corpus-") as work:
                out = python_reader(case, work)
                bless(case, out)
        return 0
    if not a.reader:
        ap.error("--reader is required")
    # 13.1: every rule of 13.2 has at least one case.
    covered = set()
    for case in cases([]):
        exp = os.path.join(case, "expected")
        for name in ("refusal", "unknown"):
            if os.path.isfile(os.path.join(exp, name)):
                covered.add(open(os.path.join(exp, name)).read().split()[0])
        for name in ("model.json", "states.json", "next.json"):
            if os.path.isfile(os.path.join(exp, name)):
                covered |= {w.split()[0] for w in json.load(open(os.path.join(exp, name)))["warnings"]}
    uncovered = sorted(SPEC_RULES - covered - UNCOVERABLE)
    for rule in uncovered:
        print(f"FAIL  coverage: no case expects {rule}")
    known = set()
    if a.known:
        with open(a.known) as f:
            known = {ln.split("#")[0].strip() for ln in f} - {""}
    passed = failed = expected_fail = skipped = 0
    with tempfile.TemporaryDirectory(prefix="ow-format-corpus-") as work:
        war = None
        if a.reader == "war":
            if not os.access(a.war, os.X_OK):
                print(f"SETUP: no war binary at {a.war}", file=sys.stderr)
                return 2
            try:
                war = War(a.war, work)
            except RuntimeError as e:
                print(f"SETUP: {e}", file=sys.stderr)
                return 2
        for i, case in enumerate(selected):
            name = os.path.basename(case)
            cw = os.path.join(work, f"c{i}")
            os.makedirs(cw)
            try:
                out = war.read(case, cw) if war else python_reader(case, cw)
                ok, detail = verdict(case, out)
            except (RuntimeError, OSError, subprocess.TimeoutExpired) as e:
                ok, detail = False, f"error: {e}"
            shutil.rmtree(cw, ignore_errors=True)
            if name in known:
                if ok:
                    failed += 1
                    print(f"FAIL  {name}: listed as a known divergence, but passes; remove it from {a.known}")
                else:
                    expected_fail += 1
                    if a.verbose:
                        print(f"known {name}: {detail}")
                continue
            if ok:
                passed += 1
                if a.verbose:
                    print(f"ok    {name}")
            else:
                failed += 1
                print(f"FAIL  {name}: {detail}")
    print(f"{a.reader}: {passed} passed, {failed} failed, {expected_fail} known, {skipped} skipped")
    return 1 if failed or uncovered else 0


if __name__ == "__main__":
    sys.exit(main())
