# SPDX-License-Identifier: Apache-2.0
import hashlib
import unittest

from hotline import responders, digest, HotlineError
from verification import VerificationError
from verifier_decision import question, decide


class VerifierDecisionTests(unittest.TestCase):
    def setUp(self):
        self.warrant='00000000-0000-4000-8000-000000000001'
        self.token='fixture-only-human-credential-0001'
        self.config={'schema':'oh.war/hotline-config/v1','responders':[{
            'id':'fixture-human','kind':'human','governing_warrants':[self.warrant],
            'token_sha256':hashlib.sha256(self.token.encode()).hexdigest()}]}
        self.job={'human_review_required':True,'evidence_state':'retained','verification_id':'recheck',
                  'request':{'warrant_id':self.warrant,'source_sha256':'a'*64,'candidate_revision':'b'*40},
                  'record':{'sequence':3,'observation':{'verdict':'fail'}}}
        self.q=question(self.job,digest(self.config))
        self.fields={'question_sha256':digest(self.q),'action':'repair','reason':'Keep approved expectations',
                     'evidence':['fixture requirement']}

    def decision(self):return decide(self.q,self.fields,self.token,responders(self.config))

    def test_human_decision_binds_scope_without_clearing_verdict_or_dispatching(self):
        r=self.decision()
        self.assertEqual(r['response']['respondent'],'fixture-human')
        self.assertFalse(r['qualified']);self.assertFalse(r['changes_verdict']);self.assertFalse(r['dispatch_permitted'])
        self.assertEqual(self.job['record']['observation']['verdict'],'fail')

    def test_ai_wrong_scope_and_wrong_credential_refuse(self):
        self.config['responders'][0]['kind']='ai'
        with self.assertRaises(HotlineError):self.decision()
        self.config['responders'][0]['kind']='human';self.config['responders'][0]['governing_warrants']=[]
        with self.assertRaises(HotlineError):self.decision()
        self.config['responders'][0]['governing_warrants']=[self.warrant];self.token='wrong-fixture-credential-000000000'
        with self.assertRaises(HotlineError):self.decision()

    def test_stale_basis_missing_evidence_and_injected_authority_refuse(self):
        for patch in ({'action':'accept'},{'qualified':True},{'evidence':[]},{'reason':''}):
            with self.subTest(patch=patch),self.assertRaises(VerificationError):
                decide(self.q,{**self.fields,**patch},self.token,responders(self.config))
        changed={**self.q,'candidate_revision':'c'*40}
        with self.assertRaises(HotlineError):decide(changed,self.fields,self.token,responders(self.config))

    def test_absent_evidence_cannot_create_dispute_decision_basis(self):
        self.job['evidence_state']='unavailable'
        with self.assertRaises(VerificationError):question(self.job,digest(self.config))
