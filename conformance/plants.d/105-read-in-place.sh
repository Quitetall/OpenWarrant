# shellcheck shell=bash
# OW-WAR-0148 M10 — read in place: an OpenSpec folder and a Spec Kit folder
# seen as Warrants and records, never converted, never written.
#
# A scratch program keeps the fixture folders (conformance/fixtures/interop/)
# as its own `openspec/` and `specs/`, named by two `[[adapters]]` entries.
# Claims: `war warrants` lists each change and feature as a Warrant with its
# progress; `war model` carries their tasks as items and their requirements,
# outcomes and stories as records, with the relations their folders state;
# `war impact` walks them (a requirement reaches the change that modifies
# it and that change's tasks; a user story the tasks for it and what waits
# on those); `war show` and `war status` show them; `war check` reads them
# clean; and afterwards every byte of both folders is as it was, and the
# program's tree is clean. Refusals, each by rule: a malformed task list in
# either folder, an adapter path that is not there, a kind this build does
# not read, an id no adapter reads.

echo "== read in place: OpenSpec and Spec Kit as records (M10) =="
PLANT_ROOT=$(scratch_corpus RP)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
RP_FIX="$REPO_ROOT/conformance/fixtures/interop"
rp_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
rp_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }
rpw() { env -u OPENWARRANT_ACTOR -u SSH_AUTH_SOCK -u SSH_AGENT_PID "$WAR" --root "$PLANT_ROOT" "$@" </dev/null; }
rpj() { rpw --json "$@" 2>/dev/null; }
rp_field() { python3 -c "import json,sys; v=json.loads(sys.argv[1]); print($2)" "$1" 2>/dev/null; }
rp_bytes() { (cd "$PLANT_ROOT" && find openspec specs -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum); }

# Before the adapters: an id shaped like one is refused, and says how.
RP_ERR0=$(rpw show openspec:add-2fa 2>&1 >/dev/null); RP_S0=$?
\cp -r "$RP_FIX/openspec" "$PLANT_ROOT/openspec"
\cp -r "$RP_FIX/speckit/specs" "$PLANT_ROOT/specs"
printf '\n[[adapters]]\nkind = "openspec"\npath = "openspec"\n\n[[adapters]]\nkind = "speckit"\npath = "specs"\n' \
    >> "$PLANT_ROOT/openwarrant.toml"
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "folders read in place" >/dev/null 2>&1
RP_B0=$(rp_bytes)

# 1. The list: each change and feature a Warrant, with its progress.
RP_LS=$(rp_field "$(rpj warrants)" '"|".join("%s:%s:%s:%d/%d" % (w["id"], w["profile"], w["state"], w["done"], w["total"]) for w in v["result"]["warrants"] if w["encoding"] == "read_in_place")')
if [[ $RP_S0 -eq 2 ]] && grep -qF 'adapter.unknown' <<<"$RP_ERR0" && grep -qF '[[adapters]]' <<<"$RP_ERR0" \
    && [[ "$RP_LS" == "openspec:add-2fa:openspec:in_progress:1/4|openspec:rename-audit-log:openspec:open:0/2|speckit:001-photo-albums:speckit:in_progress:1/5|speckit:002-album-sharing:speckit:open:0/0" ]]; then
    rp_ok "listed as Warrants" "2 changes, 2 features, with progress; before [[adapters]] the id was refused (adapter.unknown)"
else
    rp_fail "listed as Warrants" "before: $RP_S0 '$RP_ERR0'; list '$RP_LS'"
fi

# 2. The model: records and relations from both folders.
RP_MODEL=$(rpj model)
RP_REC=$(rp_field "$RP_MODEL" '",".join(sorted(r["id"]+":"+r["type"] for r in v["result"]["records"] if r["id"] in ("openspec:add-2fa", "openspec:add-2fa/1.1", "openspec:auth-session#session-expiry", "openspec:two-factor#totp-second-factor", "speckit:001-photo-albums", "speckit:001-photo-albums/T004", "speckit:001-photo-albums/US1", "speckit:001-photo-albums/FR-001", "speckit:001-photo-albums/SC-001")))')
RP_REL=$(rp_field "$RP_MODEL" '",".join(sorted(r["from"].split(":",1)[1]+" "+r["kind"]+" "+r["to"].split(":",1)[1] for r in v["result"]["relations"] if r["from"] in ("openspec:add-2fa", "speckit:001-photo-albums/T004")))')
RP_DIAG=$(rp_field "$RP_MODEL" 'len(v["result"]["diagnostics"])')
if [[ "$RP_REC" == "openspec:add-2fa/1.1:item,openspec:add-2fa:change,openspec:auth-session#session-expiry:requirement,openspec:two-factor#totp-second-factor:requirement,speckit:001-photo-albums/FR-001:requirement,speckit:001-photo-albums/SC-001:outcome,speckit:001-photo-albums/T004:item,speckit:001-photo-albums/US1:story,speckit:001-photo-albums:feature" ]] \
    && [[ "$RP_REL" == "001-photo-albums/T004 depends_on 001-photo-albums/T003,001-photo-albums/T004 implements 001-photo-albums/FR-001,001-photo-albums/T004 implements 001-photo-albums/US1,001-photo-albums/T004 part_of 001-photo-albums,add-2fa implements auth-session#session-expiry,add-2fa implements two-factor#totp-second-factor,add-2fa openspec.removes auth-session#remember-me" ]] \
    && [[ "$RP_DIAG" == 0 ]]; then
    rp_ok "records in the model" "changes, features, tasks as items, requirements, outcomes, stories; implements, depends_on, part_of, openspec.removes; 0 diagnostics"
else
    rp_fail "records in the model" "records '$RP_REC' relations '$RP_REL' diagnostics $RP_DIAG"
fi

