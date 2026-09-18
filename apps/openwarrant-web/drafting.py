# SPDX-License-Identifier: Apache-2.0
"""Configured agent drafting. No provider credentials or document semantics here."""
import hashlib
import json
import re
import threading
import time
from pathlib import Path, PurePosixPath

from execution import ExecutionError, argv, require
from harness import bounded_command


def sha(data):
    return hashlib.sha256(data).hexdigest()


class Drafter:
    def __init__(self, store, config_path, read_file, publish, decode):
        self.store, self.read_file, self.publish, self.decode = store, read_file, publish, decode
        self.lock = threading.Lock()
        self.live = set()
        self.root = store.path / ".drafting"
        require(not self.root.is_symlink(), "Drafting directory cannot be a symlink")
        self.root.mkdir(mode=0o700, exist_ok=True)
        require(self.root.stat().st_mode & 0o077 == 0, "Private drafting directory required")
        c = self.config = decode(read_file(config_path, 65536))
        require(isinstance(c, dict) and set(c) == {
            "schema", "argv", "backend", "sandbox", "cost_mode", "spend_limit_usd",
            "timeout_seconds", "context"}, "Invalid drafting configuration")
        require(c["schema"] == "oh.war/drafting-config/v1" and argv(c["argv"]), "Invalid drafter command")
        require(isinstance(c["backend"], str) and 0 < len(c["backend"]) <= 128, "Invalid backend label")
        require(c["sandbox"] == "read-only-repository", "Drafter harness must protect repository and controller storage")
        require(c["cost_mode"] in ("free", "unknown"), "Unsupported drafting cost mode")
        cap = c["spend_limit_usd"]
        require(cap is None or type(cap) in (int, float) and 0 <= cap <= 1000000, "Invalid drafting spend cap")
        require(type(c["timeout_seconds"]) is int and 1 <= c["timeout_seconds"] <= 7200, "Invalid drafting time limit")
        require(isinstance(c["context"], list) and 1 <= len(c["context"]) <= 32, "Pinned skill/context required")
        paths = set()
        for item in c["context"]:
            require(isinstance(item, dict) and set(item) == {"path", "sha256", "role"}, "Invalid context entry")
            name = item["path"]
            require(isinstance(name, str) and name and not PurePosixPath(name).is_absolute()
                    and all(p not in ("", ".", "..") for p in name.split("/")) and "\\" not in name,
                    "Context path must stay inside repository")
            require(name not in paths, "Duplicate context path")
            paths.add(name)
            require(isinstance(item["sha256"], str) and re.fullmatch(r"[0-9a-f]{64}", item["sha256"]), "Invalid context digest")
            require(item["role"] in ("skill", "context"), "Invalid context role")
        require(sum(x["role"] == "skill" for x in c["context"]) == 1, "Exactly one pinned drafting skill required")
        self.config_digest = sha(json.dumps(c, sort_keys=True, separators=(",", ":")).encode())

    def context(self):
        result, total = [], 0
        for item in self.config["context"]:
            path = self.store.sdk.repo
            for part in item["path"].split("/"):
                path = path / part
                require(not path.is_symlink(), "Context symlinks refused")
            require(path.is_file(), "Context must be a regular file")
            data = self.read_file(path, 65536)
            total += len(data)
            require(total <= 131072, "Drafting context budget exceeded")
            require(sha(data) == item["sha256"], "Context digest changed; update configured basis")
            result.append({**item, "text": data.decode("utf-8")})
        return result

    def records(self):
        latest = {}
        paths = list(self.root.iterdir())
        require(len(paths) <= 96, "Drafting history limit exceeded")
        for path in paths:
            if path.name.startswith(".pending-"):
                continue
            require(re.fullmatch(r"[0-9a-f-]{36}\.[12]\.json", path.name), "Unknown drafting record")
            envelope = self.decode(self.read_file(path))
            require(isinstance(envelope, dict) and set(envelope) == {"payload", "sha256"}
                    and isinstance(envelope["payload"], str), "Invalid drafting record")
            require(sha(envelope["payload"].encode()) == envelope["sha256"], "Drafting record integrity mismatch")
            r = self.decode(envelope["payload"])
            require(r["schema"] == "oh.war/drafting-attempt/v1"
                    and r["request_id"] == path.name[:36] and r["sequence"] == int(path.name[37]), "Drafting record identity mismatch")
            if r["request_id"] not in latest or r["sequence"] > latest[r["request_id"]]["sequence"]:
                latest[r["request_id"]] = r
        return latest

    def view(self, record):
        r = dict(record)
        if r["state"] == "running" and r["request_id"] not in self.live:
            r.update(state="unknown", error="Service interrupted; inspect old drafter before replacement")
        return r

    def listing(self):
        with self.lock:
            return {"attempts": [self.view(r) for r in self.records().values()]}

    def get(self, id):
        with self.lock:
            records = self.records()
            require(id in records, "Unknown drafting request", 404)
            return self.view(records[id])

    def save(self, record):
        payload = json.dumps(record, ensure_ascii=False, sort_keys=True) + "\n"
        data = json.dumps({"payload": payload, "sha256": sha(payload.encode())}, ensure_ascii=False).encode()
        require(len(data) <= 1024 * 1024, "Drafting record too large")
        self.publish(self.root / f"{record['request_id']}.{record['sequence']}.json", data)

    def start(self, fields):
        require(isinstance(fields, dict) and set(fields) == {"request_id", "prompt"}, "Expected request_id and prompt only", 400)
        id, prompt = fields["request_id"], fields["prompt"]
        require(isinstance(id, str) and re.fullmatch(r"[0-9a-f]{8}(?:-[0-9a-f]{4}){3}-[0-9a-f]{12}", id), "Invalid request identity", 400)
        require(isinstance(prompt, str) and prompt.strip() and len(prompt.encode()) <= 16000, "Bounded prompt required", 400)
        with self.lock:
            records = self.records()
            if id in records:
                r = records[id]
                require(r["request"]["prompt"] == prompt and r["config_sha256"] == self.config_digest, "Request identity already names different input")
                return self.view(r)
            require(len(records) < 32, "Drafting attempt limit exceeded")
            require(not any(self.view(r)["state"] in ("running", "unknown") for r in records.values()), "Drafter busy or previous execution unknown")
            require(self.config["cost_mode"] == "free" or self.config["spend_limit_usd"] is None, "Unknown cost cannot satisfy a hard spend cap", 403)
            request = {"schema": "oh.war/drafting-request/v1", "request_id": id,
                       "prompt": prompt, "skill": "war spec", "context": self.context(),
                       "output_contract": {"schema": "oh.war/drafting-result/v1", "fields": ["title", "outcome", "scope", "context"]},
                       "limits": {k: self.config[k] for k in ("timeout_seconds", "spend_limit_usd")}}
            r = {"schema": "oh.war/drafting-attempt/v1", "request_id": id, "sequence": 1,
                 "state": "running", "backend": self.config["backend"], "config_sha256": self.config_digest,
                 "request": request, "cost_usd": 0 if self.config["cost_mode"] == "free" else None,
                 "qualified": False, "saved": False, "error": None, "result": None}
            self.save(r)
            self.live.add(id)
            threading.Thread(target=self.run, args=(r,), daemon=True).start()
            return r

    def run(self, original):
        r = dict(original)
        try:
            capture = {}
            code, out, _ = bounded_command(self.config["argv"], self.store.sdk.repo,
                    json.dumps(r["request"]).encode(), time.monotonic() + self.config["timeout_seconds"], capture)
            require(code == 0, "Drafter exited unsuccessfully")
            result = self.decode(out)
            require(isinstance(result, dict) and set(result) == {"schema", "fields"}
                    and result["schema"] == "oh.war/drafting-result/v1", "Invalid draft result")
            fields = result["fields"]
            require(isinstance(fields, dict) and set(fields) == {"title", "outcome", "scope", "context"}, "Draft contains unsupported fields")
            require(all(isinstance(x, str) and x.strip() and len(x.encode()) <= 16000 for x in fields.values()), "Draft fields must contain bounded text")
            require(self.context() == r["request"]["context"], "Context changed during drafting")
            source = self.store.sdk.author({"id": r["request_id"], **fields}, 1)
            r.update(state="ready", result={"fields": fields, "source": source, "source_sha256": sha(source.encode())})
        except TimeoutError:
            r.update(state="unknown", error="Drafter exceeded execution bounds; inspect old worker before replacement")
        except Exception as error:
            r.update(state="failed", error=type(error).__name__ + ": " + str(error)[:500])
        finally:
            r["sequence"] = 2
            with self.lock:
                try:
                    self.save(r)
                finally:
                    self.live.discard(r["request_id"])
