#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Run the Format 1.0 conformance corpus (conformance/format/) against a reader.

    run.py --reader war    [--war PATH] [--known FILE] [CASE...]
    run.py --reader python [CASE...]
    run.py --bless CASE...     (write expected/model.json from the python reader;
                                review it against the specification before committing)

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
    "context.over-budget",
}
UNKNOWNS = {"format.newer-major", "format.newer-minor", "format.requires-unknown"}
WARNINGS = {"format.target-unknown", "format.refusal-missing", "format.item-id-missing",
            "format.legacy-obligation"}
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

def python_reader(case, work):
    tree = os.path.join(work, "tree")
    shutil.copytree(os.path.join(case, "input"), tree)
    p = subprocess.run([sys.executable, "-I", os.path.join(HERE, "reader.py"), tree],
                       capture_output=True, text=True, timeout=60)
    if p.returncode != 0:
        raise RuntimeError(p.stderr.strip()[-400:])
    return json.loads(p.stdout)


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
        tree = os.path.join(work, "tree")
        shutil.copytree(self.template, tree, symlinks=True)
        shutil.copytree(os.path.join(case, "input"), tree, dirs_exist_ok=True)
        files = input_files(case)
        argv_file = os.path.join(case, "args")
        if os.path.isfile(argv_file):
            with open(argv_file) as f:
                args = f.read().split()
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
        doc_of = {r["id"]: r.get("document") for r in recs}
        rels = []
        for r in model["relations"]:
            if r["from"] not in doc_of:
                continue
            # Containment is carried by the record's `document`; a reader that
            # also emits it as `part_of <document>` says nothing more.
            if r["kind"] == "part_of" and r["to"] == doc_of[r["from"]]:
                continue
            rels.append({k: r[k] for k in ("from", "kind", "to", "to_revision") if k in r})
        findings = {f"{d['rule']} {d['record']}" for d in model["diagnostics"]
                    if d.get("rule") in SPEC_RULES}
        # `war check` is the checker over the same reading: it adds the rules
        # the model does not carry (format.target-unknown).
        p = subprocess.run([self.war, "--json", "check", "--root", tree], cwd=tree, env=ENV,
                           capture_output=True, text=True, stdin=subprocess.DEVNULL, timeout=120)
        try:
            check = json.loads(p.stdout)
        except json.JSONDecodeError:
            raise RuntimeError(f"war check: exit {p.returncode}: {p.stderr.strip()[-400:]}")
        findings |= {f"{d['rule']} {d['file']}" for d in check.get("diagnostics", [])
                     if d.get("rule") in SPEC_RULES and d.get("file")}
        findings = sorted(findings)
        return {
            "records": [{k: r.get(k) for k in ("id", "type", "title", "fields", "ext",
                                               "content_digest")} for r in recs],
            "relations": rels, "findings": findings,
        }


# ---------------------------------------------------------------- compare --

def normalize(out):
    recs = sorted(out["records"], key=lambda r: r["id"].encode())
    for r in recs:
        if r.get("title") is None:
            r.pop("title", None)
    rels = sorted(out["relations"],
                  key=lambda r: (r["from"], r["kind"], r["to"], r.get("to_revision", "")))
    return recs, rels, out["findings"]


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
    with open(os.path.join(exp, "model.json"), "rb") as f:
        want_bytes = f.read()
    got = {"records": recs, "relations": rels, "warnings": warnings}
    got_bytes = jcs(got)
    if refusals or unknowns:
        return False, f"refused or UNKNOWN: {refusals + unknowns}"
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
            recs, rels, findings = normalize(out)
            warnings = [f for f in findings if f.split(" ")[0] in WARNINGS]
            bad = [f for f in findings if f.split(" ")[0] not in WARNINGS]
            if bad:
                print(f"{os.path.basename(case)}: not an accept case: {bad}", file=sys.stderr)
                continue
            os.makedirs(os.path.join(case, "expected"), exist_ok=True)
            with open(os.path.join(case, "expected", "model.json"), "wb") as f:
                f.write(jcs({"records": recs, "relations": rels, "warnings": warnings}) + b"\n")
            print(f"blessed {os.path.basename(case)}")
        return 0
    if not a.reader:
        ap.error("--reader is required")
    # 11.1: every rule of 11.2 has at least one case.
    covered = set()
    for case in cases([]):
        exp = os.path.join(case, "expected")
        for name in ("refusal", "unknown"):
            if os.path.isfile(os.path.join(exp, name)):
                covered.add(open(os.path.join(exp, name)).read().split()[0])
        if os.path.isfile(os.path.join(exp, "model.json")):
            covered |= {w.split()[0] for w in json.load(open(os.path.join(exp, "model.json")))["warnings"]}
    uncovered = sorted(SPEC_RULES - covered)
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
            if os.path.isfile(os.path.join(case, "args")) and a.reader == "python":
                skipped += 1
                if a.verbose:
                    print(f"skip  {name}: a context case; this reader is not a context compiler")
                continue
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
