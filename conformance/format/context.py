#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""A minimal second context compiler of OpenWarrant Format 1.0, draft 3.

Written from docs/format/FORMAT-1.0.md sections 10.2 to 10.5 alone,
independently of `war`. Standard library only.

    context.py <root> --index
    context.py <root> <target> [--budget B | --tokens N] [--depth D] [--json]
                               [--preset NAME] [--at TIME]

prints one JSON object: {"output": "<the packet or index>", "findings": [...]}.
A refusal (`context.target-unknown`, `context.over-budget`) emits no output.

READING: choices the text leaves open are marked `READING:` and listed in
conformance/format/README.md.
"""

from __future__ import annotations

import argparse
import datetime as dt
import fnmatch
import hashlib
import json
import os
import re
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import reader  # noqa: E402
import tracker  # noqa: E402

CORE_STEPS = ["out:implements", "out:depends_on", "in:constrains", "out:selected_over",
              "in:evaluates", "out:part_of", "in:part_of:consequence", "in:implements:interface"]
TYPE_RANK = ["warrant/ticket", "section", "item", "requirement", "outcome", "constraint", "decision",
             "interface", "term", "option", "consequence", "obligation", "check", "kpi",
             "milestone", "stage", "instruction"]
# 10.4: the built-in presets. A preset that walks the core steps names its
# walk `core`; one with steps of its own names its walk after itself.
BUILTIN = {
    "agent": {"walk": "core", "steps": CORE_STEPS, "depth": 3, "budget": 16000},
    "agent.min": {"walk": "agent.min", "steps": ["out:implements", "in:constrains", "in:evaluates"],
                  "depth": 1, "budget": 4000, "omit_fields": ["rationale"],
                  "omit_types": ["option", "consequence"]},
    "human": {"walk": "core", "steps": CORE_STEPS, "depth": 3, "budget": None, "editable": True,
              "own_first": True},
    "review": {"walk": "review", "steps": ["in:evaluates", "out:evaluates"], "depth": 2,
               "budget": None, "reads_state": True},
    # The target's record and items, the core walk to depth 1; rendered as 10.4 says.
    "resume": {"walk": "core", "steps": CORE_STEPS, "depth": 1, "budget": None,
               "reads_state": True, "resume": True},
}
MIDDLE = " · "


class Refusal(Exception):
    def __init__(self, rule, detail=""):
        super().__init__(rule)
        self.rule, self.detail = rule, detail


def rank(rtype: str) -> int:
    if rtype in ("warrant", "ticket"):
        return 0
    return TYPE_RANK.index(rtype) if rtype in TYPE_RANK else len(TYPE_RANK)


def cut120(s: str) -> str:
    b = s.encode()
    if len(b) <= 120:
        return s
    return b[:120].decode("utf-8", errors="ignore") + "…"


def show_title(rec: dict, repo) -> str:
    """10.2: the title; else the first non-blank body line that is not a relation line; else -."""
    if rec.get("title"):
        return rec["title"]
    for ln in rec["body"].split("\n"):
        if ln.strip() == "":
            continue
        m = reader.RELATION_RE.fullmatch(ln)
        if m and (m.group(1) in reader.CORE_KINDS or "." in m.group(1)):
            continue
        return cut120(ln)
    return "-"


# ------------------------------------------------------------- states --

def DocumentStates(repo_root):
    """The states of the documents alone (11.1), for presets that do not read
    state: no journal and no claim store is read (10.3's purity rule)."""
    return tracker.World(repo_root, None, read_state=False)


# -------------------------------------------------------------- index --

def repository_name(root: str) -> str:
    """10.2: `[project] name` of the repository's `openwarrant.toml`, or else the
    name of its root directory."""
    import tomllib
    try:
        with open(os.path.join(root, "openwarrant.toml"), "rb") as f:
            name = tomllib.load(f).get("project", {}).get("name")
    except (OSError, tomllib.TOMLDecodeError):
        name = None
    return name if isinstance(name, str) and name else os.path.basename(os.path.abspath(root))


def build_index(root: str) -> str:
    """10.2: `docs/generated/llms.txt`."""
    states = DocumentStates(root)
    repo = states.repo
    out = [f"# {repository_name(root)}", "",
           "> OpenWarrant index (openwarrant/1.0). Generated; do not edit. "
           "Read any line's record with `war context <id>`."]
    for d in sorted((d for d in repo.docs if not (d.refused or d.unknown)), key=lambda d: d.source.encode()):
        summ = d.fm.get("summary", ("-",))[0]
        out += ["", f"## {d.source}", f"> {summ if isinstance(summ, str) and summ else '-'}"]
        recs = [repo.by_id[d.own_id]] if d.is_work else sorted(
            (r for r in d.records), key=lambda r: r["line"])
        for r in recs:
            out.append(f"- {r['id']}{MIDDLE}{r['type']}{MIDDLE}{states.primary(r['id'])}"
                       f"{MIDDLE}{show_title(r, repo)}{MIDDLE}{r['source']}:{r['line']}")
    return "\n".join(out) + "\n"


# ------------------------------------------------------------ packets --

def resolve_target(repo, root, target):
    """10.3 step 1: (roots, distance-0 set, item-intent pair or None, work document or None)."""
    by_id = repo.by_id
    works = {d.alias: d for d in repo.docs if d.is_work and not (d.refused or d.unknown)}

    def doc_root(d):
        zero = [r["id"] for r in d.records]
        return [d.alias], zero, None, d

    if re.fullmatch(reader.ITEM_ID, target):
        hits = [r["id"] for r in repo.records if r["type"] == "item" and r["_iid"] == target]
        target = hits[0] if len(hits) == 1 else target
    if target in by_id:
        rec = by_id[target]
        if rec["type"] in ("warrant", "ticket") and target in works:
            return doc_root(works[target])
        if rec["type"] == "item":
            d = repo.doc_of[target]
            intent = f"{d.alias}/S-intent"
            pair = (intent if intent in by_id else d.alias, target)
            return [target], [target], pair, d
        return [target], [target], None, None
    if target.startswith("war://"):
        for d in works.values():
            if d.uuid == target[6:]:
                return doc_root(d)
    for d in repo.docs:
        if d.source == target and not (d.refused or d.unknown):
            if d.is_work:
                return doc_root(d)
            ids = [r["id"] for r in d.records]
            return ids, ids, None, None
    if os.path.exists(os.path.join(root, target)) or "/" in target or "." in target:
        roots = []
        for d in sorted(works.values(), key=lambda d: d.alias.encode()):
            globs = d.fm.get("paths", ([],))[0]
            globs = globs if isinstance(globs, list) else [globs]
            if any(fnmatch.fnmatchcase(target, g) for g in globs):
                roots.append(d.alias)
        if roots:
            zero = []
            for a in roots:
                zero += [r["id"] for r in works[a].records]
            return roots, zero, None, None
    raise Refusal("context.target-unknown")


def neighbours(repo, rid, step, diags):
    parts = step.split(":")
    direction, kind = parts[0], parts[1]
    want = parts[2] if len(parts) > 2 else None
    rels = repo.relations + repo.containment
    out = []
    if direction == "out":
        for r in rels:
            if r["from"] != rid or r["kind"] != kind:
                continue
            t = reader.resolve_ref(repo, r["to"])
            if t is None:
                diags.add(("format.target-unknown", rid))
                continue
            if want is None or repo.by_id[t]["type"] == want:
                out.append(t)
    else:
        for r in rels:
            if r["kind"] != kind or reader.resolve_ref(repo, r["to"]) != rid:
                continue
            if want is None or repo.by_id[r["from"]]["type"] == want:
                out.append(r["from"])
    return out


def walk(repo, zero, pair, steps, depth, omit_types, diags):
    dist = {r: 0 for r in zero}
    via = {}
    frontier = sorted(set(zero), key=str.encode)
    pre = {}
    if pair and depth >= 1 and pair[0] not in dist:
        pre[pair[0]] = ("item-intent", pair[1])
    for level in range(1, depth + 1):
        new = dict(pre) if level == 1 else {}
        order = {}
        for si, step in enumerate(steps):
            for f in frontier:
                for nb in neighbours(repo, f, step, diags):
                    if nb in dist or nb in pre or repo.by_id[nb]["type"] in omit_types:
                        continue
                    cand = (si, f.encode())
                    if nb not in order or cand < order[nb]:
                        order[nb] = cand
                        new[nb] = (step.rsplit(":", 1)[0] if step.count(":") == 2 else step, f)
        for nb, v in new.items():
            dist[nb] = level
            via[nb] = v
        frontier = sorted(new, key=str.encode)
        if not frontier:
            break
    for r in list(dist):
        if repo.by_id[r]["type"] in omit_types:
            del dist[r]
    return dist, via


def strip_fields(text: str, rec: dict, omit: list[str]) -> str:
    """agent.min: the record's text without the left-out fields (and their continuations)."""
    if not omit or rec["type"] in ("section", "item") or "_span" not in rec:
        return text
    lines = text.split("\n")
    out, skipping, in_attrs = [lines[0]], False, True
    for ln in lines[1:]:
        if in_attrs and ln.strip() == "":
            in_attrs = False
        if in_attrs:
            if skipping and ln.startswith("  "):
                continue
            skipping = False
            m = reader.FIELD_RE.fullmatch(ln)
            if m and m.group(1) in omit:
                skipping = True
                continue
        out.append(ln)
    return "\n".join(out)


def compile_packet(root, target, preset_name="agent", budget=None, depth=None, at=None,
                   as_json=False):
    repo = reader.load_repository(root)
    preset = dict(BUILTIN.get(preset_name) or {})
    if not preset:
        preset = profile_preset(repo, preset_name)
    if budget is not None:
        preset["budget"] = budget
    if depth is not None:
        preset["depth"] = depth
    reads_state = preset.get("reads_state", False)
    world = None
    if reads_state:
        if at is None:
            raise SystemExit("a preset that reads state needs --at")
        world = tracker.World(root, at)
        states = world
        repo = world.repo
    else:
        states = DocumentStates(root)
        repo = states.repo
    diags = set()
    roots, zero, pair, wdoc = resolve_target(repo, root, target)
    if preset.get("resume"):
        if wdoc is None:
            raise Refusal("context.target-unknown")
        zero = [wdoc.alias] + [r["id"] for r in wdoc.records if r["type"] == "item"]
    omit_types = preset.get("omit_types", [])
    dist, via = walk(repo, zero, pair, preset["steps"], preset["depth"], omit_types, diags)
    recs = [repo.by_id[r] for r in dist]

    def order(r):
        base = (dist[r["id"]], rank(r["type"]), r["source"].encode(), r["line"])
        if preset.get("own_first") and wdoc is not None:
            if repo.doc_of[r["id"]] is wdoc:
                return (0, (r["line"],))
            return (1, base)
        return (0, base)

    recs.sort(key=order)
    selected = {r["id"] for r in recs}
    sel_digest = reader.basis_digest(sorted(([r["id"], r["revision"]] for r in recs),
                                            key=lambda p: p[0].encode()))
    budget_v = preset.get("budget")
    head = (f"# Context: {target}\n<!-- oh.war/context/v1{MIDDLE}budget "
            f"{budget_v if budget_v is not None else 'none'}{MIDDLE}depth {preset['depth']}"
            f"{MIDDLE}preset {preset_name} (walk {preset['walk']}){MIDDLE}selection {sel_digest}")
    if reads_state:
        # 10.4: the basis_digest rule over [path, sha256 of the file's bytes]
        # for every journal and every claim file, sorted by path.
        files = [tracker.journal_path(d) for d in world.work]
        store = tracker.claim_store(root)
        if os.path.isdir(store):
            files += [os.path.relpath(os.path.join(store, n), root).replace(os.sep, "/")
                      for n in os.listdir(store) if n.endswith(".lock")]
        pairs = []
        for rel in sorted(set(files), key=str.encode):
            p = os.path.join(root, rel)
            if os.path.isfile(p):
                with open(p, "rb") as f:
                    pairs.append([rel, "sha256:" + hashlib.sha256(f.read()).hexdigest()])
        jd = reader.basis_digest(pairs)
        head += f"{MIDDLE}journal {jd[:19]}{MIDDLE}at {at}"
    head += " -->\n"

    def container(r):
        if r["type"] != "item":
            return None
        return r.get("_unit")

    def text_of(r):
        d = repo.doc_of[r["id"]]
        s, e = r["_span"]
        t = d.data[s:e].decode("utf-8")
        t = strip_fields(t, r, preset.get("omit_fields", []))
        return t if t.endswith("\n") else t + "\n"

    def state_line(r):
        if not preset.get("resume") or r["type"] != "item":
            return ""
        p = world.primary(r["id"])
        s = f"state {p}"
        c = world.claim_on(r["id"]) or world.claim_on(repo.doc_of[r["id"]].alias)
        if c and p != "done":
            end = tracker.lease_end(c, tracker.claims_config(root)["lease"])
            until = tracker.rfc3339(dt.datetime.fromtimestamp(end, dt.timezone.utc))
            s += f"{MIDDLE}holder {c.get('actor')}{MIDDLE}lease {until}"
        done = r.get("_done")
        if done and p.split("/")[0] == "done":
            s += f"{MIDDLE}by {done['actor']}{MIDDLE}{done['date']}{MIDDLE}{world.backed_level.get(r['id'], 'claimed')}"
        return s + "\n"

    def entry(r):
        hdr = f"> {r['id']}{MIDDLE}{r['type']}{MIDDLE}{r['source']}:{r['line']}{MIDDLE}{r['revision'][:19]}\n"
        body = text_of(r)
        if preset.get("editable") and r["type"] != "warrant" and r["type"] != "ticket" \
                and r["id"] != repo.doc_of[r["id"]].own_id:
            body = (f"<!-- rec {r['id']} {r['source']}:{r['line']} {r['revision']} -->\n"
                    f"{body}<!-- /rec -->\n")
        return hdr + state_line(r) + body + "\n"

    def index_line(r, tier):
        return (f"[{tier}] {r['id']}{MIDDLE}{r['type']}{MIDDLE}{states.primary(r['id'])}{MIDDLE}"
                f"{show_title(r, repo)}{MIDDLE}{r['source']}:{r['line']}\n")

    tiers, entries = {}, {}
    full_all = budget_v is None
    base_len = len((head + "\n## Index\n" + "".join(index_line(r, "F") for r in recs)
                    + "\n## Records\n").encode())
    if not full_all:
        root_len = base_len + sum(len(entry(repo.by_id[r]).encode()) for r in roots if r in selected)
        if base_len > budget_v or root_len > budget_v:
            raise Refusal("context.over-budget", str(max(base_len, root_len)))
    used, cut = base_len, False
    for r in recs:
        c = container(r)
        if c and c in selected and tiers.get(c) == "F" and r["id"] != target and r["id"] not in roots:
            tiers[r["id"]] = "F"
            continue
        e = entry(r)
        if not cut and (full_all or used + len(e.encode()) <= budget_v):
            tiers[r["id"]] = "F"
            entries[r["id"]] = e
            used += len(e.encode())
        else:
            cut = True
            tiers[r["id"]] = "S"
    out = head + "\n## Index\n" + "".join(index_line(r, tiers[r["id"]]) for r in recs)
    out += "\n## Records\n" + "".join(entries[r["id"]] for r in recs if r["id"] in entries)
    if preset.get("resume"):
        nxt = [n for n in world.next() if n.startswith(wdoc.alias + "/") or n == wdoc.alias][:3]
        out += "## Next\n" + "".join(
            f"- {n}{MIDDLE}{show_title(repo.by_id[n], repo)}\n" for n in nxt) + "\n"
    if as_json:
        packet = {"schema": "oh.war/context/v1", "target": target,
                  "budget": budget_v if budget_v is not None else "none",
                  "bytes": len(out.encode()), "depth": preset["depth"], "walk": preset["walk"],
                  "preset": preset_name, "selection_digest": sel_digest, "records": [],
                  "diagnostics": [{"rule": a, "record": b, "message": ""} for a, b in sorted(diags)]}
        for r in recs:
            x = {"id": r["id"], "type": r["type"], "title": show_title(r, repo),
                 "state": states.primary(r["id"]), "source": r["source"], "line": r["line"],
                 "revision": r["revision"], "distance": dist[r["id"]], "tier":
                     "full" if tiers[r["id"]] == "F" else "summary"}
            if r["id"] in via:
                x["via"] = {"step": via[r["id"]][0], "from": via[r["id"]][1]}
            if x["tier"] == "full":
                x["text"] = text_of(r)
            packet["records"].append(x)
        return json.dumps(packet, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n"
    return out


def profile_preset(repo, name):
    """10.4: a preset a profile declares."""
    for p in repo.profiles.values():
        if p is None:
            continue
        for pr in p.presets:
            if pr.get("name") == name:
                b = pr.get("budget", 16000)
                return {"walk": name, "steps": pr.get("steps", CORE_STEPS), "depth": pr.get("depth", 3),
                        "budget": None if b == "none" else int(b),
                        "omit_types": pr.get("omit_types", []), "omit_fields": pr.get("omit_fields", []),
                        "editable": pr.get("editable", False), "reads_state": pr.get("reads_state", False)}
    raise Refusal("context.target-unknown")


def main(argv):
    ap = argparse.ArgumentParser()
    ap.add_argument("root")
    ap.add_argument("target", nargs="?")
    ap.add_argument("--index", action="store_true")
    ap.add_argument("--budget", type=int)
    ap.add_argument("--tokens", type=int)
    ap.add_argument("--depth", type=int)
    ap.add_argument("--preset", default="agent")
    ap.add_argument("--at")
    ap.add_argument("--json", action="store_true")
    a = ap.parse_args(argv[1:])
    out = {"output": "", "findings": []}
    try:
        if a.index:
            out["output"] = build_index(a.root)
        else:
            budget = a.budget if a.budget is not None else (4 * a.tokens if a.tokens is not None else None)
            out["output"] = compile_packet(a.root, a.target, a.preset, budget, a.depth, a.at, a.json)
    except Refusal as r:
        out["findings"].append(r.rule)
    except tracker.Stop as s:
        out["findings"].append(s.finding())
    json.dump(out, sys.stdout, ensure_ascii=False, sort_keys=True, indent=1)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
