# SPDX-License-Identifier: Apache-2.0
"""Bounded verifier process seam. Caller owns admission, isolation and storage."""
import json

from harness import argv, bounded_command
from verification import request, request_digest, result, require
from verifier_workspace import prepare, unchanged


def observe(expected, source, destination, command, deadline, decode):
    expected = request(expected)
    require(argv(command), "Configured verifier command required")
    record = {"schema": "oh.war/verifier-observation/v1", "request_sha256": request_digest(expected),
              "execution_state": "unknown", "verdict": "unknown", "checks": [],
              "result": None, "cause": "Verification not completed", "qualified": False}
    try:
        workspace = prepare(source, destination, expected["candidate_revision"], deadline)
        record["workspace"] = str(workspace)
        for check in expected["checks"]:
            observation = {"argv": check, "exit_code": None}
            record["checks"].append(observation)
            code, _, _ = bounded_command(check, workspace, b"", deadline, observation)
            observation["exit_code"] = code
            unchanged(workspace, expected["candidate_revision"], deadline)
            if code != 0:
                record.update(execution_state="stopped", verdict="fail", cause="Required verification check failed")
                return record
        capture = record["harness"] = {}
        code, output, _ = bounded_command(command, workspace, json.dumps(expected).encode(), deadline, capture)
        unchanged(workspace, expected["candidate_revision"], deadline)
        require(code == 0, "Verifier exited unsuccessfully")
        observed = result(decode(output), expected)
        record.update(execution_state="stopped", verdict=observed["verdict"], result=observed,
                      cause="Verifier observations retained for exact candidate")
    except Exception as error:
        # An observation error does not establish that escaped/remote writers stopped.
        record.update(execution_state="unknown", verdict="unknown", cause=type(error).__name__ + ": " + str(error)[:1000])
    return record
