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
# restore is `git checkout` and would discard that work. docs/gates/ joined this
# list when gate plants landed: the guard and the restore must name the same
# paths, or a plant silently deletes work the guard said it was protecting.
if ! git diff --quiet -- docs/warrants/ docs/adr/ docs/gates/ docs/sas/ openwarrant.toml \
    || ! git diff --cached --quiet -- docs/warrants/ docs/adr/ docs/gates/ docs/sas/ openwarrant.toml; then
    echo "docs/warrants/, docs/adr/, docs/gates/, docs/sas/ or openwarrant.toml has uncommitted changes." >&2
    echo "Plants mutate those files and restore with 'git checkout', which would" >&2
    echo "discard your work. Commit or stash those first." >&2
    exit 1
fi

PASSED=0
FAILED=0

restore() {
    git checkout -- docs/warrants/ docs/adr/ docs/gates/ docs/sas/ openwarrant.toml 2>/dev/null || true
    # `git checkout` restores TRACKED files and leaves untracked ones behind, so
    # a plant that CREATES a file is not undone by it. AM-999 is exactly that —
    # the §91.4 test 24 positive fixture — and it leaked into a commit once
    # before this line existed. Named explicitly rather than `git clean`, which
    # would delete a developer's untracked work.
    rm -f docs/warrants/OW-WAR-0046/amendments/AM-999.yaml
}

# The mirror of `assert_gone`, for a mutation that ADDS rather than removes.
# Same reason: a sed that matched nothing leaves the corpus valid and the plant
# then scores the untouched happy path.
assert_present() {
    if ! grep -Fq -- "$1" "$2"; then
        printf 'PLANT MUTATION WAS A NO-OP: %s never appeared in %s\n' "$1" "$2" >&2
        printf 'The plant would have scored the UNMUTATED corpus. Fix the pattern.\n' >&2
        restore
        exit 9
    fi
}

assert_gone() {
    if grep -Fq -- "$1" "$2"; then
        printf 'PLANT MUTATION WAS A NO-OP: %s still present in %s\n' "$1" "$2" >&2
        printf 'The plant would have scored the UNMUTATED corpus. Fix the pattern.\n' >&2
        restore
        exit 9
    fi
}

trap restore EXIT

# plant_cmd <name> <expected-rule> <expected-detail> <expected-exit> <mutation> <args...>
#
# The general form: run any `war` subcommand rather than `check` or `gate --run`.
# plant() and plant_gate() predate it and are kept because their call sites read
# better; new plants for new subcommands use this.
plant_cmd() {
    local name="$1" rule="$2" detail="$3" want_exit="$4" mutate="$5"
    shift 5

    restore
    eval "$mutate"

    local out status
    out="$("$WAR" "$@" 2>&1)"
    status=$?
    restore

    if [[ "$status" -ne "$want_exit" ]]; then
        printf 'FAIL  %-34s exit %s, wanted %s\n' "$name" "$status" "$want_exit"
        FAILED=$((FAILED + 1))
        return
    fi
    if ! grep -q -- "$rule" <<<"$out"; then
        printf 'FAIL  %-34s exited %s but %s never appeared\n' "$name" "$status" "$rule"
        FAILED=$((FAILED + 1))
        return
    fi
    if ! grep -q -- "$detail" <<<"$out"; then
        printf 'FAIL  %-34s %s appeared but not for %s\n' "$name" "$rule" "$detail"
        FAILED=$((FAILED + 1))
        return
    fi
    printf 'ok    %-34s rejected by %s (%s)\n' "$name" "$rule" "$detail"
    PASSED=$((PASSED + 1))
}

# plant_gate <name> <expected-rule> <expected-detail> <expected-exit> <mutation>
#
# Same assertions as plant(), driving `war gate --run` instead of `war check`.
# §44's statuses are only observable when a gate is actually executed, and
# OW-WAR-0020's OBL-002 wants each status reported AS ITSELF — so the detail
# string carries the reason code, which is the thing that must not collapse.
plant_gate() {
    local name="$1" rule="$2" detail="$3" want_exit="$4" mutate="$5"

    restore
    eval "$mutate"

    local out status
    out="$("$WAR" gate --run 2>&1)"
    status=$?
    restore

    if [[ "$status" -ne "$want_exit" ]]; then
        printf 'FAIL  %-34s exit %s, wanted %s\n' "$name" "$status" "$want_exit"
        FAILED=$((FAILED + 1))
        return
    fi
    if ! grep -q -- "$rule" <<<"$out"; then
        printf 'FAIL  %-34s exited %s but rule %s never fired\n' "$name" "$status" "$rule"
        FAILED=$((FAILED + 1))
        return
    fi
    if ! grep -q -- "$detail" <<<"$out"; then
        printf 'FAIL  %-34s rule %s fired but not for %s\n' "$name" "$rule" "$detail"
        FAILED=$((FAILED + 1))
        return
    fi
    printf 'ok    %-34s rejected by %s (%s)\n' "$name" "$rule" "$detail"
    PASSED=$((PASSED + 1))
}

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

plant "controlled Warrant with no adequacy review" "assurance.adequacy-review" "requires a contract-adequacy review" 2 \
    "python3 - <<'EOF'
import pathlib
p = pathlib.Path('docs/warrants/OW-WAR-0003/atoms/60-assurance.md')
s = p.read_text()
i = s.index('## Gate Adequacy')
j = s.index('## ', i + 3)
p.write_text(s[:i] + s[j:])
EOF"

plant "adequacy review with no adversarial question" "assurance.adequacy-review" "records no adversarial question" 2 \
    "python3 - <<'EOF'
import pathlib
p = pathlib.Path('docs/warrants/OW-WAR-0003/atoms/60-assurance.md')
s = p.read_text()
i = s.index('## Gate Adequacy')
j = s.index('## ', i + 3)
p.write_text(s[:i] + '## Gate Adequacy\n\nRequired at the declared level. Reviewed and found satisfactory.\n\n' + s[j:])
EOF"

plant "obligation citing an unregistered gate" "gate.unresolved" "not in the registry" 2 \
    "sed -i 's|- \\*\\*gate:\\*\\* \`gate://software.repo.war-check@1.0.0\`|- **gate:** \`gate://does-not-exist@1.0.0\`|' docs/warrants/OW-WAR-0019/atoms/60-assurance.md"

plant "citing a draft (unqualified) gate" "gate.not-bindable" "permits binding only a qualified gate" 2 \
    "sed -i 's|^lifecycle: \"qualified\"|lifecycle: \"draft\"|' docs/gates/software.repo.war-check@1.0.0.yaml"

plant "gate definition with no version" "gate.invalid" "declares no version" 2 \
    "sed -i 's|^version: \"1.0.0\"|version: \"\"|' docs/gates/software.repo.war-check@1.0.0.yaml"

