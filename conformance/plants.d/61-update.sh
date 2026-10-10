# shellcheck shell=bash
# OW-WAR-0143 — war knows when it is old: build identity, a notice when a
# newer release exists, and an update that works from every version.
#
# Everything here runs against a local fixture server (python3's http.server,
# with a request log) through OPENWARRANT_RELEASES_URL / OW_RELEASES_URL, with
# HOME, XDG_* and PATH inside one temporary directory. It never reads the real
# GitHub and never writes outside that directory. The fixture releases are made
# from this battery's own `war`: a copy of it, or a two-line wrapper that
# answers `--version` with another version and execs it for everything else.

echo "== war update, the notice, install.sh and build identity (OW-WAR-0143) =="
PLANT_ROOT=$(scratch_corpus UP)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
up_ok() { printf 'ok    %-44s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
up_fail() { printf 'FAIL  %-44s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
up_setup_failed() { printf 'PLANT SETUP FAILED: %s\n' "$1" >&2; [[ -n "${UP_PID:-}" ]] && kill "$UP_PID" 2>/dev/null; exit 9; }

UP_T=$(mktemp -d) || up_setup_failed "mktemp"
UP_WAR=$(readlink -f "$WAR")
UP_SH="$REPO_ROOT/install.sh"
UP_V=$("$UP_WAR" version --json 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["version"])')
[[ -n "$UP_V" ]] || up_setup_failed "war version --json names no version"
UP_INSTALL_LINE=$(sed -n 's/^ *"\(curl -fsSL [^"]*install\.sh | bash\)";$/\1/p' crates/openwarrant-cli/src/install.rs | head -1)
UP_CARGO_LINE=$(sed -n 's/^pub const CARGO_LINE: &str = "\(.*\)";$/\1/p' crates/openwarrant-cli/src/install.rs)
UP_RELEASES=$(sed -n 's/^pub const RELEASES: &str = "\(.*\)";$/\1/p' crates/openwarrant-cli/src/install.rs)
[[ -n "$UP_INSTALL_LINE" && -n "$UP_CARGO_LINE" && -n "$UP_RELEASES" ]] || up_setup_failed "could not read INSTALL_SH_LINE, CARGO_LINE or RELEASES from install.rs"
case "$(uname -s)/$(uname -m)" in
    Linux/x86_64) UP_PLATFORM=linux-x86_64 ;;
    Darwin/arm64) UP_PLATFORM=darwin-arm64 ;;
    *) up_setup_failed "no release is built for $(uname -s)/$(uname -m); these plants need one" ;;
esac
for up_tool in python3 script tar curl git sha256sum; do
    command -v "$up_tool" >/dev/null || up_setup_failed "$up_tool is required"
done
for up_d in /usr/bin /bin; do
    [[ -e "$up_d/war" ]] && up_setup_failed "$up_d/war exists; the plants' PATH must hold only the war they put there"
done
UP_NEWER="99.0.0-alpha.1"   # newer than any running build; a prerelease
UP_OLDER="0.0.1-alpha.1"    # older than any running build; a prerelease
UP_SYS="/usr/bin:/bin"

# --- the fixture server -----------------------------------------------------
mkdir -p "$UP_T/srv/dl"
cat > "$UP_T/server.py" <<'PY'
import functools, http.server, os, sys
root, log, portfile = sys.argv[1:4]
class Handler(http.server.SimpleHTTPRequestHandler):
    def log_message(self, *args):
        pass
    def do_GET(self):
        names = " ".join(sorted(k.lower() for k in self.headers.keys()))
        with open(log, "a") as f:
            f.write(f"GET {self.path} headers={names} ua={self.headers.get('user-agent', '')}\n")
        super().do_GET()
