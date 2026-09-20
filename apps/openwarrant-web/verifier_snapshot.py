# SPDX-License-Identifier: Apache-2.0
"""Read verifier inputs from configured files and retained executor state."""
import copy
from pathlib import Path

from reporting import eligible
from verification import require, identity
from verifier_policy import policy


class Snapshot:
    """Internal controller reader. Paths come from service configuration, not clients.

    Caller holds executor.lock for the full snapshot/consume transition. Protected
    configuration and state require harness enforcement; filesystem reads alone do
    not isolate an agent sharing the service account.
    """

    def __init__(self, executor, attempt_id, config_path, issuer_path):
        require(identity(attempt_id), "Exact execution attempt identity required")
        self.executor, self.attempt_id = executor, attempt_id
        self.config_path, self.issuer_path = config_path, issuer_path

    def __call__(self):
        e = self.executor
        config = policy(e.decode(e.read_file(self.config_path, 65536)))
        issuer = e.decode(e.read_file(self.issuer_path, 16384))
        require(isinstance(issuer, dict) and set(issuer) == {"schema", "public_key", "principal"}
                and issuer["schema"] == "oh.war/verifier-issuer/v1", "Exact configured protection issuer required")
        records = {id: e.view(row) for id, row in e.records().items()}
        require(self.attempt_id in records, "Execution attempt unavailable")
        attempt = records[self.attempt_id]
        warrant = attempt["warrant_id"]
        require(warrant in e.config["warrants"], "Warrant no longer configured")
        execution_policy = e.config["warrants"][warrant]
        require(eligible(attempt, execution_policy), "Current completed execution required")
        require(all(row["execution_state"] == "stopped" for row in records.values()
                    if row["warrant_id"] == warrant), "Active or unknown writer blocks verification")
        for dependency in execution_policy["dependencies"]:
            require(dependency in e.config["warrants"] and any(
                row["warrant_id"] == dependency and eligible(row, e.config["warrants"][dependency])
                for row in records.values()), "Current dependency completion unavailable")
            require(all(row["execution_state"] == "stopped" for row in records.values()
                        if row["warrant_id"] == dependency), "Dependency writer active or unknown")
        source = e.store.get(warrant)
        require(source["source_sha256"] == execution_policy["source_sha256"], "Warrant source changed")
        expected_path = e.root / ("worktree-" + warrant)
        require(isinstance(attempt.get("worktree"), str) and Path(attempt["worktree"]) == expected_path
                and not expected_path.is_symlink(), "Recorded candidate is not the configured Warrant worktree")
        require(attempt.get("harness_argv") == e.config["argv"], "Execution harness configuration changed")
        return copy.deepcopy({"config": config, "attempt": attempt, "execution_policy": execution_policy,
                              "source_sha256": source["source_sha256"], "source_path": str(expected_path),
                              "issuer": {k: issuer[k] for k in ("public_key", "principal")}})
