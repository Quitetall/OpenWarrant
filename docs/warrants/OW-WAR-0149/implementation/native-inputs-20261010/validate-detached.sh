#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
set -eu
cd /mnt/4tb/tmp/openwarrant-native-input-retention
log=docs/warrants/OW-WAR-0149/implementation/native-inputs-20261010
cargo_args=(--config 'build.build-dir="/mnt/4tb/build/cargo/1b/51b162b9907432"' --config 'build.rustc-wrapper=""')
set +e
timeout 1200 cargo "${cargo_args[@]}" test -j 1 -p openwarrant-cli --all-features > "$log/cli-full-retry.log" 2>&1
result=$?
printf '%s\n' "$result" > "$log/cli-full-retry.exit"
set -e
if [ "$result" -ne 0 ]; then exit "$result"; fi
set +e
timeout 600 cargo "${cargo_args[@]}" clippy -j 1 -p openwarrant-cli --all-features --all-targets -- -D warnings > "$log/clippy.log" 2>&1
result=$?
printf '%s\n' "$result" > "$log/clippy.exit"
exit "$result"
