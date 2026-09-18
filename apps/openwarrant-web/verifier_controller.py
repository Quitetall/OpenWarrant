# SPDX-License-Identifier: Apache-2.0
"""Synchronous verifier lifecycle; server callers provide protected current state."""
import json
import time

from hotline import digest
from verification import request, request_digest, require
from verifier_dispatch import consume
from verifier_policy import policy
from verifier_run import observe
from verifier_workspace import unchanged


def run(jobs, expected, snapshot, lock, destination, *, payload, signature, schedule=None):
    """snapshot() runs under executor lock and must reject active/unknown writers.

    It returns protected config, attempt, execution_policy, source_sha256,
    source_path and issuer {public_key, principal}. This callable is internal;
    clients cannot supply it or any of its returned trust/configuration fields.
    The supplied job must already be prepared. An interrupted consumed job is
    never restarted by this function.
    """
    expected = request(expected)
    with lock:
        current = json.loads(json.dumps(snapshot(), allow_nan=False))
        require(isinstance(current, dict) and set(current) == {
            "config", "attempt", "execution_policy", "source_sha256", "source_path", "issuer"
        }, "Exact protected verifier snapshot required")
        config = policy(current["config"])
        deadline = time.monotonic() + config["timeout_seconds"]
        unchanged(current["source_path"], expected["candidate_revision"], deadline)
        acquired = consume(jobs, expected, config, current["attempt"], current["execution_policy"],
                           current["source_sha256"], payload=payload, signature=signature,
                           public_key=current["issuer"]["public_key"], principal=current["issuer"]["principal"],
                           now=int(time.time()))
        if not acquired:
            return jobs.view(expected["verification_id"])
        # Serialize now so later mutable caller data cannot rewrite comparison basis.
        basis = digest(current)
    def execute():
        observed = observe(expected, current["source_path"], destination, config["verifier"]["argv"],
                           deadline, jobs.decode)
        with lock:
            try:
                require(digest(snapshot()) == basis, "Candidate or protected configuration changed during verification")
                unchanged(current["source_path"], expected["candidate_revision"], deadline)
            except Exception as error:
                observed = {"schema": "oh.war/verifier-observation/v1", "request_sha256": request_digest(expected),
                            "execution_state": "unknown", "verdict": "unknown", "result": None,
                            "qualified": False, "candidate_observation": observed,
                            "cause": type(error).__name__ + ": " + str(error)[:1000]}
            jobs.finish(expected["verification_id"], observed)
            return jobs.view(expected["verification_id"])

    if schedule is not None:
        return schedule(execute)
    return execute()
