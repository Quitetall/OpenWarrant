# shellcheck shell=bash
# OW-WAR-0124 — brownfield adoption: `war init` in a repository with history.
#
# Every repository here is a scratch under one mktemp directory, removed at
# the end; nothing in this corpus is touched. Each claim is paired with the
# refusal that shows the control is not vacuous, and each refusal is matched
# on WHAT refused it, not merely on a non-zero exit.

echo "== brownfield adoption (OW-WAR-0124) =="
AD_TMP=$(mktemp -d)
AD_WAR="$REPO_ROOT/${WAR#./}"

ad_ok() {
    printf 'ok    %-34s %s\n' "$1" "$2"
    PASSED=$((PASSED + 1))
}
ad_fail() {
    printf 'FAIL  %-34s %s\n' "$1" "$2"
    FAILED=$((FAILED + 1))
}
# Law 15: a sub-claim this build cannot ask is neither a pass nor a failure.
ad_unknown() {
    printf 'UNKNOWN %-32s %s\n' "$1" "$2"
}
ad_git() { git -C "$1" -c user.email=plant@invalid -c user.name=plant "${@:2}"; }
# ad_commit <repo> <subject> [file]: one commit, adding a file when named.
ad_commit() {
    local repo="$1" subject="$2" file="${3:-}"
    if [[ -n "$file" ]]; then
        mkdir -p "$(dirname "$repo/$file")"
        printf '%s\n' "$subject" >"$repo/$file"
        ad_git "$repo" add -- "$file" >/dev/null
        ad_git "$repo" commit -qm "$subject" >/dev/null
    else
        ad_git "$repo" commit -q --allow-empty -m "$subject" >/dev/null
    fi
}
# ad_history <repo>: a repository with three commits and three files.
ad_history() {
    mkdir -p "$1" && git -C "$1" init -q . || {
        printf 'PLANT SETUP FAILED: git init in %s\n' "$1" >&2
        exit 9
    }
    ad_commit "$1" "first" src/a.txt
    ad_commit "$1" "second" src/b.txt
    ad_commit "$1" "third" src/c.txt
}
# ad_candidates <telemetry output>: the §95 count the report states.
ad_candidates() { grep -oE '[0-9]+ §95 untracked-work candidate' <<<"$1" | grep -oE '^[0-9]+'; }

# --- OBL-001: the baseline is recorded, and a non-commit is refused ----------

AD_A="$AD_TMP/acme"
ad_history "$AD_A"
AD_HEAD=$(git -C "$AD_A" rev-parse HEAD)
AD_OUT=$(cd "$AD_A" && "$AD_WAR" init --program Acme --namespace ACME 2>&1)
AD_STATUS=$?
AD_OUT_ACME="$AD_OUT"
AD_BASE=$(grep -E '^baseline = ' "$AD_A/openwarrant.toml" 2>/dev/null | sed -E 's/^baseline = "(.*)"$/\1/')
if [[ $AD_STATUS -eq 0 ]] && grep -q '^\[adoption\]$' "$AD_A/openwarrant.toml" \
    && [[ "$AD_BASE" == "$AD_HEAD" ]] && grep -Fq "adoption baseline $AD_HEAD" <<<"$AD_OUT"; then
    ad_ok "init records HEAD as the baseline" "$AD_HEAD"
else
    ad_fail "init records HEAD as the baseline" "exit $AD_STATUS; recorded ${AD_BASE:-nothing}; $(tr '\n' '|' <<<"$AD_OUT")"
fi
AD_CHECK=$("$AD_WAR" --root "$AD_A" check 2>&1)
if [[ $? -eq 0 ]]; then
    ad_ok "the adopted scaffold passes check" "exit 0"
else
    ad_fail "the adopted scaffold passes check" "$(grep -E '^(ERROR|UNKNOWN)' <<<"$AD_CHECK" | head -3)"
fi

