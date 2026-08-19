#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
#
# Planted-violation battery (SAS §92, §90).
#
# §92: the gate "SHALL exit zero only when every positive fixture passes and
# every planted violation is REJECTED BY THE INTENDED CONTROL."
#
# That last clause is why this script checks WHICH rule fired, not merely that
# something failed. A malformed fixture rejected by the TOML parser instead of by
# the duplicate-ordinal rule proves nothing about the rule it was meant to
# exercise, while looking identical in a pass/fail summary.
#
# Every plant mutates the working tree and is restored with `git checkout`, so
# the script refuses to run unless the tree is clean.

set -uo pipefail

# No unquoted word-splitting anywhere in this file: a `for X in $LIST` loop does
# not split in zsh, and the silent no-op that produces has already cost this
# fleet a 13-repository operation that did nothing while reporting success.

cd "$(dirname "${BASH_SOURCE[0]}")/.." || exit 1
WAR="./target/debug/war"

if [[ ! -x "$WAR" ]]; then
    echo "build first: cargo build --workspace" >&2
    exit 1
fi

# Only the tree the plants actually mutate has to be clean.
#
# Requiring a clean WORKING TREE would mean this step is skipped during ordinary
# development — and a gate step that is usually skipped is a gate step that is
# never run, which is the whole failure this battery exists to disprove. Editing
# Rust while the plants run is fine; editing docs/warrants/ is not, because the
# restore is `git checkout` and would discard that work.
if ! git diff --quiet -- docs/warrants/ docs/adr/ || ! git diff --cached --quiet -- docs/warrants/ docs/adr/; then
    echo "docs/warrants/ or docs/adr/ has uncommitted changes." >&2
    echo "Plants mutate those files and restore with 'git checkout', which would" >&2
    echo "discard your work. Commit or stash docs/warrants/ first." >&2
    exit 1
fi

PASSED=0
FAILED=0

restore() {
    git checkout -- docs/warrants/ docs/adr/ 2>/dev/null || true
}
trap restore EXIT

# plant <name> <expected-rule> <expected-detail> <expected-exit> <mutation> [check-args...]
#
# `expected-detail` is what makes this a real check. Four of the plants below all
# surface under the rule `manifest.invalid`, so matching the rule alone would let
# a duplicate-ordinal plant "pass" while actually being caught by the
# unknown-role branch. The detail pattern pins the specific violation.
plant() {
    local name="$1" rule="$2" detail="$3" want_exit="$4" mutate="$5"
    shift 5

    restore
    eval "$mutate"

    local out status
    out="$("$WAR" check "$@" 2>&1)"
    status=$?
    restore

    if [[ "$status" -ne "$want_exit" ]]; then
        printf 'FAIL  %-34s exit %s, wanted %s\n' "$name" "$status" "$want_exit"
        FAILED=$((FAILED + 1))
        return
    fi
    if ! grep -q -- "$rule" <<<"$out"; then
        printf 'FAIL  %-34s exited %s but rule %s never fired\n' "$name" "$status" "$rule"
        printf '      (rejected for the wrong reason — the failure §92 warns about)\n'
        FAILED=$((FAILED + 1))
        return
    fi
    if ! grep -q -- "$detail" <<<"$out"; then
        printf 'FAIL  %-34s rule %s fired but not for %s\n' "$name" "$rule" "$detail"
        printf '      (right rule, wrong violation — still the wrong reason)\n'
        FAILED=$((FAILED + 1))
        return
    fi
    printf 'ok    %-34s rejected by %s (%s)\n' "$name" "$rule" "$detail"
    PASSED=$((PASSED + 1))
}

echo "== positive fixture =="
if "$WAR" check --generated >/dev/null 2>&1; then
    printf 'ok    %-34s clean corpus reports well-formed\n' "corpus"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s clean corpus does not pass\n' "corpus"
    FAILED=$((FAILED + 1))
fi

echo
echo "== planted violations =="

plant "missing required atom" "manifest.invalid" "requires a milestones atom" 2 \
    "python3 - <<'EOF'
import pathlib
p = pathlib.Path('docs/warrants/OW-WAR-0001/manifest.toml')
s = p.read_text()
i = s.index('[[atoms]]\nordinal = 45')
j = s.index('[[atoms]]', i + 10)
p.write_text(s[:i] + s[j:])
EOF"

plant "duplicate ordinal" "manifest.invalid" "duplicate atom ordinal 10" 2 \
    "printf '\n[[atoms]]\nordinal = 10\nrole = \"basis\"\npath = \"atoms/20-basis.md\"\nrequired = true\n' >> docs/warrants/OW-WAR-0001/manifest.toml"

plant "unknown required role" "manifest.invalid" "neither a core role" 2 \
    "printf '\n[[atoms]]\nordinal = 55\nrole = \"hypothesis\"\npath = \"atoms/20-basis.md\"\nrequired = true\n' >> docs/warrants/OW-WAR-0001/manifest.toml"

