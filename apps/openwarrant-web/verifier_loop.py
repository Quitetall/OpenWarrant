# SPDX-License-Identifier: Apache-2.0
"""One durable verification/repair transition per tick; no authority or model calls."""
import uuid
import time

from verification import identity, require
from verifier_snapshot import Snapshot
from verifier_policy import admission
from verifier_workspace import unchanged


class Loop:
    """The host schedules ticks and supplies a read-only signed-receipt lookup.

    Claims and repairs persist through their existing services. A deterministic
    verification identity makes repeated ticks and restart converge on those
    records. Receipt lookup must not launch a provider or incur untracked cost.
    """

    def __init__(self, service, receipt_lookup):
        self.service = service
        self.receipt_lookup = receipt_lookup

    def tick(self, attempt_id):
        require(identity(attempt_id), 'Exact root attempt identity required')
        service, executor = self.service, self.service.executor
        with executor.lock:
            root = executor.get(attempt_id)
            current = root
            visited = set()
            while True:
                require(current['attempt_id'] not in visited, 'Cyclic repair/resume lineage')
                visited.add(current['attempt_id'])
                resumed = [r for r in executor.records().values() if r.get('resume_from') == current['attempt_id']]
                require(len(resumed) <= 1, 'Ambiguous resume lineage')
                if resumed:
                    current = executor.get(resumed[0]['attempt_id'])
                    require(current['warrant_id'] == root['warrant_id'], 'Resume lineage changed Warrant')
                    continue
                out = {'schema': 'oh.war/verifier-loop/v1', 'root_attempt_id': attempt_id,
                       'attempt_id': current['attempt_id'], 'warrant_id': root['warrant_id'],
                       'qualified': False, 'state': 'waiting', 'reason': ''}
                if current['execution_state'] != 'stopped':
                    return {**out, 'state': 'waiting_for_execution', 'reason': current['execution_state']}
                if current['work_state'] != 'completed':
                    return {**out, 'state': 'needs_attention', 'reason': current['work_state']}
                id = str(uuid.uuid5(uuid.UUID(attempt_id), 'verification:' + current['attempt_id']))
                if service.jobs.read(id) is None:
                    job = service.prepare({'attempt_id': current['attempt_id'], 'verification_id': id})
                else:
                    job = service.get(id)
                out['verification_id'] = id
                if job['state'] == 'prepared':
                    receipt = self.receipt_lookup(job)
                    if receipt is None:
                        return {**out, 'state': 'waiting_for_protection', 'reason': 'Signed harness receipt required'}
                    service.start(id, receipt)
                    return {**out, 'state': 'waiting_for_verification', 'reason': 'Dispatch recorded'}
                if job['state'] != 'finished':
                    return {**out, 'state': 'waiting_for_verification', 'reason': job['state']}
                if job['effective_verdict'] == 'pass':
                    snapshot = Snapshot(executor, current['attempt_id'], service.config_path, service.issuer_path)()
                    basis = admission(snapshot['config'], snapshot['attempt'], snapshot['execution_policy'], snapshot['source_sha256'])
                    require(basis['basis_sha256'] == job['basis_sha256'], 'Passing loop result has stale basis')
                    unchanged(snapshot['source_path'], job['request']['candidate_revision'], time.monotonic() + 5)
                    return {**out, 'state': 'checks_passed', 'reason': 'Independent observations passed; acceptance remains separate'}
                proposal = service.repair_preview(id)
                if proposal['state'] == 'already_dispatched':
                    current = executor.get(proposal['attempt_id'])
                    require(current['warrant_id'] == root['warrant_id'], 'Repair lineage changed Warrant')
                    continue
                if proposal['state'] == 'ready':
                    repair = service.repair(id, {})
                    return {**out, 'attempt_id': repair['attempt_id'], 'state': 'waiting_for_execution', 'reason': 'Repair dispatched'}
                return {**out, 'state': 'needs_attention', 'reason': proposal['reason']}
