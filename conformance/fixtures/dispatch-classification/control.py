#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Actual offline CLI disclosure controls; no provider authority is inferred."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

source, binary, scratch = map(Path, sys.argv[1:])
root = scratch / "repo"
shutil.copytree(source / "conformance/fixtures/inbox/repository", root)
dir = root / "docs/warrants/IX-WAR-0003/atoms"
graph = dir / "45-milestones.yaml"
graph.write_text(graph.read_text().replace('executor_kind: "agent"', 'executor_kind: "agent"\n    executor_ref: "agent://fixture"'))
p = dir / "10-intent.md"
p.write_text(p.read_text().replace('classification: internal', 'classification: customer-X'))

def run(*args, ok=True):
    result = subprocess.run([str(binary), "--root", str(root), *args, "--json"], env={**os.environ, "OPENWARRANT_NO_PROJECTS": "1"}, capture_output=True, timeout=30)
    answer = json.loads(result.stdout)
    assert (result.returncode == 0) == ok, (args, answer, result.stderr.decode())
    return answer

packet, context, bundle = [scratch / name for name in ("dispatch.json", "context.json", "bundle.json")]
run("dispatch", "IX-WAR-0003", "STAGE-001", "--prototype", "--emit", str(packet), "--emit-context", str(context))
selected = json.loads(context.read_bytes())
assert next(i for i in selected["included"] if i["id"] == "atoms/10-intent.md")["classification"] == "customer-X"
assert selected["effective_classification"] == ""
assert any("classification-policy" in x for x in selected["unresolved"])
digest = run("dispatch-bundle", "create", "IX-WAR-0003", "--dispatch", str(packet), "--context", str(context), "--emit", str(bundle))["result"]["bundle_digest"]
shutil.rmtree(root)
args = ["dispatch-bundle", "check", str(bundle), "--expected-digest", digest, "--read", "atoms/10-intent.md"]
answer = run(*args, "--allow-classification", "customer-X")
assert answer["result"]["source_classification_checked"] is True
assert answer["result"]["execution_authorized"] is False
assert "Adopt OpenWarrant" in answer["result"]["content"]
for flags in [("--check-classification",), ("--allow-classification", "internal"), ("--allow-classification", "*"), ("--allow-classification", "Customer-X")]:
    answer = run(*args, *flags, ok=False)
    assert "classification-denied" in str(answer)
args[-1] = "atoms/45-milestones.yaml"
answer = run(*args, "--allow-classification", "internal", ok=False)
assert "classification-unestablished" in str(answer)
assert answer["counts"]["unknown"] == 1 and answer["counts"]["error"] == 0
assert answer["result"]["content"] is None
assert answer["result"]["source_classification_status"] == "unknown"
