# SPDX-License-Identifier: Apache-2.0
"""Real SDK/executor/verifier/repair processes; synthetic agents and issuer only."""
import base64
import copy
import hashlib
import json
import os
import subprocess
import sys
import time
import unittest
import uuid
from pathlib import Path

from execution import Executor
from server import Store, SDK, read_file, publish, decode
import test_execution as execution_tests
from verifier_service import Verification
from verifier_attestation import NAMESPACE
from verifier_policy import PROTECTIONS
from verification import VerificationError, request
from hotline import Answers, digest
from verifier_repair import plan


class RepairExecutionFixture:
    def setUp(self):
        self.fixture = t = execution_tests.ExecutionTests();t.setUp();self.addCleanup(t.tearDown)
        if getattr(self, 'staged', False):
            fields = t.staged()
        else:
            draft = t.eligible()
            fields = {'warrant_id': draft['id'], 'source_sha256': draft['source_sha256']}
        program = t.harness.read_text().replace("['git','add','result.txt']", "['git','add','.']")
        program = program.replace("stage=r['stage']", "stage=r.get('stage','api')")
        program = program.replace("assert r['schema']=='oh.war/execution-request/v3'",
                                  "assert r['schema'] in ('oh.war/execution-request/v3','oh.war/execution-request/v4')")
        program = program.replace("subprocess.run(['git','add'", "if r.get('verification_repair'):(p/'fixed.txt').write_text('fixed')\nsubprocess.run(['git','add'", 1)
        t.harness.write_text(program)
        for stage in (['api','ui'] if getattr(self, 'staged', False) else [None]):
            code, attempt = t.call('/api/runs', 'POST', {**fields, **({'stage':stage} if stage else {})})
            self.assertEqual(code, 202)
            self.original = t.wait_run(attempt['attempt_id'])
        self.assertEqual(self.original['work_state'], 'completed')
        t.stop()
        # Give the whole synthetic lifecycle a bounded time allowance.
        t.config.update(timeout_seconds=30,repair_cycles=3);t.config_path.write_text(json.dumps(t.config))
        self.store = Store(t.root/'state', SDK(Path(execution_tests.WAR), t.repo))
        self.addCleanup(lambda: os.close(self.store.lock_fd))
        self.executor = Executor(self.store,t.config_path,read_file,publish,decode)
        self.key = t.root/'machine-key'
        subprocess.run(['ssh-keygen','-q','-t','ed25519','-N','','-f',str(self.key)],check=True)
        issuer=t.root/'issuer.json';issuer.write_text(json.dumps({'schema':'oh.war/verifier-issuer/v1',
            'public_key':' '.join(self.key.with_suffix('.pub').read_text().split()[:2]),'principal':'fixture'}))
        verifier=t.root/'reviewer.py';verifier.write_text("""import json,sys,hashlib,pathlib
r=json.load(sys.stdin)
h=hashlib.sha256(json.dumps(r,sort_keys=True,separators=(',',':'),ensure_ascii=False).encode()).hexdigest()
ok=pathlib.Path('fixed.txt').exists()
f=[] if ok else [{'id':'F1','observation':'Required marker absent','scope':'fixed.txt fixture','evidence':['fixed.txt missing'],'status':'violation','repairable':True}]
print(json.dumps({'schema':'oh.war/verification-result/v1','verification_id':r['verification_id'],'request_sha256':h,'verdict':'pass' if ok else 'fail','summary':'Fixture checked marker','findings':f}))
""")
        config=t.root/'verification.json';config.write_text(json.dumps({'schema':'oh.war/verifier-config/v1',
            'performer':{'id':'worker','argv':t.config['argv']},'verifier':{'id':'reviewer','argv':[sys.executable,str(verifier)]},
            'cost_mode':'free','spend_limit_usd':10,'timeout_seconds':10}))
        self.service=Verification(self.executor,config,issuer)

    def verify(self, attempt, prepared=None):
        id=prepared['verification_id'] if prepared else str(uuid.uuid4())
        job=prepared or self.service.prepare({'attempt_id':attempt,'verification_id':id})
        now=int(time.time());payload=json.dumps({'schema':'oh.war/harness-protection/v1',
            'basis_sha256':job['basis_sha256'],'nonce':id,'issued_at_unix':now,'expires_at_unix':now+120,
            'evidence_ref':'fixture://repair-loop','protections':{p:'pass' for p in PROTECTIONS}}).encode()
        signature=subprocess.run(['ssh-keygen','-Y','sign','-f',str(self.key),'-n',NAMESPACE],input=payload,
            stdout=subprocess.PIPE,stderr=subprocess.PIPE,check=True).stdout
        self.service.start(id,{'payload_base64':base64.b64encode(payload).decode(),
                               'signature_base64':base64.b64encode(signature).decode()})
        deadline=time.monotonic()+5
        while time.monotonic()<deadline:
            job=self.service.get(id)
            if job['state']=='finished': return job
            time.sleep(.02)
        self.fail('Verifier did not finish')

    def wait_repair(self, attempt):
        deadline=time.monotonic()+5
        while time.monotonic()<deadline:
            record=self.executor.get(attempt['attempt_id'])
            if record['execution_state']!='running':return record
            time.sleep(.02)
        self.fail('Repair did not finish')


