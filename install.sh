#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# Install `war` from a published GitHub release, into the layout `war update`
# manages (OW-WAR-0143, docs/INSTALL.md):
#
#   ~/.local/lib/openwarrant/<version>/     the release, unpacked
#   ~/.local/bin/war                        a symlink into it
#
#   curl -fsSL https://raw.githubusercontent.com/Quitetall/OpenWarrant/main/install.sh | bash
#
# Or pinned, and read before it runs:
#
#   curl -fsSLO https://github.com/Quitetall/OpenWarrant/releases/download/<tag>/install.sh
#   less install.sh && OW_VERSION=<tag> bash install.sh
#
# Environment:
#   OW_VERSION       a tag (v1.0.0-alpha.3) or `latest` (default)
#   OW_REPO          owner/name on GitHub (default Quitetall/OpenWarrant)
#   OW_BIN_DIR       where the `war` link goes (default ~/.local/bin)
#   OW_RELEASES_URL  the releases list to read (default the GitHub API's)
#
# It verifies the archive's SHA-256 against the `.sha256` the release
# publishes beside it, and refuses without one. That checksum comes from the
# same release: it catches a corrupt or truncated download; it is not a
# signature (docs/INSTALL.md says what it does not establish).
#
# It never overwrites a `war` it did not install: an existing file at
# $OW_BIN_DIR/war that is not a symlink into the install root is named, with
# the commands that would replace it, and left alone.

set -euo pipefail

REPO="${OW_REPO:-Quitetall/OpenWarrant}"
VERSION="${OW_VERSION:-latest}"
BIN_DIR="${OW_BIN_DIR:-${HOME}/.local/bin}"
RELEASES_URL="${OW_RELEASES_URL:-https://api.github.com/repos/${REPO}/releases?per_page=30}"
# The same root `war update` uses: $XDG_DATA_HOME/openwarrant, or
# ~/.local/lib/openwarrant.
if [ -n "${XDG_DATA_HOME:-}" ]; then
    ROOT="${XDG_DATA_HOME}/openwarrant"
else
    ROOT="${HOME}/.local/lib/openwarrant"
fi

die() { printf 'install: %s\n' "$*" >&2; exit 1; }
note() { printf 'install: %s\n' "$*" >&2; }

for tool in curl tar; do
    command -v "$tool" >/dev/null || die "$tool is required"
done
# One of these must exist to verify the download; without it we do not proceed.
if command -v sha256sum >/dev/null; then
    sha256() { sha256sum "$1" | cut -d' ' -f1; }
elif command -v shasum >/dev/null; then
    sha256() { shasum -a 256 "$1" | cut -d' ' -f1; }
else
    die "need sha256sum or shasum to verify the download; refusing to install unverified"
fi

# Only the targets the release workflow builds; the names are package.py's.
os="$(uname -s)"
arch="$(uname -m)"
case "${os}/${arch}" in
    Linux/x86_64) PLATFORM="linux-x86_64" ;;
    Darwin/arm64) PLATFORM="darwin-arm64" ;;
    *)
        die "no published binary for ${os}/${arch}. Build from source instead:
    git clone https://github.com/${REPO} && cd OpenWarrant && cargo install --locked --path crates/openwarrant-cli"
        ;;
esac

# The releases list, one field per line. /releases and not /releases/latest:
# the latter excludes prereleases, and until a stable release exists every
# release is one. `|| true`: under `set -e` a failing curl in a substitution
# would abort before the diagnostic below.
resp="$(curl -fsSL "$RELEASES_URL" 2>/dev/null || true)"
[ -n "$resp" ] || die "could not read the releases list at ${RELEASES_URL}"
fields="$(printf '%s' "$resp" | tr ',{}[]' '\n\n\n\n\n' | sed -n \
    -e 's/^ *"tag_name": *"\([^"]*\)".*/tag \1/p' \
    -e 's/^ *"draft": *\([a-z]*\).*/draft \1/p' \
    -e 's/^ *"prerelease": *\([a-z]*\).*/prerelease \1/p' \
    -e 's/^ *"browser_download_url": *"\([^"]*\)".*/url \1/p')"

# "tag prerelease" per published release, in list order (newest first).
releases="$(printf '%s\n' "$fields" | awk '
    function emit() { if (draft != "true") print tag, pre; tag = "" }
    $1 == "tag"        { if (tag != "") emit(); tag = $2; draft = "false"; pre = "false"; next }
    $1 == "draft"      { draft = $2; next }
    $1 == "prerelease" { pre = $2; next }
    END { if (tag != "") emit() }')"

if [ "$VERSION" = "latest" ]; then
    # The newest stable release once one exists; until then, the newest release.
    VERSION="$(printf '%s\n' "$releases" | awk '$2 == "false" { print $1; exit }')"
    [ -n "$VERSION" ] || VERSION="$(printf '%s\n' "$releases" | awk 'NF { print $1; exit }')"
    [ -n "$VERSION" ] || die "no published release found at ${RELEASES_URL}.
Check https://github.com/${REPO}/releases, or build from source:
    git clone https://github.com/${REPO} && cd OpenWarrant && cargo install --locked --path crates/openwarrant-cli"
else
    case "$VERSION" in v*) ;; *) VERSION="v${VERSION}" ;; esac
    printf '%s\n' "$releases" | awk -v t="$VERSION" '$1 == t { found = 1 } END { exit !found }' \
        || die "no published release is tagged ${VERSION}"
