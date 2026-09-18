# SPDX-License-Identifier: Apache-2.0
"""Owner-configured harness dispatch. This controller is not an agent sandbox."""

import hashlib
import json
import os
import re
import subprocess
import threading
import time
import uuid
from pathlib import Path

from reporting import eligible, render
from harness import bounded_command
from hotline import question

LIMIT = 1024 * 1024


class ExecutionError(Exception):
    def __init__(self, status, message):
        self.status, self.message = status, message


def require(condition, message, status=409):
    if not condition:
        raise ExecutionError(status, message)


def argv(value):
    return (
        isinstance(value, list)
        and 0 < len(value) <= 64
        and all(
            isinstance(s, str) and s and len(s) <= 8192 and "\0" not in s for s in value
        )
    )


class Executor:
    def __init__(self, store, config_path, read_file, publish, decode):
        self.store = store
        self.decode = decode
        self.read_file, self.publish = read_file, publish
        self.lock = threading.Lock()
        self.live = set()
        self.root = store.path / ".execution"
        require(not self.root.is_symlink(), "Execution directory cannot be a symlink")
        self.root.mkdir(mode=0o700, exist_ok=True)
        require(
            self.root.stat().st_mode & 0o077 == 0,
            "Private execution directory required",
        )
        self.config = decode(read_file(config_path, 64 * 1024))
        c = self.config
        require(
            isinstance(c, dict)
            and set(c)
            == {
                "schema",
                "argv",
                "sandbox",
                "cost_mode",
                "spend_limit_usd",
                "timeout_seconds",
                "repair_cycles",
                "warrants",
            },
            "Invalid execution configuration",
        )
        require(
            c["schema"] == "oh.war/execution-config/v1" and argv(c["argv"]),
            "Invalid harness command",
        )
        require(
            c["sandbox"] == "harness-worktree-only",
            "Harness must enforce worktree isolation and protected control storage",
        )
        require(c["cost_mode"] in ("free", "unknown"), "Unsupported cost mode")
        cap = c["spend_limit_usd"]
        require(
            cap is None or type(cap) in (int, float) and 0 <= cap <= 1000000,
            "Invalid spend limit",
        )
        require(
            type(c["timeout_seconds"]) is int and 1 <= c["timeout_seconds"] <= 7200,
            "Invalid time limit",
        )
        require(
            type(c["repair_cycles"]) is int and 0 <= c["repair_cycles"] <= 20,
            "Invalid repair limit",
        )
        require(
            isinstance(c["warrants"], dict) and len(c["warrants"]) <= 256,
            "Invalid execution inventory",
        )
        for id, p in c["warrants"].items():
            require(
                re.fullmatch(r"[0-9a-f-]{36}", id) is not None,
                "Invalid configured identity",
            )
            require(
                isinstance(p, dict)
                and set(p)
                == {
                    "source_sha256",
                    "base_commit",
                    "verified_start",
                    "dependencies",
                    "checks",
                },
                "Invalid start policy",
            )
            require(
                isinstance(p["source_sha256"], str)
                and re.fullmatch("[0-9a-f]{64}", p["source_sha256"]) is not None,
                "Invalid source digest",
            )
            require(
                isinstance(p["base_commit"], str)
                and re.fullmatch("[0-9a-f]{40,64}", p["base_commit"]) is not None,
                "Exact Git commit required",
            )
            require(
                type(p["verified_start"]) is bool
                and isinstance(p["dependencies"], list)
                and all(
                    isinstance(d, str) and d in c["warrants"] for d in p["dependencies"]
                ),
                "Invalid start requirements",
            )
            require(
                isinstance(p["checks"], list)
                and 1 <= len(p["checks"]) <= 16
                and all(argv(x) for x in p["checks"]),
                "Required bounded checks missing",
            )

    def records(self):
        entries = list(self.root.glob("*.json"))
        require(len(entries) <= 1024, "Execution record limit exceeded")
        latest = {}
        for path in entries:
            require(
                re.fullmatch(r"[0-9a-f-]{36}\.[12]\.json", path.name) is not None,
                "Unknown execution record",
            )
            envelope = self.decode(self.read_file(path))
            require(
                isinstance(envelope, dict)
                and set(envelope) == {"payload", "sha256"}
                and isinstance(envelope["payload"], str),
                "Invalid execution record",
            )
            require(
                hashlib.sha256(envelope["payload"].encode()).hexdigest()
                == envelope["sha256"],
                "Execution record integrity mismatch",
            )
            r = self.decode(envelope["payload"])
            require(
                r["attempt_id"] == path.name[:36]
                and r["sequence"] == int(path.name[37]),
                "Execution record identity mismatch",
            )
            if (
                r["attempt_id"] not in latest
                or r["sequence"] > latest[r["attempt_id"]]["sequence"]
            ):
                latest[r["attempt_id"]] = r
        return latest

    def save(self, record):
        payload = json.dumps(record, ensure_ascii=False, indent=2) + "\n"
        data = json.dumps(
            {
                "payload": payload,
                "sha256": hashlib.sha256(payload.encode()).hexdigest(),
            },
            ensure_ascii=False,
        ).encode()
        require(len(data) <= LIMIT, "Execution record limit exceeded")
        self.publish(
            self.root / (record["attempt_id"] + f".{record['sequence']}.json"), data
        )

    def view(self, r):
        r = dict(r)
        if r["execution_state"] == "running" and r["attempt_id"] not in self.live:
            r.update(
                execution_state="unknown",
                work_state="blocked",
                cause="Service interrupted; old writer requires external fencing before replacement",
            )
        return r

    def listing(self):
        with self.lock:
            return {"runs": [self.view(r) for r in self.records().values()]}

    def get(self, id):
        with self.lock:
            records = self.records()
            require(id in records, "Unknown attempt", 404)
            return self.view(records[id])

    def report(self, id, word, detail):
        with self.lock:
            records = {k: self.view(r) for k, r in self.records().items()}
            require(id in records, "Unknown attempt", 404)
            return render(records, self.config["warrants"], id, word, detail)

    def git(self, *args, cwd=None):
        return subprocess.check_output(
            ["git", "-c", "core.hooksPath=/dev/null", *args],
            cwd=cwd or self.store.sdk.repo,
            stderr=subprocess.PIPE,
            timeout=10,
            text=True,
        ).strip()

    def _check_start(self, fields):
        require(
            isinstance(fields, dict) and set(fields) == {"warrant_id", "source_sha256"},
            "Expected exact Warrant identity and source digest",
            400,
        )
        id = fields["warrant_id"]
        require(
            isinstance(id, str) and id in self.config["warrants"],
            "Warrant not configured for execution",
            403,
        )
        p = self.config["warrants"][id]
        record = self.store.get(id)
        require(
            record["source_sha256"]
            == fields["source_sha256"]
            == p["source_sha256"],
            "Changed subject; review execution configuration",
        )
        require(
            not p["verified_start"],
            "Verified-start requirement unsupported by this unverified workflow",
            403,
        )
        require(
            self.config["cost_mode"] == "free"
            or self.config["spend_limit_usd"] is None,
            "Unknown cost cannot satisfy a hard spend cap",
            403,
        )
        existing = self.records()
        require(len(existing) < 256, "Attempt inventory limit exceeded")
        attempts = [
            self.view(r) for r in existing.values() if r["warrant_id"] == id
        ]
        require(
            not any(r["execution_state"] != "stopped" for r in attempts),
            "Existing writer running or unknown; replacement refused",
        )
        require(
            not any(r.get("question") for r in attempts),
            "Hotline question requires an eligible answer and explicit resume",
        )
        require(
            not any(
                eligible(r, p)
                for r in attempts
            ),
            "Exact subject already completed",
        )
        require(
            len(attempts) < 1 + self.config["repair_cycles"],
            "Repair cycle limit reached",
        )
        for dep in p["dependencies"]:
            require(
                any(
                    r["warrant_id"] == dep
                    and eligible(r, self.config["warrants"][dep])
                    for r in existing.values()
                ),
                "Required dependency is incomplete",
            )
        require(
            self.git("rev-parse", p["base_commit"] + "^{commit}")
            == p["base_commit"],
            "Base commit unavailable",
        )
        return id, p, record

    def admission(self, fields):
        """Advisory snapshot. Never reserves a writer or authorizes dispatch."""
        with self.lock:
            try:
                self._check_start(fields)
            except ExecutionError as error:
                if error.status == 400:
                    raise
                state, reason = "blocked", error.message
            except (OSError, KeyError, TypeError, ValueError, subprocess.SubprocessError):
                state, reason = "unknown", "Stored inputs or Git base unavailable"
            else:
                state, reason = "ready", "Configured start requirements satisfied"
            return {
                "schema": "oh.war/start-preview/v1",
                "subject": fields,
                "state": state,
                "reason": reason,
                "dispatch_permitted": False,
                "qualified": False,
                "remaining_checks": ["worktree identity", "exclusive writer claim", "harness launch"],
            }

    def start(self, fields):
        # Re-evaluate live facts under the same lock used by authoring. A preview
        # is not a grant, reservation, or substitute for this check.
        with self.lock:
            id, p, record = self._check_start(fields)
            worktree = self.root / ("worktree-" + id)
            if worktree.exists():
                require(
                    not worktree.is_symlink()
                    and self.git("rev-parse", "--show-toplevel", cwd=worktree)
                    == str(worktree),
                    "Worktree identity mismatch",
                )
                self.git(
                    "merge-base",
                    "--is-ancestor",
                    p["base_commit"],
                    "HEAD",
                    cwd=worktree,
                )
            common = Path(
                self.git("rev-parse", "--path-format=absolute", "--git-common-dir")
            )
            registry = common / "openwarrant-execution"
            require(not registry.is_symlink(), "Execution registry cannot be a symlink")
            registry.mkdir(mode=0o700, exist_ok=True)
            require(
                registry.stat().st_mode & 0o077 == 0,
                "Private execution registry required",
            )
            binding = registry / (id + ".json")
            binding_bytes = json.dumps(
                {"worktree": str(worktree), "state": str(self.store.path)}
            ).encode()
            try:
                self.publish(binding, binding_bytes)
            except FileExistsError:
                require(
                    self.read_file(binding) == binding_bytes,
                    "Warrant already belongs to another execution store",
                )
            attempt = str(uuid.uuid4())
            r = {
                "schema": "oh.war/execution-attempt/v1",
                "attempt_id": attempt,
                "sequence": 1,
                "warrant_id": id,
                "source_sha256": p["source_sha256"],
                "base_commit": p["base_commit"],
                "worktree": str(worktree),
                "work_state": "in-progress",
                "execution_state": "running",
                "cause": "Start requested",
                "qualified": False,
                "cost_usd": 0 if self.config["cost_mode"] == "free" else None,
                "result_revision": None,
                "notes": "",
                "next_steps": [],
                "checks": [],
                "policy": p,
                "harness_argv": self.config["argv"],
                "created_at_unix": time.time(),
            }
            self.save(r)
            self.live.add(attempt)
            threading.Thread(target=self.run, args=(r, record, p), daemon=True).start()
            return r

    def run(self, r, draft, policy):
        r = dict(r)
        deadline = time.monotonic() + self.config["timeout_seconds"]
        try:
            worktree = Path(r["worktree"])
            if not worktree.exists():
                self.git("worktree", "add", "--detach", str(worktree), r["base_commit"])
            request = {
                "schema": "oh.war/execution-request/v1",
                "attempt_id": r["attempt_id"],
                "warrant_id": r["warrant_id"],
                "source_sha256": r["source_sha256"],
                "source": draft["source"],
                "worktree": str(worktree),
                "base_commit": r["base_commit"],
                "qualification": "unverified",
                "skill": "war start",
                "limits": {
                    "timeout_seconds": self.config["timeout_seconds"],
                    "spend_limit_usd": self.config["spend_limit_usd"],
                },
            }
            code, out, _err = bounded_command(
                self.config["argv"], worktree, json.dumps(request).encode(), deadline, r
            )
            require(code == 0, "Harness exited unsuccessfully")
            result = self.decode(out)
            if isinstance(result, dict) and result.get("schema") == "oh.war/execution-question/v1":
                require(
                    not self.git("status", "--porcelain", "--untracked-files=all", cwd=worktree),
                    "Question requires a clean committed checkpoint",
                )
                checkpoint = self.git("rev-parse", "HEAD", cwd=worktree)
                self.git("merge-base", "--is-ancestor", r["base_commit"], checkpoint, cwd=worktree)
                q = question(result, r, checkpoint)
                r.update(question=q, work_state="blocked", execution_state="stopped",
                         cause="Waiting for hotline answer", notes=q["notes"],
                         next_steps=q["next_steps"])
                return
            require(
                isinstance(result, dict)
                and set(result)
                == {
                    "schema",
                    "attempt_id",
                    "source_sha256",
                    "work_state",
                    "notes",
                    "next_steps",
                },
                "Invalid harness result",
            )
            require(
                result["schema"] == "oh.war/execution-result/v1"
                and result["attempt_id"] == r["attempt_id"]
                and result["source_sha256"] == r["source_sha256"],
                "Harness result subject mismatch",
            )
            require(
                result["work_state"] in ("completed", "blocked", "failed")
                and isinstance(result["notes"], str)
                and len(result["notes"]) <= 16000
                and isinstance(result["next_steps"], list)
                and len(result["next_steps"]) <= 32
                and all(
                    isinstance(s, str) and len(s) <= 2000 for s in result["next_steps"]
                ),
                "Invalid completion fields",
            )
            r.update(
                work_state=result["work_state"],
                notes=result["notes"],
                next_steps=result["next_steps"],
            )
            if r["work_state"] == "completed":
                require(
                    not self.git(
                        "status", "--porcelain", "--untracked-files=all", cwd=worktree
                    ),
                    "Completion requires a clean committed worktree",
                )
                revision = self.git("rev-parse", "HEAD", cwd=worktree)
                self.git(
                    "merge-base",
                    "--is-ancestor",
                    r["base_commit"],
                    revision,
                    cwd=worktree,
                )
                for check in policy["checks"]:
                    observed = {"argv": check, "exit_code": None}
                    r["checks"].append(observed)
                    code, _out, _err = bounded_command(
                        check, worktree, b"", deadline, observed
                    )
                    observed["exit_code"] = code
                    require(code == 0, "Required check failed")
                require(
                    self.git("rev-parse", "HEAD", cwd=worktree) == revision
                    and not self.git(
                        "status", "--porcelain", "--untracked-files=all", cwd=worktree
                    ),
                    "Checks changed result revision or worktree",
                )
                r["result_revision"] = revision
            r.update(
                execution_state="stopped",
                cause="Work finished"
                if r["work_state"] == "completed"
                else "Harness reported incomplete work",
            )
        except TimeoutError as e:
            r.update(work_state="blocked", execution_state="unknown", cause=str(e))
        except Exception as e:  # noqa: BLE001 -- worker boundary must record unexpected failures
            # All subprocess error paths terminate the managed group. A harness
            # protocol failure provides no assurance that remote/escaped writers ended.
            r.update(
                work_state="failed",
                execution_state="unknown",
                cause=type(e).__name__ + ": " + str(e)[:1000],
            )
        finally:
            r["sequence"] = 2
            r["finished_at_unix"] = time.time()
            with self.lock:
                try:
                    try:
                        self.save(r)
                    except ExecutionError:
                        r.update(
                            work_state="blocked",
                            execution_state="unknown",
                            cause="Detailed result exceeded storage bound; review required",
                            result_revision=None,
                            notes="",
                            next_steps=[],
                            checks=[],
                            policy={},
                            harness_argv=[],
                        )
                        self.save(r)
                finally:
                    self.live.discard(r["attempt_id"])
