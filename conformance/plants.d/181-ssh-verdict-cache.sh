# shellcheck shell=bash
# The `ssh-keygen -Y verify` verdict cache (crates/openwarrant-cli/src/ssh_verdicts.rs).
#
# `war next` and `war status` asked `ssh-keygen` once per distinct signed
# response on every run (172 and 114 spawns on this repository). They now
# keep a pass, keyed by the digest of every byte `ssh-keygen` reads, under
# `<git common dir>/openwarrant/cache/ssh-verdicts/`. On a clone of this
# repository's HEAD (signatures it already carries; nothing is signed here),
# with an `ssh-keygen` shim on PATH that counts each run of the real one:
# - cold, `war status --json` records passes; warm, the same bytes are
#   printed and `ssh-keygen` is not run; with OPENWARRANT_NO_CACHE=1, the same
#   bytes again, every signature asked;
# - `war check` and `war sign --list` never read it (each run asks
#   `ssh-keygen`, warm or not) and never write it;
# - refused, with the cache warm: one byte of a signature flipped. `war
#   status` prints exactly what it prints with the cache off, which is not
#   what it printed before the flip, and no entry is added;
# - refused, with the cache warm: allowed_signers naming a throwaway key for
#   every principal. Same bytes as with the cache off, unlike the untouched
#   output, and no entry is added.

echo "== the ssh-keygen verdict cache: a pass for exactly these bytes, never a judgement =="

SV_T=$(mktemp -d)
SV_C="$SV_T/clone"
SV_WAR="$REPO_ROOT/${WAR#./}"
SV_LOG="$SV_T/ssh-keygen.log"
SV_DIR="$SV_C/.git/openwarrant/cache/ssh-verdicts"
sv_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
sv_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
SV_REAL=$(command -v ssh-keygen)
git clone -q --no-hardlinks "$REPO_ROOT" "$SV_C" >/dev/null 2>&1 \
    || { printf 'PLANT SETUP FAILED: clone of %s\n' "$REPO_ROOT" >&2; exit 9; }
[[ -n "$SV_REAL" ]] || { printf 'PLANT SETUP FAILED: no ssh-keygen on PATH\n' >&2; exit 9; }
mkdir -p "$SV_T/bin"
printf '#!/bin/sh\necho run >> "$SV_SHIM_LOG"\nexec %s "$@"\n' "$SV_REAL" > "$SV_T/bin/ssh-keygen"
chmod +x "$SV_T/bin/ssh-keygen"
# sv_war <out> <args...>: run war on the clone with the shim; the number of
# ssh-keygen runs it made is left in SV_RUNS.
sv_war() {
    local out="$1"; shift
    : > "$SV_LOG"
    env -u SSH_AUTH_SOCK -u SSH_AGENT_PID PATH="$SV_T/bin:$PATH" SV_SHIM_LOG="$SV_LOG" \
        "$SV_WAR" --root "$SV_C" "$@" > "$SV_T/$out" 2>/dev/null </dev/null
    SV_RUNS=$(wc -l < "$SV_LOG")
}
sv_entries() { find "$SV_DIR" -maxdepth 1 -type f 2>/dev/null | wc -l; }
sv_same() { cmp -s "$SV_T/$1" "$SV_T/$2"; }

# --- cold, warm, off ---------------------------------------------------------------
sv_war cold.json --json status; SV_COLD_RUNS=$SV_RUNS
SV_COLD_ENTRIES=$(sv_entries)
sv_war warm.json --json status; SV_WARM_RUNS=$SV_RUNS
OPENWARRANT_NO_CACHE=1 sv_war off.json --json status; SV_OFF_RUNS=$SV_RUNS
if [[ $SV_COLD_RUNS -gt 0 && $SV_COLD_ENTRIES -gt 0 && $SV_WARM_RUNS -eq 0 \
    && $SV_OFF_RUNS -eq $SV_COLD_RUNS && $(sv_entries) -eq $SV_COLD_ENTRIES ]] \
    && sv_same cold.json warm.json && sv_same cold.json off.json; then
    sv_ok "a pass is not asked twice" "cold $SV_COLD_RUNS runs, $SV_COLD_ENTRIES entries; warm 0 runs; off $SV_OFF_RUNS runs; same bytes"
else
    sv_fail "a pass is not asked twice" "cold $SV_COLD_RUNS runs, $SV_COLD_ENTRIES entries; warm $SV_WARM_RUNS; off $SV_OFF_RUNS; entries now $(sv_entries)"
fi

# --- the judging commands never use it ------------------------------------------------
sv_war check1.json --json check; SV_CHECK1=$SV_RUNS
sv_war check2.json --json check; SV_CHECK2=$SV_RUNS
sv_war list.json --json sign --list; SV_LIST=$SV_RUNS
if [[ $SV_CHECK1 -gt 0 && $SV_CHECK1 -eq $SV_CHECK2 && $SV_LIST -gt 0 \
    && $(sv_entries) -eq $SV_COLD_ENTRIES ]] && sv_same check1.json check2.json; then
    sv_ok "check and sign never read it" "check asks ssh-keygen $SV_CHECK1 times warm, twice over; sign --list $SV_LIST; no entry written"
