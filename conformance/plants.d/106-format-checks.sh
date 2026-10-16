# shellcheck shell=bash
# SPDX-License-Identifier: Apache-2.0
# Checks and triggers under FORMAT-1.0 §12, on a scratch program (FC);
# nothing in this repository moves.
#
# Accepted: a check whose command passes, run by `war check run`, ticks the
# items it evaluates at `observed`, the tick backed by the recorded run (no
# work.tick-unbacked); `war done` on an item a check evaluates with the
# default trigger (`done`) ticks it `observed` when the check passes; a
# commit touching a `paths:` glob fires its check through the installed
# post-commit hook.
# Refused, by name, each the other half of a claim above: the same check
# failing ticks nothing, makes it `failing` and reads check.failed; a check
# that cannot complete (a timeout) ticks nothing, is never failing, and reads
# check.unknown (Law 15: UNKNOWN is neither a pass nor a fail); `war done`
# whose check fails is refused with the document byte-for-byte as it was; a
# post-commit hook that is not OpenWarrant's is never overwritten
# (check.hook-exists).

echo "== checks and triggers: Format 1.0 check records (FORMAT-1.0 §12) =="
FC_ROOT=$(scratch_corpus FC)
[[ -d "${FC_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
FC_WAR="$REPO_ROOT/${WAR#./}"
fc_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
fc_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
fc_war() { env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK -u SSH_AGENT_PID "$FC_WAR" --root "$FC_ROOT" "$@" </dev/null; }
fc_commit() { git -C "$FC_ROOT" add -A >/dev/null 2>&1 && git -C "$FC_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "$1" >/dev/null 2>&1; }
# fc_ticket <alias> <uuid> <items> <records>: a Format 1.0 ticket.
fc_ticket() {
    mkdir -p "$FC_ROOT/docs/tickets"
    printf -- '---\nformat: openwarrant/1.0\nprofile: ticket\nalias: %s\nuuid: %s\nsummary: Plant checks %s.\n---\n# Plant checks %s\n\n## Checklist\n%b\n%b' \
        "$1" "$2" "$1" "$1" "$3" "$4" > "$FC_ROOT/docs/tickets/$1.md"
}
# fc_rules <json>: the non-pass rules of an envelope, space-separated.
fc_rules() { python3 -c '
import json, sys
try:
    v = json.loads(sys.argv[1])
except ValueError:
    print("not-an-envelope"); sys.exit()
print(" ".join(d["rule"] for d in v.get("diagnostics", []) if d["severity"] != "pass"))' "$1"; }
fc_doc() { cat "$FC_ROOT/docs/tickets/$1.md"; }

# --- a pass ticks observed; a fail ticks nothing ------------------------------------
fc_ticket t-fc01 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a0c01 "- [ ] reset endpoint (i-fc11)\n- [ ] template (i-fc12)\n" \
    "## CHK-flag · check — the flag is there\nrun: test -f flag.txt\non: manual\nevaluates i-fc11, i-fc12\n"
fc_commit "a ticket with a check"
FC_FAIL=$(fc_war --json check run t-fc01/CHK-flag --as ci 2>/dev/null)
FC_FAIL_RULES=$(fc_rules "$FC_FAIL")
FC_FAILING=$(fc_war --json model 2>/dev/null | python3 -c '
import json, sys
v = json.load(sys.stdin)["result"]
print(" ".join(s["value"] + "/" + s["provenance"] for s in v["states"] if s["record"] == "t-fc01/CHK-flag"))' 2>&1)
if [[ "$FC_FAIL_RULES" == *check.failed* && "$(fc_doc t-fc01)" != *"[x]"* && "$FC_FAILING" == "failing/recorded" ]]; then
    fc_ok "a fail ticks nothing" "check.failed; no box ticked; the check is failing (recorded)"
else
    fc_fail "a fail ticks nothing" "rules '$FC_FAIL_RULES', state '$FC_FAILING'"
fi
: > "$FC_ROOT/flag.txt"
FC_PASS=$(fc_war --json check run t-fc01/CHK-flag --as ci 2>/dev/null)
FC_OBSERVED=$(fc_doc t-fc01 | grep -c -- '— done by ci, .* \[observed\]$')
FC_CHECKED=$(fc_rules "$(fc_war --json check 2>/dev/null)")
if [[ -z "$(fc_rules "$FC_PASS")" && "$FC_OBSERVED" == 2 && "$FC_CHECKED" != *work.tick-unbacked* ]]; then
    fc_ok "a pass ticks observed" "both items [observed], backed by the recorded run (no work.tick-unbacked)"
else
    fc_fail "a pass ticks observed" "rules '$(fc_rules "$FC_PASS")', observed $FC_OBSERVED, check '$FC_CHECKED'"
fi

# --- UNKNOWN ticks nothing and is never failing ---------------------------------------
fc_ticket t-fc02 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a0c02 "- [ ] slow (i-fc21)\n" \
    "## CHK-slow · check — too slow\nrun: sleep 5\non: manual\ntimeout: 1\nevaluates i-fc21\n"
fc_commit "a ticket whose check times out"
FC_UNK=$(fc_war --json check run t-fc02 --as ci 2>/dev/null)
FC_UNK_RULES=$(fc_rules "$FC_UNK")
FC_UNK_STATE=$(fc_war --json model 2>/dev/null | python3 -c '
import json, sys
v = json.load(sys.stdin)["result"]
print(" ".join(s["value"] for s in v["states"] if s["record"] == "t-fc02/CHK-slow"))' 2>&1)
if [[ "$FC_UNK_RULES" == check.unknown && "$(fc_doc t-fc02)" != *"[x]"* && -z "$FC_UNK_STATE" ]]; then
    fc_ok "UNKNOWN ticks nothing" "check.unknown alone (no check.failed); nothing ticked; not failing"
else
    fc_fail "UNKNOWN ticks nothing" "rules '$FC_UNK_RULES', state '$FC_UNK_STATE'"
fi

# --- war done fires the default trigger -------------------------------------------------
fc_ticket t-fc03 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a0c03 "- [ ] gated (i-fc31)\n" \
    "## CHK-gate · check — the gate file is there\nrun: test -f gate.txt\nevaluates i-fc31\n"
fc_commit "a ticket whose check fires on done"
fc_war claim t-fc03/i-fc31 --as dev >/dev/null 2>&1
FC_DOC_BEFORE=$(sha256sum "$FC_ROOT/docs/tickets/t-fc03.md" | cut -c1-64)
FC_REFUSED=$(fc_war done t-fc03/i-fc31 --as dev 2>&1 >/dev/null)
FC_DOC_AFTER=$(sha256sum "$FC_ROOT/docs/tickets/t-fc03.md" | cut -c1-64)
: > "$FC_ROOT/gate.txt"
fc_war done t-fc03/i-fc31 --as dev >/dev/null 2>&1
if [[ "$FC_REFUSED" == *"refused (check.failed)"* && "$FC_DOC_BEFORE" == "$FC_DOC_AFTER" ]] \
    && fc_doc t-fc03 | grep -q -- '- \[x\] gated (i-fc31) — done by dev, .* \[observed\]$'; then
    fc_ok "done fires its check" "failing: refused check.failed, document unchanged; passing: [observed]"
else
    fc_fail "done fires its check" "refusal '$FC_REFUSED', unchanged $([[ "$FC_DOC_BEFORE" == "$FC_DOC_AFTER" ]] && echo yes || echo no)"
fi

# --- the commit trigger through the hook ------------------------------------------------
printf '#!/bin/sh\necho theirs\n' > "$FC_ROOT/.git/hooks/post-commit"
FC_HOOK=$(fc_rules "$(fc_war --json check install-hook 2>/dev/null)")
FC_THEIRS=$(cat "$FC_ROOT/.git/hooks/post-commit")
rm -f "$FC_ROOT/.git/hooks/post-commit"
fc_war check install-hook >/dev/null 2>&1
fc_ticket t-fc04 01a0e1c2-0b7d-7c11-8f2e-5d6c7b8a0c04 "- [ ] built (i-fc41)\n" \
    "## CHK-src · check — src is there\nrun: test -d src\non: paths:src/**\nevaluates i-fc41\n"
fc_commit "a ticket whose check fires on src"
FC_BEFORE_SRC=$(fc_doc t-fc04)
mkdir -p "$FC_ROOT/src" && : > "$FC_ROOT/src/main.rs"
fc_commit "src"
if [[ "$FC_HOOK" == check.hook-exists && "$FC_THEIRS" == *"echo theirs"* && "$FC_BEFORE_SRC" != *"[x]"* ]] \
    && fc_doc t-fc04 | grep -q -- '- \[x\] built (i-fc41) — done by .* \[observed\]$'; then
    fc_ok "the commit hook fires paths:" "a foreign hook kept (check.hook-exists); ours ticked i-fc41 [observed]"
else
    fc_fail "the commit hook fires paths:" "hook '$FC_HOOK', theirs '$FC_THEIRS'; $(fc_doc t-fc04 | grep i-fc41)"
fi