# The refusal: `--baseline 0000000` names no commit. Refused by the baseline
# rule, before anything is written.
AD_B="$AD_TMP/refused"
ad_history "$AD_B"
AD_OUT=$(cd "$AD_B" && "$AD_WAR" init --program Acme --namespace ACME --baseline 0000000 2>&1)
AD_STATUS=$?
if [[ $AD_STATUS -ne 0 ]] && grep -Fq -- '--baseline "0000000" is refused' <<<"$AD_OUT" \
    && grep -Fq 'does not name a commit' <<<"$AD_OUT" \
    && [[ ! -e "$AD_B/openwarrant.toml" ]] && [[ ! -e "$AD_B/docs" ]]; then
    ad_ok "--baseline 0000000 is refused" "exit $AD_STATUS, nothing written"
else
    ad_fail "--baseline 0000000 is refused" "exit $AD_STATUS; $(ls "$AD_B"); $(tr '\n' '|' <<<"$AD_OUT")"
fi

# A real commit outside HEAD's history is refused too, by the ancestry rule.
git -C "$AD_B" checkout -q -b side
ad_commit "$AD_B" "side work" src/side.txt
AD_SIDE=$(git -C "$AD_B" rev-parse HEAD)
git -C "$AD_B" checkout -q -
AD_OUT=$(cd "$AD_B" && "$AD_WAR" init --namespace ACME --baseline "$AD_SIDE" 2>&1)
AD_STATUS=$?
if [[ $AD_STATUS -ne 0 ]] && grep -Fq "is not in HEAD's history" <<<"$AD_OUT" \
    && [[ ! -e "$AD_B/openwarrant.toml" ]]; then
    ad_ok "a commit off HEAD's history refused" "exit $AD_STATUS, nothing written"
else
    ad_fail "a commit off HEAD's history refused" "exit $AD_STATUS; $(tr '\n' '|' <<<"$AD_OUT")"
fi

# And a named commit that IS in the history is recorded as its full id.
AD_WANT=$(git -C "$AD_B" rev-parse HEAD~1)
AD_OUT=$(cd "$AD_B" && "$AD_WAR" init --namespace ACME --baseline HEAD~1 2>&1)
if [[ $? -eq 0 ]] && grep -Fxq "baseline = \"$AD_WANT\"" "$AD_B/openwarrant.toml"; then
    ad_ok "--baseline HEAD~1 records its full id" "$AD_WANT"
else
    ad_fail "--baseline HEAD~1 records its full id" "$(tr '\n' '|' <<<"$AD_OUT")"
fi

# No commits: no [adoption] table, and the three lines it always printed.
AD_C="$AD_TMP/fresh"
mkdir -p "$AD_C" && git -C "$AD_C" init -q .
AD_OUT=$(cd "$AD_C" && "$AD_WAR" init --program Acme --namespace ACME 2>&1)
AD_STATUS=$?
if [[ $AD_STATUS -eq 0 ]] && ! grep -q 'adoption' "$AD_C/openwarrant.toml" \
    && [[ $(wc -l <<<"$AD_OUT") -eq 3 ]] && ! grep -q 'baseline' <<<"$AD_OUT"; then
    ad_ok "no commits, no [adoption] table" "three lines, as before"
else
    ad_fail "no commits, no [adoption] table" "exit $AD_STATUS; $(tr '\n' '|' <<<"$AD_OUT")"
fi

# --- OBL-002: untracked work counts from the baseline, in ACME ---------------

ad_telemetry() { "$AD_WAR" --root "$1" telemetry --commit plant --out "$AD_TMP/telemetry.json" 2>&1; }
AD_T=$(ad_telemetry "$AD_A")
if [[ "$(ad_candidates "$AD_T")" == 0 ]] && grep -Fq "$AD_HEAD..HEAD" <<<"$AD_T" \
    && grep -Fq "\"adoption_baseline\": \"$AD_HEAD\"" "$AD_TMP/telemetry.json"; then
    ad_ok "0 candidates right after init" "read from $AD_HEAD..HEAD"
else
    ad_fail "0 candidates right after init" "$(tr '\n' '|' <<<"$AD_T")"
