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
    return authorized_answer(fields, matches[0], q)


def authorized_answer(fields, actor, q):
    require(isinstance(fields, dict) and set(fields) == {"question_sha256", "answer", "evidence"},
            "Expected question digest, answer and evidence only", 400)
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


class Answers:
    """Immutable answer storage under the execution lock and protected state root."""
    def __init__(self, executor, config):
        self.executor = executor
        self.rows = responders(config)
        self.authorization_digest = digest(config)
        self.root = executor.root / "hotline-answers"
        require(not self.root.is_symlink(), "Hotline answer directory cannot be a symlink")
        self.root.mkdir(mode=0o700, exist_ok=True)
        require(self.root.stat().st_mode & 0o077 == 0, "Private hotline answer directory required")

    def read(self, attempt_id, q):
        path = self.root / (attempt_id + ".json")
        if not path.exists() and not path.is_symlink():
            return None
        envelope = self.executor.decode(self.executor.read_file(path))
        require(isinstance(envelope, dict) and set(envelope) == {"record", "sha256"}
                and isinstance(envelope["record"], dict)
                and envelope["sha256"] == digest(envelope["record"]), "Hotline answer integrity mismatch")
        r = envelope["record"]
        require(r.get("attempt_id") == attempt_id and r.get("question_sha256") == digest(q)
                and r.get("schema") == "oh.war/hotline-answer/v1", "Hotline answer basis mismatch")
        return r

    def basis(self, attempt_id):
        records = self.executor.records()
        require(attempt_id in records, "Unknown hotline attempt", 404)
        r = self.executor.view(records[attempt_id])
        q = r.get("question")
        require(isinstance(q, dict) and r["sequence"] == 2
                and r["execution_state"] == "stopped" and r["work_state"] == "blocked",
                "Attempt has no stopped hotline question")
        policy = self.executor.config["warrants"].get(r["warrant_id"])
        require(policy is not None and digest(policy) == q["policy_sha256"]
                and policy["source_sha256"] == q["source_sha256"]
                and self.executor.store.get(r["warrant_id"])["source_sha256"] == q["source_sha256"],
                "Question source or policy changed")
        return r, q

    def listing(self):
        with self.executor.lock:
            result = []
            records = self.executor.records()
            for r in records.values():
                if r.get("question"):
                    q = r["question"]
                    candidates = [{"id": actor["id"], "kind": actor["kind"]} for actor in self.rows
                                  if (not q["question"]["direct_human"] or actor["kind"] == "human")
                                  and (q["question"]["kind"] != "governing"
                                       or q["warrant_id"] in actor["governing_warrants"])]
                    children = [child["attempt_id"] for child in records.values()
                                if child.get("resume_from") == r["attempt_id"]]
                    require(len(children) <= 1, "Multiple retained resume claims")
                    result.append({"question": q, "question_sha256": digest(q),
                                   "answer": self.read(r["attempt_id"], q),
                                   "eligible_responders": sorted(candidates, key=lambda a: a["kind"] != "ai"),
                                   "resumed_attempt": children[0] if children else None})
            return {"questions": result}

    def submit(self, attempt_id, fields, credential):
        with self.executor.lock:
            _, q = self.basis(attempt_id)
            r = answer(fields, credential, self.rows, q)
            return self.retain(attempt_id, q, r)

    def record_advice(self, attempt_id, fields, respondent):
        with self.executor.lock:
            _, q = self.basis(attempt_id)
            require(q["question"]["kind"] == "technical" and not q["question"]["direct_human"],
                    "Automatic advice is limited to technical questions", 403)
            actors = [r for r in self.rows if r["id"] == respondent and r["kind"] == "ai"]
            require(len(actors) == 1, "Configured AI adviser unavailable", 403)
            return self.retain(attempt_id, q, authorized_answer(fields, actors[0], q))

    def retain(self, attempt_id, q, r):
        r.update(attempt_id=attempt_id, authorization_sha256=self.authorization_digest)
        previous = self.read(attempt_id, q)
        if previous is not None:
            require(previous == r, "Question already has a different retained answer")
            return previous
        require(len(list(self.root.iterdir())) < 256, "Hotline answer inventory limit exceeded")
        data = json.dumps({"record": r, "sha256": digest(r)}, ensure_ascii=False).encode()
        self.executor.publish(self.root / (attempt_id + ".json"), data)
        return r
