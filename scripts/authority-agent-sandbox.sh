#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
# Linux reference runner. Install this runner and bwrap outside agent write access.
set -euo pipefail
if [[ $# -lt 3 || $2 != -- ]]; then
    printf 'Usage: authority-agent-sandbox.sh WORKSPACE -- COMMAND [ARGS...]\n' >&2
    exit 2
fi
workspace=$(realpath -- "$1")
shift 2
if [[ ! -d $workspace || $workspace == / || $workspace == /home || $workspace == /usr || $workspace == /etc ]]; then
    printf 'Refused: dedicated task workspace required.\n' >&2
    exit 2
fi
# Only task files and runtime libraries enter the mount namespace. No home,
# authority store, signing socket, credentials, host /proc or network is mounted.
args=(--unshare-all --die-with-parent --new-session --clearenv
      --setenv PATH /usr/bin:/bin --setenv HOME /tmp
      --ro-bind /usr /usr --proc /proc --dev /dev --tmpfs /tmp
      --bind "$workspace" /work --chdir /work)
for runtime in /bin /lib /lib64; do
    if [[ -e $runtime ]]; then args+=(--ro-bind "$runtime" "$runtime"); fi
done
exec /usr/bin/bwrap "${args[@]}" -- "$@"
