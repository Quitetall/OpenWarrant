#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Opt-in real local model experiment. No scripted replies or cloud fallback.

Requires an already isolated, local-only LAMU service; does not start, stop or
reconfigure any service. Caller declares its resource envelope separately.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import urllib.parse
import urllib.request
import uuid


def run(binding, root):
    binary = Path(os.environ['OW_KATANA_CLI'])
    url = os.environ['OW_KATANA_LOCAL_URL']
    model = os.environ['OW_KATANA_LOCAL_MODEL']
    parsed = urllib.parse.urlsplit(url)
    if (parsed.scheme != 'http' or parsed.hostname != '127.0.0.1'
            or parsed.username or parsed.password or parsed.path not in ('', '/')
            or parsed.query or parsed.fragment):
        raise ValueError('explicit localhost service origin required')
    if not all(p.is_absolute() for p in (binding, root, binary)):
        raise ValueError('absolute paths required')
    if binding.is_symlink() or not binding.is_file() or binding.stat().st_size > 32768:
        raise ValueError('bounded regular binding file required')
    # Whitelist environment; do not inherit provider credentials or proxy routes.
    names = ('HOME', 'PATH', 'LANG', 'XDG_CONFIG_HOME', 'XDG_DATA_HOME',
             'XDG_STATE_HOME', 'XDG_CACHE_HOME', 'XDG_RUNTIME_DIR', 'LAMU_SETTINGS',
             'LAMU_REGISTRY', 'LAMU_DB', 'LAMU_LLAMACPP_DIR')
    env = {key: os.environ[key] for key in names if key in os.environ}
    for key in ('LAMU_SETTINGS', 'LAMU_REGISTRY', 'LAMU_DB', 'HOME'):
        if key not in env:
            raise ValueError('explicit isolated LAMU environment required: ' + key)
    env.update(LAMU_SERVE_AUTOSTART='0', LAMU_CONTEXT_URL=url,
               LAMU_SERVE_URL=url, KATANA_BASE_URL=url)
    config = subprocess.run(['lamu', 'config', 'show', '--json'], env=env,
                            capture_output=True, timeout=15, check=True)
    settings = json.loads(config.stdout)
    # This command's structured result is retained for inspection. Check the
    # actual settings file too; an auto route may silently select a cloud model.
    import tomllib
    with open(env['LAMU_SETTINGS'], 'rb') as source:
        if tomllib.load(source).get('routing_mode') != 'local-only':
            raise ValueError('local-only routing required')
    opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))
    with opener.open(url.rstrip('/') + '/v1/models', timeout=10) as response:
        models = json.load(response)['data']
    if len(models) != 1 or models[0].get('id') != model or models[0].get('owned_by') != 'local':
        raise ValueError('isolated single local model required')
    root.mkdir(mode=0o700)
    (root / 'local-config.json').write_text(json.dumps(settings, indent=2))
    workspace = root / 'workspace'
    workspace.mkdir(mode=0o700)
    expected = {
        'calc.py': 'def add(a, b):\n    return a + b\n',
        'test_calc.py': ('import unittest\nfrom calc import add\nclass TestAdd(unittest.TestCase):\n'
                         '    def test_positive(self): self.assertEqual(add(3,4),7)\n'
                         '    def test_mixed(self): self.assertEqual(add(-2,1),-1)\n'),
        'Makefile': 'test:\n\tpython3 -m unittest -q\n',
    }
    for name, text in expected.items():
        (workspace / name).write_text(text.replace('return a + b', 'return a - b') if name == 'calc.py' else text)
    argv = [str(binary), '--base-url', url, '--model', model, '--cwd', str(workspace),
            '--policy', 'builder', '--sandbox', 'bubblewrap', '--tier', 'service',
            '--runtime-profile', 'reliable', '--wall-timeout-secs', '150',
            '--provider-timeout-secs', '60', '--tool-timeout-secs', '15',
            '--main-max-tokens', '512', '--max-turns', '4', '--max-tool-calls', '4',
            '--context-budget-bytes', '24000', '--locate-code', 'false',
            '--repo-map', '--oracles', '--oracle-timeout-secs', '15', '--lamu-mcp',
            '--result', '--new-session-path', str(root / 'session.jsonl'),
            '--openwarrant-binding', str(binding), '--openwarrant-receipt', str(root / 'receipt.json'),
            'Objective: Fix add in calc.py so add(a, b) returns a + b. Ownership: modify only calc.py; '
            'do not change test_calc.py or Makefile. Use exactly three tool calls: read calc.py '
            'only, edit its return expression, then bash make test. Do not read tests or Makefile; '
            'their two unittest cases expect add(3,4)=7 and add(-2,1)=-1. After tests pass, '
            'return your final answer without another tool call. No other files, no network, no push. '
            'Stop when tests pass or limits '
            'are reached. This is an unverified prototype fixture, not authorization or assurance.']
    (root / 'argv.json').write_text(json.dumps(argv, indent=2))
    (root / 'binding.json').write_bytes(binding.read_bytes())
    result = subprocess.run(argv, env=env, capture_output=True, timeout=165)
    (root / 'cli.stdout').write_bytes(result.stdout)
    (root / 'cli.stderr').write_bytes(result.stderr)
    if result.returncode:
        raise ValueError('actual CLI failed: ' + result.stderr.decode(errors='replace')[-1500:])
    # Never execute arbitrary model-produced code on the host. First require
    # exact bytes for the one permitted arithmetic change and protected fixtures.
    verified = root / 'verified-workspace'
    verified.mkdir(mode=0o700)
    for name, text in expected.items():
        data = (workspace / name).read_bytes()
        if (workspace / name).is_symlink() or data != text.encode():
            raise ValueError('task or protected fixture differs: ' + name)
        (verified / name).write_bytes(data)
    # Exclude any agent-created bytecode, startup file or command shim. Only
    # byte-checked source files enter this fresh host verification directory.
    test_env = {'PATH': '/usr/bin:/bin', 'HOME': env['HOME'], 'LANG': 'C.UTF-8',
                'PYTHONPYCACHEPREFIX': str(root / 'verified-bytecode')}
    tests = subprocess.run(['/usr/bin/make', 'test'], cwd=verified, env=test_env,
                           capture_output=True, timeout=15)
    (root / 'tests.log').write_bytes(tests.stdout + tests.stderr)
    if tests.returncode:
        raise ValueError('independent test command failed')
    receipt = json.loads((root / 'receipt.json').read_text())
    (root / 'fixture.json').write_text(json.dumps({
        'schema': 'katana/openwarrant-real-local-experiment/v1',
        'expected_log_head': receipt['event_log_head'],
        'scripted_provider': False, 'task_verified': True, 'cost_metered': False,
        'assurance': 'not-established', 'producer': 'actual-katana-cli',
        'cli_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
        'model': model, 'custody': 'test instrumentation only',
    }, indent=2) + '\n')


def main():
    if len(sys.argv) != 3:
        raise ValueError('BINDING_JSON NEW_OUTPUT_DIRECTORY required')
    binding, root = map(Path, sys.argv[1:])
    try:
        run(binding, root)
    finally:
        if root.exists() and (destination := os.environ.get('OW_KATANA_REAL_RETAIN')):
            retained = Path(destination) / ('real-local-' + uuid.uuid4().hex)
            shutil.copytree(root, retained, symlinks=True)
    print('Real local model fixture passed; native import still requires assessment; no assurance')


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        print(f'Real local fixture refused: {error}', file=sys.stderr)
        sys.exit(1)
