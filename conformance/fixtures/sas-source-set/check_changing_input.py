#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Observe concurrent-read refusal through the real proposal command.

Uses disposable repositories and no identities or signing keys. A bounded
stress observation complements deterministic fixtures; it is not a scheduling
or filesystem-isolation guarantee. Exit nonzero if no inconsistent read was
observed. Every attempted proposal must refuse before publishing a record.
"""
import hashlib
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import threading

war = str(Path(sys.argv[1]).resolve())
observations = []
with tempfile.TemporaryDirectory(prefix="ow-changing-source-") as temporary:
    for attempt in range(12):
        root = Path(temporary) / str(attempt)
        root.mkdir()
        subprocess.run([war, "init", "--namespace", "TEST"], cwd=root, check=True, capture_output=True)
        (root / "candidate").mkdir()
        main = b"# SAS\n| TEST-SAS-RQ-001 | Preserve evidence. |\n"
        bulk = b"a" * (8 * 1024 * 1024)
        (root / "candidate/main.md").write_bytes(main)
        (root / "candidate/changing.bin").write_bytes(bulk)
        (root / "decision.md").write_text("Fixture decision.\n")
        files = [{"path": path, "bytes": len(data), "sha256": "sha256:" + hashlib.sha256(data).hexdigest(), "role": role}
                 for path, data, role in [("main.md", main, "normative"), ("changing.bin", bulk, "reference")]]
        manifest = {"schema": "oh.war/spec-source-set/1.0.0-rc.2", "edition": "1.0.0-rc.2", "status": "candidate-unaccepted", "files": files}
        (root / "candidate/source-set.json").write_text(json.dumps(manifest))
        stop, started = threading.Event(), threading.Event()
        def change():
            with (root / "candidate/changing.bin").open("r+b", buffering=0) as output:
                payload = b"b" * len(bulk)
                while not stop.is_set():
                    output.seek(0)
                    output.write(payload)
                    started.set()
        worker = threading.Thread(target=change)
        worker.start()
        try:
            if not started.wait(5):
                raise RuntimeError("writer did not start")
            result = subprocess.run([war, "--json", "sas", "propose", "1.0.0-rc.2", "--source", "candidate/main.md",
                                     "--source-set", "candidate/source-set.json", "--adr", "decision.md"],
                                    cwd=root, capture_output=True, text=True, timeout=10)
        finally:
            stop.set()
            worker.join(5)
        if worker.is_alive():
            raise RuntimeError("writer did not stop")
        report = json.loads(result.stdout)
        observations.append({"attempt": attempt + 1, "exit_code": result.returncode, "diagnostics": report["diagnostics"]})
        if result.returncode == 0 or (root / "docs/sas/revisions/1.0.0-rc.2.toml").exists():
            raise RuntimeError("changing mismatched input published a revision")
        if "inconsistent read" in result.stdout:
            print(json.dumps({"result": "PASS", "observations": observations}, indent=2))
            break
    else:
        print(json.dumps({"result": "UNKNOWN", "observations": observations}, indent=2))
        sys.exit(1)
