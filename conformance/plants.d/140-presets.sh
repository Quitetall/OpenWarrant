# shellcheck shell=bash
# OW-WAR-0148 M14 — repo presets: `war init --vibe | --team | --regulated`,
# `war admin preset`, and what each preset sets (docs/PRESETS.md).
#
# Fresh repositories (git init, nothing else), one per preset. Each claim is
# paired with an observed refusal:
#
#   1. each preset writes [preset] and [roles] with its defaults (ticks,
#      signing, the PR gate, the roles table), read back by `war admin
#      preset --json`; team and regulated also write the signing examples,
#      vibe none. Refused: plain `war init` writes no preset table at all,
#      and two preset flags at once are refused by the parser.
#   2. `war admin preset regulated` switches a team repository and keeps a
#      person's [roles] edit and every other byte. Refused: an unknown name
#      (preset.unknown) writes nothing.
#   3. the regulated preset's tick floor: a plain `war done` is refused
#      (ticket.tick-below-minimum, naming the preset); under team the same
#      done ticks as claimed.
#   4. a wrong value in [preset] is refused by `war check` (preset.config),
#      naming the key; the file as the preset wrote it checks clean.

echo "== repo presets (M14) =="
PS_T=$(mktemp -d)
PS_WAR="$REPO_ROOT/${WAR#./}"
ps_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ps_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
# A newcomer's shell: no terminal, no inherited actor, no key.
psw() { local d="$1"; shift; (cd "$d" && env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR \
    OPENWARRANT_NO_PROJECTS=1 "$PS_WAR" "$@" </dev/null); }
ps_py() { python3 -c "import json, sys; v = json.load(sys.stdin); print($1)" 2>/dev/null; }
ps_repo() { mkdir -p "$PS_T/$1" && git -C "$PS_T/$1" init -q . ; }

# ---- 1. each preset writes its defaults ---------------------------------------
PS_BAD=""
for p in vibe team regulated; do
    ps_repo "$p"
    psw "$PS_T/$p" init "--$p" >"$PS_T/$p.out" 2>&1 || PS_BAD="$PS_BAD $p(init exit $?)"
    got=$(psw "$PS_T/$p" --json admin preset | ps_py \
        '"%s %s %s %s %s" % (v["result"]["preset"], v["result"]["ticks"], v["result"]["signing"], v["result"]["pr_requires_official"], ",".join(r + "=" + k for r, k in sorted(v["result"]["roles"].items())))')
    case "$p" in
        vibe) want="vibe claimed never False admin=vibe,maintain=vibe,none=formal,read=formal,triage=formal,write=tested" ;;
        team) want="team claimed release True admin=vibe,maintain=vibe,none=formal,read=formal,triage=formal,write=tested" ;;
        regulated) want="regulated observed merge True admin=tested,maintain=tested,none=formal,read=formal,triage=formal,write=formal" ;;
    esac
    [[ "$got" == "$want" ]] || PS_BAD="$PS_BAD $p(read back '$got')"
    examples=$(ls "$PS_T/$p/docs/authority" 2>/dev/null | tr '\n' ' ')
    case "$p" in
        vibe) [[ -z "$examples" ]] || PS_BAD="$PS_BAD vibe(wrote '$examples')" ;;
        *) [[ "$examples" == "allowed_signers.example roles.toml.example " ]] || PS_BAD="$PS_BAD $p(examples '$examples')" ;;
    esac
    [[ ! -e "$PS_T/$p/docs/authority/roles.toml" ]] || PS_BAD="$PS_BAD $p(wrote a real roles.toml)"
done
if [[ -z "$PS_BAD" ]] && grep -qF 'preset vibe:' "$PS_T/vibe.out" \
    && grep -qF 'signing (merge):' "$PS_T/regulated.out"; then
    ps_ok "each preset writes its defaults" "vibe, team, regulated: [preset] and [roles] read back; signing examples for team and regulated only"
else
    ps_fail "each preset writes its defaults" "${PS_BAD:-init lines missing}"
fi

ps_repo plain
psw "$PS_T/plain" init >/dev/null 2>&1; PS_PLAIN_RC=$?
PS_PLAIN_HAS=$(grep -cE '^\[(preset|roles|notify)\]' "$PS_T/plain/openwarrant.toml")
PS_PLAIN=$(psw "$PS_T/plain" --json admin preset | ps_py '"%s %s" % (v["result"]["preset"], v["result"]["roles_declared"])')
ps_repo both
PS_BOTH=$(psw "$PS_T/both" init --vibe --team 2>&1); PS_BOTH_RC=$?
if [[ $PS_PLAIN_RC -eq 0 && "$PS_PLAIN_HAS" == "0" && "$PS_PLAIN" == "None False" \
    && $PS_BOTH_RC -eq 2 && ! -e "$PS_T/both/openwarrant.toml" ]] \
    && grep -qF "cannot be used with" <<<"$PS_BOTH"; then
    ps_ok "plain init writes no preset" "no [preset], [roles] or [notify]; --vibe --team refused, nothing written"
else
    ps_fail "plain init writes no preset" "plain exit $PS_PLAIN_RC tables $PS_PLAIN_HAS ($PS_PLAIN); both exit $PS_BOTH_RC: $(head -1 <<<"$PS_BOTH")"
