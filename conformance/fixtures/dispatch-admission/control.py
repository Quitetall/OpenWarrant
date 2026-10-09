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

# The current signed bytes are restored after committing a different source.
# This is an observed contradiction, not an unavailable observation.
root = scratch / "mismatch"
shutil.copytree(fixture, root)
intent = root / "docs/warrants/IX-WAR-0003/atoms/10-intent.md"
original = intent.read_bytes()
intent.write_bytes(original + b"\nNot the signed source\n")
for args in [("init",), ("add", "."), ("commit", "-m", "fixture")]:
    result = subprocess.run(["git", "-c", "commit.gpgsign=false", "-c", "core.hooksPath=/dev/null", "-c", "user.name=Fixture", "-c", "user.email=fixture@example.invalid", *args], cwd=root, capture_output=True, timeout=15)
    assert result.returncode == 0, result.stderr.decode()
intent.write_bytes(original)
refused(root, "dispatch.required-holder-mismatch", "error")

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
