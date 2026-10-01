# SPDX-License-Identifier: Apache-2.0
"""Plant mismatches at the real dispatch/submit CLI boundary."""
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile


def main():
    war = str(Path(sys.argv[1]).resolve())
    source = Path(__file__).resolve().parents[1] / "fixtures/inbox/repository"
    env = {**os.environ, "OPENWARRANT_NO_UPDATE_CHECK": "1", "OPENWARRANT_NO_PROJECTS": "1"}
    with tempfile.TemporaryDirectory(prefix="ow-submit-bindings-") as temporary:
        root = Path(temporary) / "repo"
        shutil.copytree(source, root)
        directory = root / "docs/warrants/IX-WAR-0003"
        graph = directory / "atoms/45-milestones.yaml"
        graph.write_text(graph.read_text().replace(
            'executor_kind: "agent"', 'executor_kind: "agent"\n    executor_ref: "agent://fixture"'))

        def run(*args):
            return subprocess.run([war, *args], cwd=root, env=env, capture_output=True)

        result = run("dispatch", "IX-WAR-0003", "STAGE-001", "--prototype", "--emit", "packet.json")
        assert result.returncode == 0, result.stdout.decode()
        packet = json.loads((root / "packet.json").read_bytes())
        good = {key: packet[key] for key in ("dispatch_id", "contract_digest", "attempt_id", "stage_id")}
        good["requested_next_action"] = "verify"
        journal = directory / "journal.jsonl"
        recorded = directory / "submissions" / (good["dispatch_id"] + ".json")

        def submit(value):
            (root / "submission.json").write_text(json.dumps(value))
            result = run("submit", "IX-WAR-0003", "submission.json", "--json")
            return result, json.loads(result.stdout)

        for field, wrong in [("contract_digest", "sha256:" + "f" * 64),
                             ("stage_id", "STAGE-002"), ("attempt_id", "unrelated-attempt")]:
            before = journal.read_bytes()
            result, report = submit({**good, field: wrong})
            assert result.returncode != 0, (field, report)
            assert any(d["rule"] == "submission.dispatch-mismatch" for d in report["diagnostics"]), report
            assert not recorded.exists(), field
            assert journal.read_bytes() == before, field

        # Historical fixture metadata is deliberately absent. The CLI must
        # preserve that fixture's bytes and say UNKNOWN, never infer a binding.
        original = journal.read_bytes()
        events = [json.loads(line) for line in original.splitlines()]
        for field in ("contract_digest", "attempt_id", "stage"):
            legacy = json.loads(json.dumps(events))
            for event in legacy:
                if event["type"] == "dispatch.compiled":
                    payload = json.loads(event["payload"])
                    payload.pop(field)
                    event["payload"] = json.dumps(payload)
            journal.write_text("".join(json.dumps(e) + "\n" for e in legacy))
            before = journal.read_bytes()
            result, report = submit(good)
            assert result.returncode != 0, report
            assert any(d["rule"] == "submission.dispatch-unbound" and d["severity"] == "unknown"
                       for d in report["diagnostics"]), report
            assert not recorded.exists()
            assert journal.read_bytes() == before
        journal.write_bytes(original)
        result, report = submit(good)
        assert result.returncode == 0, report
        stored = recorded.read_bytes()
        before = journal.read_bytes()
        result, report = submit(good)
        assert result.returncode == 0, report
        assert any(d["rule"] == "submission.replayed" for d in report["diagnostics"]), report
        assert recorded.read_bytes() == stored
        assert journal.read_bytes() == before
        print("3 mismatches refused without writes; 3 missing bindings UNKNOWN; exact submission accepted and replayed without writes")


if __name__ == "__main__":
    main()
