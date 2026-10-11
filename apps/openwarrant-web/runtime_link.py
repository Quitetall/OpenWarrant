# SPDX-License-Identifier: Apache-2.0
"""Link a configured workbench subject to exact retained core context.

The CLI owns compilation, retention and bundle validation. This adapter grants
no authority, assurance, capability permission or sandbox enforcement.
"""
import base64
import gzip
import hashlib
import json
import re

from harness import bounded_command

CONFIG = "oh.war/execution-config/v3-draft.1"
LINK = "oh.war/workbench-runtime-link/v1-draft.1"
CONTEXT = "oh.war/workbench-runtime-context/v1-draft.1"
MAX_BUNDLE = 64 * 1024 * 1024
MAX_TRANSPORT = 1024 * 1024


class RuntimeLinkError(Exception):
    def __init__(self, state, message):
        self.state = state
        super().__init__(message)


def validate(link, stages):
    if (not isinstance(link, dict)
            or set(link) != {"schema", "warrant", "warrant_uuid", "contract_digest", "prototype", "stages"}
            or link["schema"] != LINK
            or not isinstance(link["warrant"], str)
            or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,127}", link["warrant"])
            or not isinstance(link["warrant_uuid"], str)
            or not re.fullmatch(r"[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}", link["warrant_uuid"])
            or not isinstance(link["contract_digest"], str)
            or not re.fullmatch(r"[0-9a-f]{64}", link["contract_digest"])
            or type(link["prototype"]) is not bool
            or not isinstance(link["stages"], dict)
            or set(link["stages"]) != set(stages)
            or any(not isinstance(s, str) or not s.strip() or len(s) > 1024 or "\0" in s
                   for s in link["stages"].values())
            or len(set(link["stages"].values())) != len(stages)):
        raise ValueError("Invalid exact workbench runtime link")


def prepare(sdk, directory, link, stage, attempt, read_file, decode, deadline, source_workspace=None):
    # The caller supplies only configured values and a service-minted attempt.
    # No browser-selected path, policy, identity or executable enters this call.
    directory.mkdir(mode=0o700)
    packet, manifest, bundle = (directory / name for name in ("dispatch.json", "context.json", "bundle.json"))

    def call(command, arguments):
        code, out, _ = bounded_command(
            [str(sdk.war), "--root", str(sdk.repo), command, *arguments, "--json"],
            sdk.repo, b"", deadline, {},
        )
        try:
            report = decode(out)
        except Exception as error:
            raise RuntimeLinkError("unknown", "Unexpected context preparation response") from error
        if (not isinstance(report, dict) or report.get("schema") != "oh.war/report/v1"
                or report.get("command") not in (command, "error")
                or type(report.get("exit_code")) is not int or report["exit_code"] != code
                or code not in (0, 1, 2) or not isinstance(report.get("counts"), dict)
                or any(type(report["counts"].get(k)) is not int or report["counts"][k] < 0
                       for k in ("error", "unknown"))
                or bool(code) != bool(report["counts"]["error"] or report["counts"]["unknown"])):
            raise RuntimeLinkError("unknown", "Contradictory context preparation response")
        if code:
            state = "refused" if report["counts"]["error"] else "unknown"
            rules = ", ".join(str(d.get("rule")) for d in report.get("diagnostics", []))
            raise RuntimeLinkError(state, "Context preparation " + state + ": " + rules[:512])
        return report

    arguments = [link["warrant"], link["stages"][stage],
                 "--expected-warrant", link["warrant_uuid"], "--expected-contract", link["contract_digest"],
                 "--emit", str(packet), "--emit-context", str(manifest)]
    if source_workspace is not None:
        arguments.extend(["--source-workspace", str(source_workspace)])
    if link["prototype"]:
        arguments.append("--prototype")
    call("dispatch", arguments)
    dispatch = decode(read_file(packet))
    if (not isinstance(dispatch, dict) or dispatch.get("api_version") != "oh.war/stage-dispatch/v1"
            or dispatch.get("warrant_ref") != "war://" + link["warrant_uuid"]
            or dispatch.get("contract_digest") != link["contract_digest"]
            or dispatch.get("stage_id") != link["stages"][stage]):
        raise RuntimeLinkError("unknown", "Prepared Dispatch differs from configured subject")
    bundle_arguments = ["create", link["warrant"], "--dispatch", str(packet),
                        "--context", str(manifest), "--emit", str(bundle)]
    if source_workspace is not None:
        bundle_arguments.extend(["--source-workspace", str(source_workspace)])
    created = call("dispatch-bundle", bundle_arguments)
    result = created.get("result")
    if (not isinstance(result, dict) or result.get("execution_authorized") is not False
            or not isinstance(result.get("bundle_digest"), str)
            or not re.fullmatch(r"sha256:[0-9a-f]{64}", result["bundle_digest"])):
        raise RuntimeLinkError("unknown", "Unexpected portable context result")
    checked = call("dispatch-bundle", ["check", str(bundle), "--expected-digest", result["bundle_digest"]])
    checked = checked.get("result")
    if (not isinstance(checked, dict) or checked.get("execution_authorized") is not False
            or checked.get("bundle_digest") != result["bundle_digest"]
            or checked.get("dispatch_digest") != dispatch.get("dispatch_digest")):
        raise RuntimeLinkError("unknown", "Unexpected portable context validation")
    raw = read_file(bundle, MAX_BUNDLE)
    if "sha256:" + hashlib.sha256(raw).hexdigest() != result["bundle_digest"]:
        raise RuntimeLinkError("refused", "Portable bundle bytes changed after SDK validation")
    encoded = base64.b64encode(gzip.compress(raw, mtime=0)).decode("ascii")
    if len(encoded) > MAX_TRANSPORT:
        raise RuntimeLinkError("refused", "Portable context exceeds the 1 MiB workflow transport limit")
    binding = {"schema": CONTEXT, "workbench_warrant": attempt["warrant_id"],
               "workbench_source_sha256": attempt["source_sha256"], "workbench_stage": stage,
               "warrant": link["warrant"], "warrant_ref": dispatch["warrant_ref"],
               "contract_digest": dispatch["contract_digest"], "stage_id": dispatch["stage_id"],
               "dispatch_id": dispatch["dispatch_id"], "dispatch_digest": dispatch["dispatch_digest"],
               "core_attempt_id": dispatch["attempt_id"], "bundle_digest": result["bundle_digest"],
               "qualified": False}
    return binding, {**binding, "bundle_encoding": "gzip+base64", "bundle_base64": encoded}
