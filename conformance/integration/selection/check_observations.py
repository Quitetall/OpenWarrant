#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Observe real compiler selection on fresh inputs, without implementing closure."""
import argparse
import copy
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import uuid

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from observation import publish_observation


def digest(raw):
    return hashlib.sha256(raw).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--war", type=Path, required=True)
    parser.add_argument("--producer", type=Path, required=True)
    parser.add_argument("--compiler-build", required=True)
    parser.add_argument("--record-dir", type=Path, required=True)
    args = parser.parse_args()
    war = args.war.resolve(strict=True)
    producer = args.producer.resolve(strict=True)
    output = args.record_dir.resolve()
    output.mkdir(parents=True, exist_ok=False)
    inputs = output / "inputs"
    inputs.mkdir()
    env = {**os.environ, "OPENWARRANT_NO_PROJECTS": "1", "OPENWARRANT_NO_UPDATE_CHECK": "1"}
    binaries = {str(p): digest(p.read_bytes()) for p in [war, producer]}
    observations = []
    nonce = uuid.uuid4().hex
    metadata = {"schema": "oh.war/document/1.0.0-rc.3", "kind": "warrant",
                "id": "example:selection-" + nonce, "revision": 1,
                "title": "Selection " + nonce, "state": "draft",
                "context": [{"id": "needed", "target": "#extra", "required": True,
                             "when": {"stage": ["implementation"], "path": ["src/**"]}}],
                "dependencies": [{"unit": "extra", "target": "#definition"}]}
    units = [{"id": unit, "kind": kind, "text": text} for unit, kind, text in [
        ("outcome", "binding", "# Selection " + nonce + "\n\nA bounded output.\n"),
        ("scope", "binding", "## Scope\n\nA local probe.\n"),
        ("context", "binding", "## Context\n\nNo authority is established.\n"),
        ("extra", "background", "## Supplement\n\nPAYLOAD_" + nonce + " é日.\n"),
        ("definition", "background", "## Definition\n\nDEF_" + nonce + ".\n"),
        ("outside", "background", "## Unselected\n\nOUTSIDE_" + nonce + ".\n")]]
    with (output / "wire.jsonl").open("xb") as wire:
        def run(label, argv, raw=None):
            p = subprocess.run(argv, input=raw, capture_output=True, env=env, timeout=60)
            wire.write((json.dumps({"label": label, "argv": argv,
                       "stdin": None if raw is None else raw.decode(),
                       "stdout": p.stdout.decode(), "stderr": p.stderr.decode(),
                       "exit_code": p.returncode}, ensure_ascii=False, separators=(",", ":")) + "\n").encode())
            wire.flush()
            return p

        def sdk(label, operation, **fields):
            request = {"schema": "oh.war/sdk-request/v1", "operation": operation, **fields}
            p = run(label, [str(war), "sdk", "--request", "-"], json.dumps(request).encode())
            value = json.loads(p.stdout)
            assert p.returncode == value["exit_code"] == 0, value
            assert value["schema"] == "oh.war/report/v1"
            return value["result"]

        def observe(label, meta, source_units, selected, truth=None, diagnostic=None, refusal=None, missing=False):
            begin = wire.tell()
            authored = sdk(label + ":author", "author", metadata=list(meta.items()), units=source_units)["source"]
            if missing:
                assert authored.count('"#extra"') == 1
                authored = authored.replace('"#extra"', '"#absent"')
            source = authored.encode()
            root = inputs / uuid.uuid4().hex
            root.mkdir()
            (root / "task.md").write_bytes(source)
            argv = [str(producer), str(root), "task.md", args.compiler_build]
            p = run(label + ":compile", argv)
            row = {"case": label, "source": str((root / "task.md").relative_to(output)),
                   "source_sha256": digest(source), "exit_code": p.returncode,
                   "refused": bool(refusal), "qualification_established": False}
            if refusal:
                assert p.returncode != 0 and not p.stdout, p.stdout
                assert any(d["code"] == refusal for d in json.loads(p.stderr))
                row["stderr"] = p.stderr.decode()
                row["required_refusal"] = refusal
            else:
                assert p.returncode == 0, p.stderr
                files = {k: bytes(v) for k, v in json.loads(p.stdout).items()}
                packet = json.loads(files["packet.json"])
                checked = sdk(label + ":integrity", "package-check",
                              files=[{"path": name, "hex": raw.hex()} for name, raw in files.items()])
                assert checked["integrity_checked"] is True
                assert checked["semantic_coverage_established"] is False
                assert packet["readiness"]["value"] == "blocked"
                assert files["blobs/" + digest(source) + ".bin"] == source
                bindings = {b["ref"]["unit"]: b for b in packet["binding_context"]}
                assert set(bindings) == set(selected), bindings
                for unit, binding in bindings.items():
                    ref = binding["ref"]
                    exact = sdk(label + ":unit:" + unit, "unit", source=authored, dialect="rc3", unit=unit)["unit"].encode()
                    assert ref["source_digest"] == "sha256:" + digest(source)
                    assert source[ref["start"]:ref["end"]] == exact
                    assert exact in files["ENTRY.md"]
                    assert binding["required"] is True
                assert ("OUTSIDE_" + nonce).encode() not in files["ENTRY.md"]
                for unit in ["extra", "definition"]:
                    if unit in bindings:
                        assert bindings[unit]["kind"] == "background"
                if truth:
                    pointer = next(p for p in packet["reference_catalog"] if p["id"] == "needed")
                    assert pointer["result"] == truth
                    assert pointer["selected"] == ("extra" in selected)
                if diagnostic:
                    assert any(d["code"] == diagnostic for d in packet["diagnostics"])
                repeated = run(label + ":repeat", argv)
                assert repeated.returncode == 0 and repeated.stdout == p.stdout
                row["package_sha256"] = digest(p.stdout)
                row["selected_units"] = sorted(bindings)
                row["repeat_identical"] = True
            row["wire_start"] = begin
            row["wire_end"] = wire.tell()
            observations.append(row)
            print("PASS", label, flush=True)
            return row

        roots = ["outcome", "scope", "context"]
        complete = roots + ["extra", "definition"]
        first = observe("required-unknown", metadata, units, complete, "UNKNOWN", "applicability-unknown")
        optional = copy.deepcopy(metadata)
        optional["context"][0]["required"] = False
        observe("optional-unknown", optional, units, roots, "UNKNOWN")
        false_scope = copy.deepcopy(metadata)
        false_scope["scope"] = {"paths": []}
        observe("known-false-wins", false_scope, units, roots, "FALSE")
        altered = copy.deepcopy(units)
        altered[3]["text"] += "Changed exact payload " + nonce + ".\n"
        revised = copy.deepcopy(metadata)
        revised["revision"] = 2
        changed = observe("changed-source", revised, altered, complete, "UNKNOWN", "applicability-unknown")
        assert first["source_sha256"] != changed["source_sha256"]
        assert first["package_sha256"] != changed["package_sha256"]
        assert digest((output / first["source"]).read_bytes()) == first["source_sha256"]
        cycle = copy.deepcopy(metadata)
        cycle["dependencies"].append({"unit": "definition", "target": "#extra"})
        observe("bounded-cycle", cycle, units, complete, "UNKNOWN", "dependency-cycle")
        observe("required-missing", metadata, units, [], refusal="target-missing", missing=True)
        conflict = copy.deepcopy(metadata)
        conflict["conflicts"] = [{"unit": "extra", "target": "#scope"}]
        observe("selected-conflict", conflict, units, [], refusal="binding-conflict")

    assert all(digest(Path(path).read_bytes()) == expected for path, expected in binaries.items())
    publish_observation(output / "summary.json", {"kind": "actual-fresh-selection-observations",
        "binary_sha256": binaries, "compiler_claim": args.compiler_build,
        "cases": observations, "wire_sha256": digest((output / "wire.jsonl").read_bytes()),
        "qualification_established": False,
        "limitations": ["One captured document per compiler-example invocation",
                        "Cross-source routing/access controls are exercised separately by native selection tests",
                        "Linux execution only; no trusted access service or human acceptance"]})


if __name__ == "__main__":
    main()
