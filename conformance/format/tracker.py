#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""A minimal second tracker and evaluator of OpenWarrant Format 1.0, draft 3.

Written from docs/format/FORMAT-1.0.md sections 10.5, 11 and 12 alone,
independently of `war`. Standard library only (and `git` to read a journal's
committed bytes). It reads journals (v1 and v2, 11.2), the claim store
(11.3), computes states (11.1), tick levels (11.4) and `next` (11.5), and
writes: a claim, a done tick, a note (for exactly-once, 11.2), the ticks a
passing check makes (12) and a package given back (`apply`, 10.5).

    tracker.py <root> --at <RFC 3339> [--acts DIR] <op> [args...]

    states                                   the model's states[]
    next [--actor A]                         the candidates of 11.5, in order
    claim <target> --actor A [--steal]       11.3
    done <item|alias> --actor A --base <rev> 11.3 compare-and-set, then the tick
    note <target> --actor A --note TEXT      11.2 exactly once
    evaluate --actor A                       12: what the recorded check runs tick
    apply <package> --actor A                10.5

prints one JSON object: {"findings": [...], "states": [...], "next": [...]}.
A refusal writes nothing for the document it concerns.

READING: choices the text leaves open are marked `READING:` and listed in
conformance/format/README.md.
"""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
import re
import secrets
import subprocess
import sys
import tomllib

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import reader  # noqa: E402
from reader import ITEM_RE, Refused, jcs, sha256  # noqa: E402

TRACK_REFUSALS = {"work.stale-revision", "work.claimed-by-other", "work.journal-rewritten",
                  "work.journal-malformed", "work.journal-torn", "work.idempotency-conflict",
                  "apply.stale", "apply.identity"}
TRACK_UNKNOWNS = {"work.claim-remote-unreachable", "work.journal-newer", "check.unknown"}
NEWER = object()
V2_MEMBERS = {"v", "id", "document_uuid", "type", "class", "actor_ref", "occurred_at", "payload",
              "idempotency_key"}
V1_MEMBERS = (V2_MEMBERS - {"document_uuid"}) | {"warrant_uuid"}
CLASSES = {"draft_history", "action_request", "action_receipt"}
LEVEL_RANK = {"claimed": 0, "observed": 1, "independent": 2, "signed": 3}


class Stop(Exception):
    """A refusal or UNKNOWN that ends the operation: (rule, path, line)."""

    def __init__(self, rule, path=None, line=None):
        super().__init__(rule)
        self.rule, self.path, self.line = rule, path, line

    def finding(self):
        return self.rule if self.path is None else f"{self.rule} {self.path}:{self.line}"


# ----------------------------------------------------------------- time --

def parse_time(s: str) -> dt.datetime:
    return dt.datetime.fromisoformat(s.replace("Z", "+00:00")).astimezone(dt.timezone.utc)


def unix(s: str) -> float:
    return parse_time(s).timestamp()


def rfc3339(t: dt.datetime) -> str:
    return t.astimezone(dt.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ")


def uuid7(at: dt.datetime) -> str:
    ms = int(at.timestamp() * 1000)
    rand = secrets.randbits(74)
    v = (ms << 80) | (0x7 << 76) | ((rand >> 62) << 64) | (0b10 << 62) | (rand & ((1 << 62) - 1))
    h = f"{v:032x}"
    return f"{h[:8]}-{h[8:12]}-{h[12:16]}-{h[16:20]}-{h[20:]}"


# ------------------------------------------------------------- journals --

def evidence_dir(doc) -> str:
    """4.3: the directory beside the document, docs/<place>/<alias>/."""
    src = doc.source
    if src.endswith(("/warrant.md", "/ticket.md")):
        return os.path.dirname(src)
    return src[:-3]


def journal_path(doc) -> str:
    return evidence_dir(doc) + "/journal.jsonl"


def idempotency_key(doc_uuid: str, etype: str, payload_bytes: bytes) -> str:
    """11.2: lowercase hex sha256 of `<document_uuid> LF <type> LF <RFC 8785 payload>`."""
    return hashlib.sha256(doc_uuid.encode() + b"\n" + etype.encode() + b"\n" + payload_bytes).hexdigest()


def committed_bytes(root: str, path: str) -> bytes | None:
    """The journal as last committed, from version control; None when there is none."""
    if not os.path.isdir(os.path.join(root, ".git")):
        return None
    env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
    p = subprocess.run(["git", "-C", root, "show", f"HEAD:{path}"], capture_output=True,
                       env=env, timeout=30)
    return p.stdout if p.returncode == 0 else None


def read_journal(root: str, doc) -> list[dict]:
    """11.2: every event of a document's journal, or Stop with the journal's fault."""
    path = journal_path(doc)
    full = os.path.join(root, path)
    if not os.path.isfile(full):
        data = b""
    else:
        with open(full, "rb") as f:
            data = f.read()
    lines = data.split(b"\n")
    if data and not data.endswith(b"\n"):
        raise Stop("work.journal-torn", path, len(lines))
    lines = lines[:-1] if data else []
    # Append-only: every line of the version in HEAD is still there, unchanged
    # and in order; lines a union merge added between them are no change.
    old = committed_bytes(root, path)
    if old:
        olines = old.split(b"\n")
        olines = olines[:-1] if old.endswith(b"\n") else olines
        it = iter(lines)
        for n, ol in enumerate(olines, 1):
            if not any(ln == ol for ln in it):
                raise Stop("work.journal-rewritten", path, n)
    events, seen = [], set()
    for n, raw in enumerate(lines, 1):
        ev = parse_event(raw)
        if ev is None:
            raise Stop("work.journal-malformed", path, n)
        if ev is NEWER:
            # 11.2: `v` greater than 2 is UNKNOWN, not malformed.
            events.append({"type": "_newer", "_line": n, "idempotency_key": None, "payload": {},
                           "at": "1970-01-01T00:00:00Z", "id": "", "actor": ""})
            continue
        ev["_line"] = n
        # Exactly once: a replayed key is one event.
        if ev["idempotency_key"] in seen:
            continue
        seen.add(ev["idempotency_key"])
        events.append(ev)
    return events


def parse_event(raw: bytes) -> dict | None:
    try:
        ev = json.loads(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError):
        return None
    if not isinstance(ev, dict) or isinstance(ev.get("v"), bool) or not isinstance(ev.get("v"), int):
        return None
    if ev["v"] > 2:
        return NEWER
    if ev["v"] not in (1, 2):
        return None
    need = V2_MEMBERS if ev["v"] == 2 else V1_MEMBERS
    if not need <= set(ev):
        return None
    for k in need - {"v", "payload"}:
        if not isinstance(ev[k], str):
            return None
    if ev["class"] not in CLASSES:
        return None
    try:
        parse_time(ev["occurred_at"])
    except ValueError:
        return None
    if ev["v"] == 2:
        if not isinstance(ev["payload"], dict):
            return None
        payload = ev["payload"]
    else:
        # A v1 event carries its payload as RFC 8785 JSON text.
        if not isinstance(ev["payload"], str):
            return None
        try:
            payload = json.loads(ev["payload"])
        except json.JSONDecodeError:
            return None
        if not isinstance(payload, dict):
            return None
    # 11.2: a line whose idempotency_key does not match its content is malformed.
    # A line's bytes need not be RFC 8785: the key is over the parsed payload
    # (v2), or over the payload text as written (v1).
    uuid = ev.get("document_uuid", ev.get("warrant_uuid"))
    body = jcs(payload) if ev["v"] == 2 else ev["payload"].encode()
    if idempotency_key(uuid, ev["type"], body) != ev["idempotency_key"]:
        return None
    etype = ev["type"]
    # Journals written before this format use `ticket.*` names.
    if etype.startswith("ticket."):
        etype = "work." + etype[len("ticket."):]
    return {"v": ev["v"], "id": ev["id"], "type": etype, "actor": ev["actor_ref"],
            "at": ev["occurred_at"], "payload": payload, "idempotency_key": ev["idempotency_key"],
            "doc_uuid": ev.get("document_uuid", ev.get("warrant_uuid"))}


def event_line(doc, etype: str, payload: dict, actor: str, at: dt.datetime,
               klass: str = "action_receipt") -> tuple[bytes, str]:
    key = idempotency_key(doc.uuid, etype, jcs(payload))
    ev = {"v": 2, "id": uuid7(at), "document_uuid": doc.uuid, "type": etype, "class": klass,
          "actor_ref": actor, "occurred_at": rfc3339(at), "payload": payload,
          "idempotency_key": key}
    return jcs(ev) + b"\n", key


# --------------------------------------------------------------- claims --

def claim_store(root: str) -> str:
    """11.3: `<git common dir>/openwarrant/claims/`, else `.openwarrant/state/claims/`."""
    if os.path.isdir(os.path.join(root, ".git")):
        return os.path.join(root, ".git", "openwarrant", "claims")
    return os.path.join(root, ".openwarrant", "state", "claims")


def config(root: str) -> dict:
    p = os.path.join(root, "openwarrant.toml")
    if os.path.isfile(p):
        with open(p, "rb") as f:
            return tomllib.load(f)
    return {}


def claims_config(root: str) -> dict:
    # 11.3: both settings live in `openwarrant.toml` under `[claims]`.
    c = config(root).get("claims", {})
    return {"lease": int(c.get("claim_lease_minutes", 30)), "ttl": int(c.get("claim_ttl_minutes", 120)),
            "remote": c.get("remote")}


def read_claims(root: str) -> list[dict]:
    store = claim_store(root)
    out = []
    if not os.path.isdir(store):
        return out
    for name in sorted(os.listdir(store)):
        if not name.endswith(".lock"):
            continue
        p = os.path.join(store, name)
        with open(p, "rb") as f:
            c = json.loads(f.read())
        c["_file"] = name
        c["_mtime"] = os.stat(p).st_mtime
        stem = name[:-5]
        c["_target"] = stem.replace("--", "/", 1)
        out.append(c)
    return out


def lease_end(c: dict, lease_minutes: int) -> float:
    """11.3: `claim_lease_minutes` past the later of `lease_until` and the file's modification time."""
    base = c["_mtime"]
    if c.get("lease_until_unix") is not None:
        base = max(base, float(c["lease_until_unix"]))
    elif c.get("lease_until"):
        base = max(base, unix(c["lease_until"]))
    return base + lease_minutes * 60


