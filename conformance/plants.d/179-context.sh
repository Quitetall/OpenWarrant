# shellcheck shell=bash
# Format 1.0 section 10: the context compiler and its index.
#
# On a scratch program (CX) with one directory Warrant, one ticket with a
# summary and two items, and the password-reset record area:
# - `war context <alias>` is deterministic: twice in one tree, and once in a
#   clone at another path, give the same bytes. Refused: the same command
#   after one record's bytes changed gives a different selection digest.
# - The packet lists the Warrant first, then its sections, then the rest in
#   section 10.3's type rank; every [F] line has its record's block, every
#   [S] line has none, and no block is cut.
# - A budget the roots do not fit is refused `context.over-budget`, exit 2,
#   nothing on stdout; a budget between the roots and the whole gives [S]
#   lines, and the same budget with --strict is refused by the same rule.
# - A ticket's checklist path resolves to the ticket; a record atom's path to
#   its records; a target nothing names is refused `context.target-unknown`.
# - `war admin compile` writes docs/generated/llms.txt with the ticket's
#   summary; `war check --generated` passes it and refuses a hand-edit as
#   `context-index.drift`.
# - `war create --summary` over 200 bytes is refused `ticket.summary`; a
#   Format 1.0 work document without `summary:` is `format.field-missing`,
#   and with one it is not.

