# shellcheck shell=bash
# OW-WAR-0127 — a deliverable a native system holds is a reference, never a
# repository file (RQ-065; OW-ADR-0026).
#
# One scratch program, one Warrant, two deliverables: D-001 names an artifact
# a dataset registry holds, D-002 is a file in the repository. A file is
# planted at the path D-001's text would name if it were joined to the root
# (`<root>/external:/dataset-registry/ds-42`): any reader that still joins a
# native reference to the root reads THAT file, and the cases below see its
# bytes or its digest move something. D-002 is the control each time — the
# same command still reads, pins and drifts a repository file as before.
#
# The form of the reference is OW-WAR-0127's blocking question Q-001, and
# OW-ADR-0026 records it. The cases write form (a), `external://<system>/<id>`,
# the ADR's recommendation. While that ADR is not `accepted` the form is
# undecided, the obligations' scope ("the reference form in OW-ADR-0026") is
# empty, and no case is counted: each prints UNKNOWN with what it observed
# (Law 15 — not a pass, and not a failure of work that could not start).
# Once the owner accepts the ADR, every case counts. If the accepted form is
# not (a), NA_REF below is rewritten to it in the same change.

echo "== native-held deliverables (OW-WAR-0127) =="

PLANT_ROOT=$(scratch_corpus NA)
# Sourced outside plant.sh, PLANT_ROOT is empty and every `git -C` below
# would act on the real repository. Refuse.
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }

NA_ADR="$REPO_ROOT/docs/adr/atoms/OW-ADR-0026-native-held-deliverables.md"
NA_DECIDED=0
if [[ -f "$NA_ADR" ]] && grep -qx 'status: accepted' "$NA_ADR"; then NA_DECIDED=1; fi

NA_W=NA-WAR-0001
NA_REF="external://dataset-registry/ds-42"
NA_HOLDER="dataset-registry"
NA_JOINED="$PLANT_ROOT/external:/dataset-registry/ds-42"
NA_TOML="$PLANT_ROOT/docs/warrants/$NA_W/deliverables.toml"
# What the holder reported as the artifact's digest. Nothing here can check
# it (Q-002); the plant only watches that nothing rewrites it.
NA_REC="sha256:$(printf 'held by the registry\n' | sha256sum | cut -d' ' -f1)"
NA_PLANTED="planted at the joined path"
NA_PLANTED_SHA=$(printf '%s\n' "$NA_PLANTED" | sha256sum | cut -d' ' -f1)
NA_D2_SHA=$(printf 'hello\n' | sha256sum | cut -d' ' -f1)
NA_D2_STALE="sha256:$(printf 'stale\n' | sha256sum | cut -d' ' -f1)"

# na_expect <name> <ok|fail> <detail>: counted once the form is decided;
# UNKNOWN, and not counted, before.
na_expect() {
    if [[ "$NA_DECIDED" -eq 1 ]]; then
        if [[ "$2" == ok ]]; then
            printf 'ok    %-34s %s\n' "$1" "$3"
            PASSED=$((PASSED + 1))
        else
            printf 'FAIL  %-34s %s\n' "$1" "$3"
            FAILED=$((FAILED + 1))
        fi
    else
        printf 'UNKNOWN %-32s OW-ADR-0026 not accepted (Q-001 unanswered), not counted; observed %s: %s\n' \
            "$1" "$([[ "$2" == ok ]] && echo met || echo UNMET)" "$3"
    fi
}

na_w() { "$WAR" --root "$PLANT_ROOT" "$@" 2>&1; }

