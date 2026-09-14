#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# The 1.0 wizard: the seven acts of OW-WAR-0067, walked one at a time.
#
# After mattpocock/skills `engineering/wizard` (MIT): a wizard exists for the
# steps only a human can perform. Every act here needs your ssh key or your
# judgement, so nothing in this file runs unattended and nothing here signs on
# your behalf: `war sign --ssh-sign` puts the dialog in front of you, and you
# are the one who clicks it.
#
#   bash scripts/release-1.0-wizard.sh           # walk the steps
#   bash scripts/release-1.0-wizard.sh --check   # preflight and state only, writes nothing
#   bash scripts/release-1.0-wizard.sh --from 4  # resume at step 4
#
# State is read from the tree, never from a file beside it, so a step already
# done is detected and offered as a skip. Interrupt at any point: the next run
# picks up where the records say you are.

set -uo pipefail

WAR=./target/debug/war
CHECK_ONLY=0
FROM=1
BRANCH_EXPECTED=feat/battery-split

while [[ $# -gt 0 ]]; do
    case "$1" in
        --check) CHECK_ONLY=1 ;;
        --from) FROM="${2:?--from needs a step number}"; shift ;;
        -h|--help) sed -n '3,20p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "unknown argument: $1" >&2; exit 2 ;;
    esac
    shift
done

bold() { printf '\033[1m%s\033[0m\n' "$*"; }
say() { printf '   %s\n' "$*"; }
die() { printf '\n!! %s\n' "$*" >&2; exit 1; }

# A step is offered, never assumed. `y` runs it, `s` skips it, `q` stops the
# wizard where it stands.
# `y` runs it, `s` skips it, `q` stops the wizard, and the raw answer is left
# in REPLY_WAS so a caller can offer its own letter (step 6 offers `d` for the
# diff) without every prompt having to know about it.
ask() {
    local prompt="$1"
    [[ $CHECK_ONLY -eq 1 ]] && { say "would ask: $prompt"; return 1; }
    local reply
    read -r -p "   $prompt [y/s/q] " reply
    REPLY_WAS="$reply"
    case "$reply" in
        y|Y) return 0 ;;
        q|Q) echo "   stopped."; exit 0 ;;
        *) return 1 ;;
    esac
}

run() {
    printf '   $ %s\n' "$*"
    [[ $CHECK_ONLY -eq 1 ]] && return 0
    "$@" || die "that command failed; fix it and re-run the wizard (nothing after this step ran)"
}

header() {
    echo
    bold "── step $1 of 8: $2"
}

clean_tree() {
    [[ -z "$(git status --porcelain)" ]]
}

require_clean() {
    clean_tree || die "the working tree is dirty; commit or stash first, then re-run"
}

# ── step 0: preflight ────────────────────────────────────────────────────────
bold "OpenWarrant 1.0 wizard — OW-WAR-0067's seven acts"
say "repository: $(pwd)"
say "branch:     $(git rev-parse --abbrev-ref HEAD)"
[[ -x "$WAR" ]] || { say "building the binary the steps use"; cargo build -q --workspace || die "cargo build failed"; }
say "war:        $($WAR --version)"

[[ "$(git rev-parse --abbrev-ref HEAD)" == "$BRANCH_EXPECTED" ]] \
    || say "NOTE: expected $BRANCH_EXPECTED; continuing on $(git rev-parse --abbrev-ref HEAD)"

KEY_FP=$(awk '{ for (i = 1; i <= NF; i++) if ($i ~ /^ssh-/) { print $i " " $(i + 1); exit } }' docs/authority/allowed_signers 2>/dev/null)
if [[ -n "$KEY_FP" ]] && ssh-add -L 2>/dev/null | grep -qF "$(cut -d' ' -f2 <<< "$KEY_FP")"; then
    say "key:        loaded in the agent (the confirm dialog will appear per signature)"
else
    say "key:        NOT loaded. Run: ssh-add -c ~/.ssh/id_ed25519"
    say "            Every signing step below will fail until it is."
fi

