# shellcheck shell=bash
# Configured verifier adapters cannot bypass the compiled reading budget.
# Public CLI fixture only: no model, signature, or production verdict.
echo "== verifier dispatch budget =="
BD_ROOT=$(mktemp -d)
PLANT_ROOT="$BD_ROOT"
BD_EXIT=0
python3 - "$(realpath "$WAR")" "$REPO_ROOT" "$BD_ROOT" <<'PROBE' || BD_EXIT=$?
import hashlib, json, os, pathlib, shutil, subprocess, sys
war, source, base = sys.argv[1], pathlib.Path(sys.argv[2]), pathlib.Path(sys.argv[3])
for limit in [1, 48000]:
    root = base / str(limit)
    shutil.copytree(source / "conformance/fixtures/inbox/repository", root)
    marker = root / "adapter-called"
    config = root / "openwarrant.toml"
    argv = ["sh", "-c", 'printf called > "$1"; exit 73', "fixture-adapter", str(marker)]
    text = config.read_text().replace("verifier_argv = []", "verifier_argv = " + json.dumps(argv))
    text = text.replace("[verify]", "[verify]\nmax_bundle_tokens = " + str(limit))
    config.write_text(text)
    git_env = {k:v for k,v in os.environ.items() if k not in ["GIT_DIR", "GIT_WORK_TREE"]}
    for args in [["init", "-q", "--template=", "--initial-branch=fixture"], ["add", "."], ["-c", "user.name=fixture", "-c", "user.email=fixture@invalid", "commit", "-qm", "budget fixture"]]:
        subprocess.run(["git", *args], cwd=root, env=git_env, check=True, timeout=30, capture_output=True)
    warrant = root / "docs/warrants/IX-WAR-0003"
    def snapshot():
        files = list((warrant / "verifications").rglob("*.toml")) + [warrant / "journal.jsonl"]
        return {str(p.relative_to(warrant)): hashlib.sha256(p.read_bytes()).hexdigest() for p in files if p.is_file()}
    before = snapshot()
    run = subprocess.run([war, "--root", str(root), "verify", "IX-WAR-0003", "--performer", "fixture-performer", "--run", "--json"], capture_output=True, timeout=30,
        env={**os.environ, "OPENWARRANT_NO_PROJECTS":"1", "OPENWARRANT_NO_UPDATE_CHECK":"1"})
    report = json.loads(run.stdout)
    assert run.returncode == 2, report
    if limit == 1:
        assert not marker.exists(), "over-budget invocation called the adapter"
        assert any(d["rule"] == "verify.bundle-over-budget" and d["severity"] == "error" for d in report["diagnostics"]), report
        packets = list((warrant / "verifications").glob("bundle-*.json"))
        assert packets and all(json.loads(p.read_bytes())["over_budget"] for p in packets), "exact refused packets must remain inspectable"
    else:
        assert marker.read_text() == "called", "in-budget invocation did not reach adapter"
    assert snapshot() == before, "dispatch failure changed verdicts or journal"
    assert not (warrant / "verifications/responses").exists(), "failure retained a response"
print("over-budget refuses without adapter or verdict writes; in-budget reaches adapter")
PROBE
if [[ "$BD_EXIT" -eq 0 ]]; then
    printf 'ok    %-34s %s\n' "verifier budget admission" "refusal and in-budget control observed"
    PASSED=$((PASSED + 1))
else
    printf 'FAIL  %-34s exit %s\n' "verifier budget admission" "$BD_EXIT"
    FAILED=$((FAILED + 1))
fi
corpus_gone "$BD_ROOT"
unset BD_ROOT BD_EXIT PLANT_ROOT
