#!/usr/bin/env python3
"""Check retained archive bytes and paths; never treat this as signature verification."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import subprocess
import tarfile
import tempfile


def require(condition, reason):
    if not condition:
        raise ValueError(reason)


def digest(data):
    return hashlib.sha256(data).hexdigest()


def inside(root, relative):
    parts = PurePosixPath(relative)
    if parts.is_absolute() or '..' in parts.parts:
        raise ValueError(f'Unsafe archive path: {relative}')
    result = root / relative
    if any(p.is_symlink() for p in result.parents if p != root and root in p.parents):
        raise ValueError(f'Ancestor symlink: {relative}')
    return result


def verify(root, restore):
    raw = (root / 'manifest.json').read_bytes()
    expected = (root / 'manifest.sha256').read_text().split()[0]
    require(digest(raw) == expected, 'Manifest digest mismatch')
    manifest = json.loads(raw)
    require(digest((root / 'history.bundle').read_bytes()) == manifest['bundle_sha256'], 'Bundle digest mismatch')
    for entry in manifest['baseline_files']:
        p = inside(root / 'original', entry['path'])
        if entry['kind'] == 'symlink':
            require(p.is_symlink() and os.readlink(p) == entry['target'], entry['path'])
        else:
            require(not p.is_symlink() and digest(p.read_bytes()) == entry['sha256'], entry['path'])
    for capture in manifest['worktrees']:
        p = inside(root, capture['archive'])
        require(digest(p.read_bytes()) == capture['archive_sha256'], capture['archive'])
        with tarfile.open(p, 'r:gz') as archive:
            members = archive.getmembers()
            by_name = {member.name: member for member in members}
            expected_names = {entry['path'] for entry in capture['files']}
            require(len(members) == len(by_name) == len(capture['files']), 'Duplicate or lost entry')
            require(set(by_name) == expected_names, 'Path coverage mismatch')
            for entry in capture['files']:
                member = by_name[entry['path']]
                require(member.mode == entry['mode'], entry['path'])
                if entry['kind'] == 'symlink':
                    require(member.issym() and member.linkname == entry['target'], entry['path'])
                else:
                    require(member.isfile(), entry['path'])
                    require(digest(archive.extractfile(member).read()) == entry['sha256'], entry['path'])
    for entry in json.loads((root / 'relocations.json').read_text()):
        require(digest(inside(root, entry['archive_path']).read_bytes()) == entry['sha256'], entry['archive_path'])
    with tempfile.TemporaryDirectory(prefix='ow-archive-verify-') as temp:
        repo = Path(temp) / 'restored.git'
        subprocess.run(['git', 'init', '--bare', '-q', str(repo)], check=True)
        subprocess.run(['git', '-C', str(repo), 'bundle', 'verify', str(root / 'history.bundle')],
                       check=True, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE)
        if restore:
            subprocess.run(['git', '-C', str(repo), 'fetch', '-q', str(root / 'history.bundle'),
                            '+refs/*:refs/*'], check=True)
            for ref in manifest['refs']:
                name, oid = ref.split()
                actual = subprocess.check_output(['git', '-C', str(repo), 'rev-parse', '--verify', name]).decode().strip()
                require(actual == oid, name)
            for entry in manifest['baseline_files']:
                data = subprocess.check_output(['git', '-C', str(repo), 'show',
                                                manifest['baseline'] + ':' + entry['path']])
                require(digest(data) == entry['sha256'], entry['path'])
    return {'status': 'PASS', 'baseline_files': len(manifest['baseline_files']),
            'worktrees': len(manifest['worktrees']),
            'snapshot_entries': sum(len(c['files']) for c in manifest['worktrees']),
            'restored_git_refs_and_baseline': 'PASS' if restore else 'NOT RUN',
            'signature_authenticity': 'NOT RUN'}


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--root', type=Path, default=Path(__file__).resolve().parent)
    parser.add_argument('--restore-check', action='store_true')
    args = parser.parse_args()
    try:
        print(json.dumps(verify(args.root.resolve(), args.restore_check), sort_keys=True))
    except (AssertionError, OSError, ValueError, subprocess.CalledProcessError, tarfile.TarError) as error:
        parser.exit(1, f'FAIL: {error}\n')
