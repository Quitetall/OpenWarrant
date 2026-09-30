# SPDX-License-Identifier: Apache-2.0
import json
import os
import tempfile
import unittest
from pathlib import Path

from server import decode
from verification import VerificationError
from verifier_receipts import ReceiptInbox


class ReceiptInboxTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.inbox = self.root / 'inbox'; self.inbox.mkdir()
        self.reader = ReceiptInbox(self.inbox, decode); self.addCleanup(self.reader.close)
        self.job = {'verification_id': '00000000-0000-4000-8000-000000000001'}
        self.path = self.inbox / (self.job['verification_id'] + '.json')
        self.receipt = {'payload_base64': 'e30=', 'signature_base64': 'e30='}

    def test_missing_waits_and_read_preserves_exact_envelope_without_authentication(self):
        self.assertIsNone(self.reader(self.job))
        self.path.write_text(json.dumps(self.receipt))
        self.assertEqual(self.reader(self.job), self.receipt)
        self.assertTrue(self.path.exists())
        self.reader.close()
        with self.assertRaises(VerificationError): self.reader(self.job)

    def test_symlink_fifo_oversize_and_extra_fields_refuse(self):
        target = self.root / 'target'; target.write_text(json.dumps(self.receipt))
        self.path.symlink_to(target)
        with self.assertRaises(OSError): self.reader(self.job)
        self.path.unlink(); os.mkfifo(self.path)
        with self.assertRaises(VerificationError): self.reader(self.job)
        self.path.unlink(); self.path.write_bytes(b' ' * 65537)
        with self.assertRaises(VerificationError): self.reader(self.job)
        self.path.write_text(json.dumps({**self.receipt, 'qualified': True}))
        with self.assertRaises(VerificationError): self.reader(self.job)

    def test_directory_replacement_does_not_redirect_open_reader(self):
        self.path.write_text(json.dumps(self.receipt))
        self.inbox.rename(self.root / 'retained')
        self.inbox.mkdir()
        self.path.write_text('malformed replacement')
        self.assertEqual(self.reader(self.job), self.receipt)
        link = self.root / 'link'; link.symlink_to(self.inbox)
        with self.assertRaises(OSError): ReceiptInbox(link, decode)

    def test_untrusted_identity_cannot_select_other_files(self):
        for id in ('../target', '/tmp/file', '', None):
            with self.subTest(id=id), self.assertRaises(VerificationError):
                self.reader({'verification_id': id})
