# Cross-project observations and qualification status

**Qualification: UNKNOWN / not established.** These are performer-observed
prototype controls. No independent verdict, participant acceptance, protected
operator deployment or assurance mark is recorded here.

## Current observation

On 2026-10-17, both explicit opt-in native tests passed against OpenWarrant
SDK code `9e4630f83d9993629f3d6e92fb0eb657e0011d66`. The record checkout was
`5408abfd475b659147cd8c313ea461ad73e47387`; its relevant Rust/fixture code
matches the validated SDK build. Each test ran alone with a 200-second outer
deadline and a minimal environment. The LAMU HTTP endpoint was offline;
neither test needed it, invoked a model, or made a paid call.

- **BLUT:** actual CPU execution, provider-native seal, capture import and
  current-resolution assessment in a copied test Warrant. Tampered signature
  and output and a wrong catalog are refused; a missing job is UNKNOWN.
- **Katana:** an actual local Kernel turn with scripted provider replies,
  native receipt/log validation, capture import and current assessment in a
  copied test Warrant. Excess capability, changed log, wrong head and fake
  assurance are refusal controls; unavailable current log is UNKNOWN.

[Raw BLUT output](native-replay-20261017/blut.stdout),
[raw Katana output](native-replay-20261017/katana.stdout), their stderr files,
[exact invocations and exits](native-replay-20261017/results.json) and
[input source/binary identities](native-replay-20261017/inputs.json) are
retained. All four original provider binaries matched their recorded hashes
before execution. They remain in the existing retained evidence store.

## Exact source limits

The retained Katana fixture build is from
`0fb33ad44f21b0dc960e1714658f412a5165e8b7`; the BLUT fixture build is from
`eca4ec7`, as recorded by the original source manifest. This is a replay of
those builds against the current SDK, not qualification of newer provider
checkouts. BLUT uses test keys, and Katana uses scripted replies. Both
explicitly report no assurance. Their test fixtures are disposable, including
the native artifacts altered or removed by negative controls; these logs do
not supply current production job/log custody or an eligible receipt for this
Warrant's own business outcome.

## Still required for qualification

- Exact participant-approved adapter/source versions where verified start
  requires them.
- Protected collector caller, executable, policy, native job/log and trusted
  head custody, plus revocation and fencing observations in the chosen host.
- Actual required confinement and capability enforcement. Process output and
  fixture namespace instrumentation alone are insufficient.
- Reliable spend enforcement where a hard spend cap is required; unknown
  cost remains unknown, including the scripted Katana fixture.
- Fresh required complete gates and independent findings against the exact
  candidate, followed by the required authorized human acts.

Earlier observations remain in the adjacent implementation notes and original
runtime evidence store. This report preserves their limits and does not replace
source observations, infer participant approval, or resolve OW-WAR-0149.
