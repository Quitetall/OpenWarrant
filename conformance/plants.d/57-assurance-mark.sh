# shellcheck shell=bash
# OW-WAR-0135 — the assurance mark: its decision record and baseline v1.
#
# OBL-001's structural half only. Whether the baseline is the right one, and
# the ADR's acceptance, are the owner's (document.review), and no plant can
# observe them. What a plant can observe: the ADR is an ADR `war check`
# reads, and the baseline names, for every requirement, the record that
# evidences it and what UNKNOWN means for it, in agreement with the ADR.
#
# OBL-002..OBL-005 exercise `war mark`, which OW-WAR-0135's work order holds
# until the owner accepts OW-ADR-0025 (M2 depends on M1). Their plants join
# this file with the command.

echo "== assurance mark: decision and baseline (OW-WAR-0135) =="
PLANT_ROOT=$(scratch_corpus AM)
[[ -d "${PLANT_ROOT:-}/.git" ]] || { printf 'PLANT SETUP FAILED: no scratch corpus (run through conformance/plant.sh)\n' >&2; exit 9; }
AM_ADR=docs/adr/atoms/OW-ADR-0025-assurance-mark.md
AM_BL=docs/assurance/baseline-v1.toml
[[ -f "$AM_ADR" && -f "$AM_BL" ]] || { printf 'PLANT SETUP FAILED: %s or %s missing\n' "$AM_ADR" "$AM_BL" >&2; exit 9; }
am_ok() { printf 'ok    %-34s %s\n' "$1" "$2"; PASSED=$((PASSED + 1)); }
am_fail() { printf 'FAIL  %-34s %s\n' "$1" "$2"; FAILED=$((FAILED + 1)); }

# The baseline's shape, and its agreement with the ADR. Prints nothing when
# the pair holds; otherwise one line per defect.
am_baseline_defects() { # baseline file, ADR file
    python3 - "$1" "$2" <<'PY'
import re, sys, tomllib
bl_path, adr_path = sys.argv[1], sys.argv[2]
try:
    bl = tomllib.load(open(bl_path, "rb"))
except Exception as e:
    print(f"baseline does not parse: {e}"); sys.exit(0)
adr = open(adr_path).read()
for key in ("schema", "id", "status", "independence_floor"):
    if not str(bl.get(key, "")).strip():
        print(f"baseline has no {key}")
if bl.get("status") not in ("proposed", "accepted"):
    print(f"baseline status {bl.get('status')!r} is neither proposed nor accepted")
reqs = bl.get("requirement", [])
if not reqs:
    print("baseline has no requirement")
seen = set()
for r in reqs:
    rid = str(r.get("id", "")).strip() or "<no id>"
    if rid in seen:
        print(f"{rid} appears twice")
    seen.add(rid)
    for field in ("check", "statement", "evidence", "unmet", "unknown"):
        if not str(r.get(field, "")).strip():
            print(f"{rid} has no {field}")
    if not re.search(r"(?m)^\| " + re.escape(rid) + r" \|", adr):
        print(f"{rid} is not in the ADR's baseline table")
for rid in re.findall(r"(?m)^\| (BL-\d+) \|", adr):
    if rid not in seen:
        print(f"the ADR's {rid} is not in the baseline")
# Q-001: the answer or the pending state, and every option with it.
for opt in ("### (a)", "### (b)", "### (c)"):
    if opt not in adr:
        print(f"the ADR does not record Q-001 option {opt[4:]}")
if not re.search(r"(?m)^## Decision$", adr):
    print("the ADR has no Decision section")
PY
}

# OBL-001, accept: the ADR is read as an ADR, beside the scratch program's own.
mkdir -p "$PLANT_ROOT/docs/adr/atoms"
command cp "$AM_ADR" "$PLANT_ROOT/docs/adr/atoms/"
AM_CHECK=$("$WAR" --root "$PLANT_ROOT" check 2>&1)
if grep -q 'adr.parsed' <<<"$AM_CHECK" && ! grep -q 'adr.malformed' <<<"$AM_CHECK"; then
    am_ok "the mark ADR is an ADR" "$(grep -o '[0-9]* ADR(s) parsed' <<<"$AM_CHECK" | head -1)"
else
    am_fail "the mark ADR is an ADR" "$(grep -E 'adr\.' <<<"$AM_CHECK" | head -2)"
fi
# Refusal: the same file with a status no ADR may hold is malformed, by name.
sed -i 's/^status: proposed$/status: decided-by-its-author/' "$PLANT_ROOT/docs/adr/atoms/OW-ADR-0025-assurance-mark.md"
AM_CHECK=$("$WAR" --root "$PLANT_ROOT" check 2>&1)
if grep -q 'adr.malformed' <<<"$AM_CHECK" && grep -q 'OW-ADR-0025' <<<"$(grep 'adr.malformed' <<<"$AM_CHECK")"; then
    am_ok "a mangled mark ADR is refused" "adr.malformed names OW-ADR-0025"
else
    am_fail "a mangled mark ADR is refused" "$(grep -E 'adr\.' <<<"$AM_CHECK" | head -2)"
fi

# OBL-001, accept: every requirement names its evidence and its UNKNOWN, and
# the baseline and the ADR list the same requirements.
AM_DEFECTS=$(am_baseline_defects "$AM_BL" "$AM_ADR")
if [[ -z "$AM_DEFECTS" ]]; then
    am_ok "baseline v1 names its evidence" "$(grep -c '^\[\[requirement\]\]' "$AM_BL") requirement(s), each with evidence, unmet, unknown"
else
    am_fail "baseline v1 names its evidence" "$(head -1 <<<"$AM_DEFECTS")"
fi
# Refusal: a requirement stripped of its evidence, and one the ADR never
# decided, are each named. The control against a predicate that passes all.
AM_BAD="$PLANT_ROOT/baseline-bad.toml"
python3 - "$AM_BL" "$AM_BAD" <<'PY'
import re, sys
t = open(sys.argv[1]).read()
t = re.sub(r'(?m)^evidence = .*\n', '', t, count=1)
t += '\n[[requirement]]\nid = "BL-099"\ntitle = "undecided"\ncheck = "x"\nstatement = "x"\nevidence = "x"\nunmet = "x"\nunknown = "x"\n'
open(sys.argv[2], "w").write(t)
PY
AM_DEFECTS=$(am_baseline_defects "$AM_BAD" "$AM_ADR")
if grep -q '^BL-001 has no evidence$' <<<"$AM_DEFECTS" && grep -q '^BL-099 is not in the ADR' <<<"$AM_DEFECTS"; then
    am_ok "an unevidenced requirement is named" "BL-001 has no evidence; BL-099 not in the ADR"
else
    am_fail "an unevidenced requirement is named" "${AM_DEFECTS:-no defect reported}"
fi

corpus_gone "$PLANT_ROOT"
unset PLANT_ROOT AM_CHECK AM_DEFECTS AM_BAD
