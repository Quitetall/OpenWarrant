#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Current authorization and source-holder refusals. No signing or execution."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

source, binary, scratch = map(Path, sys.argv[1:])
fixture = source / "conformance/fixtures/inbox/repository"

def run(root, *args):
    result = subprocess.run([str(binary), "--root", str(root), *args, "--json"], env={**os.environ, "OPENWARRANT_NO_PROJECTS": "1"}, capture_output=True, timeout=30)
    return result.returncode, json.loads(result.stdout)

def refused(root, rule, severity):
    directory = root / "docs/warrants/IX-WAR-0003"
    journal = directory / "journal.jsonl"
    before = journal.read_bytes() if journal.exists() else None
    auth = (directory / "authorization.toml").read_bytes()
    packet, context = root / "dispatch.json", root / "context.json"
    code, report = run(root, "dispatch", "IX-WAR-0003", "STAGE-001", "--emit", str(packet), "--emit-context", str(context))
    assert code != 0, report
    assert any(d["rule"] == rule and d["severity"] == severity for d in report["diagnostics"]), report
    assert not packet.exists() and not context.exists()
    assert (journal.read_bytes() if journal.exists() else None) == before
    assert (directory / "authorization.toml").read_bytes() == auth
    return packet, context, auth

root = scratch / "unchanged"
shutil.copytree(fixture, root)
refused(root, "dispatch.required-holder-unestablished", "unknown")

root = scratch / "changed"
shutil.copytree(fixture, root)
graph = root / "docs/warrants/IX-WAR-0003/atoms/45-milestones.yaml"
graph.write_text(graph.read_text().replace('executor_kind: "agent"', 'executor_kind: "agent"\n    executor_ref: "agent://fixture"'))
packet, context, auth = refused(root, "dispatch.stale-authorization", "error")
code, report = run(root, "dispatch", "IX-WAR-0003", "STAGE-001", "--prototype", "--emit", str(packet), "--emit-context", str(context))
assert code == 0 and packet.exists(), report
assert any(d["rule"] == "dispatch.prototype" for d in report["diagnostics"])
selected = json.loads(context.read_bytes())
assert all(not i["holder"]["commit_sha"] for i in selected["included"] if i["holder"]["kind"] == "git")
assert (graph.parent.parent / "authorization.toml").read_bytes() == auth