# na_deliverables [key=value ...]: write the Warrant's deliverables.toml.
#   prov=yes|no  preq=true|false  holder=<s>  ca=true|false  digest=<s>
#   req=true|false (D-001 required)  ref=<target_ref of D-001>
na_deliverables() {
    local prov=yes preq=true holder="$NA_HOLDER" ca=true digest="$NA_REC" req=true ref="$NA_REF" kv
    for kv in "$@"; do
        case "$kv" in
            prov=*) prov=${kv#prov=} ;;
            preq=*) preq=${kv#preq=} ;;
            holder=*) holder=${kv#holder=} ;;
            ca=*) ca=${kv#ca=} ;;
            digest=*) digest=${kv#digest=} ;;
            req=*) req=${kv#req=} ;;
            ref=*) ref=${kv#ref=} ;;
        esac
    done
    {
        printf 'schema = "oh.war/deliverables/v1"\n\n'
        printf '[[deliverable]]\nid = "D-001"\ntitle = "The dataset"\nkind = "file"\n'
        printf 'target_ref = "%s"\nrequired = %s\ncontent_addressed = %s\n' "$ref" "$req" "$ca"
        printf 'provenance_required = %s\nobligation_refs = ["OBL-001"]\n' "$preq"
        if [[ "$prov" == yes ]]; then
            printf '\n[deliverable.provenance]\nproducer = "agent://plant"\nproducing_attempt = "att-1"\n'
            printf 'contract_digest = "plant"\ntool_or_runtime_identity = "plant"\ncreation_method = "registered"\n'
            printf 'content_digest = "%s"\nmedia_type = "application/octet-stream"\n' "$digest"
            printf 'classification = "internal"\nretention = "permanent"\nsource_holder = "%s"\n' "$holder"
        fi
        printf '\n[[deliverable]]\nid = "D-002"\ntitle = "A repository file"\nkind = "file"\n'
        printf 'target_ref = "out/report.txt"\nrequired = true\ncontent_addressed = true\n'
        printf 'provenance_required = true\nobligation_refs = ["OBL-002"]\n'
        printf '\n[deliverable.provenance]\nproducer = "agent://plant"\nproducing_attempt = "att-1"\n'
        printf 'contract_digest = "plant"\ntool_or_runtime_identity = "plant"\ncreation_method = "authored"\n'
        printf 'content_digest = "%s"\nmedia_type = "text/plain"\n' "$NA_D2_STALE"
        printf 'classification = "internal"\nretention = "permanent"\nsource_holder = "git"\n'
    } >"$NA_TOML"
}

# na_digest <D-id>: the content_digest recorded for it now.
na_digest() {
    python3 - "$NA_TOML" "$1" <<'PY'
import sys, tomllib
d = tomllib.load(open(sys.argv[1], "rb"))
for x in d.get("deliverable", []):
    if x.get("id") == sys.argv[2]:
        print((x.get("provenance") or {}).get("content_digest", ""))
PY
}

# na_status_view <status-json>: D-001's view, as "<digest state>|<holder named>".
# Holder named: some value other than id, title and target_ref is the holder,
# or says "held by". The reference itself contains the holder's name, so it
# does not count.
na_status_view() {
    python3 -c '
import json, sys
holder = sys.argv[1]
try:
    d = json.loads(sys.stdin.read())
except Exception:
    print("unparsable|no"); sys.exit()
for w in (d.get("result") or {}).get("warrants") or []:
    for v in w.get("deliverables") or []:
        if v.get("id") == "D-001":
            rest = [str(x) for k, x in v.items() if k not in ("id", "title", "target_ref")]
            named = any(x == holder or "held by" in x.lower() for x in rest)
            print("%s|%s" % (v.get("digest"), "yes" if named else "no")); sys.exit()
print("absent|no")' "$NA_HOLDER"
}

# The baseline: the two deliverables, the repository file, and the planted
# file at the joined path, committed so `corpus_reset` returns to exactly this.
mkdir -p "$PLANT_ROOT/out" "$(dirname "$NA_JOINED")"
printf 'hello\n' >"$PLANT_ROOT/out/report.txt"
printf '%s\n' "$NA_PLANTED" >"$NA_JOINED"
na_deliverables
git -C "$PLANT_ROOT" add -A >/dev/null 2>&1
git -C "$PLANT_ROOT" -c user.email=plant@invalid -c user.name=plant commit -qm "a native and a repository deliverable" >/dev/null 2>&1 \
    || { printf 'PLANT SETUP FAILED: baseline commit in %s\n' "$PLANT_ROOT" >&2; exit 9; }
[[ -f "$NA_JOINED" ]] || { printf 'PLANT SETUP FAILED: nothing planted at %s\n' "$NA_JOINED" >&2; exit 9; }

# ── OBL-001: a native reference must name a holder that is not Git ─────────

corpus_reset "$PLANT_ROOT"
command rm -f "$NA_JOINED"
NA_OUT=$(na_w check "$NA_W"); NA_ST=$?
if [[ $NA_ST -eq 0 ]] && ! line_has -E '^(ERROR|WARN) ' -F 'D-001' <<<"$NA_OUT"; then
    na_expect "native with a non-git holder" ok "war check exit 0, no ERROR or WARN names D-001"
else
    na_expect "native with a non-git holder" fail "exit $NA_ST; $(grep -m1 -E '^(ERROR|WARN) .*D-001' <<<"$NA_OUT" | cut -c1-160)"
