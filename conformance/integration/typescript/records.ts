// SPDX-License-Identifier: Apache-2.0
import type { Document as Dispatch } from '../../../schemas/typescript/stage-dispatch.js';

const dispatch: Dispatch = {
  dispatch_id: 'fixture', warrant_ref: 'fixture://warrant', contract_revision: 1,
  contract_digest: 'fixture', milestone_id: 'M1', stage_id: 'S1', attempt_id: 'A1',
  attempt_basis_digest: 'fixture', objective: 'Compile record types',
  workspace_basis_digest: 'fixture', context_manifest_digest: 'fixture',
  attempt_kind: 'initial', tokens: null,
};
const { objective, ...missingObjective } = dispatch;
// @ts-expect-error Required objective cannot be omitted.
const invalidRequired: Dispatch = missingObjective;
// @ts-expect-error Known attempt kinds are not arbitrary strings.
const invalidKind: NonNullable<Dispatch['attempt_kind']> = 'invented';
// @ts-expect-error Contract revisions are numbers, not strings.
const invalidRevision: Dispatch['contract_revision'] = 'one';
void [dispatch, objective, invalidRequired, invalidKind, invalidRevision];

import type { Document as Deliverables } from '../../../schemas/typescript/deliverables.js';
const failurePair: Deliverables['failures'][number] = ['source', 'reason'];
// @ts-expect-error A failure record is exactly a two-element tuple.
const shortFailure: Deliverables['failures'][number] = ['source'];
// @ts-expect-error A failure tuple cannot contain a third element.
const longFailure: Deliverables['failures'][number] = ['source', 'reason', 'extra'];
// @ts-expect-error Both tuple elements must be strings.
const numericFailure: Deliverables['failures'][number] = ['source', 7];
void [failurePair, shortFailure, longFailure, numericFailure];