fi
ad_git "$AD_A" add -A >/dev/null && ad_git "$AD_A" commit -qm "adopt openwarrant" >/dev/null
AD_T=$(ad_telemetry "$AD_A")
if [[ "$(ad_candidates "$AD_T")" == 1 ]]; then
    ad_ok "one uncited commit is 1 candidate" "1"
else
    ad_fail "one uncited commit is 1 candidate" "$(tr '\n' '|' <<<"$AD_T")"
fi
ad_commit "$AD_A" "feat: the widget (ACME-WAR-0001)" src/widget.txt
AD_T=$(ad_telemetry "$AD_A")
if [[ "$(ad_candidates "$AD_T")" == 1 ]]; then
    ad_ok "an ACME-WAR citation is tracked" "still 1"
else
    ad_fail "an ACME-WAR citation is tracked" "$(tr '\n' '|' <<<"$AD_T")"
fi
# The refusal: another namespace's alias names no Warrant here.
ad_commit "$AD_A" "feat: the gadget (OW-WAR-0001)" src/gadget.txt
AD_T=$(ad_telemetry "$AD_A")
if [[ "$(ad_candidates "$AD_T")" == 2 ]] \
    && grep -Fq 'feat: the gadget (OW-WAR-0001)' "$AD_TMP/telemetry.json" \
    && ! grep -Fq 'the widget (ACME-WAR-0001)' "$AD_TMP/telemetry.json"; then
    ad_ok "an OW-WAR citation is not tracked" "2"
else
    ad_fail "an OW-WAR citation is not tracked" "$(tr '\n' '|' <<<"$AD_T")"
fi
# Without the baseline the same repository reads all history: the three
# pre-adoption commits are candidates again, so the bound is what hid them.
AD_A2="$AD_TMP/acme-unbounded"
cp -a "$AD_A" "$AD_A2"
sed -i -e '/^\[adoption\]$/d' -e '/^baseline = /d' "$AD_A2/openwarrant.toml"
assert_gone '[adoption]' "$AD_A2/openwarrant.toml"
AD_T=$(ad_telemetry "$AD_A2")
if [[ "$(ad_candidates "$AD_T")" == 5 ]] && grep -Fq 'all history' <<<"$AD_T" \
    && ! grep -Fq 'adoption_baseline' "$AD_TMP/telemetry.json"; then
    ad_ok "no baseline reads all history" "5, named as all history"
else
    ad_fail "no baseline reads all history" "$(tr '\n' '|' <<<"$AD_T")"
fi

# --- OBL-003: nothing before the baseline is claimed or owned ----------------

AD_BASIS="$AD_A/docs/warrants/ACME-WAR-0001/atoms/20-basis.md"
if grep -Fq "$AD_HEAD" "$AD_BASIS" \
    && grep -Fq 'Nothing before it is claimed, owned or verified by any Warrant' "$AD_BASIS" \
    && ! grep -Fq '{{' "$AD_BASIS"; then
    ad_ok "the adopt Basis names the baseline" "and claims nothing before it"
else
    ad_fail "the adopt Basis names the baseline" "$(grep -n 'Adoption\|{{' "$AD_BASIS")"
fi
# The refusal: with no history the section is not rendered at all.
AD_CBASIS="$AD_C/docs/warrants/ACME-WAR-0001/atoms/20-basis.md"
if [[ -f "$AD_CBASIS" ]] && ! grep -Fq 'Adoption baseline' "$AD_CBASIS" && ! grep -Fq '{{' "$AD_CBASIS"; then
    ad_ok "no history, no baseline section" "and no marker left"
else
    ad_fail "no history, no baseline section" "$(grep -n 'Adoption\|{{' "$AD_CBASIS")"
fi
AD_PINS=$("$AD_WAR" --root "$AD_A" pins 2>&1)
AD_STATUS=$?
if [[ $AD_STATUS -eq 0 ]] && ! grep -Eq 'src/(a|b|c)\.txt' <<<"$AD_PINS"; then
    ad_ok "pins lists no pre-baseline file" "exit 0"
