"""§91.4 test 24 — POSITIVE. A local choice does not require a new ADR.

Written as a fixture that must be ACCEPTED rather than rejected: "does not
require" is only demonstrable by the rule passing on an amendment that names no
governing ADR. A test that merely never fires would prove the rule absent.
"""
import pathlib
out = pathlib.Path("docs/warrants/OW-WAR-0046/amendments/AM-999.yaml")
out.write_text(
    'schema: "oh.war/amendment/v1"\n'
    'id: "AM-999"\n'
    'band: "local_choice"\n'
    'reason: "A local choice inside declared autonomy (§91.4 test 24)."\n'
    'governing_adr_or_policy: ""\n'
    'artifact_admissibility: "remain_admissible"\n'
    'restart_or_repair_instruction: "Continue."\n'
    're_preflight_required: "false"\n'
    'authorizer: "conformance"\n'
    'effective_time: "2026-08-21"\n'
    "semantic_diff:\n"
    '  - element: "resources"\n'
    '    before: "a"\n'
    '    after: "b"\n'
    "affected_stages: []\n"
    "affected_milestones: []\n"
)
