# shellcheck shell=bash
# OW-WAR-0148 M10 — bring your work with you: Beads, OpenSpec and Spec Kit
# in, Beads out.
#
# Fixtures (conformance/fixtures/interop/) are written from each tool's own
# format. Claims: a Beads export imports as one light Warrant per issue and
# `war export beads` gives the mapped fields back (id, text fields, status,
# priority, type, labels, parent-child and blocks, comments, dates); that
# export imported into a second program gives the same Warrants byte for
# byte; running an import again changes nothing; an OpenSpec change and a
# Spec Kit feature each become a Warrant with their tasks as items (a Spec
# Kit `depends on` becomes `after`), and Spec Kit's requirements become
# records where the program's record format admits them, checked by
# `war check`. Refusals, each by rule and each writing nothing: a custom
# Beads status, an issue type or label the profile does not admit, a
# dependency type with no place, an unknown field with a value, a memory
# line, a blocks on an issue in neither the input nor the program; a
# malformed OpenSpec or Spec Kit task list; a folder that is neither; an
# unknown format; `war export beads` with a §68 flag.

echo "== import and export: Beads, OpenSpec, Spec Kit (M10) =="
PLANT_ROOT=$(scratch_corpus IM)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
IM_TMP=$(mktemp -d)
IM_FIX="$REPO_ROOT/conformance/fixtures/interop"
im_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
im_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
# imw <root> <args...>
imw() { local r=$1; shift; env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK -u SSH_AGENT_PID "$WAR" --root "$r" "$@" </dev/null; }
imj() { local r=$1; shift; imw "$r" --json "$@" 2>/dev/null; }
im_field() { python3 -c "import json,sys; v=json.loads(sys.argv[1]); print($2)" "$1" 2>/dev/null; }
# Every byte under docs/tickets and docs/records, one digest per file.
im_bytes() { (cd "$1" && find docs/tickets docs/records -type f -print0 2>/dev/null | LC_ALL=C sort -z | xargs -0 -r sha256sum); }
# A second, untouched program for the refusals and the round trip.
IM_TWO=$(scratch_corpus IM)
IM_THREE=$(scratch_corpus IM)

# 1. Beads in, and out again with every mapped field.
IM_OUT=$(imj "$PLANT_ROOT" import beads "$IM_FIX/beads/issues.jsonl")
IM_MADE=$(im_field "$IM_OUT" '",".join(sorted(c["imported_from"] for c in v["result"]["created"]))')
IM_IDS=$(im_field "$IM_OUT" '" ".join(c["id"] for c in v["result"]["created"])')
IM_WARN=$(im_field "$IM_OUT" '",".join(d["rule"] for d in v["diagnostics"] if d["severity"] in ("warn", "WARN"))')
imw "$PLANT_ROOT" export beads > "$IM_TMP/out.jsonl" 2>/dev/null
IM_CMP=$(python3 - "$IM_FIX/beads/issues.jsonl" "$IM_TMP/out.jsonl" <<'PY'
import json, sys
src = {(i := json.loads(l))["id"]: i for l in open(sys.argv[1]) if l.strip()}
out = {(i := json.loads(l))["id"]: i for l in open(sys.argv[2]) if l.strip()}
def view(i):
    return {
        "title": i["title"],
        "text": [i.get(k, "") for k in ("description", "design", "acceptance_criteria", "notes")],
        # A Warrant is done or not: every open status reads as open.
        "status": "closed" if i.get("status") == "closed" else "open",
        "priority": i["priority"], "type": i.get("issue_type"),
        "labels": sorted(i.get("labels", [])),
        "deps": sorted((d["type"], d["depends_on_id"]) for d in i.get("dependencies", [])),
        "comments": [(c["author"], c["text"], c["created_at"]) for c in i.get("comments", [])],
        "dates": [i["created_at"], i.get("created_by"), i.get("closed_at"), i.get("close_reason")],
        "counts": [i.get("dependency_count"), i.get("dependent_count"), i.get("comment_count")],
    }
bad = [k for k in src if k not in out or view(src[k]) != view(out[k])]
extra = [k for k in out if k not in src]
print("same" if not bad and not extra else f"differ: {bad} extra {extra} " + (json.dumps(view(out[bad[0]])) if bad and bad[0] in out else ""))
PY
)
if [[ "$IM_MADE" == "beads:bd-a1b2,beads:bd-c3d4,beads:bd-e5f6,beads:bd-g7h8" && "$IM_CMP" == same ]] \
    && [[ "$IM_IDS" =~ ^t-[0-9a-f]{8}( t-[0-9a-f]{8}){3}$ ]] \
    && [[ ",$IM_WARN," == *",beads.field-dropped,"* && ",$IM_WARN," == *",beads.status-open,"* ]]; then
    im_ok "beads round trip" "4 issues in, 4 out: ids, text, status, priority, type, labels, deps, comments, dates, counts as given; assignee/owner and in_progress said"
