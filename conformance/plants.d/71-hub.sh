# shellcheck shell=bash
# OW-WAR-0115 — the hub: `war` from anywhere, every project, each read
# through that project's own records.
#
# The list lives outside every repository, so every plant here points
# XDG_CONFIG_HOME at a temporary directory and opts back in: the battery as a
# whole runs with OPENWARRANT_NO_PROJECTS=1 (lib.sh), so scratch corpora never
# land in the owner's own list.

echo "== the hub (OW-WAR-0115) =="
# Absolute: the pty and no-terminal plants run from a directory outside
# every repository, where a relative ./target/debug/war does not exist.
HUB_WAR=$(realpath "$WAR")
HUB_CFG=$(mktemp -d)
HUB_A=$(scratch_corpus HA)
HUB_B=$(scratch_corpus HB)
[[ -d "${HUB_A:-}/.git" && -d "${HUB_B:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpora (run through conformance/plant.sh)\n' >&2; exit 9; }
hub() { XDG_CONFIG_HOME="$HUB_CFG" OPENWARRANT_NO_PROJECTS= "$WAR" "$@"; }
hub_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
hub_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }

# OBL-001: remembered on use. Nobody registers a project.
hub --root "$HUB_A" status >/dev/null 2>&1
hub --root "$HUB_B" status >/dev/null 2>&1
HUB_JSON=$(hub projects --json 2>/dev/null)
if python3 -c '
import sys, json, os
d = json.load(sys.stdin)
roots = {p["root"] for p in d["result"]["projects"]}
want = {os.path.realpath(a) for a in sys.argv[1:]}
assert want <= roots, (want, roots)
' "$HUB_A" "$HUB_B" <<<"$HUB_JSON" 2>/dev/null; then
    hub_ok "used projects are remembered" "war status in two repositories lists both"
else
    hub_fail "used projects are remembered" "$(head -c 200 <<<"$HUB_JSON")"
fi

# OBL-002: each row's pending count is that project's own `war sign --list`.
HUB_LISTED=$("$WAR" --root "$HUB_A" sign --list 2>/dev/null | sed -n 's/^\([0-9][0-9]*\) awaiting a signature:.*/\1/p')
HUB_ROW=$(python3 -c '
import sys, json, os
d = json.load(sys.stdin)
a = os.path.realpath(sys.argv[1])
print(next(p.get("pending") for p in d["result"]["projects"] if p["root"] == a))
' "$HUB_A" <<<"$HUB_JSON" 2>/dev/null)
if [[ -n "$HUB_LISTED" && "$HUB_ROW" == "$HUB_LISTED" ]]; then
    hub_ok "a row's pending is sign --list's" "$HUB_ROW == $HUB_LISTED"
else
    hub_fail "a row's pending is sign --list's" "row ${HUB_ROW:-none}, sign --list ${HUB_LISTED:-none}"
fi

# A deleted repository is reported missing, not dropped.
HUB_B_REAL=$(realpath "$HUB_B")
command rm -rf "$HUB_B"
HUB_OUT=$(hub projects 2>&1)
if grep -q 'projects.missing' <<<"$HUB_OUT" && grep -q "$HUB_B_REAL" <<<"$HUB_OUT"; then
    hub_ok "a moved project is not forgotten" "listed as missing, by name"
else
    hub_fail "a moved project is not forgotten" "$(tail -2 <<<"$HUB_OUT" | tr '\n' '|')"
fi

# --forget removes exactly that one.
hub projects --forget "$HUB_B_REAL" >/dev/null 2>&1
HUB_JSON=$(hub projects --json 2>/dev/null)
if python3 -c '
import sys, json, os
d = json.load(sys.stdin)
roots = {p["root"] for p in d["result"]["projects"]}
assert sys.argv[2] not in roots and os.path.realpath(sys.argv[1]) in roots
' "$HUB_A" "$HUB_B_REAL" <<<"$HUB_JSON" 2>/dev/null; then
    hub_ok "--forget removes exactly one" "the other stays"
else
    hub_fail "--forget removes exactly one" "$(head -c 200 <<<"$HUB_JSON")"
fi

# The refusal: forgetting what is not listed says so by name.
HUB_OUT=$(hub projects --forget /nonexistent/hub-plant 2>&1); HUB_STATUS=$?
if [[ $HUB_STATUS -ne 0 ]] && grep -q 'projects.unknown' <<<"$HUB_OUT"; then
    hub_ok "forgetting an unlisted path" "projects.unknown (exit $HUB_STATUS)"
else
    hub_fail "forgetting an unlisted path" "exit $HUB_STATUS: $(tail -1 <<<"$HUB_OUT")"
fi

# The opt-out writes nothing.
HUB_CFG2=$(mktemp -d)
XDG_CONFIG_HOME="$HUB_CFG2" OPENWARRANT_NO_PROJECTS=1 "$WAR" --root "$HUB_A" status >/dev/null 2>&1
if [[ ! -e "$HUB_CFG2/openwarrant/projects.toml" ]]; then
    hub_ok "the opt-out writes nothing" "no projects.toml"
else
    hub_fail "the opt-out writes nothing" "projects.toml was written"
fi

# An unwritable list never changes a command's exit.
HUB_CFG3=$(mktemp -d)
printf 'not a directory\n' > "$HUB_CFG3/openwarrant"
"$WAR" --root "$HUB_A" status >/dev/null 2>&1; HUB_WANT=$?
XDG_CONFIG_HOME="$HUB_CFG3" OPENWARRANT_NO_PROJECTS= "$WAR" --root "$HUB_A" status >/dev/null 2>&1; HUB_GOT=$?
if [[ $HUB_WANT -eq $HUB_GOT ]]; then
    hub_ok "an unwritable list changes nothing" "exit $HUB_GOT either way"
else
    hub_fail "an unwritable list changes nothing" "exit $HUB_WANT with a list, $HUB_GOT without"
fi

# OBL-002: outside any repository, at a pty, `war` opens the Projects pane.
if command -v script >/dev/null 2>&1; then
    HUB_NOWHERE=$(mktemp -d)
    # A pty with no size draws nothing; ratatui writes cell by cell between
    # escape codes, so the screen is read with the escapes removed.
    HUB_PTY=$( (sleep 3; printf q) | XDG_CONFIG_HOME="$HUB_CFG" OPENWARRANT_NO_PROJECTS= timeout 20 \
        script -qec "stty cols 200 rows 40; cd '$HUB_NOWHERE' && '$HUB_WAR'" /dev/null 2>&1 \
        | python3 -c 'import re, sys; s = sys.stdin.buffer.read().decode("utf8", "replace"); print(re.sub(r" +", " ", re.sub(r"\x1b\[[0-9;?]*[A-Za-z]", " ", s)))')
    if grep -q 'Projects — every repository' <<<"$HUB_PTY" && grep -q "$(realpath "$HUB_A")" <<<"$HUB_PTY"; then
        hub_ok "war from anywhere opens the hub" "the Projects pane, listing the scratch project"
    else
        hub_fail "war from anywhere opens the hub" "$(tr -d '\r' <<<"$HUB_PTY" | tail -c 200 | tr '\n' ' ')"
    fi
    command rm -rf "$HUB_NOWHERE"
else
    hub_ok "war from anywhere opens the hub" "(script(1) absent; the pty plant did not run)"
fi

# OBL-002: the Help pane's first row names the running binary's version and
# path. The pty stream is replayed onto a 300x40 grid (ratatui redraws only
# the cells that change, so the raw stream is not the screen), and the row
# under the Help title must carry `war --version`'s line and this binary's
# canonical path.
if command -v script >/dev/null 2>&1; then
    HUB_VER=$("$HUB_WAR" --version 2>/dev/null | head -1)
    HUB_SCREEN=$( (sleep 4; printf 2; sleep 3; printf q) | XDG_CONFIG_HOME="$HUB_CFG" OPENWARRANT_NO_PROJECTS= timeout 30 \
        script -qec "stty cols 300 rows 40; cd '$HUB_A' && '$HUB_WAR'" /dev/null 2>/dev/null \
        | python3 -c '
import re, sys
s = sys.stdin.buffer.read().decode("utf8", "replace")
R, C = 40, 300
scr = [[" "] * C for _ in range(R)]
r = c = 0
i = 0
tok = re.compile(r"\x1b\[([0-9;?]*)([A-Za-z@])|\x1b[()][A-Za-z0-9]|\x1b[=>78]|\x1b\][^\x07]*\x07")
while i < len(s):
    m = tok.match(s, i)
    if m:
        a, f = m.group(1), m.group(2)
        if f:
            n = [int(x) if x.isdigit() else 0 for x in a.replace("?", "").split(";")] if a else []
            if f in "Hf":
                r = (n[0] if n and n[0] else 1) - 1
                c = (n[1] if len(n) > 1 and n[1] else 1) - 1
            elif f == "J" and "?" not in a:
                scr = [[" "] * C for _ in range(R)]
            elif f == "K" and 0 <= r < R:
                for x in range(max(c, 0), C):
                    scr[r][x] = " "
            elif f == "C":
                c += n[0] if n and n[0] else 1
        i = m.end()
        continue
    ch = s[i]
    i += 1
    if ch == "\r":
        c = 0
    elif ch == "\n":
        r = min(r + 1, R - 1)
    elif ch >= " " and 0 <= r < R and 0 <= c < C:
        scr[r][c] = ch
        c += 1
print("\n".join("".join(l).rstrip() for l in scr))
')
    HUB_HELP_ROW=$(awk '/Help — what next/ { getline; print; exit }' <<<"$HUB_SCREEN")
    if [[ -n "$HUB_VER" && "$HUB_HELP_ROW" == *"$HUB_VER at $HUB_WAR"* ]]; then
        hub_ok "Help's first row is this binary" "$(sed 's/^[│▶ ]*//; s/ *│$//' <<<"$HUB_HELP_ROW" | cut -c1-160)"
    else
        hub_fail "Help's first row is this binary" "wanted '$HUB_VER at $HUB_WAR'; row: ${HUB_HELP_ROW:-none}"
    fi
else
    printf 'UNKNOWN %-32s %s\n' "Help's first row is this binary" "(script(1) absent; not run, not passed)"
fi

# No terminal: still a refusal by name, from anywhere.
HUB_NOWHERE=$(mktemp -d)
HUB_OUT=$(cd "$HUB_NOWHERE" && XDG_CONFIG_HOME="$HUB_CFG" "$HUB_WAR" </dev/null 2>&1; echo "exit=$?")
if grep -q 'exit=2' <<<"$HUB_OUT" && grep -q 'tui.no-tty' <<<"$HUB_OUT"; then
    hub_ok "no terminal, no hub" "exit 2, tui.no-tty"
else
    hub_fail "no terminal, no hub" "$(tail -1 <<<"$HUB_OUT")"
fi
command rm -rf "$HUB_NOWHERE"

# Every child `war` the app starts names its project: each
# `Command::new(exe)` under tui/ passes `--root` on the next line.
HUB_CHILDREN=$(grep -rn -A1 'Command::new(exe)' crates/openwarrant-cli/src/tui | grep -c 'Command::new(exe)')
HUB_ROOTED=$(grep -rn -A1 'Command::new(exe)' crates/openwarrant-cli/src/tui | grep -c '"--root"')
if [[ "$HUB_CHILDREN" -ge 2 && "$HUB_CHILDREN" -eq "$HUB_ROOTED" ]]; then
    hub_ok "every child names its project" "$HUB_CHILDREN child(ren), each with --root"
else
    hub_fail "every child names its project" "$HUB_CHILDREN child(ren), $HUB_ROOTED with --root"
fi

command rm -rf "$HUB_CFG" "$HUB_CFG2" "$HUB_CFG3"
corpus_gone "$HUB_A"
unset HUB_WAR HUB_A HUB_B HUB_B_REAL HUB_CFG HUB_CFG2 HUB_CFG3 HUB_JSON HUB_OUT
