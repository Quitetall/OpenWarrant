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
parser.add_argument("--require-old-refusal", action="store_true",
                    help="fail if the older reader counts newly bound records as established")
parser.add_argument("--require-history-retention", action="store_true",
                    help="re-review changed work and require the previous record bytes to survive")
parser.add_argument("--require-history-refusals", action="store_true",
                    help="require collision and link refusals to preserve active records and journal; needs --require-history-retention")
args = parser.parse_args()
if args.require_history_refusals and not args.require_history_retention:
    parser.error("--require-history-refusals requires --require-history-retention")
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


initial_records = {
    obligation: (root / "docs/warrants/IX-WAR-0003/verifications" / f"{obligation}.toml").read_bytes()
    for obligation in ("OBL-001", "OBL-002")
}


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

def established(phase, reader):
    status = report["observations"][phase][reader]["status"]["report"]
    return sum(
        obligation["disposition"] == "established"
        for warrant in status.get("result", {}).get("warrants", [])
        if warrant["alias"] == "IX-WAR-0003"
        for obligation in warrant["obligations"]
    )


if args.require_old_refusal:
    assert established("fresh", "current") == 2, "the new reader must admit fresh review"
    assert established("changed", "current") == 0, "the new reader must refuse stale review"
    for phase in ("fresh", "changed", "minimum_reader"):
        observed = report["observations"][phase]["older"]
        refusal = observed["resolve"]
        assert observed["status"]["report"].get("schema") == "oh.war/report/v1", observed
        assert established(phase, "older") == 0, (
            f"older reader must not count unsupported records ({phase}): {observed}"
        )
        assert refusal["process_exit"] != 0 and any(
            diagnostic["rule"] == "verification.malformed"
            for diagnostic in refusal["report"].get("diagnostics", [])
        ), f"resolution must name the record-decoder refusal ({phase}): {refusal}"

if args.require_history_retention:
    # Undo only the setting this disposable probe just introduced. The changed
    # outcome remains, so the next review must bind a genuinely new subject.
    config.write_text(text)
    request = run(readers["current"], "verify", "IX-WAR-0003", "--performer",
                  "fixture-performer", "--bundle", "--json")
    assert request["report"]["exit_code"] == 0, request
    (base / "request-next.json").write_text(json.dumps(request["report"]))
    rebound = subprocess.check_output([
        "python3", str(Path(__file__).with_name("with-subject.py")),
        str(base / "request-next.json"), str(base / "fixture.toml"),
    ], timeout=60)
    (base / "rebound.toml").write_bytes(rebound)
    if args.require_history_refusals:
        records = root / "docs/warrants/IX-WAR-0003/verifications"
        history = records / "history"
        history.mkdir(exist_ok=True)
        collision = history / (hashlib.sha256(initial_records["OBL-002"]).hexdigest() + ".toml")
        journal = root / "docs/warrants/IX-WAR-0003/journal.jsonl"
        before_journal = journal.read_bytes()
        outside = base / "external-history-target.toml"
        sentinel = b"occupied retained evidence must not be overwritten\n"
        outside.write_bytes(sentinel)
        report["history_refusals"] = {}
        for kind in ("collision", "link"):
            if kind == "collision":
                collision.write_bytes(sentinel)
            else:
                collision.symlink_to(outside)
            refusal = run(readers["current"], "verify", "IX-WAR-0003",
                          "--response", str(base / "rebound.toml"), "--json")
            unchanged = all((records / (obligation + ".toml")).read_bytes() == previous
                            for obligation, previous in initial_records.items())
            report["history_refusals"][kind] = {
                "response": refusal, "active_records_unchanged": unchanged,
                "journal_unchanged": journal.read_bytes() == before_journal,
                "occupied_bytes_unchanged": collision.read_bytes() == sentinel,
                "external_bytes_unchanged": outside.read_bytes() == sentinel,
            }
            args.output.write_text(json.dumps(report, indent=2) + "\n")
            assert refusal["process_exit"] != 0 and any(
                diagnostic["rule"] == "verify.history-unavailable"
                for diagnostic in refusal["report"].get("diagnostics", [])
            ), f"retention must refuse occupied or linked evidence: {refusal}"
            assert unchanged and journal.read_bytes() == before_journal, (
                "failed retention must not replace active records or append a review", report["history_refusals"][kind]
            )
            assert collision.read_bytes() == sentinel and outside.read_bytes() == sentinel
            if kind == "link":
                assert collision.is_symlink(), "the refused link must remain unchanged"
            collision.unlink()

    report["re_review"] = run(readers["current"], "verify", "IX-WAR-0003",
                              "--response", str(base / "rebound.toml"), "--json")
    assert report["re_review"]["report"]["exit_code"] == 0, report["re_review"]
    report["retained_history"] = {}
    for obligation, previous in initial_records.items():
        digest = hashlib.sha256(previous).hexdigest()
        history = root / "docs/warrants/IX-WAR-0003/verifications/history" / f"{digest}.toml"
        report["retained_history"][obligation] = {
            "path": str(history), "sha256": digest,
            "exact_bytes_retained": history.is_file() and history.read_bytes() == previous,
        }
    observe("re_reviewed")
    assert all(item["exact_bytes_retained"] for item in report["retained_history"].values()), (
        "replacing a review must retain the exact earlier bytes", report["retained_history"]
    )

    assert established("re_reviewed", "current") == 2, "fresh replacement review must qualify"
    for obligation, previous in initial_records.items():
        active = root / "docs/warrants/IX-WAR-0003/verifications" / f"{obligation}.toml"
        assert active.read_bytes() != previous, "a changed subject must have a distinct bound record"