plant "qualification with no negative control" "gate.invalid" "no negative controls" 2 \
    "sed -i 's|^qualification_negative_controls: .*|qualification_negative_controls: []|' docs/gates/software.repo.war-check@1.0.0.yaml"

plant "a declared fault class that was not detected" "gate.invalid" "records no detection result" 2 \
    "sed -i '0,/    detected: \"true\"/s//    detected: \"false\"/' docs/gates/software.repo.war-check@1.0.0.yaml"

plant "a placeholder where a digest belongs" "gate.invalid" "which is not a digest" 2 \
    "sed -i 's|^qualification_digest: \"\"|qualification_digest: \"sha256:pending\"|' docs/gates/software.repo.war-check@1.0.0.yaml"

# ---------------------------------------------------------------------------
# §44 gate-run statuses. OW-WAR-0020 OBL-002: each is reported AS ITSELF, and
# `missing_tool` specifically must not be reported as `failed`. The parent
# project lost 51 gates to exactly that collapse (measured once at LamQuant
# 5369da81, 2026-08-17; historical, and being repaired -- see the README).
# ---------------------------------------------------------------------------

plant_gate "a gate whose tool is absent" "gate-run.unaskable" "missing_tool" 2 \
    "sed -i 's|^argv: .*|argv: [\"definitely-not-a-real-tool\"]|' docs/gates/software.repo.war-check@1.0.0.yaml"

plant_gate "a gate whose script is absent" "gate-run.unaskable" "missing_script" 2 \
    "sed -i 's|^argv: .*|argv: [\"./tools/does-not-exist.sh\"]|' docs/gates/software.repo.war-check@1.0.0.yaml"

plant_gate "a gate that declares no command" "gate-run.unaskable" "malformed" 2 \
    "sed -i 's|^argv: .*|argv: []|' docs/gates/software.repo.war-check@1.0.0.yaml"

plant_gate "a mutating gate in a routine run" "gate-run.unaskable" "mutating" 2 \
    "sed -i 's|^mutating: \"false\"|mutating: \"true\"|' docs/gates/software.repo.war-check@1.0.0.yaml"

# The other side of the same coin: a gate that WAS asked and answered no must
# report as a failure, not as an unknown. If this and the missing-tool plant
# above ever produce the same rule, the two have collapsed into one.
# A timeout is NOT "could not ask". This gate is asked, starts, and never
# answers; it must land on its own rule rather than borrowing the unaskable one.
# Keyed on the verdict being unknown instead of on askability, the runner told
# the reader "could not ask" about a gate it had just spawned.
plant_gate "a gate that never answers" "gate-run.no-result" "asked, but produced no result" 2 \
    "sed -i 's|^argv: .*|argv: [\"sleep\", \"30\"]|; s|^timeout_secs: .*|timeout_secs: \"1\"|' docs/gates/software.repo.war-check@1.0.0.yaml"

plant_gate "a gate that runs and fails" "gate-run.fail" "verdict fail" 2 \
    "sed -i 's|^argv: .*|argv: [\"false\"]|' docs/gates/software.repo.war-check@1.0.0.yaml"

# ---------------------------------------------------------------------------
# §17.5 projections, §71.10 diff, §74.4 planning gate.
# ---------------------------------------------------------------------------

plant_cmd "an unknown projection name" "17.5 defines" "full_warrant" 1 \
    "true" show OW-WAR-0001 --view pretty_print

plant_cmd "a hand-edited canonical JSON" "diff.changed" "contract_revision" 0 \
    "python3 - <<'EOF'
import pathlib, json
p = pathlib.Path('docs/warrants/OW-WAR-0001/generated/WAR.json')
d = json.loads(p.read_text())
d['contract_revision'] = 99
p.write_text(json.dumps(d, indent=2))
EOF" diff OW-WAR-0001

# §40.7 #1 — a performer's own report admitted as independent evidence. This
# rule existed through the whole of alpha in epistemic.rs and was called by
# NOTHING in the check path; OW-WAR-0046 wired it into obligation parsing, and
# this plant is what proves it is now reached by the shipped binary rather than
# only by a #[test].
plant "a performer report admitted as independent" "obligations.invalid" "performer assertion" 2 \
    "python3 - <<'EOF'
import pathlib
p = pathlib.Path('docs/warrants/OW-WAR-0001/atoms/60-assurance.md')
s = p.read_text()
i = s.index('- **scope:**')
p.write_text(s[:i] + '- **origin:** performer\n- **admissibility:** independent\n' + s[i:])
EOF"

# §44.6 — a run that completes produces a receipt. Blank the gate's argv so the
# run is `not_askable` and confirm NO receipt is minted: a receipt for a gate
# that never executed would be evidence of something that did not happen.
plant_gate "a receipt for a run that never happened" "gate-run.unaskable" "malformed" 2 \
    "sed -i 's|^argv: .*|argv: []|' docs/gates/software.repo.war-check@1.0.0.yaml
     rm -f docs/receipts/software_repo_war-check_1_0_0.receipt.json"

# §46.3 independence. Two plants, because one direction proves nothing.
#
# Expected exit is 0, not 2: both outcomes are WARNINGS, and `war check` exits
# non-zero only on an unknown or an error. The rule and detail assertions are
# what carry the weight here, which is exactly why the harness checks all three.
#
# A checker that always warns scores identically to one that works, so the second
# plant asserts the check can also PASS when independence is actually declared
# sufficient. That is the same negative-control logic §43.4 qualification
# requires of a gate, applied to a diagnostic.
plant_cmd "independence not declared at all" "independence.undeclared" "not the same as none" 0 \
    "python3 - <<'EOF'
import pathlib
p = pathlib.Path('openwarrant.toml')
s = p.read_text()
p.write_text(s[:s.index('[independence]')])
EOF" check --generated

plant_cmd "independence declared sufficient" "independence.sufficient" "meets §46.3" 0 \
    "python3 - <<'EOF'
import pathlib
p = pathlib.Path('openwarrant.toml')
s = p.read_text()
p.write_text(s.replace('= false', '= true'))
EOF" check --generated

# §56.1 — resolution refuses while its thirteen requirements are unmet.
#
# No mutation: this asserts the CURRENT behaviour of a real corpus, which makes
# it a regression guard rather than a planted fault. It is here because the
# failure it guards against is silent — a resolver that started closing Warrants
# would look like progress.
plant_cmd "resolution blocked by unmet requirements" "resolution.requirement-unmet" "independence requirements are met" 2 \
    "true" resolve OW-WAR-0001 --dry-run

# §56.2 — recording a resolution needs an authorizer and a stated meaning, and
# no authority model exists to supply them. Asking for a real resolution must be
# refused rather than quietly downgraded to a dry run.
plant_cmd "a resolution recorded with no authority" "authorizer" "OW-WAR-0044" 1 \
    "true" resolve OW-WAR-0001