else
    sv_fail "check and sign never read it" "check $SV_CHECK1 then $SV_CHECK2 runs; sign --list $SV_LIST; entries $(sv_entries) (were $SV_COLD_ENTRIES)"
fi

# --- refused: one byte of a signature, cache warm --------------------------------------
# The first authorization signature whose flip `war status` can see (with the
# cache off): a flip nothing reads would prove nothing.
SV_SIG=""
mapfile -t SV_SIGS < <(cd "$SV_C" && ls docs/authority/responses/*-WAR-[0-9][0-9][0-9][0-9].response.toml.sig 2>/dev/null | sort -r | head -5)
for sv_s in ${SV_SIGS[@]+"${SV_SIGS[@]}"}; do
    cp "$SV_C/$sv_s" "$SV_T/sig.orig"
    python3 - "$SV_C/$sv_s" <<'PY'
import sys
p = sys.argv[1]
b = bytearray(open(p, "rb").read())
i = b.index(b"\n") + 11
b[i] = ord("A") if b[i] != ord("A") else ord("B")
open(p, "wb").write(bytes(b))
PY
    OPENWARRANT_NO_CACHE=1 sv_war tampered-off.json --json status
    if ! sv_same cold.json tampered-off.json; then SV_SIG="$sv_s"; break; fi
    cp "$SV_T/sig.orig" "$SV_C/$sv_s"
done
if [[ -z "$SV_SIG" ]]; then
    printf 'PLANT MUTATION WAS A NO-OP: no flipped signature among %s changed war status\n' "${SV_SIGS[*]}" >&2
    rm -rf "$SV_T"
    exit 9
fi
SV_BEFORE=$(sv_entries)
sv_war tampered-warm.json --json status; SV_TAMPER_RUNS=$SV_RUNS
if [[ $SV_TAMPER_RUNS -gt 0 && $(sv_entries) -eq $SV_BEFORE ]] \
    && sv_same tampered-warm.json tampered-off.json && ! sv_same tampered-warm.json cold.json; then
    sv_ok "a flipped signature is refused" "${SV_SIG##*/}: warm cache prints what the cache-off run prints, not the untouched bytes; asked $SV_TAMPER_RUNS times; nothing recorded"
else
    sv_fail "a flipped signature is refused" "${SV_SIG##*/}: runs $SV_TAMPER_RUNS; entries $(sv_entries) (were $SV_BEFORE); same as off: $(sv_same tampered-warm.json tampered-off.json && echo yes || echo no)"
fi
cp "$SV_T/sig.orig" "$SV_C/$SV_SIG"

# --- refused: allowed_signers names another key, cache warm ----------------------------
env -u SSH_AUTH_SOCK -u SSH_AGENT_PID "$SV_REAL" -q -t ed25519 -N "" -C plant-other -f "$SV_T/other" </dev/null
SV_OTHER=$(cut -d' ' -f1,2 "$SV_T/other.pub")
cp "$SV_C/docs/authority/allowed_signers" "$SV_T/allowed.orig"
python3 - "$SV_C/docs/authority/allowed_signers" "$SV_OTHER" <<'PY'
import re, sys
p, key = sys.argv[1], sys.argv[2]
out = []
for line in open(p).read().splitlines():
    if line.strip() and not line.lstrip().startswith("#"):
        line = re.sub(r"(ssh-[a-z0-9-]+|sk-[a-z0-9@.-]+|ecdsa-[a-z0-9-]+) [A-Za-z0-9+/=]+", key, line)
    out.append(line)
open(p, "w").write("\n".join(out) + "\n")
PY
SV_BEFORE=$(sv_entries)
sv_war regranted-warm.json --json status; SV_REGRANT_RUNS=$SV_RUNS
OPENWARRANT_NO_CACHE=1 sv_war regranted-off.json --json status
if [[ $SV_REGRANT_RUNS -gt 0 && $(sv_entries) -eq $SV_BEFORE ]] \
    && sv_same regranted-warm.json regranted-off.json && ! sv_same regranted-warm.json cold.json; then
    sv_ok "another allowed key is refused" "every principal's key replaced: warm prints what cache-off prints, not the untouched bytes; asked $SV_REGRANT_RUNS times; nothing recorded"
else
    sv_fail "another allowed key is refused" "runs $SV_REGRANT_RUNS; entries $(sv_entries) (were $SV_BEFORE); same as off: $(sv_same regranted-warm.json regranted-off.json && echo yes || echo no)"
fi
cp "$SV_T/allowed.orig" "$SV_C/docs/authority/allowed_signers"

rm -rf "$SV_T"
unset SV_T SV_C SV_WAR SV_LOG SV_DIR SV_REAL SV_SIG SV_SIGS SV_BEFORE SV_RUNS
