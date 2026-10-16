#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Test-only Katana CLI producer for the opt-in OpenWarrant capture control.

Run as BINDING_JSON NEW_OUTPUT_DIRECTORY. OW_KATANA_CLI must name the real
built CLI. The only model responses come from this localhost scripted server.
This does not establish real-agent behavior, cost metering or assurance.
"""
import hashlib
import http.server
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import threading
import uuid


def main():
    if len(sys.argv) != 3:
        raise ValueError('BINDING_JSON NEW_OUTPUT_DIRECTORY required')
    binding, root = map(Path, sys.argv[1:])
    binary = Path(os.environ['OW_KATANA_CLI'])
    if not all(p.is_absolute() for p in (binding, root, binary)):
        raise ValueError('absolute paths required')
    if not binary.is_file():
        raise ValueError('explicit real Katana CLI is unavailable')
    root.mkdir(mode=0o700)
    workspace = root / 'workspace'
    workspace.mkdir(mode=0o700)
    (workspace / 'input.txt').write_text('actual bounded local fixture read\n')
    home = root / 'isolated-home'
    home.mkdir(mode=0o700)
    requests = []

    class Scripted(http.server.BaseHTTPRequestHandler):
        def log_message(self, *_args):
            pass

        def do_POST(self):
            size = int(self.headers.get('Content-Length', '0'))
            if self.path != '/v1/chat/completions' or not 0 < size <= 2 * 1024 * 1024:
                self.send_error(400, 'fixture request refused')
                return
            body = json.loads(self.rfile.read(size))
            if len(requests) >= 2:
                self.send_error(400, 'scripted fixture exhausted')
                return
            requests.append(body)
            if len(requests) == 1:
                delta = {'tool_calls': [{'index': 0, 'id': 'fixture-read',
                         'type': 'function', 'function': {'name': 'read',
                         'arguments': json.dumps({'path': 'input.txt'})}}]}
                reason = 'tool_calls'
            else:
                delta = {'content': 'local CLI fixture finished'}
                reason = 'stop'
            chunks = [
                {'choices': [{'delta': delta, 'finish_reason': None}]},
                {'choices': [{'delta': {}, 'finish_reason': reason}],
                 'usage': {'prompt_tokens': 1, 'completion_tokens': 1}},
            ]
            data = ''.join('data: ' + json.dumps(c) + '\n\n' for c in chunks)
            data = (data + 'data: [DONE]\n\n').encode()
            self.send_response(200)
            self.send_header('Content-Type', 'text/event-stream')
            self.send_header('Content-Length', str(len(data)))
            self.end_headers()
            self.wfile.write(data)

    server = http.server.HTTPServer(('127.0.0.1', 0), Scripted)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    env = dict(os.environ)
    for key in ('OPENAI_API_KEY', 'KATANA_API_KEY', 'ANTHROPIC_API_KEY', 'GEMINI_API_KEY'):
        env.pop(key, None)
    env.update(HOME=str(home), XDG_CONFIG_HOME=str(home / 'config'),
               XDG_CACHE_HOME=str(home / 'cache'), NO_PROXY='127.0.0.1,localhost',
               HTTP_PROXY='', HTTPS_PROXY='', ALL_PROXY='',
               http_proxy='', https_proxy='', all_proxy='',
               no_proxy='127.0.0.1,localhost')
    argv = [str(binary), '--base-url', f'http://127.0.0.1:{server.server_port}',
            '--model', 'scripted-local-openwarrant-cli-fixture', '--cwd', str(workspace),
            '--policy', 'reader', '--sandbox', 'none', '--result',
            '--new-session-path', str(root / 'session.jsonl'),
            '--openwarrant-binding', str(binding),
            '--openwarrant-receipt', str(root / 'receipt.json'),
            'Read input.txt, then finish this local fixture.']
    try:
        result = subprocess.run(argv, env=env, capture_output=True, timeout=25)
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=2)
    (root / 'cli.stdout').write_bytes(result.stdout)
    (root / 'cli.stderr').write_bytes(result.stderr)
    if result.returncode:
        sys.stderr.buffer.write(result.stderr)
        raise ValueError(f'actual Katana CLI exited {result.returncode}')
    if len(requests) != 2:
        raise ValueError('expected one real read and one terminal provider response')
    receipt = json.loads((root / 'receipt.json').read_text())
    (root / 'binding.json').write_bytes(binding.read_bytes())
    # Test instrumentation only: this head does not establish protected custody.
    (root / 'fixture.json').write_text(json.dumps({
        'schema': 'katana/openwarrant-local-fixture/v1',
        'expected_log_head': receipt['event_log_head'],
        'scripted_provider': True, 'cost_metered': False,
        'assurance': 'not-established', 'producer': 'actual-katana-cli',
        'provider_requests': len(requests),
        'cli_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
    }, indent=2) + '\n')
    if destination := os.environ.get('OW_KATANA_CLI_RETAIN'):
        retained = Path(destination) / ('native-cli-' + uuid.uuid4().hex)
        retained.mkdir(parents=True, mode=0o700)
        for name in ('binding.json', 'receipt.json', 'session.jsonl', 'fixture.json',
                     'cli.stdout', 'cli.stderr'):
            shutil.copyfile(root / name, retained / name)
        (retained / 'input.txt').write_bytes((workspace / 'input.txt').read_bytes())
    print('Actual Katana CLI local fixture retained; no assurance')


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, subprocess.TimeoutExpired) as error:
        print(f'CLI fixture refused: {error}', file=sys.stderr)
        sys.exit(1)
