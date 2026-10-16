#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
import copy
import hashlib
import json
import os
import shutil
import subprocess
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

    def test_preview_uploads_the_source_installer_and_its_digest(self):
        # Execute the actual workflow shell. Git/GitHub are observation stubs;
        # the candidate validator, shell, digest and asset selection are real.
        for command in ('bash', 'jq', 'sha256sum'):
            if not shutil.which(command): self.skipTest(command + ' unavailable')
        repository = Path(__file__).resolve().parents[2]
        lines = (repository / '.github/workflows/release.yml').read_text().splitlines()
        start = lines.index('      - name: Publish only the checked alpha preview')
        start = lines.index('        run: |', start) + 1
        body = []
        for line in lines[start:]:
            if line and not line.startswith('          '): break
            body.append(line[10:] if line else '')
        # The workflow expects the downloaded artifacts in dist/.
        dist = self.root / 'dist'
        dist.mkdir()
        for name in list(self.root.iterdir()):
            if name.is_file(): name.rename(dist / name.name)
        (self.root / 'conformance').mkdir()
        (self.root / 'conformance/release').symlink_to(repository / 'conformance/release', target_is_directory=True)
        notes = self.root / 'docs/releases/alpha1.md'
        notes.parent.mkdir(parents=True)
        notes.write_text('Fixture release notes; no publication occurs.\n')
        installer = (repository / 'install.sh').read_bytes()
        (self.root / 'install.sh').write_bytes(installer)
        bin_dir = self.root / 'stub-bin'
        bin_dir.mkdir()
        gh = bin_dir / 'gh'
        gh.write_text('#!/usr/bin/env python3\nimport json,os,sys\n'
                      'with open(os.environ["PREVIEW_TEST_LOG"],"a") as out: out.write(json.dumps(sys.argv[1:])+"\\n")\n'
                      'if sys.argv[1:3]==["release","view"]: print(json.dumps({"isDraft":True,"isPrerelease":True}))\n')
        gh.chmod(0o755)
        git = bin_dir / 'git'
        git.write_text('#!/bin/sh\ntest "$1" = ls-remote || exit 1\nexit 2\n')
        git.chmod(0o755)
        log = self.root / 'github-calls.jsonl'
        env = {**os.environ, 'PATH': str(bin_dir) + os.pathsep + os.environ['PATH'],
               'PREVIEW_TAG': TAG, 'GITHUB_SHA': COMMIT, 'PREVIEW_TEST_LOG': str(log)}
        result = subprocess.run(['bash', '-c', '\n'.join(body)], cwd=self.root,
                                env=env, capture_output=True, text=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        calls = [json.loads(line) for line in log.read_text().splitlines()]
        upload = next(call for call in calls if call[:2] == ['release', 'upload'])
        self.assertIn('install.sh', upload)
        self.assertIn('install.sh.sha256', upload)
        expected = hashlib.sha256(installer).hexdigest() + '  install.sh\n'
        self.assertEqual((self.root / 'install.sh.sha256').read_text(), expected)
        self.assertEqual(calls[-1][:2], ['release', 'edit'])
        # A missing bootstrap must stop publication, even if native assets
        # and an older installer checksum are still present.
        (self.root / 'install.sh').unlink()
        log.unlink()
        missing = subprocess.run(['bash', '-c', '\n'.join(body)], cwd=self.root,
                                 env=env, capture_output=True, text=True, timeout=30)
        self.assertNotEqual(missing.returncode, 0)
        calls = [json.loads(line) for line in log.read_text().splitlines()]
        self.assertFalse(any(call[:2] in (['release', 'upload'], ['release', 'edit']) for call in calls))

    def test_v2_source_capsules_use_the_same_sealed_preview_checks(self):
        import gzip
        for target in ['linux','macos']:
            path=self.root/('candidate-'+target+'.json')
            candidate=json.loads(path.read_text());manifest=candidate['manifest']
            files=fixture();files.pop('MANIFEST.json')
            files['sdk-source.tar.gz']=gzip.compress(files.pop('sdk-source.tar'),mtime=0)
            manifest['schema']='oh.war/candidate-bundle/v2'
            manifest['files']={p:{'sha256':hashlib.sha256(data).hexdigest(),'bytes':len(data),'executable':p=='bin/war'} for p,data in files.items()}
            files['MANIFEST.json']=json.dumps(manifest).encode();data=archive(files)
            (self.root/candidate['archive']).write_bytes(data)
            candidate['sha256']=hashlib.sha256(data).hexdigest();self.write(path.name,candidate)
            receipt_path=self.root/('install-'+target+'.json')
            receipt=json.loads(receipt_path.read_text());receipt['manifest']=manifest;receipt['archive_sha256']=candidate['sha256'];self.write(receipt_path.name,receipt)
        check(self.root,TAG,COMMIT)

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
