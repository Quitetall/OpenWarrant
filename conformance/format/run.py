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


def tracking_tree(case, work):
    """input/ as a git working tree; journal/ at the evidence directories; claims/ in the store."""
    tree = os.path.join(work, "repo")
    shutil.copytree(os.path.join(case, "input"), tree)
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

    @staticmethod
    def _run(argv, cwd):
        p = subprocess.run(argv, cwd=cwd, env=ENV, capture_output=True, text=True,
                           stdin=subprocess.DEVNULL, timeout=120)
        if p.returncode != 0:
            raise RuntimeError(f"{argv[0]} {argv[1]}: exit {p.returncode}: {p.stderr.strip()[-400:]}")
        return p

    def read(self, case, work):
        kind = case_kind(case)
        if kind == "tracking" and read_args(case)[:1] == ["evaluate"]:
            return war_evaluate(self.war, case, work)
        if kind in ("tracking", "apply"):
            # No adapter maps a tracking or apply case to war's commands yet;
            # war's results for these cases are listed in known-war.txt.
            raise RuntimeError(f"no war adapter for {kind} cases")
        tree = os.path.join(work, "repo")
        shutil.copytree(self.template, tree, symlinks=True)
        shutil.copytree(os.path.join(case, "input"), tree, dirs_exist_ok=True)
        files = input_files(case)
        argv_file = os.path.join(case, "args")
        if os.path.isfile(argv_file):
            args = read_args(case)
            if os.path.isfile(os.path.join(case, "expected", "context.md")):
                p = subprocess.run([self.war, "context", *args, "--root", tree], cwd=tree,
                                   env=ENV, capture_output=True, text=True,
                                   stdin=subprocess.DEVNULL, timeout=120)
                return {"records": [], "relations": [], "findings": [], "output": p.stdout,
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


def war_states(war, tree, records):
    """The model's states of `records`, as 11.1's families: `{record, kind, value, provenance}`.

    war's model names a fixed state's kind `computed` with a `facet`; 11.1's
    `kind` is the family. An item's `checklist` state is the item's own; the
    model also gives an item the phase of its checklist, which 11.1 does not.
    """
    _, out, err = war_run(war, tree, ["model", "--json"])
    try:
        states = json.loads(out)["result"]["states"]
    except (json.JSONDecodeError, KeyError, TypeError):
        raise RuntimeError(f"war model: {err.strip()[-300:]}")
    family = {"open": "phase", "in_progress": "phase", "done": "phase", "draft": "phase",
              "authorized": "phase", "resolved": "phase", "failing": "check",
              "superseded": "currency", "accepted": "acceptance",
              "partially_accepted": "acceptance", "verified": "outcome"}
    got = set()
    for st in states:
        rec = st["record"]
        if rec.split("/")[0] not in records:
            continue
        item = "/i-" in rec
        if st["kind"] == "checklist":
            if not item:
                continue
            kind = "checklist"
        elif st["kind"] in ("computed", "authenticated"):
            kind = family.get(st["value"])
            if kind is None or (item and kind == "phase"):
                continue
        else:
            continue
        got.add((rec, kind, st["value"], st["provenance"]))
    return [{"record": r, "kind": k, "value": v, "provenance": p}
            for r, k, v, p in sorted(got)]


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
            "states": war_states(war, tree, records), "tree": tree}


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
