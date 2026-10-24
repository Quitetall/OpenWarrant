#!/usr/bin/env python3
"""Deterministic skill transport fixtures. These do not evaluate model judgment."""
import hashlib
import json
import os
from pathlib import Path
import sys


def fail(message):
    raise SystemExit(f"skill-fixture: {message}")


def packet_inputs(case):
    packet = json.loads(Path(os.environ["EVAL_CONTEXT_FILE"]).read_text())
    if packet.get("schema") != "oh.war/eval-context/v1":
        fail("unsupported context packet")
    documents = {d["path"]: d for d in packet["documents"]}
    selected = f'.claude/skills/war-{case["skill"]}/SKILL.md'
    for path in [selected, ".claude/skills/openwarrant/SKILL.md", "CONTEXT.md"]:
        if path not in documents:
            fail(f"required source missing: {path}")
    for document in documents.values():
        digest = "sha256:" + hashlib.sha256(document["body"].encode()).hexdigest()
        if document["sha256"] != digest:
            fail(f'source digest mismatch: {document["path"]}')
    if case["method_marker"] not in documents[selected]["body"]:
        fail("selected skill no longer states the fixture's method; review this case")
    if Path("CONTEXT.md").read_text() != documents["CONTEXT.md"]["body"]:
        fail("fixture glossary differs from the supplied source; refresh the fixture explicitly")
    return packet


def artifact(case, request):
    lines = [f'# {case["title"]}', "", case["outcome"], ""]
    lines += case["requirements"]
    if case["skill"] == "grill":
        answer = request.get("answers", {}).get("Q-001")
        if answer is None:
            fail("cannot deliver an unanswered interview")
        lines += ["", "## Interview", "", "Q-001: Which output format?", f"Answer: {answer}"]
    return "\n".join(lines) + "\n"


def draft(case, request, packet):
    operations = []
    def atom(role, ordinal, path, body):
        operations.append(dict(op="create_atom", role=role, ordinal=ordinal, path=path, body=body))
    atom("intent", 10, "10-intent.md", f'# Intent\n\n## Problem\n\n{request["user_request"]}\n\n## Desired Outcome\n\n{case["outcome"]}\n\n## Scope\n\n{case["scope"]}\n\n## Non-goals\n\nNo signature, release or human acceptance.\n\n## SAS and Roadmap Traceability\n\n`sas://{request["namespace"]}-SAS-RQ-001` through `roadmap://{request["namespace"]}-PHASE-1/adopt`.\n')
    sources = "\n".join(f'- {d["path"]}: {d["sha256"]}' for d in packet["documents"])
    atom("basis", 20, "20-basis.md", f'# Basis\n\n## Supplied context\n\nExact sources in `.openwarrant/eval-context.json`:\n\n{sources}\n\n## Remaining uncertainty\n\nFixture observations do not establish model quality or human acceptance.\n')
    requirements = "\n".join(case["requirements"])
    targets = "\n".join(f'- `{p}`' for p in case["targets"])
    atom("work_order", 40, "40-work-order.md", f'# Work Order\n\n## Deliverables\n\n{targets}\n\n## Frozen Surfaces\n\nAll other application files.\n\n## Premade Instructions\n\n{requirements}\n\n## Autonomy and Escalation\n\nKeep unresolved material questions visible. The fixture only performs the first stage.\n\n## Rollback\n\nRemove only the fixture outputs.\n')
    stages = []
    milestones = []
    for index, title in enumerate(case["stages"], 1):
        stages.append(dict(id=f"STAGE-{index:03d}", title=title, executor_kind="agent", executor_ref="agent://skill-fixture", responsibility_tier="T1", context_sections=["40-work-order.md#Premade Instructions", "40-work-order.md#Deliverables"], budget_tokens=8000))
        milestone = dict(id=f"M{index}", title=title, stage_refs=[f"STAGE-{index:03d}"], obligation_refs=["OBL-001"])
        if index > 1:
            milestone["depends_on"] = [f"M{index-1}"]
        milestones.append(milestone)
    graph_body = 'schema: "oh.war/milestones/v1"\n\nmilestones:\n'
    for milestone in milestones:
        for index, (key, value) in enumerate(milestone.items()):
            graph_body += f'  {"- " if index == 0 else "  "}{key}: {json.dumps(value)}\n'
    graph_body += '\nstages:\n'
    for stage in stages:
        for index, (key, value) in enumerate(stage.items()):
            graph_body += f'  {"- " if index == 0 else "  "}{key}: {json.dumps(value)}\n'
    atom("milestones", 45, "45-milestones.yaml", graph_body)
    atom("assurance", 60, "60-assurance.md", '# Assurance\n\n## Acceptance Obligations\n\n### OBL-001 - the declared files exist at their pinned bytes\n- **scope:** only the files named by this fixture Warrant.\n- **gate:** `gate://software.repo.war-check@1.0.0`\n- **evidence:** retained files, content digests and the gate receipt; a missing required file must prevent satisfaction.\n\n## Gate Adequacy\n\nRequired at `basic`.\n\n**Adversarial question:** can an absent required file satisfy the result? The digest and presence checks must refuse it.\n\n- **outcome:** gap_accepted\n\n## Residual Risk\n\nThe verifier is a scripted fixture, not an independent assessment of this project.\n')
    operations.append(dict(op="add_relation", relation_kind="roadmap", relation_ref=f'roadmap://{request["namespace"]}-PHASE-1/adopt'))
    proposal = dict(api_version="oh.war/draft-proposal/v2", proposed_identity=dict(title=case["title"], profile="delivery", assurance="basic"), operations=operations, risk_assessment="deterministic fixture; authority and model quality unestablished")
    if case["skill"] == "grill":
        proposal["unresolved_questions"] = [dict(id="Q-001", question="Which output format?", removes_blocker=True, expected_information_gain=100)]
    return proposal


