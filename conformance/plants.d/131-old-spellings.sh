# shellcheck shell=bash
# OW-WAR-0148 M12 — every earlier spelling is the command it always was. Each
# top-level spelling M12 moved into a group (docs/COMMANDS.md) is run both
# ways on one scratch program, and the two answers are compared byte for
# byte: stdout (one `--json` envelope, `command` field included), stderr and
# the exit code.
#
# Each claim is paired with an observed refusal:
# - Same subcommand: `war <old> --help` and `war <group> <old> --help` differ
#   only in the usage line's group word, for every moved spelling. A member
#   asked of the wrong group (`war admin board`) is refused, exit 2.
# - Same output: a representative read-only invocation of each, with
#   `--json`, gives identical bytes both ways, with no warning on stderr.
#   The comparison has teeth: two different commands (`next`, `status`)
#   compared the same way are told apart.
#
# `war update` has no read-only invocation that stays off the network, so it
# is held to its help alone; `schemas` exists only in a `schema`-feature build
# and is compared when the binary has it. Two values minted at the moment of
# the call (inbox's `generated_at`, `amend --dry-run`'s amendment id) are the
# only text set aside. Every invocation is read-only: the writers are asked
# something they refuse (an unknown id), which still parses the whole command.

echo "== the small surface: every old spelling is the same command (M12) =="
PLANT_ROOT=$(scratch_corpus SP)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
sp_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
sp_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
SP_T=$(mktemp -d)
# The per-user directories are the plant's own and empty, so `projects`,
# `ui devices` and the notice read nothing of whoever runs the battery.
mkdir -p "$SP_T/xdg/config" "$SP_T/xdg/state" "$SP_T/xdg/cache" "$SP_T/xdg/data"
sp() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR OPENWARRANT_NO_PROJECTS=1 \
    XDG_CONFIG_HOME="$SP_T/xdg/config" XDG_STATE_HOME="$SP_T/xdg/state" \
    XDG_CACHE_HOME="$SP_T/xdg/cache" XDG_DATA_HOME="$SP_T/xdg/data" \
    "$WAR" --root "$PLANT_ROOT" "$@" </dev/null; }
SP_A=$(cd "$PLANT_ROOT/docs/warrants" && find . -maxdepth 1 -name 'SP-WAR-*' -printf '%f\n' | sort | head -1)
[[ "$SP_A" =~ ^SP-WAR-[0-9]{4}$ ]] || { printf 'PLANT SETUP FAILED: no SP Warrant in the scratch corpus (%s)\n' "$SP_A" >&2; exit 9; }
# Something for the lists to show: two tickets, one item claimed.
SP_TK=$(sp --json create "Plant ticket" --item "First" --item "Second" 2>/dev/null \
    | python3 -c 'import json,sys; print(json.load(sys.stdin)["result"]["id"])' 2>/dev/null)
sp create "Another" >/dev/null 2>&1
sp claim "$SP_TK" >/dev/null 2>&1

# run <file-prefix> <args...>: stdout, stderr and the exit code, to files.
sp_run() {
    local out="$1"; shift
    sp "$@" >"$out.out" 2>"$out.err"; echo $? >"$out.rc"
    # Two answers carry the moment of the call: inbox's `generated_at`, and
    # the amendment id `amend --dry-run` mints (AM-<n>-<4 hex>, fresh on every
    # call so two branches never mint the same file). Nothing else is set aside.
    sed -i -e '/"generated_at": /d' -e 's/AM-\([0-9]*\)-[0-9a-f]\{4\}/AM-\1-xxxx/g' "$out.out"
}
# same <old words> | <group>: the earlier spelling and the group's, compared.
sp_same() {
    local old="$1" group="$2" a="$SP_T/a" b="$SP_T/b"
    # shellcheck disable=SC2086
    sp_run "$a" --json $old
    # shellcheck disable=SC2086
    sp_run "$b" --json $group $old
    cmp -s "$a.out" "$b.out" && cmp -s "$a.err" "$b.err" && cmp -s "$a.rc" "$b.rc"
}

# --- same subcommand: the help, every moved spelling ---------------------------
declare -A SP_HOME=(
    [new]=plan [promote]=plan [render]=plan [impact]=plan [model]=plan [state]=plan [roadmap]=plan
    [frontier]=plan [questions]=plan [ask]=plan [answer]=plan [answers]=plan
    [authorize]=sign [resolve]=sign [correct]=sign [amend]=sign [attest]=sign [standing]=sign
    [sas]=sign [inbox]=sign [authority]=sign
    [gate]=evidence [verify]=evidence [prepare]=evidence [run]=evidence [perform]=evidence
    [submit]=evidence [kpi]=evidence [mark]=evidence [document]=evidence [eval]=evidence
    [ui]=view [tui]=view [board]=view [console]=view [watch]=view [overview]=view [progress]=view
    [warrants]=view [tickets]=view [ls]=view [ready]=view [prime]=view
    [compile]=admin [doctor]=admin [pins]=admin [preflight]=admin [diff]=admin [journal]=admin
    [deliver]=admin [dispatch]=admin [dispatch-bundle]=admin [commit]=admin [heartbeat]=admin
    [release]=admin [renumber]=admin [agents-md]=admin [import]=admin [export]=admin
    [migrate]=admin [archive]=admin [bridge]=admin [host]=admin [sdk]=admin [mcp]=admin [kf]=admin
    [telemetry]=admin [bonsai]=admin [blut]=admin [projects]=admin [update]=admin [version]=admin
    [merge-ticket]=admin
)
SP_HBAD=""; SP_HN=0
for w in "${!SP_HOME[@]}"; do
    g=${SP_HOME[$w]}
    SP_HN=$((SP_HN + 1))
    ho=$(sp "$w" --help 2>&1); hro=$?
    hn=$(sp "$g" "$w" --help 2>&1 | sed "s/^Usage: war $g /Usage: war /; s/^       war $g /       war /"); hrn=$?
    [[ $hro -eq 0 && $hrn -eq 0 && "$ho" == "$hn" ]] || SP_HBAD="$SP_HBAD $w"
