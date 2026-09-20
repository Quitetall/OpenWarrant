# SPDX-License-Identifier: Apache-2.0
"""Bounded receipt inbox reads; signature and current-basis checks remain in dispatch."""
import os
import stat

from verification import identity, require


class ReceiptInbox:
    """Pin an operator-selected directory; never launch a receipt producer.

    A harness publishes <verification UUID>.json atomically. The file contains
    the existing start endpoint's payload_base64 and signature_base64 fields.
    Missing receipt means wait. Invalid receipt means refusal, never fallback.
    Directory/file permissions are not proof of execution isolation.
    """

    def __init__(self, path, decode):
        self.decode = decode
        self.fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)

    def close(self):
        if self.fd is not None:
            os.close(self.fd)
            self.fd = None

    def __call__(self, job):
        require(self.fd is not None, 'Receipt inbox closed')
        id = job.get('verification_id')
        require(identity(id), 'Exact verification identity required for receipt lookup')
        try:
            fd = os.open(id + '.json', os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK, dir_fd=self.fd)
        except FileNotFoundError:
            return None
        with os.fdopen(fd, 'rb') as stream:
            require(stat.S_ISREG(os.fstat(stream.fileno()).st_mode), 'Regular receipt file required')
            data = stream.read(65537)
        require(len(data) <= 65536, 'Receipt file exceeds 64 KiB')
        value = self.decode(data)
        require(isinstance(value, dict) and set(value) == {'payload_base64', 'signature_base64'}
                and all(isinstance(v, str) and 0 < len(v) <= 21848 for v in value.values()),
                'Bounded signed protection receipt required')
        return value