else
    im_fail "beads round trip" "made '$IM_MADE' ids '$IM_IDS' warn '$IM_WARN' compare: $IM_CMP"
fi

# 2. Deterministic and idempotent: the export into a second program gives
#    the same Warrants, byte for byte; running an import again writes
#    nothing.
imj "$IM_TWO" import beads "$IM_TMP/out.jsonl" >/dev/null
IM_A=$( (cd "$PLANT_ROOT/docs/tickets" && find . -type f ! -name journal.jsonl -print0 | LC_ALL=C sort -z | xargs -0 sha256sum) )
IM_B=$( (cd "$IM_TWO/docs/tickets" && find . -type f ! -name journal.jsonl -print0 | LC_ALL=C sort -z | xargs -0 sha256sum) )
IM_B0=$(im_bytes "$PLANT_ROOT")
IM_AGAIN=$(imj "$PLANT_ROOT" import beads "$IM_FIX/beads/issues.jsonl")
IM_B1=$(im_bytes "$PLANT_ROOT")
IM_SAME=$(im_field "$IM_AGAIN" '"%d/%d" % (len(v["result"]["created"]), len(v["result"]["unchanged"]))')
if [[ -n "$IM_A" && "$IM_A" == "$IM_B" && "$IM_B0" == "$IM_B1" && "$IM_SAME" == "0/4" ]]; then
    im_ok "deterministic and idempotent" "$(wc -l <<<"$IM_A") Warrant files equal in two programs; again: 0 created, 4 left as they are, no byte moved"
else
    im_fail "deterministic and idempotent" "programs equal: $([[ "$IM_A" == "$IM_B" ]] && echo yes || echo no); again $IM_SAME; moved: $([[ "$IM_B0" == "$IM_B1" ]] && echo no || echo yes)"
fi

# 3. Beads refusals: each by its rule, and nothing written.
IM_BAD=""
im_refuse() {  # <rule> <python that edits the issues list `l`>
    local rule=$1 file="$IM_TMP/bad.jsonl" before after err rc
    python3 - "$IM_FIX/beads/issues.jsonl" "$file" "$2" <<'PY'
import json, sys
l = [json.loads(x) for x in open(sys.argv[1]) if x.strip()]
exec(sys.argv[3])
open(sys.argv[2], "w").write("".join(json.dumps(i) + "\n" for i in l))
PY
    before=$(im_bytes "$IM_THREE")
    err=$(imw "$IM_THREE" import beads "$file" 2>&1 >/dev/null); rc=$?
    after=$(im_bytes "$IM_THREE")
    if [[ $rc -ne 2 || "$before" != "$after" ]] || ! grep -qF "$rule" <<<"$err"; then
        IM_BAD="$IM_BAD [$rule: exit $rc, wrote $([[ "$before" == "$after" ]] && echo nothing || echo something): $(head -c 200 <<<"$err")]"
    fi
}
im_refuse beads.status-unmapped 'l[3]["status"] = "review"'
im_refuse beads.type-unmapped 'l[3]["issue_type"] = "spike"'
im_refuse beads.label-unmapped 'l[3]["labels"] = ["Frontend"]'
im_refuse beads.dependency-unmapped 'l[3]["dependencies"] = [{"issue_id": "bd-g7h8", "depends_on_id": "bd-a1b2", "type": "related"}]'
im_refuse beads.field-unmapped 'l[3]["await_type"] = "gh:pr"'
im_refuse beads.not-an-issue 'l.append({"_type": "memory", "key": "k", "value": "v"})'
im_refuse import.target-unknown 'l[3]["dependencies"] = [{"issue_id": "bd-g7h8", "depends_on_id": "bd-zzzz", "type": "blocks"}]'
IM_ERR=$(imw "$IM_THREE" import jira "$IM_FIX/beads/issues.jsonl" 2>&1 >/dev/null); IM_S=$?
IM_ERR2=$(imw "$IM_THREE" export beads --force 2>&1); IM_S2=$?
IM_LEFT=$(find "$IM_THREE/docs/tickets" -mindepth 1 -maxdepth 1 2>/dev/null | wc -l)
if [[ -z "$IM_BAD" && $IM_S -eq 2 && $IM_S2 -ne 0 && $IM_LEFT -eq 0 ]] && grep -qF 'import.format-unknown' <<<"$IM_ERR" \
    && grep -qF 'export.beads-flags' <<<"$IM_ERR2"; then
    im_ok "beads refusals write nothing" "custom status, type, label, dependency type, unknown field, memory line, unknown blocks target, unknown format, a §68 flag: each by rule"
