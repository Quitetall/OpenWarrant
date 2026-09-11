#!/usr/bin/env bash
# A drafter that answers every request with the same fixture proposal. Reads
# stdin (the request) so a broken pipe is never the failure under test.
cat > /dev/null
cat "$(dirname "${BASH_SOURCE[0]}")/../proposals/v2-minimal.json"
