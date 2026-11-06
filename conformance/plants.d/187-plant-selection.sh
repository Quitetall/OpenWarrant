# shellcheck shell=bash
# Which plants run where: the shards of the checkpoint and the affected set of
# Fast CI (conformance/plant-select.py, conformance/plant-map.toml).
#
# Claims, each paired with its refusal:
#   - every plant is in exactly one shard, at every width; a report set that
#     drops or doubles a plant, or misses a shard, is refused by name
#   - shard totals add up, with lib.sh's corpus check counted once
#   - a changed file selects the plants mapped to it, and a broken plant that
#     is selected fails the run; one that is not selected does not run
#   - what the map cannot answer runs more: an unmapped path and an unreadable
#     base select everything, and a plant no rule names always runs
#   - the cap names each plant it leaves out
#   - a selection the runner cannot honour is refused; none reaches the gate
#     whose receipt says "the battery"
#
# Run against a scratch program with its own conformance/, two plants of which
# one is broken on purpose: the real battery inside the battery would recurse.

echo "== plant selection: shards and the affected set =="
PS_TMP=$(mktemp -d)
PS_ROOT=$(scratch_corpus PS)
[[ -d "${PS_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
ps_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ps_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
PS_REAL=(python3 "$REPO_ROOT/conformance/plant-select.py")
PS_SEL=(python3 "$PS_ROOT/conformance/plant-select.py")

# ---- the real battery ------------------------------------------------------

mapfile -t PS_ALL < <(plant_files_in "$REPO_ROOT/conformance/plants.d" | xargs -n1 basename)
PS_BAD=""
for n in 1 2 4 6 7 ${#PS_ALL[@]} $((${#PS_ALL[@]} + 3)); do
    : > "$PS_TMP/assigned"
    for ((i = 1; i <= n; i++)); do "${PS_REAL[@]}" shard "$i/$n" >> "$PS_TMP/assigned"; done
    # Read back as lines, not through the partition: this is the claim itself.
    PS_DIFF=$(diff <(printf '%s\n' "${PS_ALL[@]}") <(LC_ALL=C sort "$PS_TMP/assigned"))
    [[ -z "$PS_DIFF" ]] || PS_BAD="$PS_BAD $n: $(head -3 <<<"$PS_DIFF" | tr '\n' ' ')"
done
if [[ -z "$PS_BAD" ]]; then
    ps_ok "each plant in exactly one shard" "${#PS_ALL[@]} files at seven widths"
else
    ps_fail "each plant in exactly one shard" "$PS_BAD"
fi

if PS_OUT=$("${PS_REAL[@]}" check-map 2>&1); then
    ps_ok "the map names only real things" "$PS_OUT"
else
    ps_fail "the map names only real things" "$(head -3 <<<"$PS_OUT" | tr '\n' '|')"
fi

# The rule that says no plant reads a path is a claim about the plants' text.
PS_NOREAD=$(python3 - "$REPO_ROOT" <<'PY'
import pathlib, sys, tomllib
root = pathlib.Path(sys.argv[1])
rules = tomllib.loads((root / "conformance/plant-map.toml").read_text())["rule"]
plants = {p.name: p.read_text(errors="replace") for p in (root / "conformance/plants.d").glob("*.sh")}
plants.pop("187-plant-selection.sh", None)
for rule in rules:
    if rule["plants"]:
        continue
    for pattern in rule["paths"]:
        literal = pattern.split("*")[0].rstrip("/")
        for name, text in plants.items():
            if literal in text:
                print(f"{name} names {literal}, which rule {rule['name']!r} says no plant reads")
PY
)
if [[ -z "$PS_NOREAD" ]]; then
    ps_ok "no plant reads what the map skips" "the plants=[] paths appear in no plant"
else
    ps_fail "no plant reads what the map skips" "$(head -2 <<<"$PS_NOREAD" | tr '\n' '|')"
fi

# The workflows agree with themselves: a matrix of four closed with another width
# is a run that can never pass, and a job the aggregator does not read protects
# nothing while looking configured.
PS_CI="$REPO_ROOT/.github/workflows/ci.yml"
PS_FAST="$REPO_ROOT/.github/workflows/fast.yml"
ps_count() { grep -cE -- "$1" "$2"; }
PS_WIRED="$(ps_count '^ +shard: \[1, 2, 3, 4\]$' "$PS_CI") $(ps_count 'OW_PLANT_SHARD: \$\{\{ matrix\.shard \}\}/4$' "$PS_CI")"
PS_WIRED="$PS_WIRED $(ps_count 'plant-select\.py reports .* --of 4( |$)' "$PS_CI") $(ps_count 'needs\.plants-total\.result \}\}" = success' "$PS_CI")"
PS_WIRED="$PS_WIRED $(ps_count '^ +shard: \[1, 2, 3, 4\]$' "$PS_FAST") $(ps_count '--shard "\$SHARD/4"' "$PS_FAST")"
PS_WIRED="$PS_WIRED $(ps_count 'needs\.plants\.result \}\}" = success' "$PS_FAST") $(ps_count 'needs: \[.*\bplants\b.*\]$' "$PS_FAST")"
if [[ "$PS_WIRED" == "1 1 1 1 1 1 1 1" ]]; then
    ps_ok "the workflows agree on the shards" "four legs each, closed and required"
else
    ps_fail "the workflows agree on the shards" "wanted eight 1s, have: $PS_WIRED"
fi

# ---- a scratch battery -----------------------------------------------------

mkdir -p "$PS_ROOT/conformance/plants.d" "$PS_ROOT/crates" "$PS_ROOT/docs"
command cp conformance/lib.sh conformance/plant.sh conformance/plant-select.py conformance/plant-isolated.sh "$PS_ROOT/conformance/"
ln -s "$(realpath target)" "$PS_ROOT/target"
printf 'target\n' >> "$PS_ROOT/.git/info/exclude"
cat > "$PS_ROOT/conformance/plants.d/10-green.sh" <<'P'
printf 'ok    %-34s %s\n' "green" "ran"; PASSED=$((PASSED + 1))
P
cat > "$PS_ROOT/conformance/plants.d/20-broken.sh" <<'P'
printf 'FAIL  %-34s %s\n' "broken" "planted"; FAILED=$((FAILED + 1))
P
cat > "$PS_ROOT/conformance/plants.d/30-unmapped.sh" <<'P'
printf 'ok    %-34s %s\n' "unmapped" "ran"; PASSED=$((PASSED + 1))
P
cat > "$PS_ROOT/conformance/plant-map.toml" <<'M'
[[rule]]
name = "green"
paths = ["crates/green.rs"]
plants = ["10-green.sh"]
[[rule]]
name = "broken"
paths = ["crates/broken.rs"]
plants = ["20-broken.sh"]
[[rule]]
name = "everything"
paths = ["crates/core.rs"]
plants = ["*"]
[[rule]]
name = "nothing"
paths = ["docs/prose.md"]
plants = []
[[rule]]
name = "dead"
paths = ["crates/dead.rs"]
plants = ["99-gone.sh"]
M
printf '100\t10-green.sh\n100\t20-broken.sh\n100\t30-unmapped.sh\n' > "$PS_ROOT/conformance/plant-weights.tsv"
for f in green broken core dead other; do printf 'one\n' > "$PS_ROOT/crates/$f.rs"; done
printf 'prose\n' > "$PS_ROOT/docs/prose.md"
git -C "$PS_ROOT" add -A >/dev/null 2>&1
git -C "$PS_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "a battery with one broken plant" >/dev/null 2>&1
"$WAR" --root "$PS_ROOT" compile >/dev/null 2>&1
git -C "$PS_ROOT" add -A >/dev/null 2>&1
git -C "$PS_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "compiled" >/dev/null 2>&1
PS_BASE=$(git -C "$PS_ROOT" rev-parse HEAD)

# ps_change <path...> -> commits a change to each path
ps_change() {
    git -C "$PS_ROOT" reset -q --hard "$PS_BASE"
    local p
    for p in "$@"; do printf 'two\n' >> "$PS_ROOT/$p"; done
    git -C "$PS_ROOT" add -A >/dev/null 2>&1
    git -C "$PS_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "a change" >/dev/null 2>&1
}
# ps_select <base> [cap] -> sets PS_LOG (what it printed) and PS_LIST (what leg 1/1 runs)
ps_select() {
    PS_LOG=$(env -u GITHUB_ACTIONS -u GITHUB_STEP_SUMMARY "${PS_SEL[@]}" affected --base "$1" --shard 1/1 --cap "${2:-1000}" --out "$PS_TMP/list" 2>&1)
    PS_STATUS=$?
    PS_LIST=$(tr '\n' ' ' < "$PS_TMP/list")
}
# ps_battery -> runs the scratch battery on the last list
ps_battery() {
    PS_RUN=$(env -u OW_PLANT_SHARD -u OW_PLANT_REPORT "OW_PLANT_FILES=$PS_TMP/list" bash "$PS_ROOT/conformance/plant.sh" 2>&1)
    PS_RC=$?
}

ps_change crates/broken.rs
ps_select "$PS_BASE"
if [[ "$PS_LIST" == "20-broken.sh 30-unmapped.sh " ]] && grep -q 'rule broken' <<<"$PS_LOG"; then
    ps_ok "a changed file selects its plant" "$PS_LIST"
else
    ps_fail "a changed file selects its plant" "list: $PS_LIST status $PS_STATUS"
fi
ps_battery
if [[ "$PS_RC" -ne 0 ]] && grep -q '^FAIL  broken' <<<"$PS_RUN"; then
    ps_ok "a selected broken plant fails" "exit $PS_RC with its FAIL line"
else
    ps_fail "a selected broken plant fails" "exit $PS_RC: $(tail -3 <<<"$PS_RUN" | tr '\n' '|')"
fi

ps_change crates/green.rs
ps_select "$PS_BASE"
ps_battery
if [[ "$PS_LIST" == "10-green.sh 30-unmapped.sh " && "$PS_RC" -eq 0 ]] && ! grep -q 'broken' <<<"$PS_RUN"; then
    ps_ok "an unselected plant does not run" "$PS_LIST: the broken plant stayed out, exit 0"
else
    ps_fail "an unselected plant does not run" "list: $PS_LIST exit $PS_RC"
fi

ps_change docs/prose.md
ps_select "$PS_BASE"
if [[ "$PS_LIST" == "30-unmapped.sh " ]]; then
    ps_ok "a plant with no rule always runs" "a change no plant reads still runs 30-unmapped.sh"
else
    ps_fail "a plant with no rule always runs" "list: $PS_LIST"
fi

ps_change crates/other.rs
ps_select "$PS_BASE"
ps_battery
if [[ "$PS_LIST" == "10-green.sh 20-broken.sh 30-unmapped.sh " ]] \
    && grep -q '^  UNMAPPED  crates/other.rs' <<<"$PS_LOG" && grep -q 'WARNING: crates/other.rs' <<<"$PS_LOG" \
    && [[ "$PS_RC" -ne 0 ]]; then
    ps_ok "an unmapped path runs everything" "named in the log, and the broken plant fails the run"
else
    ps_fail "an unmapped path runs everything" "list: $PS_LIST exit $PS_RC: $(head -3 <<<"$PS_LOG" | tr '\n' '|')"
fi

ps_change crates/green.rs
ps_select no-such-base
if [[ "$PS_LIST" == "10-green.sh 20-broken.sh 30-unmapped.sh " ]] && grep -q 'could not be read' <<<"$PS_LOG"; then
    ps_ok "an unreadable base runs everything" "named in the log"
else
    ps_fail "an unreadable base runs everything" "list: $PS_LIST: $(head -2 <<<"$PS_LOG" | tr '\n' '|')"
fi

ps_change crates/dead.rs
ps_select "$PS_BASE"
if [[ "$PS_LIST" == "10-green.sh 20-broken.sh 30-unmapped.sh " ]] && grep -q "'99-gone.sh' names no plant" <<<"$PS_LOG"; then
    ps_ok "a dead rule runs everything" "named in the log"
else
    ps_fail "a dead rule runs everything" "list: $PS_LIST"
fi
if PS_OUT=$("${PS_SEL[@]}" check-map 2>&1); then
    ps_fail "check-map refuses a dead rule" "it passed: $PS_OUT"
elif grep -q "99-gone.sh" <<<"$PS_OUT"; then
    ps_ok "check-map refuses a dead rule" "named 99-gone.sh"
else
    ps_fail "check-map refuses a dead rule" "$(head -2 <<<"$PS_OUT" | tr '\n' '|')"
fi

ps_change crates/core.rs
ps_select "$PS_BASE" 150
if [[ "$(wc -w <<<"$PS_LIST")" -eq 1 ]] && [[ "$(grep -c '^  NOT RUN ' <<<"$PS_LOG")" -eq 2 ]] \
    && grep -q 'unknown here, the full gate runs it' <<<"$PS_LOG"; then
    ps_ok "the cap names what it leaves out" "one ran; the two left out are NOT RUN, by name"
else
    ps_fail "the cap names what it leaves out" "list: $PS_LIST: $(tail -4 <<<"$PS_LOG" | tr '\n' '|')"
fi

# ---- the runner refuses what it cannot honour ------------------------------

git -C "$PS_ROOT" reset -q --hard "$PS_BASE"
# ps_refused <label> <why-pattern> <env...>
ps_refused() {
    local label=$1 why=$2
    shift 2
    PS_OUT=$(env -u OW_PLANT_SHARD -u OW_PLANT_FILES -u OW_PLANT_REPORT "$@" bash "$PS_ROOT/conformance/plant.sh" 2>&1)
    PS_RC=$?
    if [[ "$PS_RC" -ne 0 ]] && grep -q -- "$why" <<<"$PS_OUT" && ! grep -q '^FAIL  broken' <<<"$PS_OUT"; then
        ps_ok "$label" "refused before any plant ran"
    else
        ps_fail "$label" "exit $PS_RC: $(tail -2 <<<"$PS_OUT" | tr '\n' '|')"
    fi
}
printf 'no-such-plant.sh\n' > "$PS_TMP/typo"
ps_refused "a malformed shard is refused" 'must be i/n' OW_PLANT_SHARD=5/4
ps_refused "both selectors are refused" 'set one' OW_PLANT_SHARD=1/2 "OW_PLANT_FILES=$PS_TMP/typo"
ps_refused "a name no plant has is refused" 'not a plant file' "OW_PLANT_FILES=$PS_TMP/typo"
ps_refused "an unreadable list is refused" 'cannot be read' "OW_PLANT_FILES=$PS_TMP/absent"

# ---- reports: every plant once, and one total -------------------------------

mkdir -p "$PS_TMP/reports"
PS_RC=0
for i in 1 2; do
    env -u OW_PLANT_FILES "OW_PLANT_SHARD=$i/2" "OW_PLANT_REPORT=$PS_TMP/reports/$i.report" \
        bash "$PS_ROOT/conformance/plant.sh" >/dev/null 2>&1
done
# Passing --of 2 and a failed plant: the check prints the total and exits 1.
PS_OUT=$("${PS_SEL[@]}" reports "$PS_TMP/reports" --of 2 2>&1)
PS_RC=$?
if [[ "$PS_RC" -eq 1 ]] && grep -qx '3 passed, 1 failed' <<<"$PS_OUT"; then
    ps_ok "shards add up to one total" "two shards, the corpus check counted once: 3 passed, 1 failed"
else
    ps_fail "shards add up to one total" "exit $PS_RC: $(tr '\n' '|' <<<"$PS_OUT")"
fi
# ps_reports <label> <why-pattern> -> the check over $PS_TMP/r, wanted refused
ps_reports() {
    PS_OUT=$("${PS_SEL[@]}" reports "$PS_TMP/r" --of 2 2>&1)
    PS_RC=$?
    if [[ "$PS_RC" -eq 1 ]] && grep -q -- "REFUSED  .*$2" <<<"$PS_OUT"; then
        ps_ok "$1" "refused by name"
    else
        ps_fail "$1" "exit $PS_RC: $(tail -3 <<<"$PS_OUT" | tr '\n' '|')"
    fi
}
# Fix the broken plant's failure out of the tally, so each refusal below is
# about coverage and not about that FAIL.
ps_reset_r() {
    rm -rf "$PS_TMP/r"
    command cp -r "$PS_TMP/reports" "$PS_TMP/r"
    sed -i -E 's/^total ([0-9]+) 1$/total \1 0/' "$PS_TMP"/r/*.report
}
ps_reset_r
sed -i '/^file 10-green.sh /d' "$PS_TMP"/r/*.report
ps_reports "a plant in no shard is refused" '10-green.sh ran in no shard'
ps_reset_r
PS_DUP=$(grep -h '^file 10-green.sh ' "$PS_TMP"/r/*.report | head -1)
for f in "$PS_TMP"/r/*.report; do
    grep -q '^file 10-green.sh ' "$f" || sed -i "2a $PS_DUP" "$f"
done
ps_reports "a plant in two shards is refused" '10-green.sh ran in 2 shards'
ps_reset_r
rm -f "$PS_TMP/r/2.report"
ps_reports "a missing shard is refused" 'shard 2/2 sent no report'
ps_reset_r
sed -i '/^total /d' "$PS_TMP/r/1.report"
ps_reports "a shard that died is refused" '1.report: incomplete report'

# ---- the gate that says "the battery" ---------------------------------------

cat > "$PS_ROOT/conformance/fake-battery.sh" <<'FAKE'
#!/usr/bin/env bash
echo "selectors: ${OW_PLANT_SHARD:-unset} ${OW_PLANT_FILES:-unset} ${OW_PLANT_REPORT:-unset}"
FAKE
git -C "$PS_ROOT" add -A >/dev/null 2>&1
git -C "$PS_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "a fake battery" >/dev/null 2>&1
PS_OUT=$(env -u OPENWARRANT_IN_BATTERY OW_PLANT_SHARD=1/99 OW_PLANT_FILES=/nowhere OW_PLANT_REPORT=/nowhere \
    TMPDIR="$PS_TMP" OPENWARRANT_ISOLATED_PLANT_SH=conformance/fake-battery.sh \
    bash "$PS_ROOT/conformance/plant-isolated.sh" 2>&1)
if grep -qx 'selectors: unset unset unset' <<<"$PS_OUT"; then
    ps_ok "the battery gate drops selectors" "a selector in the caller's environment reaches no receipt"
else
    ps_fail "the battery gate drops selectors" "$(head -2 <<<"$PS_OUT" | tr '\n' '|')"
fi

corpus_gone "$PS_ROOT"
rm -rf "$PS_TMP"
