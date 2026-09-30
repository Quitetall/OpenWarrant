#!/usr/bin/env python3
"""Observe the proposed doctor gate against clean source and three regressions.

This writes review evidence, not Warrant receipts or verifier dispositions.
Source mutations occur only in disposable archives of the named commit.
"""

import hashlib
import gzip
import json
import pathlib
import subprocess
import tarfile
import tempfile


ROOT = pathlib.Path(__file__).resolve().parents[3]
OUT = pathlib.Path(__file__).resolve().parent / "observations"
ARGV = ["python3", str(pathlib.Path(__file__).resolve().parent / "doctor-gate.py")]
SOURCE = pathlib.Path("crates/openwarrant-cli/src/doctor.rs")
TEST = "doctor_aggregates_refusals_without_running_backends_or_mutating_records"
MUTATIONS = [
    (
        "false-authorization",
        '"execution_authorized": false',
        '"execution_authorized": true',
    ),
    (
        "subject-write",
        "    match crate::check::run(&repo, alias, generated) {",
        '    std::fs::write(repo.root.join("PLANTED_DOCTOR_WRITE"), b"plant").unwrap();\n'
        "    match crate::check::run(&repo, alias, generated) {",
    ),
    (
        "hidden-authority-error",
        'Err(error) => unavailable(&mut report, "authority", error),',
        "Err(_error) => {},",
    ),
]


def snapshot(root):
    return {
        str(path.relative_to(root)): hashlib.sha256(path.read_bytes()).hexdigest()
        for path in root.rglob("*") if path.is_file()
    }


def main():
    revision = subprocess.check_output(
        ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True,
    ).strip()
    archive = subprocess.check_output(["git", "archive", revision], cwd=ROOT)
    OUT.mkdir(parents=True, exist_ok=True)
    report = {
        "schema": "oh.war/doctor-gate-qualification-observation/v1",
        "kind": "performer_observation",
        "source_revision": revision,
        "source_archive_sha256": hashlib.sha256(archive).hexdigest(),
        "argv": ARGV,
        "adapter_sha256": hashlib.sha256(pathlib.Path(ARGV[1]).read_bytes()).hexdigest(),
        "cases": [],
    }
    with tempfile.TemporaryDirectory(prefix="ow97-doctor-gate-") as temporary:
        workspace = pathlib.Path(temporary)
        archive_path = workspace / "source.tar"
        archive_path.write_bytes(archive)
        cases = [("clean", None, None), *MUTATIONS, ("clean-after-mutations", None, None)]
        for name, old, new in cases:
            case = workspace / name
            case.mkdir()
            with tarfile.open(archive_path) as source:
                source.extractall(case, filter="data")
            if old is not None:
                path = case / SOURCE
                body = path.read_text()
                if body.count(old) != 1:
                    raise RuntimeError(f"{name}: mutation must match exactly once")
                path.write_text(body.replace(old, new))
            before = snapshot(case)
            print(f"Running {name} against {revision}", flush=True)
            log = OUT / f"{name}.log"
            try:
                with log.open("w") as output:
                    run = subprocess.run(
                        ARGV, cwd=case, stdout=output, stderr=subprocess.STDOUT,
                        timeout=900, check=False,
                    )
            except subprocess.TimeoutExpired:
                report["cases"].append({"name": name, "observation": "UNKNOWN", "reason": "timeout"})
                (OUT / "qualification.json").write_text(json.dumps(report, indent=2) + "\n")
                raise
            after = snapshot(case)
            text = log.read_text()
            raw_log = log.read_bytes()
            suite_ran = "running 2 tests" in text
            expected = (
                run.returncode == 0 and "2 passed; 0 failed" in text
                if old is None else
                run.returncode == 101 and f"{TEST} ... FAILED" in text
                and "test result: FAILED" in text
            )
            observed = expected and suite_ran and before == after
            compressed = log.with_suffix(".log.gz")
            compressed.write_bytes(gzip.compress(raw_log, mtime=0))
            log.unlink()
            report["cases"].append({
                "name": name,
                "exit_code": run.returncode,
                "expected_result_observed": observed,
                "suite_ran": suite_ran,
                "source_unchanged_by_execution": before == after,
                "source_snapshot_sha256": hashlib.sha256(
                    json.dumps(before, sort_keys=True).encode()
                ).hexdigest(),
                "mutation": None if old is None else {"path": str(SOURCE), "before": old, "after": new},
                "log": compressed.name,
                "log_sha256": hashlib.sha256(compressed.read_bytes()).hexdigest(),
                "uncompressed_log_sha256": hashlib.sha256(raw_log).hexdigest(),
            })
            (OUT / "qualification.json").write_text(json.dumps(report, indent=2) + "\n")
            print(f"{name}: exit={run.returncode}, expected={observed}, source_unchanged={before == after}", flush=True)
            if not observed:
                raise RuntimeError(f"{name}: qualification control did not establish its claim; see {compressed}")


if __name__ == "__main__":
    main()
