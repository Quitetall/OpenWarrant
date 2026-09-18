# SPDX-License-Identifier: Apache-2.0
"""Configured verification inventory and exact preparation, without implicit launch."""
import json
import re

from hotline import digest
from verification import identity, request, request_digest, require
from verifier_jobs import Jobs
from verifier_policy import admission
from verifier_snapshot import Snapshot


class Verification:
    def __init__(self, executor, config_path, issuer_path):
        self.executor, self.config_path, self.issuer_path = executor, config_path, issuer_path
        self.jobs = Jobs(executor.root / "verification", publish=executor.publish,
                         read_file=executor.read_file, decode=executor.decode)

    def initial(self, id):
        require(identity(id), "Exact verification identity required")
        require(self.jobs.read(id) is not None, "Unknown verification job")
        return self.jobs.decode(self.jobs.read_file(self.jobs.path(id, 1)))["record"]

    def binding(self, id):
        initial = self.initial(id)
        binding = self.jobs.decode(self.jobs.read_file(self.jobs.root / f"{id}.binding.json"))
        require(isinstance(binding, dict) and set(binding) == {"schema", "attempt_id", "request_sha256"}
                and binding["schema"] == "oh.war/verifier-binding/v1" and identity(binding["attempt_id"])
                and binding["request_sha256"] == request_digest(initial["request"]), "Verifier binding mismatch")
        return binding

    def get(self, id):
        with self.executor.lock:
            initial, binding = self.initial(id), self.binding(id)
            return {**self.jobs.view(id), "attempt_id": binding["attempt_id"],
                    "request": initial["request"], "basis_sha256": initial["basis_sha256"],
                    "dispatch_permitted": False}

    def listing(self):
        with self.executor.lock:
            ids = []
            for path in self.jobs.root.glob("*.1.json"):
                require(re.fullmatch(r"[0-9a-f-]{36}\.1\.json", path.name), "Unknown verifier claim filename")
                ids.append(path.name[:-7])
                require(len(ids) <= 256, "Verifier inventory limit exceeded")
            return {"schema": "oh.war/verification-inventory/v1", "jobs": [self.get(id) for id in sorted(ids)],
                    "qualified": False}

    def prepare(self, fields):
        require(isinstance(fields, dict) and set(fields) == {"attempt_id", "verification_id"}
                and all(identity(value) for value in fields.values()), "Exact execution and verification identities required")
        e, id = self.executor, fields["verification_id"]
        with e.lock:
            require(len(self.listing()["jobs"]) < 256 or self.jobs.path(id, 1).exists(), "Verifier inventory limit exceeded")
            snapshot = Snapshot(e, fields["attempt_id"], self.config_path, self.issuer_path)()
            config, attempt, execution_policy = snapshot["config"], snapshot["attempt"], snapshot["execution_policy"]
            preview = admission(config, attempt, execution_policy, snapshot["source_sha256"])
            require(preview["state"] != "blocked", preview["reason"])
            expected = request({"schema": "oh.war/verification-request/v1", "verification_id": id,
                "warrant_id": attempt["warrant_id"], "source_sha256": snapshot["source_sha256"],
                "candidate_revision": attempt["result_revision"], "policy_sha256": digest(execution_policy),
                "performer": config["performer"]["id"], "verifier": config["verifier"]["id"],
                "checks": execution_policy["checks"], "source": e.store.get(attempt["warrant_id"])["source"]})
            binding = {"schema": "oh.war/verifier-binding/v1", "attempt_id": fields["attempt_id"],
                       "request_sha256": request_digest(expected)}
            path = self.jobs.root / f"{id}.binding.json"
            try:
                self.jobs.publish(path, json.dumps(binding).encode())
            except FileExistsError:
                require(self.jobs.decode(self.jobs.read_file(path)) == binding, "Verification identity already bound")
            self.jobs.claim(expected, preview["basis_sha256"])
            return self.get(id)
