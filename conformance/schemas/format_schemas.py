#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Check the hand-written normative schemas under schemas/format/.

Three parts, each reported line by line and failed by name:

1. The registry (schemas/format/registry.json). Every frozen record of
   docs/COMPATIBILITY.md, and of `FROZEN` in crates/openwarrant-cli/src/compat.rs,
   has a schema; every schema under schemas/oh.war/ and schemas/format/ has its
   `$id` in the registry; every registry entry's file exists and carries that
   `$id`. The check is then planted in memory (a frozen id with no schema, a
   schema missing from the registry, a registry entry whose file is gone) and
   must refuse each by name.
2. Every hand-written schema is a valid draft 2020-12 schema.
3. Every hand-written schema is run against instances, and must accept at least
   one and refuse at least one: the repository's own files where they exist
   (SAS revisions, amendments, profiles, type packs, a legacy manifest that the
   Format 1.0 manifest must refuse), the examples of docs/format/FORMAT-1.0.md,
   the fixtures beside this script, and copies of real records doctored in
   memory. An invalid fixture names the error it must be refused with, so a
   schema that refuses it for some other reason does not pass.

The readers below (Format 1.0 frontmatter, attribute blocks, items; the
structured-atom subset) are conversion helpers for this check only. They are
not a reference implementation of FORMAT-1.0.md.

    python3 conformance/schemas/format_schemas.py [--root DIR] [--legacy-report]

`--legacy-report` also lowers every legacy ticket and Warrant manifest to Format
1.0 frontmatter (section 12) and prints which would be refused and why. It is a
report for the format's authors, not a check: the lowering's summary is an
assumption the specification does not state.

