#!/usr/bin/env python3
"""Audit this fixed reference corpus. Not an RC.2 parser or compiler.

Checks authored TOML, declared example membership, exact spans, byte hashes, and
stored JCS preimages. It does not implement general condition evaluation, authority
verification, canonical JSON serialization, or the planned production test suite.
"""
from pathlib import Path
import hashlib
import json
import re
import sys
import tomllib

BASE = Path(__file__).resolve().parent


def require(condition, message):
    if not condition:
        raise ValueError(message)


def sha(data):
    return "sha256:" + hashlib.sha256(data).hexdigest()


def read_json(path):
    def unique_pairs(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, f"duplicate JSON key: {key}")
            result[key] = value
        return result
    return json.loads(path.read_bytes(), object_pairs_hook=unique_pairs)


def units_in_example(path):
    raw = path.read_bytes()
    require(raw.startswith(b"+++\n"), f"fixture framing: {path.name}")
    stop = raw.find(b"\n+++\n", 4)
    require(stop >= 0, f"fixture closing delimiter: {path.name}")
    meta = tomllib.loads(raw[4:stop].decode("utf-8"))
    for field in ("schema", "kind", "id", "revision", "title", "state"):
        require(field in meta, f"missing {field}: {path.name}")
    require(meta["schema"] == "oh.war/document/1.0.0-rc.2", "wrong example edition")
    require(meta["state"] in ("draft", "proposed"), "example falsely claims acceptance")
    matches = list(re.finditer(rb"^<!-- ow:unit ([a-z][a-z0-9-]*) (binding|background) -->\n", raw, re.M))
    units = {}
    for index, match in enumerate(matches):
        uid = match[1].decode()
        require(uid not in units, f"duplicate unit: {path.name}#{uid}")
        end = matches[index + 1].start() if index + 1 < len(matches) else len(raw)
        body = raw[match.end():end]
        require(body.startswith(b"#"), f"unit heading absent: {uid}")
        require(body.partition(b"\n")[2].strip(), f"empty unit: {uid}")
        units[uid] = (match.end(), end, match[2].decode())
    required = {"adr": {"context", "decision", "consequences"}, "warrant": {"outcome", "scope", "context"}}[meta["kind"]]
    require(required <= units.keys(), f"missing required unit: {path.name}")
    for uid in required:
        require(units[uid][2] == "binding", f"required unit not binding: {uid}")
    return raw, meta, units


def main():
    parsed = {p.name: units_in_example(p) for p in sorted((BASE / "sources").glob("*.md"))}
    corpus = read_json(BASE / "expected/cases.json")
    expected = next(c for c in corpus["cases"] if c["id"] == "detailed-implementation")
    pkg = BASE / "expected/signup-packet"
    manifest = read_json(pkg / "manifest.json")
    packet = read_json(pkg / "packet.json")
    entry = (pkg / "ENTRY.md").read_bytes()
    expected_files = {f["path"] for f in manifest["files"]} | {"manifest.json"}
    actual_files = {str(p.relative_to(pkg)) for p in pkg.rglob("*") if p.is_file()}
    require(actual_files == expected_files, "package file set mismatch")
    require(not any(p.is_symlink() for p in pkg.rglob("*")), "symlink in example package")
    for item in manifest["files"]:
        path = Path(item["path"])
        require(not path.is_absolute() and ".." not in path.parts, "unsafe manifest path")
        data = (pkg / path).read_bytes()
        require(len(data) == item["bytes"] and sha(data) == item["sha256"], f"file mismatch: {path}")
    root_preimage = (BASE / "expected/package-root-preimage.json").read_bytes()
    root_value = read_json(BASE / "expected/package-root-preimage.json")
    require(root_value["digest_domain"] == "oh.war/package/1.0.0-rc.2", "wrong package domain")
    require(root_value["payload"] == {k: v for k, v in manifest.items() if k != "root_digest"}, "root preimage payload mismatch")
    require(sha(root_preimage) == manifest["root_digest"], "package root mismatch")
    basis_preimage = (BASE / "expected/basis-preimage.json").read_bytes()
    basis_value = read_json(BASE / "expected/basis-preimage.json")
    require(basis_value["digest_domain"] == "oh.war/basis/1.0.0-rc.2", "wrong basis domain")
    require(basis_value["payload"] == manifest["basis"], "basis payload mismatch")
    require(sha(basis_preimage) == packet["basis_digest"], "basis digest mismatch")
    require(manifest["basis"]["sources"] == packet["sources"], "source inventory mismatch")
    sources = {s["source_digest"]: s for s in packet["sources"]}
    supplied = set()
    for source in sources.values():
        blob = (pkg / "blobs" / (source["source_digest"][7:] + ".bin")).read_bytes()
        require(blob == (BASE / "sources" / source["path"]).read_bytes(), "source blob drift")
        require(source["byte_length"] == len(blob) and sha(blob) == source["source_digest"], "source metadata mismatch")
    for item in packet["binding_context"]:
        ref = item["ref"]
        src = sources[ref["source_digest"]]
        name = src["path"]
        raw = (pkg / "blobs" / (ref["source_digest"][7:] + ".bin")).read_bytes()
        supplied.add(name + "#" + ref["unit"])
        if ref["unit"] == "*":
            require(ref["start"] == 0 and ref["end"] == len(raw), "opaque span mismatch")
        else:
            start, end, kind = parsed[name][2][ref["unit"]]
            require((ref["start"], ref["end"]) == (start, end), "unit span mismatch")
            require(item["kind"] == kind, "unit kind mismatch")
            require(raw[start:end] in entry, "required exact text missing from entry")
    require(supplied == set(expected["required_units"]), "reference membership differs from reviewed expectation")
    require("architecture.md#inactive" in supplied, "definition without SHALL was lost")
    for omitted in expected["omitted_units"]:
        require(omitted not in supplied, "unexpected selected unit")
    history = next(c for c in packet["reference_catalog"] if c["id"] == "design-history")
    require(not history["supplied"] and not history["selected"], "absent optional content mislabeled")
    require(packet["readiness"]["value"] == "blocked", "example falsely ready to execute")
    require(not manifest["basis"]["records"], "example invents authority records")
    account = packet["accounting"]
    require(account["entry_bytes"] == len(entry), "entry accounting mismatch")
    require(account["estimated_tokens"] == (len(entry) + 3) // 4, "estimate mismatch")
    require(account["estimated_tokens"] <= account["budget"], "example exceeds declared budget")
    require(account["package_payload_bytes"] == len(entry) + sum(s["byte_length"] for s in sources.values()), "payload accounting mismatch")
    print(f"PASS reference audit: {len(parsed)} Markdown sources; {len(supplied)} selected units; {len(expected_files)} package files; exact text and digests intact.")
    print("NOT RUN: production RC.2 parser/compiler conformance, condition cases, authority/assurance verification, or workflow execution.")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, KeyError, OSError, StopIteration) as error:
        print(f"FAIL reference audit: {error}", file=sys.stderr)
        raise SystemExit(1)
