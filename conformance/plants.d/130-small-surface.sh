# shellcheck shell=bash
# OW-WAR-0148 M12 — a small surface. `war --help` lists the daily verbs, at
# most twelve, and below them one "More:" block naming the five groups with a
# purpose each; every group's help lists its members (docs/COMMANDS.md).
#
# Each claim is paired with an observed refusal:
# - The Commands section is exactly the daily verbs (clap's own `help` set
#   aside), at most twelve. The same reader over the help with planted verbs
#   accepts a twelfth (`war start`, M15's) and refuses a thirteenth.
# - "More:" names plan, sign, evidence, view and admin, each with a purpose.
#   The same reader over the help with one group's line removed refuses it.
# - Each group's help lists exactly its members. A member asked of the wrong
#   group (`war view compile`) is refused by clap, exit 2, naming the word,
#   and so is a word that was never a command (`war frobnicate`).
# - No earlier spelling is in `war --help`, and each still answers its own
#   `--help` with its own usage line and nothing on stderr.

echo "== the small surface: help (M12) =="
SS_WAR="$REPO_ROOT/${WAR#./}"
ss_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ss_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
sw() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR "$SS_WAR" "$@" </dev/null; }

# The words listed in one section of a help text (`Commands:` or `More:`),
# clap's own `help` set aside, space-separated in the order shown.
ss_section() {
    awk -v h="$2" '$0 == h {f=1; next} /^[^ ]/ || /^$/ {f=0} f && /^  [a-z]/ {print $1}' <<<"$1" \
        | awk '$0 != "help"' | tr '\n' ' ' | sed 's/ $//'
}
# The reader the claims use: a Commands section of one to twelve verbs, and a
# More: section naming exactly the five groups, each with a purpose after it.
ss_reads_ok() {
    local daily n groups purposes
    daily=$(ss_section "$1" "Commands:")
    n=$(wc -w <<<"$daily")
    groups=$(ss_section "$1" "More:")
    purposes=$(awk '$0 == "More:" {f=1; next} /^[^ ]/ || /^$/ {f=0} f && NF >= 3' <<<"$1" | wc -l)
    [[ $n -ge 1 && $n -le 12 && "$groups" == "plan sign evidence view admin" && $purposes -eq 5 ]]
}

SS_DAILY_WANT="init create next claim done add note edit show status check"
SS_HELP=$(sw --help 2>&1); SS_RC=$?
SS_DAILY=$(ss_section "$SS_HELP" "Commands:")
SS_LINES=$(wc -l <<<"$SS_HELP")
if [[ $SS_RC -eq 0 && "$SS_DAILY" == "$SS_DAILY_WANT" ]] && ss_reads_ok "$SS_HELP"; then
    ss_ok "help lists the daily verbs" "$(wc -w <<<"$SS_DAILY") verbs and five groups, $SS_LINES lines"
else
    ss_fail "help lists the daily verbs" "exit $SS_RC, Commands: '$SS_DAILY', More: '$(ss_section "$SS_HELP" "More:")'"
fi

# The reader's bound, seen both ways on planted help: a twelfth verb is
# admitted, a thirteenth refused; a More: block missing a group is refused.
SS_P12=$(sed 's/^  check /  start   Start a Warrant (planted twelfth)\n  check /' <<<"$SS_HELP")
SS_P13=$(sed 's/^  check /  start   Start a Warrant (planted twelfth)\n  frob    A planted thirteenth verb\n  check /' <<<"$SS_HELP")
SS_PMORE=$(awk '!/^  view /' <<<"$SS_HELP")
if ss_reads_ok "$SS_P12" && [[ $(wc -w <<<"$(ss_section "$SS_P13" "Commands:")") -eq 13 ]] \
    && ! ss_reads_ok "$SS_P13" && ! ss_reads_ok "$SS_PMORE"; then
    ss_ok "a planted 13th verb is refused" "12 admitted, 13 refused; More: without view refused"
else
    ss_fail "a planted 13th verb is refused" "the reader did not tell 12, 13 and a missing group apart"
fi

