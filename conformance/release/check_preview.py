#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Validate native alpha assets before publishing an existing prerelease draft."""
import argparse
import json
from pathlib import Path
import re
from install_check import verify


def check(directory, tag, commit):
    if not re.fullmatch(r'v\d+\.\d+\.\d+-alpha\.\d+', tag):
        raise ValueError('Only explicit alpha prereleases may publish')
    hosts = set()
    candidates = sorted(directory.glob('candidate-*.json'))
    if len(candidates) != 2:
        raise ValueError('Both native candidates required')
    for path in candidates:
        candidate = json.loads(path.read_text())
        name = candidate['archive']
        if Path(name).name != name:
            raise ValueError('Unsafe archive name')
        _, manifest = verify(directory / name, candidate['sha256'])
        if manifest != candidate['manifest']:
            raise ValueError('Candidate manifest mismatch')
        if manifest['source_git_head'] != commit or manifest['label'] != tag:
            raise ValueError('Candidate source or label mismatch')
        if manifest['binary_version'] != 'war ' + tag[1:]:
            raise ValueError('Candidate binary version mismatch')
        host = (manifest['host'], manifest['architecture'])
        if host in hosts:
            raise ValueError('Duplicate native candidate')
        hosts.add(host)
        target = path.name.removeprefix('candidate-').removesuffix('.json')
        receipt = json.loads((directory / ('install-' + target + '.json')).read_text())
        if (receipt.get('passed') is not True or receipt.get('manifest') != manifest
            or receipt.get('archive_sha256') != candidate['sha256']
            or receipt.get('qualification_established') is not False):
            raise ValueError('Native install proof missing or mismatched')
        for command in ('prototype-init', 'prototype-new', 'prototype-compile', 'prototype-overview'):
            if receipt['observations'][command]['exit_code'] != 0:
                raise ValueError('Prototype setup did not pass')
    if hosts != {('Linux', 'x86_64'), ('Darwin', 'arm64')}:
        raise ValueError('Linux x86_64 and macOS arm64 required')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory', type=Path, required=True)
    parser.add_argument('--tag', required=True)
    parser.add_argument('--commit', required=True)
    args = parser.parse_args()
    check(args.directory, args.tag, args.commit)
    print('Alpha artifact and native prototype install checks passed; no qualification granted')
