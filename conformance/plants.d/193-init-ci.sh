# shellcheck shell=bash
# W1.5: `war init --ci`, one opt-in fast pull-request gate. Each claim beside
# its refusal:
#   plain init writes no workflow;
#   `--ci` writes one parseable workflow that runs `war check --changed` and
#   the repository's own test, found from what it holds or named;
#   the check step really does pass a clean change and fail a broken one;
#   a workflow that differs is left alone, and twice changes nothing.

echo "== init --ci: an opt-in fast gate (W1.5) =="
CI_T=$(mktemp -d)
ci_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ci_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
CI_WAR="$REPO_ROOT/${WAR#./}"
ciw() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR OPENWARRANT_NO_PROJECTS=1 \
    "$CI_WAR" --root "$CI_R" "$@" </dev/null; }
CI_F=.github/workflows/openwarrant.yml

CI_R="$CI_T/plain"; mkdir -p "$CI_R"
ciw init --namespace CI >/dev/null 2>&1
if [[ ! -e "$CI_R/.github" ]]; then
    ci_ok "plain init writes no workflow" "the gate is opt-in"
else
    ci_fail "plain init writes no workflow" "$(ls -A "$CI_R/.github")"
fi

CI_R="$CI_T/rust"; mkdir -p "$CI_R"; printf '[package]\nname="x"\n' > "$CI_R/Cargo.toml"
CI_OUT=$(ciw init --namespace CI --ci 2>&1); CI_S=$?
CI_Y=$(python3 - "$CI_R/$CI_F" <<'PY' 2>&1
# Read the `run` steps by hand: a runner need not have PyYAML. Where it does,
# the file must parse as well.
import re, sys
text = open(sys.argv[1]).read()
try:
    import yaml
except ImportError:
    pass
else:
    yaml.safe_load(text)
runs, lines, i = [], text.splitlines(), 0
while i < len(lines):
    m = re.match(r"( *)(?:- )?run: *(.*)$", lines[i])
    i += 1
    if not m:
        continue
    body = m.group(2)
    if body == "|":
        block = []
        while i < len(lines) and (not lines[i].strip() or len(lines[i]) - len(lines[i].lstrip()) > len(m.group(1))):
            block.append(lines[i].strip())
            i += 1
        body = "\n".join(block).strip()
    runs.append(body)
print(" | ".join(r for r in runs if "curl" not in r))
PY
)
if [[ $CI_S -eq 0 && -f "$CI_R/openwarrant.toml" && "$CI_Y" == 'war check --changed --since "$BASE" | cargo test' ]]; then
    ci_ok "--ci writes one working workflow" "$CI_Y"
else
    ci_fail "--ci writes one working workflow" "exit $CI_S: $CI_OUT / $CI_Y"
fi

# On a repository already set up it adds the workflow and nothing else, with
# the test it is told; twice is the same bytes.
CI_R="$CI_T/plain"
CI_CFG=$(cksum < "$CI_R/openwarrant.toml")
ciw init --ci --test "make verify" >/dev/null 2>&1; CI_S=$?
CI_B=$(cksum < "$CI_R/$CI_F")
ciw init --ci --test "make verify" >/dev/null 2>&1; CI_S2=$?
if [[ $CI_S -eq 0 && $CI_S2 -eq 0 && "$(cksum < "$CI_R/$CI_F")" == "$CI_B" \
    && "$(cksum < "$CI_R/openwarrant.toml")" == "$CI_CFG" ]] \
    && grep -q '^ *make verify$' "$CI_R/$CI_F"; then
    ci_ok "--ci on a set-up repository" "the workflow only, and twice changes nothing"
else
    ci_fail "--ci on a set-up repository" "exit $CI_S then $CI_S2"
fi

# The step the workflow runs, run: a broken ticket document fails it, and a
# pull request that touches nothing passes.
CI_R="$CI_T/run"; mkdir -p "$CI_R"
ciw init --namespace CI >/dev/null 2>&1
CI_ID=$(ciw create "Tracked" -i one 2>/dev/null | awk 'NR==1{print $1}')
git -C "$CI_R" add -A >/dev/null 2>&1
git -C "$CI_R" -c user.name=t -c user.email=t@t commit -qm base >/dev/null 2>&1
CI_BASE=$(git -C "$CI_R" rev-parse HEAD)
ciw check --changed --since "$CI_BASE" >/dev/null 2>&1; CI_CLEAN=$?
sed -i 's/^format: .*/format: openwarrant\/9.9/' "$CI_R/docs/tickets/$CI_ID.md"
git -C "$CI_R" add -A >/dev/null 2>&1
git -C "$CI_R" -c user.name=t -c user.email=t@t commit -qm change >/dev/null 2>&1
ciw check --changed --since "$CI_BASE" >"$CI_T/bad.out" 2>&1; CI_BAD=$?
if [[ $CI_CLEAN -eq 0 && $CI_BAD -ne 0 ]]; then
    ci_ok "its check passes and refuses" "clean exit 0, a broken ticket exit $CI_BAD"
else
    ci_fail "its check passes and refuses" "clean=$CI_CLEAN bad=$CI_BAD: $(head -c 300 "$CI_T/bad.out")"
fi

# A single-file Warrant is in scope too: nothing else names its path.
CI_R="$CI_T/doc"; mkdir -p "$CI_R"
ciw init --namespace CD >/dev/null 2>&1
ciw plan new --format 1.0 "One document" >/dev/null 2>&1
git -C "$CI_R" add -A >/dev/null 2>&1
git -C "$CI_R" -c user.name=t -c user.email=t@t commit -qm base >/dev/null 2>&1
sed -i 's/^## Intent$/## Not it/' "$CI_R/docs/warrants/CD-WAR-0001.md"
ciw check --changed >"$CI_T/doc.out" 2>&1; CI_DOC=$?
if [[ $CI_DOC -ne 0 ]] && grep -q 'format.section-missing' "$CI_T/doc.out"; then
    ci_ok "a changed document is checked" "single-file Warrant refused by its rule, exit $CI_DOC"
else
    ci_fail "a changed document is checked" "exit $CI_DOC: $(head -c 300 "$CI_T/doc.out")"
fi

CI_R="$CI_T/plain"
printf 'mine\n' > "$CI_R/$CI_F"
CI_ERR=$(ciw init --ci 2>&1); CI_S=$?
if [[ $CI_S -ne 0 && "$(cat "$CI_R/$CI_F")" == mine ]] && grep -q 'left alone' <<<"$CI_ERR"; then
    ci_ok "a workflow that differs is left alone" "refused, exit $CI_S"
else
    ci_fail "a workflow that differs is left alone" "exit $CI_S: $CI_ERR"
fi
command rm -rf "$CI_T"
