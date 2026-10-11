#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""A minimal second reader of OpenWarrant Format 1.0 (docs/format/FORMAT-1.0.md).

Written from the specification alone, independently of `war`, so that the
conformance corpus (conformance/format/) is checked by two readers that share
no code. Standard library only. It implements the reader class of section 1
for sections 3 to 9: parse, refuse, UNKNOWN, the model's records and
relations, and `content_digest` (RFC 8785). It is not a checker of profiles
beyond the built-in ones, and not a context compiler.

    reader.py <repository root>

prints one JSON object on stdout:

    {"records": [...], "relations": [...], "findings": ["<rule> <path>:<line>", ...]}

Where the specification leaves a choice open, the choice taken is named in a
comment beginning `READING:`; conformance/format/README.md lists them.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import sys

READER_MAJOR, READER_MINOR = 1, 0

CORE_KINDS = {
    "part_of", "depends_on", "implements", "constrains", "evaluates", "supersedes",
    "supports", "refutes", "trades_off_against", "causes", "qualifies", "selected_over",
}
CORE_KEYS = {"format", "profile", "alias", "uuid", "summary", "paths", "requires"}
# Section 3.3: the keys the built-in profiles declare.
PROFILE_KEYS = {
    "delivery": {"assurance_level"},
    "decision": set(),
    "ticket": {"type", "priority", "labels", "created_at", "created_by", "imported_from"},
}
# Section 4.1: the sections each built-in profile requires.
REQUIRED_SECTIONS = {
    "delivery": ["Intent", "Basis", "Work order", "Assurance"],
    "decision": ["Intent", "Basis"],
    "ticket": ["Checklist"],
}
# Section 3.7 and 4.1: the core types whose fields are defined.
TYPE_FIELDS = {
    "obligation": {"scope", "evidence", "refusal", "gate", "scope_kind", "criticality"},
    "stage": {"executor", "tier", "budget_tokens", "context"},
}
AUTHORED_STATE = {"disposition", "verdict", "state", "status"}

REFUSALS = {
    "format.encoding", "format.frontmatter", "format.key-unknown", "format.heading",
    "format.title-missing", "format.attribute", "format.field-duplicate",
    "format.field-missing", "format.authored-state", "format.relation-kind",
    "format.relation-target", "format.item", "format.id-duplicate",
    "format.fence-unclosed", "format.section-missing", "format.document-duplicate",
    "format.manifest-restates", "format.unrecognised",
}
UNKNOWNS = {"format.newer-major", "format.newer-minor", "format.requires-unknown"}

WORD = r"[a-z][a-z0-9_-]*"
LABEL = r"[a-z0-9][a-z0-9_-]*"
DOTTED = rf"{LABEL}(?:\.{LABEL})+"
NAME = rf"(?:{DOTTED}|{WORD})"
RECORD_ID = r"[A-Z][A-Z0-9]*(?:-[A-Za-z0-9]+)+"
ITEM_ID = r"i-[0-9a-f]+"
ALIAS = rf"(?:{RECORD_ID}|t-[0-9a-f]+)"
UUID = r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"
REF = (
    rf"(?:war://{UUID}|md:[^#\s,]+#[a-z0-9-]+|{ALIAS}/(?:{RECORD_ID}|{ITEM_ID})|{ALIAS})"
)
TARGET_RE = re.compile(rf"({REF})(?:@(sha256:[0-9a-f]{{64}}))?")
FIELD_RE = re.compile(rf"({NAME}):(?: (.*))?")
RELATION_RE = re.compile(rf"({NAME}) (\S.*)")
HEAD_RE = re.compile(rf"({RECORD_ID}) · ({WORD}) — (.*)")
FENCE_OPEN = re.compile(r"(`{3,}|~{3,})(.*)")
BOX_RE = re.compile(r"\s*[-*] \[([^\]]*)\]")
ITEM_RE = re.compile(
    rf"\s*[-*] \[( |x|X)\] (.*?) ?\(({ITEM_ID})((?:, after (?:{ALIAS}/{ITEM_ID}|{ITEM_ID}|{ALIAS}))*)\)"
    r"((?: — done by [^,]+, \d{4}-\d{2}-\d{2}(?:: .*)?)?)"
)
IMPLEMENTS_RE = re.compile(rf"\bimplements ({REF})")


