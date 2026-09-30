# SPDX-License-Identifier: Apache-2.0
"""Pure reference context-entry proposal adapter. Never reads or writes host files."""
import hashlib
import re

BEGIN = '<!-- openwarrant:context-entry -->\n'
END = '<!-- /openwarrant:context-entry -->\n'


class Refusal(ValueError):
    pass


def digest(data):
    return 'sha256:' + hashlib.sha256(data).hexdigest()


def propose(*, before, source, source_digest, source_path, host_path, host_digest,
            scope, actual_target, approved_target, previous=None, remove=False,
            conflict=False):
    """Return proposed bytes and exact supplied identities, never authorization.

    Target resolution, source applicability and conflict findings come from the
    host. This function does not infer them from filenames or inspect symlinks.
    The caller must recheck exact inputs before any actual filesystem write.
    """
    if type(remove) is not bool or type(conflict) is not bool:
        raise Refusal('entry.flags')
    if not isinstance(before, bytes) or not isinstance(source, bytes):
        raise Refusal('entry.bytes')
    if len(before) + len(source) > 1024 * 1024:
        raise Refusal('entry.limit')
    if not source:
        raise Refusal('entry.source-missing')
    if digest(before) != host_digest or digest(source) != source_digest:
        raise Refusal('entry.digest')
    for value in [source_path, host_path, scope, actual_target, approved_target]:
        if not isinstance(value, str) or not value or len(value) > 4096 or any(ord(c) < 32 for c in value):
            raise Refusal('entry.identity')
    # Restrict link text syntax; the host resolves the explicit relative pointer.
    if not re.fullmatch(r'[A-Za-z0-9_./-]+', source_path) or source_path.startswith('/'):
        raise Refusal('entry.pointer')
    if not re.fullmatch(r'[A-Za-z0-9_./-]+', scope):
        raise Refusal('entry.scope')
    if actual_target != approved_target:
        raise Refusal('entry.target-not-approved')
    if conflict:
        raise Refusal('entry.conflict')
    block = (BEGIN + f'For OpenWarrant work in `{scope}`, read [shared guidance]({source_path}).\n'
             'Keep applying host and nested instructions in their existing scopes.\n' + END).encode()
    start, end = BEGIN.encode(), END.encode()
    if before.count(start.rstrip(b'\n')) != before.count(start) or before.count(end.rstrip(b'\n')) != before.count(end):
        raise Refusal('entry.framing')
    if before.count(start) != before.count(end) or before.count(start) > 1:
        raise Refusal('entry.framing')
    if start in before:
        at = before.index(start)
        stop = before.find(end, at)
        if stop < at:
            raise Refusal('entry.framing')
        stop += len(end)
        current = before[at:stop]
        if previous is None or current != previous:
            raise Refusal('entry.modified')
        if not remove and current != block:
            raise Refusal('entry.revision-required')
        after = before[:at] + (b'' if remove else block) + before[stop:]
    elif previous is not None or remove:
        raise Refusal('entry.missing-owned')
    else:
        if before and not before.endswith(b'\n'):
            raise Refusal('entry.line-boundary')
        after = before + block
    return {'before_digest': host_digest, 'after_digest': digest(after),
            'source_digest': source_digest, 'source_path': source_path,
            'host_path': host_path, 'scope': scope, 'actual_target': actual_target,
            'proposed_bytes': after, 'owned_entry': None if remove else block,
            'applied': False, 'authority_established': False}
