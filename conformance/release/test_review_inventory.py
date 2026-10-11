# SPDX-License-Identifier: Apache-2.0
"""Exercise the release inventory command against committed repositories."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest


COMMAND = Path(__file__).with_name('review_inventory.py')


class ReviewInventoryTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(prefix='ow-review-')
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.git('init', '-q')
        self.git('config', 'user.name', 'Fixture')
        self.git('config', 'user.email', 'fixture@example.invalid')
        self.base = self.root / 'docs/warrants/OW-WAR-0001'
        (self.base / 'implementation').mkdir(parents=True)
        (self.base / 'manifest.toml').write_text('alias = "OW-WAR-0001"\n')
        self.report = self.base / 'implementation/progress.json'
        self.report.write_text(json.dumps({
            'schema': 'oh.war/viewer-work-report/v1', 'warrant': 'OW-WAR-0001',
            'work_state': 'completed', 'summary': 'Fixture only',
            'next_steps': ['Independent review remains'],
            'evidence': ['proof.txt', 'missing.txt'],
        }))
        (self.root / 'proof.txt').write_bytes(b'abc')
        self.commit()

    def git(self, *args):
        return subprocess.check_output(['git', *args], cwd=self.root, timeout=20).decode().strip()

    def commit(self):
        self.git('add', '.')
        self.git('commit', '-qm', 'fixture')
        return self.git('rev-parse', 'HEAD')

    def run_command(self, *args):
        return subprocess.run(['python3', str(COMMAND), '--repo', str(self.root), *args],
                              capture_output=True, text=True, timeout=30)

    def test_inventory_uses_committed_bytes_and_reports_missing_evidence(self):
        head = self.git('rev-parse', 'HEAD')
        (self.root / 'proof.txt').write_bytes(b'changed working bytes')
        (self.root / 'untracked.txt').write_bytes(b'not part of the subject')
        first = self.run_command('--revision', head)
        self.assertEqual(first.returncode, 0, first.stderr)
        self.assertEqual(first.stdout, self.run_command('--revision', head).stdout)
        result = json.loads(first.stdout)
        self.assertEqual(result['snapshot_git_head'], head)
        self.assertNotIn('untracked.txt', result['source_files'])
        proof = next(x for x in result['evidence_files'] if x['path'] == 'proof.txt')
        self.assertEqual(proof['bytes'], 3)
        self.assertEqual(proof['sha256'], 'ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad')
        self.assertEqual(result['missing_evidence'], ['missing.txt'])
        self.assertEqual(result['work_reports'][0]['report']['work_state'], 'completed')
        self.assertFalse(result['qualification_established'])
        self.assertFalse(result['publication_permitted'])

    def test_ambiguous_or_invalid_report_refuses_without_output(self):
        original = json.loads(self.report.read_text())
        for kind in ('duplicate', 'escape', 'absolute', 'wrong-subject', 'wrong-schema'):
            with self.subTest(kind=kind):
                report = dict(original)
                if kind == 'escape':
                    report['evidence'] = ['../proof.txt']
                elif kind == 'absolute':
                    report['evidence'] = [str(self.root / 'proof.txt')]
                elif kind == 'wrong-subject':
                    report['warrant'] = 'OW-WAR-0002'
                elif kind == 'wrong-schema':
                    report['schema'] = 'unknown'
                text = json.dumps(report)
                if kind == 'duplicate':
                    text = text.replace('"completed"', '"blocked", "work_state": "completed"')
                self.report.write_text(text)
                self.commit()
                output = self.root / (kind + '-refused.json')
                result = self.run_command('--output', str(output))
                self.assertNotEqual(result.returncode, 0)
                expected = 'Duplicate JSON field' if kind == 'duplicate' else ('Unsafe evidence reference' if kind in ('escape', 'absolute') else 'Invalid work report')
                self.assertIn(expected, result.stderr)
                self.assertFalse(output.exists())

    def test_records_without_work_report_remain_visible(self):
        other = self.root / 'docs/warrants/OW-WAR-0002'
        other.mkdir()
        (other / 'manifest.toml').write_text('alias = "OW-WAR-0002"\n')
        self.commit()
        result = self.run_command()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)['warrants_without_work_reports'], ['OW-WAR-0002'])

    def test_optional_evidence_field_can_be_absent(self):
        report = json.loads(self.report.read_text())
        del report['evidence']
        self.report.write_text(json.dumps(report))
        self.commit()
        result = self.run_command()
        self.assertEqual(result.returncode, 0, result.stderr)
        data = json.loads(result.stdout)
        self.assertEqual(data['evidence_files'], [])
        self.assertEqual(data['work_reports'][0]['report'], report)

    def test_existing_snapshot_and_symlink_inputs_are_not_replaced_or_followed(self):
        output = self.root / 'snapshot.json'
        first = self.run_command('--output', str(output))
        self.assertEqual(first.returncode, 0, first.stderr)
        before = output.read_bytes()
        result = self.run_command('--output', str(output))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('FileExistsError', result.stderr)
        self.assertEqual(output.read_bytes(), before)
        self.assertEqual(list(self.root.glob('.ow-package-*')), [])
        link = self.root / 'linked-proof.txt'
        link.symlink_to(self.root / 'proof.txt')
        report = json.loads(self.report.read_text())
        report['evidence'] = ['linked-proof.txt']
        self.report.write_text(json.dumps(report))
        self.commit()
        result = self.run_command('--output', str(self.root / 'unsafe.json'))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('not a regular committed file', result.stderr)
        self.assertFalse((self.root / 'unsafe.json').exists())


if __name__ == '__main__':
    unittest.main()