server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), functools.partial(Handler, directory=root))
open(portfile + ".tmp", "w").write(str(server.server_address[1]))
os.rename(portfile + ".tmp", portfile)
server.serve_forever()
PY
cat > "$UP_T/fixture.py" <<'PY'
import gzip, hashlib, io, json, os, shutil, sys, tarfile, time
def sha(b): return hashlib.sha256(b).hexdigest()
cmd, srv = sys.argv[1], sys.argv[2]
if cmd == "archive":
    # archive <tag> <bin/war source> <mode: ok|tamper|nosum|manifest-bad> <platform>
    tag, src, mode, platform = sys.argv[3:7]
    files = {"bin/war": open(src, "rb").read(), "LICENSE": b"fixture licence\n"}
    manifest = {"schema": "oh.war/candidate-bundle/v1", "label": tag,
                "files": {n: {"sha256": sha(d), "bytes": len(d), "executable": n == "bin/war"} for n, d in files.items()}}
    if mode == "manifest-bad":
        manifest["files"]["LICENSE"]["sha256"] = "0" * 64
    files["MANIFEST.json"] = (json.dumps(manifest, indent=2, sort_keys=True) + "\n").encode()
    buf = io.BytesIO()
    with gzip.GzipFile(fileobj=buf, mode="wb", mtime=0) as gz:
        with tarfile.open(fileobj=gz, mode="w") as t:
            for n, d in sorted(files.items()):
                info = tarfile.TarInfo(n); info.size = len(d); info.mtime = 0
                info.mode = 0o755 if n == "bin/war" else 0o644
                t.addfile(info, io.BytesIO(d))
    data = buf.getvalue(); summed = data
    if mode == "tamper":
        data = bytearray(data); data[len(data) // 2] ^= 0x01; data = bytes(data)
    name = f"openwarrant-{tag}-{platform}.tar.gz"
    d = os.path.join(srv, "dl", tag); shutil.rmtree(d, ignore_errors=True); os.makedirs(d)
    open(os.path.join(d, name), "wb").write(data)
    if mode != "nosum":
        open(os.path.join(d, name + ".sha256"), "w").write(f"{sha(summed)}  {name}\n")
elif cmd == "list":
    # list <base url> <tag>:<prerelease> ...   (newest first, as GitHub orders)
    base, out = sys.argv[3], []
    for spec in sys.argv[4:]:
        tag, pre = spec.split(":")
        d = os.path.join(srv, "dl", tag)
        names = sorted(os.listdir(d)) if os.path.isdir(d) else []
        out.append({"tag_name": tag, "draft": False, "prerelease": pre == "true",
                    "assets": [{"name": n, "browser_download_url": f"{base}/dl/{tag}/{n}"} for n in names]})
    open(os.path.join(srv, "releases.json"), "w").write(json.dumps(out, indent=2) + "\n")
elif cmd == "cache":
    # cache <file> <age seconds> <newest tag>
    path, age, tag = srv, int(sys.argv[3]), sys.argv[4]
    os.makedirs(os.path.dirname(path), exist_ok=True)
    open(path, "w").write(json.dumps({"checked_at": int(time.time()) - age, "channel": "preview", "newest": tag}) + "\n")
PY
: > "$UP_T/requests.log"
python3 "$UP_T/server.py" "$UP_T/srv" "$UP_T/requests.log" "$UP_T/port" >/dev/null 2>&1 &
UP_PID=$!
for _ in $(seq 1 100); do [[ -s "$UP_T/port" ]] && break; sleep 0.05; done
[[ -s "$UP_T/port" ]] || up_setup_failed "the fixture server did not start"
UP_URL="http://127.0.0.1:$(cat "$UP_T/port")"

up_archive() { python3 "$UP_T/fixture.py" archive "$UP_T/srv" "v$1" "$2" "$3" "$UP_PLATFORM"; }
up_list() { python3 "$UP_T/fixture.py" list "$UP_T/srv" "$UP_URL" "$@"; }
up_wrapper() { # <version it claims> -> path of a `war` that says so and execs the battery's
    local f="$UP_T/wrap-$1"
    printf '#!/bin/sh\nif [ "$1" = "--version" ]; then echo "war %s"; exit 0; fi\nexec %q "$@"\n' "$1" "$UP_WAR" > "$f"
    chmod +x "$f"; printf '%s' "$f"
}
up_log_clear() { : > "$UP_T/requests.log"; }
up_gets() { grep -c "^GET $1" "$UP_T/requests.log"; }

# The environment every command below runs in: nothing of the caller's HOME,
# caches, data, proxies or opt-outs leaks in.
UP_HOME="$UP_T/home"; UP_DATA="$UP_T/data"; UP_PATH="$UP_T/bin:$UP_SYS"
up_run() {
    local data=()
    if [[ -n "$UP_DATA" ]]; then data=("XDG_DATA_HOME=$UP_DATA"); else data=(-u XDG_DATA_HOME); fi
    env -u CI -u OPENWARRANT_NO_UPDATE_CHECK -u OPENWARRANT_RELEASE_TAG \
        -u http_proxy -u https_proxy -u HTTP_PROXY -u HTTPS_PROXY -u ALL_PROXY -u all_proxy \
        -u GIT_DIR -u GIT_WORK_TREE -u SSH_AUTH_SOCK -u SSH_AGENT_PID "${data[@]}" \
        HOME="$UP_HOME" XDG_CACHE_HOME="$UP_HOME/.cache" XDG_CONFIG_HOME="$UP_HOME/.config" \
        PATH="$UP_PATH" SHELL=/bin/sh NO_PROXY=127.0.0.1 no_proxy=127.0.0.1 \
        GIT_CEILING_DIRECTORIES="$UP_T" OPENWARRANT_NO_PROJECTS=1 \
        OPENWARRANT_RELEASES_URL="$UP_URL/releases.json" OW_RELEASES_URL="$UP_URL/releases.json" \
        "$@"
}
# Under a pty: stdout to <out>, stderr (the terminal) to <err>, \r stripped.
up_pty() { # <out> <err> [VAR=value ...] -- <war args...>
    local out="$1" err="$2" rc; shift 2
    local vars=()
    while [[ "$1" != "--" ]]; do vars+=("$1"); shift; done; shift
    up_run env ${vars[@]+"${vars[@]}"} script -qec "$(printf '%q ' "$UP_WAR" "$@")>$(printf '%q' "$out")" /dev/null > "$err.raw" 2>&1
    rc=$?
    tr -d '\r' < "$err.raw" > "$err"
    return "$rc"
}
up_fresh_home() { command rm -rf "$UP_HOME" "$UP_DATA"; mkdir -p "$UP_HOME"; }
up_snapshot() { # the install root's listing, link targets and bytes, and the PATH link
    ( cd "$1" 2>/dev/null && find . -printf '%p %y %l\n' | sort && find . -type f -exec sha256sum {} + | sort )
    readlink "$UP_T/bin/war" 2>/dev/null
}

# =========================================================================
# OBL-001 — a build says what it is, and only a release build says only its version
UP_R="$UP_T/repo"; mkdir -p "$UP_R"
git -C "$UP_R" init -q && printf 'one\n' > "$UP_R/f.txt" && git -C "$UP_R" add f.txt \
    && git -C "$UP_R" -c user.email=plant@invalid -c user.name=plant commit -qm one \
    || up_setup_failed "scratch git repository"
UP_HEAD=$(git -C "$UP_R" rev-parse HEAD)
up_probe() { up_run "$@" >"$UP_T/probe.json" 2>"$UP_T/probe.err"; echo $?; }
up_field() { python3 -c "import json,sys; v=json.load(open(sys.argv[1])); print($2)" "$UP_T/probe.json" 2>/dev/null; }

rc=$(up_probe "$UP_WAR" version --probe "$UP_R")
if [[ "$rc" == 0 && "$(up_field x "v['class']")" == unreleased && "$(up_field x "v['commit']")" == "$UP_HEAD" \
      && "$(up_field x "v['dirty']")" == False ]]; then
    up_ok "a clean checkout is unreleased" "commit ${UP_HEAD:0:12}, dirty false"
else
    up_fail "a clean checkout is unreleased" "rc $rc: $(head -c 300 "$UP_T/probe.json")"
fi
rc=$(up_probe env OPENWARRANT_RELEASE_TAG="v$UP_V" "$UP_WAR" version --probe "$UP_R")
if [[ "$rc" == 0 && "$(up_field x "v['class']")" == release && "$(up_field x "v['line']")" == "war $UP_V" ]]; then
    up_ok "the same tree, tagged v<version>, is a release" "line is exactly 'war $UP_V'"
else
    up_fail "the same tree, tagged v<version>, is a release" "rc $rc: $(head -c 300 "$UP_T/probe.json")"
fi
mkdir -p "$UP_T/crate"
printf '{\n  "git": {\n    "sha1": "0123456789abcdef0123456789abcdef01234567"\n  },\n  "path_in_vcs": "crates/openwarrant-cli"\n}\n' > "$UP_T/crate/.cargo_vcs_info.json"
rc=$(up_probe "$UP_WAR" version --probe "$UP_T/crate")
if [[ "$rc" == 0 && "$(up_field x "v['class']")" == crate && "$(up_field x "v['commit']")" == 0123456789abcdef0123456789abcdef01234567 ]]; then
    up_ok "a published crate's source is crate" "sha1 from .cargo_vcs_info.json"
else
    up_fail "a published crate's source is crate" "rc $rc: $(head -c 300 "$UP_T/probe.json")"
fi
mkdir -p "$UP_T/nothing"
rc=$(up_probe "$UP_WAR" version --probe "$UP_T/nothing")
if [[ "$rc" == 0 && "$(up_field x "v['class']")" == unknown && "$(up_field x "v['commit']")" == None ]]; then
    up_ok "neither source: unknown" "no commit claimed"
else
    up_fail "neither source: unknown" "rc $rc: $(head -c 300 "$UP_T/probe.json")"
fi
# Refusals.
printf 'two\n' >> "$UP_R/f.txt"
rc=$(up_probe "$UP_WAR" version --probe "$UP_R")
if [[ "$rc" == 0 && "$(up_field x "v['dirty']")" == True ]]; then
    up_ok "a modified tracked file is dirty" "dirty true"
else
    up_fail "a modified tracked file is dirty" "rc $rc: $(head -c 300 "$UP_T/probe.json")"
fi
rc=$(up_probe env OPENWARRANT_RELEASE_TAG="v$UP_V" "$UP_WAR" version --probe "$UP_R")
if [[ "$rc" != 0 && "$(up_field x "v['refused']")" == *dirty* && "$(up_field x "'class' in v")" == False ]]; then
    up_ok "a release tag over a dirty tree is refused" "exit $rc: $(up_field x "v['refused']" | head -c 90)"
else
    up_fail "a release tag over a dirty tree is refused" "rc $rc: $(head -c 300 "$UP_T/probe.json")"
fi
git -C "$UP_R" checkout -q -- f.txt
rc=$(up_probe env OPENWARRANT_RELEASE_TAG=v9.9.9 "$UP_WAR" version --probe "$UP_R")
UP_WHY=$(up_field x "v['refused']")
if [[ "$rc" != 0 && "$UP_WHY" == *v9.9.9* && "$UP_WHY" == *"v$UP_V"* ]]; then
    up_ok "a tag that is not v<version> is refused" "names v9.9.9 and v$UP_V"
else
    up_fail "a tag that is not v<version> is refused" "rc $rc: $UP_WHY"
fi
rc=$(up_probe env OPENWARRANT_RELEASE_TAG="v$UP_V" "$UP_WAR" version --probe "$UP_T/nothing")
if [[ "$rc" != 0 && "$(up_field x "v['refused']")" == *"never a release"* ]]; then
    up_ok "unknown is never release" "a tag with no source is refused"
else
    up_fail "unknown is never release" "rc $rc: $(head -c 300 "$UP_T/probe.json")"
fi
UP_LINE=$("$UP_WAR" --version)
UP_CLASS=$("$UP_WAR" version --json 2>/dev/null | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["build"]["class"])')
UP_COMMIT=$("$UP_WAR" version --json 2>/dev/null | python3 -c 'import json,sys; print((json.load(sys.stdin)["result"]["build"]["commit"] or "")[:12])')
if [[ "$UP_LINE" != "war $UP_V" && "$UP_LINE" == "war $UP_V ("*"$UP_CLASS"* ]] \
    && [[ "$UP_CLASS" == unreleased || "$UP_CLASS" == unknown ]]; then
    up_ok "the battery's war says what it is" "$UP_LINE; build.class $UP_CLASS"
else
    up_fail "the battery's war says what it is" "'$UP_LINE', build.class '$UP_CLASS'"
fi

# =========================================================================
# A managed install: the battery's war copied into the install root, and a
# link to it on PATH. OBL-002 and OBL-003 run against it.
up_managed() {
    up_fresh_home
    mkdir -p "$UP_DATA/openwarrant/$UP_V/bin" "$UP_T/bin"
    command cp "$UP_WAR" "$UP_DATA/openwarrant/$UP_V/bin/war"
    ln -sfn "$UP_DATA/openwarrant/$UP_V/bin/war" "$UP_T/bin/war"
    UP_PATH="$UP_T/bin:$UP_SYS"
}
up_managed
up_archive "$UP_NEWER" "$(up_wrapper "$UP_NEWER")" ok
up_archive "$UP_OLDER" "$(up_wrapper "$UP_OLDER")" ok
up_archive "$UP_V" "$UP_WAR" ok

# OBL-003 — war update finds the right release and never downgrades unasked
if [[ "$UP_V" == *-* ]]; then
    up_list "v$UP_NEWER:true" "v$UP_OLDER:true"
    UP_OUT=$(up_run "$UP_WAR" update --check 2>&1)
    if grep -q 'update.available' <<<"$UP_OUT" && grep -q "v$UP_NEWER" <<<"$UP_OUT" && ! grep -q 'update.unavailable' <<<"$UP_OUT"; then
        up_ok "no flags, prereleases only: found" "update.available v$UP_NEWER (default channel preview)"
    else
        up_fail "no flags, prereleases only: found" "$(grep -E 'update\.' <<<"$UP_OUT" | head -2)"
    fi
else
    up_fail "no flags, prereleases only: found" "the running build $UP_V is not a prerelease; this plant's precondition does not hold"
fi
UP_T11=ok
for up_v in 1.0.0-alpha 1.0.0-alpha.1 1.0.0-alpha.beta 1.0.0-beta 1.0.0-beta.2 1.0.0-beta.11 1.0.0-rc.1; do
    grep -q "\"$up_v\"," crates/openwarrant-cli/src/install.rs || UP_T11="missing $up_v"
done
if [[ "$UP_T11" == ok ]] && grep -q 'fn precedence_follows_the_semver_example_ordering' crates/openwarrant-cli/src/install.rs; then
    up_ok "§11's ordering is a unit test of precedes" "precedence_follows_the_semver_example_ordering (cargo test runs it)"
else
    up_fail "§11's ordering is a unit test of precedes" "$UP_T11"
fi
up_list "v$UP_OLDER:true"
UP_BEFORE=$(up_snapshot "$UP_DATA/openwarrant")
up_log_clear
UP_OUT=$(up_run "$UP_WAR" update --check 2>&1)
UP_OUT2=$(up_run "$UP_WAR" update 2>&1); rc=$?
if grep -q 'update.ahead' <<<"$UP_OUT" && grep -q 'update.ahead' <<<"$UP_OUT2" && [[ "$rc" == 0 ]] \
    && [[ "$(up_gets /dl/)" == 0 ]] && [[ "$(up_snapshot "$UP_DATA/openwarrant")" == "$UP_BEFORE" ]]; then
    up_ok "an older newest release is not offered" "update.ahead; no archive GET; root unchanged"
else
    up_fail "an older newest release is not offered" "rc $rc, $(up_gets /dl/) archive GET(s); $(grep -E 'update\.' <<<"$UP_OUT2" | head -2)"
fi
up_list "v$UP_V:true"
UP_OUT=$(up_run "$UP_WAR" update --check 2>&1)
if grep -q 'update.unreleased' <<<"$UP_OUT" && ! grep -q 'update.current' <<<"$UP_OUT" \
    && [[ -n "$UP_COMMIT" ]] && grep -q "$UP_COMMIT" <<<"$(grep 'update.unreleased' -A2 <<<"$UP_OUT")"; then
    up_ok "same version, not the release: unreleased" "update.unreleased names commit $UP_COMMIT; never update.current"
else
    up_fail "same version, not the release: unreleased" "$(grep -E 'update\.' <<<"$UP_OUT" | head -2)"
fi

# OBL-002 — nothing switches until the download is verified
up_refused_update() { # <label> <rule>
    UP_BEFORE=$(up_snapshot "$UP_DATA/openwarrant")
    UP_OUT=$(up_run "$UP_WAR" update 2>&1); rc=$?
    local incoming; incoming=$(find "$UP_DATA/openwarrant" -name '*.incoming.*' | wc -l)
    if [[ "$rc" != 0 ]] && grep -q "$2" <<<"$UP_OUT" && [[ "$(up_snapshot "$UP_DATA/openwarrant")" == "$UP_BEFORE" ]] && [[ "$incoming" == 0 ]]; then
        up_ok "$1" "exit $rc, $2; root and link unchanged"
    else
        up_fail "$1" "rc $rc, $incoming incoming dir(s); $(grep -E '^(ERROR|WARN)' <<<"$UP_OUT" | head -2)"
    fi
}
up_managed
up_archive "$UP_NEWER" "$(up_wrapper "$UP_NEWER")" tamper; up_list "v$UP_NEWER:true"
up_refused_update "a changed byte is refused" update.checksum-mismatch
up_archive "$UP_NEWER" "$(up_wrapper "$UP_NEWER")" nosum; up_list "v$UP_NEWER:true"
up_refused_update "no .sha256 is refused" update.no-checksum
up_archive "$UP_NEWER" "$(up_wrapper "$UP_NEWER")" manifest-bad; up_list "v$UP_NEWER:true"
up_refused_update "a MANIFEST.json disagreement is refused" update.manifest-mismatch
up_archive "$UP_NEWER" "$(up_wrapper 9.9.9)" ok; up_list "v$UP_NEWER:true"
up_refused_update "a bin/war of another version is refused" update.identity-mismatch
up_archive "$UP_NEWER" "$(up_wrapper "$UP_NEWER")" ok; up_list "v$UP_NEWER:true"
UP_OUT=$(up_run "$UP_WAR" update 2>&1); rc=$?
if [[ "$rc" == 0 ]] && [[ -x "$UP_DATA/openwarrant/$UP_NEWER/bin/war" ]] \
    && [[ "$(readlink "$UP_T/bin/war")" == "$UP_DATA/openwarrant/$UP_NEWER/bin/war" ]] \
    && [[ -x "$UP_DATA/openwarrant/$UP_V/bin/war" ]] && grep -q 'update.unsigned' <<<"$UP_OUT" \
    && grep -q 'update.source' <<<"$UP_OUT" && [[ -z "$(find "$UP_DATA/openwarrant" -name '*.incoming.*')" ]]; then
    up_ok "a verified release switches the link" "$UP_NEWER installed; $UP_V kept; update.unsigned, update.source"
else
    up_fail "a verified release switches the link" "rc $rc; link $(readlink "$UP_T/bin/war"); $(grep -E '^(ERROR|WARN)' <<<"$UP_OUT" | head -2)"
fi

# OBL-004 — an unmanaged war is never overwritten, and the refusal says what to do
up_unmanaged() { # <label> <file> <expected command>
    local label="$1" file="$2" want="$3" before after
    before="$(sha256sum < "$file") $(stat -c '%a %Y' "$file")"
    up_log_clear
    UP_OUT=$(up_run "$UP_WAR" update 2>&1); rc=$?
    after="$(sha256sum < "$file") $(stat -c '%a %Y' "$file")"
    if [[ "$rc" != 0 ]] && grep -q 'update.unmanaged' <<<"$UP_OUT" && grep -qF -- "$want" <<<"$UP_OUT" \
        && [[ "$before" == "$after" ]] && [[ "$(up_gets /dl/)" == 0 ]]; then
        up_ok "$label" "update.unmanaged, no download, untouched; names: ${want:0:40}"
    else
        up_fail "$label" "rc $rc, $(up_gets /dl/) archive GET(s), bytes/mode/mtime $([[ "$before" == "$after" ]] && echo same || echo CHANGED); $(grep -E '^ERROR' <<<"$UP_OUT" | head -1)"
    fi
}
up_fresh_home; command rm -f "$UP_T/bin/war"
mkdir -p "$UP_T/u1" "$UP_T/u2" "$UP_HOME/.cargo/bin"
command cp "$UP_WAR" "$UP_T/u1/war"
printf '#!/bin/sh\nexec %q "$@"\n' "$UP_WAR" > "$UP_T/u2/war"; chmod +x "$UP_T/u2/war"
touch -d '2020-01-01 00:00:00' "$UP_T/u1/war" "$UP_T/u2/war"
UP_PATH="$UP_T/u1:$UP_SYS"; up_unmanaged "a copied binary is refused" "$UP_T/u1/war" "$UP_INSTALL_LINE"
UP_PATH="$UP_T/u2:$UP_SYS"; up_unmanaged "a #! script is refused" "$UP_T/u2/war" "$UP_INSTALL_LINE"
command cp "$UP_WAR" "$UP_HOME/.cargo/bin/war"; touch -d '2020-01-01 00:00:00' "$UP_HOME/.cargo/bin/war"
UP_PATH="$UP_HOME/.cargo/bin:$UP_SYS"; up_unmanaged "a ~/.cargo/bin binary is refused" "$UP_HOME/.cargo/bin/war" "$UP_CARGO_LINE"
# Accepted: beside a managed link, the unmanaged script is reported and left alone.
up_managed
mkdir -p "$UP_T/u2"
printf '#!/bin/sh\nexec %q "$@"\n' "$UP_WAR" > "$UP_T/u2/war"; chmod +x "$UP_T/u2/war"
UP_SCRIPT_BEFORE=$(sha256sum < "$UP_T/u2/war")
UP_PATH="$UP_T/u2:$UP_T/bin:$UP_SYS"
UP_OUT=$(up_run "$UP_WAR" update 2>&1); rc=$?
if [[ "$rc" == 0 ]] && [[ "$(readlink "$UP_T/bin/war")" == "$UP_DATA/openwarrant/$UP_NEWER/bin/war" ]] \
    && grep -q "update.not-ours" <<<"$UP_OUT" && grep -q "$UP_T/u2/war" <<<"$(grep 'update.not-ours' <<<"$UP_OUT")" \
    && [[ "$(sha256sum < "$UP_T/u2/war")" == "$UP_SCRIPT_BEFORE" ]]; then
    up_ok "a managed link is repointed beside an unmanaged war" "update.repointed; the script update.not-ours, untouched"
else
    up_fail "a managed link is repointed beside an unmanaged war" "rc $rc; $(grep -E '^(ERROR|WARN)' <<<"$UP_OUT" | head -2)"
fi
# remedy("99.0.0"), read where it is printed: the notice's "Update:" line.
up_remedy() { # -> the remedy the notice prints for a cached v99.0.0
    python3 "$UP_T/fixture.py" cache "$UP_HOME/.cache/openwarrant/release-check.json" 0 v99.0.0
    up_pty "$UP_T/r.out" "$UP_T/r.err" -- --root "$PLANT_ROOT" status
    sed -n 's/^war: v99\.0\.0 is published (this is [^)]*)\. Update: //p' "$UP_T/r.err" | head -1
}
UP_PATH="$UP_T/bin:$UP_SYS"
UP_REMEDY_MANAGED=$(up_remedy)
up_fresh_home; UP_PATH="$UP_T/u1:$UP_SYS"
UP_REMEDY_UNMANAGED=$(up_remedy)
if [[ "$UP_REMEDY_MANAGED" == "war admin update --to 99.0.0" && "$UP_REMEDY_UNMANAGED" == "$UP_INSTALL_LINE" ]]; then
    up_ok "remedy(99.0.0): update when managed, install.sh when not" "'$UP_REMEDY_MANAGED' / install.sh line"
else
    up_fail "remedy(99.0.0): update when managed, install.sh when not" "managed '$UP_REMEDY_MANAGED', unmanaged '$UP_REMEDY_UNMANAGED'"
fi

# =========================================================================
# OBL-005 — one line on stderr when a newer release exists, and silence otherwise
up_managed
up_list "v$UP_NEWER:true"
UP_CACHE="$UP_HOME/.cache/openwarrant/release-check.json"
up_notices() { grep -c '^war: .* is published (this is ' "$1"; }
python3 "$UP_T/fixture.py" cache "$UP_CACHE" 0 "v$UP_NEWER"
up_log_clear
up_pty "$UP_T/on.out" "$UP_T/on.err" -- --root "$PLANT_ROOT" status; rc_on=$?
up_pty "$UP_T/off.out" "$UP_T/off.err" OPENWARRANT_NO_UPDATE_CHECK=1 -- --root "$PLANT_ROOT" status; rc_off=$?
sleep 1
if [[ "$(up_notices "$UP_T/on.err")" == 1 ]] && grep -q "^war: v$UP_NEWER is published (this is $UP_V)\. Update: war admin update --to $UP_NEWER$" "$UP_T/on.err" \
    && cmp -s "$UP_T/on.out" "$UP_T/off.out" && [[ -s "$UP_T/on.out" && "$rc_on" == "$rc_off" ]] && [[ "$(up_gets /)" == 0 ]]; then
    up_ok "a newer release: one stderr line" "names v$UP_NEWER and the remedy; stdout identical to the disabled run"
else
    up_fail "a newer release: one stderr line" "$(up_notices "$UP_T/on.err") notice line(s); stdout $(cmp -s "$UP_T/on.out" "$UP_T/off.out" && echo same || echo differs); $(up_gets /) request(s)"
fi
python3 "$UP_T/fixture.py" cache "$UP_CACHE" 200000 "v$UP_OLDER"
up_log_clear
up_pty "$UP_T/s1.out" "$UP_T/s1.err" -- --root "$PLANT_ROOT" status
up_pty "$UP_T/s2.out" "$UP_T/s2.err" -- --root "$PLANT_ROOT" status
for _ in $(seq 1 100); do grep -q '"newest"' "$UP_CACHE" 2>/dev/null && break; sleep 0.1; done
sleep 1
UP_HDRS=$(sed -n 's/^GET \/releases.json headers=\(.*\) ua=.*/\1/p' "$UP_T/requests.log" | head -1)
if [[ "$(up_gets /releases.json)" == 1 && "$(up_gets /)" == 1 ]] && grep -q "\"v$UP_NEWER\"" "$UP_CACHE" \
    && [[ "$UP_HDRS" == "host user-agent" ]] && grep -q "ua=openwarrant/$UP_V$" "$UP_T/requests.log"; then
    up_ok "a stale cache: one request for two runs" "GET /releases.json once, headers host+user-agent; cache names v$UP_NEWER"
else
    up_fail "a stale cache: one request for two runs" "$(up_gets /) request(s), headers '$UP_HDRS'; cache $(head -c 160 "$UP_CACHE" 2>/dev/null)"
fi
up_silent() { # <label> <out> <err>: no notice and no request
    sleep 1
    if [[ "$(up_notices "$3")" == 0 && "$(up_gets /)" == 0 ]]; then
        up_ok "$1" "no notice, no request"
    else
        up_fail "$1" "$(up_notices "$3") notice line(s), $(up_gets /) request(s)"
    fi
}
up_stale() { python3 "$UP_T/fixture.py" cache "$UP_CACHE" 200000 "v$UP_NEWER"; up_log_clear; }
up_stale; up_pty "$UP_T/q.out" "$UP_T/q.err" OPENWARRANT_NO_UPDATE_CHECK=1 -- --root "$PLANT_ROOT" status
up_silent "OPENWARRANT_NO_UPDATE_CHECK=1: silent" "$UP_T/q.out" "$UP_T/q.err"
up_stale; up_pty "$UP_T/q.out" "$UP_T/q.err" CI=true -- --root "$PLANT_ROOT" status
up_silent "CI=true: silent" "$UP_T/q.out" "$UP_T/q.err"
up_stale; up_pty "$UP_T/q.out" "$UP_T/q.err" -- --root "$PLANT_ROOT" --json status
if python3 -c 'import json,sys; t=open(sys.argv[1]).read(); v=json.loads(t); assert v["schema"]=="oh.war/report/v1"' "$UP_T/q.out" 2>/dev/null; then
    up_silent "--json: silent, one envelope on stdout" "$UP_T/q.out" "$UP_T/q.err"
else
    up_fail "--json: silent, one envelope on stdout" "stdout is not one oh.war/report/v1 envelope: $(head -c 120 "$UP_T/q.out")"
fi
up_stale; up_run "$UP_WAR" --root "$PLANT_ROOT" status >"$UP_T/q.out" 2>"$UP_T/q.err"
up_silent "stderr not a terminal: silent" "$UP_T/q.out" "$UP_T/q.err"
up_stale; up_pty "$UP_T/q.out" "$UP_T/q.err" -- version
up_silent "war version: silent" "$UP_T/q.out" "$UP_T/q.err"

# =========================================================================
# OBL-006 — the bootstrap installs a managed war, and refuses what update refuses
UP_DATA=""    # install.sh and war both fall back to ~/.local/lib/openwarrant
up_list "v$UP_V:true"
up_fresh_home
UP_PATH="$UP_HOME/.local/bin:$UP_SYS"
UP_OUT=$(up_run env OW_BIN_DIR="$UP_HOME/.local/bin" bash "$UP_SH" 2>&1); rc=$?
UP_LINK="$UP_HOME/.local/bin/war"
UP_INST="$UP_HOME/.local/lib/openwarrant/$UP_V/bin/war"
UP_CHECK=$(up_run "$UP_LINK" update --check 2>&1)
if [[ "$rc" == 0 && -x "$UP_INST" && "$(readlink "$UP_LINK")" == "$UP_INST" ]] \
    && grep -q "^PASS install.on-path *$UP_LINK" <<<"$UP_CHECK" && ! grep -q 'update.not-ours' <<<"$UP_CHECK"; then
    up_ok "install.sh on an empty HOME: a managed war" "~/.local/lib/openwarrant/$UP_V/bin/war, linked; install.on-path"
else
    up_fail "install.sh on an empty HOME: a managed war" "rc $rc; link $(readlink "$UP_LINK"); $(tail -3 <<<"$UP_OUT" | tr '\n' ' ')"
fi
up_archive "$UP_V" "$UP_WAR" tamper; up_list "v$UP_V:true"
up_fresh_home
UP_OUT=$(up_run env OW_BIN_DIR="$UP_HOME/.local/bin" bash "$UP_SH" 2>&1); rc=$?
if [[ "$rc" != 0 && -z "$(ls -A "$UP_HOME/.local/lib/openwarrant" 2>/dev/null)" && ! -e "$UP_LINK" && ! -L "$UP_LINK" ]] \
    && grep -q 'CHECKSUM MISMATCH' <<<"$UP_OUT"; then
    up_ok "install.sh refuses a tampered archive" "exit $rc; nothing under the root, no link"
else
    up_fail "install.sh refuses a tampered archive" "rc $rc; $(ls -A "$UP_HOME/.local/lib/openwarrant" 2>/dev/null | head -2)"
fi
up_archive "$UP_V" "$UP_WAR" ok; up_list "v$UP_V:true"
up_fresh_home; mkdir -p "$UP_HOME/.local/bin"
printf 'a plain file somebody put here\n' > "$UP_LINK"
UP_PLAIN=$(sha256sum < "$UP_LINK")
UP_OUT=$(up_run env OW_BIN_DIR="$UP_HOME/.local/bin" bash "$UP_SH" 2>&1); rc=$?
if [[ "$rc" != 0 && ! -L "$UP_LINK" && "$(sha256sum < "$UP_LINK")" == "$UP_PLAIN" ]] && grep -qF "$UP_LINK is a file" <<<"$UP_OUT"; then
    up_ok "install.sh leaves a plain file alone" "exit $rc; byte-identical; named"
else
    up_fail "install.sh leaves a plain file alone" "rc $rc; $(grep 'install:' <<<"$UP_OUT" | tail -2 | tr '\n' ' ')"
fi
up_fresh_home; mkdir -p "$UP_T/fake"
printf '#!/bin/sh\necho "war 1.0.0-alpha.2"\n' > "$UP_T/fake/war"; chmod +x "$UP_T/fake/war"
UP_PATH="$UP_T/fake:$UP_HOME/.local/bin:$UP_SYS"
UP_OUT=$(up_run env OW_BIN_DIR="$UP_HOME/.local/bin" bash "$UP_SH" 2>&1); rc=$?
if grep -qF "WARNING: $UP_T/fake/war comes before $UP_HOME/.local/bin on PATH, and says: war 1.0.0-alpha.2" <<<"$UP_OUT"; then
    up_ok "install.sh names an older war earlier on PATH" "warns, naming it and 'war 1.0.0-alpha.2'"
else
    up_fail "install.sh names an older war earlier on PATH" "rc $rc; $(grep 'install:' <<<"$UP_OUT" | tail -2 | tr '\n' ' ')"
fi

# =========================================================================
# OBL-007 — the install document says what is sent, what is checked, and how a
# stranded install gets out
up_doc_check() { # <INSTALL.md> -> prints what is missing; 0 when nothing is
    local doc="$1" missing=() stranded
    grep -qF -- "$UP_RELEASES" "$doc" || missing+=("the RELEASES URL")
    grep -qF -- 'OPENWARRANT_NO_UPDATE_CHECK' "$doc" || missing+=("OPENWARRANT_NO_UPDATE_CHECK")
    grep -qF -- '`CI`' "$doc" || missing+=("CI")
    grep -qF -- 'not a signature' "$doc" || missing+=("'not a signature'")
    stranded=$(awk '/^## A stranded install/{on=1; next} /^## /{on=0} on' "$doc")
    grep -qxF -- "$UP_INSTALL_LINE" <<<"$stranded" || missing+=("the stranded section's install.sh line")
    [[ "$UP_INSTALL_LINE" == "$UP_REMEDY_UNMANAGED" ]] || missing+=("remedy's line")
    for c in release crate unreleased unknown; do grep -qF -- "| \`$c\` |" "$doc" || missing+=("class $c"); done
    [[ ${#missing[@]} -eq 0 ]] || { printf '%s; ' "${missing[@]}"; return 1; }
}
UP_MISSING=$(up_doc_check docs/INSTALL.md)
if [[ -z "$UP_MISSING" ]] && grep -qF '](docs/INSTALL.md)' README.md && grep -qF '](docs/INSTALL.md)' QUICKSTART.md; then
    up_ok "docs/INSTALL.md says what it must" "URL, both opt-outs, not a signature, stranded line = remedy, 4 classes; linked twice"
else
    up_fail "docs/INSTALL.md says what it must" "missing: ${UP_MISSING:-a link from README.md or QUICKSTART.md}"
fi
sed 's/OPENWARRANT_NO_UPDATE_CHECK/OPENWARRANT_NO_NOTICE/g' docs/INSTALL.md > "$UP_T/INSTALL.md"
if UP_MISSING=$(up_doc_check "$UP_T/INSTALL.md"); then
    up_fail "a renamed opt-out fails the document check" "the check passed a copy without OPENWARRANT_NO_UPDATE_CHECK"
else
    up_ok "a renamed opt-out fails the document check" "missing: $UP_MISSING"
fi

# =========================================================================
# OBL-005, last: the server stopped. A refused connection changes nothing a
# caller sees, prints nothing, and is cached as UNKNOWN.
UP_DATA="$UP_T/data"
up_managed
kill "$UP_PID" 2>/dev/null; wait "$UP_PID" 2>/dev/null; UP_PID=""
python3 "$UP_T/fixture.py" cache "$UP_CACHE" 200000 "v$UP_V"
up_pty "$UP_T/c.out" "$UP_T/c.err" -- --root "$PLANT_ROOT" status; rc_on=$?
for _ in $(seq 1 100); do grep -q '"unknown"' "$UP_CACHE" 2>/dev/null && ! grep -q 'in progress' "$UP_CACHE" && break; sleep 0.1; done
up_pty "$UP_T/c2.out" "$UP_T/c2.err" OPENWARRANT_NO_UPDATE_CHECK=1 -- --root "$PLANT_ROOT" status; rc_off=$?
if [[ "$rc_on" == "$rc_off" ]] && cmp -s "$UP_T/c.out" "$UP_T/c2.out" && [[ "$(up_notices "$UP_T/c.err")" == 0 ]] \
    && grep -q '"unknown"' "$UP_CACHE" && ! grep -q 'in progress' "$UP_CACHE"; then
    up_ok "connection refused: nothing changes, UNKNOWN cached" "exit $rc_on and stdout as disabled; no notice"
else
    up_fail "connection refused: nothing changes, UNKNOWN cached" "exit $rc_on vs $rc_off; $(up_notices "$UP_T/c.err") notice(s); cache $(head -c 160 "$UP_CACHE" 2>/dev/null)"
fi

[[ -n "${UP_PID:-}" ]] && kill "$UP_PID" 2>/dev/null
command rm -rf "$UP_T"
corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT UP_PID UP_T UP_OUT UP_OUT2 UP_DATA UP_HOME UP_PATH