# §31 — an amendment record that is malformed is worse than none, because it
# looks like a reason. Three plants: no reason, no authorizer, and a semantic
# diff naming something that is not a §28.5 contract element.
plant "an amendment with no stated reason" "amendment.invalid" "reason" 2 \
    "sed -i 's|^reason: .*|reason: \"\"|' docs/warrants/OW-WAR-0046/amendments/AM-001.yaml"

plant "an amendment with no authorizer" "amendment.invalid" "authorizer" 2 \
    "sed -i 's|^authorizer: .*|authorizer: \"\"|' docs/warrants/OW-WAR-0046/amendments/AM-001.yaml"

plant "an amendment diffing a non-element" "amendment.invalid" "28.5 element" 2 \
    "sed -i 's|element: \"deliverables\"|element: \"vibes\"|' docs/warrants/OW-WAR-0046/amendments/AM-001.yaml"

# §91.11 — evidence integrity, tests 76 through 81. Five separate plants rather
# than one, because a single rule covering five omissions cannot tell you which
# fired, and "evidence is malformed" is not an actionable finding.

# test 76 — evidence with no digest cannot be checked for corruption.
plant "evidence with no content digest" "evidence.invalid" "digest" 2 \
    "sed -i '0,/^- \\*\\*digest:\\*\\* /{s|^- \\*\\*digest:\\*\\* .*|- **digest:** |}' docs/warrants/OW-WAR-0046/atoms/60-assurance.md"

# test 77 — an observation with no method is an assertion about a method.
plant "an observation with no method" "evidence.invalid" "method" 2 \
    "sed -i 's|^- \\*\\*method:\\*\\* each plant asserts.*|- **method:** |' docs/warrants/OW-WAR-0046/atoms/60-assurance.md"

# test 78 — an inference with no premises has not reasoned from anything.
plant "an inference with no premises" "evidence.invalid" "no premises" 2 \
    "sed -i 's|^- \\*\\*premises:\\*\\* OBS-002|- **premises:** |' docs/warrants/OW-WAR-0046/atoms/60-assurance.md"

# test 79 — a judgment with no stated meaning. §42: "An approval with no stated
# meaning is invalid."
plant "a judgment with no stated meaning" "evidence.invalid" "meaning" 2 \
    "python3 - <<'EOF'
import pathlib, re
p = pathlib.Path('docs/warrants/OW-WAR-0046/atoms/60-assurance.md')
s = p.read_text()
p.write_text(re.sub(r'- \\*\\*meaning:\\*\\* .*?(?=\\n- \\*\\*basis)', '- **meaning:** ', s, flags=re.S))
EOF"

# test 81 — an author supplying its own recorded_at. §40.2 assigns it elsewhere.
plant "an author supplying recorded_at" "evidence.invalid" "recorded" 2 \
    "sed -i '0,/^- \\*\\*occurred at:\\*\\*/{s|^- \\*\\*occurred at:\\*\\*|- **recorded at:** 2026-01-01\\n- **occurred at:**|}' docs/warrants/OW-WAR-0046/atoms/60-assurance.md"

# A premise naming a record nobody wrote — the §40.4 analogue of the dangling
# obligation_ref OW-WAR-0016 caught.
plant "an inference on a premise nobody wrote" "evidence.invalid" "not a declared record" 2 \
    "sed -i 's|^- \\*\\*premises:\\*\\* OBS-002|- **premises:** OBS-999|' docs/warrants/OW-WAR-0046/atoms/60-assurance.md"

# §91.10 tests 64, 65, 73, 74. Tests 66, 67 and 69 are planted above as
# `a gate whose tool is absent`, `a gate that never answers`, and `a performer
# report admitted as independent`.

# test 74 — a claim/evidence graph with a cycle. A record citing ITSELF as its
# own premise resolves perfectly well, and supports nothing.
plant "evidence that rests only on itself" "evidence.invalid" "Circular" 2 \
    "sed -i 's|^- [*][*]premises:[*][*] OBS-002|- **premises:** INF-001|' docs/warrants/OW-WAR-0046/atoms/60-assurance.md"

# test 73 — a sampled result cannot establish a universal claim (§38.4).
plant "a universal claim resting on a sample" "obligations.invalid" "sampling alone is insufficient" 2 \
    "sed -i -e '0,/^- [*][*]scope:[*][*]/s||- **scope kind:** universal\n- **scope:**|' \
        -e '0,/^- [*][*]evidence:[*][*] a §44.6 receipt/s||- **evidence:** a representative sample of receipts|' docs/warrants/OW-WAR-0046/atoms/60-assurance.md"

# test 65 — a gate that can select nothing is INVALID, not a pass. A run that
# selected nothing and exited zero has measured nothing.
plant_gate "a gate that selects nothing" "gate-run.unaskable" "malformed" 2 \
    "sed -i 's|^argv: .*|argv: []|' docs/gates/software.repo.war-check@1.0.0.yaml"

# §49.2 / §91.7 test 47 — an unsupported lowering FAILS rather than degrades.
#
# Degradation is silent by nature: a PlanSpec that quietly dropped a stage still
# runs, and still produces artifacts. The refusal is the evidence; a successful
# lowering is not.
plant_cmd "a lowering with no computational stage" "blut.not-computational" "reject, not degrade" 0 \
    "sed -i 's|executor_kind: \"blut\"|executor_kind: \"human\"|' docs/warrants/OW-WAR-0047/atoms/45-milestones.yaml" blut OW-WAR-0047

# §49.3 — BLUT's authority is not duplicated. The lowering carries a pinned
# registry identity, never BLUT's lineage.
plant_cmd "a lowering against no pinned registry" "blut.lowered" "pinned registry" 0 \
    "true" blut OW-WAR-0047

# §91.13 tests 91-94 — §68 preservation.
#
# The export REFUSES today, and that is the test passing rather than failing:
# §68.2 names fourteen required contents and this repository can supply ten.
# A package that omitted four and looked complete would be the defect.
plant_cmd "a §68 export missing required contents" "68.2 requires" "audit receipts" 1 \
    "true" export OW-WAR-0044

# §68.3's VACUOUS comparison, which is the failure mode the section warns about.
# Two exports that both omit the same evidence agree about nothing in particular,
# so identical digests are not enough — the bytes must have been reconnected.
plant_cmd "a round trip that never reconnected the bytes" "not reconnected" "missing the same data" 1 \
    "true" export OW-WAR-0044 --round-trip

# The positive control. Without it, a build that refused EVERY round trip would
# satisfy the plant above and look like a working §68.3 check.
plant_cmd "a round trip with the bytes reconnected" "round trip verified" "sha256:" 0 \
    "true" export OW-WAR-0044 --round-trip --reconnect