class Refused(Exception):
    def __init__(self, rule: str, line: int):
        super().__init__(rule)
        self.rule, self.line = rule, line


# ------------------------------------------------------------------ digests --

def jcs(value) -> bytes:
    """RFC 8785 for the values this model uses: objects, arrays, strings."""
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def domain_digest(domain: str, payload) -> str:
    return "sha256:" + hashlib.sha256(jcs({"digest_domain": domain, "payload": payload})).hexdigest()


def norm_body(body: str) -> str:
    """Section 9: CRLF to LF, trailing spaces and tabs off each line, trailing empty lines off."""
    lines = [ln.rstrip(" \t") for ln in body.replace("\r\n", "\n").split("\n")]
    while lines and lines[-1] == "":
        lines.pop()
    return "\n".join(lines)


def content_digest(rec: dict, relations: list[dict]) -> str:
    out = sorted(
        ({k: v for k, v in r.items() if k != "from" and not k.startswith("_")} for r in relations),
        key=lambda r: (r["kind"], r["to"], r.get("to_revision", "")),
    )
    payload = {
        "id": rec["id"], "type": rec["type"], "title": rec.get("title", ""),
        "fields": rec["fields"], "relations": out, "body": norm_body(rec["body"]),
    }
    return domain_digest("oh.war/record-content/v1", payload)


# ------------------------------------------------------------------- lines --

def split_lines(text: str):
    """(line number, content without terminator) for each line."""
    out, n = [], 1
    for raw in text.splitlines(keepends=True):
        out.append((n, raw.rstrip("\r\n") if raw.endswith("\n") else raw))
        n += 1
    return out


def slug(name: str) -> str:
    """3.4: ASCII letters lowercased, digits kept, every other run one `-`, trimmed."""
    s = "".join(c.lower() if c.isascii() and c.isalnum() else "-" for c in name)
    return re.sub(r"-+", "-", s).strip("-")


# ------------------------------------------------------------- frontmatter --

def scalar(text: str, line: int) -> str:
    if text.startswith('"'):
        m = re.fullmatch(r'"((?:[^"\\]|\\["\\])*)"', text)
        if not m:
            raise Refused("format.frontmatter", line)
        return re.sub(r'\\(["\\])', r"\1", m.group(1))
    if text.startswith("'"):
        m = re.fullmatch(r"'((?:[^']|'')*)'", text)
        if not m:
            raise Refused("format.frontmatter", line)
        return m.group(1).replace("''", "'")
    if not text or text[0] in "[{&*!|>%@`" or text.startswith("- ") or text == "-":
        raise Refused("format.frontmatter", line)
    if ": " in text or " #" in text:
        raise Refused("format.frontmatter", line)
    return text


def frontmatter(lines):
    """(pairs as [(key, value, line)], index of the first body line)."""
    if not lines or lines[0][1] != "---":
        return None, 0
    if not any(ln == "---" for _, ln in lines[1:]):
        raise Refused("format.frontmatter", 1)  # the closing fence is missing
    pairs, seen, list_key = [], set(), None
    for i in range(1, len(lines)):
        n, ln = lines[i]
        if ln == "---":
            return pairs, i + 1
        if ln.strip() == "" or ln.startswith("#"):
            continue
        if ln.startswith("  - "):
            if list_key is None:
                raise Refused("format.frontmatter", n)
            pairs[-1][1].append(scalar(ln[4:], n))
            continue
        list_key = None
        m = re.fullmatch(rf"({NAME}):(?: (.*))?", ln)
        if not m:
            raise Refused("format.frontmatter", n)
        key, val = m.group(1), m.group(2)
        if key in seen:
            raise Refused("format.frontmatter", n)
        seen.add(key)
        if val is None or val == "":
            pairs.append((key, [], n))
            list_key = key
        else:
            pairs.append((key, scalar(val, n), n))
    raise Refused("format.frontmatter", 1)


# --------------------------------------------------------------- documents --

