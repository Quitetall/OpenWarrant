# SPDX-License-Identifier: Apache-2.0
"""Derive bounded repair eligibility from an exact retained verifier observation."""
from verification import require


def plan(job, attempts, repair_cycles=3):
    require(type(repair_cycles) is int and 0 <= repair_cycles <= 20, 'Invalid repair-cycle limit')
    expected, id = job['request'], job['verification_id']
    repairs = [r for r in attempts if r['warrant_id'] == expected['warrant_id']
               and r.get('verification_repair') and not r.get('resume_from')]
    prior = [r for r in repairs if r['verification_repair']['verification_id'] == id]
    require(len(prior) <= 1, 'Multiple repair attempts bind the same verification')
    out = {'schema': 'oh.war/verifier-repair-plan/v1', 'verification_id': id,
           'warrant_id': expected['warrant_id'], 'candidate_revision': expected['candidate_revision'],
           'repair_cycles': repair_cycles, 'cycles_used': len(repairs), 'state': 'blocked',
           'reason': '', 'findings': [], 'qualified': False, 'dispatch_permitted': False}
    if prior:
        out.update(state='already_dispatched', reason='Retained repair must finish and be independently verified',
                   attempt_id=prior[0]['attempt_id'])
        return out
    record = job['record']
    if record['sequence'] != 3:
        out['reason'] = 'Completed verifier observation required'
        return out
    observed = record['observation']
    if job['evidence_state'] != 'retained' or observed['execution_state'] != 'stopped':
        out['reason'] = 'Retained evidence and stopped verifier required'
    elif job['effective_verdict'] == 'pass':
        out.update(state='not_required', reason='Exact candidate passed verification')
    elif job['effective_verdict'] != 'fail':
        out.update(state='escalate', reason='Unknown observation is not an automatic repair instruction')
    elif expected.get('schema') == 'oh.war/verification-request/v2':
        out.update(state='escalate', reason='Unresolved recheck requires human decision')
    elif len(repairs) >= repair_cycles:
        out.update(state='exhausted', reason='Configured repair-cycle limit reached')
    else:
        result = observed.get('result')
        findings = result['findings'] if result else []
        if result and (not findings or any(f['status'] != 'violation' or not f['repairable'] for f in findings)):
            out.update(state='escalate', reason='Finding requires decision or additional evidence')
        else:
            failed_checks = [c for c in observed.get('checks', [])
                             if type(c.get('exit_code')) is int and c['exit_code'] != 0]
            require(findings or failed_checks, 'Failed verification lacks actionable observations')
            out.update(state='ready', reason='Repair observed defects within existing scope and protected checks',
                       findings=findings, failed_checks=failed_checks)
    return out