else
    ad_fail "pins lists no pre-baseline file" "exit $AD_STATUS; $(tr '\n' '|' <<<"$AD_PINS")"
fi
# The control is not vacuous: on this corpus, with resolved Warrants, the
# same command does list pinned paths.
# Captured first: `grep -q` closing the pipe early under pipefail would score
# war's SIGPIPE, not its listing.
AD_PINNED=$("$AD_WAR" --root "$REPO_ROOT" pins --resolved-only 2>/dev/null)
if grep -Fq 'crates/' <<<"$AD_PINNED"; then
    ad_ok "pins lists paths where some are pinned" "this corpus"
else
    ad_fail "pins lists paths where some are pinned" "nothing listed on this corpus"
fi
AD_R=$("$AD_WAR" --root "$AD_A" resolve --dry-run ACME-WAR-0001 2>&1)
AD_STATUS=$?
# Nothing established: the dispositions requirement is unmet by name, and
# §38.6 is UNKNOWN because no obligation has an admissible verification.
if [[ $AD_STATUS -eq 2 ]] \
    && grep -Eq '^UNKNOWN resolution\.requirement-unmet .*every required obligation is dispositioned — not established' <<<"$AD_R" \
    && grep -Fq 'no admissible verification' <<<"$AD_R" \
    && ! grep -Eq '^PASS .*every required obligation is dispositioned' <<<"$AD_R"; then
    ad_ok "resolve --dry-run: nothing established" "exit $AD_STATUS"
else
    ad_fail "resolve --dry-run: nothing established" "exit $AD_STATUS; $(head -5 <<<"$AD_R" | tr '\n' '|')"
fi

# --- OBL-004: existing ADRs are pointed at, never imported -------------------

AD_D="$AD_TMP/with-adrs"
ad_history "$AD_D"
ad_commit "$AD_D" "record a decision" docs/adr/0001-x.md
AD_DHEAD=$(git -C "$AD_D" rev-parse HEAD)
AD_OUT=$(cd "$AD_D" && "$AD_WAR" init --program Acme --namespace ACME 2>&1)
if [[ $? -eq 0 ]] && [[ $(grep -Fc "war migrate --corpus docs/adr --commit $AD_DHEAD" <<<"$AD_OUT") -eq 1 ]] \
    && [[ ! -e "$AD_D/artifacts" ]]; then
    ad_ok "existing ADRs get the migrate line" "one line, nothing imported"
else
    ad_fail "existing ADRs get the migrate line" "$(ls "$AD_D"); $(tr '\n' '|' <<<"$AD_OUT")"
fi
# The refusals: no ADR directory, and one holding no NNNN-*.md file.
AD_E="$AD_TMP/readme-only"
ad_history "$AD_E"
ad_commit "$AD_E" "adr readme" docs/adr/README.md
AD_OUT_E=$(cd "$AD_E" && "$AD_WAR" init --program Acme --namespace ACME 2>&1)
if [[ -f "$AD_E/openwarrant.toml" ]] && ! grep -q 'war migrate' <<<"$AD_OUT_E" \
    && ! grep -q 'war migrate' <<<"$AD_OUT_ACME"; then
    ad_ok "no ADR corpus, no migrate line" "README.md alone, or no directory"
else
    ad_fail "no ADR corpus, no migrate line" "$(tr '\n' '|' <<<"$AD_OUT_E")"
fi

# --- OBL-007: init makes a git repository, and never nests one ---------------

AD_F="$AD_TMP/outside"
mkdir -p "$AD_F"
if git -C "$AD_F" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    printf 'PLANT SETUP FAILED: %s is already inside a work tree\n' "$AD_F" >&2
    exit 9
fi
AD_OUT=$(cd "$AD_F" && "$AD_WAR" init --program Acme --namespace ACME 2>&1)
AD_STATUS=$?
AD_CHECK=$("$AD_WAR" --root "$AD_F" check 2>&1)
AD_CSTATUS=$?
if [[ $AD_STATUS -eq 0 ]] && [[ -d "$AD_F/.git" ]] && grep -q '^git init: ' <<<"$AD_OUT" \
    && [[ $AD_CSTATUS -eq 0 ]] && ! grep -Eq '^UNKNOWN.*identity\.changed' <<<"$AD_CHECK"; then
    ad_ok "outside a work tree: git init" "said so; check ready"