done
SP_WRONG=$(sp admin board 2>&1); SP_WRONG_RC=$?
if [[ -z "$SP_HBAD" && $SP_HN -eq 73 && $SP_WRONG_RC -eq 2 ]] && grep -q "unrecognized subcommand 'board'" <<<"$SP_WRONG"; then
    sp_ok "each old spelling is its member" "$SP_HN spellings: the same help under the group; \`war admin board\` refused"
else
    sp_fail "each old spelling is its member" "$SP_HN checked, help differs:$SP_HBAD; admin board exit $SP_WRONG_RC"
fi

# --- same output: a read-only invocation of each, with --json ------------------
SP_NONE="$SP_T/none"
SP_CASES=(
    "new Plant-draft --preset no-such-preset|plan"
    "promote t-ffff|plan"
    "render prd|plan"
    "impact $SP_A|plan"
    "model|plan"
    "state t-ffff in_review|plan"
    "roadmap|plan"
    "frontier|plan"
    "questions|plan"
    "ask $SP_A STAGE-999 question|plan"
    "answer $SP_A Q-999 answer --as nobody|plan"
    "answers $SP_A|plan"
    "authorize $SP_A|sign"
    "resolve $SP_A --dry-run|sign"
    "correct $SP_A D-001|sign"
    "amend $SP_A --dry-run|sign"
    "attest --all|sign"
    "standing show|sign"
    "sas status|sign"
    "inbox|sign"
    "authority status --store $SP_NONE|sign"
    "gate|evidence"
    "verify $SP_A|evidence"
    "prepare $SP_A --dry-run|evidence"
    "run $SP_A STAGE-999|evidence"
    "perform $SP_A STAGE-999|evidence"
    "submit $SP_A $SP_NONE.json|evidence"
    "kpi run t-ffff|evidence"
    "mark $SP_A|evidence"
    "document review|evidence"
    "eval verify $SP_NONE.json|evidence"
    "ui devices|view"
    "tui|view"
    "board|view"
    "console|view"
    "watch --once|view"
    "overview|view"
    "progress|view"
    "warrants|view"
    "tickets --state open|view"
    "ls --label none|view"
    "ready|view"
    "prime|view"
    "compile|admin"
    "doctor|admin"
    "pins|admin"
    "preflight $SP_A|admin"
    "diff $SP_A|admin"
    "journal $SP_A|admin"
    "deliver $SP_A --dry-run|admin"
    "dispatch $SP_A STAGE-999|admin"
    "dispatch-bundle check $SP_NONE.json --expected-digest sha256:00|admin"
    "commit|admin"
    "heartbeat t-ffff|admin"
    "release t-ffff|admin"
    "renumber SP-WAR-9998 SP-WAR-9999|admin"
    "agents-md --stdout|admin"
    "import beads $SP_NONE.jsonl|admin"
    "export beads|admin"
    "migrate --corpus $SP_NONE --commit HEAD|admin"
    "archive inspect $SP_NONE|admin"
    "bridge claude-tasks --file $SP_NONE.json|admin"
    "host --export|admin"
    "sdk --request $SP_NONE.json|admin"
    "mcp --describe|admin"
    "kf health --base http://127.0.0.1:9|admin"
    "telemetry --commit HEAD --verify --out $SP_NONE.json|admin"
    "bonsai verify-evidence --evidence $SP_NONE.json|admin"
    "blut $SP_A|admin"
    "projects|admin"
    "version|admin"
    "merge-ticket --probe|admin"
)
# `war view timeline` is `war status --timeline` under the name a reader
# looks for; the binary built with `--features schema` also has `schemas`.
SP_DIFF=""; SP_N=0
for c in "${SP_CASES[@]}"; do
    SP_N=$((SP_N + 1))
    sp_same "${c%|*}" "${c##*|}" || SP_DIFF="$SP_DIFF [${c%|*}]"
done
sp_run "$SP_T/a" --json status --timeline
sp_run "$SP_T/b" --json view timeline
cmp -s "$SP_T/a.out" "$SP_T/b.out" && cmp -s "$SP_T/a.err" "$SP_T/b.err" || SP_DIFF="$SP_DIFF [view timeline]"
if sp admin schemas --help >/dev/null 2>&1; then
    SP_N=$((SP_N + 1))
    sp_same "schemas --check" admin || SP_DIFF="$SP_DIFF [schemas --check]"
fi
# Teeth: the same comparison of two different commands tells them apart.
sp_run "$SP_T/a" --json next
sp_run "$SP_T/b" --json status
SP_TEETH=0; cmp -s "$SP_T/a.out" "$SP_T/b.out" || SP_TEETH=1
if [[ -z "$SP_DIFF" && $SP_N -ge 72 && $SP_TEETH -eq 1 ]]; then
    sp_ok "each old spelling, the same bytes" "$SP_N invocations and view timeline: stdout, stderr, exit identical; next/status told apart"
else
    sp_fail "each old spelling, the same bytes" "$SP_N compared; differ:$SP_DIFF; teeth $SP_TEETH"
fi
command rm -rf "$SP_T"