fi

# na_refused <name> <detail-pattern> <deliverables args...>: war check exits
# non-zero with an ERROR line naming D-001 that matches the detail.
na_refused() {
    local name="$1" detail="$2"
    shift 2
    corpus_reset "$PLANT_ROOT"
    command rm -f "$NA_JOINED"
    na_deliverables "$@"
    NA_OUT=$(na_w check "$NA_W"); NA_ST=$?
    if [[ $NA_ST -ne 0 ]] && line_has -E '^ERROR .*D-001' -iE "$detail" <<<"$NA_OUT"; then
        na_expect "$name" ok "refused: $(grep -m1 -E '^ERROR .*D-001' <<<"$NA_OUT" | awk '{print $2}')"
    else
        na_expect "$name" fail "exit $NA_ST; no ERROR naming D-001 and /$detail/ ($(grep -m1 -E '^(ERROR|WARN) .*D-001' <<<"$NA_OUT" | awk '{print $2}'))"
    fi
}
# The record a native reference must not be: no provenance at all, which a
# repository path may be (the control below).
na_refused "native with no provenance" 'provenance' prov=no preq=false ca=false
na_refused "native with Source Holder git" 'holder' holder=git
na_refused "native with an empty Source Holder" 'holder' holder=
# The refusal is the named one, "records no content digest"
# (ContentAddressedWithoutDigest). `deliverable.unsupported-digest` also
# fires on an empty digest, and it is not this rule: it says the algorithm
# is unsupported, which is not what is wrong.
na_refused "content-addressed native, no digest" 'no content digest' preq=false digest=

# Control: the no-provenance record is refused because the reference is
# native. The same record naming a repository path passes as today.
corpus_reset "$PLANT_ROOT"
na_deliverables prov=no preq=false ca=false ref=out/report.txt
NA_OUT=$(na_w check "$NA_W"); NA_ST=$?
if [[ $NA_ST -eq 0 ]] && ! line_has -E '^ERROR ' -F 'D-001' <<<"$NA_OUT"; then
    na_expect "a repo path without provenance" ok "passes war check: the refusal above is specific to native references"
else
    na_expect "a repo path without provenance" fail "exit $NA_ST; $(grep -m1 -E '^ERROR .*D-001' <<<"$NA_OUT" | cut -c1-160)"
fi

# ── OBL-002: nothing reads, rewrites or corrects a native-held artifact ────

corpus_reset "$PLANT_ROOT"
NA_OUT=$(na_w pins --refresh --alias "$NA_W"); NA_ST=$?
NA_D1=$(na_digest D-001)
NA_D2=$(na_digest D-002)
if [[ "$NA_D1" == "$NA_REC" && "$NA_D2" == "sha256:$NA_D2_SHA" ]]; then
    na_expect "pins --refresh skips the native" ok "D-001 still $NA_REC; D-002 moved to its bytes"
else
    na_expect "pins --refresh skips the native" fail "exit $NA_ST; D-001 $NA_D1 (recorded ${NA_REC:0:19}…, planted file sha256:${NA_PLANTED_SHA:0:12}…); D-002 $NA_D2"
fi

corpus_reset "$PLANT_ROOT"
NA_OUT=$(na_w pins)
if line_has -F 'D-001' -iE "held by.*$NA_HOLDER" <<<"$NA_OUT"; then
    na_expect "pins lists the native as held" ok "$(grep -m1 -F 'D-001' <<<"$NA_OUT" | tr -s ' ')"
else
    na_expect "pins lists the native as held" fail "$(grep -m1 -F 'D-001' <<<"$NA_OUT" | tr -s ' ')"
fi

