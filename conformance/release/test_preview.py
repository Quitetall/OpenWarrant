#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
import copy
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from check_preview import check
from test_package import fixture, archive

TAG = 'v1.0.0-alpha.1'
COMMIT = 'a' * 40


class PreviewTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        for target, host, arch in [('linux', 'Linux', 'x86_64'), ('macos', 'Darwin', 'arm64')]:
            files = fixture()
            manifest = json.loads(files['MANIFEST.json'])
            manifest.update(source_git_head=COMMIT, label=TAG,
                            binary_version='war 1.0.0-alpha.1', host=host, architecture=arch)
            files['MANIFEST.json'] = json.dumps(manifest).encode()
            data = archive(files)
            name = target + '.tar.gz'
            (self.root / name).write_bytes(data)
            candidate = dict(archive=name, sha256=hashlib.sha256(data).hexdigest(), manifest=manifest)
            self.write('candidate-' + target + '.json', candidate)
            # Synthetic receipt exercises validation only; no native execution claim.
            receipt = dict(passed=True, manifest=manifest, archive_sha256=candidate['sha256'],
                           qualification_established=False, observations={
                               key: dict(exit_code=0) for key in (
                                   'prototype-init','prototype-new','prototype-compile','prototype-overview')})
            self.write('install-' + target + '.json', receipt)

    def write(self, name, value):
        (self.root / name).write_text(json.dumps(value))

    def test_both_declared_native_receipts_and_sealed_archives(self):
        check(self.root, TAG, COMMIT)

    def test_stable_tag_and_wrong_source_refuse(self):
        for tag, commit in [('v1.0.0', COMMIT), (TAG, 'b' * 40)]:
            with self.assertRaises(ValueError): check(self.root, tag, commit)

    def test_missing_host_and_changed_archive_refuse(self):
        path = self.root / 'candidate-macos.json'
        path.unlink()
        with self.assertRaises(ValueError): check(self.root, TAG, COMMIT)
        self.setUp()
        (self.root / 'linux.tar.gz').write_bytes(b'changed')
        with self.assertRaises(ValueError): check(self.root, TAG, COMMIT)

    def test_failed_or_stale_install_cannot_publish(self):
        path = self.root / 'install-linux.json'
        original = json.loads(path.read_text())
        for field, value in [('passed', False), ('archive_sha256', 'wrong'), ('qualification_established', True)]:
            bad = copy.deepcopy(original); bad[field] = value; self.write(path.name, bad)
            with self.assertRaises(ValueError): check(self.root, TAG, COMMIT)
        bad = copy.deepcopy(original)
        bad['observations']['prototype-init']['exit_code'] = 1
        self.write(path.name, bad)
        with self.assertRaises(ValueError): check(self.root, TAG, COMMIT)


if __name__ == '__main__': unittest.main()
