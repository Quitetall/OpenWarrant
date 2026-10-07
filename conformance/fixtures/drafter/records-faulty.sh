#!/usr/bin/env bash
# A drafter whose records proposal is refused three ways at once
# (OW-WAR-0148 M7): a type the profile does not declare (RISK-prf1 · risk),
# a relation to a record that exists nowhere (REQ-prf1 implements OUT-nope),
# and an id the corpus already has (REQ-pr1). Reads stdin first.
cat > /dev/null
cat "$(dirname "${BASH_SOURCE[0]}")/../proposals/records-faulty.json"
