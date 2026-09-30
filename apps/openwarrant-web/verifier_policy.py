# SPDX-License-Identifier: Apache-2.0
"""Verifier policy validation and advisory admission, never a dispatch grant."""
import json
import math

from harness import argv
from hotline import digest
from reporting import eligible
from verification import require, bounded

PROTECTIONS = frozenset({"separate-context", "read-only-candidate", "protected-checks", "protected-control-storage"})


def policy(value):
    require(isinstance(value, dict) and set(value) == {
        "schema", "performer", "verifier", "cost_mode", "spend_limit_usd", "timeout_seconds"
    } and value["schema"] == "oh.war/verifier-config/v1", "Invalid verifier configuration")
    for role in ("performer", "verifier"):
        actor = value[role]
        require(isinstance(actor, dict) and set(actor) == {"id", "argv"}
                and bounded(actor["id"], 128) and argv(actor["argv"]), "Configured actor and command required")
    require(value["performer"]["id"] != value["verifier"]["id"], "Distinct verifier required")
    require(value["cost_mode"] in ("free", "unknown"), "Unsupported verifier cost accounting")
    cap = value["spend_limit_usd"]
    require(cap is None or type(cap) in (int, float) and math.isfinite(cap) and 0 <= cap <= 1000000,
            "Invalid verifier spend cap")
    require(type(value["timeout_seconds"]) is int and 1 <= value["timeout_seconds"] <= 7200,
            "Finite verifier time limit required")
    return json.loads(json.dumps(value))


def admission(config, attempt, execution_policy, current_source, *, trusted_observation=None):
    """Caller must authenticate capability evidence outside performer/verifier output.

    This function does not establish evidence authenticity. Missing trusted evidence
    remains UNKNOWN. Browser/model-supplied dictionaries must never be passed as
    trusted_observation by the future controller.
    """
    config = policy(config)
    basis = {"attempt_id": attempt.get("attempt_id"), "candidate_revision": attempt.get("result_revision"),
             "source_sha256": current_source, "execution_policy_sha256": digest(execution_policy),
             "verifier_config_sha256": digest(config)}
    out = {"schema": "oh.war/verifier-admission/v1", "basis": basis, "basis_sha256": digest(basis),
           "state": "blocked", "reason": "", "dispatch_permitted": False, "qualified": False}
    if not eligible(attempt, execution_policy) or current_source != attempt.get("source_sha256"):
        out["reason"] = "Exact current completed candidate required"
    elif attempt.get("harness_argv") != config["performer"]["argv"]:
        out["reason"] = "Configured performer does not match retained execution"
    elif config["cost_mode"] == "unknown" and config["spend_limit_usd"] is not None:
        out["reason"] = "Unknown verifier cost cannot satisfy hard cap"
    elif trusted_observation is None:
        out.update(state="unknown", reason="Authenticated harness protection evidence unavailable")
    else:
        observed = trusted_observation
        require(isinstance(observed, dict) and set(observed) == {"basis_sha256", "evidence_ref", "protections"}
                and bounded(observed["evidence_ref"], 2000), "Invalid trusted protection observation")
        require(isinstance(observed["protections"], dict) and set(observed["protections"]) == PROTECTIONS
                and all(v in ("pass", "fail", "unknown") for v in observed["protections"].values()),
                "Exact protection observations required")
        if observed["basis_sha256"] != out["basis_sha256"]:
            out.update(state="unknown", reason="Harness protection evidence has stale basis")
        elif "fail" in observed["protections"].values():
            out["reason"] = "Required harness protection failed"
        elif "unknown" in observed["protections"].values():
            out.update(state="unknown", reason="Required harness protection unavailable")
        else:
            out.update(state="ready", reason="Candidate and authenticated protection observations match",
                       evidence_ref=observed["evidence_ref"])
    return out