class Doc:
    def __init__(self, source: str):
        self.source = source
        self.findings: list[tuple[str, int]] = []
        self.records: list[dict] = []
        self.relations: list[dict] = []
        self.refused = False
        self.unknown = False
        self.alias = None
        self.uuid = None
        self.newer_minor = False

    def find(self, rule, line):
        self.findings.append((rule, line))
        if rule in REFUSALS:
            self.refused = True
        if rule in ("format.newer-major", "format.requires-unknown"):
            # The whole document is UNKNOWN; a newer minor's UNKNOWN is per unit.
            self.unknown = True


def read_document(source: str, data: bytes, work: str | None) -> Doc:
    """work: None for a records document, else "warrant" or "ticket" (the place)."""
    doc = Doc(source)
    try:
        parse(doc, data, work)
    except Refused as r:
        doc.find(r.rule, r.line)
    return doc


def parse(doc: Doc, data: bytes, work):
    # 3.1 encoding
    try:
        text = data.decode("utf-8")
    except UnicodeDecodeError as e:
        raise Refused("format.encoding", data[: e.start].count(b"\n") + 1)
    if "\x00" in text:
        raise Refused("format.encoding", text[: text.index("\x00")].count("\n") + 1)
    bom = 0
    if text.startswith("﻿"):
        text, bom = text[1:], 3
    lines = split_lines(text)
    # byte offset of each line start (for nothing normative here but spans)
    pairs, body_at = frontmatter(lines)
    if pairs is None:
        # Section 6: no `format` -> a legacy encoding or `format.unrecognised`.
        raise Refused("format.unrecognised", 1)
    fm = {k: (v, n) for k, v, n in pairs}
    if "format" not in fm:
        raise Refused("format.unrecognised", 1)
    fv, fline = fm["format"]
    m = re.fullmatch(r"openwarrant/(\d+)\.(\d+)", fv if isinstance(fv, str) else "")
    if not m:
        raise Refused("format.unrecognised", fline)
    major, minor = int(m.group(1)), int(m.group(2))
    if major > READER_MAJOR:
        doc.find("format.newer-major", fline)
        return
    newer_minor = major == READER_MAJOR and minor > READER_MINOR
    doc.newer_minor = newer_minor
    profile = fm.get("profile", (None, 1))[0]
    if profile is None:
        raise Refused("format.field-missing", 1)
    if "requires" in fm:
        req, rline = fm["requires"]
        req = req if isinstance(req, list) else [req]
        if req:
            # This reader understands no extension (7.3).
            doc.find("format.requires-unknown", rline)
            return
    allowed = CORE_KEYS | PROFILE_KEYS.get(profile, set())
    fields, ext = {}, {}
    for k, v, n in pairs:
        if "." in k:
            ext[k] = v
            continue
        if k not in allowed and profile in PROFILE_KEYS:
            if newer_minor:
                doc.find("format.newer-minor", n)
                continue
            raise Refused("format.key-unknown", n)
        # READING: `format` is the encoding's version, not the work's meaning;
        # it is left out of the document record's fields so that section 9's
        # "same content_digest in every encoding" can hold.
        if k != "format":
            fields[k] = v
    is_work = "alias" in fm
    if work is not None and not is_work:
        raise Refused("format.field-missing", 1)
    if is_work:
        for key in ("uuid", "summary"):
            if key not in fm:
                raise Refused("format.field-missing", 1)
        doc.alias = fm["alias"][0]
        doc.uuid = fm["uuid"][0]

    # fences first (3.9), then units (3.4)
    body_lines = lines[body_at:]
    in_fence = []  # per body line: True when inside (or opening/closing) a fence
    fence = None
    for n, ln in body_lines:
        if fence is None:
            fm_ = FENCE_OPEN.fullmatch(ln)
            if fm_ and not (fm_.group(1)[0] == "`" and "`" in fm_.group(2)):
                fence = (fm_.group(1)[0], len(fm_.group(1)), n)
                in_fence.append(True)
                continue
            in_fence.append(False)
        else:
            in_fence.append(True)
            c, k, _ = fence
            mm = re.fullmatch(r"(`{3,}|~{3,})[ \t]*", ln)
            if mm and mm.group(1)[0] == c and len(mm.group(1)) >= k:
                fence = None
    if fence is not None:
        raise Refused("format.fence-unclosed", fence[2])

    # split into preamble + units
    units, cur = [], None
    preamble = []
    for (n, ln), fenced in zip(body_lines, in_fence):
        if not fenced and ln.startswith("## "):
            cur = {"line": n, "head": ln[3:], "lines": []}
            units.append(cur)
        elif cur is None:
            preamble.append((n, ln, fenced))
        else:
            cur["lines"].append((n, ln, fenced))

    title = None
    for n, ln, fenced in preamble:
        if not fenced and ln.startswith("# "):
            title = ln[2:]
            break
    alias = doc.alias
    scope = (alias + "/") if is_work else ""

    def gid(local):
        return scope + local

    records, rels = [], []
    if is_work:
        # READING: the document record's body is the preamble without its
        # `# ` title line (the title is not repeated).
        pre = []
        dropped = False
        for n, ln, fenced in preamble:
            if not dropped and not fenced and ln.startswith("# "):
                dropped = True
                continue
            pre.append(ln)
        records.append({
            "id": alias, "type": "ticket" if profile == "ticket" else "warrant",
            "title": title or "", "fields": fields, "ext": ext,
            "body": "\n".join(pre), "line": 1,
        })

    seen_ids: dict[str, int] = {}
    section_names = []
    item_ids: set[str] = set()

    def claim(i, n):
        if i in seen_ids:
            raise Refused("format.id-duplicate", n)
        seen_ids[i] = n

    def unit(u):
        """One unit: its records and relations, all or nothing."""
        head, n = u["head"], u["line"]
        urecs, urels = [], []
        if " · " in head:
            m = HEAD_RE.fullmatch(head)
            if not m:
                m2 = re.fullmatch(rf"({RECORD_ID}) · ({WORD})(?: —.*)?", head)
                if m2 and len(m2.group(1)) <= 64:
                    raise Refused("format.title-missing", n)
                raise Refused("format.heading", n)
            rid, rtype, rtitle = m.groups()
            if len(rid.encode()) > 64:
                raise Refused("format.heading", n)
            # READING: an empty or over-long title is `format.title-missing`,
            # the rule section 3.4 names for the sentence that sets both.
            if rtitle.strip() == "" or len(rtitle.encode()) > 120:
                raise Refused("format.title-missing", n)
            rec, rrels = record_unit(doc, gid(rid), rtype, rtitle, u["lines"], n)
            urecs.append(rec)
            urels.extend(rrels)
            body_items = u["lines"][rec.pop("_body_at"):]
            claim_ids = [rid]
        else:
            s = slug(head)
            if not s:
                raise Refused("format.heading", n)
            sid = "S-" + s
            # READING: a section's body skips one blank line after its heading,
            # as a record with no attribute block does (3.5).
            lines_ = [ln for _, ln, _ in u["lines"]]
            if lines_ and lines_[0].strip() == "":
                lines_ = lines_[1:]
            urecs.append({
                "id": gid(sid), "type": "section", "title": head,
                "fields": {}, "ext": {}, "body": "\n".join(lines_), "line": n,
            })
            body_items = u["lines"]
            claim_ids = [sid]
        new_items = set()
        for ln_n, ln, fenced in body_items:
            if not fenced:
                item(doc, ln_n, ln, gid, item_ids | new_items, new_items, urecs, urels)
        for ln_n, ln, fenced in u["lines"]:
            # 3.7: a `### OBL-` heading in a Format 1.0 document is prose.
            if not fenced and re.match(r"### OBL-", ln):
                doc.find("format.legacy-obligation", ln_n)
        for i in claim_ids:
            claim(i, n)
        if " · " not in head:
            section_names.append(head)
        item_ids.update(new_items)
        records.extend(urecs)
        rels.extend(urels)

    for u in units:
        if not newer_minor:
            unit(u)
            continue
        try:
            unit(u)
        except Refused:
            doc.find("format.newer-minor", u["line"])

    if is_work:
        required = REQUIRED_SECTIONS.get(profile, [])
        have = {slug(s) for s in section_names}
        for s in required:
            if slug(s) not in have:
                raise Refused("format.section-missing", 1)
        if profile == "delivery" and not any(r["type"] == "obligation" for r in records):
            raise Refused("format.section-missing", 1)

    # resolve local targets
    local = {r["id"] for r in records}
    for r in rels:
        t = r["to"]
        if is_work and "/" not in t and gid(t) in local:
            r["to"] = gid(t)
    doc.records, doc.relations = records, rels