Exit 0 when every check passes, 1 when any fails, 2 when the check cannot run
(jsonschema missing). Requires jsonschema 4.26.0 (CI pins it).
"""

import argparse
import copy
import json
import re
import sys
import tomllib
from pathlib import Path

try:
    from jsonschema import Draft202012Validator
    from referencing import Registry, Resource
except ImportError as err:  # pragma: no cover - environment
    print(f"UNKNOWN cannot import jsonschema/referencing ({err}); nothing was checked", file=sys.stderr)
    sys.exit(2)


HERE = Path(__file__).resolve().parent
FIXTURES = HERE / "fixtures"


class ReadError(Exception):
    """A helper reader refused its input. Carries the rule a conforming reader would use."""

    def __init__(self, rule, line, message):
        super().__init__(f"{rule} line {line}: {message}")
        self.rule = rule
        self.line = line


# ---------------------------------------------------------------- readers


def read_structured(text):
    """The restricted structured-atom subset (crates/openwarrant-core/src/structured.rs).

    Top-level `key: scalar`, `key: [flow, list]`, or `key:` and a block
    sequence of flat mappings. Every scalar a string: no implicit typing.
    """
    def unquote(raw, line):
        t = raw.strip()
        for q in ('"', "'"):
            if t.startswith(q):
                rest = t[1:]
                if rest and rest.endswith(q):
                    return rest[:-1]
                raise ReadError("structured.quote", line, t)
        return t

    def flow(raw, line):
        inner = raw.strip()[1:-1] if raw.strip().endswith("]") else None
        if inner is None:
            raise ReadError("structured.malformed", line, raw)
        if "[" in inner or "{" in inner:
            raise ReadError("structured.unsupported", line, "a nested flow collection")
        if not inner.strip():
            return []
        items, cur, quoted = [], "", False
        for ch in inner:
            if ch == '"':
                quoted = not quoted
                cur += ch
            elif ch == "," and not quoted:
                items.append(cur)
                cur = ""
            else:
                cur += ch
        if quoted:
            raise ReadError("structured.malformed", line, inner)
        items.append(cur)
        return [unquote(i, line) for i in items]

    def pair(text, line):
        if ":" not in text:
            raise ReadError("structured.malformed", line, text)
        key, rest = text.split(":", 1)
        key, rest = key.strip(), rest.strip()
        if not key or key[0] in "\"'":
            raise ReadError("structured.malformed", line, text)
        if not rest:
            return key, None
        if rest[0] in "&*!{|>":
            raise ReadError("structured.unsupported", line, rest[0])
        return key, (flow(rest, line) if rest.startswith("[") else unquote(rest, line))

    out, open_key, records = {}, None, []
    text = text.removeprefix("﻿")
    for n, raw in enumerate(text.splitlines(), 1):
        t = raw.strip()
        if not t or t.startswith("#"):
            continue
        if t in ("---", "..."):
            raise ReadError("structured.unsupported", n, "a multi-document stream marker")
        indent = len(raw) - len(raw.lstrip())
        if indent == 0:
            if open_key is not None:
                out[open_key], open_key, records = records, None, []
            key, value = pair(t, n)
            if key in out:
                raise ReadError("structured.duplicate", n, key)
            if value is None:
                open_key = key
            else:
                out[key] = value
        elif indent == 2:
            if not t.startswith("- ") or open_key is None:
                raise ReadError("structured.malformed", n, t)
            key, value = pair(t[2:], n)
            if value is None:
                raise ReadError("structured.unsupported", n, "a nested block sequence")
            records.append({key: value})
        elif indent == 4:
            if t.startswith("- ") or not records:
                raise ReadError("structured.unsupported", n, t)
            key, value = pair(t, n)
            if value is None or key in records[-1]:
                raise ReadError("structured.unsupported", n, t)
            records[-1][key] = value
        else:
            raise ReadError("structured.indent", n, str(indent))
    if open_key is not None:
        out[open_key] = records
    return out


NAME = r"(?:[a-z][a-z0-9_-]*|[a-z0-9][a-z0-9_-]*(?:\.[a-z0-9][a-z0-9_-]*)+)"
FM_PAIR = re.compile(rf"^({NAME}):[ ](.*)$")
FM_HEAD = re.compile(rf"^({NAME}):$")
FM_ITEM = re.compile(r"^  - (.*)$")


def fm_scalar(raw, line):
    """Appendix A `scalar`: plain, double-quoted (\\" \\\\ only) or single-quoted ('' for ')."""
    if raw.startswith('"'):
        out, i = "", 1
        while i < len(raw):
            c = raw[i]
            if c == "\\":
                if i + 1 < len(raw) and raw[i + 1] in '"\\':
                    out += raw[i + 1]
                    i += 2
                    continue
                raise ReadError("format.frontmatter", line, "an escape other than \\\" or \\\\")
            if c == '"':
                if raw[i + 1:].strip():
                    raise ReadError("format.frontmatter", line, "text after a quoted scalar")
                return out
            out += c
            i += 1
        raise ReadError("format.frontmatter", line, "an unterminated double-quoted scalar")
    if raw.startswith("'"):
        body = raw[1:]
        out, i = "", 0
        while i < len(body):
            if body[i] == "'":
                if body[i + 1:i + 2] == "'":
                    out += "'"
                    i += 2
                    continue
                if body[i + 1:].strip():
                    raise ReadError("format.frontmatter", line, "text after a quoted scalar")
                return out
            out += body[i]
            i += 1
        raise ReadError("format.frontmatter", line, "an unterminated single-quoted scalar")
    names = {"&": "an anchor", "*": "an alias", "!": "a tag", "[": "a flow collection",
             "{": "a flow collection", "|": "a block scalar", ">": "a folded scalar"}
    if raw[:1] in names:
        raise ReadError("format.frontmatter", line, names[raw[:1]])
    if raw[:1] in "%@`" or raw.startswith("- ") or ": " in raw or " #" in raw or not raw:
        raise ReadError("format.frontmatter", line, f"a plain scalar Appendix A does not admit: {raw!r}")
    return raw


def read_frontmatter(text):
    """Section 3.2 and Appendix A `frontmatter`, to a JSON object. Returns (object, rest)."""
    text = text.removeprefix("﻿")
    lines = text.split("\n")
    if lines[0].rstrip("\r") != "---":
        raise ReadError("format.frontmatter", 1, "a missing fence")
    obj, head = {}, None
    for n, raw in enumerate(lines[1:], 2):
        line = raw.rstrip("\r")
        if line == "---":
            return obj, "\n".join(lines[n:])
        if not line.strip() or line.startswith("#"):
            continue
        m = FM_ITEM.match(line)
        if m:
            if head is None:
                raise ReadError("format.frontmatter", n, "a list item under no list key")
            obj[head].append(fm_scalar(m.group(1), n))
            continue
        if line[:1] in (" ", "\t"):
            raise ReadError("format.frontmatter", n, "a nested mapping")
        m = FM_HEAD.match(line) or FM_PAIR.match(line)
        if not m:
            raise ReadError("format.frontmatter", n, f"not a frontmatter line: {line!r}")
        key = m.group(1)
        if key in obj:
            raise ReadError("format.frontmatter", n, f"a duplicate key {key!r}")
        if m.re is FM_HEAD:
            obj[key], head = [], key
        else:
            obj[key], head = fm_scalar(m.group(2), n), None
    raise ReadError("format.frontmatter", len(lines), "a missing closing fence")


CORE_KINDS = {"part_of", "depends_on", "implements", "constrains", "evaluates", "supersedes",
              "supports", "refutes", "trades_off_against", "causes", "qualifies", "selected_over"}
RECORD_HEAD = re.compile(r"^## (\S+) · (\S+) — (.+)$")
FIELD = re.compile(rf"^({NAME}):(?: (.*))?$")
RELATION = re.compile(rf"^({NAME}) (\S.*)$")
TARGET = re.compile(r"^(.+?)(?:@(sha256:[0-9a-f]{64}))?$")


def read_record(unit):
    """Sections 3.4 to 3.6: one record unit's heading, attribute block and body."""
    lines = unit.split("\n")
    m = RECORD_HEAD.match(lines[0].rstrip("\r"))
    if not m:
        raise ReadError("format.heading", 1, lines[0])
    record = {"id": m.group(1), "type": m.group(2), "title": m.group(3),
              "fields": {}, "relations": [], "body": ""}
    last = None
    i = 1
    for i, raw in enumerate(lines[1:], 1):
        line = raw.rstrip("\r")
        if not line.strip():
            break
        if line.startswith("  "):
            if last is None:
                raise ReadError("format.attribute", i + 1, "a continuation under no field")
            prior = record["fields"][last]
            record["fields"][last] = line[2:] if prior == "" else prior + "\n" + line[2:]
            continue
        f = FIELD.match(line)
        if f:
            if f.group(1) in record["fields"]:
                raise ReadError("format.field-duplicate", i + 1, f.group(1))
            record["fields"][f.group(1)] = f.group(2) or ""
            last = f.group(1)
            continue
        r = RELATION.match(line)
        if r and (r.group(1) in CORE_KINDS or "." in r.group(1)):
            for target in r.group(2).split(", "):
                t = TARGET.match(target)
                rel = {"kind": r.group(1), "to": t.group(1)}
                if t.group(2):
                    rel["to_revision"] = t.group(2)
                record["relations"].append(rel)
            last = None
            continue
        raise ReadError("format.attribute", i + 1, line)
    else:
        i = len(lines)
    record["body"] = "\n".join(lines[i + 1:])
    return record


ITEM = re.compile(
    r"^\s*[-*] \[(?P<box>[^\]]*)\] (?P<text>.*?)"
    r"(?: \((?P<id>i-[0-9a-f]+)(?P<after>(?:, after [^,()\s]+)*)\))?"
    r"(?P<done> — done by (?P<actor>[^,]+), (?P<date>[0-9]{4}-[0-9]{2}-[0-9]{2})"
    r"(?: \[(?P<level>[a-z]+)\])?(?:: (?P<note>.*))?)?$"
)


def read_item(line):
    """Section 3.8 and Appendix A `item`, or None for a line that is not a box line."""
    m = ITEM.match(line.rstrip("\r"))
    if not m:
        return None
    if m.group("box") not in (" ", "x", "X"):
        raise ReadError("format.item", 1, f"box [{m.group('box')}]")
    item = {"done": m.group("box") != " ", "text": m.group("text"), "after": []}
    if m.group("id"):
        item["id"] = m.group("id")
        item["after"] = re.findall(r", after ([^,()\s]+)", m.group("after") or "")
    implements = re.findall(r"\bimplements (\S+)", m.group("text"))
    if implements:
        item["implements"] = implements
    if m.group("done"):
        item["done_by"] = m.group("actor")
        item["done_on"] = m.group("date")
        if m.group("level") is not None:
            item["level"] = m.group("level")
        if m.group("note") is not None:
            item["note"] = m.group("note")
    return item


def spec_examples(spec_text):
    """The ```markdown examples of FORMAT-1.0.md, as (label, schema key, instance)."""
    out = []
    blocks = re.findall(r"^```markdown\n(.*?)^```$", spec_text, flags=re.S | re.M)
    for n, block in enumerate(blocks, 1):
        # Section 3.7's example pins a target with the placeholder `<64 hex>`,
        # which does not parse as a target (`format.relation-target`). It is
        # read with 64 zeros in its place, and that substitution is reported.
        block = block.replace("sha256:<64 hex>", "sha256:" + "0" * 64)
        label = f"FORMAT-1.0.md markdown example {n}"
        if block.startswith("---\n"):
            fm, _ = read_frontmatter(block)
            out.append((label + " (frontmatter)", "frontmatter", fm))
            if fm.get("profile") == "ticket":
                out.append((label + " (frontmatter)", "ticket", fm))
            continue
        if RECORD_HEAD.match(block.split("\n", 1)[0]):
            record = read_record(block.rstrip("\n"))
            key = {"obligation": "obligation", "check": "check", "kpi": "check"}.get(record["type"], "record")
            out.append((label + f" ({record['id']})", key, record))
            if key != "record":
                out.append((label + f" ({record['id']} as a record)", "record", record))
            continue
        items = [read_item(line) for line in block.split("\n") if line.strip()]
        if items and all(items):
            for i, item in enumerate(items, 1):
                out.append((label + f" (item {i})", "item", item))
    return out


# ---------------------------------------------------------------- registry


def frozen_from_compatibility(text):
    """The record names after "**Record shapes** `oh.war/<record>/v1` —" in COMPATIBILITY.md."""
    m = re.search(r"\*\*Record shapes\*\*\s+`oh\.war/<record>/v1`\s+—(.*?)\.\s+Their JSON Schemas", text, re.S)
    if not m:
        return None
    names = []
    for part in m.group(1).split(","):
        name = re.sub(r"\(.*?\)", "", part).strip()
        names.append(name)
    return names


def frozen_from_compat_rs(text):
    m = re.search(r"pub\(crate\) const FROZEN: &\[\(&str, u32\)\] = &\[(.*?)\];", text, re.S)
    if not m:
        return None
    return re.findall(r'\("([a-z0-9-]+)",\s*\d+\)', m.group(1))


def registry_problems(compat_text, compat_rs_text, registry, schema_files):
    """Every way the registry drifts. `schema_files` maps schemas/-relative path -> $id (or None)."""
    problems = []
    frozen = frozen_from_compatibility(compat_text)
    if frozen is None:
        return ["docs/COMPATIBILITY.md: the frozen record list could not be read"]
    in_code = frozen_from_compat_rs(compat_rs_text)
    if in_code is None:
        problems.append("crates/openwarrant-cli/src/compat.rs: FROZEN could not be read")
    else:
        for name in sorted(set(frozen) - set(in_code)):
            problems.append(f"oh.war/{name}/v1: frozen in docs/COMPATIBILITY.md and absent from compat.rs FROZEN")
        for name in sorted(set(in_code) - set(frozen)):
            problems.append(f"oh.war/{name}: in compat.rs FROZEN and not frozen in docs/COMPATIBILITY.md")
    entries = registry.get("entries", [])
    by_id = {}
    for e in entries:
        by_id.setdefault(e["id"], []).append(e)
    for name in frozen:
        wire = f"oh.war/{name}/v1"
        present = [e for e in by_id.get(wire, []) if e["schema"] in schema_files]
        if not present:
            problems.append(f"{wire}: frozen in docs/COMPATIBILITY.md and no schema file is registered for it")
    registered = {}
    for e in entries:
        path = e["schema"]
        registered[e["$id"]] = path
        if path not in schema_files:
            problems.append(f"{e['id']}: registry names schemas/{path}, which does not exist")
        elif schema_files[path] != e["$id"]:
            problems.append(f"{e['id']}: schemas/{path} has $id {schema_files[path]!r}, registry says {e['$id']!r}")
        if e.get("authority") not in ("normative", "generated"):
            problems.append(f"{e['id']}: authority {e.get('authority')!r} is not normative or generated")
    for path, sid in sorted(schema_files.items()):
        if sid is None:
            problems.append(f"schemas/{path}: no $id")
        elif registered.get(sid) != path:
            problems.append(f"schemas/{path}: $id {sid!r} is not in schemas/format/registry.json")
    return problems


def schema_files_under(schemas_dir):
    files = {}
    for sub in ("oh.war", "format"):
        for p in sorted((schemas_dir / sub).rglob("*.json")):
            rel = p.relative_to(schemas_dir).as_posix()
            if rel == "format/registry.json":
                continue
            files[rel] = json.loads(p.read_text()).get("$id")
    return files


# ---------------------------------------------------------------- the run


class Tally:
    def __init__(self):
        self.passed = 0
        self.failed = []

    def ok(self, label, detail=""):
        self.passed += 1
        print(f"ok    {label}{'  ' + detail if detail else ''}")

    def fail(self, label, detail):
        self.failed.append(label)
        print(f"FAIL  {label}  {detail}")


def errors(validator, instance):
    return sorted(validator.iter_errors(instance), key=lambda e: list(e.absolute_path))


def describe(errs):
    return "; ".join(f"{'/'.join(map(str, e.absolute_path)) or '<root>'}: {e.message}"[:240] for e in errs[:3])


def expect_accept(t, counts, key, validator, label, instance):
    if validator is None:
        t.fail(f"{key}: accepts {label}", "the schema could not be loaded")
        return
    errs = errors(validator, instance)
    if errs:
        t.fail(f"{key}: accepts {label}", describe(errs))
    else:
        counts[key][0] += 1
        t.ok(f"{key}: accepts {label}")


def expect_refuse(t, counts, key, validator, label, instance, expect):
    if validator is None:
        t.fail(f"{key}: refuses {label}", "the schema could not be loaded")
        return
    errs = errors(validator, instance)
    # Every error, including those under anyOf/oneOf/if branches, so a fixture
    # can name the keyword that refuses it wherever that keyword sits.
    def flatten(es):
        for e in es:
            yield e
            yield from flatten(e.context or [])
    # `@<instance path> <keyword>: <message>`, so an expectation can name the
    # place and the keyword (`@records/0/title type`) or a message fragment.
    text = [f"@{'/'.join(map(str, e.absolute_path))} {e.validator}: {e.message}" for e in flatten(errs)]
    if not errs:
        t.fail(f"{key}: refuses {label}", "accepted")
    elif expect and not any(expect in line for line in text):
        t.fail(f"{key}: refuses {label}", f"refused, but not with {expect!r}: {describe(errs)}")
    else:
        counts[key][1] += 1
        t.ok(f"{key}: refuses {label}", f"({expect})" if expect else "")


def load_toml(path):
    return tomllib.loads(path.read_text())


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--root", default=str(HERE.parent.parent))
    ap.add_argument("--legacy-report", action="store_true")
    args = ap.parse_args()
    root = Path(args.root).resolve()
    schemas_dir = root / "schemas"
    t = Tally()

    # ---- 1. the registry, and the plants that show it refusing.
    print("== registry: frozen ids, schema files and $ids agree ==")
    registry = json.loads((schemas_dir / "format/registry.json").read_text())
    compat = (root / "docs/COMPATIBILITY.md").read_text()
    compat_rs = (root / "crates/openwarrant-cli/src/compat.rs").read_text()
    files = schema_files_under(schemas_dir)
    problems = registry_problems(compat, compat_rs, registry, files)
    if problems:
        for p in problems:
            t.fail("registry", p)
    else:
        frozen = frozen_from_compatibility(compat)
        t.ok("registry", f"{len(frozen)} frozen ids, {len(files)} schema files, {len(registry['entries'])} entries")

    planted = compat.replace(", war (the IR root)", ", war (the IR root), planted-record")
    plants = [
        ("a frozen id with no schema", planted, compat_rs, registry, files,
         "oh.war/planted-record/v1: frozen in docs/COMPATIBILITY.md and no schema file is registered"),
        ("a schema whose $id is not registered", compat, compat_rs, registry,
         {**files, "format/1.0/planted.json": "https://openwarrant.dev/schemas/format/1.0/planted.json"},
         "schemas/format/1.0/planted.json: $id"),
        ("a frozen record's schema dropped from the registry", compat, compat_rs,
         {**registry, "entries": [e for e in registry["entries"] if e["id"] != "oh.war/sas-revision/v1"]}, files,
         "oh.war/sas-revision/v1: frozen in docs/COMPATIBILITY.md and no schema file is registered"),
        ("a registered schema file that is gone", compat, compat_rs, registry,
         {k: v for k, v in files.items() if k != "format/records/amendment.v1.json"},
         "oh.war/amendment/v1: registry names schemas/format/records/amendment.v1.json, which does not exist"),
        ("a frozen id compat.rs does not read", planted, compat_rs, registry, files,
         "oh.war/planted-record/v1: frozen in docs/COMPATIBILITY.md and absent from compat.rs FROZEN"),
    ]
    for label, c, crs, reg, fs, want in plants:
        got = registry_problems(c, crs, reg, fs)
        if any(want in p for p in got):
            t.ok(f"registry refuses {label}", f"({want.split(':')[0]})")
        else:
            t.fail(f"registry refuses {label}", f"expected a problem naming {want!r}; got {got}")

    # ---- 2. every hand-written schema is a schema, and they resolve together.
    print("== hand-written schemas are draft 2020-12 ==")
    normative = [e for e in registry["entries"] if e["authority"] == "normative"]
    contents = {}
    for e in normative:
        path = schemas_dir / e["schema"]
        if not path.is_file():
            t.fail(f"{e['key']}: is a draft 2020-12 schema", f"schemas/{e['schema']} does not exist")
            continue
        schema = json.loads(path.read_text())
        try:
            Draft202012Validator.check_schema(schema)
        except Exception as err:  # noqa: BLE001 - reported by name
            t.fail(f"{e['key']}: is a draft 2020-12 schema", str(err)[:200])
            continue
        if "prose wins" not in schema.get("description", ""):
            t.fail(f"{e['key']}: says prose wins", "its description does not say the prose wins over the schema")
        else:
            t.ok(f"{e['key']}: is a draft 2020-12 schema that says prose wins")
        contents[e["$id"]] = schema
    resources = Registry().with_resources(
        [(sid, Resource.from_contents(s)) for sid, s in contents.items()]
    )
    # A schema that could not be loaded leaves its key mapped to None: every
    # instance of it then fails by name instead of the run stopping.
    validators = {e["key"]: (Draft202012Validator(contents[e["$id"]], registry=resources)
                             if e["$id"] in contents else None) for e in normative}
    counts = {key: [0, 0] for key in validators}

    # ---- 3. instances.
    print("== the repository's own records ==")
    for path in sorted((root / "docs/sas/revisions").glob("*.toml")):
        expect_accept(t, counts, "sas-revision", validators["sas-revision"],
                      path.relative_to(root).as_posix(), load_toml(path))
    amendments = sorted((root / "docs/warrants").glob("*/amendments/*.y*ml"))
    for path in amendments:
        label = path.relative_to(root).as_posix()
        try:
            doc = read_structured(path.read_text())
        except ReadError as err:
            t.fail(f"amendment: reads {label}", str(err))
            continue
        expect_accept(t, counts, "amendment", validators["amendment"], label, doc)
    # The built-in profiles of FORMAT-1.0 section 7.6 are files shipped with it.
    profiles = sorted((root / "profiles").glob("*.toml")) + sorted(
        p for p in (root / "packs").glob("*/*.toml") if p.name != "pack.toml") + sorted(
        (schemas_dir / "format/1.0/profiles").glob("*.toml"))
    for path in profiles:
        expect_accept(t, counts, "profile", validators["profile"],
                      path.relative_to(root).as_posix(), load_toml(path))
    for path in sorted((root / "packs").glob("*/pack.toml")):
        expect_accept(t, counts, "type-pack", validators["type-pack"],
                      path.relative_to(root).as_posix(), load_toml(path))
    legacy = sorted((root / "docs/warrants").glob("*/manifest.toml"))
    if legacy:
        expect_refuse(t, counts, "manifest", validators["manifest"],
                      f"{legacy[0].relative_to(root).as_posix()} (a legacy manifest lists atoms)",
                      load_toml(legacy[0]), "atoms")

    print("== doctored copies of real records (in memory) ==")
    revisions = sorted((root / "docs/sas/revisions").glob("*.toml"))
    changing = [p for p in revisions if load_toml(p).get("architecture_changing")]
    if revisions:
        base = load_toml(revisions[-1])
        doctored = [
            ("an accepted revision without its acceptance", lambda r: r.pop("acceptance"),
             "'acceptance' is a required property"),
            ("an acceptance by an agent", lambda r: r["acceptance"].update(actor_kind="agent"),
             "@acceptance/actor_kind enum"),
            ("a sha256 with a prefix", lambda r: r.update(sha256="sha256:" + r["sha256"]), "@sha256 pattern"),
            ("a state outside the two", lambda r: r.update(state="draft"), "@state enum"),
        ]
        for label, edit, expect in doctored:
            r = copy.deepcopy(base)
            edit(r)
            expect_refuse(t, counts, "sas-revision", validators["sas-revision"],
                          f"{revisions[-1].name} with {label}", r, expect)
    if changing:
        r = load_toml(changing[0])
        r["acceptance"].pop("adr_ref", None)
        expect_refuse(t, counts, "sas-revision", validators["sas-revision"],
                      f"{changing[0].name}, architecture-changing, without adr_ref", r,
                      "'adr_ref' is a required property")
    if amendments:
        base = read_structured(amendments[0].read_text())
        doctored = [
            ("an empty semantic diff", lambda a: a.update(semantic_diff=[]), "@semantic_diff minItems"),
            ("a band outside section 30", lambda a: a.update(band="minor"), "@band enum"),
            ("an element outside section 28.5", lambda a: a["semantic_diff"][0].update(element="vibes"),
             "@semantic_diff/0/element enum"),
            ("a manual revision with no authority", lambda a: (a.update(band="manual_revision"),
                                                              a.pop("governing_adr_or_policy", None)),
             "'governing_adr_or_policy' is a required property"),
            ("inadmissible work without a re-preflight", lambda a: a.update(
                artifact_admissibility="inadmissible", re_preflight_required="false"),
             "@re_preflight_required const"),
            ("a typed boolean (the reader never types a scalar)", lambda a: a.update(re_preflight_required=False),
             "@re_preflight_required enum"),
        ]
        for label, edit, expect in doctored:
            a = copy.deepcopy(base)
            edit(a)
            expect_refuse(t, counts, "amendment", validators["amendment"],
                          f"{amendments[0].relative_to(root).as_posix()} with {label}", a, expect)
    delivery = root / "profiles/delivery.toml"
    ticket = root / "profiles/ticket.toml"
    if delivery.exists() and ticket.exists():
        doctored = [
            (delivery, "an unknown member", lambda p: p.update(sections=["Intent"]), "('sections' was unexpected)"),
            (delivery, "a capability outside the closed set", lambda p: p["capabilities"].append("telepathy"),
             "'telepathy' is not one of"),
            (delivery, "a core profile that extends", lambda p: p.update(extends="decision"),
             "False schema does not allow 'decision'"),
            (ticket, "an extension with no approval stated", lambda p: p.pop("approved"),
             "'approved' is a required property"),
            (ticket, "a working form with no core roles", lambda p: p.pop("core_roles"),
             "'core_roles' is a required property"),
        ]
        for path, label, edit, expect in doctored:
            p = load_toml(path)
            edit(p)
            expect_refuse(t, counts, "profile", validators["profile"], f"{path.name} with {label}", p, expect)

    print("== the examples of docs/format/FORMAT-1.0.md ==")
    spec = (root / "docs/format/FORMAT-1.0.md").read_text()
    try:
        examples = spec_examples(spec)
    except ReadError as err:
        t.fail("FORMAT-1.0.md examples read", str(err))
        examples = []
    seen = {key for _, key, _ in examples}
    for key in ("frontmatter", "obligation", "record", "item", "check"):
        if key not in seen:
            t.fail(f"FORMAT-1.0.md has a {key} example", "none was found; the extraction drifted from the text")
    for label, key, instance in examples:
        expect_accept(t, counts, key, validators[key], label, instance)

    print("== fixtures (conformance/schemas/fixtures/<key>/) ==")
    for key in sorted(validators):
        for path in sorted((FIXTURES / key / "valid").glob("*")):
            instance = fixture_instance(path)
            expect_accept(t, counts, key, validators[key], f"fixture {path.relative_to(FIXTURES)}", instance)
        for path in sorted((FIXTURES / key / "invalid").glob("*.json")):
            wrapper = json.loads(path.read_text())
            expect_refuse(t, counts, key, validators[key],
                          f"fixture {path.relative_to(FIXTURES)}: {wrapper['why']}",
                          wrapper["instance"], wrapper["expect"])

    print("== every hand-written schema was seen accepting and refusing ==")
    for key, (accepted, refused) in sorted(counts.items()):
        if key == "common":
            continue
        if accepted and refused:
            t.ok(f"{key}: exercised", f"{accepted} accepted, {refused} refused")
        else:
            t.fail(f"{key}: exercised", f"{accepted} accepted, {refused} refused; each needs at least one")

    if args.legacy_report:
        legacy_report(root, validators)

    print(f"\n{t.passed} passed, {len(t.failed)} failed")
    return 0 if not t.failed else 1


def fixture_instance(path):
    """A valid fixture: JSON as is; Markdown read by the helper readers to the fixture's key."""
    if path.suffix == ".json":
        return json.loads(path.read_text())
    text = path.read_text()
    key = path.parent.parent.name
    if key in ("frontmatter", "ticket"):
        return read_frontmatter(text)[0]
    if key in ("record", "obligation", "check"):
        return read_record(text.rstrip("\n"))
    if key == "item":
        return read_item(text.rstrip("\n"))
    if key in ("manifest", "profile"):
        return tomllib.loads(text)
    if key == "amendment":
        return read_structured(text)
    raise SystemExit(f"{path}: no reader for a {path.suffix} fixture of {key}")


def legacy_report(root, validators):
    """Section 12 lowerings of legacy manifests to frontmatter. A report, not a check."""
    print("\n== report: legacy manifests lowered to Format 1.0 frontmatter (section 12) ==")
    print("   assumption: `summary` is the legacy `title`; the specification does not say where a")
    print("   lowered work document's REQUIRED summary comes from.")
    v = validators["frontmatter"]
    tally = {}
    for path in sorted((root / "docs/warrants").glob("*/manifest.toml")):
        m = load_toml(path)
        fm = {"format": "openwarrant/1.0", "profile": m["profile"], "alias": m["local_alias"],
              "uuid": m["uuid"], "summary": m["title"]}
        if "assurance_level" in m:
            fm["assurance_level"] = m["assurance_level"]
        if m.get("enterprise_id"):
            fm["kf.enterprise_id"] = m["enterprise_id"]
        report_one(v, path, root, fm, tally)
    for path in sorted((root / "docs/tickets").glob("*/manifest.toml")):
        m = load_toml(path)
        fm = {"format": "openwarrant/1.0", "profile": m["profile"], "alias": m["id"],
              "uuid": m["uuid"], "summary": m["title"], "priority": str(m.get("priority", 2)),
              "created_at": m["created_at"], "created_by": m["created_by"]}
        for key in ("type", "labels", "imported_from", "promoted_to", "part_of", "issue", "issue_url", "due"):
            if key in m:
                fm[key] = m[key] if isinstance(m[key], list) else str(m[key])
        report_one(v, path, root, fm, tally)
    for why, paths in sorted(tally.items()):
        print(f"   {len(paths):4d}  {why}")
        for p in paths[:3]:
            print(f"         e.g. {p}")


def report_one(v, path, root, fm, tally):
    errs = list(v.iter_errors(fm))
    rel = path.relative_to(root).as_posix()
    if not errs:
        tally.setdefault("accepted", []).append(rel)
    for e in errs:
        why = f"{'/'.join(map(str, e.absolute_path)) or '<root>'} {e.validator}: {e.message[:90]}"
        why = re.sub(r"'[^']{20,}'", "'…'", why)
        tally.setdefault(why, []).append(rel)


if __name__ == "__main__":
    sys.exit(main())
