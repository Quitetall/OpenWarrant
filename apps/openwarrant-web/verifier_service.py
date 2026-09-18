# SPDX-License-Identifier: Apache-2.0
"""Configured verification inventory and exact preparation, without implicit launch."""
import json
import re
import base64
import threading
import time

from hotline import digest
from verification import identity, request, request_digest, require
from verifier_jobs import Jobs
from verifier_policy import admission
from verifier_snapshot import Snapshot
from verifier_controller import run
from verifier_budget import allowance
from verifier_repair import plan as repair_plan
from verifier_workspace import unchanged
from verifier_rebuttal import context as rebuttal_context
from verifier_decision import question as dispute_question, decide


class Verification:
    def __init__(self, executor, config_path, issuer_path, answers=None):
        self.executor, self.config_path, self.issuer_path = executor, config_path, issuer_path
        self.jobs = Jobs(executor.root / "verification", publish=executor.publish,
                         read_file=executor.read_file, decode=executor.decode)
        self.live = set()
        self.answers = answers

    def initial(self, id):
        require(identity(id), "Exact verification identity required")
        require(self.jobs.read(id) is not None, "Unknown verification job")
        return self.jobs.decode(self.jobs.read_file(self.jobs.path(id, 1)))["record"]

    def binding(self, id):
        initial = self.initial(id)
        binding = self.jobs.decode(self.jobs.read_file(self.jobs.root / f"{id}.binding.json"))
        require(isinstance(binding, dict) and set(binding) == {"schema", "attempt_id", "request_sha256"}
                and binding["schema"] == "oh.war/verifier-binding/v1" and identity(binding["attempt_id"])
                and binding["request_sha256"] == request_digest(initial["request"]), "Verifier binding mismatch")
        return binding

    def get(self, id):
        with self.executor.lock:
            initial, binding = self.initial(id), self.binding(id)
            view = self.jobs.view(id, live=id in self.live)
            evidence_state = "not_received"
            if view["record"]["sequence"] >= 2:
                try:
                    consumed = self.jobs.decode(self.jobs.read_file(self.jobs.path(id, 2)))["record"]
                    receipt_hash = consumed["protection_sha256"]
                    receipt = self.jobs.decode(self.jobs.read_file(self.jobs.root / f"protection-{receipt_hash}.json"))
                    require(digest(receipt) == receipt_hash, "Protection receipt integrity mismatch")
                    evidence_state = "retained"
                except Exception:
                    # Preserve signed history and verdict; missing/corrupt evidence
                    # cannot support its current effective verdict.
                    evidence_state = "unavailable"
            effective = (view["record"]["observation"]["verdict"]
                         if view["record"]["sequence"] == 3 and evidence_state == "retained" else "unknown")
            return {**view, "attempt_id": binding["attempt_id"],
                    "request": initial["request"], "basis_sha256": initial["basis_sha256"],
                    "dispatch_permitted": False, "evidence_state": evidence_state, "effective_verdict": effective,
                    "human_review_required": initial['request']['schema'] == 'oh.war/verification-request/v2'
                    and view['record']['sequence'] == 3 and effective != 'pass'}

    def listing(self):
        with self.executor.lock:
            ids = []
            for path in self.jobs.root.glob("*.1.json"):
                require(re.fullmatch(r"[0-9a-f-]{36}\.1\.json", path.name), "Unknown verifier claim filename")
                ids.append(path.name[:-7])
                require(len(ids) <= 256, "Verifier inventory limit exceeded")
            return {"schema": "oh.war/verification-inventory/v1", "jobs": [self.get(id) for id in sorted(ids)],
                    "qualified": False}

    def prepare(self, fields, recheck=None):
        require(isinstance(fields, dict) and set(fields) == {"attempt_id", "verification_id"}
                and all(identity(value) for value in fields.values()), "Exact execution and verification identities required")
        e, id = self.executor, fields["verification_id"]
        with e.lock:
            require(len(self.listing()["jobs"]) < 256 or self.jobs.path(id, 1).exists(), "Verifier inventory limit exceeded")
            snapshot = Snapshot(e, fields["attempt_id"], self.config_path, self.issuer_path)()
            config, attempt, execution_policy = snapshot["config"], snapshot["attempt"], snapshot["execution_policy"]
            preview = admission(config, attempt, execution_policy, snapshot["source_sha256"])
            require(preview["state"] != "blocked", preview["reason"])
            expected = request({"schema": "oh.war/verification-request/v2" if recheck else "oh.war/verification-request/v1", "verification_id": id,
                "warrant_id": attempt["warrant_id"], "source_sha256": snapshot["source_sha256"],
                "candidate_revision": attempt["result_revision"], "policy_sha256": digest(execution_policy),
                "performer": config["performer"]["id"], "verifier": config["verifier"]["id"],
                "checks": execution_policy["checks"], "source": e.store.get(attempt["warrant_id"])["source"],
                **({'recheck':recheck} if recheck else {})})
            binding = {"schema": "oh.war/verifier-binding/v1", "attempt_id": fields["attempt_id"],
                       "request_sha256": request_digest(expected)}
            path = self.jobs.root / f"{id}.binding.json"
            try:
                self.jobs.publish(path, json.dumps(binding).encode())
            except FileExistsError:
                require(self.jobs.decode(self.jobs.read_file(path)) == binding, "Verification identity already bound")
            self.jobs.claim(expected, preview["basis_sha256"])
            return self.get(id)

    def rebut(self, id, fields):
        with self.executor.lock:
            job,binding=self.get(id),self.binding(id)
            recheck=rebuttal_context(job,fields)
            children=[j for j in self.listing()['jobs'] if j['request'].get('recheck',{}).get('verification_id') == id]
            require(not children or (len(children) == 1 and children[0]['verification_id'] == fields['verification_id']),
                    'Existing independent recheck must settle before another challenge')
            current=Snapshot(self.executor,binding['attempt_id'],self.config_path,self.issuer_path)()
            basis=admission(current['config'],current['attempt'],current['execution_policy'],current['source_sha256'])
            require(basis['basis_sha256'] == job['basis_sha256'], 'Rebuttal basis changed; new verification required')
            unchanged(current['source_path'],job['request']['candidate_revision'],time.monotonic()+5)
            return self.prepare({'attempt_id':binding['attempt_id'],'verification_id':fields['verification_id']},recheck)

    def start(self, id, fields):
        require(isinstance(fields, dict) and set(fields) == {"payload_base64", "signature_base64"}
                and all(isinstance(v, str) and 0 < len(v) <= 21848 for v in fields.values()),
                "Bounded signed protection receipt required")
        payload, signature = (base64.b64decode(fields[k], validate=True)
                              for k in ("payload_base64", "signature_base64"))
        with self.executor.lock:
            initial, binding = self.initial(id), self.binding(id)
            if self.jobs.read(id)["sequence"] != 1:
                return self.get(id)
            for job in self.listing()["jobs"]:
                record = job["record"]
                require(job["evidence_state"] != "unavailable", "Prior verifier evidence unavailable")
                require(record["sequence"] != 2 and not (
                    record["sequence"] == 3 and record["observation"]["execution_state"] == "unknown"),
                    "Prior verifier running or uncertain; inspect before replacement")
            snapshot = Snapshot(self.executor, binding["attempt_id"], self.config_path, self.issuer_path)
            current = snapshot()
            budget = allowance(initial["request"]["warrant_id"], self.executor.config, current["config"],
                               list(self.executor.records().values()), self.listing()["jobs"])
            return run(self.jobs, initial["request"], snapshot, self.executor.lock,
                       self.jobs.root / ("workspace-" + id), payload=payload, signature=signature,
                       schedule=lambda work: self.schedule(id, work), remaining_seconds=budget["remaining_seconds"])

    def repair_preview(self, id):
        with self.executor.lock:
            job=self.get(id)
            decision=self.dispute(id,self.answers)['decision'] if job['human_review_required'] and self.answers else None
            preview=repair_plan(job, list(self.executor.records().values()),
                                self.executor.config.get('repair_cycles', 3),
                                human_repair=decision is not None and decision['action']=='repair')
            if decision and decision['action']!='repair':
                preview.update(state='blocked',reason='Human decision requires '+decision['action'])
            if any(j['request'].get('recheck',{}).get('verification_id') == id and j['human_review_required']
                   for j in self.listing()['jobs']):
                preview.update(state='escalate',reason='Unresolved independent recheck requires human decision')
            return preview

    def dispute(self, id, answers):
        with self.executor.lock:
            job=self.get(id)
            q=dispute_question(job,answers.authorization_digest if answers else None)
            path=self.jobs.root / (id+'.decision-'+digest(q)+'.json')
            decision=None
            try:
                envelope=self.jobs.decode(self.jobs.read_file(path))
                require(isinstance(envelope,dict) and set(envelope)=={'record','sha256'}
                        and digest(envelope['record'])==envelope['sha256']
                        and envelope['record']['question']==q,'Dispute decision integrity mismatch')
                decision=envelope['record']
            except FileNotFoundError:
                pass
            eligible=[{'id':r['id'],'kind':r['kind']} for r in answers.rows
                      if r['kind']=='human' and q['warrant_id'] in r['governing_warrants']] if answers else []
            return {'question':q,'question_sha256':digest(q),'decision':decision,
                    'eligible_responders':eligible,'state':'decision_recorded' if decision else
                    ('waiting_for_human' if eligible else 'waiting_for_authorized_responder'),'qualified':False}

    def settle(self, id, fields, credential, answers):
        require(answers is not None,'Authorized human responders are not configured')
        with self.executor.lock:
            job,binding=self.get(id),self.binding(id)
            current=Snapshot(self.executor,binding['attempt_id'],self.config_path,self.issuer_path)()
            basis=admission(current['config'],current['attempt'],current['execution_policy'],current['source_sha256'])
            require(basis['basis_sha256']==job['basis_sha256'],'Dispute basis changed; new verification required')
            unchanged(current['source_path'],job['request']['candidate_revision'],time.monotonic()+5)
            pending=self.dispute(id,answers)
            record=decide(pending['question'],fields,credential,answers.rows)
            if pending['decision'] is not None:
                require(pending['decision']==record,'Dispute already has a different retained decision')
                return pending
            self.jobs.publish(self.jobs.root/(id+'.decision-'+pending['question_sha256']+'.json'),
                              json.dumps({'record':record,'sha256':digest(record)}).encode())
            return self.dispute(id,answers)

    def repair(self, id, fields):
        require(isinstance(fields, dict) and not fields, 'Repair accepts no replacement scope or checks')
        e = self.executor
        with e.lock:
            job, binding = self.get(id), self.binding(id)
            proposal = self.repair_preview(id)
            if proposal['state'] == 'already_dispatched':
                return e.get(proposal['attempt_id'])
            require(proposal['state'] == 'ready', proposal['reason'])
            current = Snapshot(e, binding['attempt_id'], self.config_path, self.issuer_path)()
            preview = admission(current['config'], current['attempt'], current['execution_policy'], current['source_sha256'])
            require(preview['state'] != 'blocked' and preview['basis_sha256'] == job['basis_sha256'],
                    'Repair source, policy or verifier configuration changed')
            unchanged(current['source_path'], job['request']['candidate_revision'], time.monotonic() + 5)
            jobs = self.listing()['jobs']
            require(all(j['record']['sequence'] != 2 for j in jobs), 'Verifier still running or uncertain')
            budget = allowance(job['request']['warrant_id'], e.config, current['config'], list(e.records().values()), jobs)
            repair = {'verification_id': id, 'observation_sha256': digest(job['record']),
                      'candidate_revision': job['request']['candidate_revision'],
                      'findings': proposal['findings'], 'failed_checks': proposal.get('failed_checks', [])}
            if job['human_review_required']:
                decision=self.dispute(id,self.answers)['decision'] if self.answers else None
                require(decision is not None and decision['action']=='repair','Current human repair decision required')
                repair['decision_sha256']=digest(decision)
            return e.start({'warrant_id': job['request']['warrant_id'], 'source_sha256': current['source_sha256']},
                           repair_context={'binding': repair, 'remaining_seconds': budget['remaining_seconds']})

    def schedule(self, id, work):
        self.live.add(id)
        def worker():
            try:
                work()
            finally:
                # If final persistence failed, consumed claim remains UNKNOWN.
                with self.executor.lock:
                    self.live.discard(id)
        thread = threading.Thread(target=worker, daemon=True)
        try:
            thread.start()
        except Exception:
            self.live.discard(id)
            raise
        return self.get(id)