class RepairExecutionTests(RepairExecutionFixture, unittest.TestCase):
    def test_other_human_decisions_cannot_launch_repair(self):
        token = 'fixture-only-human-credential-0001'
        answers = Answers(self.executor, {'schema': 'oh.war/hotline-config/v1', 'responders': [{
            'id': 'fixture-human', 'kind': 'human', 'governing_warrants': [self.original['warrant_id']],
            'token_sha256': hashlib.sha256(token.encode()).hexdigest()}]})
        self.service.answers = answers
        for action in ('stop', 'revise_scope', 'verify_again'):
            with self.subTest(action=action):
                failed = self.verify(self.original['attempt_id'])
                prepared = self.service.rebut(failed['verification_id'], {
                    'verification_id': str(uuid.uuid4()), 'argument': 'Recheck fixture requirement',
                    'evidence': ['fixture contract'], 'finding_ids': ['F1']})
                rechecked = self.verify(self.original['attempt_id'], prepared)
                pending = self.service.dispute(rechecked['verification_id'], answers)
                self.service.settle(rechecked['verification_id'], {
                    'question_sha256': pending['question_sha256'], 'action': action,
                    'reason': 'Human selected next action', 'evidence': ['fixture contract']}, token, answers)
                preview = self.service.repair_preview(rechecked['verification_id'])
                self.assertEqual(preview['state'], 'blocked')
                self.assertIn(action, preview['reason'])
                with self.assertRaises(VerificationError):
                    self.service.repair(rechecked['verification_id'], {})
                self.assertEqual(len(self.executor.records()), 1)
                self.assertEqual(self.service.get(rechecked['verification_id'])['effective_verdict'], 'fail')
                if action != 'verify_again':
                    with self.assertRaises(VerificationError):
                        self.service.reverify(rechecked['verification_id'], {'verification_id': str(uuid.uuid4())})
                    continue
                fields = {'verification_id': str(uuid.uuid4())}
                prepared = self.service.reverify(rechecked['verification_id'], fields)
                self.assertEqual(prepared['request']['schema'], 'oh.war/verification-request/v3')
                self.assertEqual(prepared['request']['human_recheck']['prior_record'], rechecked['record'])
                for target in ('candidate', 'decision', 'observation', 'respondent'):
                    altered = copy.deepcopy(prepared['request'])
                    context = altered['human_recheck']
                    if target == 'candidate': altered['candidate_revision'] = 'a' * 40
                    if target == 'decision': context['decision']['action'] = 'repair'
                    if target == 'observation': context['prior_record']['observation']['verdict'] = 'pass'
                    if target == 'respondent': context['decision']['response']['respondent_kind'] = 'ai'
                    with self.subTest(target=target), self.assertRaises(VerificationError): request(altered)
                self.assertEqual(self.service.reverify(rechecked['verification_id'], fields), prepared)
                with self.assertRaises(VerificationError):
                    self.service.reverify(rechecked['verification_id'], {'verification_id': str(uuid.uuid4())})
                self.service.answers = None
                with self.assertRaises(VerificationError):
                    self.verify(self.original['attempt_id'], prepared)
                self.assertEqual(self.service.get(prepared['verification_id'])['state'], 'prepared')
                self.service.answers = answers
                result = self.verify(self.original['attempt_id'], prepared)
                self.assertEqual(result['effective_verdict'], 'fail')
                self.assertTrue(result['human_review_required'])
                self.assertEqual(self.service.repair_preview(result['verification_id'])['state'], 'escalate')
                self.assertEqual(self.service.get(rechecked['verification_id'])['effective_verdict'], 'fail')

    def test_rebuttal_preserves_fail_and_unresolved_recheck_waits_for_human(self):
        failed=self.verify(self.original['attempt_id'])
        fields={'verification_id':str(uuid.uuid4()),'argument':'Inspect the fixture expectation again',
                'evidence':['fixture contract paragraph 1'],'finding_ids':['F1']}
        prepared=self.service.rebut(failed['verification_id'],fields)
        self.assertEqual(prepared['state'],'prepared')
        self.assertEqual(prepared['request']['schema'],'oh.war/verification-request/v2')
        self.assertEqual(prepared['request']['recheck']['observation'],failed['record']['observation'])
        self.assertEqual(self.service.rebut(failed['verification_id'],fields),prepared)
        rechecked=self.verify(self.original['attempt_id'],prepared)
        self.assertEqual(rechecked['effective_verdict'],'fail')
        self.assertTrue(rechecked['human_review_required'])
        self.assertEqual(self.service.repair_preview(rechecked['verification_id'])['state'],'escalate')
        self.assertEqual(self.service.repair_preview(failed['verification_id'])['state'],'escalate')
        with self.assertRaises(VerificationError):self.service.repair(failed['verification_id'],{})
        with self.assertRaises(VerificationError):
            self.service.rebut(failed['verification_id'],{**fields,'verification_id':str(uuid.uuid4())})
        with self.assertRaises(VerificationError):
            self.service.rebut(rechecked['verification_id'],{**fields,'verification_id':str(uuid.uuid4())})
        self.assertEqual(self.service.get(failed['verification_id'])['effective_verdict'],'fail')
        self.assertEqual(len(self.executor.records()),1)
        self.assertEqual(self.service.dispute(rechecked['verification_id'],None)['state'],'waiting_for_authorized_responder')
        token='fixture-only-human-credential-0001'
        answers=Answers(self.executor,{'schema':'oh.war/hotline-config/v1','responders':[{
            'id':'fixture-human','kind':'human','governing_warrants':[self.original['warrant_id']],
            'token_sha256':hashlib.sha256(token.encode()).hexdigest()}]})
        pending=self.service.dispute(rechecked['verification_id'],answers)
        fields={'question_sha256':pending['question_sha256'],'action':'repair','reason':'Keep fixture requirement',
                'evidence':['fixture requirement']}
        settled=self.service.settle(rechecked['verification_id'],fields,token,answers)
        self.assertEqual(settled['state'],'decision_recorded')
        self.assertFalse(settled['decision']['changes_verdict'])
        self.assertEqual(self.service.settle(rechecked['verification_id'],fields,token,answers),settled)
        with self.assertRaises(VerificationError):
            self.service.settle(rechecked['verification_id'],{**fields,'action':'stop'},token,answers)
        self.assertEqual(self.service.get(rechecked['verification_id'])['effective_verdict'],'fail')
        self.service.answers=answers
        self.assertEqual(self.service.repair_preview(rechecked['verification_id'])['state'],'ready')
        self.service.answers=Answers(self.executor,{'schema':'oh.war/hotline-config/v1','responders':[]})
        self.assertEqual(self.service.repair_preview(rechecked['verification_id'])['state'],'escalate')
        with self.assertRaises(VerificationError):self.service.repair(rechecked['verification_id'],{})
        self.service.answers=answers
        repaired=self.wait_repair(self.service.repair(rechecked['verification_id'],{}))
        self.assertEqual(repaired['work_state'],'completed',repaired)
        self.assertEqual(repaired['verification_repair']['decision_sha256'],digest(settled['decision']))
        self.assertEqual(self.service.get(rechecked['verification_id'])['effective_verdict'],'fail')

    def test_rebuttal_requires_existing_findings_and_evidence_without_authority_fields(self):
        failed=self.verify(self.original['attempt_id'])
        fields={'verification_id':str(uuid.uuid4()),'argument':'Inspect evidence',
                'evidence':['fixture observation'],'finding_ids':['F1']}
        for patch in ({'evidence':[]},{'argument':''},{'finding_ids':['invented']},{'qualified':True}):
            with self.subTest(patch=patch),self.assertRaises(VerificationError):
                self.service.rebut(failed['verification_id'],{**fields,**patch})
        self.assertEqual(len(self.service.listing()['jobs']),1)

    def test_repair_hotline_resume_preserves_lineage_budget_and_single_cycle(self):
        failed=self.verify(self.original['attempt_id'])
        implementation=self.fixture.harness.read_text()
        self.fixture.harness.write_text("""import json,sys
r=json.load(sys.stdin)
assert r['verification_repair']
print(json.dumps({'schema':'oh.war/execution-question/v1','attempt_id':r['attempt_id'],
'source_sha256':r['source_sha256'],'notes':'Repair needs technical answer','next_steps':['Answer then resume'],
'question':{'kind':'technical','text':'Use existing marker format?','direct_human':False,'affected_stages':['repair']}}))
""")
        paused=self.wait_repair(self.service.repair(failed['verification_id'],{}))
        self.assertEqual(paused['work_state'],'blocked',paused)
        token='fixture-only-responder-credential-001'
        answers=Answers(self.executor,{'schema':'oh.war/hotline-config/v1','responders':[{
            'id':'fixture-adviser','kind':'ai','governing_warrants':[],
            'token_sha256':hashlib.sha256(token.encode()).hexdigest()}]})
        question_hash=digest(paused['question'])
        answers.submit(paused['attempt_id'],{'question_sha256':question_hash,'answer':'Use existing format',
                                            'evidence':['fixture contract']},token)
        self.fixture.harness.write_text(implementation)
        resumed=self.wait_repair(self.executor.resume(paused['attempt_id'],{'question_sha256':question_hash},answers))
        self.assertEqual(resumed['work_state'],'completed',resumed)
        self.assertEqual(resumed['verification_repair'],paused['verification_repair'])
        self.assertLess(resumed['remaining_seconds'],paused['remaining_seconds'])
        self.assertEqual(plan(self.service.get(failed['verification_id']),list(self.executor.records().values()))['cycles_used'],1)
        self.assertEqual(self.verify(resumed['attempt_id'])['effective_verdict'],'pass')

    def test_failure_repair_new_revision_and_independent_pass_preserve_history(self):
        failed=self.verify(self.original['attempt_id']);self.assertEqual(failed['effective_verdict'],'fail',failed)
        repaired=self.service.repair(failed['verification_id'],{})
        replay=self.service.repair(failed['verification_id'],{})
        self.assertEqual(repaired['attempt_id'],replay['attempt_id'])
        deadline=time.monotonic()+5
        while time.monotonic()<deadline:
            record=self.executor.get(repaired['attempt_id'])
            if record['execution_state']!='running':break
            time.sleep(.02)
        self.assertEqual(record['work_state'],'completed',record)
        self.assertNotEqual(record['result_revision'],self.original['result_revision'])
        self.assertEqual(record['worktree'],self.original['worktree'])
        passed=self.verify(record['attempt_id']);self.assertEqual(passed['effective_verdict'],'pass',passed)
        self.assertEqual(self.service.get(failed['verification_id'])['effective_verdict'],'fail')
        self.assertFalse(passed['qualified']);self.assertFalse(record['qualified'])

    def test_limit_budget_and_stale_candidate_refuse_before_repair_claim(self):
        failed=self.verify(self.original['attempt_id'])
        self.executor.config['repair_cycles']=0
        with self.assertRaises(VerificationError):self.service.repair(failed['verification_id'],{})
        self.executor.config['repair_cycles']=3
        self.executor.config['timeout_seconds']=0
        with self.assertRaises(VerificationError):self.service.repair(failed['verification_id'],{})
        self.executor.config['timeout_seconds']=30
        with self.assertRaises(VerificationError):self.service.repair(failed['verification_id'],{'checks':[]})
        candidate=Path(self.original['worktree'])
        (candidate/'outside.txt').write_text('unrelated change')
        subprocess.run(['git','add','.'],cwd=candidate,check=True,stdout=subprocess.DEVNULL)
        subprocess.run(['git','commit','-qm','fixture stale candidate'],cwd=candidate,check=True)
        with self.assertRaises(VerificationError):self.service.repair(failed['verification_id'],{})
        self.assertEqual(len(self.executor.records()),1)


