# SPDX-License-Identifier: Apache-2.0
"""Workflow verifier protocol. Validation is not proof of harness independence."""
import hashlib
import json
import re
import uuid

from harness import argv


class VerificationError(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise VerificationError(message)


def bounded(value, maximum):
    return isinstance(value, str) and bool(value.strip()) and len(value.encode()) <= maximum


def identity(value):
    try:
        return isinstance(value, str) and str(uuid.UUID(value)) == value
    except (ValueError, AttributeError):
        return False


def request(value):
    """Validate trusted controller input; caller must establish configured identities."""
    require(isinstance(value, dict), "Invalid verification request")
    version = value.get('schema')
    require(version in ('oh.war/verification-request/v1', 'oh.war/verification-request/v2') and set(value) == {
        "schema", "verification_id", "warrant_id", "source_sha256", "candidate_revision",
        "policy_sha256", "performer", "verifier", "checks", "source"
    } | ({'recheck'} if version == 'oh.war/verification-request/v2' else set()), "Invalid verification request")
    if version == 'oh.war/verification-request/v2':
        from verifier_rebuttal import validate
        validate(value['recheck'])
        require(value['recheck']['verification_id'] != value.get('verification_id'), 'Recheck requires a new identity')
    require(identity(value["verification_id"]) and identity(value["warrant_id"]), "Exact request identities required")
    require(all(isinstance(value[k], str) and re.fullmatch(r"[0-9a-f]{64}", value[k])
                for k in ("source_sha256", "policy_sha256")), "Exact source and policy digests required")
    require(isinstance(value["candidate_revision"], str)
            and re.fullmatch(r"[0-9a-f]{40}|[0-9a-f]{64}", value["candidate_revision"]), "Exact candidate revision required")
    require(bounded(value["performer"], 128) and bounded(value["verifier"], 128)
            and value["performer"] != value["verifier"], "Distinct configured performer and verifier required")
    require(bounded(value["source"], 65536)
            and hashlib.sha256(value["source"].encode()).hexdigest() == value["source_sha256"],
            "Source does not match request digest")
    require(isinstance(value["checks"], list) and 1 <= len(value["checks"]) <= 16
            and all(argv(command) for command in value["checks"]), "Required verification checks missing or invalid")
    return json.loads(json.dumps(value))


def request_digest(value):
    return hashlib.sha256(json.dumps(request(value), sort_keys=True, separators=(",", ":"),
                                     ensure_ascii=False, allow_nan=False).encode()).hexdigest()


def result(value, expected):
    """Bind bounded observations to the exact request; never create an assurance mark."""
    expected = request(expected)
    require(isinstance(value, dict) and set(value) == {
        "schema", "verification_id", "request_sha256", "verdict", "summary", "findings"
    } and value["schema"] == "oh.war/verification-result/v1", "Invalid verification result")
    require(value["verification_id"] == expected["verification_id"]
            and value["request_sha256"] == request_digest(expected), "Stale or unrelated verification result")
    require(value["verdict"] in ("pass", "fail", "unknown") and bounded(value["summary"], 16000),
            "Invalid verification verdict or summary")
    findings = value["findings"]
    require(isinstance(findings, list) and len(findings) <= 64, "Bounded finding list required")
    ids = set()
    for finding in findings:
        require(isinstance(finding, dict) and set(finding) == {
            "id", "observation", "scope", "evidence", "status", "repairable"
        }, "Invalid finding fields")
        require(isinstance(finding["id"], str) and re.fullmatch(r"[A-Za-z0-9_-]{1,64}", finding["id"])
                and finding["id"] not in ids, "Distinct finding identity required")
        ids.add(finding["id"])
        require(bounded(finding["observation"], 16000) and bounded(finding["scope"], 4000)
                and isinstance(finding["evidence"], list) and 1 <= len(finding["evidence"]) <= 32
                and all(bounded(item, 2000) for item in finding["evidence"]), "Bounded finding evidence required")
        require(finding["status"] in ("violation", "unknown") and type(finding["repairable"]) is bool,
                "Invalid finding status")
        require(not finding["repairable"] or finding["status"] == "violation", "Unknown is not a repair instruction")
    require(value["verdict"] != "pass" or not findings, "Passing result cannot retain unresolved findings")
    require(value["verdict"] != "fail" or any(f["status"] == "violation" for f in findings),
            "Failed result requires observed violation")
    return json.loads(json.dumps(value))
