# SPDX-License-Identifier: Apache-2.0
"""Authenticated human dispute decisions, distinct from acceptance or qualification."""
import json

from hotline import answer, digest
from verification import require, bounded, identity


ACTIONS = frozenset({'repair', 'verify_again', 'revise_scope', 'stop'})


def validate_recheck(value, expected):
    """Validate retained context, not credentials; the service checks current authority."""
    require(isinstance(value, dict) and set(value) == {'decision', 'prior_record'},
            'Exact human recheck context required')
    decision, prior = value['decision'], value['prior_record']
    require(isinstance(decision, dict) and set(decision) == {
                'schema', 'question', 'response', 'action', 'reason', 'qualified', 'changes_verdict', 'dispatch_permitted'}
            and decision.get('schema') == 'oh.war/verifier-decision/v1'
            and decision.get('action') == 'verify_again' and decision.get('qualified') is False
            and decision.get('changes_verdict') is False and decision.get('dispatch_permitted') is False,
            'Retained verify-again decision required')
    q, response = decision.get('question'), decision.get('response')
    require(isinstance(q, dict) and q.get('schema') == 'oh.war/verifier-dispute/v1'
            and identity(q.get('verification_id')) and isinstance(prior, dict)
            and prior.get('schema') == 'oh.war/verifier-job/v1' and prior.get('sequence') == 3
            and q.get('observation_sha256') == digest(prior)
            and q.get('verification_id') == prior.get('verification_id')
            and q.get('verification_id') != expected.get('verification_id')
            and all(q.get(k) == expected.get(k) for k in ('warrant_id', 'source_sha256', 'candidate_revision'))
            and len(json.dumps(value).encode()) <= 262144, 'Human recheck subject or evidence mismatch')
    require(isinstance(response, dict) and response.get('schema') == 'oh.war/hotline-answer/v1'
            and response.get('question_sha256') == digest(q) and response.get('respondent_kind') == 'human'
            and response.get('qualified') is False and bounded(decision['reason'], 15000)
            and response.get('answer') == 'verify_again: ' + decision['reason'],
            'Human recheck decision response mismatch')
    return value


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