corpus_reset "$PLANT_ROOT"
NA_BEFORE=$(git -C "$PLANT_ROOT" status --porcelain --untracked-files=all | sort)
NA_OUT=$(na_w correct "$NA_W" D-001); NA_ST=$?
NA_AFTER=$(git -C "$PLANT_ROOT" status --porcelain --untracked-files=all | sort)
# The reference itself contains the holder's name, so it does not count as
# naming it. On a draft Warrant `correct` exits 2 for any deliverable (a
# correction is for a resolved pin), so the exit alone shows nothing: the
# refusal must name the holder and must not have read the joined file.
NA_NOREF=${NA_OUT//"$NA_REF"/}
if [[ $NA_ST -ne 0 && "$NA_BEFORE" == "$NA_AFTER" ]] && grep -qF "$NA_HOLDER" <<<"$NA_NOREF" \
    && ! grep -qF "$NA_PLANTED_SHA" <<<"$NA_OUT"; then
    na_expect "correct refuses the native" ok "exit $NA_ST, names $NA_HOLDER, tree unchanged"
else
    na_expect "correct refuses the native" fail "exit $NA_ST; tree moved: $([[ "$NA_BEFORE" == "$NA_AFTER" ]] && echo no || echo yes); holder named outside the reference: $(grep -qF "$NA_HOLDER" <<<"$NA_NOREF" && echo yes || echo no); joined file's digest in the output: $(grep -qF "$NA_PLANTED_SHA" <<<"$NA_OUT" && echo yes || echo no)"
fi
# Control: on the same Warrant, the request for the repository deliverable
# still reads its bytes, so the check above is not blind.
corpus_reset "$PLANT_ROOT"
NA_OUT=$(na_w correct "$NA_W" D-002)
if grep -qF "current_digest = \"sha256:$NA_D2_SHA\"" <<<"$NA_OUT"; then
    na_expect "correct still reads a repo file" ok "D-002's request carries its bytes' digest"
else
    na_expect "correct still reads a repo file" fail "$(grep -m1 -F 'current_digest' <<<"$NA_OUT")"
fi

corpus_reset "$PLANT_ROOT"
NA_OUT="$(na_w check "$NA_W")
$(na_w pins)"
if ! line_has -F 'D-001' -E 'pin-stale|digest-drift|target-unreadable|pins\.unreadable' <<<"$NA_OUT" \
    && line_has -F 'D-002' -F 'deliverable.pin-stale' <<<"$NA_OUT"; then
    na_expect "check finds no drift in the native" ok "nothing for D-001; D-002's stale pin still reported"
else
    na_expect "check finds no drift in the native" fail "$(grep -m1 -E '(pin-stale|digest-drift|target-unreadable|pins\.unreadable).*D-001' <<<"$NA_OUT" | awk '{print $2}'); D-002 pin-stale: $(line_has -F 'D-002' -F 'deliverable.pin-stale' <<<"$NA_OUT" && echo reported || echo missing)"
fi

# The bytes at the joined path change; nothing about D-001 may move. D-002's
# bytes change too, and its line must move: the capture is not blind.
na_capture() {
    local c s
    c=$(na_w check "$NA_W")
    s=$(na_w status --json)
    printf '%s\n' "$(grep -F "$1" <<<"$c")" "$(grep -F "$1" <<<"$(na_w pins)")"
    [[ "$1" == D-001 ]] && na_status_view <<<"$s"
}
corpus_reset "$PLANT_ROOT"
NA_A1=$(na_capture D-001); NA_A2=$(na_capture D-002)
printf 'the registry moved it\n' >"$NA_JOINED"
printf 'hello, edited\n' >"$PLANT_ROOT/out/report.txt"
NA_B1=$(na_capture D-001); NA_B2=$(na_capture D-002)
if [[ "$NA_A1" == "$NA_B1" && "$NA_A2" != "$NA_B2" ]]; then
    na_expect "joined-path bytes move nothing" ok "check, pins and status for D-001 identical; D-002 moved"
else
    na_expect "joined-path bytes move nothing" fail "D-001 moved: $([[ "$NA_A1" == "$NA_B1" ]] && echo no || echo yes); D-002 moved: $([[ "$NA_A2" == "$NA_B2" ]] && echo no || echo yes)"
fi

# ── OBL-003: a native deliverable neither passes nor reads as missing ──────

# na_req <resolve-output> <text>: the §56.1 line for a requirement.
na_req() { grep -m1 -F "$2" <<<"$1"; }

corpus_reset "$PLANT_ROOT"
NA_OUT=$(na_w resolve --dry-run "$NA_W")
NA_R2=$(na_req "$NA_OUT" 'required deliverables exist')
NA_R3=$(na_req "$NA_OUT" 'artifact digests verify')
if [[ -n "$NA_R2" && "$NA_R2" != PASS* && -n "$NA_R3" && "$NA_R3" != PASS* ]] \
    && line_has -E '^UNKNOWN ' -F "$NA_HOLDER" <<<"$NA_OUT"; then
    na_expect "requirements 2 and 3, native" ok "both unmet; an UNKNOWN names $NA_HOLDER"
else
    na_expect "requirements 2 and 3, native" fail "req 2: ${NA_R2%% *}; req 3: ${NA_R3%% *}; UNKNOWN naming $NA_HOLDER: $(line_has -E '^UNKNOWN ' -F "$NA_HOLDER" <<<"$NA_OUT" && echo yes || echo no)"
fi

corpus_reset "$PLANT_ROOT"
command rm -f "$NA_JOINED"
NA_OUT="$(na_w resolve --dry-run "$NA_W")
$(na_w check "$NA_W")"
NA_MISSING='does not exist|cannot be read|no such file|target-unreadable|missing'
if ! line_has -F 'D-001' -iE "$NA_MISSING" <<<"$NA_OUT" && ! line_has -F "$NA_REF" -iE "$NA_MISSING" <<<"$NA_OUT"; then
    na_expect "a native never reads as missing" ok "no line says D-001 is missing or unreadable"
else
    na_expect "a native never reads as missing" fail "$(grep -m1 -iE "(D-001|$NA_REF).*($NA_MISSING)" <<<"$NA_OUT" | cut -c1-160)"
fi

corpus_reset "$PLANT_ROOT"
NA_V1=$(na_w status --json | na_status_view)
command rm -f "$NA_JOINED"
NA_V2=$(na_w status --json | na_status_view)
na_held() { [[ "$1" == *'|yes' ]] && [[ "${1%%|*}" != @(verified|drift|corrected|target_unreadable|absent|unparsable) ]]; }
if na_held "$NA_V1" && na_held "$NA_V2"; then
    na_expect "status says held by its system" ok "D-001 is ${NA_V1%%|*}, naming $NA_HOLDER, with or without the joined file"
else
    na_expect "status says held by its system" fail "digest state with the joined file: $NA_V1; without: $NA_V2 (state|holder named)"
fi

# Control: the rule is about native references. D-001 not required, D-002
# present: requirement 2 is met.
corpus_reset "$PLANT_ROOT"
command rm -f "$NA_JOINED"
na_deliverables req=false
NA_OUT=$(na_w resolve --dry-run "$NA_W")
NA_R2=$(na_req "$NA_OUT" 'required deliverables exist')
if [[ "$NA_R2" == PASS* ]]; then
    na_expect "requirement 2 without a native" ok "met with D-001 not required and D-002 present"
else
    na_expect "requirement 2 without a native" fail "req 2: $(cut -c1-120 <<<"$NA_R2")"
fi

# ── OBL-004: the verifier is told what it cannot see ───────────────────────

corpus_reset "$PLANT_ROOT"
NA_OUT=$(na_w verify "$NA_W" --performer plant --bundle); NA_ST=$?
NA_BUNDLE=$(ls "$PLANT_ROOT/docs/warrants/$NA_W/verifications/"bundle-*.json 2>/dev/null | head -1)
NA_B=$(python3 - "${NA_BUNDLE:-/nonexistent}" "$NA_REC" "$NA_HOLDER" "$NA_PLANTED" "$NA_PLANTED_SHA" <<'PY'
import json, sys
path, rec, holder, planted, planted_sha = sys.argv[1:6]
try:
    b = json.load(open(path))
except Exception as e:
    print("no bundle"); sys.exit()
ds = {d.get("id"): d for d in b.get("deliverables") or []}
d1, d2 = ds.get("D-001"), ds.get("D-002")
why = []
if d1 is None:
    why.append("D-001 not carried")
else:
    s = json.dumps(d1)
    rest = [str(v) for k, v in d1.items() if k not in ("id", "title", "target_ref")]
    if planted in s or planted_sha in s: why.append("D-001 carries the joined file's bytes or digest")
    if d1.get("target_ref") != "external://dataset-registry/ds-42": why.append("no reference")
    if rec not in s: why.append("no recorded digest")
    if not any(v == holder or "held by" in v.lower() for v in rest): why.append("no holder")
    if not (d1.get("read") is False or "not read" in s.lower() or "not_read" in s.lower()): why.append("not marked not read")
if d2 is None or d2.get("text") != "hello\n":
    why.append("D-002's bytes not carried")
print("; ".join(why) if why else "ok")
PY
)
if [[ $NA_ST -eq 0 && "$NA_B" == ok ]]; then
    na_expect "bundle carries the reference" ok "D-001: reference, holder, recorded digest, marked not read, no bytes; D-002's bytes present"
else
    na_expect "bundle carries the reference" fail "exit $NA_ST; $NA_B"
fi

corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT
