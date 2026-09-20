# SPDX-License-Identifier: Apache-2.0
"""Authenticate and consume an exact verifier claim before any process launch."""
import base64
import json

from hotline import digest
from verification import request, require
from verifier_attestation import authenticate
from verifier_policy import admission, policy


def consume(jobs, expected, config, attempt, execution_policy, current_source, *,
            payload, signature, public_key, principal, now):
    """Caller holds the execution lock and supplies freshly read protected inputs.

    No browser/model input may supply configuration, issuer trust or candidate state.
    True permits this caller to launch once. False means already consumed; never
    retry that launch, including after a crash between consumption and process start.
    """
    expected, config = request(expected), policy(config)
    initial = admission(config, attempt, execution_policy, current_source)
    require(initial["state"] != "blocked", initial["reason"])
    require(expected["warrant_id"] == attempt.get("warrant_id")
            and expected["candidate_revision"] == attempt.get("result_revision")
            and expected["source_sha256"] == current_source
            and expected["policy_sha256"] == digest(execution_policy)
            and expected["performer"] == config["performer"]["id"]
            and expected["verifier"] == config["verifier"]["id"]
            and expected["checks"] == execution_policy.get("checks"),
            "Verifier request differs from current protected execution inputs")
    prior = jobs.read(expected["verification_id"])
    require(prior is not None, "Prepare verifier claim before requesting protection evidence")
    # claim checks idempotency against the full retained request and admission basis.
    jobs.claim(expected, initial["basis_sha256"])
    if prior["sequence"] != 1:
        return False
    trusted = authenticate(payload, signature, public_key=public_key, principal=principal,
                           basis_sha256=initial["basis_sha256"], nonce=expected["verification_id"],
                           now=now, decode=jobs.decode)
    ready = admission(config, attempt, execution_policy, current_source, trusted_observation=trusted)
    require(ready["state"] == "ready", ready["reason"])
    receipt = {"schema": "oh.war/verifier-protection-receipt/v1",
               "payload_base64": base64.b64encode(payload).decode("ascii"),
               "signature_base64": base64.b64encode(signature).decode("ascii"),
               "public_key": public_key, "principal": principal}
    receipt_digest = digest(receipt)
    # Keep original signed bytes, not only their digest. Content-addressed files
    # tolerate concurrent identical submission and preserve rejected race losers.
    path = jobs.root / f"protection-{receipt_digest}.json"
    try:
        jobs.publish(path, json.dumps(receipt, ensure_ascii=False).encode())
    except FileExistsError:
        require(jobs.decode(jobs.read_file(path)) == receipt, "Retained protection receipt differs")
    return jobs.begin(expected["verification_id"], receipt_digest)