echo "== war context: the context compiler and its index (Format 1.0 s10) =="
CX_TMP=$(mktemp -d)
CX_ROOT=$(scratch_corpus CX)
[[ -d "${CX_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus\n' >&2; exit 9; }
CX_WAR="$REPO_ROOT/${WAR#./}"
cx_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
cx_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
cx_war() { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR "$CX_WAR" --root "$CX_ROOT" "$@" </dev/null; }
cx_git() { git -C "$CX_ROOT" -c user.email=plant@invalid -c user.name=plant -c commit.gpgSign=false "$@"; }

mkdir -p "$CX_ROOT/profiles" "$CX_ROOT/docs/records/password-reset"
for p in delivery decision ticket prd architecture test-plan agent-packet; do
    cp "$REPO_ROOT/profiles/$p.toml" "$CX_ROOT/profiles/"
done
for f in 10-records.md 20-product.md 30-architecture.md; do
    cp "$REPO_ROOT/docs/records/password-reset/$f" "$CX_ROOT/docs/records/password-reset/"
done
cx_war new "Parse the manifest" >/dev/null 2>&1
cx_war create "Ship the parser" --summary "The parser reads manifests and refuses duplicates." \
    --item "Read the file" --item "Refuse a duplicate" >/dev/null 2>&1
cx_war compile >/dev/null 2>&1
cx_git add -A >/dev/null 2>&1
cx_git commit -qm "context fixtures" >/dev/null 2>&1
CX_ALIAS=CX-WAR-0001
CX_TICKET=$(basename "$(dirname "$(ls "$CX_ROOT"/docs/tickets/*/manifest.toml | head -1)")")

# --- determinism ------------------------------------------------------------
cx_war context "$CX_ALIAS" > "$CX_TMP/a.md" 2>"$CX_TMP/a.err"; CX_RC=$?
cx_war context "$CX_ALIAS" > "$CX_TMP/b.md" 2>/dev/null
CX_CLONE="$CX_TMP/moved/elsewhere"
mkdir -p "$CX_TMP/moved"
git clone -q "$CX_ROOT" "$CX_CLONE" >/dev/null 2>&1
env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR "$CX_WAR" --root "$CX_CLONE" context "$CX_ALIAS" \
    > "$CX_TMP/c.md" 2>/dev/null </dev/null
printf '\nAn added line.\n' >> "$CX_CLONE/docs/warrants/$CX_ALIAS/atoms/10-intent.md"
env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_ACTOR "$CX_WAR" --root "$CX_CLONE" context "$CX_ALIAS" \
    > "$CX_TMP/d.md" 2>/dev/null </dev/null
CX_SEL_A=$(sed -n 2p "$CX_TMP/a.md")
CX_SEL_D=$(sed -n 2p "$CX_TMP/d.md")
if [[ $CX_RC -eq 0 && -s "$CX_TMP/a.md" ]] && cmp -s "$CX_TMP/a.md" "$CX_TMP/b.md" \
    && cmp -s "$CX_TMP/a.md" "$CX_TMP/c.md" && [[ "$CX_SEL_A" != "$CX_SEL_D" ]] \
    && ! grep -qF "$CX_ROOT" "$CX_TMP/a.md"; then
    cx_ok "context is deterministic" "$(wc -c < "$CX_TMP/a.md") bytes, equal twice and in a moved clone; an edit moves the digest"
else
    cx_fail "context is deterministic" "exit $CX_RC; $(head -2 "$CX_TMP/a.err" | tr '\n' '|')"
fi

# --- order and whole records -----------------------------------------------
CX_TYPES=$(awk '/^## Index/{f=1; next} /^$/{f=0} f' "$CX_TMP/a.md" | awk -F' · ' '{print $2}' | tr '\n' ' ')
CX_WHOLE=$(python3 - "$CX_TMP/a.md" "$CX_ROOT" <<'PY'
import sys, pathlib
md = open(sys.argv[1], encoding="utf-8").read()
root = pathlib.Path(sys.argv[2])
index = md.split("\n## Index\n", 1)[1].split("\n\n## Records\n", 1)[0].splitlines()
bad = []
for line in index:
    mark, rest = line[:3], line[4:]
    rid = rest.split(" · ", 1)[0]
    has = f"\n> {rid} · " in md
    if (mark == "[F]") != has:
        bad.append(f"{mark} {rid} block={has}")
print("ok" if not bad else "; ".join(bad))
PY
)
if [[ $CX_TYPES == "warrant section section section "* && $CX_WHOLE == ok ]]; then
    cx_ok "warrant, sections, then by rank" "$(wc -w <<<"$CX_TYPES") records; [F] lines carry blocks, [S] none"
else
    cx_fail "warrant, sections, then by rank" "types: $CX_TYPES; $CX_WHOLE"
fi

# --- budget -----------------------------------------------------------------
cx_war context "$CX_ALIAS" --budget 300 > "$CX_TMP/tiny.md" 2>"$CX_TMP/tiny.err"; CX_TINY_RC=$?
CX_FULL=$(wc -c < "$CX_TMP/a.md")
CX_PART=$((CX_FULL - 40))
cx_war context "$CX_ALIAS" --budget "$CX_PART" > "$CX_TMP/part.md" 2>/dev/null; CX_PART_RC=$?
cx_war context "$CX_ALIAS" --budget "$CX_PART" --strict > "$CX_TMP/strict.md" 2>"$CX_TMP/strict.err"; CX_STRICT_RC=$?
CX_S=$(grep -c '^\[S\] ' "$CX_TMP/part.md")
if [[ $CX_TINY_RC -eq 2 && ! -s "$CX_TMP/tiny.md" ]] && grep -q 'context.over-budget' "$CX_TMP/tiny.err" \
    && [[ $CX_PART_RC -eq 0 && $CX_S -ge 1 && $(wc -c < "$CX_TMP/part.md") -le $CX_PART ]] \
    && [[ $CX_STRICT_RC -eq 2 && ! -s "$CX_TMP/strict.md" ]] && grep -q 'context.over-budget' "$CX_TMP/strict.err"; then
    cx_ok "budget refuses or marks [S]" "300 refused; $CX_PART bytes gives $CX_S [S]; --strict refused"
else
    cx_fail "budget refuses or marks [S]" "tiny exit $CX_TINY_RC, partial exit $CX_PART_RC with $CX_S [S], strict exit $CX_STRICT_RC"
fi

# --- targets ----------------------------------------------------------------
cx_war context "docs/tickets/$CX_TICKET/atoms/15-checklist.md" > "$CX_TMP/path.md" 2>/dev/null; CX_PATH_RC=$?
cx_war context "docs/records/password-reset/10-records.md" --depth 0 > "$CX_TMP/rec.md" 2>/dev/null; CX_REC_RC=$?
cx_war context "NOPE-404" > "$CX_TMP/nope.md" 2>"$CX_TMP/nope.err"; CX_NOPE_RC=$?
CX_PATH_FIRST=$(awk '/^## Index/{getline; print; exit}' "$CX_TMP/path.md")
if [[ $CX_PATH_RC -eq 0 && $CX_PATH_FIRST == "[F] $CX_TICKET · ticket · "* && $CX_REC_RC -eq 0 ]] \
    && grep -q '^\[F\] REQ-pr1 · requirement' "$CX_TMP/rec.md" \
    && [[ $CX_NOPE_RC -eq 2 && ! -s "$CX_TMP/nope.md" ]] && grep -q 'context.target-unknown' "$CX_TMP/nope.err"; then
    cx_ok "paths resolve, unknown refused" "checklist -> $CX_TICKET; record atom -> its records; NOPE-404 refused"
else
    cx_fail "paths resolve, unknown refused" "path exit $CX_PATH_RC ($CX_PATH_FIRST), records exit $CX_REC_RC, unknown exit $CX_NOPE_RC"
fi

# --- the index ----------------------------------------------------------------
CX_IDX="$CX_ROOT/docs/generated/llms.txt"
cx_war --json check --generated > "$CX_TMP/gen.json" 2>/dev/null
CX_GEN=$(python3 -c "import json,sys; v=json.load(open(sys.argv[1])); print(' '.join(sorted(d['severity']+':'+d['rule'] for d in v['diagnostics'] if d['rule'].startswith('context-index'))))" "$CX_TMP/gen.json" 2>&1)
cp "$CX_IDX" "$CX_TMP/llms.txt"
printf -- '- HAND-1 · edited\n' >> "$CX_IDX"
cx_war --json check --generated > "$CX_TMP/gen2.json" 2>/dev/null; CX_GEN2_RC=$?
CX_GEN2=$(python3 -c "import json,sys; v=json.load(open(sys.argv[1])); print(' '.join(sorted(d['severity']+':'+d['rule'] for d in v['diagnostics'] if d['rule'].startswith('context-index'))))" "$CX_TMP/gen2.json" 2>&1)
cp "$CX_TMP/llms.txt" "$CX_IDX"
if grep -qF '> The parser reads manifests and refuses duplicates.' "$CX_TMP/llms.txt" \
    && grep -q "^- $CX_ALIAS · warrant · " "$CX_TMP/llms.txt" \
    && [[ $CX_GEN == "pass:context-index.drift" && $CX_GEN2 == "error:context-index.drift" && $CX_GEN2_RC -ne 0 ]]; then
    cx_ok "llms.txt compiled, drift refused" "$(wc -l < "$CX_TMP/llms.txt") lines; a hand-edit is context-index.drift"
else
    cx_fail "llms.txt compiled, drift refused" "clean: '$CX_GEN'; edited: '$CX_GEN2' exit $CX_GEN2_RC"
fi

# --- the summary tier ---------------------------------------------------------
CX_LONG=$(printf 'x%.0s' $(seq 1 201))
cx_war --json create "Too long" --summary "$CX_LONG" > "$CX_TMP/long.json" 2>/dev/null; CX_LONG_RC=$?
mkdir -p "$CX_ROOT/docs/warrants"
printf -- '---\nformat: openwarrant/1.0\nprofile: delivery\nalias: CX-WAR-0900\nuuid: 01a0d342-ee5e-7b71-9623-f2d8cd5acd4e\n---\n# A Format 1.0 Warrant\n' \
    > "$CX_ROOT/docs/warrants/CX-WAR-0900.md"
cx_war --json check > "$CX_TMP/f1.json" 2>/dev/null
CX_F1=$(python3 -c "import json,sys; v=json.load(open(sys.argv[1])); print(' '.join(d['severity']+':'+d['rule']+'@'+(d['file'] or '') for d in v['diagnostics'] if d['rule']=='format.field-missing'))" "$CX_TMP/f1.json" 2>&1)
sed -i 's/^alias: CX-WAR-0900$/alias: CX-WAR-0900\nsummary: What this Warrant is for./' "$CX_ROOT/docs/warrants/CX-WAR-0900.md"
cx_war --json check > "$CX_TMP/f2.json" 2>/dev/null
CX_F2=$(python3 -c "import json,sys; v=json.load(open(sys.argv[1])); print(' '.join(d['rule'] for d in v['diagnostics'] if d['rule']=='format.field-missing'))" "$CX_TMP/f2.json" 2>&1)
/usr/bin/rm -f "$CX_ROOT/docs/warrants/CX-WAR-0900.md"
if [[ $CX_LONG_RC -ne 0 ]] && grep -q '"ticket.summary"' "$CX_TMP/long.json" \
    && [[ $CX_F1 == "error:format.field-missing@docs/warrants/CX-WAR-0900.md:1" && -z $CX_F2 ]]; then
    cx_ok "summaries: written, bounded, required" "201 bytes refused; a Format 1.0 Warrant without one is format.field-missing"
else
    cx_fail "summaries: written, bounded, required" "long exit $CX_LONG_RC; without: '$CX_F1'; with: '$CX_F2'"
fi

/usr/bin/rm -rf "$CX_TMP"
unset CX_TMP CX_ROOT CX_WAR CX_ALIAS CX_TICKET CX_CLONE CX_IDX
