#!/usr/bin/env python3
"""Local CLI source-retention controls; no native provider execution or qualification."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

source, binary, scratch = map(Path, sys.argv[1:])
root = scratch / "repo"
shutil.copytree(source / "conformance/fixtures/inbox/repository", root)
directory = root / "docs/warrants/IX-WAR-0003"
graph = directory / "atoms/45-milestones.yaml"
graph.write_text(graph.read_text().replace('executor_kind: "agent"', 'executor_kind: "katana"\n    executor_ref: "synthetic"'))


def run(*args, ok=True):
    result = subprocess.run([str(binary), "--root", str(root), *args, "--json"],
                            env={**os.environ, "OPENWARRANT_NO_PROJECTS": "1"}, capture_output=True, timeout=30)
    answer = json.loads(result.stdout)
    assert (result.returncode == 0) == ok, (args, answer)
    return answer


pending = root / "dispatch.pending.json"
run("dispatch", "IX-WAR-0003", "STAGE-001", "--prototype", "--emit", str(pending))
dispatch = json.loads(pending.read_bytes())
(directory / "dispatches").mkdir()
pending.rename(directory / "dispatches" / (dispatch["dispatch_id"] + ".json"))
(root / "receipt.bin").write_bytes(b"synthetic receipt fixture\x00\xff")
request = {
    "schema": "oh.war/runtime-capture-request/v1-draft.1", "dispatch_id": dispatch["dispatch_id"],
    "receipt": "receipt.bin", "provider": {"kind": "katana", "identity": "synthetic", "version": "synthetic/v1"},
    "metadata": {"observation_id": "synthetic-local-capture", "observed_at": "2026-10-09T12:00:00Z",
                 "original_receipt_ref": "fixture://synthetic-receipt", "binary_identity": None,
                 "source_identity": None, "transport": "fixture-file", "argv": [], "exit_code": 0,
                 "status": "synthetic-byte-fixture"},
}
(root / "request.json").write_text(json.dumps(request))
first = run("runtime", "import", "IX-WAR-0003", "--request", "request.json")["result"]
assert first["retained"] and not first["assurance_granted"]
assert first["native_observation"]["standing"] == "unknown"
assert first["native_observation"]["code"] == "runtime.verifier-unavailable"
retained = root / first["reference"]
original = retained.read_bytes()
assert run("runtime", "import", "IX-WAR-0003", "--request", "request.json")["result"]["digest"] == first["digest"]
assert retained.read_bytes() == original
selection = {"schema": "oh.war/runtime-selection-request/v1-draft.1",
             "selections": [{"stage_id": "STAGE-001", "capture_digest": first["digest"]}]}
(root / "selection.json").write_text(json.dumps(selection))
unknown = run("runtime", "assess", "IX-WAR-0003", "--selection", "selection.json", ok=False)
assert unknown["diagnostics"][0]["severity"] == "unknown"
assert unknown["result"]["stages"][0]["receipt"]["code"] == "runtime.verifier-unavailable"
assert not unknown["result"]["saved_native_observations_used"]
assert not unknown["result"]["assurance_granted"]
assert retained.read_bytes() == original
intent = directory / "atoms/10-intent.md"
before = intent.read_bytes()
intent.write_bytes(before + b"\nchanged current contract\n")
stale = run("runtime", "import", "IX-WAR-0003", "--request", "request.json", ok=False)
assert stale["diagnostics"][0]["rule"] == "runtime-basis.dispatch-mismatch"
assert retained.read_bytes() == original
intent.write_bytes(before)
retained.write_bytes(b"conflicting occupant")
run("runtime", "import", "IX-WAR-0003", "--request", "request.json", ok=False)
assert retained.read_bytes() == b"conflicting occupant"
altered = run("runtime", "show", "IX-WAR-0003", first["digest"], ok=False)
assert altered["diagnostics"][0]["rule"] == "runtime.capture-altered"
retained.write_bytes(original)  # Restore only this planted fixture, never an authored record.
run("dispatch", "IX-WAR-0003", "STAGE-001", "--prototype", "--emit", str(pending))
next_dispatch = json.loads(pending.read_bytes())
assert next_dispatch["attempt_id"] != dispatch["attempt_id"]
pending.rename(directory / "dispatches" / (next_dispatch["dispatch_id"] + ".json"))
stale = run("runtime", "assess", "IX-WAR-0003", "--selection", "selection.json", ok=False)
assert stale["diagnostics"][0]["rule"] == "runtime.selection-superseded-attempt"
assert retained.read_bytes() == original
(root / "receipt.bin").unlink()
shutil.rmtree(directory / "dispatches")
shown = run("runtime", "show", "IX-WAR-0003", first["digest"])["result"]
assert not shown["native_observation_is_trusted"] and not shown["assurance_granted"]
assert shown["record"]["observation"]["result_digest"] is None
assert len(list((directory / "runtime-receipts").iterdir())) == 1