# §67 — the Knowledge Fabric seam WRITES, and must be hard to reach by accident.
#
# `war kf act` POSTs a typed action to an authoritative external record. A seam
# that is easy to trigger is how a diagnostic becomes a fabrication, so the write
# needs --confirm-write and refuses without it.
plant_cmd "a §67 action without --confirm-write" "refusing to POST" "fabrication" 1 \
    "true" kf act document.create --actor t --acting-role r --organization o \
    --reason t --idempotency-key abcdefgh

# A scheme the client cannot serve is refused at construction, not inside the
# transport, where it would read as the service being unreachable. `https://` is
# NOT such a scheme any more — it was for one commit, until deny.toml gained a
# narrow exception for the CA bundle — so this plants the case that remains.
plant_cmd "a KF url with an unservable scheme" "not an http(s) URL" "inside the transport" 1 \
    "true" kf health --base ftp://kf.example.org

# §95 — a relation attached after the fact WITHOUT REVIEW is a fabrication.
#
# `UntrackedWork::attach_relation` refuses an empty reviewer and shipped in
# alpha; no command called it, so the refusal had never run outside a unit test.
# This is OW-WAR-0041's OBL-003 and OW-WAR-0049's OBL-004, which wanted the same
# verb.
#
# The danger is specific: a tool that attributed orphan commits to whichever
# Warrant was open at the time would turn a diagnostic into a fabrication, and it
# would look like tidying up.
plant_cmd "a relation attached with no reviewer" "no recorded reviewer" "fabricate a relationship" 1 \
    "true" telemetry --commit x --attach "commit abc123" --warrant OW-WAR-0041

# The positive control. Without it, a build that refused EVERY attachment would
# satisfy the plant above and look like a working review requirement.
plant_cmd "a relation attached with a reviewer" "related to" "reviewed by" 0 \
    "true" telemetry --commit x --attach "commit abc123" --warrant OW-WAR-0041 --reviewer QuiteTall

# §91.8 tests 52-58 — the agent-drafting seam (§74, §75.2).
#
# These plants feed COMMITTED FIXTURES to the shipped binary rather than mutating
# the corpus, so there is no sed to become a no-op — the input is the artefact.
# Each fixture is a Draft Proposal an agent could plausibly return.
#
# F=conformance/fixtures/proposals
plant_cmd "52: a malformed Draft Proposal" "did not parse" "key must be a string" 1 \
    "true" plan "t" --proposal conformance/fixtures/proposals/52-malformed.json --reviewed

# 53 and 54 are enforced by the SHAPE of the proposal, not by a check that looks
# for these fields. `DraftProposal` is the agent's entire output surface and now
# carries `deny_unknown_fields`, so a field it does not name cannot travel.
# Before that, both of these parsed and validated CLEAN — serde dropped them
# silently, and the agent had no way to learn that the identifier it believed it
# allocated went nowhere.
plant_cmd "53: an agent authorizing itself" "unknown field" "authorized_by" 1 \
    "true" plan "t" --proposal conformance/fixtures/proposals/53-self-authorized.json --reviewed

plant_cmd "54: an agent allocating an enterprise ID" "unknown field" "enterprise_id" 1 \
    "true" plan "t" --proposal conformance/fixtures/proposals/54-enterprise-id.json --reviewed

# An invented citation has the same SHAPE as a real one. Only resolution tells
# them apart, and until now nothing resolved them.
plant_cmd "55: an agent-invented source reference" "cannot resolve" "blocks rather than warns" 1 \
    "true" plan "t" --proposal conformance/fixtures/proposals/55-invented-ref.json --reviewed

plant_cmd "56: a durable choice with no ADR draft" "74.7" "bury the choice" 1 \
    "true" plan "t" --proposal conformance/fixtures/proposals/56-choice-no-adr.json --reviewed

# `require_blockers_answered` was written, correct, and called from nowhere: a
# proposal with an unanswered blocker-removing question reported itself
# APPLICABLE. The rule existed and the binary never asked it.
plant_cmd "58: an unanswered blocking question" "74.6" "MINIMUM set" 1 \
    "true" plan "t" --proposal conformance/fixtures/proposals/58-unanswered-blocker.json --reviewed

# §74.3 / OBL-003 — the model writes no file, and cannot ask to.
#
# `AtomOperation` is an enum of exactly §74.3's seven operations, so `write_file`
# is not a forbidden operation — it is an unrepresentable one, and the refusal
# names all seven alternatives. A vocabulary that cannot express the dangerous
# thing beats a check that looks for it.
plant_cmd "an agent asking to write a file" "unknown variant" "write_file" 1 \
    "true" plan "t" --proposal conformance/fixtures/proposals/74-3-write-file.json --reviewed

# §91.5 test 31 — the parent's generated view LISTS ITS CHILD.
#
# §20.4's child list is rendered from the corpus, not written by hand. Deleting
# the line for OW-WAR-0002 from OW-WAR-0001's committed view is what a parent
# silently losing a child looks like on disk.
#
# Asserted against `relations.child-listed`, the rule §20.4 has its own name for,
# NOT `generated.drift`. Both fire — any hand-edit to a projection drifts — and
# §92 requires the INTENDED control, because a rejection for the wrong reason
# proves nothing about the rule it was meant to exercise. This plant was written
# against drift first and the harness refused it.
plant "a child removed from its parent's view" "relations.child-listed" "smaller family" 2 \
    "sed -i '/OW-WAR-0002 (current)/d' docs/warrants/OW-WAR-0001/generated/WAR.md
     assert_gone 'OW-WAR-0002 (current)' docs/warrants/OW-WAR-0001/generated/WAR.md"

# §91.2 test 10 — a GENERATED atom cannot be edited through an authored-source
# command. OW-WAR-0005's OBL-002 claimed this test as in scope while the roadmap
# recorded it unimplemented and no implementation existed: a resolved Warrant
# claiming coverage it did not have, in the repository built to prevent that.
#
# The types shipped in alpha. `Jurisdiction::is_directly_editable` exists to
# answer "may I write this?", and `Jurisdiction::from_str` was referenced by one
# unit test — the declared jurisdiction reached the IR as a plain String and the
# class was never consulted about any atom.
plant "an own atom declared not directly editable" "atom.generated-as-source" "no longer a projection" 2 \
    "sed -i '0,/^jurisdiction: authored$/s//jurisdiction: generated/' docs/warrants/OW-WAR-0049/atoms/10-intent.md
     assert_present 'jurisdiction: generated' docs/warrants/OW-WAR-0049/atoms/10-intent.md"

plant "an atom jurisdiction outside §13.3" "atom.unknown-jurisdiction" "not one of" 2 \
    "sed -i '0,/^jurisdiction: authored$/s//jurisdiction: editable/' docs/warrants/OW-WAR-0049/atoms/10-intent.md
     assert_present 'jurisdiction: editable' docs/warrants/OW-WAR-0049/atoms/10-intent.md"

