#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Validate the portable native draft pack, including observed refusals.

This checks structure and retained bytes, never signatures or assurance.
"""
import copy
from datetime import datetime
import hashlib
import json
from pathlib import Path
import sys
import tomllib
import re
import shutil
import subprocess
import tempfile
import os

from jsonschema import Draft202012Validator, FormatChecker


FORMATS = FormatChecker()


@FORMATS.checks("date-time", raises=ValueError)
def rfc3339(value):
    # jsonschema's date-time extra is optional. Use an explicit stdlib calendar
    # assertion so a missing extra cannot silently turn this control into pass.
    if not isinstance(value, str):
        return True
    if not re.fullmatch(r"[0-9]{4}-[0-9]{2}-[0-9]{2}[Tt][0-9]{2}:[0-9]{2}:[0-9]{2}(\.[0-9]+)?([Zz]|[+-][0-9]{2}:[0-9]{2})", value):
        return False
    parsed = datetime.fromisoformat(value.replace("t", "T").replace("z", "Z"))
    return parsed.tzinfo is not None


def check(root):
    manifest = json.loads((root / "pack.json").read_text())
    assert manifest["schema"] == "oh.war/native-schema-pack/v1-draft.1"
    assert manifest["experimental"] is True
    assert manifest["runtime_validation_required"] is True
    files = manifest["files"]
    lines = []
    for name, digest in sorted(files.items()):
        path = Path(name)
        assert not path.is_absolute() and ".." not in path.parts
        actual = hashlib.sha256((root / name).read_bytes()).hexdigest()
        assert actual == digest, name
        lines.append(f"{name}:{digest}\n")
    assert hashlib.sha256("".join(lines).encode()).hexdigest() == manifest["transitive_digest"]

    names = {
        "response": "oh.war/native-resolution-response/v2-draft.1",
        "resolution": "oh.war/resolution/v2-draft.1",
        "signing-result": "oh.war/native-signing-result/v1-draft.1",
        "signed-proposal": "oh.war/native-signed-proposal/v1-draft.1",
        "policy": "oh.war/runtime-native-policy-binding/v1-draft.1",
        "selection-request": "oh.war/runtime-selection-request/v1-draft.1",
    }
    validators, examples = {}, {}
    for name, identity in names.items():
        schema = json.loads((root / (name + ".json")).read_text())
        assert schema["$id"] == identity
        Draft202012Validator.check_schema(schema)
        validators[name] = Draft202012Validator(schema, format_checker=FORMATS)
        examples[name] = json.loads((root / "examples" / (name + ".json")).read_text())
        validators[name].validate(examples[name])
        assert (root / "typescript" / (name + ".ts")).is_file()
    wire = (root / "examples" / "response.toml").read_text()
    assert tomllib.loads(wire) == examples["response"]
    assert examples["resolution"]["native_response"] == wire
    assert examples["signed-proposal"]["response"] == wire
    assert examples["signing-result"]["response"] == examples["response"]
    assert examples["signing-result"]["recorded"] is False
    assert examples["resolution"]["native_signature"] == "UNSIGNED EXAMPLE; NOT A SIGNATURE"

    refused = 0

    def reject(name, mutate):
        nonlocal refused
        value = copy.deepcopy(examples[name])
        mutate(value)
        assert list(validators[name].iter_errors(value)), (name, value)
        refused += 1

    reject("response", lambda v: v.update(schema="oh.war/native-resolution-response/future"))
    reject("response", lambda v: v.update(acting_role="performer"))
    reject("response", lambda v: v.update(meaning=" \n\t"))
    reject("response", lambda v: v.update(extra="not declared"))
    reject("response", lambda v: v["locator"].update(extra=True))
    reject("response", lambda v: v["locator"].update(worktree_clean=False))
    reject("response", lambda v: v["locator"].update(paths_dirty=["changed.rs"]))
    reject("response", lambda v: v["locator"].update(commit_sha="A" * 40))
    reject("response", lambda v: v.update(resolution_id="not-a-uuid"))
    reject("response", lambda v: v.update(effective_time="yesterday"))
    reject("response", lambda v: v.update(effective_time="2026-02-30T00:00:00Z"))
    reject("response", lambda v: v["policy"].update(schema="future"))
    reject("response", lambda v: v["policy"].update(extra=True))
    reject("response", lambda v: v["selections"][0].update(extra=True))
    reject("response", lambda v: v.update(selections=v["selections"] * 257))
    reject("response", lambda v: v.pop("policy"))
    reject("resolution", lambda v: v.pop("native_signature"))
    reject("resolution", lambda v: v.pop("native_response"))
    reject("resolution", lambda v: v.pop("locator"))
    reject("resolution", lambda v: v.update(schema="oh.war/resolution/v1"))
    reject("signed-proposal", lambda v: v.pop("signature"))
    reject("signing-result", lambda v: v.update(recorded="false"))
    reject("policy", lambda v: v.update(schema="future"))
    reject("selection-request", lambda v: v.update(selections=v["selections"] * 257))
    # Correct structure permits a deliberately unsigned illustration. This is
    # the observed boundary: schema success cannot establish cryptography.
    validators["signed-proposal"].validate(examples["signed-proposal"])
    print(f"PASS: six examples, exact TOML delivery, manifest hashes, {refused} refusals; no assurance evaluated")


def drift(war, source):
    with tempfile.TemporaryDirectory(prefix="openwarrant-native-schema-", dir=os.environ.get("TMPDIR")) as name:
        root = Path(name)
        shutil.copyfile(source / "openwarrant.toml", root / "openwarrant.toml")
        if (source / "profiles").is_dir():
            shutil.copytree(source / "profiles", root / "profiles")

        def run(checking):
            args = [war, "--root", str(root), "admin", "schemas", "--json"]
            if checking:
                args.append("--check")
            result = subprocess.run(args, text=True, capture_output=True, timeout=30)
            return result.returncode, json.loads(result.stdout)

        code, report = run(False)
        assert code == 0, report
        # Compare against the committed active publication, not a hash computed
        # from the same candidate schema generator.
        active = json.loads((source / "schemas/pack.json").read_text())
        assert (root / "schemas/pack.json").read_bytes() == (source / "schemas/pack.json").read_bytes()
        for record in active["files"]:
            path = "schemas/oh.war/" + record + "/v1.json"
            assert (root / path).read_bytes() == (source / path).read_bytes(), path
        for p in (source / "schemas/typescript").iterdir():
            if p.is_file():
                assert (root / "schemas/typescript" / p.name).read_bytes() == p.read_bytes(), p.name
        first = {str(p.relative_to(root)): p.read_bytes() for p in (root / "schemas/native-draft").rglob("*") if p.is_file()}
        code, report = run(False)
        assert code == 0, report
        second = {str(p.relative_to(root)): p.read_bytes() for p in (root / "schemas/native-draft").rglob("*") if p.is_file()}
        assert first == second
        code, report = run(True)
        assert code == 0, report
        for path in ["response.json", "typescript/response.ts", "examples/response.toml", "pack.json"]:
            relative = "schemas/native-draft/" + path
            p = root / relative
            original = p.read_bytes()
            p.write_bytes(original + b"\nchanged fixture\n")
            code, report = run(True)
            assert code == 2 and any(d["rule"] == "schemas.drift" and d["file"] == relative for d in report["diagnostics"]), report
            p.write_bytes(original)
        p = root / "schemas/native-draft/policy.json"
        original = p.read_bytes()
        p.unlink()
        code, report = run(True)
        assert code == 2 and any(d["rule"] == "schemas.missing" and d["file"] == "schemas/native-draft/policy.json" for d in report["diagnostics"]), report
        p.write_bytes(original)
        code, report = run(True)
        assert code == 0, report
        print("PASS: unchanged active v1 bytes; deterministic native pack; schema, TypeScript, example and manifest drift; missing native schema; fixture recovery")


if __name__ == "__main__":
    check(Path(sys.argv[1]))
    if len(sys.argv) > 2:
        drift(sys.argv[2], Path(__file__).resolve().parents[1])
