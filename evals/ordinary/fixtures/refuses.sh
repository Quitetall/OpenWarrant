#!/usr/bin/env bash
# Fixture agent for `war eval ordinary`: the behaviour M9 exists to remove.
# It refuses, asks for a Warrant, and edits nothing.
set -euo pipefail
cat > /dev/null
echo "I can't change code in this repository without a Warrant."
echo "Please create a warrant first with \`war new\` and have it signed."