fi
PLAIN="${VERSION#v}"
ASSET="openwarrant-${VERSION}-${PLATFORM}.tar.gz"
url_of() { printf '%s\n' "$fields" | awk -v want="/${VERSION}/$1" '
    $1 == "url" && length($2) >= length(want) && substr($2, length($2) - length(want) + 1) == want { print $2; exit }'; }
ARCHIVE_URL="$(url_of "$ASSET")"
SUM_URL="$(url_of "${ASSET}.sha256")"
[ -n "$ARCHIVE_URL" ] || die "${VERSION} carries no ${ASSET}"
[ -n "$SUM_URL" ] || die "${VERSION} publishes no ${ASSET}.sha256; refusing to install unverified"

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

note "downloading ${ASSET}..."
curl -fsSL "$ARCHIVE_URL" -o "${TMP}/${ASSET}" || die "download failed: $ARCHIVE_URL"
curl -fsSL "$SUM_URL" -o "${TMP}/${ASSET}.sha256" \
    || die "could not download ${ASSET}.sha256; refusing to install unverified"
expected="$(cut -d' ' -f1 < "${TMP}/${ASSET}.sha256")"
actual="$(sha256 "${TMP}/${ASSET}")"
if [ -z "$expected" ] || [ "$expected" != "$actual" ]; then
    die "CHECKSUM MISMATCH for ${ASSET}: the release records sha256:${expected:-nothing}, the download is sha256:${actual}. Nothing is installed.
Do not retry blindly; report it at https://github.com/${REPO}/security/advisories/new"
fi
note "verified sha256:${actual}"

# Unpacked and checked outside the install root; only a verified tree moves in.
mkdir "${TMP}/stage"
tar -xzf "${TMP}/${ASSET}" -C "${TMP}/stage" || die "could not extract ${ASSET}"
[ -f "${TMP}/stage/bin/war" ] || die "${ASSET} carries no bin/war; nothing is installed"
said="$("${TMP}/stage/bin/war" --version 2>/dev/null || true)"
case "$said" in
    "war ${PLAIN}" | "war ${PLAIN} "*) ;;
    *) die "${ASSET}'s bin/war says '${said}', not 'war ${PLAIN}'; nothing is installed" ;;
esac

mkdir -p "$ROOT"
TARGET="${ROOT}/${PLAIN}"
INCOMING="${ROOT}/${PLAIN}.incoming.$$"
rm -rf "$INCOMING"
mv "${TMP}/stage" "$INCOMING"
if [ -e "$TARGET" ]; then
    ASIDE="${ROOT}/${PLAIN}.previous.$$"
    mv "$TARGET" "$ASIDE"
    mv "$INCOMING" "$TARGET" || { mv "$ASIDE" "$TARGET"; die "could not install into ${TARGET}"; }
    rm -rf "$ASIDE"
else
    mv "$INCOMING" "$TARGET"
fi
INSTALLED="${TARGET}/bin/war"
note "installed ${VERSION} at ${INSTALLED}"

# The link: created when absent, repointed when it is already ours, and
# otherwise left exactly as it is.
LINK="${BIN_DIR}/war"
root_real="$(cd "$ROOT" && pwd -P)"
ours() {
    [ -L "$1" ] || return 1
    local to
    to="$(readlink -f "$1" 2>/dev/null || true)"
    case "$to" in "${root_real}/"* | "${ROOT}/"*) return 0 ;; esac
    return 1
}
mkdir -p "$BIN_DIR"
if [ ! -e "$LINK" ] && [ ! -L "$LINK" ]; then
    ln -s "$INSTALLED" "$LINK"
elif ours "$LINK"; then
    ln -sfn "$INSTALLED" "${LINK}.incoming.$$" && mv -f "${LINK}.incoming.$$" "$LINK"
else
    what="a file"
    [ -L "$LINK" ] && what="a symlink to $(readlink "$LINK")"
    die "${LINK} is ${what} this installer did not create, so it is left as it is.
${VERSION} is installed at ${INSTALLED}. To use it, either replace it:
    ln -sfn ${INSTALLED} ${LINK}
or move it aside and link the new one:
    mv ${LINK} ${LINK}.old && ln -s ${INSTALLED} ${LINK}"
fi
note "linked ${LINK} -> ${INSTALLED}"

# Another `war` earlier on PATH is what a shell will run instead: a stranded
# install learns here that it is shadowed.
on_path=false
installed_real="$(readlink -f "$INSTALLED" 2>/dev/null || echo "$INSTALLED")"
IFS=: read -r -a dirs <<< "${PATH}"
for dir in "${dirs[@]}"; do
    [ -n "$dir" ] || continue
    if [ "$dir" = "$BIN_DIR" ]; then on_path=true; break; fi
    other="${dir}/war"
    if [ -x "$other" ] && [ "$(readlink -f "$other" 2>/dev/null || echo "$other")" != "$installed_real" ]; then
        note "WARNING: ${other} comes before ${BIN_DIR} on PATH, and says: $("$other" --version 2>/dev/null || echo 'nothing')"
        note "  a shell runs that one, not ${VERSION}. Remove it, or put ${BIN_DIR} first on PATH."
    fi
done
$on_path || note "NOTE: ${BIN_DIR} is not on PATH. Add it, or run ${LINK} directly."

"$LINK" --version || true

cat >&2 <<'EOF'

From now on `war update` updates this install; docs/INSTALL.md has the rest.

Next:
    war init --namespace <NS>     initialize a repository
    war new "What this does"      create a Warrant
    war check                     validate it
EOF
