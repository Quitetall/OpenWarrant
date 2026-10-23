#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Run OW131's unchanged physical cancellation/handoff plant on native Unix.
set +e
set -uo pipefail
NATIVE_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd) || exit 1
export PATH="$NATIVE_ROOT/conformance/native/bin:$PATH"
printf 'Native host: %s\n' "$(uname -s -m)"
printf 'Source: %s\n' "$(git -C "$NATIVE_ROOT" rev-parse HEAD)"
printf 'Source tree: %s\n' "$(git -C "$NATIVE_ROOT" rev-parse 'HEAD^{tree}')"
printf 'Bash: %s\n' "$BASH_VERSION"
printf 'setsid: %s\n' "$(command -v setsid)"
"$NATIVE_ROOT/target/debug/war" --version || exit 1
sha256sum "$NATIVE_ROOT/target/debug/war" || exit 1

# lib.sh keeps the real corpus guard, restoration, isolated fixture programs
# and positive corpus control. Do not substitute a second cancellation test.
source "$NATIVE_ROOT/conformance/lib.sh"
run_plant_files "$REPO_ROOT/conformance/plants.d/60-perform-cancel.sh"
printf '\n%s passed, %s failed\n' "$PASSED" "$FAILED"
# The retained Linux baseline records all 22 controls, including the positive
# corpus. A partial source/early return must not look like native coverage.
[[ "$PASSED" -eq 22 && "$FAILED" -eq 0 ]]