else
    im_fail "beads refusals write nothing" "$IM_BAD; format $IM_S '$IM_ERR'; flag $IM_S2 '$IM_ERR2'; tickets left $IM_LEFT"
fi

# 4. OpenSpec: each change a Warrant, its tasks the items; again, nothing.
IM_OS=$(imj "$PLANT_ROOT" import openspec "$IM_FIX/openspec")
IM_OSID=$(im_field "$IM_OS" '[c["id"] for c in v["result"]["created"] if c["imported_from"] == "openspec:add-2fa"][0]')
IM_OSN=$(im_field "$IM_OS" '",".join(sorted("%s=%d" % (c["imported_from"], c["items"]) for c in v["result"]["created"]))')
IM_OSCL="$PLANT_ROOT/docs/tickets/$IM_OSID/atoms/15-checklist.md"
IM_OSAGAIN=$(im_field "$(imj "$PLANT_ROOT" import openspec "$IM_FIX/openspec")" '"%d/%d" % (len(v["result"]["created"]), len(v["result"]["unchanged"]))')
if [[ "$IM_OSN" == "openspec:add-2fa=4,openspec:rename-audit-log=2" && "$IM_OSAGAIN" == "0/2" ]] \
    && grep -qE '^- \[x\] 1\.1 Store a TOTP secret with the user \(i-[0-9a-f]+\)' "$IM_OSCL" \
    && grep -qE '^- \[ \] 2\.2 Shorten the session' "$IM_OSCL" \
    && grep -qF 'openspec:two-factor#totp-second-factor' "$PLANT_ROOT/docs/tickets/$IM_OSID/atoms/10-intent.md"; then
    im_ok "openspec changes import" "add-2fa (4 tasks, 1.1 done) and rename-audit-log (2), deltas named; again: 0 created"
else
    im_fail "openspec changes import" "made '$IM_OSN' again '$IM_OSAGAIN'; $(head -c 300 "$IM_OSCL" 2>/dev/null)"
fi

# 5. Spec Kit: each feature a Warrant, `depends on` as `after`. Records
#    where the record format admits them: not in a program whose delivery
#    profile declares none (said, by rule), and in one that declares
#    requirement and outcome (written, and `war check` reads them clean).
IM_SK=$(imj "$PLANT_ROOT" import speckit "$IM_FIX/speckit")
IM_SKID=$(im_field "$IM_SK" '[c["id"] for c in v["result"]["created"] if c["imported_from"] == "speckit:001-photo-albums"][0]')
IM_SKW=$(im_field "$IM_SK" '",".join(sorted(set(d["rule"] for d in v["diagnostics"] if d["rule"].startswith("speckit."))))')
IM_SKCL="$PLANT_ROOT/docs/tickets/$IM_SKID/atoms/15-checklist.md"
IM_T3=$(sed -n 's/^- \[ \] T003 .*(\(i-[0-9a-f]*\))$/\1/p' "$IM_SKCL")
mkdir -p "$IM_TWO/profiles"
sed '/^require = /d' "$REPO_ROOT/profiles/delivery.toml" > "$IM_TWO/profiles/delivery.toml"
IM_SK2=$(imj "$IM_TWO" import speckit "$IM_FIX/speckit")
IM_REC=$(im_field "$IM_SK2" '",".join(v["result"]["records"])')
IM_MODEL=$(imj "$IM_TWO" model)
IM_RT=$(im_field "$IM_MODEL" '",".join(sorted(r["id"]+":"+r["type"] for r in v["result"]["records"] if r["id"].startswith("SK001-")))')
IM_CHK=$(imw "$IM_TWO" check 2>&1)
if [[ -n "$IM_T3" && "$IM_SKW" == "speckit.records-not-admitted" && ! -d "$PLANT_ROOT/docs/records/001-photo-albums" ]] \
    && grep -qE "^- \[ \] T004 .*\(i-[0-9a-f]+, after $IM_T3\)$" "$IM_SKCL" \
    && [[ "$IM_REC" == "docs/records/001-photo-albums/10-speckit.md,docs/records/002-album-sharing/10-speckit.md" ]] \
    && [[ "$IM_RT" == "SK001-FR-001:requirement,SK001-FR-002:requirement,SK001-FR-003:requirement,SK001-SC-001:outcome" ]] \
    && grep -q '^PASS records.well-formed' <<<"$IM_CHK" && ! grep -q '^ERROR record\.' <<<"$IM_CHK"; then
    im_ok "spec kit features import" "T004 waits on T003; records not admitted here (said), admitted there: 4 records of 001, war check clean"
