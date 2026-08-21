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
if ! git diff --quiet -- docs/warrants/ docs/adr/ docs/gates/ openwarrant.toml \
    || ! git diff --cached --quiet -- docs/warrants/ docs/adr/ docs/gates/ openwarrant.toml; then
    echo "docs/warrants/, docs/adr/, docs/gates/ or openwarrant.toml has uncommitted changes." >&2
    echo "Plants mutate those files and restore with 'git checkout', which would" >&2
    echo "discard your work. Commit or stash those first." >&2
    exit 1
fi

PASSED=0
FAILED=0

restore() {
    git checkout -- docs/warrants/ docs/adr/ docs/gates/ openwarrant.toml 2>/dev/null || true
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
