# SPDX-License-Identifier: Apache-2.0
"""Evidence-backed challenges carry prior observations; never clear findings."""
import json

from hotline import digest
from verification import require, identity, bounded, request_digest


def validate(value):
    require(isinstance(value, dict) and set(value) == {'verification_id', 'request_sha256',
            'observation_sha256', 'observation', 'argument', 'evidence', 'finding_ids'}, 'Invalid recheck context')
    require(identity(value['verification_id']) and bounded(value['argument'],16000), 'Bounded rebuttal argument required')
    evidence=value['evidence'];ids=value['finding_ids'];observed=value['observation']
    require(isinstance(evidence,list) and 1 <= len(evidence) <= 32 and all(bounded(v,2000) for v in evidence),
            'Rebuttal requires bounded evidence references')
    require(isinstance(ids,list) and 1 <= len(ids) <= 64 and all(bounded(v,64) for v in ids)
            and len(set(ids)) == len(ids), 'Distinct challenged findings required')
    require(isinstance(observed,dict) and observed.get('schema') == 'oh.war/verifier-observation/v1'
            and observed.get('request_sha256') == value['request_sha256']
            and digest(observed) == value['observation_sha256']
            and len(json.dumps(observed).encode()) <= 131072, 'Exact bounded prior observation required')
    known={f['id'] for f in (observed.get('result') or {}).get('findings',[])}
    known |= {'check-'+str(i+1) for i,c in enumerate(observed.get('checks',[]))
              if type(c.get('exit_code')) is int and c['exit_code'] != 0}
    require(set(ids) <= known, 'Rebuttal names an unknown finding')
    return value


def context(job, fields):
    require(isinstance(fields,dict) and set(fields) == {'verification_id','argument','evidence','finding_ids'}
            and identity(fields['verification_id']) and fields['verification_id'] != job['verification_id'],
            'Exact new recheck identity and evidence-backed argument required')
    require(job['request']['schema'] == 'oh.war/verification-request/v1', 'Unresolved recheck requires human decision')
    require(job['record']['sequence'] == 3 and job['evidence_state'] == 'retained'
            and job['effective_verdict'] == 'fail', 'Retained failed verification required for rebuttal')
    observed=job['record']['observation']
    require(observed['execution_state'] == 'stopped', 'Stopped independent observation required')
    return validate({'verification_id':job['verification_id'], 'request_sha256':request_digest(job['request']),
                     'observation_sha256':digest(observed),'observation':observed,
                     **{k:fields[k] for k in ('argument','evidence','finding_ids')}})
