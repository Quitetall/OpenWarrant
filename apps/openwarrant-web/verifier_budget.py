# SPDX-License-Identifier: Apache-2.0
"""Conservative aggregate accounting over retained execution and verifier attempts."""
import math

from verification import require


def number(value):
    return type(value) in (int, float) and math.isfinite(value) and value >= 0


def allowance(warrant_id, execution_config, verifier_config, attempts, jobs):
    """Missing observations are unknown, never zero. Prepared jobs use no budget.

    The execution timeout is the Warrant-wide active-time ceiling. The verifier
    timeout additionally bounds each verification. Both configured cost caps apply.
    Caller must serialize admission with writer/verification claim consumption.
    """
    seconds, known_cost, unknown_cost = 0.0, 0.0, False
    observations = []
    for attempt in attempts:
        if attempt['warrant_id'] != warrant_id:
            continue
        require(attempt['execution_state'] == 'stopped', 'Execution accounting uncertain')
        observations.append(attempt)
    for job in jobs:
        if job['request']['warrant_id'] != warrant_id or job['record']['sequence'] == 1:
            continue
        require(job['record']['sequence'] == 3 and job['evidence_state'] == 'retained',
                'Verifier accounting uncertain')
        observed = job['record']['observation']
        require(observed['execution_state'] == 'stopped', 'Verifier execution uncertain')
        observations.append(observed)
    for observed in observations:
        require(number(observed.get('active_seconds')), 'Active time unavailable; budget cannot be established')
        seconds += observed['active_seconds']
        cost = observed.get('cost_usd')
        if cost is None:
            unknown_cost = True
        else:
            require(number(cost), 'Invalid retained cost observation')
            known_cost += cost
    require(number(seconds) and number(known_cost), 'Aggregate accounting overflow')
    total_seconds = execution_config['timeout_seconds']
    per_verifier = verifier_config['timeout_seconds']
    require(number(total_seconds) and number(per_verifier), 'Invalid time ceiling')
    remaining = min(per_verifier, total_seconds - seconds)
    require(remaining > 0, 'Shared Warrant time budget exhausted')
    caps = [c['spend_limit_usd'] for c in (execution_config, verifier_config) if c['spend_limit_usd'] is not None]
    require(all(number(cap) for cap in caps), 'Invalid aggregate spend cap')
    cap = min(caps) if caps else None
    unknown_cost |= verifier_config['cost_mode'] == 'unknown'
    require(cap is None or not unknown_cost, 'Unknown cost cannot satisfy shared hard cap')
    require(cap is None or known_cost <= cap, 'Shared Warrant spend budget exhausted')
    # Currently supported verifier modes are free and unknown. No paid reservation
    # is invented here; metered backends need a reliable reservation interface.
    return {'active_seconds': seconds, 'remaining_seconds': remaining,
            'cost_usd': None if unknown_cost else known_cost, 'known_cost_usd': known_cost,
            'spend_limit_usd': cap}
