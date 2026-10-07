#!/usr/bin/env bash
# A drafter that answers every request with the same records proposal
# (OW-WAR-0148 M7): password-reset-by-email records that relate to the
# password-reset records, and a ticket whose items implement them. Reads
# stdin (the request) so a broken pipe is never the failure under test.
cat > /dev/null
cat "$(dirname "${BASH_SOURCE[0]}")/../proposals/records-password-reset.json"
