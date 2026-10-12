# shellcheck shell=bash
# Direct ingestion must refuse ambiguous answers before writing. Synthetic
# fixture actors are not production verdicts; no model or signing key is used.
echo "== direct verifier response ambiguity =="
DV_ROOT=$(mktemp -d)
python3 - "$REPO_ROOT" "$DV_ROOT" <<'COPY'
import shutil, sys
shutil.copytree(sys.argv[1] + "/conformance/fixtures/inbox/repository", sys.argv[2], dirs_exist_ok=True)
COPY
PLANT_ROOT="$DV_ROOT"
DV_EXIT=0
python3 - "$(realpath "$WAR")" "$DV_ROOT" <<'PROBE' || DV_EXIT=$?
import hashlib, json, os, pathlib, shutil, subprocess, sys
war, base = sys.argv[1], pathlib.Path(sys.argv[2])
def run(root, *args):
    result = subprocess.run([war, "--root", str(root), *args, "--json"], capture_output=True,
        timeout=30, env={**os.environ, "OPENWARRANT_NO_PROJECTS":"1", "OPENWARRANT_NO_UPDATE_CHECK":"1"})
    return json.loads(result.stdout)
def snapshot(root):
    subject = root / "docs/warrants/IX-WAR-0003"
    files = list((subject / "verifications").rglob("*")) + [subject / "journal.jsonl"]
    return {str(p.relative_to(subject)):hashlib.sha256(p.read_bytes()).hexdigest() for p in files if p.is_file()}
for bound in [False, True]:
    for conflicting in [False, True]:
        root = base / (f"case-{bound}-{conflicting}")
        # Copy only the original seed, not prior test cases.
        root.mkdir()
        for entry in list(base.iterdir()):
            if entry.name.startswith("case-"): continue
            if entry.is_dir(): shutil.copytree(entry, root/entry.name)
            else: shutil.copy2(entry, root/entry.name)
        fields = """obligation = "OBL-001"
disposition = "established"
evidence = "synthetic refusal fixture, not production assurance"
performer = "fixture-performer"
[verifications.verifier]
actor = "fixture-independent-verifier"
kind = "service"
[verifications.verifier.independence]
performer_transcript_blind = true
performer_rationale_blind = true
separate_writable_workspace = true
cannot_modify_subject_artifacts = true
cannot_modify_gate_definition = true
cannot_modify_gate_fixtures = true
separate_context_compilation = true
distinct_model_required = false
distinct_human_required = false
"""
        # JSON fields are TOML inline-table literals after conversion below.
        header = 'schema = "oh.war/verification-response/v1"\nwarrant = "IX-WAR-0003"\n'
        if bound:
            packet = run(root, "verify", "IX-WAR-0003", "--performer", "fixture-performer", "--bundle")
            assert packet["exit_code"] == 0, packet
            result = packet["result"]
            def literal(v):
                if isinstance(v, dict): return "{" + ", ".join(json.dumps(k)+" = "+literal(x) for k,x in v.items()) + "}"
                if isinstance(v, list): return "[" + ", ".join(literal(x) for x in v) + "]"
                if v is None: raise AssertionError("unexpected null field")
                return json.dumps(v)
            header = 'schema = "oh.war/verification-response/v2"\nwarrant = "IX-WAR-0003"\n'
            header += "reviewed_subject = " + literal(result["reviewed_subject"]) + "\n"
            header += "reviewed_packets = " + literal(result["packets"]) + "\n"
        peer = fields.replace("OBL-001", "OBL-002")
        valid = header + "\n[[verifications]]\n" + fields + "\n[[verifications]]\n" + peer
        duplicate = fields.replace('disposition = "established"', 'disposition = "not_established"') if conflicting else fields
        response = base / "response.toml"
        response.write_text(valid + "\n[[verifications]]\n" + duplicate)
        before = snapshot(root)
        result = run(root, "verify", "IX-WAR-0003", "--response", str(response))
        assert snapshot(root) == before, (bound, conflicting, "record/journal changed", result)
        assert result["exit_code"] == 2 and any(d["rule"] == "verify.duplicate-obligation" for d in result["diagnostics"]), result
        if bound:
            response.write_text(valid)
            accepted = run(root, "verify", "IX-WAR-0003", "--response", str(response))
            assert accepted["exit_code"] == 0 and accepted["counts"]["pass"] == 2, accepted
        print(f"ok: bound={bound}, conflicting={conflicting}, ambiguous response unchanged; valid restoration accepted when bound")
PROBE
if [[ "$DV_EXIT" -eq 0 ]]; then
    printf 'ok    direct response ambiguity: four refusals; bound positive restoration\n'
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  direct response ambiguity: CLI probe exited %s\n' "$DV_EXIT"
    FAILED=$((FAILED + 1))
fi