def record_unit(doc, rid, rtype, title, lines, head_line):
    fields, ext, rels = {}, {}, []
    last = None
    i = 0
    while i < len(lines):
        n, ln, fenced = lines[i]
        if ln.strip() == "":
            i += 1
            break
        if ln.startswith("  "):
            if last is None:
                raise Refused("format.attribute", n)
            store, key = last
            store[key] = (store[key] + "\n" if store[key] != "" else "") + ln[2:]
            i += 1
            continue
        last = None
        fm = FIELD_RE.fullmatch(ln)
        if fm:
            key, val = fm.group(1), fm.group(2) or ""
            store = ext if "." in key else fields
            if key in store:
                raise Refused("format.field-duplicate", n)
            if "." not in key:
                if rtype == "obligation" and key in AUTHORED_STATE:
                    raise Refused("format.authored-state", n)
                allowed = TYPE_FIELDS.get(rtype)
                if allowed is not None and key not in allowed:
                    if doc.newer_minor:
                        doc.find("format.newer-minor", n)
                        i += 1
                        continue
                    raise Refused("format.key-unknown", n)
            store[key] = val
            last = (store, key)
            i += 1
            continue
        rm = RELATION_RE.fullmatch(ln)
        if rm:
            kind, targets = rm.groups()
            if kind not in CORE_KINDS and "." not in kind:
                raise Refused("format.relation-kind", n)
            for t in targets.split(", "):
                tm = TARGET_RE.fullmatch(t)
                if not tm:
                    raise Refused("format.relation-target", n)
                rel = {"from": rid, "kind": kind, "to": tm.group(1)}
                if tm.group(2):
                    rel["to_revision"] = tm.group(2)
                rel["_line"] = n
                rels.append(rel)
            i += 1
            continue
        raise Refused("format.attribute", n)
    body = [ln for _, ln, _ in lines[i:]]
    if rtype == "obligation":
        for req in ("scope", "evidence"):
            if req not in fields:
                raise Refused("format.field-missing", head_line)
        if "refusal" not in fields:
            doc.find("format.refusal-missing", head_line)
    rec = {"id": rid, "type": rtype, "title": title, "fields": fields, "ext": ext,
           "body": "\n".join(body), "line": head_line, "_body_at": i}
    return rec, rels