else
    im_fail "spec kit features import" "T003 '$IM_T3' warn '$IM_SKW' records '$IM_REC' model '$IM_RT'; check: $(grep -E '^(ERROR|PASS) record' <<<"$IM_CHK" | head -3 | tr '\n' ' ')"
fi

# 6. Folders refused by rule, writing nothing: a malformed task list in
#    either format, and a folder that is neither.
\cp -r "$IM_FIX/openspec" "$IM_TMP/os"
printf -- '- [y] 3.1 A bad box\n' >> "$IM_TMP/os/changes/add-2fa/tasks.md"
\cp -r "$IM_FIX/speckit" "$IM_TMP/sk"
printf -- '- [ ] Untracked task with no id\n' >> "$IM_TMP/sk/specs/001-photo-albums/tasks.md"
IM_F0=$(im_bytes "$IM_THREE")
IM_E1=$(imw "$IM_THREE" import openspec "$IM_TMP/os" 2>&1 >/dev/null); IM_S1=$?
IM_E2=$(imw "$IM_THREE" import speckit "$IM_TMP/sk" 2>&1 >/dev/null); IM_S2=$?
IM_E3=$(imw "$IM_THREE" import openspec "$IM_FIX/beads" 2>&1 >/dev/null); IM_S3=$?
IM_E4=$(imw "$IM_THREE" import speckit "$IM_FIX/beads" 2>&1 >/dev/null); IM_S4=$?
IM_F1=$(im_bytes "$IM_THREE")
if [[ $IM_S1 -eq 2 && $IM_S2 -eq 2 && $IM_S3 -eq 2 && $IM_S4 -eq 2 && "$IM_F0" == "$IM_F1" ]] \
    && grep -qF 'openspec.tasks-malformed' <<<"$IM_E1" && grep -qF 'changes/add-2fa/tasks.md:12' <<<"$IM_E1" \
    && grep -qF 'speckit.tasks-malformed' <<<"$IM_E2" && grep -qF '001-photo-albums/tasks.md:29' <<<"$IM_E2" \
    && grep -qF 'openspec.missing' <<<"$IM_E3" && grep -qF 'speckit.missing' <<<"$IM_E4"; then
    im_ok "folder refusals write nothing" "a [y] box (tasks.md:12), a Spec Kit task with no id (tasks.md:29), a folder that is neither: each by rule"
else
    im_fail "folder refusals write nothing" "$IM_S1 '$(head -c 160 <<<"$IM_E1")' $IM_S2 '$(head -c 160 <<<"$IM_E2")' $IM_S3 $IM_S4; wrote: $([[ "$IM_F0" == "$IM_F1" ]] && echo nothing || echo something)"
fi

command rm -rf "$IM_TMP"
corpus_gone "$IM_TWO"
corpus_gone "$IM_THREE"
corpus_gone "$PLANT_ROOT"
unset IM_TMP IM_FIX IM_TWO IM_THREE IM_OUT IM_MADE IM_IDS IM_WARN IM_CMP IM_A IM_B IM_B0 IM_B1 IM_AGAIN IM_SAME \
    IM_BAD IM_ERR IM_ERR2 IM_S IM_S1 IM_S2 IM_S3 IM_S4 IM_LEFT IM_OS IM_OSID IM_OSN IM_OSCL IM_OSAGAIN IM_SK IM_SKID \
    IM_SKW IM_SKCL IM_T3 IM_SK2 IM_REC IM_MODEL IM_RT IM_CHK IM_F0 IM_F1 IM_E1 IM_E2 IM_E3 IM_E4
