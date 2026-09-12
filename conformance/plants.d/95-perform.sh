# shellcheck shell=bash
# `war perform` (OW-WAR-0069): an agent stage started by the tool, and every
# line it must not cross. The fixtures under conformance/fixtures/performer/
# stand in for a model: one answers legally, one asks to be resolved, one fails
# without answering, one never finishes.
#
# Each case runs against OW-WAR-0068, whose M1 stages are open agent stages.
# `war perform` writes under docs/warrants/<alias>/{dispatches,submissions}/,
# which `git checkout` does not undo for new files, so they are removed by name.

P_ALIAS=OW-WAR-0068
P_DIR=docs/warrants/$P_ALIAS
P_FX=conformance/fixtures/performer

p_cleanup() {
    rm -rf "$P_DIR/dispatches" "$P_DIR/submissions"
    restore
}

# performer_argv is set by rewriting the config the same way a user would, and
# `assert_present` proves the rewrite landed before anything is scored.
p_set_performer() {
    python3 - "$1" <<'PY'
import re, sys
argv = sys.argv[1]
p = "openwarrant.toml"
s = open(p).read()
body = f'performer_argv = ["{argv}"]' if argv else "performer_argv = []"
s = re.sub(r'^performer_argv = .*$', body, s, count=1, flags=re.M)
open(p, "w").write(s)
PY
}

# 1. The positive: a legal answer is ingested, and the submission lands.
p_set_performer "$P_FX/echo-submission.sh"
assert_present "$P_FX/echo-submission.sh" openwarrant.toml
out=$("$WAR" perform "$P_ALIAS" STAGE-001 2>&1)
if grep -q 'perform.answered' <<< "$out" && grep -q 'submission.recorded' <<< "$out" \
    && ls "$P_DIR"/submissions/*.json > /dev/null 2>&1; then
    printf 'ok    %-34s the dispatch was answered and recorded\n' "an agent stage is performed"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "an agent stage is performed" "$(head -2 <<< "$out")"
    FAILED=$((FAILED + 1))
fi
p_cleanup

# 2. §51.2: a performer asking to be resolved is refused, and NOTHING is left
# behind — not the submission, and not the raw answer either.
p_set_performer "$P_FX/requests-resolution.sh"
assert_present "$P_FX/requests-resolution.sh" openwarrant.toml
out=$("$WAR" perform "$P_ALIAS" STAGE-001 2>&1)
P_LEFT=$(ls "$P_DIR"/submissions/*.json "$P_DIR"/dispatches/answer-*.json 2>/dev/null | wc -l)
if grep -q 'submission.self-completion' <<< "$out" && [[ "$P_LEFT" == "0" ]]; then
    printf 'ok    %-34s refused, and no answer left on disk\n' "a performer asking to resolve"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s files left\n%s\n' "a performer asking to resolve" "$P_LEFT" "$(head -2 <<< "$out")"
    FAILED=$((FAILED + 1))
fi
p_cleanup

# 3. A performer that fails without answering leaves the stage where it was.
p_set_performer "$P_FX/says-nothing.sh"
out=$("$WAR" perform "$P_ALIAS" STAGE-001 2>&1)
P_LEFT=$(ls "$P_DIR"/submissions/*.json 2>/dev/null | wc -l)
if grep -q 'perform.failed' <<< "$out" && [[ "$P_LEFT" == "0" ]]; then
    printf 'ok    %-34s nothing claims the work happened\n' "a performer that fails"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s submission(s) written\n' "a performer that fails" "$P_LEFT"
    FAILED=$((FAILED + 1))
fi
p_cleanup

# 4. A human stage is a human's. No performer stands in for a person (§27.2).
p_set_performer "$P_FX/echo-submission.sh"
out=$("$WAR" perform OW-WAR-0069 STAGE-001 2>&1)
if grep -q 'perform.human-stage' <<< "$out"; then
    printf 'ok    %-34s rejected by perform.human-stage\n' "performing a human stage"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "performing a human stage" "$(head -2 <<< "$out")"
    FAILED=$((FAILED + 1))
fi

# 5. No performer configured: the refusal names the config key.
p_set_performer ""
assert_gone 'fixtures/performer' openwarrant.toml
out=$("$WAR" perform "$P_ALIAS" STAGE-001 2>&1)
if grep -q 'perform.no-performer' <<< "$out"; then
    printf 'ok    %-34s rejected by perform.no-performer\n' "no performer configured"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "no performer configured" "$(head -2 <<< "$out")"
    FAILED=$((FAILED + 1))
fi
p_cleanup

# 6. Concurrency above one is refused while nothing contains a performer.
p_set_performer "$P_FX/echo-submission.sh"
sed -i 's/^max_concurrent = 1$/max_concurrent = 4/' openwarrant.toml
assert_present 'max_concurrent = 4' openwarrant.toml
out=$("$WAR" perform "$P_ALIAS" STAGE-001 2>&1)
if grep -q 'perform.no-containment' <<< "$out" && grep -q 'Q-006' <<< "$out"; then
    printf 'ok    %-34s rejected by perform.no-containment\n' "max_concurrent above one"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "max_concurrent above one" "$(head -2 <<< "$out")"
    FAILED=$((FAILED + 1))
fi
p_cleanup

# 7. The deadline: a performer that never finishes is killed and reported.
p_set_performer "$P_FX/sleeps.sh"
python3 - <<'PY'
import re
p = "openwarrant.toml"
s = open(p).read()
s = re.sub(r'^performer_timeout_secs = .*$', 'performer_timeout_secs = 2', s, count=1, flags=re.M)
open(p, "w").write(s)
PY
assert_present 'performer_timeout_secs = 2' openwarrant.toml
out=$("$WAR" perform "$P_ALIAS" STAGE-001 2>&1)
if grep -q 'perform.timeout' <<< "$out"; then
    printf 'ok    %-34s killed at the bound, nothing recorded\n' "a performer that never finishes"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s %s\n' "a performer that never finishes" "$(head -2 <<< "$out")"
    FAILED=$((FAILED + 1))
fi
p_cleanup
