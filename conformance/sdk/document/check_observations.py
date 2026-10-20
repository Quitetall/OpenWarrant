#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Check the reference probe's observations through its executable boundary."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import shutil
import tempfile


def run(binary, fixtures):
    return subprocess.run(
        [str(binary), "--scope", "75", "--fixtures", str(fixtures), "--observations"],
        capture_output=True, text=True, timeout=60,
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path, nargs="?", help="build the probe when omitted")
    args = parser.parse_args()
    fixtures = Path(__file__).resolve().parent.parent
    if args.binary is None:
        result = subprocess.run(
            ["cargo", "build", "--locked", "-p", "openwarrant-core", "--example",
             "sdk_probe", "--message-format=json"],
            cwd=fixtures.parents[1], capture_output=True, text=True, timeout=480,
        )
        assert result.returncode == 0, result.stderr
        binaries = {
            item["executable"] for line in result.stdout.splitlines()
            if (item := json.loads(line)).get("reason") == "compiler-artifact"
            and item.get("target", {}).get("name") == "sdk_probe" and item.get("executable")
        }
        assert len(binaries) == 1, binaries
        args.binary = Path(binaries.pop())
    result = run(args.binary.resolve(), fixtures)
    assert result.returncode == 0, (result.returncode, result.stdout, result.stderr)
    rows = [json.loads(line) for line in result.stdout.splitlines()]
    cases = {row["case"]: row for row in rows if "case" in row}
    suite = json.loads((fixtures / "document/cases.json").read_text())
    assert set(cases) == {case["id"] for case in suite["cases"]}
    for case in suite["cases"]:
        row = cases[case["id"]]
        source = (fixtures / "document" / case["file"]).read_bytes()
        assert row["passed"] is True, row
        assert row["source"] == {
            "bytes": len(source), "sha256": hashlib.sha256(source).hexdigest(),
        }
    observed = cases["T01-minimal-rc3"]["observed"]
    assert observed["parse"] == "parsed"
    assert observed["original_preserved"] is True
    assert observed["metadata"]["title"] == "Make signup easier"
    scope = next(unit for unit in observed["units"] if unit["id"] == "scope")
    expected = (
        "## Scope\n\nExplore the signup form. Payment and account deletion "
        "are outside this draft.\n\n"
    )
    assert scope["text"] == expected
    source = (fixtures / "document/minimal-rc3.md").read_bytes()
    start, end = scope["span"]
    assert source[start:end] == expected.encode()
    assert observed["validation"]["readiness"] == "not_evaluated"
    assert observed["validation"]["context_resolution"] == "not_evaluated"
    summary = rows[-1]["summary"]
    assert summary["failed"] == 0
    assert summary["cases"] == len(suite["cases"])
    assert summary["qualification_established"] is False
    print("PASS probe observations retain actual source identity and exact unit bytes")

    with tempfile.TemporaryDirectory(prefix="probe-observations-") as temporary:
        copied = Path(temporary)
        shutil.copytree(fixtures / "document", copied / "document")
        changed = dict(suite["cases"][0])
        changed.update(id="RUNTIME-UNSEEN", file="unseen-runtime-document.md")
        altered = dict(suite)
        altered["cases"] = [changed, *suite["cases"][1:]]
        (copied / "document/cases.json").write_text(json.dumps(altered))
        source = (fixtures / "document/minimal-rc3.md").read_bytes().replace(
            b"Explore the signup form.", b"Explore the runtime observation form."
        )
        unknown = copied / "document" / changed["file"]
        unknown.write_bytes(source)
        result = run(args.binary.resolve(), copied)
        assert result.returncode == 0, result.stderr
        row = json.loads(result.stdout.splitlines()[0])
        assert row["case"] == "RUNTIME-UNSEEN" and row["passed"] is True
        scope = next(unit for unit in row["observed"]["units"] if unit["id"] == "scope")
        assert scope["text"] == expected.replace("signup", "runtime observation")
        start, end = scope["span"]
        assert source[start:end] == scope["text"].encode()
        assert row["source"]["sha256"] == hashlib.sha256(source).hexdigest()

        unknown.write_bytes(source + b"\ntrailing bytes are invalid\n")
        result = run(args.binary.resolve(), copied)
        assert result.returncode == 1, (result.returncode, result.stdout, result.stderr)
        rows = [json.loads(line) for line in result.stdout.splitlines()]
        row = rows[0]
        assert row["case"] == "RUNTIME-UNSEEN" and row["passed"] is False
        assert row["observed"]["parse"] == "refused"
        assert row["observed"]["diagnostic"]["code"] == "source-invalid"
        assert rows[-1]["summary"]["failed"] == 1
        assert unknown.read_bytes() == source + b"\ntrailing bytes are invalid\n"
    print("PASS unseen content changes observations; an unexpected refusal remains a failing case")

    result = subprocess.run(
        [str(args.binary.resolve()), "--scope", "75", "--fixtures", str(fixtures)],
        capture_output=True, text=True, timeout=60,
    )
    assert result.returncode == 0 and result.stdout.startswith("PASS T01-minimal-rc3\n")
    assert result.stdout.endswith(
        f"{len(suite['cases'])} document cases passed; context/readiness not evaluated\n"
    )
    print("PASS default human-readable output remains available")


if __name__ == "__main__":
    main()
