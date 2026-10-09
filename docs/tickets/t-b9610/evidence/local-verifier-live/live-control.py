#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Explicit live local-model control, never part of the offline/default gate.

The corpus is synthetic. The model answers are real. This does not qualify any
production Warrant, provider receipt protocol, or general model accuracy.
"""
import argparse
import hashlib
import json
from pathlib import Path
import re
import shutil
import subprocess
import sys
import tomllib


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--war", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--model", required=True)
    parser.add_argument("--endpoint", default="http://127.0.0.1:8020")
    args = parser.parse_args()
    repo = Path(__file__).resolve().parents[3]
    war = args.war.resolve()
    wrapper = repo / "tools/verifier/local-verifier.py"
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    shutil.copyfile(Path(__file__), output / "live-control.py")
    corpus = output / "corpus"
    shutil.copytree(repo / "conformance/fixtures/inbox/repository", corpus)

    def command(name, argv):
        result = subprocess.run([str(war), *argv, "--json"], cwd=corpus, capture_output=True, text=True)
        (output / (name + ".json")).write_text(result.stdout)
        (output / (name + ".stderr")).write_text(result.stderr)
        return json.loads(result.stdout)

    created = command("created", ["new", "Local verifier bounded observation control"])
    assert created["exit_code"] == 0, created
    alias = created["result"]["alias"]
    one = corpus / created["result"]["dir"]
    uuid = tomllib.loads((one / "manifest.toml").read_text())["uuid"]
    source = corpus / "docs/warrants/IX-WAR-0003/atoms"
    for atom in source.iterdir():
        (one / "atoms" / atom.name).write_text(re.sub(
            r"warrant_uuid: [^\n]+", "warrant_uuid: " + uuid, atom.read_text()))
    assurance = one / "atoms/60-assurance.md"
    header = assurance.read_text().split("# Assurance", 1)[0]
    assurance.write_text(header + """# Assurance

## Acceptance Obligations

### OBL-001 — the delivered control file begins with READY
- **scope:** the complete bytes of `local-review-control.txt` only. No execution or production claim.
- **evidence:** inspect the delivered text and determine whether the first five characters are READY.

### OBL-002 — a real production integration run has been observed
- **scope:** the supplied packet's executable observations only; source text cannot prove execution.
- **evidence:** an actual production integration execution receipt and observed output. If absent, this is not established.

## Gate Adequacy
This is a scratch verifier transport control, not a production qualification.

## Residual Risk
No human has accepted or authorized this scratch Warrant. The second claim has no execution evidence.
""")
    artifact = corpus / "local-review-control.txt"
    artifact.write_text("READY: alpha\n")
    (one / "deliverables.toml").write_text("""schema = "oh.war/deliverables/v1"
[[deliverable]]
id = "D-001"
title = "bounded local verifier control"
kind = "file"
target_ref = "local-review-control.txt"
required = true
content_addressed = false
provenance_required = false
obligation_refs = ["OBL-001"]
""")
    config = corpus / "openwarrant.toml"
    argv = [sys.executable, str(wrapper), "--endpoint", args.endpoint, "--model", args.model,
            "--log-root", str(output / "model-runs")]
    config.write_text(config.read_text().replace("verifier_argv = []", "verifier_argv = " + json.dumps(argv))
                      .replace("verifier_timeout_secs = 0", "verifier_timeout_secs = 900"))
    authority = {str(p.relative_to(corpus)): digest(p) for p in (corpus / "docs/authority").rglob("*") if p.is_file()}
    first = command("first", ["verify", alias, "--performer", "codex", "--run"])
    assert first["exit_code"] == 0, first
    records = one / "verifications"
    old_records = {p.name: p.read_bytes() for p in records.glob("OBL-*.toml")}
    assert len(old_records) == 2, "expected both actual model verdict records"
    before = output / "before-refresh"
    shutil.copytree(records, before)
    old_response = output / "old-response.toml"
    shutil.copyfile(next((records / "responses").glob("*.toml")), old_response)
    artifact.write_text("READY: beta\n")

    def snapshot():
        return {str(p.relative_to(one)): digest(p) for p in one.rglob("*")
                if p.is_file() and (p.is_relative_to(records) or p.name == "journal.jsonl")}

    unchanged = snapshot()
    stale = command("stale-refusal", ["verify", alias, "--response", str(old_response)])
    assert any(d["rule"] == "verify.subject-stale" for d in stale["diagnostics"]), stale
    assert snapshot() == unchanged, "stale refusal changed records or journal"
    refreshed = command("refreshed", ["verify", alias, "--performer", "codex", "--run"])
    assert refreshed["exit_code"] == 0, refreshed
    for name, raw in old_records.items():
        assert any(p.read_bytes() == raw for p in records.rglob("*.toml") if p.name != name), "old record bytes not retained"
        current = tomllib.loads((records / name).read_text())
        old = tomllib.loads(raw.decode())
        assert current["reviewed_subject"] != old["reviewed_subject"], "fresh review retained old subject"
    dispositions = {p.stem: tomllib.loads(p.read_text())["verification"]["disposition"] for p in records.glob("OBL-*.toml")}
    assert dispositions == {"OBL-001": "established", "OBL-002": "not_established"}, dispositions
    assert not (one / "authorization.toml").exists() and not (one / "resolution.toml").exists()
    assert authority == {str(p.relative_to(corpus)): digest(p) for p in (corpus / "docs/authority").rglob("*") if p.is_file()}
    summary = {"scope": "synthetic corpus, real local model; no production qualification", "warrant": alias,
               "war_sha256": digest(war), "client_sha256": digest(wrapper), "model": args.model,
               "stale_response_refused_without_writes": True, "previous_record_bytes_retained": True,
               "fresh_review_binds_changed_subject": True, "missing_execution_not_established": True,
               "authority_unchanged": True, "dispositions": dispositions}
    (output / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    print(json.dumps(summary))


if __name__ == "__main__":
    main()