# §16.1 assigns the `adr` role to `bound`. The FIRST run of this rule caught a
# real one: all six ADR atoms declared `authored`, so a Warrant binding a
# decision claimed the right to rewrite it.
plant "an adr atom claiming it may be written here" "atom.jurisdiction-mismatch" "places under" 2 \
    "sed -i '0,/^jurisdiction: bound$/s//jurisdiction: authored/' docs/adr/atoms/OW-ADR-0001-canonical-json-implementation.md
     assert_present 'jurisdiction: authored' docs/adr/atoms/OW-ADR-0001-canonical-json-implementation.md"

# §49.3 — BLUT's lineage stays authoritative in BLUT. This is OBL-003's plant.
#
# `BlutLineageReceipt` shipped with a doc comment saying a copy would be wrong
# and NO validate() at all, and no command inspected a Warrant's bytes for one.
# The rule lived entirely in prose.
plant "lineage copied into a Warrant" "lineage.reproduced" "node_idx" 2 \
    "printf '\n- node_idx: 3\n- output_content_id: sha256:deadbeef\n' >> docs/warrants/OW-WAR-0047/atoms/60-assurance.md"

# A fenced block is where a REAL paste lands — someone copies BLUT output and
# wraps it in a code fence. Exempting fences to allow "what not to do" examples
# would put the hole exactly where the copies arrive, so they are matched like
# any other line. This plant pins that choice.
plant "lineage pasted inside a code fence" "lineage.reproduced" "output_content_id" 2 \
    "printf '\n\`\`\`yaml\noutput_content_id: sha256:deadbeef\n\`\`\`\n' >> docs/warrants/OW-WAR-0047/atoms/60-assurance.md"

# The negative control, and the one that matters most here.
#
# OW-ADR-0005 records a prose scan for \`gate://\` firing on OW-WAR-0019's own
# sentence explaining the rule. A Warrant MUST be able to say "this carries no
# node_idx" without that sentence counting as carrying one. If this plant ever
# fails, the detector went back to matching mentions instead of key positions.
# Deliberately NOT --generated: mutating an atom always drifts the committed
# projection, so a --generated run would fail for a reason unrelated to lineage
# and this control would pass while proving nothing about the detector.
plant "prose naming lineage fields is not a copy" "0 error" "worst: WARN" 0 \
    "printf '\nThis Warrant carries no node_idx and no output_content_id; BLUT owns them.\n' >> docs/warrants/OW-WAR-0047/atoms/60-assurance.md"

# §49.2 — `executor_args` is a JSON scalar the milestones grammar cannot check:
# openwarrant-core holds it as a raw string because parsing needs a JSON crate
# its production dependency surface deliberately excludes. So the check lives in
# the CLI, and these prove it RUNS rather than merely existing.
plant "executor_args that is not JSON" "milestones.bad-executor-args" "not JSON" 2 \
    "sed -i 's|executor_args: .{\"min_turns\": 2, \"drop_errors\": true}.|executor_args: \"{not json\"|' docs/warrants/OW-WAR-0047/atoms/45-milestones.yaml
     assert_present 'not json' docs/warrants/OW-WAR-0047/atoms/45-milestones.yaml"

# A JSON scalar that parses but is not an OBJECT. Lowered into `SpecNode.args`
# it would come back as a complaint about the stage, sending the author to read
# BLUT's source instead of their own line.
plant "executor_args that is not an object" "milestones.bad-executor-args" "must be a JSON object" 2 \
    "sed -i 's|executor_args: .{\"min_turns\": 2, \"drop_errors\": true}.|executor_args: \"[1, 2, 3]\"|' docs/warrants/OW-WAR-0047/atoms/45-milestones.yaml
     assert_present '[1, 2, 3]' docs/warrants/OW-WAR-0047/atoms/45-milestones.yaml"

# §49.2 — a stage NAME must resolve against the pinned registry, and a WAR stage
# id is not that name. Dropping `executor_ref` must refuse rather than fall back
# to the WAR id: lowering `STAGE-002` under its own id produces a PlanSpec naming
# a stage the author never chose, and BLUT then refuses it for a reason that
# looks like the pinned-registry rule working.
# The mutation VERIFIES ITSELF. A sed whose pattern stops matching after a
# formatting change deletes nothing, the corpus stays valid, and the plant then
# exercises the untouched happy path while reporting a pass — a silent probe
# read as a passing guard, which has cost this fleet twice.
#
# `assert_gone` makes that loud. Tested by pointing the sed at a pattern that
# cannot match: the battery stops with "PLANT MUTATION WAS A NO-OP" rather than
# scoring it. The exit-code assertion happens to catch this one too, but only
# because an unmutated corpus lowers cleanly; a plant whose unmutated state
# already failed for another reason would score green on a dead sed.
# -F: a fixed-string check, which is what the name promises. Without it a
# future caller passing a pattern containing `.` or `[` gets regex semantics
# and a guard that quietly matches the wrong thing.
plant_cmd "a blut stage bound to no executor stage" "blut.unbound-stage" "never chose" 2 \
    "sed -i '/executor_ref: \"materialize_dataset_path\"/d' docs/warrants/OW-WAR-0047/atoms/45-milestones.yaml
     assert_gone materialize_dataset_path docs/warrants/OW-WAR-0047/atoms/45-milestones.yaml" blut OW-WAR-0047

# §49.2 / §91.7 test 47 — an INCOMPATIBLE PORT KIND is refused, naming the port.
#
# This is the plant OBL-002 asks for, and until now it could not exist: the
# adapter hardcoded `compatible: true` on every mapping, so
# `BlutLowering::validate`'s incompatible-kind branch was unreachable from the
# shipped binary no matter what a Warrant declared. The rule was unit-tested and
# enforcing nothing — the same shape as the twenty alpha types that no command
# called.
plant_cmd "a port typed outside the §49.2 map" "incompatible kind" "STAGE-003.corpus" 1 \
    "sed -i 's|inputs: \[\"corpus:war/corpus\"\]|inputs: [\"corpus:war/not-a-kind\"]|' docs/warrants/OW-WAR-0047/atoms/45-milestones.yaml" blut OW-WAR-0047

# The mapped case must still pass, or the plant above would also fire for a
# correctly typed port and prove nothing about the kind check.
plant_cmd "a correctly typed port maps" "blut.ports-mapped" "3 port(s)" 0 \
    "true" blut OW-WAR-0047

