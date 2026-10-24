# W129 implementation completion — actual historical account comparison

**Complete, unverified.** All four declared source files match protected CI
source `4a9f0b8088756aa933eff08079de35b18f83a768`, whose 14-step gate and
1,716 plants passed. The current implementation also passes 14 budget controls
plus positive corpus (15/0) and the compiler boundary test. The previously
missing actual before/after observation is now retained.

Clean source builds on Rust 1.97.1:

- before the budget move: `fdd346c54417069593d0f393dc63fe24d70424b8`;
- after the budget move: `4f1f6a80fd65c5a0c0e7b256a1fc494f43248a79`.

On the original corpus at the latter commit, both CLIs compile
OW-WAR-0047 / STAGE-002 to exactly the same TokenAccount: **4,533** estimated
tokens, **32,000** budget, method `oh.war/token-estimate/bytes-div-4/v1`.
The selected context manifests are identical too. This is an actual two-binary
public CLI run, not a formula recomputed by the current implementation.

A separate generated fixture produces 1,048/12,000 identically; both versions
refuse its 10-token stage with dispatch.over-budget, name the largest inputs and
write no refused packet. Replaying with the original source checkout unavailable
still passes; inherited Git routing is cleared and an unrelated Git canary's
file, HEAD and status stay unchanged. The exact generated fixture was archived
with its Git baseline and every file hash verified before scratch removal.

## Coverage of the full original scope

| Requirement | Retained observation |
| --- | --- |
| OBL-001 / compiler ownership | Source delegates the account to the compiler, which handles over-budget/unrecorded inputs. The boundary unit covers in-budget, exact budget, one token over and missing account. Actual original-corpus before/after packets preserve all three account fields. |
| OBL-002 / every actor-start path | Current controls observe perform and perform --all refusing only the over-budget stage before actor, packet or journal claim; the admitted actor runs. Service run refuses before a gate receipt, and the admitted service mints one. |
| OBL-003 / untrusted packet before bundle | Edited over-budget and absent accounts receive their named refusals without bundle output; the unedited packet bundles. Source judges tokens before reading the context. |
| Frozen surfaces | All four declared files match protected CI. The method/estimate and token fields retain their named legacy meaning; no record or digest domain changed in this audit. |

This completes the implementation report under the ordinary unverified completion
model. It does not change the legacy lifecycle state or establish the eight
formal requirements the earlier closure report left UNKNOWN. In particular,
later OW133 delivery ownership, admissible gate receipts, independent assessment,
risk authority and secure human resolution are still separate acts. Earlier
reports and independent findings remain intact.

## Reproduce the original corpus comparison

Use the retained binaries named by before-identity.json and after-identity.json,
or clean builds of the exact source commits. From any clone containing the
recorded corpus commit:

```sh
python3 compare-corpus-token-accounts.py /absolute/before-war /absolute/after-war \
  --repo /absolute/OpenWarrant \
  --scratch /mnt/4tb/tmp/ow129-new-replay \
  --out /mnt/4tb/artifacts/OpenWarrant/keep/ow129-new-replay
```

Both output paths must be new. Only the helper-created scratch is reset/cleaned
and removed. Git routing/config from the caller is isolated and hooks disabled.
The helper never signs, executes an actor or invokes a model. Its replay was
executed successfully; this is not an unrun example.

Binary hashes, source witnesses, raw results, the original fixture backup and
build logs remain under
`/mnt/4tb/artifacts/OpenWarrant/keep/ow129-history-20261024/`.
