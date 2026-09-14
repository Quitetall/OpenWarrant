# shellcheck shell=bash
# `--json` (SAS §76.4) — the envelope names the failing rule, the error path is
# still JSON, and a request document rides inside `result`. These are plants
# rather than only unit tests because an envelope that dropped a diagnostic
# would pass every unit test and still lie to the agent reading it.

# The same duplicate-ordinal mutation the battery already uses; the assertion
# is that the RULE and the SCHEMA both appear in the JSON on stdout.
plant_cmd "the json envelope names the failing rule" "oh.war/report/v1" "manifest.invalid" 2 \
    "sed -i '0,/^ordinal = 20/s//ordinal = 10/' docs/warrants/OW-WAR-0010/manifest.toml; \
     assert_present 'ordinal = 10' docs/warrants/OW-WAR-0010/manifest.toml" \
    --json check OW-WAR-0010

plant_cmd "the json error path is still json" "cli.error" "OW-WAR-9999" 1 \
    "true" \
    --json show OW-WAR-9999

plant_cmd "an authorization request rides inside result" "oh.war/authorization-request/v1" "eligible_authorizers" 0 \
    "true" \
    --json authorize OW-WAR-0010