# §46 / §51.3 — what happens when the NEIGHBOUR is the untrustworthy part.
#
# `war blut --verify` invokes a real BLUT binary and records its verdict as
# `authoritative_external`. That upgrade is only sound if OpenWarrant refuses a
# verdict it cannot actually attribute. The failure these five guard against is
# the one that looks like success: a --verify that silently reports PASS when
# the binary is missing, or that believes whatever JSON it is handed, is worse
# than no --verify at all, because it manufactures external evidence out of
# nothing while looking exactly like the real thing.
#
# The fake binaries stand in for a broken or hostile neighbour. They are testing
# OUR controls, not BLUT — a stand-in is never evidence ABOUT BLUT, and none of
# these plants claims to be.
FAKE="${TMPDIR:-/tmp}/war-plant-fake-blut-$$"

plant_cmd "a --verify naming a missing binary" "could not run" "refusal to guess" 1 \
    "true" blut OW-WAR-0047 --verify /nonexistent/blut-binary

plant_cmd "a neighbour accepting while exiting nonzero" "disagrees with its own exit status" "not recordable" 1 \
    "cat > \"$FAKE\" <<'SH'
#!/bin/sh
echo '{\"accepted\":true,\"fingerprint\":\"deadbeef\",\"recipes_registered\":9}'
exit 1
SH
chmod +x \"$FAKE\"" blut OW-WAR-0047 --verify "$FAKE"

plant_cmd "a neighbour printing no JSON at all" "did not print JSON" "stdout:" 1 \
    "cat > \"$FAKE\" <<'SH'
#!/bin/sh
echo 'accepted, trust me'
exit 0
SH
chmod +x \"$FAKE\"" blut OW-WAR-0047 --verify "$FAKE"

plant_cmd "a neighbour printing JSON with no verdict" "no \`accepted\` field" "no verdict to record" 1 \
    "cat > \"$FAKE\" <<'SH'
#!/bin/sh
echo '{\"recipes_registered\":3}'
exit 0
SH
chmod +x \"$FAKE\"" blut OW-WAR-0047 --verify "$FAKE"

# The refusal PATH itself: BLUT's own words must reach the report verbatim. A
# refusal summarised into "BLUT said no" would lose the stage name, which is the
# only part an author can act on.
plant_cmd "a neighbour refusing an unknown stage" "blut.refused" "is not in any registered cookbook" 2 \
    "cat > \"$FAKE\" <<'SH'
#!/bin/sh
echo '{\"accepted\":false,\"error\":\"PlanSpec does not typecheck: stage '\"'\"'STAGE-002'\"'\"' is not in any registered cookbook\",\"recipes_registered\":8}'
exit 1
SH
chmod +x \"$FAKE\"" blut OW-WAR-0047 --verify "$FAKE"

# A neighbour that never answers. The timeout is 30s by design, so running this
# on every gate would add 30s to every gate run — and a gate people skip because
# it is slow is a gate that does not run. Set WAR_PLANT_SLOW=1 to include it.
if [[ "${WAR_PLANT_SLOW:-0}" == "1" ]]; then
    plant_cmd "a neighbour that never answers" "did not answer within" "not a refusal" 1 \
        "cat > \"$FAKE\" <<'SH'
#!/bin/sh
sleep 3600
SH
chmod +x \"$FAKE\"" blut OW-WAR-0047 --verify "$FAKE"
else
    echo "skip  a neighbour that never answers      (WAR_PLANT_SLOW=1 to run; 30s timeout)"
fi

plant "milestone carrying a stage field" "milestones.invalid" "belongs to a stage" 2 \
    "python3 - <<'EOF'
import pathlib
p = pathlib.Path('docs/warrants/OW-WAR-0001/atoms/45-milestones.yaml')
s = p.read_text()
p.write_text(s.replace('  - id: \"M1\"\n    title:', '  - id: \"M1\"\n    executor_kind: \"human\"\n    title:', 1))
EOF"

# ── §20/§21 parent, child and supersession (OW-WAR-0043 OBL-004) ────────────
#
# §91.4 test 24 and §91.5 tests 30-35, plus §21.1's required `reason`.
#
# Each mutation is a script in conformance/plants/ rather than an inline
# one-liner. The inline version needed five levels of shell-inside-python-inside
# heredoc escaping and was unreadable AND wrong; a plant nobody can read is a
# plant nobody can check.
#
# P1 is OW-WAR-0001's uuid, read from the manifest rather than pasted, so these
# survive a re-mint.
P1="$(grep '^uuid' docs/warrants/OW-WAR-0001/manifest.toml | cut -d'"' -f2)"
PLANTS="python3 conformance/plants"

plant_cmd "child missing from parent view" "relations.child-listed" \
    "OW-WAR-0005" 2 "$PLANTS/31-child-missing-from-view.py" check

plant_cmd "child state in parent source" "relations.parent-source" \
    "OW-WAR-0002" 2 "$PLANTS/30-child-state-in-parent-source.py" check

plant_cmd "supersession without currency" "relations.currency" \
    "21.2" 2 "$PLANTS/33-supersession-without-currency.py $P1" check

plant_cmd "retired Warrant emptied" "relations.retired-available" \
    "21.4" 2 "$PLANTS/34-retired-warrant-emptied.py" check

plant_cmd "silent carry-forward" "relations.adoption" \
    "21.5" 2 "$PLANTS/35-silent-carry-forward.py $P1" check

# §21.1 makes `reason` part of the relation's SHAPE, so its absence is refused
# by the manifest parser. That IS the intended control: a required field beats a
# rule that has to remember to look.
# Exit 1, not 2: a manifest that does not PARSE is a diagnostic, and §76.2
# keeps that distinct from a Warrant that parsed and then failed a rule. The
# first version of this entry wanted 2 and also forgot to pass $P1, so the
# mutation script crashed and the plant ran against a clean tree — it reported
# "exit 0, wanted 2", which is the harness catching its own broken plant.
plant_cmd "supersession with no reason" "missing field" \
    "reason" 1 "$PLANTS/21-supersession-without-reason.py $P1" check

# POSITIVE (§91.4 test 24). "Does not require an ADR" is only demonstrable by
# the rule PASSING on an amendment that names no governing ADR. Expected exit 0.
plant_cmd "local choice needs no ADR" "amendment.valid" \
    "AM-999" 0 "$PLANTS/24-local-choice-needs-no-adr.py" check

# ── §96 import (OW-WAR-0043) ────────────────────────────────────────────────
#
# These mutate NOTHING tracked. Each builds its own scratch corpus under
# $MIGRATE_TMP and passes it with --corpus, so the guard and restore above stay
# irrelevant to them — a plant that needs the tree clean is a plant people learn
# to skip.
MIGRATE_TMP="$(mktemp -d)"
trap 'restore; rm -rf "$MIGRATE_TMP"' EXIT
mkdir -p "$MIGRATE_TMP/corpus"
cat > "$MIGRATE_TMP/corpus/0001-planted.md" <<'ADR'
---
status: accepted
---