else
    ad_fail "outside a work tree: git init" "exit $AD_STATUS/$AD_CSTATUS; $(tr '\n' '|' <<<"$AD_OUT")"
fi
AD_F2="$AD_TMP/outside-plain"
mkdir -p "$AD_F2"
AD_OUT=$(cd "$AD_F2" && "$AD_WAR" init --namespace ACME 2>&1)
if [[ $? -eq 0 ]] && [[ -d "$AD_F2/.git" ]] && grep -q '^git init: ' <<<"$AD_OUT"; then
    ad_ok "plain init outside: git init too" "said so"
else
    ad_fail "plain init outside: git init too" "$(tr '\n' '|' <<<"$AD_OUT")"
fi

# The refusal to nest: a subdirectory of a repository stays a subdirectory.
AD_G="$AD_TMP/enclosing"
ad_history "$AD_G"
mkdir -p "$AD_G/sub"
AD_GHEAD=$(git -C "$AD_G" rev-parse HEAD)
AD_OUT=$(cd "$AD_G/sub" && "$AD_WAR" init --program Acme --namespace ACME 2>&1)
AD_STATUS=$?
if [[ $AD_STATUS -eq 0 ]] && [[ ! -e "$AD_G/sub/.git" ]] && ! grep -q '^git init' <<<"$AD_OUT" \
    && [[ "$(git -C "$AD_G" rev-parse HEAD)" == "$AD_GHEAD" ]] && [[ -f "$AD_G/sub/openwarrant.toml" ]]; then
    ad_ok "inside a work tree: no nested .git" "enclosing HEAD unchanged"
else
    ad_fail "inside a work tree: no nested .git" "exit $AD_STATUS; $(ls -a "$AD_G/sub" | tr '\n' ' ')"
fi

# No git on PATH, for these commands only: init continues and says what it
# cannot check; check must not pass the identity question it cannot ask.
AD_NOGIT="$AD_TMP/no-git-bin"
mkdir -p "$AD_NOGIT"
AD_H="$AD_TMP/no-git"
mkdir -p "$AD_H"
AD_OUT=$(cd "$AD_H" && PATH="$AD_NOGIT" "$AD_WAR" init --program Acme --namespace ACME 2>&1)
AD_STATUS=$?
if [[ $AD_STATUS -eq 0 ]] && [[ -f "$AD_H/openwarrant.toml" ]] && [[ -d "$AD_H/docs/warrants/ACME-WAR-0001" ]] \
    && [[ ! -e "$AD_H/.git" ]] && grep -q '^git is not available' <<<"$AD_OUT" \
    && grep -Fq 'cannot be checked until' <<<"$AD_OUT"; then
    ad_ok "no git: init continues, says so" "exit 0, scaffold written"
else
    ad_fail "no git: init continues, says so" "exit $AD_STATUS; $(tr '\n' '|' <<<"$AD_OUT")"
fi
AD_CHECK=$(PATH="$AD_NOGIT" "$AD_WAR" --root "$AD_H" check 2>&1)
if grep -q 'identity\.changed' <<<"$AD_CHECK"; then
    if grep -Eq '^UNKNOWN.*identity\.changed' <<<"$AD_CHECK" \
        && ! grep -Eq '^(PASS|ok).*identity\.changed' <<<"$AD_CHECK"; then
        ad_ok "no git: identity.changed is UNKNOWN" "never a pass"
    else
        ad_fail "no git: identity.changed is UNKNOWN" "$(grep 'identity\.changed' <<<"$AD_CHECK" | head -3)"
    fi
else
    ad_unknown "no git: identity.changed" "this build has no identity.changed rule (OW-WAR-0119 not merged); unasked"
fi

rm -rf "$AD_TMP"