fi

# ---- 2. switching keeps a person's edits ----------------------------------------
sed -i 's/^write = "tested"$/write = "vibe"/' "$PS_T/team/openwarrant.toml"
printf '\n# kept by hand\n[tickets]\ncompact_after_days = 3\n' >> "$PS_T/team/openwarrant.toml"
PS_SW=$(psw "$PS_T/team" admin preset regulated 2>&1); PS_SW_RC=$?
PS_AFTER=$(psw "$PS_T/team" --json admin preset | ps_py '"%s %s %s" % (v["result"]["preset"], v["result"]["roles"]["write"], v["result"]["signing"])')
PS_PRESETS=$(grep -c '^\[preset\]$' "$PS_T/team/openwarrant.toml")
PS_SUM_BEFORE=$(sha256sum "$PS_T/team/openwarrant.toml" | cut -d' ' -f1)
PS_UNK=$(psw "$PS_T/team" admin preset strict 2>&1); PS_UNK_RC=$?
PS_SUM_AFTER=$(sha256sum "$PS_T/team/openwarrant.toml" | cut -d' ' -f1)
if [[ $PS_SW_RC -eq 0 && "$PS_AFTER" == "regulated vibe merge" && "$PS_PRESETS" == "1" ]] \
    && grep -qF 'preset team -> regulated; your [roles] kept' <<<"$PS_SW" \
    && [[ "$(cat "$PS_T/team/openwarrant.toml")" == *$'# kept by hand\n[tickets]'* ]]; then
    ps_ok "admin preset switches, keeps edits" "team -> regulated; the hand-set write = vibe and [tickets] kept"
else
    ps_fail "admin preset switches, keeps edits" "exit $PS_SW_RC, read back '$PS_AFTER', [preset] x$PS_PRESETS: $(head -1 <<<"$PS_SW")"
fi
if [[ $PS_UNK_RC -eq 2 && "$PS_SUM_BEFORE" == "$PS_SUM_AFTER" ]] && grep -qF 'preset.unknown' <<<"$PS_UNK"; then
    ps_ok "an unknown preset is refused" "preset.unknown; openwarrant.toml unchanged"
else
    ps_fail "an unknown preset is refused" "exit $PS_UNK_RC, file moved: $([[ "$PS_SUM_BEFORE" == "$PS_SUM_AFTER" ]] && echo no || echo yes): $PS_UNK"
fi

# ---- 3. the regulated tick floor ----------------------------------------------------
ps_done() { # dir -> "exit|output"
    local id out rc
    id=$(psw "$1" --json create "Guard the empty password" | ps_py 'v["result"]["id"]')
    psw "$1" claim "$id" >/dev/null 2>&1
    out=$(psw "$1" done "$id" 2>&1); rc=$?
    printf '%s|%s' "$rc" "$out"
}
PS_REG=$(ps_done "$PS_T/regulated")
PS_TEAM_T="$PS_T/team2"; ps_repo team2; psw "$PS_TEAM_T" init --team >/dev/null 2>&1
PS_TEAM=$(ps_done "$PS_TEAM_T")
if [[ "${PS_REG%%|*}" == "2" ]] && grep -qF 'ticket.tick-below-minimum' <<<"$PS_REG" \
    && grep -qF "the repo preset's [preset] ticks minimum" <<<"$PS_REG" \
    && grep -qF 'war done' <<<"$PS_REG"; then
    ps_ok "regulated refuses a claimed tick" "ticket.tick-below-minimum, naming the preset and \`--check\`"
else
    ps_fail "regulated refuses a claimed tick" "${PS_REG:0:300}"
fi
if [[ "${PS_TEAM%%|*}" == "0" ]] && grep -qF 'ticked as claimed' <<<"$PS_TEAM"; then
    ps_ok "team ticks the same done" "claimed, as before presets"
else
    ps_fail "team ticks the same done" "${PS_TEAM:0:300}"
fi

# ---- 4. a wrong value is refused by war check ----------------------------------------
PS_CLEAN=$(psw "$PS_T/vibe" check 2>&1); PS_CLEAN_RC=$?
sed -i 's/^signing = "never"$/signing = "sometimes"/' "$PS_T/vibe/openwarrant.toml"
PS_WRONG=$(psw "$PS_T/vibe" check 2>&1); PS_WRONG_RC=$?
if [[ $PS_CLEAN_RC -eq 0 && $PS_WRONG_RC -eq 2 ]] && grep -qF 'preset.config' <<<"$PS_WRONG" \
    && grep -qF '[preset] signing = "sometimes"' <<<"$PS_WRONG" \
    && ! grep -qF 'preset.config' <<<"$PS_CLEAN"; then
    ps_ok "a wrong [preset] value is refused" "war check: preset.config names [preset] signing; the written file checks clean"
else
    ps_fail "a wrong [preset] value is refused" "clean exit $PS_CLEAN_RC, wrong exit $PS_WRONG_RC: $(grep -m1 preset <<<"$PS_WRONG")"
fi

unset -f ps_ok ps_fail psw ps_py ps_repo ps_done
command rm -rf "$PS_T"