def perform(case, document):
    alias = document["warrant"]
    request = json.loads(Path(f"docs/warrants/{alias}/plan/request.json").read_text())
    target = Path(case["targets"][0])
    target.write_text(artifact(case, request))
    entries = ['schema = "oh.war/deliverables/v1"', ""]
    for index, name in enumerate(case["targets"], 1):
        content = Path(name).read_bytes() if Path(name).is_file() else b""
        digest = "sha256:" + hashlib.sha256(content).hexdigest()
        entries += ["[[deliverable]]", f'id = "D-{index:03d}"', f'title = {json.dumps(name)}', 'kind = "file"', f'target_ref = {json.dumps(name)}', 'required = true', 'content_addressed = true', 'provenance_required = true', 'obligation_refs = ["OBL-001"]', "", "[deliverable.provenance]", 'producer = "eval-performer"', f'producing_attempt = "{alias}/attempt-1"', 'contract_digest = "unrecorded"', 'input_digests = []', 'tool_or_runtime_identity = "evals/skills/fixture-agent.py perform"', 'creation_method = "authored"', f'content_digest = "{digest}"', 'media_type = "text/markdown"', 'classification = "internal"', 'retention = "repository-lifetime"', 'source_holder = "git"', ""]
    Path(f"docs/warrants/{alias}/deliverables.toml").write_text("\n".join(entries))
    print(json.dumps(dict(schema="oh.war/eval-performed/v1", warrant=alias, fixture_only=True, performed_stage=document["stage"])))


if __name__ == "__main__":
    case = json.loads((Path(os.environ["EVAL_TASK_DIR"])/"case.json").read_text())
    packet = packet_inputs(case)
    document = json.load(sys.stdin)
    if sys.argv[1:] == ["perform"]:
        perform(case, document)
    elif not sys.argv[1:]:
        print(json.dumps(draft(case, document, packet), ensure_ascii=False))
    else:
        fail("unknown mode")
