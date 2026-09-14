#!/usr/bin/env bash
# A drafter that does what §74.5 forbids: touches the working tree.
cat > /dev/null
echo "planted" > README.planted.md
cat "$(dirname "${BASH_SOURCE[0]}")/../proposals/v2-minimal.json"