def item(doc, n, ln, gid, item_ids, new_items, records, rels):
    b = BOX_RE.match(ln)
    if not b:
        return
    if b.group(1) not in (" ", "x", "X"):
        raise Refused("format.item", n)
    m = ITEM_RE.fullmatch(ln)
    if not m:
        # READING: a box line whose text does not end in an id group (and an
        # optional `done` suffix) has no id group.
        doc.find("format.item-id-missing", n)
        return
    _, text, iid, after, _done = m.groups()
    if iid in item_ids:
        raise Refused("format.id-duplicate", n)
    new_items.add(iid)
    rid = gid(iid)
    irels = []
    for ref in re.findall(r", after (\S+?)(?=,|$)", after):
        to = gid(ref) if re.fullmatch(ITEM_ID, ref) else ref
        irels.append({"from": rid, "kind": "depends_on", "to": to, "_line": n})
    for ref in IMPLEMENTS_RE.findall(text):
        irels.append({"from": rid, "kind": "implements", "to": ref, "_line": n})
    rels.extend(irels)
    # READING: an item is a record of type `item` whose title is its text;
    # its box and `done by` suffix are state, not content (2: a state is
    # never authored, it is computed).
    records.append({"id": rid, "type": "item", "title": text, "fields": {}, "ext": {},
                    "body": "", "line": n})


# -------------------------------------------------------------- repository --

def is_alias(name: str) -> bool:
    return re.fullmatch(ALIAS, name) is not None


