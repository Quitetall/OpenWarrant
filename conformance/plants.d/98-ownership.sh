# shellcheck shell=bash
# OW-WAR-0112 — ownership (OW-ADR-0021) and the dry run.
#
# Every plant here runs on a scratch program or reads this corpus without
# mutating it. The dry-run plants come first because they are what an agent
# reaches for before asking a human to sign; the ownership plants follow in
# M1 phase B, once the drift rule itself lands in check.rs.

PLANT_ROOT=$(scratch_corpus DR)
# Sourced outside plant.sh, `scratch_corpus` is undefined and PLANT_ROOT is
# empty, and every `git -C "$PLANT_ROOT"` below would then act on the real
# repository — it did once, committing a working tree as "plant". Refuse.
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
W1=DR-WAR-0001
# A scaffold ships the authority EXAMPLES only — nobody is eligible to sign,
# and a dry run rightly stops at `sign.who`. The plant needs an eligible
# human on the register, so the example becomes the register here. This is a
# throwaway program: the write-once rule for these files is about this
# repository's, not a scratch's.
cp "$PLANT_ROOT/docs/authority/roles.toml.example" "$PLANT_ROOT/docs/authority/roles.toml"
cp "$PLANT_ROOT/docs/authority/allowed_signers.example" "$PLANT_ROOT/docs/authority/allowed_signers"
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "register" >/dev/null 2>&1

# A fresh scaffold's adopt Warrant awaits authorization. A dry run must say
# it WOULD record, and must leave no trace: no response, no draft, no journal
# line, no attestation, and a tree byte-identical to before.
DR_BEFORE=$(git -C "$PLANT_ROOT" status --porcelain | sort)
# The example register names two humans, and `sign` rightly refuses to pick
# one; the plant picks, as an operator would.
DR_OUT=$("$WAR" --root "$PLANT_ROOT" sign "$W1" --dry-run --as your-name-here 2>&1)
DR_STATUS=$?
DR_AFTER=$(git -C "$PLANT_ROOT" status --porcelain | sort)
if [[ $DR_STATUS -eq 0 ]] && grep -q 'authorize.would-record' <<<"$DR_OUT" \
    && grep -q 'sign.would-record' <<<"$DR_OUT" \
    && [[ "$DR_BEFORE" == "$DR_AFTER" ]] \
    && [[ ! -d "$PLANT_ROOT/docs/authority/responses" || -z "$(ls -A "$PLANT_ROOT/docs/authority/responses" 2>/dev/null)" ]]; then
    printf 'ok    %-34s would record, wrote nothing\n' "dry run of an authorization"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s; tree moved: %s\n' "dry run of an authorization" "$DR_STATUS" \
        "$([[ "$DR_BEFORE" == "$DR_AFTER" ]] && echo no || echo yes)"
    FAILED=$((FAILED + 1))
fi

# The dry run must exercise the SAME refusals the real ingest does, and must
# never say an act was recorded. Over this corpus's whole queue: every act
# ends as `would-record` or `would-refuse` (or `needs-decision`, the sweep's
# own word for an act waiting on a flag), nothing ends as `<act>.recorded`,
# and a refusal is preceded by the ingest rule it came from. Not pinned to
# an alias: the queue moves, and a plant pinned to a moment in it breaks the
# night the owner signs (lib.sh, `scratch_warrant`).
DR2_BEFORE=$(git status --porcelain -- docs/ | sort)
DR2_OUT=$("$WAR" sign --all --dry-run 2>&1)
DR2_AFTER=$(git status --porcelain -- docs/ | sort)
DR2_JUDGED=$(grep -cE 'sign\.(would-record|would-refuse)' <<<"$DR2_OUT")
DR2_RECORDED=$(grep -cE '(authorize|resolution|correction)\.recorded|attest\.emitted' <<<"$DR2_OUT")
if [[ "$DR2_JUDGED" -gt 0 && "$DR2_RECORDED" -eq 0 ]] \
    && [[ "$DR2_BEFORE" == "$DR2_AFTER" ]] \
    && ! ls docs/authority/responses/*.draft.toml >/dev/null 2>&1; then
    printf 'ok    %-34s %s act(s) judged, none recorded, nothing written\n' "dry run names the real refusal" "$DR2_JUDGED"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s judged %s, recorded %s, tree moved: %s\n' "dry run names the real refusal" "$DR2_JUDGED" "$DR2_RECORDED" \
        "$([[ "$DR2_BEFORE" == "$DR2_AFTER" ]] && echo no || echo yes)"
    FAILED=$((FAILED + 1))
fi

# --all --dry-run over a real queue: many acts, still nothing written, no
# draft left in a temp directory either.
DR3_BEFORE=$(git status --porcelain | sort)
"$WAR" sign --all --dry-run >/dev/null 2>&1
DR3_AFTER=$(git status --porcelain | sort)
if [[ "$DR3_BEFORE" == "$DR3_AFTER" ]] && [[ -z "$(ls -d /tmp/war-dry-run-* 2>/dev/null)" ]]; then
    printf 'ok    %-34s a whole queue judged, nothing written\n' "dry run of every pending act"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s tree moved or a temp draft survived\n' "dry run of every pending act"
    FAILED=$((FAILED + 1))
fi

# A dry run holds no key: the code path must never reach ssh-keygen.
if ! grep -q 'ssh-keygen\|SSH_AUTH_SOCK' <<<"$DR_OUT$DR2_OUT"; then
    printf 'ok    %-34s no ssh in the transcript\n' "dry run reaches no key"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s ssh mentioned\n' "dry run reaches no key"
    FAILED=$((FAILED + 1))
fi

corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