# 3. Impact walks them; show and status show them; check reads them clean.
RP_I1=$(rp_field "$(rpj impact 'openspec:auth-session#session-expiry')" '",".join(sorted(a["id"] for a in v["result"]["affected"]))')
RP_I2=$(rp_field "$(rpj impact speckit:001-photo-albums/US1)" '",".join(sorted(a["id"].split("/")[1] for a in v["result"]["affected"]))')
RP_SH=$(rpw show openspec:add-2fa 2>/dev/null)
RP_ST=$(rp_field "$(rpj status speckit:001-photo-albums)" 'v["result"]["warrant"]["state"]')
RP_SS=$(rpw status 2>/dev/null)
RP_SJ=$(rp_field "$(rpj status)" 'len(v["result"]["read_in_place"]["warrants"])')
RP_CHK=$(rpw check 2>&1); RP_CS=$?
if [[ "$RP_I1" == "openspec:add-2fa,openspec:add-2fa/1.1,openspec:add-2fa/1.2,openspec:add-2fa/2.1,openspec:add-2fa/2.2" \
    && "$RP_I2" == "T003,T004,T005" && "$RP_ST" == in_progress && "$RP_SJ" == 4 && $RP_CS -eq 0 ]] \
    && grep -qF '1/4 tasks done' <<<"$RP_SH" && grep -qF 'nothing here is written' <<<"$RP_SH" \
    && grep -qF '## Read in place' <<<"$RP_SS" && grep -q '^PASS adapter.read' <<<"$RP_CHK"; then
    rp_ok "impact, show, status, check" "session-expiry reaches add-2fa and its 4 tasks; US1 reaches T003, T004 and T005 (depends on T004)"
else
    rp_fail "impact, show, status, check" "impact '$RP_I1' / '$RP_I2'; status $RP_ST $RP_SJ; check $RP_CS $(grep -E 'adapter|openspec|speckit' <<<"$RP_CHK" | head -2 | tr '\n' ' ')"
fi

# 4. Nothing was written: both folders byte for byte, the program clean.
RP_B1=$(rp_bytes)
RP_GIT=$(git -C "$PLANT_ROOT" status --porcelain)
if [[ -n "$RP_B0" && "$RP_B0" == "$RP_B1" && -z "$RP_GIT" ]]; then
    rp_ok "the folders are never written" "$(wc -l <<<"$RP_B0") files byte-identical after list, model, impact, show, status, check; tree clean"
else
    rp_fail "the folders are never written" "changed: $(diff <(echo "$RP_B0") <(echo "$RP_B1") | head -3 | tr '\n' ' ') git: $RP_GIT"
fi

# 5. Refused by rule: a malformed task list in either folder (check fails
#    and names the line; the list still reads the rest), a path that is not
#    there, a kind this build does not read, an id no adapter reads.
printf -- '- [y] 3.1 A bad box\n' >> "$PLANT_ROOT/openspec/changes/add-2fa/tasks.md"
printf -- '- [ ] Untracked task with no id\n' >> "$PLANT_ROOT/specs/001-photo-albums/tasks.md"
RP_E1=$(rpw check 2>&1); RP_S1=$?
RP_LS2=$(rp_field "$(rpj warrants)" 'len([w for w in v["result"]["warrants"] if w["encoding"] == "read_in_place"])')
RP_MD=$(rp_field "$(rpj model)" '",".join(sorted(d["rule"] for d in v["result"]["diagnostics"]))')
git -C "$PLANT_ROOT" checkout -q -- openspec specs
printf '\n[[adapters]]\nkind = "speckit"\npath = "no-such-folder"\n\n[[adapters]]\nkind = "jira"\npath = "specs"\n' \
    >> "$PLANT_ROOT/openwarrant.toml"
RP_E2=$(rpw check 2>&1); RP_S2=$?
git -C "$PLANT_ROOT" checkout -q -- openwarrant.toml
RP_E3=$(rpw show openspec:no-such-change 2>&1 >/dev/null); RP_S3=$?
if [[ $RP_S1 -ne 0 && "$RP_LS2" == 4 && $RP_S2 -ne 0 && $RP_S3 -eq 2 ]] \
    && grep -qF 'openspec.tasks-malformed' <<<"$RP_E1" && grep -qF 'openspec/changes/add-2fa/tasks.md:12' <<<"$RP_E1" \
    && grep -qF 'speckit.tasks-malformed' <<<"$RP_E1" && grep -qF 'specs/001-photo-albums/tasks.md:29' <<<"$RP_E1" \
    && [[ "$RP_MD" == "openspec.tasks-malformed,speckit.tasks-malformed" ]] \
    && grep -q '^ERROR adapter.path-missing' <<<"$RP_E2" && grep -q '^ERROR adapter.kind-unknown' <<<"$RP_E2" \
    && grep -qF 'adapter.unknown' <<<"$RP_E3"; then
    rp_ok "refused by rule" "a [y] box (tasks.md:12), a task with no id (tasks.md:29), a missing path, kind jira, an unknown id; the rest still listed"
else
    rp_fail "refused by rule" "malformed $RP_S1 listed $RP_LS2 model '$RP_MD'; config $RP_S2 $(grep -E '^ERROR adapter' <<<"$RP_E2" | tr '\n' ' '); unknown $RP_S3 '$RP_E3'"
fi

corpus_gone "$PLANT_ROOT"
unset RP_FIX RP_ERR0 RP_S0 RP_B0 RP_B1 RP_LS RP_LS2 RP_MODEL RP_REC RP_REL RP_DIAG RP_I1 RP_I2 RP_SH RP_ST RP_SS \
    RP_SJ RP_CHK RP_CS RP_GIT RP_E1 RP_E2 RP_E3 RP_S1 RP_S2 RP_S3 RP_MD
