#!/usr/bin/env python3
"""Check this draft's inventory, links, traceability and compatibility examples.

This is a documentation check, not a general SDK/compiler or authority verifier.
--write-manifest derives the review inventory from the authored files.
"""
from pathlib import Path
import argparse
import hashlib
import json
import re
import subprocess
import sys
import tomllib

BASE = Path(__file__).resolve().parent
NORMATIVE = {
    "WAR_Software_Architecture_Specification.md", "format-contract.md",
    "sdk-contract.md", "phase-plan.md", "phase-1-build-scope.md",
    "skill-adaptation.md", "schemas/packet.schema.json",
    "context-views-and-shared-work.md", "work-stop-contract.md",
}


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(data):
    return "sha256:" + hashlib.sha256(data).hexdigest()


def inventory():
    result = []
    for p in sorted(BASE.rglob("*")):
        require(not p.is_symlink(), f"symlink: {p}")
        if not p.is_file() or p.name == "source-set.json":
            continue
        if "__pycache__" in p.parts:
            continue
        b = p.read_bytes()
        rel = p.relative_to(BASE).as_posix()
        result.append({"path": rel, "bytes": len(b), "sha256": sha(b),
                       "role": "normative" if rel in NORMATIVE else "reference"})
    return result


def marked_units(path):
    raw = path.read_bytes()
    _, header, body = raw.split(b"+++\n", 2)
    meta = tomllib.loads(header.decode())
    require(meta["revision"] == 3, f"wrong authored revision: {path.name}")
    require(meta["schema"] == "oh.war/document/1.0.0-rc.2", "wire grammar was renamed")
    units = set()
    fence = None
    for line in body.decode().splitlines():
        m = re.match(r"^(`{3,}|~{3,})(.*)$", line)
        if m:
            if fence is None:
                fence = m[1]
            elif m[1][0] == fence[0] and len(m[1]) >= len(fence) and not m[2].strip():
                fence = None
            continue
        if fence:
            continue
        m = re.fullmatch(r"<!-- ow:unit ([a-z][a-z0-9-]{0,63}) (binding|background) -->", line)
        if m:
            require(m[1] not in units, f"duplicate unit: {m[1]}")
            units.add(m[1])
    require(fence is None, f"unclosed fence: {path.name}")
    return meta, units


def main():
    args = argparse.ArgumentParser()
    args.add_argument("--write-manifest", action="store_true")
    options = args.parse_args()
    files = inventory()
    if options.write_manifest:
        previous = json.loads((BASE.parent / "1.0.0-rc.2/source-set.json").read_bytes())
        manifest = {"schema": previous["schema"], "edition": "1.0.0-rc.3",
                    "status": "candidate-unaccepted", "files": files,
                    "historical_baseline": previous["historical_baseline"]}
        (BASE / "source-set.json").write_text(json.dumps(manifest, sort_keys=True, separators=(",", ":")))
    manifest = json.loads((BASE / "source-set.json").read_bytes())
    require(manifest["edition"] == "1.0.0-rc.3", "wrong SAS edition")
    require(manifest["files"] == files, "source inventory/hash mismatch")
    require(NORMATIVE <= {f["path"] for f in files}, "missing normative companion")

    source = BASE / "WAR_Software_Architecture_Specification.md"
    sas = source.read_text()
    old_sas = (BASE.parent / "1.0.0-rc.2" / source.name).read_text()
    pattern = r"^\| (WAR-SAS-RQ-\d+) \|"
    ids = re.findall(pattern, sas, re.M)
    old_ids = re.findall(pattern, old_sas, re.M)
    require(len(ids) == len(set(ids)), "duplicate requirement ID")
    require(set(old_ids) <= set(ids), "historical requirement ID lost")
    require(set(ids) - set(old_ids) == {f"WAR-SAS-RQ-{n}" for n in range(122, 135)},
            "unexpected new requirement inventory")

    parsed = {p.name: marked_units(p) for p in [source, BASE / "format-contract.md"]}
    for name, (meta, units) in parsed.items():
        for edge in meta.get("dependencies", []):
            require(edge["unit"] in units, f"missing local unit: {name}/{edge['unit']}")
            target, _, unit = edge["target"].partition("#")
            target = target or name
            require((BASE / target).is_file(), f"missing dependency: {target}")
            if unit:
                require(target in parsed and unit in parsed[target][1], f"missing target unit: {target}#{unit}")

    roadmap = json.loads((BASE / "roadmap.json").read_bytes())
    require({c["case"] for c in roadmap["cases"]} == {f"T{n:02}" for n in range(1, 57)}, "case coverage")
    require(len(roadmap["cases"]) == 56, "duplicate cases")
    require({c["old_feature"] for c in roadmap["features"]} == {f"F{n:02}" for n in range(1, 12)}, "feature coverage")
    require({w["alias"] for w in roadmap["warrants"]} == {f"OW-WAR-{n:04}" for n in range(74, 92)}, "Warrant coverage")
    require(all(not w["historical_state_changed"] for w in roadmap["warrants"]), "false migration claim")

    sdk_cases = re.findall(r"^- (SDK-\d+):", (BASE / "phase-1-build-scope.md").read_text(), re.M)
    require(sdk_cases == [f"SDK-{n:02}" for n in range(1, 25)], "SDK case coverage")
    context_cases = re.findall(r"^\| (CTX-\d+) \|",
                              (BASE / "context-views-and-shared-work.md").read_text(), re.M)
    require(context_cases == [f"CTX-{n:02}" for n in range(1, 9)], "context case coverage")

    checked_links = 0
    for path in BASE.glob("*.md"):
        for target in re.findall(r"\]\(([^)\s]+)\)", path.read_text()):
            if "://" in target or target.startswith("#"):
                continue
            require((path.parent / target.split("#")[0]).is_file(), f"broken link: {path.name}: {target}")
            checked_links += 1

    old_examples = BASE.parent / "1.0.0-rc.2/examples"
    originals = [p for p in old_examples.rglob("*") if p.is_file()]
    for path in originals:
        require(path.read_bytes() == (BASE / "examples" / path.relative_to(old_examples)).read_bytes(),
                f"historical fixture changed: {path.name}")
    subprocess.run([sys.executable, str(BASE / "examples/check_examples.py")], check=True)
    print(json.dumps({"status": "PASS", "scope": "draft-documentation-only",
                      "source_files": len(files), "requirement_ids": len(ids),
                      "historical_ids_preserved": len(old_ids), "mapped_cases": 56,
                      "mapped_features": 11, "mapped_warrants": 18,
                      "sdk_cases": len(sdk_cases), "context_cases": len(context_cases),
                      "local_links": checked_links, "retained_example_files": len(originals),
                      "sdk_or_compiler_qualification": False, "signed_adoption": False}, sort_keys=True))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, subprocess.CalledProcessError) as error:
        print(f"FAIL: {error}", file=sys.stderr)
        sys.exit(1)
