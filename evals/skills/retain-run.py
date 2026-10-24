#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Retain actual fixture observations, then reject unexpected or missing cases."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys
import tarfile

result = Path(sys.argv[1])
allowed_parent = Path(sys.argv[2]).resolve()
data = json.loads(result.read_text())
assert data["schema"] == "oh.war/eval-result/v1"
output = result.parent / "skills-scratch"
output.mkdir(exist_ok=True)
manifest = []
for task in data["tasks"]:
    name = task["id"]
    assert re.fullmatch(r"[a-z0-9-]+", name), name
    kept = [note.removeprefix("scratch kept at ") for note in task["notes"]
            if note.startswith("scratch kept at ")]
    assert len(kept) == 1, name
    source = Path(kept[0]).resolve()
    assert source.parent == allowed_parent and source.name.startswith("war-eval-")
    archive = output / f"{name}.tar.gz"
    # Do not follow a scratch link into the build tree or the checkout.
    with tarfile.open(archive, "w:gz", dereference=False) as tar:
        tar.add(source, arcname=source.name)
    manifest.append(dict(task=name, source=str(source), archive=archive.name,
                         sha256=hashlib.sha256(archive.read_bytes()).hexdigest()))

binary = Path("target/debug/war")
identity = dict(source_commit=subprocess.check_output(
    ["git", "rev-parse", "HEAD"], text=True).strip(),
    binary_version=subprocess.check_output([str(binary.resolve()), "--version"], text=True).strip(),
    binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),
    archives=manifest, fixture_only=True, qualification_granted=False)
(output / "identity.json").write_text(json.dumps(identity, indent=2) + "\n")
expected = {"grill": "would_satisfy", "spec": "would_satisfy", "tickets": "partial",
            "grill-unanswered": "needs_input", "grill-12k-budget": "over_budget"}
assert len(data["tasks"]) == len(expected)
assert {task["id"] for task in data["tasks"]} == set(expected)
for task in data["tasks"]:
    assert task["score"] == expected[task["id"]] and task["as_expected"] is True, task
