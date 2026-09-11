# shellcheck shell=bash
# `war init --program` (slice C5): the scaffold passes the tool that judges it.
# Runs in a scratch directory, never in this corpus.

PROG_TMP=$(mktemp -d)
(
    cd "$PROG_TMP" && git init -q . && "$OLDPWD/$WAR" init --program "Plant Program" --namespace PP >/dev/null 2>&1
)
PROG_WAR="$PWD/$WAR"

prog_expect() {
    local name="$1" want_exit="$2" want="$3"
    shift 3
    local out status
    out=$(cd "$PROG_TMP" && "$PROG_WAR" "$@" 2>&1)
    status=$?
    if [[ $status -eq $want_exit ]] && { [[ -z "$want" ]] || grep -Fq -- "$want" <<< "$out"; }; then
        printf 'ok    %-34s exit %s %s\n' "$name" "$status" "$want"
        PASSED=$((PASSED + 1))
    else
        printf 'FAIL  %-34s wanted exit %s %s; got exit %s:\n%s\n' "$name" "$want_exit" "$want" "$status" "$(grep -E '^(ERROR|UNKNOWN|error)' <<< "$out" | head -4)"
        FAILED=$((FAILED + 1))
    fi
}

prog_expect "a scaffolded program passes check" 0 "" check
prog_expect "the scaffold proposes its SAS" 0 "sas.proposed" sas propose 0.1.0
prog_expect "the scaffold still passes after the proposal" 0 "" check
prog_expect "next names the human acts" 0 "HUMAN" next
# The first Warrant traces into the SAS just written, and the tool reads that.
prog_expect "the first Warrant implements RQ-001" 0 "PP-SAS-RQ-001" show PP-WAR-0001
# A namespace a §106 row cannot carry is refused by name, not scaffolded wrong.
prog_expect "a digit in the namespace is refused" 1 "uppercase letters only" init --program "X" --namespace P1 --root "$PROG_TMP/bad"

rm -rf "$PROG_TMP"
