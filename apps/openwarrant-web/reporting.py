# SPDX-License-Identifier: Apache-2.0
"""Deterministic projections of retained execution records, never qualification."""
import hashlib
import html
import json
import re


def complete(record):
    checks = record.get("checks", [])
    expected = record.get("policy", {}).get("checks", [])
    return (record.get("sequence") == 2
            and record.get("work_state") == "completed"
            and record.get("execution_state") == "stopped"
            and isinstance(record.get("result_revision"), str)
            and re.fullmatch(r"(?:[0-9a-f]{40}|[0-9a-f]{64})", record["result_revision"]) is not None
            and bool(expected) and len(checks) == len(expected)
            and all(c.get("argv") == argv and type(c.get("exit_code")) is int
                    and c["exit_code"] == 0 for c, argv in zip(checks, expected)))


def render(records, inventory, attempt_id, word="WORK_DONE", detail="full"):
    record = records[attempt_id]
    # Snapshot includes both configured denominator and every observed attempt.
    snapshot = {"inventory": inventory, "attempts": [records[k] for k in sorted(records)]}
    digest = hashlib.sha256(json.dumps(snapshot, sort_keys=True, ensure_ascii=False,
                                     separators=(",", ":"), allow_nan=False).encode()).hexdigest()
    done = sorted(k for k, policy in inventory.items() if any(
        r["warrant_id"] == k and r["source_sha256"] == policy["source_sha256"] and complete(r)
        for r in records.values()))
    pending = sorted(set(inventory) - set(done))
    finished = complete(record)
    link = "/#attempt=" + attempt_id
    first = word if finished else "WORK_INCOMPLETE"
    lines = [first, "Progress: " + link]
    if detail == "full" or not finished:
        lines += ["Scope: " + record["warrant_id"], "Source: " + record["source_sha256"],
                  "Work: " + ("completed" if finished else "incomplete"),
                  "Execution: " + record["execution_state"], "Qualification: unverified",
                  "Snapshot: " + digest, "Notes: " + record.get("notes", ""),
                  "Cause: " + record.get("cause", ""),
                  "Next steps: " + "; ".join(record.get("next_steps", []))]
    progress = {"scope": "configured execution inventory, not whole repository",
                "completed": len(done), "total": len(inventory), "pending": pending}
    esc = lambda value: html.escape(str(value), quote=True)
    document = ('<!doctype html><html lang="en"><meta charset="utf-8">'
                '<meta name="viewport" content="width=device-width,initial-scale=1">'
                '<meta http-equiv="Content-Security-Policy" content="default-src \'none\'; style-src \'unsafe-inline\'">'
                '<title>OpenWarrant work report</title><style>body{font:16px system-ui;max-width:850px;margin:3rem auto;padding:1rem}pre{white-space:pre-wrap;overflow-wrap:anywhere}progress{width:100%}</style>'
                '<h1>' + esc(first) + '</h1><p>Qualification: unverified</p>'
                '<p>' + esc(progress["scope"]) + '</p><progress max="' + str(max(1, len(inventory)))
                + '" value="' + str(len(done)) + '"></progress><p>' + str(len(done)) + ' / '
                + str(len(inventory)) + ' complete</p><h2>Work</h2><p>'
                + esc(record["warrant_id"]) + ' · ' + esc(record["work_state"])
                + ' · execution ' + esc(record["execution_state"]) + '</p>'
                + '<h2>Implementation notes</h2><pre>' + esc(record.get("notes", ""))
                + '</pre><h2>Next steps</h2><ul>'
                + ''.join('<li>' + esc(step) + '</li>' for step in record.get("next_steps", []))
                + '</ul><h2>Pending configured work</h2><ul>'
                + ''.join('<li>' + esc(item) + '</li>' for item in pending)
                + '</ul><details><summary>Exact source, checks and recorded evidence</summary><pre>'
                + esc(json.dumps({"snapshot_sha256": digest, "attempt": record,
                                  "progress": progress}, indent=2, ensure_ascii=False)) + '</pre></details></html>')
    return {"schema": "oh.war/reference-work-report/v1", "attempt_id": attempt_id,
            "snapshot_sha256": digest, "completion_signal": word if finished else None,
            "qualified": False, "progress": progress, "text": "\n".join(lines) + "\n",
            "html": document, "overview_link": link}
