# SPDX-License-Identifier: Apache-2.0
"""Workflow hotline contracts. Advice is never a signature or permission grant."""
import hashlib
import hmac
import json
import re


class HotlineError(ValueError):
    def __init__(self, message, status=409):
        super().__init__(message)
        self.message, self.status = message, status


def require(condition, message, status=409):
    if not condition:
        raise HotlineError(message, status)


def text(value, limit):
    return isinstance(value, str) and bool(value.strip()) and len(value.encode("utf-8")) <= limit


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":"),
                                      ensure_ascii=False).encode()).hexdigest()


def question(result, attempt, checkpoint):
    """Bind a stopped harness question to controller-observed work and policy."""
    require(isinstance(result, dict) and set(result) == {
        "schema", "attempt_id", "source_sha256", "question", "notes", "next_steps"
    }, "Invalid question result")
    require(result["schema"] == "oh.war/execution-question/v1"
            and result["attempt_id"] == attempt["attempt_id"]
            and result["source_sha256"] == attempt["source_sha256"],
            "Question subject mismatch")
    q = result["question"]
    require(isinstance(q, dict) and set(q) == {
        "kind", "text", "direct_human", "affected_stages"
    }, "Invalid question fields")
    require(q["kind"] in ("technical", "governing") and text(q["text"], 16000)
            and type(q["direct_human"]) is bool, "Invalid question kind or text")
    stages = q["affected_stages"]
    require(isinstance(stages, list) and 1 <= len(stages) <= 32
            and all(isinstance(s, str) and re.fullmatch(r"[A-Za-z0-9_-]{1,128}", s) for s in stages)
            and len(set(stages)) == len(stages), "Invalid affected stages")
    require(text(result["notes"], 16000) and isinstance(result["next_steps"], list)
            and len(result["next_steps"]) <= 32
            and all(text(s, 2000) for s in result["next_steps"]), "Invalid question progress notes")
    require(isinstance(checkpoint, str) and re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", checkpoint),
            "Committed question checkpoint required")
    record = {
        "schema": "oh.war/hotline-question/v1", "attempt_id": attempt["attempt_id"],
        "warrant_id": attempt["warrant_id"], "source_sha256": attempt["source_sha256"],
        "policy_sha256": digest(attempt["policy"]), "checkpoint": checkpoint,
        "question": q, "notes": result["notes"], "next_steps": result["next_steps"],
    }
    return json.loads(json.dumps(record))  # Snapshot, never retain mutable caller containers.


def responders(config):
    require(isinstance(config, dict) and set(config) == {"schema", "responders"}
            and config["schema"] == "oh.war/hotline-config/v1", "Invalid hotline configuration")
    rows = config["responders"]
    require(isinstance(rows, list) and len(rows) <= 32, "Invalid responder inventory")
    names, tokens = set(), set()
    for r in rows:
        require(isinstance(r, dict) and set(r) == {"id", "kind", "token_sha256", "governing_warrants"},
                "Invalid responder fields")
        require(text(r["id"], 128) and r["id"] not in names and r["kind"] in ("human", "ai"),
                "Invalid responder identity")
        token = r["token_sha256"]
        require(isinstance(token, str) and re.fullmatch(r"[0-9a-f]{64}", token)
                and token not in tokens, "Distinct responder credential required")
        grants = r["governing_warrants"]
        require(isinstance(grants, list) and len(grants) <= 256
                and all(isinstance(w, str) and re.fullmatch(r"[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}", w) for w in grants)
                and len(set(grants)) == len(grants), "Invalid governing answer scope")
        names.add(r["id"]); tokens.add(token)
    return json.loads(json.dumps(rows))


def answer(fields, token, rows, q):
    """Resolve responder from credential; caller cannot choose its own identity."""
    require(isinstance(fields, dict) and set(fields) == {"question_sha256", "answer", "evidence"},
            "Expected question digest, answer and evidence only", 400)
    require(isinstance(token, str) and 32 <= len(token) <= 256, "Responder authentication required", 401)
    token_hash = hashlib.sha256(token.encode()).hexdigest()
    matches = [r for r in rows if hmac.compare_digest(r["token_sha256"], token_hash)]
    require(len(matches) == 1, "Responder authentication required", 401)
    actor = matches[0]
    require(fields["question_sha256"] == digest(q), "Question basis changed")
    kind = q["question"]["kind"]
    require(not q["question"]["direct_human"] or actor["kind"] == "human",
            "Direct human response required", 403)
    require(kind != "governing" or q["warrant_id"] in actor["governing_warrants"],
            "Responder lacks governing answer scope", 403)
    require(text(fields["answer"], 16000) and isinstance(fields["evidence"], list)
            and len(fields["evidence"]) <= 32 and all(text(e, 2000) for e in fields["evidence"]),
            "Invalid answer or evidence", 400)
    return {"schema": "oh.war/hotline-answer/v1", "question_sha256": digest(q),
            "respondent": actor["id"], "respondent_kind": actor["kind"],
            "answer": fields["answer"], "evidence": list(fields["evidence"]),
            "qualified": False}
