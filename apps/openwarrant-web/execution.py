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
from harness import bounded_command, argv
from hotline import question, digest as hotline_digest
from stages import plan as stage_plan, completed_from_evidence, frontier, StageError
from stage_checks import checkpoint as stage_checkpoint

LIMIT = 1024 * 1024


class ExecutionError(Exception):
    def __init__(self, status, message):
        self.status, self.message = status, message


def require(condition, message, status=409):
    if not condition:
        raise ExecutionError(status, message)


class Executor:
    def __init__(self, store, config_path, read_file, publish, decode):
        self.store = store
        self.decode = decode
        self.read_file, self.publish = read_file, publish
        self.lock = threading.RLock()
        self.live = set()
        self.adviser = None
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
            c["schema"] in ("oh.war/execution-config/v1", "oh.war/execution-config/v2") and argv(c["argv"]),
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
                } | ({"stage_plan"} if c["schema"] == "oh.war/execution-config/v2" else set()),
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

            if "stage_plan" in p:
                try:
                    stage_plan(p["stage_plan"])
                except StageError as error:
                    raise ExecutionError(400, str(error)) from error

    def stage_facts(self, id, policy, attempts, resuming=None):
        """Read retained evidence on the current clean worktree; no writer claim."""
        config = policy["stage_plan"]
        graph = {k: v["dependencies"] for k, v in config["stages"].items()}
        worktree = self.root / ("worktree-" + id)
        done = set()
        if worktree.exists():
            require(not worktree.is_symlink()
                    and self.git("rev-parse", "--show-toplevel", cwd=worktree) == str(worktree)
                    and not self.git("status", "--porcelain", "--untracked-files=all", cwd=worktree),
                    "Stage dispatch requires a clean known worktree")
            revision = self.git("rev-parse", "HEAD", cwd=worktree)
            for r in attempts:
                if r.get("policy") == policy and r.get("execution_state") == "stopped":
                    done |= completed_from_evidence(config, policy["source_sha256"], revision,
                                                    r.get("stage_checkpoint"))
        blocks = set()
        for r in attempts:
            if (r.get("question") and r["attempt_id"] != resuming
                    and not any(child.get("resume_from") == r["attempt_id"] for child in attempts)):
                blocks.update(r["question"]["question"]["affected_stages"])
        # An unresolved question invalidates completion for its affected graph.
        blocked = {row["stage"] for row in frontier(graph, question_blocks=blocks)
                   if row["question_blocked"]}
        done -= blocked
        return done, frontier(graph, completed=done, question_blocks=blocks)

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

    def _check_start(self, fields, resuming=None):
        require(
            isinstance(fields, dict) and set(fields) in ({"warrant_id", "source_sha256"},
                                                        {"warrant_id", "source_sha256", "stage"}),
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
        stage = fields.get("stage")
        require(("stage_plan" in p and isinstance(stage, str) and stage in p["stage_plan"]["stages"])
                or ("stage_plan" not in p and "stage" not in fields),
                "Select an exact configured stage for staged execution", 400)
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
        if "stage_plan" in p:
            _, rows = self.stage_facts(id, p, attempts, resuming)
            require(next(row for row in rows if row["stage"] == stage)["state"] == "ready",
                    "Stage prerequisites or hotline questions block dispatch")
        require(
            "stage_plan" in p or not any(r.get("question") and r["attempt_id"] != resuming
                    and not any(child.get("resume_from") == r["attempt_id"] for child in attempts)
                    for r in attempts),
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
            sum(not r.get("resume_from") for r in attempts if r.get("stage") == stage)
            < 1 + self.config["repair_cycles"] + (1 if resuming else 0),
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

    def stage_listing(self, id):
        """Owner-configured selection and advisory readiness for browser callers."""
        with self.lock:
            require(id in self.config["warrants"], "Warrant not configured for execution", 403)
            policy = self.config["warrants"][id]
            source = self.store.get(id)["source_sha256"]
            rows = []
            done = set()
            if "stage_plan" in policy and source == policy["source_sha256"]:
                attempts = [self.view(r) for r in self.records().values() if r["warrant_id"] == id]
                if all(r["execution_state"] == "stopped" for r in attempts):
                    try:
                        done, _ = self.stage_facts(id, policy, attempts)
                    except (ExecutionError, StageError, OSError, subprocess.SubprocessError):
                        pass  # Admission below reports the unavailable/currently blocked basis.
            for stage, spec in policy.get("stage_plan", {}).get("stages", {}).items():
                subject = {"warrant_id": id, "source_sha256": source, "stage": stage}
                preview = self.admission(subject)
                rows.append({"stage": stage, "title": spec["title"], "outcome": spec["outcome"],
                             "dependencies": spec["dependencies"], "state": "completed" if stage in done else preview["state"],
                             "reason": "Checks passed on current revision" if stage in done else preview["reason"],
                             "dispatch_permitted": False})
            return {"schema": "oh.war/stage-selection/v1", "warrant_id": id,
                    "source_sha256": source, "mode": "staged" if "stage_plan" in policy else "whole-warrant",
                    "stages": rows, "dispatch_permitted": False}

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

    def start(self, fields, resume_context=None):
        # Re-evaluate live facts under the same lock used by authoring. A preview
        # is not a grant, reservation, or substitute for this check.
        with self.lock:
            id, p, record = self._check_start(fields, resume_context["from"] if resume_context else None)
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
            remaining = resume_context["remaining"] if resume_context else self.config["timeout_seconds"]
            if "stage_plan" in p:
                used = sum(r.get("active_seconds", 0) for r in self.records().values()
                           if r["warrant_id"] == id)
                remaining = min(remaining, self.config["timeout_seconds"] - used)
                require(remaining > 0, "Warrant execution time budget exhausted")
            attempt = str(uuid.uuid4())
            r = {
                "schema": "oh.war/execution-attempt/v1",
                "attempt_id": attempt,
                "sequence": 1,
                "warrant_id": id,
                "source_sha256": p["source_sha256"],
                "base_commit": p["base_commit"],
                "input_revision": self.git("rev-parse", "HEAD", cwd=worktree) if worktree.exists() else p["base_commit"],
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
                "execution_config_sha256": hotline_digest(self.config),
                "remaining_seconds": remaining,
            }
            if "stage_plan" in p:
                done, _ = self.stage_facts(id, p, [self.view(x) for x in self.records().values()
                                                 if x["warrant_id"] == id],
                                           resume_context["from"] if resume_context else None)
                r.update(stage=fields["stage"], prior_completed_stages=sorted(done))
            if resume_context:
                r.update(resume_from=resume_context["from"], hotline_context=resume_context["history"])
            self.save(r)
            self.live.add(attempt)
            threading.Thread(target=self.run, args=(r, record, p), daemon=True).start()
            return r

    def question_checkpoint(self, attempt_id, hotline):
        """Prove checkpoint movement from recorded independent stage results only."""
        with self.lock:
            prior, q = hotline.basis(attempt_id)
            require(prior.get("execution_config_sha256") == hotline_digest(self.config),
                    "Execution configuration changed since question")
            records = [self.view(r) for r in self.records().values()
                       if r["warrant_id"] == prior["warrant_id"]]
            require(all(r["execution_state"] == "stopped" for r in records),
                    "Existing writer running or unknown; checkpoint review refused")
            require(not any(r.get("resume_from") == attempt_id for r in records),
                    "Question already resumed")
            worktree = Path(prior["worktree"])
            require(worktree.exists() and not worktree.is_symlink()
                    and self.git("rev-parse", "--show-toplevel", cwd=worktree) == str(worktree)
                    and not self.git("status", "--porcelain", "--untracked-files=all", cwd=worktree),
                    "Checkpoint review requires clean known worktree")
            revision = self.git("rev-parse", "HEAD", cwd=worktree)
            chain, cursor = [], revision
            policy = prior["policy"]
            if cursor != q["checkpoint"]:
                require("stage_plan" in policy, "Changed checkpoint requires staged execution")
                graph = {k: v["dependencies"] for k, v in policy["stage_plan"]["stages"].items()}
                blocked = {r["stage"] for r in frontier(graph, question_blocks=q["question"]["affected_stages"])
                           if r["question_blocked"]}
                visited = set()
                while cursor != q["checkpoint"]:
                    require(cursor not in visited, "Checkpoint lineage cycle")
                    visited.add(cursor)
                    candidates = []
                    for r in records:
                        if (r.get("result_revision") == cursor and r.get("input_revision") != cursor
                                and r.get("stage") in graph and r["stage"] not in blocked
                                and r.get("policy") == policy and r.get("worktree") == str(worktree)
                                and r.get("execution_config_sha256") == prior["execution_config_sha256"]
                                and r.get("created_at_unix", 0) >= prior["finished_at_unix"]
                                and r["stage"] in completed_from_evidence(policy["stage_plan"], q["source_sha256"],
                                                                        cursor, r.get("stage_checkpoint"))):
                            candidates.append(r)
                    require(len(candidates) == 1, "Checkpoint movement lacks unique independent stage evidence")
                    r = candidates[0]
                    require(isinstance(r.get("input_revision"), str)
                            and re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", r["input_revision"]),
                            "Recorded stage input revision required")
                    self.git("merge-base", "--is-ancestor", r["input_revision"], cursor, cwd=worktree)
                    chain.append({"attempt_id": r["attempt_id"], "stage": r["stage"],
                                  "from_revision": r["input_revision"], "to_revision": cursor})
                    cursor = r["input_revision"]
            return {"schema": "oh.war/question-checkpoint-review/v1", "attempt_id": attempt_id,
                    "question_sha256": hotline_digest(q), "from_revision": q["checkpoint"],
                    "to_revision": revision, "independent_stages": list(reversed(chain)),
                    "answer_reconfirmation_required": revision != q["checkpoint"],
                    "dispatch_permitted": False}

    def resume(self, attempt_id, fields, hotline):
        with self.lock:
            require(isinstance(fields, dict) and set(fields) == {"question_sha256"},
                    "Expected exact question digest", 400)
            records = self.records()
            require(attempt_id in records and isinstance(records[attempt_id].get("question"), dict),
                    "Unknown hotline question", 404)
            prior = records[attempt_id]
            require(fields["question_sha256"] == hotline_digest(prior["question"]),
                    "Question basis changed")
            children = [r for r in records.values() if r.get("resume_from") == attempt_id]
            require(len(children) <= 1, "Multiple retained resume claims")
            if children:
                return self.view(children[0])
            prior, q = hotline.basis(attempt_id)
            response = hotline.read(attempt_id, q)
            require(response is not None and response.get("authorization_sha256") == hotline.authorization_digest,
                    "Current eligible hotline answer required")
            require(prior.get("execution_config_sha256") == hotline_digest(self.config),
                    "Execution configuration changed since question")
            history = prior.get("hotline_context", []) + [{"question": q, "answer": response}]
            if "stage" in prior:
                review = self.question_checkpoint(attempt_id, hotline)
                if review["answer_reconfirmation_required"]:
                    renewed = hotline.checkpoint_answer(attempt_id, review)
                    require(renewed is not None and renewed.get("original_answer_sha256") == hotline_digest(response),
                            "Current checkpoint answer reconfirmation required")
                    history.append({"question": renewed["question"], "answer": renewed["answer"],
                                    "checkpoint_review": review})
            else:
                worktree = Path(prior["worktree"])
                require(worktree.exists() and not worktree.is_symlink()
                        and self.git("rev-parse", "--show-toplevel", cwd=worktree) == str(worktree)
                        and self.git("rev-parse", "HEAD", cwd=worktree) == q["checkpoint"]
                        and not self.git("status", "--porcelain", "--untracked-files=all", cwd=worktree),
                        "Question checkpoint changed")
            remaining = prior["remaining_seconds"] - prior["active_seconds"]
            require(remaining > 0, "Question execution time budget exhausted")
            return self.start({"warrant_id": prior["warrant_id"], "source_sha256": prior["source_sha256"],
                               **({"stage": prior["stage"]} if "stage" in prior else {})},
                              {"from": attempt_id, "remaining": remaining,
                               "history": history})

    def run(self, r, draft, policy):
        r = dict(r)
        began = time.monotonic()
        deadline = began + r["remaining_seconds"]
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
                    "timeout_seconds": r["remaining_seconds"],
                    "spend_limit_usd": self.config["spend_limit_usd"],
                },
            }
            if r.get("resume_from"):
                request.update(schema="oh.war/execution-request/v2",
                               resume_from=r["resume_from"], hotline_context=r["hotline_context"])
            if "stage" in r:
                request.update(schema="oh.war/execution-request/v3", stage=r["stage"],
                               stage_plan=policy["stage_plan"],
                               prior_completed_stages=r["prior_completed_stages"])
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
                if "stage" in r:
                    require(r["stage"] in q["question"]["affected_stages"]
                            and set(q["question"]["affected_stages"]) <= policy["stage_plan"]["stages"].keys(),
                            "Question must name current stage and only configured affected stages")
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
                if "stage" in r:
                    selected = sorted(set(r["prior_completed_stages"]) | {r["stage"]})
                    observed = stage_checkpoint(policy["stage_plan"], r["source_sha256"], revision,
                                                worktree, selected, deadline)
                    r["stage_checkpoint"] = observed
                    require(observed["execution_state"] == "stopped", "Stage checks did not stop on unchanged revision")
                    done = completed_from_evidence(policy["stage_plan"], r["source_sha256"], revision, observed)
                    if set(selected) != done:
                        r.update(work_state="failed", execution_state="stopped",
                                 cause="Required stage checks failed")
                        return
                    if done != policy["stage_plan"]["stages"].keys():
                        r.update(work_state="in-progress", execution_state="stopped",
                                 cause="Stage finished; Warrant has remaining stages", result_revision=revision)
                        return
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
            r["active_seconds"] = time.monotonic() - began
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

            if r.get("question") and self.adviser is not None:
                self.adviser.schedule()
