# SPDX-License-Identifier: Apache-2.0
"""Authenticated human dispute decisions, distinct from acceptance or qualification."""
from hotline import answer, digest
from verification import require, bounded


ACTIONS = frozenset({'repair', 'verify_again', 'revise_scope', 'stop'})


def question(job, authorization_digest):
    require(job['human_review_required'] and job['evidence_state'] == 'retained'
            and job['record']['sequence'] == 3, 'Retained unresolved recheck required')
    return {'schema': 'oh.war/verifier-dispute/v1', 'verification_id': job['verification_id'],
            'warrant_id': job['request']['warrant_id'], 'source_sha256': job['request']['source_sha256'],
            'candidate_revision': job['request']['candidate_revision'],
            'observation_sha256': digest(job['record']), 'authorization_sha256': authorization_digest,
            'question': {'kind': 'governing', 'direct_human': True,
                         'text': 'Independent recheck did not settle this dispute. Choose the next action.'}}


def decide(q, fields, credential, responders):
    require(isinstance(fields, dict) and set(fields) == {'question_sha256', 'action', 'reason', 'evidence'}
            and isinstance(fields['action'], str) and fields['action'] in ACTIONS,
            'Exact dispute basis and supported next action required')
    require(bounded(fields['reason'], 15000) and isinstance(fields['evidence'], list)
            and 1 <= len(fields['evidence']) <= 32, 'Human decision requires reason and evidence')
    authenticated = answer({'question_sha256': fields['question_sha256'],
                            'answer': fields['action'] + ': ' + fields['reason'],
                            'evidence': fields['evidence']}, credential, responders, q)
    return {'schema': 'oh.war/verifier-decision/v1', 'question': q, 'response': authenticated,
            'action': fields['action'], 'reason': fields['reason'], 'qualified': False,
            'changes_verdict': False, 'dispatch_permitted': False}
