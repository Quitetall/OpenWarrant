#!/usr/bin/env bash
# The fixture agent `war eval` runs for free: a drafter that answers the
# request with the task's committed proposal, and a performer that copies the
# task's committed delivery into the scratch program. No model, no network,
# byte-deterministic — which is what makes the baseline comparable.
#
#   draft   (default): request on stdin  → proposal on stdout, {{NS}} filled
#   perform          : oh.war/eval-perform/v1 on stdin → files under deliver/
#                      copied into the cwd; {{ALIAS}} and {{DIGEST:<path>}}
#                      filled in any *.tmpl, which is written without .tmpl
set -euo pipefail
: "${EVAL_TASK_DIR:?EVAL_TASK_DIR is set by war eval}"
mode="${1:-draft}"
input=$(cat)
case "$mode" in
    draft)
        # The namespace never meets a shell or sed: python reads the request
        # and writes the proposal.
        python3 -c '
import json, pathlib, sys
ns = json.loads(sys.argv[2])["namespace"]
sys.stdout.write(pathlib.Path(sys.argv[1]).read_text().replace("{{NS}}", ns))
' "$EVAL_TASK_DIR/proposal.json" "$input"
        ;;
    perform)
        alias=$(python3 -c 'import json,sys; print(json.loads(sys.argv[1])["warrant"])' "$input")
        if [[ -d "$EVAL_TASK_DIR/deliver" ]]; then
            cp -R "$EVAL_TASK_DIR/deliver/." .
            python3 - "$alias" <<'PY'
import hashlib, pathlib, re, sys
alias = sys.argv[1]
for tmpl in pathlib.Path(".").rglob("*.tmpl"):
    text = tmpl.read_text().replace("{{ALIAS}}", alias)
    here = pathlib.Path(".").resolve()
    def digest(m):
        p = pathlib.Path(m.group(1))
        if not p.resolve().is_relative_to(here):
            raise SystemExit(f"fixture-agent: refusing to digest {p}: outside the scratch program")
        return "sha256:" + hashlib.sha256(p.read_bytes()).hexdigest()
    text = re.sub(r"\{\{DIGEST:([^}]+)\}\}", digest, text)
    out = pathlib.Path(str(tmpl)[: -len(".tmpl")].replace("WARRANT_ALIAS", alias))
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(text)
    tmpl.unlink()
    parent = tmpl.parent
    while parent != pathlib.Path('.') and not any(parent.iterdir()):
        parent.rmdir(); parent = parent.parent
PY
        fi
        printf '{"schema":"oh.war/eval-performed/v1","warrant":"%s"}\n' "$alias"
        ;;
    *)
        echo "fixture-agent: unknown mode $mode" >&2
        exit 2
        ;;
esac
