# shellcheck shell=bash
# §56.1 requirement 3 reads the correction chain (OW-WAR-0069).
#
# It used to read only `provenance.content_digest`, so after a correction was
# signed `war check` printed `deliverable.corrected` while `war resolve
# --dry-run` reported *artifact digests verify — not established* for the same
# bytes. Eighteen Warrants were in that state. §34.4 supersedes, it does not
# erase, so the digest a deliverable OUGHT to have is the head of its chain.
#
# OW-WAR-0062 is the fixture: resolved, and its D-003 (README.md) carries a
# correction. README.md is outside `restore`'s paths, so each mutation here
# checks it out by name.

R3_ALIAS=OW-WAR-0062
R3_FILE=README.md

# README.md has since moved under a later Warrant (OW-WAR-0116 governs it,
# OW-ADR-0021), so the live file is no longer 0062's chain head. The fixture
# is the chain head's own bytes, found in history by digest — the plants test
# the correction chain, not whoever edits README.md next.
R3_HEAD=$(sed -n 's/^new_digest = "sha256:\([0-9a-f]*\)"/\1/p' "$(ls docs/warrants/$R3_ALIAS/corrections/D-003-*.toml | sort -V | tail -1)")
R3_BYTES=$(mktemp)
for R3_C in $(git rev-list HEAD -- "$R3_FILE"); do
    git show "$R3_C:$R3_FILE" > "$R3_BYTES" 2>/dev/null || continue
    [[ "$(sha256sum "$R3_BYTES" | cut -d' ' -f1)" == "$R3_HEAD" ]] && break
    : > "$R3_BYTES"
done
[[ -s "$R3_BYTES" ]] || { printf 'PLANT SETUP FAILED: no commit holds %s at the chain head %s\n' "$R3_FILE" "$R3_HEAD" >&2; exit 9; }
command cp "$R3_BYTES" "$R3_FILE"

# The positive: a corrected deliverable satisfies requirement 3, and the whole
# Warrant still meets the thirteen.
out=$("$WAR" resolve "$R3_ALIAS" --dry-run 2>&1)
if grep -q 'all 13 §56.1 requirements are met' <<< "$out" \
    && ! grep -q 'artifact digests verify — not established' <<< "$out"; then
    printf 'ok    %-34s the chain head is the digest it wants\n' "a corrected deliverable resolves"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "a corrected deliverable resolves" "$(grep -i 'digest' <<< "$out" | head -1)"
    FAILED=$((FAILED + 1))
fi

# The negative: bytes that match NEITHER the pin nor the chain head are still
# unverified. A fix that made requirement 3 pass unconditionally would show up
# here, not in the positive above.
printf '\n<!-- planted drift -->\n' >> "$R3_FILE"
assert_present 'planted drift' "$R3_FILE"
out=$("$WAR" resolve "$R3_ALIAS" --dry-run 2>&1)
command cp "$R3_BYTES" "$R3_FILE"
if grep -q 'artifact digests verify — not established' <<< "$out"; then
    printf 'ok    %-34s drift past the chain head is unmet\n' "requirement 3 still fails closed"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s drift was not reported\n' "requirement 3 still fails closed"
    FAILED=$((FAILED + 1))
fi

# And a correction chain that cannot be read is not a verified digest either:
# requirement 3 is unmet rather than assumed.
R3_CORR=$(ls docs/warrants/$R3_ALIAS/corrections/*.toml | head -1)
# Restored by checkout, not by copying bytes around: the journal witnesses
# this record's own digest, so a restore that lost the trailing newline would
# leave the corpus reporting `correction.edited` for the rest of the battery —
# and a checkout is what the battery's EXIT trap would do anyway if this were
# killed between the mutation and the restore.
printf 'this is not toml = = =\n' > "$R3_CORR"
assert_present 'not toml' "$R3_CORR"
out=$("$WAR" resolve "$R3_ALIAS" --dry-run 2>&1)
git checkout -- "$R3_CORR"
# The chain-head fixture is done with: the live README.md comes back.
git checkout -- "$R3_FILE" 2>/dev/null || true
command rm -f "$R3_BYTES"
if grep -q 'artifact digests verify — not established' <<< "$out"; then
    printf 'ok    %-34s an unreadable chain is unmet\n' "requirement 3 fails closed on TOML"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s an unreadable chain still passed\n' "requirement 3 fails closed on TOML"
    FAILED=$((FAILED + 1))
fi

# The projection asks the admissibility question the resolution rules ask
# (OW-WAR-0069). `obligation_views` reported the first matching verification's
# disposition verbatim, so one CORPUS_STATUS.json object could carry
# `established` for an obligation the same file listed as unestablished. A
# self-verification is the cheapest inadmissible record to plant.
R3_VER=docs/warrants/OW-WAR-0046/verifications/OBL-001.toml
sed -i 's/^actor = "lamu:mimo-v2.5-pro"/actor = "claude"/' "$R3_VER"
assert_present 'actor = "claude"' "$R3_VER"
"$WAR" compile > /dev/null 2>&1
R3_ROW=$(python3 - <<'PY'
import json
d = json.load(open("docs/warrants/generated/CORPUS_STATUS.json"))
for w in d["warrants"]:
    if w["alias"] != "OW-WAR-0046":
        continue
    for o in w.get("obligations", []):
        if o["id"] == "OBL-001":
            print(o["disposition"], "|", o.get("inadmissible_because", ""))
PY
)
restore
"$WAR" compile > /dev/null 2>&1
if grep -q '^inadmissible' <<< "$R3_ROW" && grep -qi 'verif' <<< "$R3_ROW"; then
    printf 'ok    %-34s %s\n' "a self-verification is inadmissible" "${R3_ROW%% |*}, with a reason"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s projected %s\n' "a self-verification is inadmissible" "$R3_ROW"
    FAILED=$((FAILED + 1))
fi
