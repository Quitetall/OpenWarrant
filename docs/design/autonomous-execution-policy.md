# Autonomous execution: owner budget decision

Recorded 2026-09-14 from the owner's answer about autonomous work within signed
Warrant scope. This records an execution constraint, not a Warrant signature,
SAS acceptance, or a claim that runtime enforcement already exists.

## Spend policy

| Setting | Meaning |
| --- | --- |
| Default | USD 10 per autonomous run. |
| Configured amount | The owner may select a different nonnegative limit. Zero permits no paid spend. |
| Explicitly unset | No dollar ceiling. This is distinct from an absent override, which uses the default. |
| Accounting | Refuse additional paid-model calls without reliable cost tracking, including when the ceiling is explicitly unset. |

Display the effective limit and recorded cost. Unknown cost remains unknown;
neither an unknown cost nor an unset ceiling means zero cost. Agents cannot
raise or remove the effective ceiling themselves.

Within a run, implementation, independent review, retries and replacement
workers share the budget. A continuation or repair does not silently reset it.
A finite cap requires admission control for additional requests and concurrent
work as well as cost reporting; after-the-fact totals alone cannot enforce it.
Provider fallback and nested model calls must be accounted for before describing
the cap as enforced. The owner has not approved untracked paid execution.

This applies to additional paid calls that the agent dispatches. It does not
claim control over the host chat application's existing subscription or billing.

## Scope and other limits

Existing signed scope, prerequisites, human-only acts, and independent
verification requirements still apply. The existing user-selectable three-repair
fallback remains. The suggested two-hour run and two-implementer/one-verifier
limits were not separately confirmed by this budget answer; they remain
proposals, not newly granted permissions.

Before dispatching paid work, establish accounting and, for a finite ceiling,
the corresponding enforcement mechanism. Until then, continue work using the
current session and local tools without launching additional paid-model calls.

This is a repository/workflow policy choice. It does not make USD 10 a universal
OpenWarrant document-format rule. The current RC.2 source-set bytes and signed
0074 contract remain unchanged; carry this policy decision into the applicable
workflow configuration design during adoption.
