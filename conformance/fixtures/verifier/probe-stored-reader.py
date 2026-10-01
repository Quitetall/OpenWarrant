# SPDX-License-Identifier: Apache-2.0
"""Manual CLI downgrade probe using only a disposable, synthetic fixture.

No signer, owner key, paid model, production verdict or authority edit is used.
The two readers and output path are explicit. Retain the report and scratch
fixture: an outcome applies to these exact executable bytes, not every version.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--current", type=Path, required=True)
parser.add_argument("--older", type=Path, required=True)
parser.add_argument("--output", type=Path, required=True)
args = parser.parse_args()
source = Path(__file__).resolve().parents[3]
base = Path(tempfile.mkdtemp(prefix="ow-stored-reader-probe-"))
root = base / "repo"
shutil.copytree(source / "conformance/fixtures/inbox/repository", root)
env = os.environ | {
    "OPENWARRANT_NO_PROJECTS": "1",
    "OPENWARRANT_NO_UPDATE_CHECK": "1",
    "CLAUDE_BIN": str(base / "never-run-claude"),
}
readers = {"current": args.current.resolve(), "older": args.older.resolve()}
report = {"fixture": str(base), "readers": {}, "observations": {}}
for name, binary in readers.items():
    with binary.open("rb") as stream:
        digest = hashlib.file_digest(stream, "sha256").hexdigest()
    report["readers"][name] = {"path": str(binary), "sha256": digest}


def run(binary, *argv):
    result = subprocess.run(
        [str(binary), *argv], cwd=root, env=env, stdin=subprocess.DEVNULL,
        capture_output=True, text=True, timeout=60,
    )
    try:
        data = json.loads(result.stdout)
    except json.JSONDecodeError:
        data = {"stdout": result.stdout, "stderr": result.stderr}
    return {"process_exit": result.returncode, "report": data}


request = run(readers["current"], "verify", "IX-WAR-0003", "--performer",
              "fixture-performer", "--bundle", "--json")
assert request["report"]["exit_code"] == 0, request
(base / "request.json").write_text(json.dumps(request["report"]))
body = 'schema = "oh.war/verification-response/v1"\nwarrant = "IX-WAR-0003"\n'
for obligation in ["OBL-001", "OBL-002"]:
    body += f'''
[[verifications]]
obligation = "{obligation}"
disposition = "established"
evidence = "synthetic downgrade control, not production assurance"
performer = "fixture-performer"
[verifications.verifier]
actor = "fixture-independent-verifier"
kind = "service"
[verifications.verifier.independence]
performer_transcript_blind = true
performer_rationale_blind = true
separate_writable_workspace = true
cannot_modify_subject_artifacts = true
cannot_modify_gate_definition = true
cannot_modify_gate_fixtures = true
separate_context_compilation = true
distinct_model_required = false
distinct_human_required = false
'''
(base / "fixture.toml").write_text(body)
bound = subprocess.check_output([
    "python3", str(Path(__file__).with_name("with-subject.py")),
    str(base / "request.json"), str(base / "fixture.toml"),
], timeout=60)
(base / "bound.toml").write_bytes(bound)
report["ingest"] = run(readers["current"], "verify", "IX-WAR-0003",
                       "--response", str(base / "bound.toml"), "--json")
assert report["ingest"]["report"]["exit_code"] == 0, report["ingest"]


def observe(phase):
    report["observations"][phase] = {
        name: {
            "status": run(binary, "status", "--json"),
            "resolve": run(binary, "resolve", "--dry-run", "IX-WAR-0003", "--json"),
        }
        for name, binary in readers.items()
    }
    args.output.write_text(json.dumps(report, indent=2) + "\n")


observe("fresh")
assurance = root / "docs/warrants/IX-WAR-0003/atoms/60-assurance.md"
text = assurance.read_text()
assert "the SAS is accepted and pinned" in text
assurance.write_text(text.replace(
    "the SAS is accepted and pinned", "the new architecture requirement is reviewed", 1
))
observe("changed")
config = root / "openwarrant.toml"
text = config.read_text()
assert "[project]" in text and "requires_war" not in text
config.write_text(text.replace(
    "[project]", '[project]\nrequires_war = ">=1.0.0-alpha.3"', 1
))
observe("minimum_reader")
print(args.output)
print(base)