def discover(root: str):
    """(source, place) for every document section 4.1 places, plus record documents."""
    out = []
    for place, base, inner in (("warrant", "docs/warrants", "warrant.md"),
                               ("ticket", "docs/tickets", "ticket.md")):
        d = os.path.join(root, base)
        if not os.path.isdir(d):
            continue
        for name in sorted(os.listdir(d)):
            p = os.path.join(d, name)
            if os.path.isdir(p) and os.path.isfile(os.path.join(p, inner)):
                out.append((f"{base}/{name}/{inner}", place, name))
            elif name.endswith(".md") and is_alias(name[:-3]):
                out.append((f"{base}/{name}", place, name[:-3]))
    # READING: section 4 does not say where a document that is not a work
    # document lives; this reader takes `docs/records/<area>/*.md`.
    rd = os.path.join(root, "docs/records")
    if os.path.isdir(rd):
        for area in sorted(os.listdir(rd)):
            ap = os.path.join(rd, area)
            if not os.path.isdir(ap):
                continue
            for name in sorted(os.listdir(ap)):
                if name.endswith(".md"):
                    out.append((f"docs/records/{area}/{name}", None, None))
    return out


def manifest_check(root: str, source: str, doc: Doc):
    """4.2: a manifest.toml beside warrant.md must not list atoms or restate a frontmatter key."""
    if not source.endswith(("/warrant.md", "/ticket.md")):
        return
    mp = os.path.join(root, os.path.dirname(source), "manifest.toml")
    if not os.path.isfile(mp):
        return
    import tomllib
    with open(mp, "rb") as f:
        try:
            man = tomllib.load(f)
        except tomllib.TOMLDecodeError:
            return
    defined = CORE_KEYS | {k for ks in PROFILE_KEYS.values() for k in ks}
    if "atoms" in man or any(k in defined for k in man):
        rel = os.path.relpath(mp, root)
        doc.findings.append(("format.manifest-restates", 1, rel))
        doc.refused = True


def read_repository(root: str) -> dict:
    docs, findings = [], []
    by_alias: dict[tuple, str] = {}
    for source, place, alias in discover(root):
        with open(os.path.join(root, source), "rb") as f:
            data = f.read()
        if place is None and not re.match(rb"(\xef\xbb\xbf)?---\r?\n(?:.*\r?\n)*?format:", data):
            continue
        if place is not None:
            key = (place, alias)
            if key in by_alias:
                # READING: the form found second in byte order of path is the one reported.
                findings.append(f"format.document-duplicate {source}:1")
                docs = [d for d in docs if d.source != by_alias[key]]
                continue
            by_alias[key] = source
        d = read_document(source, data, place)
        manifest_check(root, source, d)
        docs.append(d)
    records, relations = [], []
    for d in docs:
        for f in d.findings:
            if len(f) == 3:
                findings.append(f"{f[0]} {f[2]}:{f[1]}")
            else:
                findings.append(f"{f[0]} {d.source}:{f[1]}")
        if d.refused or d.unknown:
            continue
        rel_by_from: dict[str, list] = {}
        for r in d.relations:
            rel_by_from.setdefault(r["from"], []).append(r)
        for rec in d.records:
            rec["content_digest"] = content_digest(rec, rel_by_from.get(rec["id"], []))
            rec["source"] = d.source
            records.append(rec)
        relations.extend(d.relations)
    # 3.6: a target naming no record is a warning; the relation is kept.
    known = {r["id"] for r in records}
    uuids = {d.uuid for d in docs if d.uuid and not (d.refused or d.unknown)}
    srcs = {r["id"]: r["source"] for r in records}
    for r in relations:
        t = r["to"]
        if t in known or (t.startswith("war://") and t[6:] in uuids):
            continue
        findings.append(f"format.target-unknown {srcs[r['from']]}:{r['_line']}")
    out_records = [
        {k: r[k] for k in ("id", "type", "title", "fields", "ext", "content_digest")}
        for r in sorted(records, key=lambda r: r["id"].encode())
    ]
    out_rel = sorted(
        ({k: v for k, v in r.items() if not k.startswith("_")} for r in relations),
        key=lambda r: (r["from"], r["kind"], r["to"], r.get("to_revision", "")),
    )
    return {"records": out_records, "relations": out_rel, "findings": sorted(set(findings))}


def main(argv):
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 64
    json.dump(read_repository(argv[1]), sys.stdout, ensure_ascii=False, sort_keys=True, indent=1)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