def live_claims(root: str, at: float) -> list[dict]:
    cfg = claims_config(root)
    return [c for c in read_claims(root) if at < lease_end(c, cfg["lease"])]


# --------------------------------------------------------------- states --

class World:
    """The repository read, with its journals and claims, at a time."""

    def __init__(self, root: str, at: str, acts_dir: str | None = None, read_state: bool = True):
        self.root, self.at_text = root, at
        self.at = parse_time(at) if at else None
        self.repo = reader.load_repository(root)
        self.work = [d for d in self.repo.docs if d.is_work and not (d.refused or d.unknown)]
        self.journals: dict[str, list[dict]] = {}
        self.claims: list[dict] = []
        self.warnings: set[str] = set()
        self.unknowns: set[str] = set()
        if read_state:
            for d in self.work:
                self.journals[d.alias] = read_journal(root, d)
            self.claims = live_claims(root, self.at.timestamp())
        self.acts = load_acts(acts_dir) if acts_dir else None
        self.compute()

    # -- helpers
    def items(self, doc) -> list[dict]:
        return [r for r in doc.records if r["type"] == "item"]

    def events(self, alias, etype=None):
        return [e for e in self.journals.get(alias, []) if etype is None or e["type"] == etype]

    def ev_target(self, doc, e) -> str | None:
        """11.2: `target` is the global id (`<alias>/<item>`, or an alias); `item` the local id."""
        p = e["payload"]
        t = p.get("target")
        if isinstance(t, str) and "/" in t:
            return t
        if isinstance(p.get("item"), str):
            i = p["item"]
            return i if "/" in i else f"{doc.alias}/{i}"
        return t if isinstance(t, str) else None

    def claim_on(self, target: str) -> dict | None:
        for c in self.claims:
            if c["_target"] == target:
                return c
        return None

    # -- the fixed states of 11.1
    def compute(self):
        self.st: dict[str, list[dict]] = {}  # record -> [{kind, value, provenance}]
        self.declared: list[dict] = []
        self.item_done: dict[str, bool] = {}
        self.backed_level: dict[str, str] = {}
        self.children: dict[str, list] = {}
        if any(e["type"] == "_newer" for evs in self.journals.values() for e in evs):
            self.unknowns.add("work.journal-newer")
        for d in self.work:
            for r in d.relations:
                if r["from"] == d.alias and r["kind"] == "part_of":
                    parent = reader.resolve_ref(self.repo, r["to"])
                    if parent and self.repo.by_id[parent]["type"] in ("warrant", "ticket"):
                        self.children.setdefault(parent, []).append(d)
        for d in self.work:
            for it in self.items(d):
                # 11.1: an item whose box is [x] is done, whether a tool or a
                # person ticked it; how is its tick level (11.4).
                self.item_done[it["id"]] = it["_box"] == "x"
        self.phase: dict[str, str] = {}
        self.phase_prov: dict[str, str] = {}
        for d in self.work:
            self.doc_phase(d)
        for d in self.work:
            for it in self.items(d):
                self.add(it["id"], "checklist", self.checklist(d, it))
            self.add(d.alias, "phase", self.phase[d.alias], self.phase_prov[d.alias])
            self.tick_levels(d)
            self.check_states(d)
        self.superseded()
        self.declared_states()
        if self.acts is not None:
            self.acceptance()

    def add(self, record, kind, value, provenance="computed"):
        self.st.setdefault(record, []).append({"kind": kind, "value": value, "provenance": provenance})

    def holds(self, record, value) -> bool:
        return any(s["value"] == value for s in self.st.get(record, []))

    def of_kind(self, record, kind):
        for s in self.st.get(record, []):
            if s["kind"] == kind:
                return s["value"]
        return None

    def latest(self, d, target, types):
        """11.2: of conflicting events about a target, the later `occurred_at` wins, then the greater `id`."""
        evs = [e for e in self.journals.get(d.alias, []) if e["type"] in types
               and self.ev_target(d, e) == target]
        return max(evs, key=lambda e: (unix(e["at"]), e["id"])) if evs else None

    def checklist(self, d, it) -> str:
        """11.1: `done` (the box), `claimed` (a live claim), `blocked`, else `open`."""
        rid = it["id"]
        if self.item_done[rid]:
            return "done"
        if self.claim_on(rid) or self.claim_on(d.alias):
            return "claimed"
        if self.is_blocked(d, it):
            return "blocked"
        return "open"

    def doc_items(self, d) -> list[tuple[str, bool]]:
        """(id, done) of a document's items; a work document part_of it counts as one."""
        out = [(it["id"], self.item_done[it["id"]]) for it in self.items(d)]
        for c in self.children.get(d.alias, []):
            out.append((c.alias, self.doc_phase(c) == "done"))
        return out

    def doc_phase(self, d) -> str:
        """11.1 `phase`: `done`, `in_progress` or `open` (`draft` belongs to Appendix C)."""
        if d.alias in self.phase:
            return self.phase[d.alias]
        self.phase[d.alias], self.phase_prov[d.alias] = "open", "computed"  # cycle guard
        items = self.doc_items(d)
        prov = "computed"
        if items:
            done = all(x for _, x in items)
            started = any(x for _, x in items) or any(self.claim_on(i) for i, _ in items)
        else:
            # With no items, a `work.item_done` naming the document makes it done.
            last = self.latest(d, d.alias, ("work.item_done", "work.item_reopened"))
            done = last is not None and last["type"] == "work.item_done"
            prov = "recorded" if done else "computed"
            started = False
        started = started or self.claim_on(d.alias) is not None
        p = "done" if done else ("in_progress" if started else "open")
        self.phase[d.alias], self.phase_prov[d.alias] = p, prov
        return p

    def dep_done(self, d, ref) -> bool:
        if re.fullmatch(reader.ITEM_ID, ref):
            ref = f"{d.alias}/{ref}"
        if ref in self.item_done:
            return self.item_done[ref]
        target = reader.resolve_ref(self.repo, ref)
        if target and target in self.phase:
            return self.phase[target] == "done"
        # READING: a dependency on a record that is not tracked does not block.
        return True

    def doc_deps(self, d) -> list[str]:
        return [r["to"] for r in d.relations if r["from"] == d.alias and r["kind"] == "depends_on"]

    def is_blocked(self, d, it) -> bool:
        """11.1: an item or document it waits on (`after`, its document's `depends_on`) is not done."""
        return any(not self.dep_done(d, ref) for ref in it["_after"] + self.doc_deps(d))

    def tick_levels(self, d):
        """11.4: a level above `claimed` is believed only as far as the journal backs it."""
        runs = {e["payload"].get("run"): e for e in self.events(d.alias, "check.run")}
        for it in self.items(d):
            if it["_box"] != "x":
                continue
            done = it.get("_done") or {"level": "claimed"}
            backed = "claimed"
            for e in self.events(d.alias, "work.item_done"):
                if self.ev_target(d, e) != it["id"]:
                    continue
                p = e["payload"]
                rs = p.get("runs") or []
                if p.get("level") == "observed" and rs and all(
                        r in runs and str(runs[r]["payload"].get("verdict", "")) == "PASS" for r in rs):
                    backed = "observed"
            for e in self.events(d.alias, "work.tick_raised"):
                if self.ev_target(d, e) == it["id"] and e["payload"].get("to") == "observed":
                    backed = "observed"
            # `independent` and `signed` need an act of Appendix C, which this
            # core tracker does not read: they read down to what is backed.
            self.backed_level[it["id"]] = min(backed, done["level"], key=LEVEL_RANK.get)
            if LEVEL_RANK[done["level"]] > LEVEL_RANK[backed]:
                self.warnings.add(f"work.tick-unbacked {d.source}:{it['line']}")

    def check_runs(self, d, rec, current=True) -> list[dict]:
        local = rec["id"].split("/", 1)[-1]
        return [e for e in self.events(d.alias, "check.run")
                if e["payload"].get("check") in (rec["id"], local)
                and (not current or e["payload"].get("revision") == rec["revision"])]

    def last_run(self, d, rec):
        """12: the last run on the check's current revision, by `occurred_at` then `id`."""
        runs = self.check_runs(d, rec)
        return max(runs, key=lambda e: (unix(e["at"]), e["id"])) if runs else None

    def check_states(self, d):
        """12: `failing` when the last run on its current revision failed; UNKNOWN when it could not complete."""
        for rec in d.records:
            if rec["type"] not in ("check", "kpi"):
                continue
            last = self.last_run(d, rec)
            if last is None:
                continue
            verdict = last["payload"].get("verdict")
            if verdict == "FAIL":
                self.add(rec["id"], "check", "failing", "recorded")
            elif verdict != "PASS":
                self.unknowns.add("check.unknown")

    def superseded(self):
        """11.1 `currency`: another current record `supersedes` it."""
        sup = {}
        for r in self.repo.relations:
            if r["kind"] == "supersedes":
                t = reader.resolve_ref(self.repo, r["to"])
                if t:
                    sup.setdefault(t, set()).add(r["from"])

        def current(rid, seen=()):
            if rid in seen:
                return True
            return not any(current(s, seen + (rid,)) for s in sup.get(rid, ()))

        for t, froms in sup.items():
            if any(f != t and current(f, (t,)) for f in froms):
                self.add(t, "currency", "superseded")

    def declared_states(self):
        """11.1: a declared state holds while its parent holds; the later entry wins.
        Its kind is its parent's and its provenance `declared`."""
        for d in self.work:
            prof = d.profile
            declared = {s["name"]: s["refines"] for s in (prof.states if prof else [])}
            latest = {}
            for e in self.events(d.alias, "state.entered"):
                p = e["payload"]
                rec, name = p.get("record"), p.get("state")
                if not isinstance(rec, str) or name not in declared or p.get("refines") != declared[name]:
                    continue
                if "/" not in rec and rec != d.alias:
                    rec = f"{d.alias}/{rec}"
                key = (unix(e["at"]), e["id"])
                if rec not in latest or key > latest[rec][0]:
                    latest[rec] = (key, name)
            for rec, (_, name) in sorted(latest.items()):
                parent = declared[name]
                kind = next((s["kind"] for s in self.st.get(rec, []) if s["value"] == parent),
                            "phase" if rec == d.alias else "checklist")
                s = {"record": rec, "kind": kind, "value": name, "provenance": "declared", "refines": parent}
                if not self.holds(rec, parent):
                    s["lapsed"] = True
                self.declared.append(s)

    def acceptance(self):
        """4.5 and Appendix C.3: acceptance is held per record and lapses with its revision."""
        spec_docs = [d for d in self.repo.docs if d.profile_name == "spec" and not (d.refused or d.unknown)]
        if not spec_docs:
            return
        accepted = set()
        for act in self.acts:
            if act.get("act") != "accept":
                continue
            for rid, rev, cd in act.get("records", []):
                r = self.repo.by_id.get(rid)
                if r and r["revision"] == rev and r["content_digest"] == cd:
                    accepted.add(rid)
        # 4.5: every unit of SPEC.md and its parts, sections included; the
        # documents' own `doc:` records are not counted.
        recs = [r for d in spec_docs for r in d.records if r["id"] != d.own_id]
        for r in recs:
            if r["id"] in accepted:
                self.add(r["id"], "acceptance", "accepted", "recorded")
        n = sum(1 for r in recs if r["id"] in accepted)
        root = "doc:SPEC.md"
        if root in self.repo.by_id and recs:
            if n == len(recs):
                self.add(root, "acceptance", "accepted")
            elif n:
                self.add(root, "acceptance", "partially_accepted")

    def states(self) -> list[dict]:
        out = [{"record": r, **s} for r, vs in self.st.items() for s in vs]
        out += self.declared
        return sorted(out, key=lambda s: (s["record"].encode(), s["kind"], s["value"]))

    def primary(self, rid: str) -> str:
        """11.1: the first that holds of superseded, phase, checklist, check,
        disposition, acceptance; else `-`; a holding declared state after it."""
        if rid not in self.repo.by_id:
            return "-"
        p = "-"
        if self.holds(rid, "superseded"):
            p = "superseded"
        else:
            for kind in ("phase", "checklist", "check", "disposition", "acceptance"):
                v = self.of_kind(rid, kind)
                if v is not None:
                    p = v
                    break
        for s in self.declared:
            if s["record"] == rid and not s.get("lapsed"):
                p = f"{p}/{s['value']}"
        return p

    # -- next, 11.5
    def next(self) -> list[str]:
        cands = []  # (id, doc, item or None)
        for d in self.work:
            items = self.doc_items(d)
            for it in self.items(d):
                if self.of_kind(it["id"], "checklist") == "open":
                    cands.append((it["id"], d, it))
            if not items and self.phase[d.alias] == "open":
                cands.append((d.alias, d, None))
        # waits-on graph: waiter -> waitees
        waiters: dict[str, set] = {}
        nodes = {}
        for d in self.work:
            nodes[d.alias] = (d, None)
            for it in self.items(d):
                nodes[it["id"]] = (d, it)
                for ref in it["_after"]:
                    tgt = f"{d.alias}/{ref}" if re.fullmatch(reader.ITEM_ID, ref) else ref
                    waiters.setdefault(tgt, set()).add(it["id"])
            for r in d.relations:
                if r["from"] == d.alias and r["kind"] == "depends_on":
                    t = reader.resolve_ref(self.repo, r["to"])
                    if t:
                        waiters.setdefault(t, set()).add(d.alias)

        def waiting_on(nid):
            """Everything that waits on nid: directly, or on its document."""
            out = set(waiters.get(nid, set()))
            if "/" in nid:
                out |= waiters.get(nid.split("/")[0], set())
            return {w for w in out if not self.node_done(w)}

        est = self.estimates()

        def due(nid, seen=()):
            d = nodes[nid][0] if nid in nodes else None
            own = d.fm.get("due", (None,))[0] if d else None
            vals = [own] if isinstance(own, str) else []
            for w in waiting_on(nid):
                if w not in seen and w in nodes:
                    x = due(w, seen + (nid,))
                    if x:
                        vals.append(x)
            return min(vals) if vals else None

        def remaining(nid, seen=()):
            e = est(nid)
            longest = 0
            for w in waiting_on(nid):
                if w not in seen and w in nodes:
                    longest = max(longest, remaining(w, seen + (nid,)))
            return e + longest

        def key(c):
            rid, d, it = c
            dv = due(rid)
            pr = d.fm.get("priority", (None,))[0]
            try:
                prio = int(pr) if isinstance(pr, str) else 2
            except ValueError:
                prio = 2
            created = d.fm.get("created_at", (None,))[0]
            return (
                (0, dv) if dv else (1, ""),
                -remaining(rid),
                prio,
                # 11.5: `created_at`, earliest first, none last.
                (0, created) if isinstance(created, str) else (1, ""),
                d.alias.encode(),
                it["line"] if it else 0,
            )

        return [c[0] for c in sorted(cands, key=key)]

    def node_done(self, nid) -> bool:
        if nid in self.item_done:
            return self.item_done[nid]
        return self.phase.get(nid) == "done"

    def estimates(self):
        """11.5: the median claim-to-done time over done items, by type and label."""
        samples = []  # (type, labels, secs)
        for d in self.work:
            dtype = d.fm.get("type", (None,))[0]
            labels = d.fm.get("labels", ([],))[0]
            labels = set(labels if isinstance(labels, list) else [labels])
            for it in self.items(d):
                if not self.item_done[it["id"]]:
                    continue
                claims = [e for e in self.events(d.alias, "work.claimed") if self.ev_target(d, e) == it["id"]]
                dones = [e for e in self.events(d.alias, "work.item_done") if self.ev_target(d, e) == it["id"]]
                if not claims or not dones:
                    continue
                c = max(unix(e["at"]) for e in claims)
                done_at = max(unix(e["at"]) for e in dones)
                samples.append((dtype, labels, done_at - c))
        prior = float(config(self.root).get("next", {}).get("prior_secs", 1800))

        def est(nid):
            d = None
            if nid in self.repo.by_id:
                d = self.repo.doc_of.get(nid)
            if d is None:
                return prior
            dtype = d.fm.get("type", (None,))[0]
            labels = d.fm.get("labels", ([],))[0]
            labels = set(labels if isinstance(labels, list) else [labels])
            for pick in (lambda s: s[0] == dtype and dtype is not None and s[1] & labels,
                         lambda s: s[1] & labels,
                         lambda s: s[0] == dtype and dtype is not None,
                         lambda s: True):
                xs = [s[2] for s in samples if pick(s)]
                if xs:
                    # 11.5: in whole seconds; with an even count, the lower middle value.
                    return int(sorted(xs)[(len(xs) - 1) // 2])
            return prior
        return est


def load_acts(path: str) -> list[dict]:
    """Corpus device: acts of Appendix C whose signatures the case takes as verified."""
    out = []
    for name in sorted(os.listdir(path)):
        if name.endswith(".json"):
            with open(os.path.join(path, name)) as f:
                out.append(json.load(f))
    return out


# --------------------------------------------------------------- writes --

# 11.2: the event types that carry `episode`.
EPISODIC = {"work.claimed", "work.claim_reclaimed", "work.claim_stolen", "work.released",
            "work.item_done", "work.item_reopened"}


class Writer:
    """Writes to work documents through the compare-and-set of 11.3."""

    def __init__(self, world: World, actor: str):
        self.w, self.actor = world, actor
        self.root = world.root

    def lock(self, doc):
        """11.3: an exclusive lock on the document's evidence directory for the write."""
        d = os.path.join(self.root, evidence_dir(doc))
        os.makedirs(d, exist_ok=True)
        p = os.path.join(d, ".lock")
        fd = os.open(p, os.O_CREAT | os.O_EXCL | os.O_WRONLY)
        os.close(fd)
        return p

    def append(self, doc, etype, payload, klass="action_receipt"):
        """11.2: append one event; a replay is not written; a conflict is refused.

        A claim, release, done or reopen carries `episode`: the number of
        events of its type for its target already in the journal (11.2)."""
        if etype in EPISODIC and "episode" not in payload:
            target = payload.get("target")
            payload = dict(payload, episode=sum(
                1 for e in self.w.events(doc.alias, etype) if self.w.ev_target(doc, e) == target))
        line, key = event_line(doc, etype, payload, self.actor, self.w.at, klass)
        for e in self.w.journals.get(doc.alias, []):
            if e["idempotency_key"] == key:
                if e["actor"] == self.actor:
                    return False
                raise Stop("work.idempotency-conflict", journal_path(doc), e["_line"])
        full = os.path.join(self.root, journal_path(doc))
        os.makedirs(os.path.dirname(full), exist_ok=True)
        with open(full, "ab") as f:
            f.write(line)
        return True

    def done_line(self, line: str, level="claimed", note=None) -> str:
        """3.8: `- [x] text (id[, after …]) — done by <actor>, <date>[ [level]][: note]`."""
        m = ITEM_RE.fullmatch(line)
        lead, _, text, iid, after = m.groups()[:5]
        s = f"{lead}[x] {text} ({iid}{after}) — done by {self.actor}, {self.w.at_text[:10]}"
        if level != "claimed":
            s += f" [{level}]"
        if note:
            s += f": {note}"
        return s

    def raised_line(self, line: str, level: str) -> str:
        """A done line with its level raised; one with no done suffix gets this writer's."""
        m = ITEM_RE.fullmatch(line)
        lead, _, text, iid, after, done, actor, date, _lv, note = m.groups()
        if not done:
            return self.done_line(line, level)
        s = f"{lead}[x] {text} ({iid}{after}) — done by {actor}, {date} [{level}]"
        return s + (f": {note}" if note else "")

    def open_line(self, line: str) -> str:
        m = ITEM_RE.fullmatch(line)
        lead, _, text, iid, after = m.groups()[:5]
        return f"{lead}[ ] {text} ({iid}{after})"

    def line_text(self, it) -> str:
        d = self.w.repo.doc_of[it["id"]]
        s, e = it["_span"]
        return d.data[s:e].decode().rstrip("\r\n")

    def replace(self, doc, edits):
        """Replace byte spans of a document: [(start, end, new bytes)]."""
        data = doc.data
        for s, e, new in sorted(edits, reverse=True):
            data = data[:s] + new + data[e:]
        with open(os.path.join(self.root, doc.source), "wb") as f:
            f.write(data)

    def line_edit(self, doc, it, new: str):
        s, e = it["_span"]
        old = doc.data[s:e]
        term = old[len(old.rstrip(b"\r\n")):]
        return (s, e, new.encode() + term)


def op_claim(world: World, target: str, actor: str, steal: bool):
    cfg = claims_config(world.root)
    if cfg["remote"]:
        # 11.3: the remote decides; a remote that cannot be reached is UNKNOWN.
        env = {k: v for k, v in os.environ.items() if not k.startswith("GIT_")}
        env["GIT_TERMINAL_PROMPT"] = "0"
        try:
            p = subprocess.run(["git", "-C", world.root, "ls-remote", cfg["remote"]],
                               capture_output=True, env=env, timeout=20)
            ok = p.returncode == 0
        except (OSError, subprocess.TimeoutExpired):
            ok = False
        if not ok:
            raise Stop("work.claim-remote-unreachable")
        # Pushing the claim ref with --force-with-lease is not implemented by
        # this tracker, so a claim it cannot place on the remote is not granted.
        raise Stop("work.claim-remote-unreachable")
    alias = target.split("/")[0]
    doc = world.repo.doc_by_alias(alias)
    if doc is None or (target != alias and target not in world.repo.by_id):
        raise SystemExit(f"claim: no such target {target}")
    at = world.at.timestamp()
    store = claim_store(world.root)
    os.makedirs(store, exist_ok=True)
    reclaimed = stolen = None
    for c in read_claims(world.root):
        t = c["_target"]
        covers = t == target or t == alias or (target == alias and t.startswith(alias + "/"))
        if not covers:
            continue
        live = at < lease_end(c, cfg["lease"])
        if c.get("actor") == actor:
            if live and t == target:
                os.utime(os.path.join(store, c["_file"]), (at, at))  # renewal
                return
            os.unlink(os.path.join(store, c["_file"]))
            continue
        if not live:
            reclaimed = c
        elif steal and at - float(c.get("since_unix", unix(c["since"]))) > cfg["ttl"] * 60:
            stolen = c
        else:
            raise Stop("work.claimed-by-other", f"claims/{c['_file']}", 1)
        os.unlink(os.path.join(store, c["_file"]))
    since = world.at_text
    claim = {"schema": "oh.war/ticket-claim/v1", "claim": uuid7(world.at), "ticket": alias,
             "actor": actor, "since": since, "since_unix": int(at)}
    if target != alias:
        claim["item"] = target.split("/", 1)[1]
    name = alias + ("--" + claim["item"] if "item" in claim else "") + ".lock"
    tmp = os.path.join(store, f".{name}.{secrets.token_hex(4)}")
    with open(tmp, "wb") as f:
        f.write(jcs(claim))
    try:
        os.link(tmp, os.path.join(store, name))  # fails if the name exists
    except FileExistsError:
        raise Stop("work.claimed-by-other", f"claims/{name}", 1)
    finally:
        os.unlink(tmp)
    os.utime(os.path.join(store, name), (at, at))
    w = Writer(world, actor)
    payload = {"claim": claim["claim"], "target": target, "since": since}
    etype = "work.claimed"
    if reclaimed or stolen:
        prev = reclaimed or stolen
        etype = "work.claim_reclaimed" if reclaimed else "work.claim_stolen"
        payload.update({"from": prev.get("actor"), "from_since": prev.get("since")})
    w.append(doc, etype, payload)


def op_done(world: World, target: str, actor: str, base: str):
    """11.3: a done tick, compare-and-set against the item's line revision, or,
    for a work document with no items, against the document's revision (11.1)."""
    rec = world.repo.by_id.get(target)
    if rec is None or rec["type"] not in ("item", "warrant", "ticket"):
        raise SystemExit(f"done: no such item or work document {target}")
    doc = world.repo.doc_of[target]
    if rec["type"] != "item" and world.items(doc):
        raise SystemExit(f"done: {target} has items; tick them")
    w = Writer(world, actor)
    lock = w.lock(doc)
    try:
        if rec["revision"] != base:
            raise Stop("work.stale-revision", doc.source, rec["line"])
        if rec["type"] == "item":
            w.replace(doc, [w.line_edit(doc, rec, w.done_line(w.line_text(rec)))])
            w.append(doc, "work.item_done", {"item": rec["_iid"], "target": target, "on": world.at_text[:10]})
        else:
            w.append(doc, "work.item_done", {"target": target, "on": world.at_text[:10]})
        for c in read_claims(world.root):
            if c["_target"] == target:
                os.unlink(os.path.join(claim_store(world.root), c["_file"]))
    finally:
        os.unlink(lock)


def op_note(world: World, target: str, actor: str, note: str):
    doc = world.repo.doc_by_alias(target.split("/")[0])
    w = Writer(world, actor)
    w.append(doc, "work.note_added", {"target": target, "note": note})


def op_evaluate(world: World, actor: str):
    """12: what the last run on each check's current revision does.

    A pass ticks every open item it evaluates `observed` and raises a done one
    below `observed`; a gating fail with `reopen: true` reopens each done item
    it evaluates; UNKNOWN ticks nothing. A run counts only for the subjects whose
    revision it names as current.
    """
    w = Writer(world, actor)
    for d in world.work:
        edits, events = [], []
        planned = set()

        def required_runs(item):
            required = []
            for check in d.records:
                if check["type"] not in ("check", "kpi") or check["fields"].get("gating") == "false":
                    continue
                covers = any(r["from"] == check["id"] and r["kind"] == "evaluates"
                             and reader.resolve_ref(world.repo, r["to"]) == item["id"]
                             for r in world.repo.relations)
                if not covers:
                    continue
                run = world.last_run(d, check)
                if run is None or run["payload"].get("verdict") != "PASS":
                    return []
                payload = run["payload"]
                subjects = {(sid if "/" in sid else f"{d.alias}/{sid}"): rev
                            for sid, rev in payload.get("subjects", [])}
                if subjects.get(item["id"]) != item["revision"]:
                    return []
                required.append(payload["run"])
            return required

        for rec in d.records:
            if rec["type"] not in ("check", "kpi"):
                continue
            last = world.last_run(d, rec)
            if last is None:
                continue
            p = last["payload"]
            verdict = p.get("verdict")
            subjects = {}
            for pair in p.get("subjects") or []:
                if isinstance(pair, list) and len(pair) == 2:
                    sid = pair[0] if "/" in pair[0] or pair[0] in world.repo.by_id else f"{d.alias}/{pair[0]}"
                    subjects[sid] = pair[1]
            for r in world.repo.relations:
                if r["from"] != rec["id"] or r["kind"] != "evaluates":
                    continue
                t = reader.resolve_ref(world.repo, r["to"])
                it = world.repo.by_id.get(t) if t else None
                if not it or it["type"] != "item":
                    continue
                if verdict == "PASS" and subjects.get(it["id"]) == it["revision"]:
                    if rec["fields"].get("gating") == "false":
                        continue
                    runs = required_runs(it)
                    milestone = world.repo.by_id.get(it.get("_unit"), {})
                    minimum = milestone.get("fields", {}).get("min", "claimed") if milestone.get("type") == "milestone" else "claimed"
                    if LEVEL_RANK.get(minimum, 0) > LEVEL_RANK["observed"]:
                        continue
                    if not runs or it["id"] in planned:
                        continue
                    planned.add(it["id"])
                    line = w.line_text(it)
                    if it["_box"] != "x":
                        edits.append(w.line_edit(d, it, w.done_line(line, "observed")))
                        events.append(("work.item_done", {"item": it["_iid"], "target": it["id"],
                                       "on": world.at_text[:10], "level": "observed", "runs": runs}))
                    elif LEVEL_RANK[world.backed_level.get(it["id"], "claimed")] < LEVEL_RANK["observed"]:
                        edits.append(w.line_edit(d, it, w.raised_line(line, "observed")))
                        events.append(("work.tick_raised", {"item": it["_iid"], "target": it["id"],
                                       "from": world.backed_level.get(it["id"], "claimed"), "to": "observed"}))
                elif (verdict == "FAIL" and rec["fields"].get("reopen") == "true"
                      and rec["fields"].get("gating") != "false" and it["_box"] == "x"
                      and subjects.get(it["id"]) == it["revision"]):
                    edits.append(w.line_edit(d, it, w.open_line(w.line_text(it))))
                    events.append(("work.item_reopened", {"item": it["_iid"], "target": it["id"],
                                   "reason": f"{rec['id']} failed (run {p.get('run')})"}))
        if edits:
            lock = w.lock(d)
            try:
                w.replace(d, edits)
                for etype, e in events:
                    w.append(d, etype, e)
            finally:
                os.unlink(lock)


# ---------------------------------------------------------------- apply --

MARK_OPEN = re.compile(r"<!-- rec (\S+) (\S+):(\d+) (sha256:[0-9a-f]{64}) -->")
MARK_CLOSE = "<!-- /rec -->"


def package_blocks(text: str) -> list[dict]:
    """10.5: every marked block of a package: (id, source, line, revision, text)."""
    blocks, cur = [], None
    for raw in re.findall(r"[^\n]*\n|[^\n]+\Z", text):
        line = raw.rstrip("\n")
        if cur is None:
            m = MARK_OPEN.fullmatch(line)
            if m:
                cur = {"id": m.group(1), "source": m.group(2), "line": int(m.group(3)),
                       "revision": m.group(4), "lines": []}
        elif line == MARK_CLOSE:
            cur["text"] = "".join(cur.pop("lines"))
            blocks.append(cur)
            cur = None
        else:
            cur["lines"].append(raw)
    return blocks


def op_apply(world: World, package: str, actor: str) -> list[str]:
    with open(package, encoding="utf-8") as f:
        blocks = package_blocks(f.read())
    by_doc: dict[str, list] = {}
    for b in blocks:
        by_doc.setdefault(b["source"], []).append(b)
    refusals = []
    for source in sorted(by_doc):
        doc = next((d for d in world.work if d.source == source), None)
        if doc is None:
            continue
        try:
            apply_document(world, doc, by_doc[source], actor)
        except Stop as s:
            # Blocks of one document are applied together or not at all; a
            # refusal in one document does not stop another.
            refusals.append(s.finding())
    return refusals


def apply_document(world: World, doc, blocks, actor):
    w = Writer(world, actor)
    edits, events = [], []
    for b in blocks:
        new = b["text"].encode()
        # 1. unchanged since the package was made: ignored.
        if sha256(new) == b["revision"]:
            continue
        rec = world.repo.by_id.get(b["id"])
        # 2. the source moved on since the package was made.
        if rec is None or rec["revision"] != b["revision"]:
            raise Stop("apply.stale", doc.source, b["line"])
        # 3. the block must parse as one unit of the same id and type.
        parsed = parse_block(world, doc, rec, b)
        # A changed box is an event, not a text edit; the line is rewritten as 3.8 shows.
        old_items = {r["_iid"]: r for r in doc.records if r["type"] == "item"
                     and rec["_span"][0] <= r["_span"][0] < rec["_span"][1]}
        text = b["text"]
        out_lines = []
        for raw in re.findall(r"[^\n]*\n|[^\n]+\Z", text):
            line = raw.rstrip("\n")
            term = raw[len(line):]
            m = ITEM_RE.fullmatch(line.rstrip("\r"))
            if m and m.group(4) in old_items:
                o = old_items[m.group(4)]
                was, now = o["_box"], ("x" if m.group(2) in "xX" else " ")
                if was == " " and now == "x":
                    events.append(("work.item_done", {"item": o["_iid"], "target": o["id"],
                                                      "on": world.at_text[:10]}))
                    line = w.done_line(line.rstrip("\r"))
                elif was == "x" and now == " ":
                    events.append(("work.item_reopened", {"item": o["_iid"], "target": o["id"],
                                                          "reason": "reopened in a package"}))
                    line = w.open_line(line.rstrip("\r"))
            out_lines.append(line + term)
        del parsed
        s, e = rec["_span"]
        edits.append((s, e, "".join(out_lines).encode()))
    if not edits:
        return
    lock = w.lock(doc)
    try:
        # 11.3: every applied change is a compare-and-set against the bytes read.
        with open(os.path.join(world.root, doc.source), "rb") as f:
            if f.read() != doc.data:
                raise Stop("work.stale-revision", doc.source, 1)
        w.replace(doc, edits)
        for etype, payload in events:
            w.append(doc, etype, payload)
    finally:
        os.unlink(lock)


def parse_block(world: World, doc, rec, b):
    """10.5 step 3: the block, read as a unit under section 3, in place of the source span."""
    s, e = rec["_span"]
    trial = doc.data[:s] + b["text"].encode() + doc.data[e:]
    d2 = reader.read_document(doc.source, trial, doc.place, world.repo.profiles)
    refusals = [f for f in d2.findings if f[0] in reader.REFUSALS]
    if refusals:
        rule, line = refusals[0][0], refusals[0][1]
        raise Stop(rule, doc.source, line)
    start = s
    unit_at = [r for r in d2.records if r.get("_span", (None,))[0] == start and r["type"] != "item"]
    if len(unit_at) != 1 or unit_at[0]["id"] != rec["id"] or unit_at[0]["type"] != rec["type"]:
        raise Stop("apply.identity", doc.source, b["line"])
    # The block is one unit: the next unit of the trial starts where the source's did.
    nxt = [r["_span"][0] for r in d2.records if r.get("_span") and r["_span"][0] > start and r["type"] != "item"]
    if nxt and min(nxt) != start + len(b["text"].encode()):
        raise Stop("apply.identity", doc.source, b["line"])
    return unit_at[0]


# ----------------------------------------------------------------- main --

def main(argv):
    ap = argparse.ArgumentParser()
    ap.add_argument("root")
    ap.add_argument("--at", required=True)
    ap.add_argument("--acts")
    ap.add_argument("op")
    ap.add_argument("rest", nargs=argparse.REMAINDER)
    a = ap.parse_args(argv[1:])
    sub = argparse.ArgumentParser()
    sub.add_argument("target", nargs="?")
    sub.add_argument("--actor")
    sub.add_argument("--steal", action="store_true")
    sub.add_argument("--base")
    sub.add_argument("--note")
    s = sub.parse_args(a.rest)
    out = {"findings": []}
    try:
        world = World(a.root, a.at, a.acts)
        if a.op == "states":
            pass
        elif a.op == "next":
            out["next"] = world.next()
        elif a.op == "claim":
            op_claim(world, s.target, s.actor, s.steal)
        elif a.op == "done":
            op_done(world, s.target, s.actor, s.base)
        elif a.op == "note":
            op_note(world, s.target, s.actor, s.note)
        elif a.op == "evaluate":
            op_evaluate(world, s.actor)
        elif a.op == "apply":
            out["findings"].extend(op_apply(world, s.target, s.actor))
        else:
            raise SystemExit(f"unknown op {a.op}")
        if a.op in ("claim", "done", "evaluate", "apply", "note"):
            world2 = World(a.root, a.at, a.acts)
            world = world2
        out["states"] = world.states()
        out["findings"].extend(sorted(world.repo.findings | world.warnings | world.unknowns))
    except Stop as e:
        out["findings"].append(e.finding())
    except Refused as e:
        out["findings"].append(f"{e.rule}")
    json.dump(out, sys.stdout, ensure_ascii=False, sort_keys=True, indent=1)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