# Each group's members, as docs/COMMANDS.md lists them. `schemas` is in a
# build with the `schema` feature only. M14 added `sign approve`, `sign
# release` and `admin preset`, under their groups only.
declare -A SS_MEMBERS=(
    [plan]="new promote render impact model state roadmap types type frontier questions ask answer answers"
    [sign]="authorize resolve correct amend attest standing sas inbox authority approve release"
    [evidence]="record gate verify prepare run perform submit kpi mark document eval"
    [view]="ui tui board console watch overview warrants ready prime timeline"
    [admin]="compile doctor pins preflight diff journal deliver dispatch dispatch-bundle commit heartbeat release renumber agents-md import export migrate archive bridge host sdk mcp kf telemetry bonsai blut projects update version merge-ticket preset"
)
SS_BAD=""
for g in plan sign evidence view admin; do
    gh=$(sw "$g" --help 2>&1); grc=$?
    got=$(ss_section "$gh" "Commands:" | tr ' ' '\n' | awk '$0 != "schemas"' | tr '\n' ' ' | sed 's/ $//')
    [[ $grc -eq 0 && "$got" == "${SS_MEMBERS[$g]}" ]] || SS_BAD="$SS_BAD $g(exit $grc: $got)"
done
SS_WRONG=$(sw view compile 2>&1); SS_WRONG_RC=$?
SS_NEVER=$(sw frobnicate 2>&1); SS_NEVER_RC=$?
if [[ -z "$SS_BAD" && $SS_WRONG_RC -eq 2 && $SS_NEVER_RC -eq 2 ]] \
    && grep -q "unrecognized subcommand 'compile'" <<<"$SS_WRONG" \
    && grep -q "unrecognized subcommand 'frobnicate'" <<<"$SS_NEVER"; then
    ss_ok "each group's help lists its members" "5 groups; \`war view compile\` and \`war frobnicate\` refused, exit 2"
else
    ss_fail "each group's help lists its members" "${SS_BAD:-groups ok}; view compile exit $SS_WRONG_RC, frobnicate exit $SS_NEVER_RC"
fi

# Every earlier top-level spelling (docs/COMMANDS.md's inventory, 89 with the
# aliases; `schemas` is a `schema`-feature build's and is left out here):
# absent from `war --help`, and answering its own `--help` under its own
# name, with nothing on stderr (no deprecation warning).
SS_OLD="new promote render impact model state roadmap frontier questions ask answer answers \
authorize resolve correct amend attest standing sas inbox authority \
gate verify prepare run perform submit kpi mark document eval \
ui tui board console watch overview progress warrants tickets ls ready prime \
compile doctor pins preflight diff journal deliver dispatch dispatch-bundle commit heartbeat \
release renumber agents-md import export migrate archive bridge host sdk mcp kf telemetry \
bonsai blut projects update version merge-ticket \
init create next claim done add note edit show status check plan sign evidence __release-check"
SS_ERR=$(mktemp)
SS_MISS=""; SS_N=0
for w in $SS_OLD; do
    SS_N=$((SS_N + 1))
    h=$(sw "$w" --help 2>"$SS_ERR"); hrc=$?
    # An alias answers with its command's own name.
    case "$w" in progress) name=overview ;; tickets | ls) name=warrants ;; *) name=$w ;; esac
    if [[ $hrc -ne 0 || -s "$SS_ERR" ]] || ! grep -q "^Usage: war $name\b" <<<"$h"; then
        SS_MISS="$SS_MISS $w"
    fi
    # Listed under Commands: only if it is a daily verb.
    case " $SS_DAILY_WANT " in *" $w "*) ;; *)
        case " $SS_DAILY " in *" $w "*) SS_MISS="$SS_MISS $w(listed)" ;; esac ;;
    esac
done
command rm -f "$SS_ERR"
if [[ -z "$SS_MISS" && $SS_N -eq 88 ]]; then
    ss_ok "every old spelling, hidden, answers" "$SS_N spellings: none in \`war --help\`, each its own usage, no warning"
else
    ss_fail "every old spelling, hidden, answers" "$SS_N checked; wrong:$SS_MISS"
fi
