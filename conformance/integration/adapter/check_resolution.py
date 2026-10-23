#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Observe refusal of an actual Cargo SDK override in a disposable provider checkout."""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import tomllib


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--provider-root", type=Path, required=True)
    parser.add_argument("--record-dir", type=Path, required=True)
    args = parser.parse_args()
    root = args.provider_root.resolve(strict=True)
    output = args.record_dir.resolve()
    output.mkdir(parents=True, exist_ok=False)
    manifest = root / "lamu-openwarrant/Cargo.toml"
    pin = "4b626f23b3076ff7e764f08eeb15e8bdfa6af5cd"
    declared = tomllib.loads(manifest.read_text())["dependencies"]["openwarrant-core"]
    assert declared == {"git": "https://github.com/Quitetall/OpenWarrant.git", "rev": pin}
    metadata_argv = ["cargo", "+1.97.1", "metadata", "--locked", "--format-version", "1",
                     "--manifest-path", str(manifest)]
    metadata = subprocess.run(metadata_argv, cwd=root, capture_output=True, text=True, timeout=600)
    (output / "metadata.json").write_text(metadata.stdout)
    (output / "metadata.stderr").write_text(metadata.stderr)
    assert metadata.returncode == 0, metadata.stderr
    value = json.loads(metadata.stdout)
    provider = next(p for p in value["packages"] if Path(p["manifest_path"]).resolve() == manifest)
    node = next(n for n in value["resolve"]["nodes"] if n["id"] == provider["id"])
    dependency = next(d for d in node["deps"] if d["name"] == "openwarrant_core")
    actual = next(p for p in value["packages"] if p["id"] == dependency["pkg"])
    assert actual["source"] is None, "This refusal control requires an actual path override"
    assert Path(actual["manifest_path"]).is_file()
    argv = [sys.executable, str(Path(__file__).with_name("run.py")),
            "--provider-root", str(root), "--output", str(output / "unexpected-observation.json")]
    result = subprocess.run(argv, capture_output=True, text=True, timeout=6000)
    (output / "driver.stdout").write_text(result.stdout)
    (output / "driver.stderr").write_text(result.stderr)
    report = {"metadata_argv": metadata_argv, "metadata_exit": metadata.returncode,
              "driver_argv": argv, "driver_exit": result.returncode,
              "declared_sdk": declared, "actual_sdk": {k: actual[k] for k in ["id", "source", "manifest_path"]},
              "observation_published": (output / "unexpected-observation.json").exists(),
              "qualification_established": False}
    (output / "result.json").write_text(json.dumps(report, indent=2) + "\n")
    assert result.returncode != 0, "Driver accepted a different resolved SDK under the original declared pin"
    assert "Provider SDK resolution differs from shared contract" in result.stderr, result.stderr
    assert not report["observation_published"]
    print("PASS: actual Cargo override refused before observation publication")


if __name__ == "__main__":
    main()
