"""The Jira arm: the backlog as Jira Cloud issues, read and updated by the agent over the REST API.

Each cell gets its own label (`STUDY_JIRA_LABEL`), so concurrent cells and later runs never
see one another's issues; the issues are deleted when the cell finishes. One issue per
backlog item: summary `<ID>: <title>`, the description holding the same summary,
description, acceptance criteria and notes every other arm gets, labels `<cell label>` and
`item-<id>`, and a "Blocks" link from each prerequisite. Done means the issue's status is in
Jira's done category.

Credentials come from the environment (`secrets run`): JIRA_SITE (https://<site>.atlassian.net),
JIRA_EMAIL, JIRA_API_TOKEN, JIRA_PROJECT (a project key). They are passed into the agent's
sandbox for Jira cells only, because the agent needs them to use Jira; use a Jira site or
project kept for the study.
"""

from __future__ import annotations

import base64
import json
import os
import urllib.error
import urllib.parse
import urllib.request

ENV = ("JIRA_SITE", "JIRA_EMAIL", "JIRA_API_TOKEN", "JIRA_PROJECT")

POINTER = """## Planned work

The backlog is tracked in Jira: project `{project}` at {site}, the issues labelled
`{label}`. Use the Jira REST API with `curl` (base URL in `$JIRA_SITE`, credentials in
`$JIRA_EMAIL` and `$JIRA_API_TOKEN`, basic auth): read the issues, move one to In Progress
when you start it and to Done when it is finished, and add a comment with anything the next
person needs to know.
"""


def missing() -> list[str]:
    return [k for k in ENV if not os.environ.get(k)]


class Jira:
    def __init__(self, env: dict | None = None):
        e = env or os.environ
        self.site = e["JIRA_SITE"].rstrip("/")
        self.project = e["JIRA_PROJECT"]
        token = base64.b64encode(f"{e['JIRA_EMAIL']}:{e['JIRA_API_TOKEN']}".encode()).decode()
        self.headers = {"Authorization": f"Basic {token}", "Accept": "application/json",
                        "Content-Type": "application/json"}

    def call(self, method: str, path: str, body: dict | None = None) -> dict:
        req = urllib.request.Request(self.site + path, method=method, headers=self.headers,
                                     data=json.dumps(body).encode() if body is not None else None)
        try:
            with urllib.request.urlopen(req, timeout=60) as r:
                raw = r.read()
        except urllib.error.HTTPError as err:
            raise RuntimeError(f"Jira {method} {path.split('?')[0]}: {err.code} {err.read()[:300]!r}") from None
        return json.loads(raw) if raw else {}

    def search(self, jql: str, fields: str) -> list[dict]:
        out, token = [], None
        while True:
            q = {"jql": jql, "fields": fields, "maxResults": "100"}
            if token:
                q["nextPageToken"] = token
            page = self.call("GET", "/rest/api/3/search/jql?" + urllib.parse.urlencode(q))
            out += page.get("issues", [])
            token = page.get("nextPageToken")
            if not token:
                return out

    def transition_to_done(self, key: str) -> None:
        ts = self.call("GET", f"/rest/api/3/issue/{key}/transitions")["transitions"]
        done = [t for t in ts if t.get("to", {}).get("statusCategory", {}).get("key") == "done"]
        if not done:
            raise RuntimeError(f"Jira: {key} has no transition into a done status")
        self.call("POST", f"/rest/api/3/issue/{key}/transitions", {"transition": {"id": done[0]["id"]}})


