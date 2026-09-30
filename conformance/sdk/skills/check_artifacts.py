#!/usr/bin/env python3
# SPDX-License-Identifier: Apache-2.0
"""Validate fixed agent-authored artifacts through the real offline SDK CLI.

This checks artifact contracts, not stochastic skill invocation or human effort.
"""
import argparse
import copy
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile


def call(war, request, cwd):
    result = subprocess.run([str(war), 'sdk', '--request', '-'], input=json.dumps(request),
                            text=True, capture_output=True, cwd=cwd, timeout=30)
    envelope = json.loads(result.stdout)
    return result.returncode, envelope


def inspect(war, case, source, cwd):
    base = {'schema':'oh.war/sdk-request/v1', 'source':source, 'dialect':'rc3'}
    code, report = call(war, dict(base, operation='validate'), cwd)
    if code:
        raise ValueError('artifact.invalid')
    expected_kind = dict(case['request']['metadata'])['kind']
    if report['result']['metadata']['kind'] != expected_kind:
        raise ValueError('artifact.kind')
    for name, expectations in case['unit_expectations'].items():
        code, unit = call(war, dict(base, operation='unit', unit=name), cwd)
        if code:
            raise ValueError('artifact.unit')
        for text in expectations:
            if text not in unit['result']['unit']:
                raise ValueError('artifact.constraint-dropped')
    if case['id'] == 'grill':
        _, questions = call(war, dict(base, operation='unit', unit='questions'), cwd)
        if 'Q-001' in questions['result']['unit']:
            raise ValueError('artifact.settled-question-repeated')


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--war', required=True, type=Path)
    args=parser.parse_args();war=args.war.resolve(strict=True)
    suite=json.loads(Path(__file__).with_name('artifacts.json').read_text())
    if suite['schema']!='oh.war/fixed-skill-artifact-cases/v1':
        raise ValueError('artifact.profile')
    observations=[]
    with tempfile.TemporaryDirectory(prefix='ow85-artifacts-') as temp:
        for case in suite['cases']:
            code, result=call(war,case['request'],temp)
            if code: raise ValueError((case['id'],result))
            source=result['result']['source'];inspect(war,case,source,temp)
            # Mutation stays a syntactically valid document but loses a binding constraint.
            text=next(iter(case['unit_expectations'].values()))[0]
            changed=source.replace(text,'REMOVED EXPECTATION')
            try: inspect(war,case,changed,temp)
            except ValueError as error:
                if str(error)!='artifact.constraint-dropped':raise
            else: raise ValueError('mutation accepted')
            invalid=copy.deepcopy(case['request'])
            invalid['metadata'].append(['state','verified'])
            bad_code,bad=call(war,invalid,temp)
            if not bad_code or bad['diagnostics'][0]['code']!='field-duplicate':
                raise ValueError('duplicate state accepted')
            invalid=copy.deepcopy(case['request'])
            for field in invalid['metadata']:
                if field[0]=='state':field[1]='verified'
            bad_code,bad=call(war,invalid,temp)
            if not bad_code or bad['diagnostics'][0]['code']!='source-invalid':
                raise ValueError('false maturity accepted')
            if case['id']=='grill':
                repeated=source.replace('Q-002:', 'Q-001: Already settled?\nQ-002:')
                try:inspect(war,case,repeated,temp)
                except ValueError as error:
                    if str(error)!='artifact.settled-question-repeated':raise
                else:raise ValueError('repeated settled question accepted')
            observations.append({'id':case['id'],'source_digest':'sha256:'+hashlib.sha256(source.encode()).hexdigest(),
                                 'valid':True,'dropped_constraint_refused':True,'authority_mutation_refused':True})
    # Read-only legacy stage-proposal gauntlet; never --apply or sign.
    root=Path(__file__).resolve().parents[3]
    proposal=json.loads(Path(__file__).with_name('stages.json').read_text())
    with tempfile.TemporaryDirectory(prefix='ow85-stages-') as temp:
        path=Path(temp)/'proposal.json'
        path.write_text(json.dumps(proposal))
        result=subprocess.run([str(war),'plan','--proposal',str(path),'--reviewed','--json'],cwd=root,text=True,capture_output=True,timeout=30)
        if result.returncode:raise ValueError('stage proposal refused')
        # Milestone-cycle semantics are tested through the public graph parser;
        # the legacy proposal gauntlet does not promise that full graph check.
    # A review response is a supplied verification record, not a human act.
    subject={'warrant':'test:work','contract_digest':'sha256:'+'a'*64,'result_digest':'sha256:'+'b'*64}
    review={'schema':'oh.war/record/1.0.0-rc.2','id':'test:review','kind':'verification',
            'subject':subject,'actor':{'id':'test:verifier','kind':'agent','role':'verifier'},
            'policy_ref':None,'evidence_refs':[],
            'payload':{'performer_id':'test:performer','candidate_digest':subject['result_digest'],
                       'obligations':[{'id':'OBL-1','scope':'Signup fixture only','disposition':'not-established','evidence_refs':[]}],
                       'isolation_refs':[]},'provenance':{'source':'test:fixture','authenticity':'unverified'}}
    request={'schema':'oh.war/sdk-request/v1','operation':'records','records':[json.dumps(review)],'subject':subject}
    code,result=call(war,request,root)
    if code or result['result']['qualification_established']:raise ValueError(('review claims authority',result))
    request['subject']=dict(subject,result_digest='sha256:'+'f'*64)
    code,result=call(war,request,root)
    if code==0 or result['diagnostics'][0]['code']!='record.subject':raise ValueError('review candidate mismatch not refused')
    print(json.dumps({'scope':'fixed-artifact-contracts','observations':observations,
                      'model_invocation_measured':False,'human_effort_measured':False,'stage_proposal_validated':True,'review_candidate_mismatch_refused':True,
                      'qualification_established':False},sort_keys=True))


if __name__=='__main__':main()
