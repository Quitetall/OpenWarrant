# Shared condition evaluation contract

Revision3 implementation draft, shared OW-WAR-0078. Canonical owner: OpenWarrant;
LAMU retains a digest/pointer. No signature, readiness or assurance is granted.

OpenWarrant SDK4b626f23b3076ff7e764f08eeb15e8bdfa6af5cd owns F4 syntax validation.
LAMU owns `lamu-openwarrant::conditions`: deterministic evaluation without I/O,
implicit policy, models or filesystem glob semantics. Source capture remains the
separate source profile. Adding evaluation does not create a second SDK compiler.

## In-process profile `lamu.openwarrant.conditions/1`

`evaluate(ValidatedCondition, &Inputs, EvaluationLimits)` returns each condition
field's TRUE/FALSE/UNKNOWN result, combined result and missing field names in
lexical order. Inputs contain optional stage, subsystems and concrete paths.
Missing is UNKNOWN; explicit empty lists are FALSE. OR within a field; AND across
fields with FALSE dominating UNKNOWN. All declared fields are reported.

Path matching follows F4: exact case/Unicode characters, single-star within one
segment, whole-segment double-star across zero or more segments, whole-path
anchoring. Unsupported syntax is refused by the SDK before evaluation.

`effective_inputs(snapshot, task_path, task_stage, overrides, limits)` chooses
explicit overrides, then task stage or Warrant scope for unspecified inputs.
There is no guessed stage. Override origins are nonempty and correspond exactly
to supplied fields. Original task stage and effective origins remain in the result;
source bytes retain original scope. These values declare scope, not authorization.

Positive evaluation limits default to4096 input items,1MiB text and1Mi matching
steps. Input text and provenance are bounded before cloning. Pattern computation
consumes an explicit step quota and refuses with resource-limit; it never returns
UNKNOWN or partial truth on exhaustion. The source snapshot retains validated
metadata for defaults, without rereading mutable files.

Required UNKNOWN inclusion and dependency closure are exercised with the selection
provider in OW-WAR-0079. Evaluation alone does not claim those behaviors complete.
T16–T20 cover truth tables, matching, malformed syntax and bounded operation; scope
fallback and exact override provenance are additional public tests.

Participant writes: OpenWarrant condition SDK/fixtures, integration contract/driver,
0078 evidence; LAMU condition module/tests, SDK dependency pin, compatible source
profile declaration and participant pointer. Historical receipts stay unchanged.