## Decision

Something was decided.

## Completion / Resolution

- **verdict:** `passed`
ADR
mkdir -p "$MIGRATE_TMP/empty"
FROZEN="ba9ed833faa9a52940d5e9d424566466e9066867"

# OBL-001. A branch is not a frozen commit, and neither is an abbreviation.
plant_cmd "import at a branch name" "not a full 40-character" \
    "ONE NAMED, FROZEN commit" 1 ":" \
    migrate --corpus "$MIGRATE_TMP/corpus" --commit main --out "$MIGRATE_TMP/a.json"

plant_cmd "import at an abbreviated sha" "not a full 40-character" \
    "moving target" 1 ":" \
    migrate --corpus "$MIGRATE_TMP/corpus" --commit ba9ed83 --out "$MIGRATE_TMP/a.json"

# An empty import satisfies every count while importing nothing, so it is an
# error rather than a clean run of zero.
plant_cmd "import of an empty corpus" "contains no ADR files" \
    "importing nothing" 1 ":" \
    migrate --corpus "$MIGRATE_TMP/empty" --commit "$FROZEN" --out "$MIGRATE_TMP/a.json"

# OBL-003, the one the assurance atom names: a Complete line with no admissible
# evidence cannot be promoted, and the refusal is observable from OUTSIDE the
# binary rather than only in a unit test.
plant_cmd "promoting a legacy Complete line" "HISTORICAL CLAIM" \
    "0001-planted.md" 1 ":" \
    migrate --corpus "$MIGRATE_TMP/corpus" --commit "$FROZEN" \
    --out "$MIGRATE_TMP/a.json" --attempt-promotion

# OBL-001's other half: "a re-run at that SHA producing byte-identical output".
# Tamper the artifact and the verify must refuse.
plant_cmd "a tampered import artifact" "not reproducible" \
    "byte-identical output" 1 \
    "\"$WAR\" migrate --corpus \"$MIGRATE_TMP/corpus\" --commit \"$FROZEN\" --out \"$MIGRATE_TMP/t.json\" >/dev/null 2>&1; \
     python3 -c \"import pathlib,sys; p=pathlib.Path(sys.argv[1]); p.write_text(p.read_text().replace('\\\"adr_count\\\": 1','\\\"adr_count\\\": 2',1))\" \"$MIGRATE_TMP/t.json\"" \
    migrate --corpus "$MIGRATE_TMP/corpus" --commit "$FROZEN" --out "$MIGRATE_TMP/t.json" --verify

# §37.2 — a deliverable record that no longer describes its artifact.
#
# This control exists because it was needed twice in two days: an unrelated pull
# request edited `.github/workflows/ci.yml`, which OW-WAR-0001 declares, and the
# Warrant silently lost a §56.1 requirement while every check stayed green.
#
# The mutation tampers with the RECORD rather than the artifact, and that is not
# laziness — `restore` only covers docs/warrants/, docs/adr/, docs/gates/ and
# openwarrant.toml. A plant that edited `deny.toml` to move its bytes would not
# be undone by the restore and would leave the developer's tree modified. Same
# control either way: recorded digest versus bytes on disk.
DELIVERABLE="docs/warrants/OW-WAR-0001/deliverables.toml"

plant "a deliverable digest that no longer matches" "deliverable.digest-drift" \
    "regenerate the record" 2 \
    "python3 -c \"
import pathlib, re
p = pathlib.Path('$DELIVERABLE')
t = p.read_text()
p.write_text(re.sub(r'content_digest = \\\"sha256:[0-9a-f]{4}', 'content_digest = \\\"sha256:dead', t, count=1))
\"; assert_present 'sha256:dead' '$DELIVERABLE'"

# The other half: a record naming an artifact nobody produced. Distinct from
# drift, and reported distinctly — "the digest is wrong" and "there is nothing to
# hash" send a reader to different fixes.
plant "a deliverable naming a missing artifact" "deliverable.target-unreadable" \
    "not a verified one" 2 \
    "python3 -c \"
import pathlib
p = pathlib.Path('$DELIVERABLE')
p.write_text(p.read_text().replace('target_ref = \\\"Cargo.toml\\\"', 'target_ref = \\\"Cargo.toml.nope\\\"', 1))
\"; assert_present 'Cargo.toml.nope' '$DELIVERABLE'"

# OW-WAR-0055 — the corpus projection. §34 and §105 references, and the two
# generated files a person and an agent read.
#
# The manifest chosen for mutation is OW-WAR-0014, which carries one roadmap
# ref and one complete contribution and whose contract nothing else in this
# battery depends on.
MANIFEST_0014="docs/warrants/OW-WAR-0014/manifest.toml"

# OBL-001. Phase 11 does not exist in §98; a reference to it names nothing.
plant "a roadmap ref past the last phase" "roadmap.malformed" "0..=10" 2 \
    "sed -i 's|roadmap://OW-PHASE-1/rationale|roadmap://OW-PHASE-11/rationale|' $MANIFEST_0014; \
     assert_present 'OW-PHASE-11' '$MANIFEST_0014'"

# OBL-001, the other half of the grammar: an uppercase slug is a second spelling.
plant "a roadmap ref with an uppercase slug" "roadmap.malformed" "[a-z0-9-]" 2 \
    "sed -i 's|roadmap://OW-PHASE-1/rationale|roadmap://OW-PHASE-1/Rationale|' $MANIFEST_0014; \
     assert_present '/Rationale' '$MANIFEST_0014'"

# OBL-002. §34.2 names five contribution values; "mostly" is not one of them,
# and a projection deriving status from it would be deriving from a word.
# OW-WAR-0009, not 0014: 0014 declares no [[implements]] at all, and the first
# version of this plant targeted it — the no-op guard caught the sed matching
# nothing, which is the guard doing the one job it has.
MANIFEST_0009="docs/warrants/OW-WAR-0009/manifest.toml"
plant "a contribution outside the five" "traceability.contribution" "supersession" 2 \
    "sed -i '0,/contribution = \"complete\"/s||contribution = \"mostly\"|' $MANIFEST_0009; \
     assert_present 'mostly' '$MANIFEST_0009'"

# OBL-005. Both files a reader trusts, each tampered by one byte.
plant "Corpus Status Markdown edited by hand" "corpus-status.drift" "edited by hand" 2 \
    "sed -i 's|^# Corpus Status|# Corpus Status TAMPERED|' docs/warrants/generated/CORPUS_STATUS.md; \
     assert_present 'TAMPERED' docs/warrants/generated/CORPUS_STATUS.md" \
    --generated

plant "Corpus Status JSON edited by hand" "corpus-status.drift" "edited by hand" 2 \
    "sed -i 's|\"satisfied\":0|\"satisfied\":57|' docs/warrants/generated/CORPUS_STATUS.json; \
     assert_present '\"satisfied\":57' docs/warrants/generated/CORPUS_STATUS.json" \
    --generated