def adf(markdown: str) -> dict:
    """Plain Markdown to Atlassian Document Format: paragraphs, bullet lists, code blocks."""
    content, para, bullets, code = [], [], [], None

    def flush():
        nonlocal para, bullets
        if para:
            content.append({"type": "paragraph", "content": [{"type": "text", "text": " ".join(para)}]})
            para = []
        if bullets:
            content.append({"type": "bulletList", "content": [
                {"type": "listItem", "content": [{"type": "paragraph", "content": [{"type": "text", "text": b}]}]}
                for b in bullets]})
            bullets = []

    for line in markdown.splitlines():
        if code is not None:
            if line.strip().startswith("```"):
                content.append({"type": "codeBlock", "content": [{"type": "text", "text": "\n".join(code) or " "}]})
                code = None
            else:
                code.append(line)
            continue
        if line.strip().startswith("```"):
            flush()
            code = []
        elif line.startswith("    ") and not para and not bullets and line.strip():
            content.append({"type": "codeBlock", "content": [{"type": "text", "text": line.strip()}]})
        elif line.lstrip().startswith("- "):
            if para:
                flush()
            bullets.append(line.lstrip()[2:].strip())
        elif line.startswith("  ") and bullets and line.strip():
            bullets[-1] += " " + line.strip()
        elif not line.strip():
            flush()
        else:
            if bullets:
                flush()
            para.append(line.strip())
    flush()
    if code is not None:
        content.append({"type": "codeBlock", "content": [{"type": "text", "text": "\n".join(code) or " "}]})
    return {"type": "doc", "version": 1, "content": content or [{"type": "paragraph", "content": []}]}


def item_markdown(item: dict) -> str:
    parts = [item["summary"].strip(), "", item["description"].strip(), "", "Acceptance criteria:", ""]
    parts += [f"- {c}" for c in item["acceptance"]] + [""]
    if item.get("notes", "").strip():
        parts += ["Notes:", "", item["notes"].strip(), ""]
    deps = item.get("depends_on", [])
    parts.append("Depends on: " + (", ".join(deps) if deps else "nothing"))
    return "\n".join(parts)


def _label(item_id: str) -> str:
    return "item-" + item_id.lower()


def seed(backlog: dict, done: set[str], label: str, env: dict | None = None) -> dict[str, str]:
    j = Jira(env)
    keys: dict[str, str] = {}
    for item in backlog["item"]:
        r = j.call("POST", "/rest/api/3/issue", {"fields": {
            "project": {"key": j.project}, "issuetype": {"name": "Task"},
            "summary": f"{item['id']}: {item['title']}", "labels": [label, _label(item["id"])],
            "description": adf(item_markdown(item))}})
        keys[item["id"]] = r["key"]
    for item in backlog["item"]:
        for dep in item.get("depends_on", []):
            j.call("POST", "/rest/api/3/issueLink", {"type": {"name": "Blocks"},
                   "outwardIssue": {"key": keys[dep]}, "inwardIssue": {"key": keys[item["id"]]}})
    for iid in backlog["order"]:
        if iid in done:
            j.transition_to_done(keys[iid])
    return keys


def tracker(backlog: dict, label: str, env: dict | None = None) -> dict:
    state: dict[str, bool | None] = {i["id"]: None for i in backlog["item"]}
    j = Jira(env)
    for issue in j.search(f'project = "{j.project}" AND labels = "{label}"', "status,labels"):
        f = issue["fields"]
        for iid in state:
            if _label(iid) in (f.get("labels") or []):
                state[iid] = f["status"]["statusCategory"]["key"] == "done"
    return state


def text(label: str, env: dict | None = None) -> str:
    """Every issue's summary and description as plain text, for the parity check."""
    j = Jira(env)

    def walk(node) -> str:
        if isinstance(node, dict):
            return (node.get("text", "") + ("\n" if node.get("type") in ("paragraph", "listItem", "codeBlock") else "")
                    + "".join(walk(c) for c in node.get("content", [])))
        return ""

    out = []
    for issue in j.search(f'project = "{j.project}" AND labels = "{label}"', "summary,description"):
        f = issue["fields"]
        out.append(f["summary"] + "\n" + walk(f.get("description") or {}))
    return "\n".join(out)


def cleanup(label: str, env: dict | None = None) -> int:
    j = Jira(env)
    issues = j.search(f'project = "{j.project}" AND labels = "{label}"', "summary")
    for issue in issues:
        j.call("DELETE", f"/rest/api/3/issue/{issue['key']}")
    return len(issues)
