# SPDX-License-Identifier: Apache-2.0
"""Shipped CLI controls; owned local Git fixtures, no signing or model calls."""
import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

project, binary = (Path(p).resolve() for p in sys.argv[1:])
fixture = project / 'conformance/fixtures/inbox/repository'
alias = 'IX-WAR-0003'
expected = '---\nclassification: internal\n---\nCheckpoint two.\n'


def git(root, *args):
    result = subprocess.run(['git', '-c', 'core.hooksPath=/dev/null', '-c', 'user.name=Fixture',
                             '-c', 'user.email=fixture@example.invalid', *args], cwd=root,
                            capture_output=True, text=True, timeout=10)
    assert result.returncode == 0, result.stderr[:1024]
    return result.stdout.strip()


def war(root, *args, refusal=None):
    result = subprocess.run([str(binary), '--root', str(root), *args, '--json'],
                            capture_output=True, timeout=20)
    report = json.loads(result.stdout)
    if refusal:
        assert result.returncode != 0 and report['exit_code'] == result.returncode and report['counts']['error'] > 0, (result.returncode, report)
        assert refusal in result.stdout.decode(), report
    else:
        assert result.returncode == 0, (report, result.stderr[:1024])
    return report


with tempfile.TemporaryDirectory(prefix='ow-workspace-control-') as temporary:
    root = Path(temporary)
    repo, workspace = root / 'repo', root / 'workspace'
    shutil.copytree(fixture, repo)
    graph = repo / 'docs/warrants' / alias / 'atoms/45-milestones.yaml'
    graph.write_text(graph.read_text().replace('executor_kind: "agent"',
        'executor_kind: "agent"\n    executor_ref: "agent://fixture"\n    context_artifacts: ["notes.md"]'))
    (repo / 'notes.md').write_text(expected.replace('two', 'one'))
    git(repo, 'init', '-q')
    git(repo, 'add', '.')
    git(repo, 'commit', '-qm', 'original')
    git(repo, 'worktree', 'add', '--detach', str(workspace), 'HEAD')
    (workspace / 'notes.md').write_text(expected)
    git(workspace, 'add', 'notes.md')
    git(workspace, 'commit', '-qm', 'checkpoint')
    checkpoint = git(workspace, 'rev-parse', 'HEAD')
    dispatch, context, bundle = (root / f'{p}.json' for p in ('dispatch', 'context', 'bundle'))
    war(repo, 'dispatch', alias, 'STAGE-001', '--prototype', '--source-workspace', str(workspace),
        '--emit', str(dispatch), '--emit-context', str(context))
    packet = json.loads(dispatch.read_bytes())
    manifest = json.loads(context.read_bytes())
    item = next(i for i in manifest['included'] if i['id'] == 'notes.md')
    assert item['holder']['commit_sha'] == checkpoint
    assert packet['dispatch_id'] in war(repo, 'admin', 'journal', alias)['result']['rendered']
    assert packet['dispatch_id'] not in war(workspace, 'admin', 'journal', alias)['result']['rendered']
    packaged = war(repo, 'dispatch-bundle', 'create', alias, '--dispatch', str(dispatch),
                   '--context', str(context), '--emit', str(bundle), '--source-workspace', str(workspace))
    result = war(root, 'dispatch-bundle', 'check', str(bundle), '--expected-digest',
                 packaged['result']['bundle_digest'], '--read', 'notes.md')['result']
    assert result['content'] == expected and result['execution_authorized'] is False

    # A later clean checkpoint cannot be substituted into an earlier packet.
    (workspace / 'notes.md').write_text(expected.replace('two', 'three'))
    git(workspace, 'add', 'notes.md')
    git(workspace, 'commit', '-qm', 'later checkpoint')
    refused_bundle = root / 'refused-bundle.json'
    war(repo, 'dispatch-bundle', 'create', alias, '--dispatch', str(dispatch), '--context', str(context),
        '--emit', str(refused_bundle), '--source-workspace', str(workspace),
        refusal='bundle-workspace-checkpoint-mismatch')
    assert not refused_bundle.exists()

    def refuse_source(source, rule):
        d, c = root / 'refused-dispatch.json', root / 'refused-context.json'
        war(repo, 'dispatch', alias, 'STAGE-001', '--prototype', '--source-workspace', str(source),
            '--emit', str(d), '--emit-context', str(c), refusal=rule)
        assert not d.exists() and not c.exists()
        assert war(repo, 'admin', 'journal', alias)['result']['rendered'].count('dispatch.compiled') == 1

    # Changed contract, dirty code and another repository remain distinct refusals.
    intent = workspace / 'docs/warrants' / alias / 'atoms/10-intent.md'
    original_intent = intent.read_bytes()
    intent.write_bytes(original_intent + b'\nChanged contract.\n')
    git(workspace, 'add', '.')
    git(workspace, 'commit', '-qm', 'changed contract')
    refuse_source(workspace, 'workspace.contract-mismatch')
    intent.write_bytes(original_intent)
    git(workspace, 'add', '.')
    git(workspace, 'commit', '-qm', 'restore fixture contract')
    (workspace / 'notes.md').write_text('uncommitted context\n')
    refuse_source(workspace, 'workspace.dirty')
    other = root / 'other'
    shutil.copytree(repo, other, ignore=shutil.ignore_patterns('.git'))
    git(other, 'init', '-q')
    git(other, 'add', '.')
    git(other, 'commit', '-qm', 'unrelated repository')
    refuse_source(other, 'workspace.repository-mismatch')

print('PASS: checkpoint bytes and original-project retention; four specific refusal controls')
