#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""A minimal second reader of OpenWarrant Format 1.0 (docs/format/FORMAT-1.0.md).

Written from the specification alone, independently of `war`, so that the
conformance corpus (conformance/format/) is checked by two readers that share
no code. Standard library only. It implements the reader class of section 1
for sections 3 to 9 of draft 3: parse, refuse, UNKNOWN, the model's records
and relations, `content_digest` (RFC 8785), profile data (7.4 to 7.6, the
built-in profiles loaded from schemas/format/1.0/profiles/), specifications
(4.5) and the warnings of 13.2. context.py and tracker.py build on
`load_repository`.

    reader.py <repository root>

prints one JSON object on stdout:

    {"records": [...], "relations": [...], "findings": ["<rule> <path>:<line>", ...]}

Where the specification leaves a choice open, the choice taken is named in a
comment beginning `READING:`; conformance/format/README.md lists them.
"""

from __future__ import annotations

import copy
import hashlib
import json
import os
import re
import sys
import tomllib

READER_MAJOR, READER_MINOR = 1, 0
HERE = os.path.dirname(os.path.abspath(__file__))
# Section 7.6: the built-in profiles are files shipped with the document.
BUILTIN_PROFILES = os.path.join(os.path.dirname(os.path.dirname(HERE)),
                                "schemas", "format", "1.0", "profiles")

CORE_KINDS = (
    "part_of", "depends_on", "implements", "constrains", "evaluates", "supersedes",
    "supports", "refutes", "trades_off_against", "causes", "qualifies", "selected_over",
)
CORE_KEYS = {"format", "profile", "alias", "uuid", "summary", "paths", "requires"}
# Section 3.3: frontmatter keys that are relations of the document's own record.
RELATION_KEYS = {"implements", "depends_on", "supersedes", "part_of"}
# Section 2: the core types.
CORE_TYPES = {
    "section", "item", "warrant", "ticket", "obligation", "milestone", "stage", "check", "kpi",
    "spec", "requirement", "constraint", "outcome", "decision", "option", "consequence",
    "interface", "term", "document",
}
# The fields the core text defines per type: 3.7 (obligation), 4.1 (stage),
# 12 (check, kpi). READING: 4.5's `requirement` and `constraint` fields are
# read as the `spec` profile's (schemas/format/1.0/profiles/spec.toml), so a
# requirement in another document declares none.
_CHECK_FIELDS = {"run", "on", "timeout", "gating",
                 "reopen"}
CORE_FIELDS = {
    "obligation": {"scope", "evidence", "refusal", "gate", "scope_kind", "criticality"},
    "stage": {"executor", "tier", "budget_tokens", "context"},
    "milestone": {"min"},
    "check": _CHECK_FIELDS,
    "kpi": _CHECK_FIELDS | {"direction", "target", "mode"},
}
# REQUIRED by the core text under any profile.
CORE_REQUIRED = {"obligation": ["scope", "evidence"], "check": ["run"], "kpi": ["run", "direction"]}
AUTHORED_STATE = {"disposition", "verdict", "state", "status"}
# Section 7.6 and 11.1.
CAPABILITIES = {"structure", "links", "claims", "checks", "stages", "acceptance", "evidence",
                "verification", "authorization", "resolution"}
FIXED_STATES = {"draft", "open", "in_progress", "done", "blocked", "claimed", "superseded",
                "failing", "accepted", "partially_accepted", "authorized", "verified", "resolved"}
BUILTIN_PRESETS = {"agent", "agent.min", "human", "review", "resume"}
SHAPES = {"text", "line", "int", "number", "id", "ids", "gate", "date", "enum", "bool"}
# Appendix D.
REGISTERED_PREFIXES = {"kf", "katana", "liminal", "blut", "contractor", "legacy", "x"}

REFUSALS = {
    "format.encoding", "format.frontmatter", "format.key-unknown", "format.heading",
    "format.title-missing", "format.attribute", "format.field-duplicate",
    "format.field-missing", "format.authored-state", "format.relation-kind",
    "format.relation-target", "format.item", "format.id-duplicate",
    "format.fence-unclosed", "format.section-missing", "format.document-duplicate",
    "format.manifest-restates", "format.unrecognised", "format.field-shape", "profile.invalid",
}
UNKNOWNS = {"format.newer-major", "format.newer-minor", "format.requires-unknown",
            "format.profile-unknown"}

WORD = r"[a-z][a-z0-9_-]*"
LABEL = r"[a-z0-9][a-z0-9_-]*"
DOTTED = rf"{LABEL}(?:\.{LABEL})+"
NAME = rf"(?:{DOTTED}|{WORD})"
RECORD_ID = r"[A-Z][A-Z0-9]*(?:-[A-Za-z0-9]+)+"
ITEM_ID = r"i-[0-9a-f]{3,16}"
ALIAS = rf"(?:{RECORD_ID}|t-[0-9a-f]+)"
# Appendix A `path`: no empty, `.` or `..` segment.
DOC_PATH = r"(?!/)(?:(?!\.\.?(?:/|[\s,@#]|$))[^/\s,@#\\]+)(?:/(?!\.\.?(?:/|[\s,@#]|$))[^/\s,@#\\]+)*"
UUID = r"[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}"
REF = (
    rf"(?:war://{UUID}|md:[^#\s,]+#[a-z0-9-]+|doc:{DOC_PATH}"
    rf"|{ALIAS}/(?:{RECORD_ID}|{ITEM_ID})|{ALIAS}|{ITEM_ID})"
)
TARGET_RE = re.compile(rf"({REF})(?:@(sha256:[0-9a-f]{{64}}))?")
FIELD_RE = re.compile(rf"({NAME}):(?: (.*))?")
RELATION_RE = re.compile(rf"({NAME}) (\S.*)")
TYPE = rf"(?:{WORD}|{DOTTED})"
# Section 3.4: the Unicode separators, or the ASCII ones; one spelling per heading.
HEAD_UNICODE = re.compile(rf"({RECORD_ID}) · ({TYPE}) — (.*)")
HEAD_ASCII = re.compile(rf"({RECORD_ID}) \| ({TYPE}) -- (.*)")
HEAD_UNICODE_UNTITLED = re.compile(rf"({RECORD_ID}) · ({TYPE})(?: —.*)?")
HEAD_ASCII_UNTITLED = re.compile(rf"({RECORD_ID}) \| ({TYPE})(?: --.*)?")
ASCII_START = re.compile(rf"{RECORD_ID} \| ")
FENCE_OPEN = re.compile(r"(`{3,}|~{3,})(.*)")
BOX_RE = re.compile(r"\s*[-*] \[([^\]]*)\]")
LEVELS = ("observed", "independent", "signed")
DONE_SUFFIX = (r" — done by ([^,]+), (\d{4}-\d{2}-\d{2})"
               r"(?: \[(observed|independent|signed)\])?(?:: (.*))?")
ITEM_RE = re.compile(
    rf"(\s*[-*] )\[( |x|X)\] (.*?) ?\(({ITEM_ID})((?:, after (?:{ALIAS}/{ITEM_ID}|{ITEM_ID}|{ALIAS}))*)\)"
    rf"((?:{DONE_SUFFIX})?)"
)
DONE_RE = re.compile(DONE_SUFFIX)
IMPLEMENTS_RE = re.compile(rf"\bimplements ({REF})")
GATE_RE = re.compile(r"gate://[a-z0-9][a-z0-9_-]*(?:\.[a-z0-9][a-z0-9_-]*)*@\d+\.\d+\.\d+(?:-\S+)?")
SECTION_REF_RE = re.compile(r"§")
SENTENCE_BREAK_RE = re.compile(r"[.!?] ")


class Refused(Exception):
    def __init__(self, rule: str, line: int, path: str | None = None):
        super().__init__(rule)
        self.rule, self.line, self.path = rule, line, path


# ------------------------------------------------------------------ digests --

def jcs(value) -> bytes:
    """RFC 8785 for the values this model uses: objects, arrays, strings, integers."""
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()


def sha256(data: bytes) -> str:
    return "sha256:" + hashlib.sha256(data).hexdigest()


def domain_digest(domain: str, payload) -> str:
    return sha256(jcs({"digest_domain": domain, "payload": payload}))


def basis_digest(pairs) -> str:
    """Section 8: sha256 of the RFC 8785 bytes of the array of [id, revision] pairs."""
    return sha256(jcs([list(p) for p in pairs]))


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

def split_lines(text: str, base: int = 0):
    """(line number, content without terminator, byte start, byte end) for each line."""
    out, off = [], base
    for n, raw in enumerate(re.findall(r"[^\n]*\n|[^\n]+\Z", text), 1):
        size = len(raw.encode())
        if raw.endswith("\r\n"):
            content = raw[:-2]
        elif raw.endswith("\n"):
            content = raw[:-1]
        else:
            content = raw
        out.append((n, content, off, off + size))
        off += size
    return out


def slug(name: str) -> str:
    """3.4: ASCII letters lowercased, digits kept, every other run one `-`, trimmed."""
    s = "".join(c.lower() if c.isascii() and c.isalnum() else "-" for c in name)
    return re.sub(r"-+", "-", s).strip("-")


def prefix_registered(name: str) -> bool:
    """7.1 and Appendix D: a registered prefix, or a reverse domain name: a
    name of three or more labels whose first label is letters only."""
    labels = name.split(".")
    return labels[0] in REGISTERED_PREFIXES or (len(labels) >= 3 and labels[0].isalpha())


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
    if not any(ln[1] == "---" for ln in lines[1:]):
        raise Refused("format.frontmatter", 1)  # the closing fence is missing
    pairs, seen, list_key = [], set(), None
    for i in range(1, len(lines)):
        n, ln = lines[i][0], lines[i][1]
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


# ---------------------------------------------------------------- profiles --

PROFILE_KEYS = {"schema", "format", "name", "extends", "form", "note", "capabilities", "frontmatter",
                "sections", "records", "fields", "relations", "states", "ticks", "presets"}
PRESET_KEYS = {"name", "steps", "depth", "budget", "omit_types", "omit_fields", "editable",
               "reads_state"}
STEP_RE = re.compile(r"(in|out):(" + "|".join(CORE_KINDS) + r")(?::[a-z][a-z0-9_-]*)?")


class Profile:
    """Profile data (7.5), after `extends`."""

    def __init__(self, data: dict):
        fm = data.get("frontmatter", {})
        sec = data.get("sections", {})
        self.name = data["name"]
        self.form = data.get("form", "work")
        self.capabilities = set(data.get("capabilities", []))
        self.fm_required = list(fm.get("required", []))
        self.fm_optional = list(fm.get("optional", []))
        self.sec_required = list(sec.get("required", []))
        self.sec_optional = list(sec.get("optional", []))
        self.sec_closed = bool(sec.get("closed", False))
        self.types = list(data.get("records", {}).get("types", []))
        self.types_closed = bool(data.get("records", {}).get("closed", False))
        self.fields = data.get("fields", {})
        self.relations = data.get("relations", {})
        self.states = data.get("states", [])
        self.ticks = data.get("ticks", {})
        self.presets = data.get("presets", [])

    def field_table(self, rtype: str) -> dict:
        return self.fields.get(rtype, {})


def profile_problem(data: dict, stem: str) -> str | None:
    """7.5: why a profile file is refused (`profile.invalid`), or None."""
    def strings(v):
        return isinstance(v, list) and all(isinstance(x, str) for x in v)

    unknown = set(data) - PROFILE_KEYS
    if unknown:
        return f"unknown key {sorted(unknown)[0]}"
    if data.get("schema") != "oh.war/profile/v1":
        return "schema is not oh.war/profile/v1"
    if data.get("format") != "openwarrant/1.0":
        return "format is not openwarrant/1.0"
    if data.get("name") != stem:
        return "name is not the file stem"
    if data.get("form", "work") not in ("work", "document"):
        return "form is neither work nor document"
    caps = data.get("capabilities", [])
    if not strings(caps) or set(caps) - CAPABILITIES:
        return "a capability outside 7.6"
    if (data.get("extends") is None) and (problem := capability_problem(set(caps))):
        return problem
    for table, keys in (("frontmatter", {"required", "optional"}),
                        ("sections", {"required", "optional", "closed"}),
                        ("records", {"types", "closed"}), ("relations", {"allow", "require"}),
                        ("ticks", {"item", "milestone", "types"})):
        t = data.get(table, {})
        if not isinstance(t, dict) or set(t) - keys:
            return f"unknown key in [{table}]"
    for kind in data.get("relations", {}).get("allow", []):
        if kind not in CORE_KINDS:
            return "relations.allow names a kind outside 3.6"
    for rtype, table in data.get("fields", {}).items():
        if not isinstance(table, dict):
            return f"[fields.{rtype}] is not a table"
        for fname, spec in table.items():
            if not isinstance(spec, dict) or set(spec) - {"required", "shape", "values"}:
                return f"unknown key in [fields.{rtype}].{fname}"
            if spec.get("shape", "text") not in SHAPES:
                return f"[fields.{rtype}].{fname} has an unknown shape"
            if spec.get("shape") == "enum" and not strings(spec.get("values")):
                return f"[fields.{rtype}].{fname} is an enum without values"
    for st in data.get("states", []):
        if not isinstance(st, dict) or set(st) - {"name", "refines", "note"}:
            return "unknown key in [[states]]"
        if st.get("refines") not in FIXED_STATES:
            return "a refines naming no fixed state"
    for p in data.get("presets", []):
        if not isinstance(p, dict) or set(p) - PRESET_KEYS:
            return "unknown key in [[presets]]"
        if p.get("name") in BUILTIN_PRESETS:
            return "a preset named like a built-in"
        for s in p.get("steps", []):
            if not STEP_RE.fullmatch(s):
                return "a preset step that is no walk step"
    return None


def capability_problem(caps: set) -> str | None:
    """7.6: every other capability needs `structure`; `verification` needs
    `evidence`; `resolution` needs `authorization`; `checks` needs `claims`."""
    if caps and "structure" not in caps:
        return "a capability without structure"
    for c, need in (("verification", "evidence"), ("resolution", "authorization"), ("checks", "claims")):
        if c in caps and need not in caps:
            return f"{c} without {need}"
    return None


def merge_profile(base: dict, ext: dict, path=()) -> dict:
    """7.5 `extends`: start from the base's declarations; add and narrow.
    Capabilities are the union; a field either requires is required."""
    out = copy.deepcopy(base)
    for k, v in ext.items():
        if path[:1] == ("fields",) and len(path) == 2 and isinstance(v, dict) and k in out:
            merged = {**out[k], **v}
            merged["required"] = bool(out[k].get("required")) or bool(v.get("required"))
            out[k] = merged
        elif isinstance(v, dict) and isinstance(out.get(k), dict):
            out[k] = merge_profile(out[k], v, path + (k,))
        elif isinstance(v, list) and isinstance(out.get(k), list) and k not in ("require",):
            out[k] = out[k] + [x for x in v if x not in out[k]]
        else:
            out[k] = copy.deepcopy(v)
    return out


def load_profiles(root: str):
    """(profiles by name, findings). An invalid repository profile maps to None."""
    raw, findings = {}, []
    for name in sorted(os.listdir(BUILTIN_PROFILES)):
        if name.endswith(".toml"):
            with open(os.path.join(BUILTIN_PROFILES, name), "rb") as f:
                data = tomllib.load(f)
            problem = profile_problem(data, name[:-5])
            if problem:
                raise SystemExit(f"built-in profile {name}: {problem}")
            raw[name[:-5]] = data
    builtin = set(raw)
    pdir = os.path.join(root, "profiles")
    local = {}
    if os.path.isdir(pdir):
        for name in sorted(os.listdir(pdir)):
            if not name.endswith(".toml"):
                continue
            stem, path = name[:-5], f"profiles/{name}"
            try:
                with open(os.path.join(pdir, name), "rb") as f:
                    data = tomllib.load(f)
            except (tomllib.TOMLDecodeError, UnicodeDecodeError):
                findings.append(("profile.invalid", path, 1))
                local[stem] = None
                continue
            if "format" not in data:
                # 7.5: a profile file without `format` is a legacy profile and
                # never governs a Format 1.0 document.
                continue
            problem = profile_problem(data, stem)
            # 7.5: a file with a built-in's name replaces it only by naming it in `extends`.
            if problem is None and stem in builtin and data.get("extends") != stem:
                problem = "a built-in profile's name without extends"
            if problem:
                findings.append(("profile.invalid", path, 1))
                local[stem] = None
            else:
                local[stem] = data
    resolved: dict[str, Profile | None] = {}

    def resolve(name, seen=()):
        if name in resolved:
            return resolved[name]
        if name in local:
            data = local[name]
            if data is None:
                return None
            ext = data.get("extends")
            if ext:
                if ext in seen:
                    return None
                base = raw.get(ext) if ext == name else None
                if base is None:
                    bp = resolve(ext, seen + (name,))
                    base = bp._data if bp is not None else None
                if base is None:
                    findings.append(("profile.invalid", f"profiles/{name}.toml", 1))
                    resolved[name] = None
                    return None
                # 7.5: its `form` is the base's; its capabilities, after the
                # union, must still meet the prerequisites of 7.6.
                if data.get("form", base.get("form")) != base.get("form"):
                    findings.append(("profile.invalid", f"profiles/{name}.toml", 1))
                    resolved[name] = None
                    return None
                data = merge_profile(base, {k: v for k, v in data.items() if k != "extends"})
                if capability_problem(set(data.get("capabilities", []))):
                    findings.append(("profile.invalid", f"profiles/{name}.toml", 1))
                    resolved[name] = None
                    return None
        elif name in raw:
            data = raw[name]
        else:
            return None
        p = Profile(data)
        p._data = data
        resolved[name] = p
        return p

    names = set(raw) | set(local)
    return {n: resolve(n) for n in sorted(names)}, findings


def shape_ok(value: str, spec: dict) -> bool:
    """7.5 **Shapes**."""
    shape = spec.get("shape", "text")
    if shape == "text":
        return True
    if shape == "line":
        return "\n" not in value
    if shape == "int":
        return re.fullmatch(r"-?[0-9]+", value) is not None
    if shape == "number":
        return re.fullmatch(r"-?[0-9]+(?:\.[0-9]+)?", value) is not None
    if shape == "bool":
        return value in ("true", "false")
    if shape == "id":
        return TARGET_RE.fullmatch(value) is not None
    if shape == "ids":
        return all(TARGET_RE.fullmatch(t) for t in value.split(", "))
    if shape == "gate":
        return all(GATE_RE.fullmatch(t) for t in value.split(", "))
    if shape == "date":
        return re.fullmatch(r"\d{4}-\d{2}-\d{2}", value) is not None
    if shape == "enum":
        return value in spec.get("values", [])
    return False


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
        self.profile_name = None
        self.profile: Profile | None = None
        self.fm: dict = {}
        self.data = b""
        self.place = None

    def find(self, rule, line):
        self.findings.append((rule, line))
        if rule in REFUSALS:
            self.refused = True
        if rule in ("format.newer-major", "format.requires-unknown"):
            # The whole document is UNKNOWN; a newer minor's UNKNOWN is per unit.
            self.unknown = True

    @property
    def own_id(self):
        return self.alias if self.alias else "doc:" + self.source

    @property
    def is_work(self):
        return self.alias is not None


def read_document(source: str, data: bytes, work: str | None, profiles=None) -> Doc:
    """work: None for a records document, "spec" for SPEC.md, else "warrant" or "ticket"."""
    doc = Doc(source)
    doc.data, doc.place = data, work
    try:
        parse(doc, data, work, profiles or {})
    except Refused as r:
        doc.find(r.rule, r.line)
    return doc


def parse(doc: Doc, data: bytes, work, profiles):
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
    lines = split_lines(text, bom)
    pairs, body_at = frontmatter(lines)
    if pairs is None:
        # Section 6: no `format` -> a legacy encoding or `format.unrecognised`.
        raise Refused("format.unrecognised", 1)
    fm = {k: (v, n) for k, v, n in pairs}
    doc.fm = fm
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
    profile, pline = fm.get("profile", (None, 1))
    if profile is None or profile == [] or profile == "":
        raise Refused("format.field-missing", 1)
    doc.profile_name = profile
    if "requires" in fm:
        req, rline = fm["requires"]
        req = req if isinstance(req, list) else [req]
        if req:
            # This reader understands no extension (7.3).
            doc.find("format.requires-unknown", rline)
            return
    prof = profiles.get(profile) if isinstance(profile, str) else None
    if prof is None:
        # 7.4: read under the core alone; every profile rule over it is UNKNOWN.
        doc.find("format.profile-unknown", pline)
    doc.profile = prof
    allowed = CORE_KEYS | RELATION_KEYS
    if prof is not None:
        allowed |= set(prof.fm_required) | set(prof.fm_optional)
    fields, ext = {}, {}
    doc_rels = []
    for k, v, n in pairs:
        if "." in k:
            ext[k] = v
            if not prefix_registered(k):
                doc.find("format.prefix-unregistered", n)
            continue
        if k in RELATION_KEYS:
            for t in (v if isinstance(v, list) else v.split(", ")):
                tm = TARGET_RE.fullmatch(t.strip())
                if not tm:
                    raise Refused("format.relation-target", n)
                rel = {"kind": k, "to": tm.group(1), "_line": n}
                if tm.group(2):
                    rel["to_revision"] = tm.group(2)
                doc_rels.append(rel)
            continue
        if k not in allowed and prof is not None:
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
    if work in ("warrant", "ticket") and not is_work:
        raise Refused("format.field-missing", 1)
    if is_work:
        for key in ("uuid", "summary"):
            if key not in fm or fm[key][0] in ("", []):
                raise Refused("format.field-missing", 1)
        doc.alias = fm["alias"][0]
        doc.uuid = fm["uuid"][0]
    if prof is not None:
        for key in prof.fm_required:
            if key not in fm or fm[key][0] in ("", []):
                raise Refused("format.field-missing", 1)
    # 4.5: `version` is REQUIRED on SPEC.md.
    if work == "spec" and ("version" not in fm or fm["version"][0] in ("", [])):
        raise Refused("format.field-missing", 1)
    if "summary" in fm and isinstance(fm["summary"][0], str) and len(fm["summary"][0].encode()) > 200:
        doc.find("context.summary-shape", fm["summary"][1])

    # fences first (3.9), then units (3.4)
    body_lines = lines[body_at:]
    in_fence = []  # per body line: True when inside (or opening/closing) a fence
    fence = None
    for n, ln, _, _ in body_lines:
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
    for (n, ln, s, e), fenced in zip(body_lines, in_fence):
        if not fenced and ln.startswith("## "):
            cur = {"line": n, "head": ln[3:], "lines": [], "start": s, "end": e}
            units.append(cur)
        elif cur is None:
            preamble.append((n, ln, fenced, s, e))
        else:
            cur["lines"].append((n, ln, fenced, s, e))
            cur["end"] = e
    for i, u in enumerate(units):
        u["end"] = units[i + 1]["start"] if i + 1 < len(units) else len(data)

    title = None
    for n, ln, fenced, _, _ in preamble:
        if not fenced and ln.startswith("# "):
            title = ln[2:]
            break
    alias = doc.alias
    scope = (alias + "/") if is_work else ""

    def gid(local):
        return scope + local

    records, rels = [], []
    # The document's own record (4.1, 4.4, 4.5): a work document's is its
    # alias; a specification's is `doc:<path>` of type `spec`; any other
    # document's is `doc:<path>` of type `document`. Its body is the preamble
    # without its `# ` title line.
    pre = []
    dropped = False
    for n, ln, fenced, _, _ in preamble:
        if not dropped and not fenced and ln.startswith("# "):
            dropped = True
            continue
        pre.append(ln)
    own = doc.own_id
    if is_work:
        own_type = "ticket" if profile == "ticket" else "warrant"
    elif profile == "spec":
        own_type = "spec"
    else:
        own_type = "document"
    # 10.3: the document record's span is the frontmatter and the preamble,
    # up to its first unit (BOM excluded).
    first_unit = units[0]["start"] if units else len(data)
    records.append({
        "id": own, "type": own_type,
        "title": title or "", "fields": fields, "ext": ext,
        "body": "\n".join(pre), "line": 1, "_span": (bom, first_unit),
    })
    for r in doc_rels:
        r["from"] = own
        rels.append(r)

    seen_ids: dict[str, int] = {}
    section_names = []
    item_ids: set[str] = set()
    is_spec = profile == "spec"
    state = {"section": None}

    def claim(i, n):
        if i in seen_ids:
            raise Refused("format.id-duplicate", n)
        seen_ids[i] = n

    def unit(u):
        """One unit: its records and relations, all or nothing."""
        head, n = u["head"], u["line"]
        urecs, urels, warns = [], [], []
        is_record = " · " in head or ASCII_START.match(head) is not None
        if is_record:
            m = HEAD_UNICODE.fullmatch(head) or HEAD_ASCII.fullmatch(head)
            if not m:
                m2 = HEAD_UNICODE_UNTITLED.fullmatch(head) or HEAD_ASCII_UNTITLED.fullmatch(head)
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
            if "." in rtype and not prefix_registered(rtype):
                warns.append(("format.prefix-unregistered", n))
            if ("." not in rtype and prof is not None and prof.types_closed
                    and rtype not in CORE_TYPES and rtype not in prof.types):
                # 7.5: `[records] closed = true` refuses any other undotted type.
                raise Refused("format.key-unknown", n)
            rec, rrels, rwarns = record_unit(doc, gid(rid), rtype, rtitle, u["lines"], n, prof,
                                             is_spec)
            rec["_span"] = (u["start"], u["end"])
            rec["_section"] = state["section"]
            urecs.append(rec)
            urels.extend(rrels)
            warns.extend(rwarns)
            body_items = u["lines"][rec.pop("_body_at"):]
            claim_ids = [rid]
            unit_gid = gid(rid)
            section_of_items = state["section"]
        else:
            s = slug(head)
            if not s:
                raise Refused("format.heading", n)
            sid = "S-" + s
            if prof is not None and prof.sec_closed:
                listed = {slug(x) for x in prof.sec_required + prof.sec_optional}
                if s not in listed:
                    # 7.5: a section outside a `closed = true` set is refused
                    # as a field outside its table is.
                    raise Refused("format.key-unknown", n)
            # READING: a section's body skips one blank line after its heading,
            # as a record with no attribute block does (3.5).
            lines_ = [ln for _, ln, _, _, _ in u["lines"]]
            if lines_ and lines_[0].strip() == "":
                lines_ = lines_[1:]
            urecs.append({
                "id": gid(sid), "type": "section", "title": head,
                "fields": {}, "ext": {}, "body": "\n".join(lines_), "line": n,
                "_span": (u["start"], u["end"]), "_section": state["section"],
            })
            body_items = u["lines"]
            claim_ids = [sid]
            unit_gid = gid(sid)
            section_of_items = gid(sid)
        new_items = set()
        if is_work:
            for ln_n, ln, fenced, s, e in body_items:
                if not fenced:
                    item(doc, ln_n, ln, gid, item_ids | new_items, new_items, urecs, urels,
                         (s, e), unit_gid, section_of_items, warns)
        for ln_n, ln, fenced, _, _ in u["lines"]:
            # 3.7: a `### OBL-` heading in a Format 1.0 document is prose.
            if not fenced and re.match(r"### OBL-", ln):
                warns.append(("format.legacy-obligation", ln_n))
        if is_spec:
            # 4.5: a checker warns on a `§` in any record of a specification, at the line.
            if SECTION_REF_RE.search(head):
                warns.append(("context.section-ref", n))
            for ln_n, ln, fenced, _, _ in u["lines"]:
                if SECTION_REF_RE.search(ln):
                    warns.append(("context.section-ref", ln_n))
        for i in claim_ids:
            claim(i, n)
        if not is_record:
            section_names.append(head)
            state["section"] = gid("S-" + slug(head))
        item_ids.update(new_items)
        records.extend(urecs)
        rels.extend(urels)
        for w in warns:
            doc.find(*w)

    for u in units:
        if not newer_minor:
            unit(u)
            continue
        try:
            unit(u)
        except Refused:
            doc.find("format.newer-minor", u["line"])

    have = {slug(s) for s in section_names}
    required = list(prof.sec_required) if prof is not None else []
    if work == "spec":
        required += ["Purpose", "Scope"]  # 4.5: SPEC.md requires Purpose and Scope
    if is_work or prof is not None:
        for s in required:
            if slug(s) not in have:
                raise Refused("format.section-missing", 1)
    if is_work and profile == "delivery" and not any(r["type"] == "obligation" for r in records):
        raise Refused("format.section-missing", 1)

    # resolve local targets
    local = {r["id"] for r in records}
    for r in rels:
        t = r["to"]
        if is_work and "/" not in t and gid(t) in local:
            r["to"] = gid(t)
    doc.records, doc.relations = records, rels


def record_unit(doc, rid, rtype, title, lines, head_line, prof, is_spec):
    fields, ext, rels, warns = {}, {}, [], []
    field_line = {}
    last = None
    table = dict(prof.field_table(rtype)) if prof is not None else {}
    if rtype == "milestone":
        table["min"] = {"shape": "enum", "values": ["claimed", "observed", "independent", "signed"]}
    declared = CORE_FIELDS.get(rtype, set()) | set(table)
    i = 0
    while i < len(lines):
        n, ln = lines[i][0], lines[i][1]
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
            if "." in key:
                if not prefix_registered(key):
                    warns.append(("format.prefix-unregistered", n))
            else:
                if rtype == "obligation" and key in AUTHORED_STATE:
                    raise Refused("format.authored-state", n)
                # 7.5: once the core or the profile declares a field table for
                # a type, a field outside it is refused; a type for which
                # neither declares any field accepts any field.
                if declared and key not in declared:
                    if doc.newer_minor:
                        doc.find("format.newer-minor", n)
                        i += 1
                        continue
                    raise Refused("format.key-unknown", n)
            store[key] = val
            field_line[key] = n
            last = (store, key)
            i += 1
            continue
        rm = RELATION_RE.fullmatch(ln)
        if rm:
            kind, targets = rm.groups()
            if kind not in CORE_KINDS and "." not in kind:
                raise Refused("format.relation-kind", n)
            if "." in kind and not prefix_registered(kind):
                warns.append(("format.prefix-unregistered", n))
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
    body = [ln[1] for ln in lines[i:]]
    required = list(CORE_REQUIRED.get(rtype, [])) + [k for k, s in table.items() if s.get("required")]
    # 3.5: an OPTIONAL field whose value is empty is absent.
    for key in [k for k, v in fields.items() if v == "" and k not in required]:
        del fields[key]
    # 7.5: a value that does not have its shape is refused at its line.
    for key, val in fields.items():
        spec = table.get(key)
        if spec is not None and not shape_ok(val, spec):
            raise Refused("format.field-shape", field_line[key])
    for req in required:
        # An empty value is a missing one.
        if fields.get(req, "").strip() == "":
            raise Refused("format.field-missing", head_line)
    if rtype == "obligation" and "refusal" not in fields:
        warns.append(("format.refusal-missing", head_line))
    if is_spec and rtype in ("requirement", "constraint") and "statement" in fields:
        # 4.5: warned at the heading when longer than 300 bytes, or when a
        # `.`, `!` or `?` followed by a space occurs before its end.
        st = fields["statement"]
        if len(st.encode()) > 300 or SENTENCE_BREAK_RE.search(st):
            warns.append(("context.statement-shape", head_line))
    rec = {"id": rid, "type": rtype, "title": title, "fields": fields, "ext": ext,
           "body": "\n".join(body), "line": head_line, "_body_at": i}
    return rec, rels, warns


def item(doc, n, ln, gid, item_ids, new_items, records, rels, span, unit_gid, section, warns):
    b = BOX_RE.match(ln)
    if not b:
        return
    if b.group(1) not in (" ", "x", "X"):
        raise Refused("format.item", n)
    m = ITEM_RE.fullmatch(ln)
    if not m and re.search(r"\(i-[^)]*\)(?: — done by .*)?$", ln):
        # An id group whose id is not `i-` and 3 to 16 lowercase hex digits,
        # or a done suffix that does not parse (3.8, Appendix A `done`).
        raise Refused("format.item", n)
    if not m:
        # READING: a box line whose text does not end in an id group (and an
        # optional `done` suffix) has no id group.
        warns.append(("format.item-id-missing", n))
        return
    _lead, box, text, iid, after, done, actor, date, level, note = m.groups()
    if iid in item_ids:
        raise Refused("format.id-duplicate", n)
    new_items.add(iid)
    rid = gid(iid)
    irels = []
    afters = re.findall(r", after (\S+?)(?=,|$)", after)
    for ref in afters:
        to = gid(ref) if re.fullmatch(ITEM_ID, ref) else ref
        irels.append({"from": rid, "kind": "depends_on", "to": to, "_line": n})
    for ref in IMPLEMENTS_RE.findall(text):
        irels.append({"from": rid, "kind": "implements", "to": ref, "_line": n})
    rels.extend(irels)
    # READING: an item is a record of type `item` whose title is its text;
    # its box and `done by` suffix are state, not content (2: a state is
    # never authored, it is computed).
    rec = {"id": rid, "type": "item", "title": text, "fields": {}, "ext": {},
           "body": "", "line": n, "_span": span, "_unit": unit_gid, "_section": section,
           "_box": "x" if box in "xX" else " ", "_text": text, "_iid": iid,
           "_after": afters}
    if done:
        rec["_done"] = {"actor": actor, "date": date, "level": level or "claimed", "note": note}
    records.append(rec)


# -------------------------------------------------------------- repository --

def is_alias(name: str) -> bool:
    return re.fullmatch(ALIAS, name) is not None


def discover(root: str):
    """(source, place, alias) for every document sections 4.1, 4.4 and 4.5 place."""
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
    # READING: section 4.4 places record documents at `docs/records/<area>/*.md`.
    rd = os.path.join(root, "docs/records")
    if os.path.isdir(rd):
        for area in sorted(os.listdir(rd)):
            ap = os.path.join(rd, area)
            if not os.path.isdir(ap):
                continue
            for name in sorted(os.listdir(ap)):
                if name.endswith(".md"):
                    out.append((f"docs/records/{area}/{name}", None, None))
    # 4.5: `SPEC.md` at the root, and its parts `spec/<area>.md`.
    if os.path.isfile(os.path.join(root, "SPEC.md")):
        out.append(("SPEC.md", "spec", None))
    sd = os.path.join(root, "spec")
    if os.path.isdir(sd):
        for name in sorted(os.listdir(sd)):
            if name.endswith(".md") and os.path.isfile(os.path.join(sd, name)):
                out.append((f"spec/{name}", None, None))
    return out


def manifest_check(root: str, source: str, doc: Doc):
    """4.2: a manifest.toml beside warrant.md must not list atoms or restate a frontmatter key."""
    if not source.endswith(("/warrant.md", "/ticket.md")):
        return
    mp = os.path.join(root, os.path.dirname(source), "manifest.toml")
    if not os.path.isfile(mp):
        return
    with open(mp, "rb") as f:
        try:
            man = tomllib.load(f)
        except tomllib.TOMLDecodeError:
            return
    defined = set(CORE_KEYS) | RELATION_KEYS
    if doc.profile is not None:
        defined |= set(doc.profile.fm_required) | set(doc.profile.fm_optional)
    if "atoms" in man or any(k in defined for k in man):
        rel = os.path.relpath(mp, root)
        doc.findings.append(("format.manifest-restates", 1, rel))
        doc.refused = True


class Repository:
    """Every document read, and the model's records and relations of those not refused."""

    def __init__(self, root):
        self.root = root
        self.docs: list[Doc] = []
        self.records: list[dict] = []
        self.relations: list[dict] = []
        self.containment: list[dict] = []
        self.findings: set[str] = set()
        self.profiles = {}
        self.by_id: dict[str, dict] = {}
        self.doc_of: dict[str, Doc] = {}

    def doc_by_alias(self, alias):
        for d in self.docs:
            if d.alias == alias and not (d.refused or d.unknown):
                return d
        return None


