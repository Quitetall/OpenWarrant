# shellcheck shell=bash
# The ladder (FORMAT-1.0 §4.6, revision 4): two kinds, four tiers, one Spec and
# one Roadmap, each refusal beside the claim it guards.
#
#   masters of both kinds, typed at their derived tier, check clean;
#   a `type` that is not the derived tier is refused (format.type-tier);
#   a `kind` that disagrees with the id is refused (format.kind-mismatch);
#   the Spec and the Roadmap at their fixed places check clean;
#   a second Roadmap is refused, and so is the master `spec` beside SPEC.md;
#   `war context` gives a higher tier first within one distance, and with the
#   tiers reversed (the same records, profiles swapped) the order turns.

echo "== ladder (FORMAT-1.0 §4.6) =="
LD_ROOT=$(scratch_corpus LD)
[[ -d "${LD_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
ld()      { env -u SSH_AUTH_SOCK -u SSH_AGENT_PID -u OPENWARRANT_SESSION_ID -u KATANA_SESSION_ID \
                -u CODEX_THREAD_ID -u CODEX_SESSION_ID -u CLAUDE_CODE_SESSION_ID \
                "$REPO_ROOT/${WAR#./}" --root "$LD_ROOT" "$@"; }
ld_ok()   { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
ld_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
ld_lines() { grep -E 'format\.|ERROR|error' <<<"$1" | head -3 | tr '\n' '|'; }
# ld_doc <id> <file name> <profile> <extra frontmatter line or ""> [section]
ld_doc() {
    mkdir -p "$LD_ROOT/docs/work/$1"
    {
        printf -- '---\nformat: openwarrant/1.0\nprofile: %s\nalias: %s\n' "$3" "$1"
        printf 'uuid: %s\nsummary: Ladder plant item %s.\n' "$(ld_uuid)" "$1"
        [[ -n "$4" ]] && printf '%s\n' "$4"
        printf -- '---\n# Item %s\n\n%s\n' "$1" "${5:-## Notes
Plain text.}"
    } > "$LD_ROOT/docs/work/$1/$2"
}
LD_N=0
ld_uuid() { LD_N=$((LD_N + 1)); printf '01a1b788-533f-76ca-8adf-%012x' "$((0x73e0ddb93400 + LD_N))"; }
ld_rule() { ld --json check 2>/dev/null | python3 -c '
import json, sys
rule = sys.argv[1]
d = json.load(sys.stdin)
print("\n".join(sorted(x.get("file", "") for x in d.get("diagnostics", []) if x.get("rule") == rule)))' "$1"; }

# ---- 1. masters of both kinds, typed at their tier, check clean ----
ld_doc w-1a2b3 w-1a2b3-a-ticket.md ticket "kind: work" "## Checklist
- [ ] One item (i-c01)"
ld_doc d-4c5d6 d-4c5d6-a-design.md design "type: design"
ld_doc d-7e8f9 d-7e8f9.md guide "type: guide"
ld_doc d-0a1b2 d-0a1b2-an-adr.md adr "type: adr" "## Context
Why.

## Decision
What.

## Consequences
So."
out=$(ld check 2>&1); st=$?
bad=$(ld_rule format.type-tier; ld_rule format.kind-mismatch)
if [[ $st -eq 0 && -z "$bad" ]]; then
    ld_ok "typed masters check clean" "design, guide, adr, ticket"
else
    ld_fail "typed masters check clean" "exit $st: $(ld_lines "$out") $bad"
fi

# ---- 2. a type that is not the derived tier is refused ----
ld_doc d-3c4d5 d-3c4d5-says-adr.md guide "type: adr"
got=$(ld_rule format.type-tier)
ld check >/dev/null 2>&1; st=$?
if [[ $st -ne 0 && "$got" == *"d-3c4d5-says-adr.md:7"* ]]; then
    ld_ok "type above its tier refused" "format.type-tier at the type line"
else
    ld_fail "type above its tier refused" "exit $st: [$got]"
fi
/usr/bin/rm -rf "$LD_ROOT/docs/work/d-3c4d5"
ld_doc d-3c4d5 d-3c4d5-says-novel.md guide "type: novel"
got=$(ld_rule format.type-tier)
[[ "$got" == *"d-3c4d5-says-novel.md:7"* ]] \
    && ld_ok "unknown type refused" "format.type-tier" \
    || ld_fail "unknown type refused" "[$got]"
/usr/bin/rm -rf "$LD_ROOT/docs/work/d-3c4d5"

# ---- 3. a kind that disagrees with the id is refused ----
ld_doc w-6f7a8 w-6f7a8-claims-document.md ticket "kind: document" "## Checklist
- [ ] One item (i-c02)"
got=$(ld_rule format.kind-mismatch)
[[ "$got" == *"w-6f7a8-claims-document.md:7"* ]] \
    && ld_ok "kind against the id refused" "format.kind-mismatch" \
    || ld_fail "kind against the id refused" "[$got]"
/usr/bin/rm -rf "$LD_ROOT/docs/work/w-6f7a8"

# ---- 4. the singletons at their places check clean ----
ld_doc spec spec.md spec "version: 0.1.0
type: spec" "## Purpose
Why the system exists.

## Scope
In: everything here.

## REQ-ld-1 · requirement — the ladder holds
statement: A type MUST match its tier."
ld_doc roadmap roadmap.md roadmap "" "## PH-1 · phase — first"
dup=$(ld_rule format.document-duplicate)
out=$(ld check 2>&1); st=$?
if [[ $st -eq 0 && -z "$dup" ]]; then
    ld_ok "Spec and Roadmap check clean" "docs/work/spec, docs/work/roadmap"
else
    ld_fail "Spec and Roadmap check clean" "exit $st: $(ld_lines "$out") [$dup]"
fi

# ---- 5. a second Roadmap, and the master spec beside SPEC.md, are refused ----
ld_doc w-9b8c7 w-9b8c7-another-roadmap.md roadmap "" "## PH-2 · phase — second"
got=$(ld_rule format.document-duplicate)
[[ "$got" == *"w-9b8c7-another-roadmap.md"* ]] \
    && ld_ok "second Roadmap refused" "format.document-duplicate" \
    || ld_fail "second Roadmap refused" "[$got]"
/usr/bin/rm -rf "$LD_ROOT/docs/work/w-9b8c7"
printf -- '---\nformat: openwarrant/1.0\nprofile: spec\nuuid: %s\nversion: 0.1.0\nsummary: Older place.\n---\n# Old spec\n\n## Purpose\nP.\n\n## Scope\nS.\n' \
    "$(ld_uuid)" > "$LD_ROOT/SPEC.md"
got=$(ld_rule format.document-duplicate)
[[ "$got" == *"docs/work/spec/spec.md"* && "$got" != *"SPEC.md:"*"SPEC.md"* ]] \
    && ld_ok "second Spec refused" "the master beside SPEC.md" \
    || ld_fail "second Spec refused" "[$got]"
/usr/bin/rm -f "$LD_ROOT/SPEC.md"

# ---- 6. context: a higher tier first within a distance, and the reverse ----
# One design at tier 3 and one ADR at tier 2, both implementing REQ-ld-1,
# reached from the requirement at distance 1 (in:implements from an interface
# is not walked, so the target is a ticket that implements the requirement and
# depends on both).
/usr/bin/rm -rf "$LD_ROOT/docs/work/w-1a2b3" "$LD_ROOT/docs/work/d-4c5d6" "$LD_ROOT/docs/work/d-0a1b2" "$LD_ROOT/docs/work/d-7e8f9"
ld_doc d-11111 d-11111-low.md guide "" "## Body
Low."
ld_doc d-22222 d-22222-high.md adr "" "## Context
C.

## Decision
D.

## Consequences
Q."
ld_doc w-33333 w-33333-target.md ticket "depends_on:
  - d-11111
  - d-22222" "## Checklist
- [ ] Do it (i-c03)"
ld_order() { ld context w-33333 --budget 100000 2>/dev/null | grep -E '^\[[FS]\] d-(11111|22222) ' | cut -c5-11 | tr '\n' ' '; }
first=$(ld_order)
if [[ "$first" == "d-22222 d-11111 " ]]; then
    ld_ok "context ranks tier 2 over tier 4" "$first"
else
    ld_fail "context ranks tier 2 over tier 4" "[$first]"
fi
# The same ids, profiles swapped: the order turns, so it is the tier that decides.
sed -i 's/^profile: guide$/profile: TMP/' "$LD_ROOT/docs/work/d-11111/d-11111-low.md"
sed -i 's/^profile: adr$/profile: guide/' "$LD_ROOT/docs/work/d-22222/d-22222-high.md"
sed -i 's/^profile: TMP$/profile: adr/' "$LD_ROOT/docs/work/d-11111/d-11111-low.md"
# The ADR profile requires Context, Decision and Consequences; give d-11111 them.
python3 - "$LD_ROOT/docs/work/d-11111/d-11111-low.md" <<'PY'
import sys
p = sys.argv[1]
t = open(p).read().replace("## Body\nLow.", "## Context\nC.\n\n## Decision\nD.\n\n## Consequences\nQ.")
open(p, "w").write(t)
PY
second=$(ld_order)
if [[ "$second" == "d-11111 d-22222 " ]]; then
    ld_ok "swapped tiers turn the order" "$second"
else
    ld_fail "swapped tiers turn the order" "[$second]"
fi