plant "fabricated enterprise id" "manifest.invalid" "cannot be fabricated locally" 2 \
    "sed -i 's|^enterprise_id = \"\"|enterprise_id = \"OH-WAR-000042\"|' docs/warrants/OW-WAR-0001/manifest.toml"

plant "deleted atom file" "atom.missing" "20-basis.md" 2 \
    "rm docs/warrants/OW-WAR-0001/atoms/20-basis.md"

plant "stale parent contract digest" "relations.parent-digest" "no longer the one it was authorized against" 2 \
    "printf '\nAn edit to the parent that its children were never re-authorized against.\n' >> docs/warrants/OW-WAR-0001/atoms/10-intent.md"

plant "generated view edited by hand" "generated.drift" "edited by hand" 2 \
    "sed -i 's|^# OW-WAR-0001|# OW-WAR-0001 TAMPERED|' docs/warrants/OW-WAR-0001/generated/WAR.md" \
    --generated

plant "generated view deleted" "generated.missing" "WAR.json is missing" 2 \
    "rm docs/warrants/OW-WAR-0002/generated/WAR.json" \
    --generated

plant "frontmatter anchor injected" "atom.frontmatter" "YAML anchor" 2 \
    "sed -i 's|^role: intent|role: \&anchor intent|' docs/warrants/OW-WAR-0001/atoms/10-intent.md"

plant "ADR with an unknown status" "adr.malformed" "unknown ADR status" 2 \
    "sed -i 's|^status: accepted|status: probably-fine|' docs/adr/atoms/OW-ADR-0001-canonical-json-implementation.md"

plant "ADR missing a required key" "adr.malformed" "missing required frontmatter key" 2 \
    "sed -i '/^adr_uuid:/d' docs/adr/atoms/OW-ADR-0002-frontmatter-subset.md"

plant "ADR Overview edited by hand" "adr-overview.drift" "edited by hand" 2 \
    "sed -i 's|^# Architecture Decision Record Overview|# Architecture Decision Record Overview TAMPERED|' docs/adr/generated/ADR_OVERVIEW.md" \
    --generated

plant "Warrant Overview edited by hand" "warrant-overview.drift" "edited by hand" 2 \
    "sed -i 's|^# Warrant Overview|# Warrant Overview TAMPERED|' docs/warrants/generated/WARRANT_OVERVIEW.md" \
    --generated

# §91.2 test 12 — a composition cycle. This closes OW-WAR-0002 M2, which was
# unmet because the cycle detector had unit tests but was never exercised through
# the shipped binary.
plant "composition cycle (self-parent)" "composition.cycle" "OW-WAR-0002" 2 \
    "python3 - <<'EOF'
import pathlib
p = pathlib.Path('docs/warrants/OW-WAR-0002/manifest.toml')
s = p.read_text()
p.write_text(s.replace('ref = \"war://01a018db-19fc-7f2a-8e39-69730f255e33\"',
                       'ref = \"war://01a018db-19fc-7f34-92db-54b2dca5446d\"'))
EOF"

plant "milestone dangling stage_ref" "milestones.invalid" "which is not declared" 2 \
    "sed -i 's|stage_refs: \[\"STAGE-001\"\]|stage_refs: [\"STAGE-999\"]|' docs/warrants/OW-WAR-0001/atoms/45-milestones.yaml"

plant "milestone dependency cycle" "milestones.invalid" "dependency cycle" 2 \
    "python3 - <<'EOF'
import pathlib
p = pathlib.Path('docs/warrants/OW-WAR-0001/atoms/45-milestones.yaml')
s = p.read_text()
p.write_text(s.replace('  - id: \"M1\"\n    title:', '  - id: \"M1\"\n    depends_on: [\"M3\"]\n    title:', 1))
EOF"

plant "obligation with no scope" "obligations.invalid" "declares no scope" 2 \
    "python3 - <<'EOF'
import pathlib
p = pathlib.Path('docs/warrants/OW-WAR-0001/atoms/60-assurance.md')
s = p.read_text()
i = s.index('- **scope:**')
j = s.index('\n', i)
p.write_text(s[:i] + s[j+1:])
EOF"

plant "dangling obligation_refs" "obligations.dangling-ref" "which is not declared" 2 \
    "sed -i 's|obligation_refs: \[\"OBL-001\"\]|obligation_refs: [\"OBL-999\"]|' docs/warrants/OW-WAR-0001/atoms/45-milestones.yaml"

plant "milestone carrying a stage field" "milestones.invalid" "belongs to a stage" 2 \
    "python3 - <<'EOF'
import pathlib
p = pathlib.Path('docs/warrants/OW-WAR-0001/atoms/45-milestones.yaml')
s = p.read_text()
p.write_text(s.replace('  - id: \"M1\"\n    title:', '  - id: \"M1\"\n    executor_kind: \"human\"\n    title:', 1))
EOF"

echo
echo "$PASSED passed, $FAILED failed"
[[ "$FAILED" -eq 0 ]]
