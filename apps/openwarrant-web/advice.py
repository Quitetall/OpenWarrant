# SPDX-License-Identifier: Apache-2.0
"""At-most-once configured technical advice; no automatic work resumption."""
import json
import re
import threading
import time

from execution import argv
from harness import bounded_command
from hotline import digest, require


class Adviser:
    def __init__(self, answers, config):
        self.answers, self.executor, self.config = answers, answers.executor, config
        require(isinstance(config, dict) and set(config) == {
            "schema", "argv", "respondent", "sandbox", "cost_mode", "spend_limit_usd", "timeout_seconds"
        } and config["schema"] == "oh.war/hotline-adviser/v1", "Invalid adviser configuration")
        require(argv(config["argv"]) and config["sandbox"] == "read-only-repository", "Bounded read-only adviser required")
        require(any(a["id"] == config["respondent"] and a["kind"] == "ai" for a in answers.rows),
                "Adviser must name a configured AI responder")
        require(config["cost_mode"] in ("free", "unknown"), "Invalid adviser cost mode")
        cap = config["spend_limit_usd"]
        require(cap is None or type(cap) in (int, float) and 0 <= cap <= 1000000, "Invalid adviser spend cap")
        require(type(config["timeout_seconds"]) is int and 1 <= config["timeout_seconds"] <= 7200,
                "Invalid adviser time limit")
        self.root = self.executor.root / "hotline-advice"
        require(not self.root.is_symlink(), "Advice directory cannot be a symlink")
        self.root.mkdir(mode=0o700, exist_ok=True)
        require(self.root.stat().st_mode & 0o077 == 0, "Private advice directory required")
        self.live = set()

    def records(self):
        paths = list(self.root.iterdir())
        require(len(paths) <= 768, "Advice record limit exceeded")
        latest = {}
        for path in paths:
            if path.name.startswith(".pending-"):
                continue
            require(re.fullmatch(r"[0-9a-f-]{36}\.[12]\.json", path.name), "Unknown advice record")
            e = self.executor.decode(self.executor.read_file(path))
            require(isinstance(e, dict) and set(e) == {"record", "sha256"}
                    and isinstance(e["record"], dict) and e["sha256"] == digest(e["record"]),
                    "Advice record integrity mismatch")
            r = e["record"]
            require(r["attempt_id"] == path.name[:36] and r["sequence"] == int(path.name[37])
                    and r["schema"] == "oh.war/hotline-advice-attempt/v1", "Advice record identity mismatch")
            if r["attempt_id"] not in latest or r["sequence"] > latest[r["attempt_id"]]["sequence"]:
                latest[r["attempt_id"]] = r
        return latest

    def view(self, record):
        r = dict(record)
        if r["state"] == "running" and r["attempt_id"] not in self.live:
            r.update(state="unknown", error="Interrupted adviser; inspect old process before replacement")
        return r

    def listing(self):
        with self.executor.lock:
            return {"advice": [self.view(r) for r in self.records().values()]}

    def save(self, r):
        data = json.dumps({"record": r, "sha256": digest(r)}, ensure_ascii=False).encode()
        require(len(data) <= 1024 * 1024, "Advice record too large")
        self.executor.publish(self.root / f"{r['attempt_id']}.{r['sequence']}.json", data)

    def schedule(self):
        with self.executor.lock:
            existing = self.records()
            if any(self.view(r)["state"] in ("running", "unknown") for r in existing.values()):
                return
            for prior in self.executor.records().values():
                q, id = prior.get("question"), prior["attempt_id"]
                if not q or id in existing or q["question"]["kind"] != "technical" or q["question"]["direct_human"]:
                    continue
                if self.answers.read(id, q) is not None:
                    continue
                r = {"schema": "oh.war/hotline-advice-attempt/v1", "attempt_id": id,
                     "sequence": 1, "state": "running", "question_sha256": digest(q),
                     "config_sha256": digest(self.config), "respondent": self.config["respondent"],
                     "cost_usd": 0 if self.config["cost_mode"] == "free" else None, "error": None}
                self.save(r)
                self.live.add(id)
                threading.Thread(target=self.run, args=(r,), daemon=True).start()
                return

    def run(self, original):
        r = dict(original)
        try:
            require(self.config["cost_mode"] == "free" or self.config["spend_limit_usd"] is None,
                    "Unknown advice cost cannot satisfy a hard cap", 403)
            with self.executor.lock:
                prior, q = self.answers.basis(r["attempt_id"])
                source = self.executor.store.get(prior["warrant_id"])["source"]
            request = {"schema": "oh.war/hotline-advice-request/v1", "question": q,
                       "question_sha256": digest(q), "source": source,
                       "scope": "technical advice only; no architecture or permission changes",
                       "limits": {k: self.config[k] for k in ("timeout_seconds", "spend_limit_usd")}}
            code, out, _ = bounded_command(self.config["argv"], self.executor.store.sdk.repo,
                json.dumps(request).encode(), time.monotonic() + self.config["timeout_seconds"], {})
            require(code == 0, "Adviser exited unsuccessfully")
            result = self.executor.decode(out)
            require(isinstance(result, dict) and set(result) == {"schema", "question_sha256", "answer", "evidence"}
                    and result["schema"] == "oh.war/hotline-advice/v1", "Invalid adviser result")
            self.answers.record_advice(r["attempt_id"], {k: result[k] for k in ("question_sha256", "answer", "evidence")},
                                       self.config["respondent"])
            r.update(state="answered")
        except TimeoutError:
            r.update(state="unknown", error="Advice process exceeded bounds; inspect before replacement")
        except Exception as error:
            r.update(state="failed", error=type(error).__name__ + ": " + str(error)[:500])
        finally:
            with self.executor.lock:
                r["sequence"] = 2
                try:
                    self.save(r)
                finally:
                    self.live.discard(r["attempt_id"])
            self.schedule()
