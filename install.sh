#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Install the `war` binary from a published GitHub release.
#
#   curl -fsSL https://raw.githubusercontent.com/Quitetall/OpenWarrant/main/install.sh | bash
#
# Or, to pin a version and location:
#
#   OW_VERSION=v0.1.0 OW_BIN_DIR=~/.local/bin bash install.sh
#
# This script verifies the SHA-256 of what it downloads against the checksum the
# release publishes. That check is not decoration: a curl-to-shell installer that
# does not verify its download is a remote-code-execution vector with good
# manners. If the checksum is missing or does not match, this refuses to install.

set -euo pipefail

REPO="${OW_REPO:-Quitetall/OpenWarrant}"
VERSION="${OW_VERSION:-latest}"
BIN_DIR="${OW_BIN_DIR:-${HOME}/.local/bin}"

die() { printf 'install: %s\n' "$*" >&2; exit 1; }
note() { printf 'install: %s\n' "$*" >&2; }

for tool in curl tar; do
    command -v "$tool" >/dev/null || die "$tool is required"
done

# One of these must exist to verify the download; without it we do not proceed.
if command -v sha256sum >/dev/null; then
    SHA_CMD="sha256sum"
elif command -v shasum >/dev/null; then
    SHA_CMD="shasum -a 256"
else
    die "need sha256sum or shasum to verify the download; refusing to install unverified"
fi

# Target detection. Only the targets the release workflow actually builds are
# claimed here — an unlisted platform gets an honest failure rather than a
# 404 halfway through.
os="$(uname -s)"
arch="$(uname -m)"
case "${os}/${arch}" in
    Linux/x86_64)   TARGET="x86_64-unknown-linux-gnu" ;;
    Darwin/arm64)   TARGET="aarch64-apple-darwin" ;;
    *)
        die "no published binary for ${os}/${arch}. Build from source instead:
    git clone https://github.com/${REPO} && cd OpenWarrant && cargo build --release"
        ;;
esac

if [ "$VERSION" = "latest" ]; then
    note "resolving the latest release..."
    # `|| true` is load-bearing: under `set -e`, a failing curl inside a command
    # substitution aborts the script before the diagnostic below can run, and the
    # user sees a bare `curl: (22)` instead of being told no release exists.
    resp="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" 2>/dev/null || true)"
    VERSION="$(printf '%s' "$resp" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -1)"
    [ -n "$VERSION" ] || die "no published release found for ${REPO}.

Either none has been cut yet, or the repository is unreachable. Check
https://github.com/${REPO}/releases — and if it is empty, build from source:

    git clone https://github.com/${REPO} && cd OpenWarrant && cargo build --release"
fi

NAME="war-${VERSION}-${TARGET}"
URL="https://github.com/${REPO}/releases/download/${VERSION}/${NAME}.tar.gz"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

note "downloading ${VERSION} for ${TARGET}..."
curl -fsSL "$URL" -o "${TMP}/${NAME}.tar.gz" \
    || die "download failed: $URL"
curl -fsSL "${URL}.sha256" -o "${TMP}/${NAME}.tar.gz.sha256" \
    || die "no checksum published alongside ${NAME}.tar.gz; refusing to install unverified"

note "verifying checksum..."
(
    cd "$TMP"
    # The published checksum file names the archive; compare in its own directory
    # so the name in the file matches what is on disk.
    $SHA_CMD -c "${NAME}.tar.gz.sha256" >/dev/null 2>&1
) || die "CHECKSUM MISMATCH for ${NAME}.tar.gz — refusing to install.
This means the download was corrupted or tampered with. Do not retry blindly;
report it at https://github.com/${REPO}/security/advisories/new"

tar -xzf "${TMP}/${NAME}.tar.gz" -C "$TMP"
[ -f "${TMP}/${NAME}/war" ] || die "archive did not contain the expected binary"

mkdir -p "$BIN_DIR"
install -m 0755 "${TMP}/${NAME}/war" "${BIN_DIR}/war"

note "installed war ${VERSION} to ${BIN_DIR}/war"

case ":${PATH}:" in
    *":${BIN_DIR}:"*) ;;
    *) note "NOTE: ${BIN_DIR} is not on your PATH. Add it, or run ${BIN_DIR}/war directly." ;;
esac

"${BIN_DIR}/war" --version || true

cat >&2 <<'EOF'

Next:
    war init --namespace <NS>     initialize a repository
    war new "What this does"      create a Warrant
    war check                     validate it

If an AI agent will work in this repository, copy AGENTS.md from
https://github.com/Quitetall/OpenWarrant — the rules on self-verification are
load-bearing and easy to violate by accident.
EOF
