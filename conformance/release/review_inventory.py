#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Freeze committed release-review inputs. Never authorize or qualify a release."""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess

from package import publish_new


MAX_BLOB = 64 * 1024 * 1024


def unique_fields(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError('Duplicate JSON field: ' + key)
        result[key] = value
    return result


def git(repo, *args):
    return subprocess.check_output(['git', *args], cwd=repo, timeout=30)


def inventory(repo, revision):
    head = git(repo, 'rev-parse', '--verify', '--end-of-options', revision + '^{commit}').decode().strip()
    tree = git(repo, 'rev-parse', head + '^{tree}').decode().strip()
    sources = {}
    for row in git(repo, 'ls-tree', '-r', '-z', '-l', head).split(b'\0'):
        if not row:
            continue
        info, name = row.split(b'\t', 1)
        mode, kind, oid, size = info.split()
        if kind != b'blob':
            raise ValueError('Submodule source requires an explicit release profile')
        sources[name.decode()] = {'mode': mode.decode(), 'object': oid.decode(), 'bytes': int(size)}

    def read(path):
        entry = sources[path]
        if entry['mode'] not in ('100644', '100755'):
            raise ValueError('Review input is not a regular committed file: ' + path)
        if entry['bytes'] > MAX_BLOB:
            raise ValueError('Review input size limit exceeded: ' + path)
        return git(repo, 'cat-file', 'blob', entry['object'])

    def digest(path, data):
        return {'path': path, 'bytes': len(data), 'sha256': hashlib.sha256(data).hexdigest()}

    reports = []
    evidence = set()
    for path in sorted(sources):
        parts = path.split('/')
        if len(parts) != 5 or parts[:2] != ['docs', 'warrants'] or parts[3:] != ['implementation', 'progress.json']:
            continue
        data = read(path)
        report = json.loads(data, object_pairs_hook=unique_fields)
        if (not isinstance(report, dict) or report.get('schema') != 'oh.war/viewer-work-report/v1'
                or report.get('warrant') != parts[2] or not isinstance(report.get('evidence', []), list)
                or not isinstance(report.get('work_state'), str) or not report['work_state']):
            raise ValueError('Invalid work report: ' + path)
        reports.append({**digest(path, data), 'report': report})
        for ref in report.get('evidence', []):
            if not isinstance(ref, str) or not ref or ref.startswith('/') or any(x in ('', '.', '..') for x in ref.split('/')):
                raise ValueError('Unsafe evidence reference in ' + path)
            evidence.add(ref)
    files = []
    missing = []
    for path in sorted(evidence):
        if path not in sources:
            missing.append(path)
        else:
            files.append(digest(path, read(path)))
    warrants = {parts[2] for path in sources if len(parts := path.split('/')) == 4
                and parts[:2] == ['docs', 'warrants'] and parts[3] == 'manifest.toml'}
    reported = {entry['report']['warrant'] for entry in reports}
    return {
        'schema': 'openwarrant.performer-release-review/v1',
        'snapshot_git_head': head, 'snapshot_git_tree': tree,
        'generator_sha256': hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        'inventory_only': True, 'qualification_established': False, 'publication_permitted': False,
        'source_files': dict(sorted(sources.items())), 'work_reports': reports,
        'evidence_files': files, 'missing_evidence': missing,
        'warrants_without_work_reports': sorted(warrants - reported),
        'scope_note': 'Committed inputs only. Work reports are performer claims; hashes do not establish truth, independence, authority or build provenance. Missing reports do not imply completion. This snapshot is not a signed review manifest.',
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--repo', type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument('--revision', default='HEAD')
    parser.add_argument('--output', type=Path, help='New file only; existing snapshots are never replaced')
    args = parser.parse_args()
    result = inventory(args.repo, args.revision)
    data = (json.dumps(result, sort_keys=True, indent=2) + '\n').encode()
    if args.output:
        publish_new(args.output, data)
    else:
        print(data.decode(), end='')


if __name__ == '__main__':
    main()