echo
bold "state, from the records"
say "pending acts:   $($WAR sign --list 2>/dev/null | head -1)"
say "open questions: $($WAR questions --open 2>/dev/null | head -1)"
say "licence:        $(grep -m1 '^license' Cargo.toml | cut -d'"' -f2)"
say "dsse namespace: $(grep -q 'oh.war/dsse' docs/authority/allowed_signers && echo present || echo absent)"
say "SAS 1.0.0:      $(grep -m1 '^state' docs/sas/revisions/1.0.0.toml 2>/dev/null | cut -d'"' -f2 || echo 'no record')"
[[ $CHECK_ONLY -eq 1 ]] && { echo; say "--check: nothing was written."; exit 0; }

# ── step 1: relicense ────────────────────────────────────────────────────────
step_relicense() {
    header 1 "relicense to Apache-2.0 (RELICENSING.md steps 1 to 6)"
    if grep -q '^license = "Apache-2.0"' Cargo.toml; then
        if clean_tree; then
            say "already done: the manifest says Apache-2.0 and the tree is committed."
            return 0
        fi
        say "PARTLY done: the manifest says Apache-2.0 but the tree is not committed."
        say "A relicense is the bytes AND the records that pin them, so the rest"
        say "of this step (re-pin, compile, gate, commit) still has to run."
        ask "finish it?" || { say "skipped."; return 0; }
        relicense_finish
        return 0
    fi
    require_clean
    say "Preconditions, both checked now rather than assumed:"
    run cargo deny check licenses
    say "contributors: $(git log --format='%an' | sort -u | tr '\n' ' ')"
    say ""
    say "This rewrites LICENSE, adds NOTICE, flips the manifest, rewrites the"
    say "SPDX header in every file that is NOT pinned by a resolved Warrant,"
    say "lists the ones that are in RELICENSING-PENDING.txt (their header moves"
    say "with their correction), and writes OW-ADR-0017."
    ask "run the relicense?" || { say "skipped."; return 0; }

    local apache
    apache=$(ls ~/.cargo/registry/src/*/serde-1.0.0/LICENSE-APACHE 2>/dev/null | head -1)
    [[ -s "$apache" ]] || die "no Apache-2.0 text found in the cargo registry; put one at /tmp/LICENSE-APACHE and re-run"
    cp "$apache" LICENSE
    cat > NOTICE <<'NOTICE_EOF'
OpenWarrant
Copyright 2026 Brian Lam

Licensed under the Apache License, Version 2.0 (the "License"); you may not
use this work except in compliance with the License. You may obtain a copy of
the License at http://www.apache.org/licenses/LICENSE-2.0.

Versions distributed before the relicense (OW-ADR-0017, 2026-09-12) remain
available under the GNU Affero General Public License v3.0 or later; the
relicense does not withdraw them.
NOTICE_EOF

    # The files behind the wall that are clean today: their header waits for
    # the correction that frees them, so the ratchet in `cargo xtask gate`
    # tolerates the old identifier for exactly these.
    "$WAR" pins --resolved-only 2>/dev/null | awk '{ print $4 }' | sort -u > /tmp/war-pinned.txt
    "$WAR" check 2>/dev/null | grep 'deliverable.digest-drift' | grep -oE 'for [^ ]+' | awk '{ print $2 }' | sort -u > /tmp/war-drifting.txt
    grep -rl 'SPDX-License-Identifier: Apache-2.0' crates xtask 2>/dev/null | sort > /tmp/war-spdx.txt
    comm -12 /tmp/war-spdx.txt /tmp/war-pinned.txt | comm -23 - /tmp/war-drifting.txt > RELICENSING-PENDING.txt
    say "$(wc -l < RELICENSING-PENDING.txt) file(s) stay AGPL until their correction is signed"

    local flipped=0 f
    while IFS= read -r f; do
        grep -qxF "$f" RELICENSING-PENDING.txt && continue
        sed -i 's|SPDX-License-Identifier: Apache-2.0|SPDX-License-Identifier: Apache-2.0|' "$f"
        flipped=$((flipped + 1))
    done < <(grep -rl 'SPDX-License-Identifier: Apache-2.0' crates xtask conformance evals scripts .github docs schemas 2>/dev/null)
    say "$flipped header(s) rewritten"

    python3 - <<'PY'
import pathlib, re
c = pathlib.Path("Cargo.toml"); s = c.read_text()
s = re.sub(r'^license = "AGPL-3\.0-or-later"$', 'license = "Apache-2.0"', s, count=1, flags=re.M)
c.write_text(s)
d = pathlib.Path("deny.toml"); t = d.read_text()
t = re.sub(r'\[\[licenses\.exceptions\]\]\nname = "openwarrant-core"\nallow = \["AGPL-3\.0-or-later"\]\n\n?', '', t)
d.write_text(t)
for path, old, new in [
    ("README.md",
     "**AGPL-3.0-or-later** today; **Apache-2.0** intended when this goes public.",
     "**Apache-2.0** (relicensed 2026-09-12, OW-ADR-0017). Versions distributed before that date remain available under AGPL-3.0-or-later."),
    ("CONTRIBUTING.md",
     "OpenWarrant ships **AGPL-3.0-or-later** today and is intended to be relicensed",
     "OpenWarrant ships **Apache-2.0** (relicensed 2026-09-12, OW-ADR-0017); before that date it shipped AGPL-3.0-or-later and was relicensed"),
]:
    p = pathlib.Path(path)
    if p.exists():
        text = p.read_text()
        if old in text:
            p.write_text(text.replace(old, new, 1))
PY

    cat > docs/adr/atoms/OW-ADR-0017-relicense-apache-2-0.md <<'ADR_EOF'
---
schema: oh.war/atom/v1
adr_uuid: 7a3d9c1e-5b28-4f0a-9e64-2c8b1d7f3a90
local_alias: OW-ADR-0017
role: adr
jurisdiction: bound
order: 30
classification: internal
status: accepted
governs:
  - "war://01a021a2-b570-7f57-85b2-0f8189873d9e"
---

# ADR OW-0017: relicense from AGPL-3.0-or-later to Apache-2.0

## Status

Accepted 2026-09-12 by the repository owner, who ran the relicense through
`scripts/release-1.0-wizard.sh` at the workstation.

## Context

RELICENSING.md set two preconditions and kept them true from the first commit:
every dependency permissive, gated by `cargo deny check licenses` on every gate
run, and the copyright ours to relicense (one author, with contribution terms
written down ahead of any outside contribution). Apache-2.0 carries an express
patent grant, which is worth more to a protocol implementation others
interoperate with than MIT's brevity. The 1.0 release publishes four crates,
and a version published under AGPL-3.0-or-later could never be relicensed
afterwards, so the flip precedes the tag.

## Decision

`license = "Apache-2.0"`, the Apache-2.0 text as LICENSE, a NOTICE, the
`SPDX-License-Identifier` header rewritten in every file the wall allows, the
`deny.toml` exception for our own licence removed. Versions distributed before
2026-09-12 remain available under AGPL-3.0-or-later; nothing is withdrawn.

A file pinned by a RESOLVED Warrant keeps the old identifier until the
correction that frees it, and is listed in RELICENSING-PENDING.txt meanwhile.
`cargo xtask gate` ratchets on that list: the old identifier passes only for a
file it names, so a half-applied relicense is visible rather than silent.

## Consequences

- `release.yml`'s crates job, which refuses any other licence, can publish.
- Each listed file costs one correction, signed with its own reason.
- CONTRIBUTING.md's dual-licence contribution term is moot for new
  contributions and stays as history.
ADR_EOF

    relicense_finish
}

# The mechanical tail, also reachable when a previous run died part-way: the
# re-pin comes BEFORE the gate, because the gate's corpus step reads the pins
# the header rewrite just invalidated.
relicense_finish() {
    run cargo fmt --all
    run cargo build --workspace
    say "re-pinning every unresolved Warrant on the new bytes"
    python3 - <<'PY'
import hashlib, pathlib, re, subprocess
out = subprocess.run(["./target/debug/war", "check"], capture_output=True, text=True).stdout
drift = re.findall(r'^ERROR deliverable\.digest-drift\s+(OW-WAR-\d+): (D-\d+) records \S+ for (\S+) but', out, re.M)
n = 0
for alias, did, target in drift:
    d = pathlib.Path("docs/warrants") / alias
    if (d / "resolution.toml").exists():
        continue
    p = d / "deliverables.toml"
    blocks = re.split(r'(?=^\[\[deliverable\]\])', p.read_text(), flags=re.M)
    out_blocks = []
    for b in blocks:
        if re.search(rf'^id = "{did}"', b, re.M):
            dg = "sha256:" + hashlib.sha256(pathlib.Path(target).read_bytes()).hexdigest()
            b, k = re.subn(r'^content_digest = "sha256:[0-9a-f]+"', f'content_digest = "{dg}"', b, flags=re.M)
            n += k
        out_blocks.append(b)
    p.write_text("".join(out_blocks))
print(f"   {n} unresolved pin(s) moved")
PY
    run "$WAR" compile
    run cargo xtask gate
    run git add -A
    git commit -q -F - <<'MSG_EOF'
release: relicense to Apache-2.0 (OW-ADR-0017, OW-WAR-0067 STAGE-001)

RELICENSING.md steps 1 to 6, run by the owner through
scripts/release-1.0-wizard.sh: `cargo deny check licenses` green, a single
author, the Apache-2.0 text as LICENSE, a NOTICE, `license = "Apache-2.0"`,
the SPDX header rewritten in every file the wall allows, the deny.toml
exception for our own licence removed, OW-ADR-0017 recorded.

Files pinned by a resolved Warrant keep the old identifier until the
correction that frees them and are listed in RELICENSING-PENDING.txt; `cargo
xtask gate` ratchets on that list, so a half-applied relicense is visible.

Versions distributed before today remain available under AGPL-3.0-or-later.
MSG_EOF
    say "committed."
}

# ── step 2: the attestation namespace ────────────────────────────────────────
step_namespace() {
    header 2 "add oh.war/dsse to your namespaces in allowed_signers"
    if grep -q 'oh.war/dsse' docs/authority/allowed_signers; then
        say "already done."
        return 0
    fi
    say "allowed_signers is human-written: an agent never edits it. This is the"
    say "one line that lets your DSSE attestations verify (OW-ADR-0015)."
    say ""
    say '   sed -i '"'"'s|namespaces="oh.war/response"|namespaces="oh.war/response,oh.war/dsse"|'"'"' docs/authority/allowed_signers'
    ask "run that edit now?" || { say "do it by hand, then re-run with --from 3"; return 0; }
    sed -i 's|namespaces="oh.war/response"|namespaces="oh.war/response,oh.war/dsse"|' docs/authority/allowed_signers
    grep -q 'oh.war/dsse' docs/authority/allowed_signers || die "the edit did not apply; check the file by hand"
    run git add docs/authority/allowed_signers
    git commit -q -m "authority: the signing key may sign in the oh.war/dsse namespace (OW-WAR-0067 STAGE-002)

A human edit to a human-written register: the namespace every DSSE
attestation is signed under (OW-ADR-0015). Without it \`war attest verify\`
rejects a signature the key really made."
    say "committed."
}

# ── step 3: the SAS ──────────────────────────────────────────────────────────
step_sas() {
    header 3 "accept SAS 1.0.0 (the last act before the tag can mean anything)"
    if grep -q '^state = "accepted"' docs/sas/revisions/1.0.0.toml 2>/dev/null; then
        say "already accepted."
        return 0
    fi
    say "One dialog. The revision folds every SAS edit of the 1.0 plan and adds"
    say "four §106 rows, so §101.3 requires the ADR flag."
    ask "sign it?" || { say "skipped."; return 0; }
    run "$WAR" sign 1.0.0 --ssh-sign --adr OW-ADR-0016   # a path also works
    run "$WAR" compile
    run git add -A
    git commit -q -m "sas: 1.0.0 accepted (OW-WAR-0067 STAGE-003)

Signed by the owner with the ssh key, with OW-ADR-0016 as §101.3 requires.
The accepted revision is normative from here (§101.6)."
}

# ── step 4: the three authorizations ─────────────────────────────────────────
step_authorize() {
    header 4 "authorize the Warrants awaiting a first signature"
    local pending
    pending=$("$WAR" sign --list 2>/dev/null | awk '$2 == "rev" && $4 == "authorize" { print $1 }')
    if [[ -z "$pending" ]]; then
        say "nothing awaits an authorization."
        return 0
    fi
    say "awaiting: $(tr '\n' ' ' <<< "$pending")"
    say "Each is one dialog. 0064 is the correction act itself, so sign it first:"
    say "the corrections in step 6 are acts under it."
    say ""
    say "A revision above 1 needs an amendment record first (§31): the tool"
    say "refuses the signature with authorize.no-amendment and names what moved."
    say "If that happens, stop and have the amendment drafted; the signature"
    say "adopts it, and an amendment nobody wrote is not one you can sign."
    local a
    for a in $pending; do
        ask "authorize $a?" || { say "skipped."; continue; }
        run "$WAR" sign "$a" --ssh-sign
    done
    run "$WAR" compile
    if ! clean_tree; then
        run git add -A
        git commit -q -m "authorize: the Warrants awaiting a first signature (OW-WAR-0067 STAGE-004)

Each signed by the owner with the ssh key. An authorization binds one contract
revision and never changes afterwards (§28.4)."
    fi
}

# ── step 5: OW-WAR-0042's recorded draft ─────────────────────────────────────
step_draft() {
    header 5 "review and apply OW-WAR-0042's recorded draft"
    local proposal=docs/warrants/OW-WAR-0042/evidence/run-1/proposal.json
    [[ -s "$proposal" ]] || { say "no recorded proposal at $proposal"; return 0; }
    say "The drafter was a real separate process (claude -p, no tools, 57 s)."
    say "§74.4 steps 5 and 6 are your review; nothing applied it for you."
    say ""
    python3 - "$proposal" <<'PY'
import json, sys
p = json.load(open(sys.argv[1]))
print("   title:", p["proposed_identity"]["title"])
for op in p["operations"]:
    what = op.get("role") or op.get("relation_kind") or op.get("adr_title", "")
    print(f"   - {op['op']}: {what}")
print("   risk:", p.get("risk_assessment", "")[:120])
PY
    say ""
    say "Read it in full before answering: $proposal"
    say "A reviewer will notice it proposes \`war inbox\` for what \`war next\` and"
    say "\`war sign --list\` already do. Applying it is still honest: the Warrant"
    say "records what the agent proposed, and the review is what caught it."
    ask "apply it (writes a new Warrant directory)?" || { say "skipped."; return 0; }
    run "$WAR" plan --proposal "$proposal" --apply --reviewed
    run "$WAR" compile
    run git add -A
    git commit -q -m "plan: apply OW-WAR-0042's recorded draft after review (OW-WAR-0067 STAGE-005)

§74.4 steps 5 to 8: the owner reviewed the proposal the recorded drafter run
produced and applied it. The Warrant it creates is the agent's proposal, not
the reviewer's rewrite."
    say ""
    say "Next for 0042, when you want it closed: evidence, a BLIND verification"
    say "(a verifier that sees only the bundle), then the resolution."
    say "   $WAR evidence record OW-WAR-0042"
    say "   $WAR verify OW-WAR-0042 --performer claude --bundle"
    say "   # hand the bundle to an independent verifier, then:"
    say "   $WAR verify OW-WAR-0042 --response <file>"
    say "   $WAR resolve --dry-run OW-WAR-0042 && $WAR sign OW-WAR-0042 --ssh-sign"
}

# ── step 6: the corrections ──────────────────────────────────────────────────
# One kind for the batch, then a dialog each. The reason is drafted by the tool
# from the commits that touched the file since the Warrant resolved, and shown
# on the screen before the prompt, so signing twenty-six corrections costs
# twenty-six confirmations and no typing. Type a sentence only where you want
# to say more than the record already does.
step_corrections() {
    header 6 "sign every correction, on final bytes"
    local rows count
    rows=$("$WAR" sign --list 2>/dev/null | awk '$2 == "correct" { print $1 }')
    if [[ -z "$rows" ]]; then
        say "no deliverable of a resolved Warrant is drifting."
        return 0
    fi
    count=$(wc -l <<< "$rows")
    say "$count correction(s) await you."
    say ""
    say "The kind is the one judgement the tree cannot make for you:"
    say "  1  behaviour-change  the file now does something different"
    say "  2  added-refusal     it now refuses something it used to allow"
    say ""
    say "The reason is drafted from the commits that touched each file since its"
    say "Warrant resolved, and shown before each prompt. Signing last matters: a"
    say "correction binds the bytes as they are now."
    require_clean

    local kind meaning
    while true; do
        read -r -p "   kind for this batch [1/2, or the name] " kind
        case "$kind" in
            1|behaviour-change|behavior-change) kind=behaviour-change; break ;;
            2|added-refusal) kind=added-refusal; break ;;
            q|Q) echo "   stopped."; exit 0 ;;
            *) say "   1 or 2, or the name." ;;
        esac
    done
    read -r -p "   one sentence to add to every reason (Enter for none) " meaning
    local -a args=(--ssh-sign --kind "$kind")
    [[ -n "$meaning" ]] && args+=(--meaning "$meaning")

    local target signed=0 skipped=0
    for target in $rows; do
        echo
        "$WAR" sign "$target" --show "${args[@]:1}" 2>&1 | sed -n '1,14p' | sed 's/^/      /'
        if ! ask "sign $target?"; then
            say "skipped."
            skipped=$((skipped + 1))
            continue
        fi
        if "$WAR" sign "$target" "${args[@]}"; then
            signed=$((signed + 1))
        else
            say "   refused; it stays in the queue. Continuing."
            skipped=$((skipped + 1))
        fi
    done
    echo
    say "$signed signed, $skipped left in the queue."
    run "$WAR" compile
    if ! clean_tree; then
        run git add -A
        git commit -q -m "correct: the drifted deliverables of resolved Warrants (OW-WAR-0067 STAGE-006)

Each correction signed by the owner with its kind, and a reason the tool
drafted from the commits that touched the file since that Warrant resolved
(§34.4, OW-WAR-0064): the superseded digest stays on record and the delivered
artifact moves for a stated reason, never silently."
    fi
    say ""
    say "A file freed by a correction can take the new SPDX header: remove its"
    say "line from RELICENSING-PENDING.txt, rewrite the header, and it wants one"
    say "more correction. One later pass is cheaper than one per file."
}

# ── step 7: the gate ─────────────────────────────────────────────────────────
step_gate() {
    header 7 "the whole gate, before the tag"
    say "This is what the release workflow runs. A tag on a red gate is a"
    say "release nobody can reproduce."
    ask "run cargo xtask gate?" || { say "skipped."; return 0; }
    run cargo xtask gate
    say "green."
}

# ── step 8: the tag ──────────────────────────────────────────────────────────
step_tag() {
    header 8 "tag v1.0.0 and push"
    if git rev-parse -q --verify refs/tags/v1.0.0 >/dev/null; then
        say "v1.0.0 already exists."
        return 0
    fi
    grep -q '^license = "Apache-2.0"' Cargo.toml || die "step 1 has not run: the crates job would refuse to publish"
    grep -q '^state = "accepted"' docs/sas/revisions/1.0.0.toml 2>/dev/null || die "step 3 has not run: SAS 1.0.0 is not accepted"
    require_clean
    say "The tag starts release.yml: the gate, both hosts, IR parity, the"
    say "release, then crates.io in dependency order. A published version's"
    say "licence is permanent."
    ask "tag and push?" || { say "skipped."; return 0; }
    run git tag -a v1.0.0 -m "OpenWarrant 1.0.0

The protocol is stable from here: every oh.war/*/v1 record shape, every
DigestDomain string, the RFC 8785 canonical form and the schema pack are
frozen. See docs/COMPATIBILITY.md and docs/roadmap/RELEASE_1_0.md."
    run git push origin "$(git rev-parse --abbrev-ref HEAD)"
    run git push origin v1.0.0
    say "pushed. Watch the release workflow; the crates job publishes last."
}

steps=(step_relicense step_namespace step_sas step_authorize step_draft step_corrections step_gate step_tag)
for i in "${!steps[@]}"; do
    n=$((i + 1))
    [[ $n -lt $FROM ]] && continue
    "${steps[$i]}"
done

echo
bold "where you are now"
say "$($WAR sign --list 2>/dev/null | head -1)"
say "$($WAR questions --open 2>/dev/null | head -1)"
say "battery: bash conformance/plant.sh"