# OBL-006, positive: the projection on the UNMUTATED corpus names a stage to
# pick up, and puts a Warrant with no [[roadmap]] under `unassigned` rather than
# dropping it. `plant_cmd` with a no-op mutation and a wanted exit of 0 is the
# battery's way of asserting a thing is SAID, not only that a thing is refused.
plant_cmd "next actionable is never empty" "Next actionable" "STAGE-" 0 ":" status
plant_cmd "a Warrant with no roadmap is listed as unassigned" "unassigned" "OW-WAR-0050" 0 ":" status

# OBL-004. Two runs, byte-identical. A projection that differed between runs
# would drift-fail on every commit, so this is also what makes OBL-005 usable.
plant_cmd "the projection is byte-deterministic" "satisfied" "unaddressed" 0 \
    "\"$WAR\" status --json > \"$MIGRATE_TMP/status-a.json\" 2>/dev/null; \
     \"$WAR\" status --json > \"$MIGRATE_TMP/status-b.json\" 2>/dev/null; \
     cmp -s \"$MIGRATE_TMP/status-a.json\" \"$MIGRATE_TMP/status-b.json\" || { echo 'NOT DETERMINISTIC' >&2; exit 9; }" \
    status --json

# OW-WAR-0057 — the page. One byte, and the drift check names it.
plant "Corpus Status page edited by hand" "corpus-status.drift" "edited by hand" 2 \
    "sed -i 's|<h1>Corpus Status</h1>|<h1>Corpus Status TAMPERED</h1>|' docs/warrants/generated/CORPUS_STATUS.html; \
     assert_present 'TAMPERED' docs/warrants/generated/CORPUS_STATUS.html" \
    --generated

# OBL-001 of OW-WAR-0057, positive: the page is self-contained. No fetch, no
# script or stylesheet from any host. Asserted on the file the gate commits.
plant_cmd "the page reaches for nothing" "GENERATED BY OPENWARRANT" "# Corpus Status" 0 \
    "! grep -qE 'fetch\\(|<script src=|<link |src=\"http|url\\(' docs/warrants/generated/CORPUS_STATUS.html || { echo 'PAGE REACHES OUT' >&2; exit 9; }" \
    status

# OW-WAR-0056 — the Stage Dispatch compiler (§47).
#
# OW-WAR-0047's STAGE-002 is the one stage in this corpus that declares an
# `executor_ref`, so it is the positive case. The three refusals below are the
# shipped binary refusing on real files; byte-determinism and the omitted
# required atom are held by unit tests in `openwarrant-compiler`, because a
# dispatch minted by the CLI carries fresh UUIDv7 ids and cannot be compared
# across two runs — that is a property of the command, not a gap in the battery.

# OBL-004, positive: a §47.1 packet with its digest, on stdout and nothing else.
plant_cmd "a bound stage compiles to a dispatch" "oh.war/stage-dispatch/v1" "dispatch_digest" 0 ":" \
    dispatch OW-WAR-0047 STAGE-002

# An unbound stage is refused for the same reason `war blut` refuses it.
plant_cmd "an unbound stage is not dispatched" "declares no executor_ref" "Refused rather than dispatched" 1 ":" \
    dispatch OW-WAR-0021 STAGE-003

# OBL-005. A repair that cannot see what failed is a retry wearing a label.
plant_cmd "a repair with no prior failure evidence" "is a repair and carries no prior failure evidence" "52.3" 1 ":" \
    dispatch OW-WAR-0047 STAGE-002 --attempt-kind repair

# A stage id that names nothing is named back, with what does exist.
plant_cmd "a stage that does not exist" "no stage" "STAGE-001, STAGE-002, STAGE-003" 1 ":" \
    dispatch OW-WAR-0047 STAGE-099

# OW-WAR-0058 — the SAS as a controlled document (§101). `docs/sas/` joined
# the guard and the restore above the moment a plant learned to mutate it: the
# guard and the restore must name the same paths, or a plant silently edits the
# one document everything else is measured against and leaves it edited.

# OBL-003. One byte of the document, under an unchanged record.
plant "the SAS edited under an unchanged revision" "sas.digest-drift" "101.6" 2 \
    "sed -i 's|^## 106. Architecture requirements index|## 106. Architecture requirements index TAMPERED|' docs/sas/WAR_Software_Architecture_Specification.md; \
     assert_present 'index TAMPERED' docs/sas/WAR_Software_Architecture_Specification.md"

# OBL-004. A candidate with one §106 row deleted is refused, naming the id.
plant_cmd "a candidate SAS that drops a requirement id" "sas.diff.removed" "WAR-SAS-RQ-042" 2 \
    "grep -v '^| WAR-SAS-RQ-042 |' docs/sas/WAR_Software_Architecture_Specification.md > \"$MIGRATE_TMP/sas-minus-042.md\"; \
     grep -q 'WAR-SAS-RQ-041' \"$MIGRATE_TMP/sas-minus-042.md\" || exit 9" \
    sas diff "$MIGRATE_TMP/sas-minus-042.md"

# OBL-004, the other direction: an added row is reported and not refused.
plant_cmd "a candidate SAS that appends a requirement id" "sas.diff.added" "WAR-SAS-RQ-999" 0 \
    "sed 's/^| WAR-SAS-RQ-084 |.*$/&\\n| WAR-SAS-RQ-999 | A planted requirement |/' docs/sas/WAR_Software_Architecture_Specification.md > \"$MIGRATE_TMP/sas-plus-999.md\"; \
     grep -q 'WAR-SAS-RQ-999' \"$MIGRATE_TMP/sas-plus-999.md\" || exit 9" \
    sas diff "$MIGRATE_TMP/sas-plus-999.md"

# OBL-002. An acceptance signed by the agent that proposed it is refused, and
# the record stays proposed.
SAS_DIGEST="$(grep '^sha256' docs/sas/revisions/0.1.0-draft.1.toml | cut -d'"' -f2)"
plant_cmd "an agent accepting the SAS revision" "sas.unknown-actor\|sas.not-permitted" "claude" 2 \
    "printf 'schema = \"oh.war/sas-acceptance-response/v1\"\nversion = \"0.1.0-draft.1\"\nsha256 = \"%s\"\naccepted_by = \"claude\"\nacting_role = \"owner\"\nmeaning = \"x\"\neffective_time = \"2026-09-02T00:00:00Z\"\n' \"$SAS_DIGEST\" > \"$MIGRATE_TMP/agent-accept.toml\"" \
    sas accept 0.1.0-draft.1 --response "$MIGRATE_TMP/agent-accept.toml"

echo
echo "$PASSED passed, $FAILED failed"
[[ "$FAILED" -eq 0 ]]