def load_repository(root: str) -> Repository:
    repo = Repository(root)
    profiles, pfind = load_profiles(root)
    repo.profiles = profiles
    for rule, path, line in pfind:
        repo.findings.add(f"{rule} {path}:{line}")
    docs = []
    by_alias: dict[tuple, str] = {}
    for source, place, alias in discover(root):
        with open(os.path.join(root, source), "rb") as f:
            data = f.read()
        if place in (None, "spec") and not re.match(rb"(\xef\xbb\xbf)?---\r?\n(?:.*\r?\n)*?format:", data):
            continue
        if place in ("warrant", "ticket"):
            key = (place, alias)
            if key in by_alias:
                # READING: the form found second in byte order of path is the one reported.
                repo.findings.add(f"format.document-duplicate {source}:1")
                docs = [d for d in docs if d.source != by_alias[key]]
                continue
            by_alias[key] = source
        d = read_document(source, data, place, profiles)
        manifest_check(root, source, d)
        docs.append(d)
    # 4.5: at most one root specification: any other file with profile `spec`
    # that names no `part_of` is refused, whether or not SPEC.md exists.
    for d in docs:
        if d.profile_name == "spec" and d.source != "SPEC.md" and not (d.refused or d.unknown):
            if not any(r["kind"] == "part_of" and r["from"] == d.own_id for r in d.relations):
                d.findings.append(("format.document-duplicate", 1))
                d.refused = True
    repo.docs = docs
    for d in docs:
        for f in d.findings:
            if len(f) == 3:
                repo.findings.add(f"{f[0]} {f[2]}:{f[1]}")
            else:
                repo.findings.add(f"{f[0]} {d.source}:{f[1]}")
        if d.refused or d.unknown:
            continue
        rel_by_from: dict[str, list] = {}
        for r in d.relations:
            rel_by_from.setdefault(r["from"], []).append(r)
        for rec in d.records:
            rec["content_digest"] = content_digest(rec, rel_by_from.get(rec["id"], []))
            rec["source"] = d.source
            s, e = rec["_span"]
            rec["revision"] = sha256(d.data[s:e])
            if d.is_work and rec["id"] != d.alias:
                rec["document"] = d.alias
            repo.records.append(rec)
            repo.doc_of[rec["id"]] = d
            if rec["id"] != d.own_id:
                # 4.1, 4.4: each unit and item is part_of its document; in the
                # model, outside every content_digest.
                repo.containment.append({"from": rec["id"], "kind": "part_of", "to": d.own_id})
        repo.relations.extend(d.relations)
    repo.by_id = {r["id"]: r for r in repo.records}
    # 3.6: a target naming no record is a warning; the relation is kept.
    known = set(repo.by_id)
    uuids = {d.uuid for d in docs if d.uuid and not (d.refused or d.unknown)}
    for r in repo.relations:
        t = r["to"]
        if t in known or (t.startswith("war://") and t[6:] in uuids):
            continue
        # An extension kind is inert; its target may be outside the repository.
        if "." in r["kind"]:
            continue
        repo.findings.add(f"format.target-unknown {repo.by_id[r['from']]['source']}:{r['_line']}")
    return repo


def resolve_ref(repo: Repository, ref: str) -> str | None:
    """A target as authored, to the id of a record of the model, or None."""
    if ref in repo.by_id:
        return ref
    if ref.startswith("war://"):
        for d in repo.docs:
            if d.uuid == ref[6:] and d.alias in repo.by_id:
                return d.alias
    return None


def read_repository(root: str) -> dict:
    repo = load_repository(root)
    out_records = [
        {k: r[k] for k in ("id", "type", "title", "fields", "ext", "content_digest")}
        for r in sorted(repo.records, key=lambda r: r["id"].encode())
    ]
    out_rel = sorted(
        ({k: v for k, v in r.items() if not k.startswith("_")} for r in repo.relations),
        key=lambda r: (r["from"], r["kind"], r["to"], r.get("to_revision", "")),
    )
    return {"records": out_records, "relations": out_rel, "findings": sorted(repo.findings)}


def main(argv):
    if len(argv) != 2:
        print(__doc__, file=sys.stderr)
        return 64
    json.dump(read_repository(argv[1]), sys.stdout, ensure_ascii=False, sort_keys=True, indent=1)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
