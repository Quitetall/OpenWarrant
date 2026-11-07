# shellcheck shell=bash
# W1.4: `war init` writes `openwarrant.toml` and the instruction block, and the
# schemas stay in the binary until someone asks. Each claim beside its refusal:
#   plain init writes no schemas/, and `war check` passes without them;
#   an archive of such a repository still retains the pack, from the binary;
#   `--schemas` writes the pack, also into a repository already set up;
#   `--schemas` over a file that differs is refused and leaves it alone.

echo "== init: schemas in the binary (W1.4) =="
SI_T=$(mktemp -d)
si_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
si_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
SI_WAR="$REPO_ROOT/${WAR#./}"
siw() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR OPENWARRANT_NO_PROJECTS=1 \
    "$SI_WAR" --root "$SI_R" "$@" </dev/null; }

SI_R="$SI_T/plain"; mkdir -p "$SI_R"
siw init --namespace SI >/dev/null 2>&1; SI_I=$?
siw check >/dev/null 2>&1; SI_C=$?
if [[ $SI_I -eq 0 && $SI_C -eq 0 && -f "$SI_R/openwarrant.toml" && -f "$SI_R/AGENTS.md" && ! -e "$SI_R/schemas" ]]; then
    si_ok "plain init writes no schemas" "openwarrant.toml and AGENTS.md; war check exit 0 without schemas/"
else
    si_fail "plain init writes no schemas" "init=$SI_I check=$SI_C; $(ls -A "$SI_R" | tr '\n' ' ')"
fi

SI_P="$SI_T/program"; mkdir -p "$SI_P"
SI_R="$SI_P" siw init --namespace SIP --program "Archive me" >/dev/null 2>&1
( cd "$SI_P" && SI_R="$SI_P" siw --json admin archive export SIP-WAR-0001 a.json ) >"$SI_T/export.out" 2>&1; SI_E=$?
if [[ $SI_E -eq 0 ]] && grep -q 'schemas/oh.war/war/v1.json' "$SI_P/a.json" && [[ ! -e "$SI_P/schemas" ]]; then
    si_ok "an archive retains the binary's pack" "the repository still holds no schemas/"
else
    si_fail "an archive retains the binary's pack" "export=$SI_E: $(head -c 300 "$SI_T/export.out")"
fi

siw init --schemas >"$SI_T/seed.out" 2>&1; SI_S=$?
SI_A=$(cmp "$SI_R/schemas/pack.json" "$REPO_ROOT/schemas/pack.json" 2>&1)
if [[ $SI_S -eq 0 && -z "$SI_A" && -f "$SI_R/schemas/oh.war/war/v1.json" ]]; then
    si_ok "--schemas writes them when asked" "into a repository already set up; pack.json matches this build's"
else
    si_fail "--schemas writes them when asked" "exit $SI_S: $(head -c 300 "$SI_T/seed.out") $SI_A"
fi

printf 'mine\n' > "$SI_R/schemas/pack.json"
SI_ERR=$(siw init --schemas 2>&1); SI_S=$?
if [[ $SI_S -ne 0 && "$(cat "$SI_R/schemas/pack.json")" == mine ]] && grep -q 'schema pack setup refused' <<<"$SI_ERR"; then
    si_ok "a file that differs is left alone" "refused by name, exit $SI_S"
else
    si_fail "a file that differs is left alone" "exit $SI_S: $SI_ERR"
fi
command rm -rf "$SI_T"
