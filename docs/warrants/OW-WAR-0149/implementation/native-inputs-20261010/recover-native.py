#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Owned CPU-fixture observation. Never executes archived producer code or grants trust."""
import base64, hashlib, json, os, pathlib, shutil, subprocess, sys
cli, source, out, verifier = map(pathlib.Path, sys.argv[1:])
out.mkdir(parents=True, exist_ok=False)
budget = ["--max-content-bytes", "134217728", "--max-archive-bytes", "268435456"]
def run(args, name, cwd, expect=True):
    r = subprocess.run(list(map(str, args)), cwd=cwd, capture_output=True, timeout=180)
    (out / (name + ".stdout")).write_bytes(r.stdout)
    (out / (name + ".stderr")).write_bytes(r.stderr)
    if expect and r.returncode: raise RuntimeError(name + ": " + r.stderr.decode(errors="replace")[-1200:])
    return r
archive = out / "native-complete.archive.json"
r = run([cli, "--root", source, "archive", "export", "ND-WAR-0001", archive, "--history", "--json", *budget], "export", source)
assert json.loads(r.stdout)["result"]["complete"] is True
# Preserve the true pre-change gap; discard only the owned, superseded duplicate archive.
for name in ["context-red.json", "pre-context-cli.sha256", "retention-result.json"]:
    shutil.copy2(source / name, out / name)
shutil.rmtree(source)
assert not source.exists()
empty = out / "empty"; empty.mkdir(mode=0o700)
imported = out / "imported"
run([cli, "archive", "import", archive, imported, "--json", *budget], "import", empty)
reexport = out / "reexport.archive.json"
run([cli, "archive", "reexport", imported, reexport, "--json", *budget], "reexport", empty)
assert archive.read_bytes() == reexport.read_bytes()
r = run([cli, "archive", "runtime-basis", archive, "--json", *budget], "query", empty)
query = json.loads(r.stdout)["result"]
assert not query["provider_capture_inventory"]["unresolved"]
assert all(x["native_inputs_reconnected"] and not x["assurance_granted"] for x in query["provider_capture_inventory"]["records"])
records = imported / "records/docs/warrants/ND-WAR-0001"
manifest_path = next((records / "native-inputs").glob("inputs-*.json"))
manifest = json.loads(manifest_path.read_bytes())
workspace = out / "native-data"; workspace.mkdir(mode=0o700)
inputs = {}
for role, value in manifest["inputs"].items():
    blob = manifest_path.parent / value["blob"]
    data = blob.read_bytes()
    assert "sha256:" + hashlib.sha256(data).hexdigest() == value["digest"]
    assert os.stat(blob).st_mode & 0o777 == 0o600
    parts = pathlib.PurePosixPath(role).parts
    assert parts and all(x not in (".", "..") for x in parts)
    target = workspace.joinpath(*parts)
    target.parent.mkdir(parents=True, exist_ok=True, mode=0o700)
    target.write_bytes(data)
    # Explicit fixture replay policy, never import behavior. Producer remains inert.
    os.chmod(target, value["mode"] & 0o777 if role.startswith("job/") else 0o600)
    inputs[role] = target
capture = json.loads(next((records / "runtime-receipts").glob("capture-*.json")).read_bytes())
receipt = workspace / "receipt.json"
receipt.write_bytes(base64.b64decode(capture["receipt"]["base64"]))
# This is a public fixture key, not an activated authority root.
expected_key = pathlib.Path("/home/brianklam/Projects/OpenWarrant/docs/runtime-evidence/native-dispatch-retention-20261010/source/native-run/public-key.bin").read_bytes()
assert inputs["public_key"].read_bytes() == expected_key
argv = [verifier, "openwarrant", "verify", "--receipt", receipt, "--public-key", inputs["public_key"], "--binding", inputs["binding"], "--plan", inputs["plan"], "--job", workspace / "job", "--producer-executable", inputs["producer"]]
r = run(argv, "native-verify", empty)
answer = json.loads(r.stdout)
assert answer["status"] == "validated" and answer["assurance"] == "not-established"
original = receipt.read_bytes(); broken = json.loads(original); broken["signature"][0] ^= 1
receipt.write_text(json.dumps(broken))
r = run(argv, "changed-signature", empty, False)
assert json.loads(r.stdout)["status"] == "rejected"
receipt.write_bytes(original)
jobfile = inputs["job/status.jsonl"]; jobfile.write_bytes(jobfile.read_bytes() + b"changed\n")
r = run(argv, "changed-job", empty, False)
assert json.loads(r.stdout)["status"] == "rejected"
summary = {"source_removed": True, "archive_bytes": archive.stat().st_size, "archive_sha256": hashlib.sha256(archive.read_bytes()).hexdigest(), "byte_identical_reexport": True, "all_required_categories_retained_or_absent": True, "native_fixture_verified": True, "changed_signature_rejected": True, "changed_job_rejected": True, "import_modes_inert": True, "assurance_granted": False, "independent_qualification": False, "kf_roundtrip": "pending"}
(out / "observation.json").write_text(json.dumps(summary, indent=2) + "\n")
# Inputs can be rebuilt from the retained archive; keep only evidence and receipts.
shutil.rmtree(imported); shutil.rmtree(workspace); empty.rmdir(); reexport.unlink()
print(json.dumps(summary))