class StagedRepairTests(RepairExecutionFixture, unittest.TestCase):
    staged = True

    def test_repair_rechecks_every_stage_then_independently_verifies_new_commit(self):
        failed=self.verify(self.original['attempt_id'])
        record=self.wait_repair(self.service.repair(failed['verification_id'],{}))
        self.assertEqual(record['work_state'],'completed',record)
        from stages import completed_from_evidence
        done=completed_from_evidence(record['policy']['stage_plan'],record['source_sha256'],
                                     record['result_revision'],record['stage_checkpoint'])
        self.assertEqual(done,{'api','ui'})
        self.assertNotIn('stage',record)
        self.assertEqual(self.verify(record['attempt_id'])['effective_verdict'],'pass')

    def test_regressing_completed_stage_prevents_repair_completion(self):
        failed=self.verify(self.original['attempt_id'])
        program=self.fixture.harness.read_text()
        program=program.replace("if r.get('verification_repair'):",
            "if r.get('verification_repair'):(p/'api.txt').write_text('broken')\nif r.get('verification_repair'):")
        self.fixture.harness.write_text(program)
        record=self.wait_repair(self.service.repair(failed['verification_id'],{}))
        self.assertEqual(record['work_state'],'failed',record)
        self.assertEqual(record['execution_state'],'stopped')
        self.assertEqual(record['cause'],'Required stage checks failed')
        with self.assertRaises(VerificationError):
            self.service.prepare({'attempt_id':record['attempt_id'],'verification_id':str(uuid.uuid4())})
