#!/usr/bin/env bash
# A performer that writes part of an answer and then dies. The partial answer is
# scratch and must not be left in the Warrant's directory.
set -uo pipefail
cat > /dev/null
printf '{ "schema": "oh.war/stage-submission/v1", "dispatch_id": "'
echo "died mid-sentence" >&2
exit 3
