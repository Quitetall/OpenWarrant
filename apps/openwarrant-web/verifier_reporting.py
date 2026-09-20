# SPDX-License-Identifier: Apache-2.0
"""Deterministic verification supplement to an execution report; never acceptance."""
import html
import json

from hotline import digest
from verification import require


def attach(report, snapshot, detail='full'):
    require(report['qualified'] is False and snapshot['qualified'] is False,
            'Workflow reports cannot award qualification')
    snapshot = json.loads(json.dumps(snapshot))
    fingerprint = digest(snapshot)
    lines = ['Retained verification history; acceptance remains separate.']
    for job in snapshot['jobs']:
        lines.append(job['verification_id'] + ': ' + job['effective_verdict']
                     + ' · evidence ' + job['evidence_state']
                     + ' · candidate ' + job['request']['candidate_revision'])
        observation = job['record'].get('observation', {})
        if observation:
            lines.append('Historical verdict: ' + observation['verdict'])
            lines.append(observation.get('cause', ''))
        for finding in (observation.get('result') or {}).get('findings', []):
            lines.append(finding['id'] + ': ' + finding['observation'] + ' · scope ' + finding['scope'])
    for repair in snapshot['repairs']:
        lines.append('Repair ' + repair['attempt_id'] + ': ' + repair['work_state']
                     + ' · execution ' + repair['execution_state'])
    for dispute in snapshot['disputes']:
        decision = dispute.get('decision')
        lines.append('Dispute ' + dispute['question']['verification_id'] + ': '
                     + (decision['action'] if decision else dispute['state']))
    if not snapshot['jobs']:
        lines.append('No retained verification jobs for this Warrant source.')
    lines.append('Verification snapshot: ' + fingerprint)
    section = ('<section><h2>Verification and repair history</h2><p>Snapshot only; '
               'not current authorization or human acceptance.</p><pre>'
               + html.escape('\n'.join(lines)) + '</pre><details><summary>Exact verification evidence</summary><pre>'
               + html.escape(json.dumps(snapshot, indent=2, ensure_ascii=False)) + '</pre></details></section>')
    return {**report, 'schema': 'oh.war/reference-work-report/v2',
            'verification': snapshot, 'verification_snapshot_sha256': fingerprint,
            'text': report['text'] + ('\n'.join(lines) + '\n' if detail == 'full' else
                                     'Verification history included in offline report; acceptance remains separate.\n'),
            'html': report['html'].replace('</html>', section + '</html>')}


def report(service, attempt_id, word, detail):
    executor = service.executor
    with executor.lock:
        attempt = executor.get(attempt_id)
        jobs = [j for j in service.listing()['jobs'] if j['request']['warrant_id'] == attempt['warrant_id']
                and j['request']['source_sha256'] == attempt['source_sha256']]
        repairs = [executor.view(r) for _, r in sorted(executor.records().items())
                   if r['warrant_id'] == attempt['warrant_id'] and r['source_sha256'] == attempt['source_sha256']
                   and r.get('verification_repair')]
        disputes = [service.dispute(j['verification_id'], service.answers) for j in jobs
                    if j['human_review_required'] and j['evidence_state'] == 'retained']
        snapshot = {'schema': 'oh.war/verification-report-snapshot/v1',
                    'warrant_id': attempt['warrant_id'], 'source_sha256': attempt['source_sha256'],
                    'jobs': jobs, 'repairs': repairs, 'disputes': disputes, 'qualified': False}
        return attach(executor.report(attempt_id, word, detail), snapshot, detail)
