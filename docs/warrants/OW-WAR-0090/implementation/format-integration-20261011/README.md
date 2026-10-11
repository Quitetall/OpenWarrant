# Combined Format 1.0 integration

Unverified performer observation of source
`7108e15ca577cc1b82edb81471a0bb27b444e4fb`. It contains the original PR235
and PR236 heads, including their history, plus the release-inventory primitive.
The other agent's branch and checkout were not changed. Generated progress
conflicts were resolved by recompiling the combined authored sources; both
the new context guidance and the corrected additive-installer pointer remain.

[Observation](observation.json) identifies retained original logs and hashes.
Rust 1.97.1 build and format checks pass. The committed CLI identifies source
`7108e15ca577`, without a dirty marker. Its Format corpus passes 105 cases.
The Python reader passes 103 and skips its two unimplemented context cases.
The schema runner passes 264 checks. Record checks report 2,029 PASS,
630 WARN, zero ERROR/UNKNOWN. These are bounded performer observations,
not independent dispositions, authenticated build provenance or phase exits.

The initial committed-source corpus observation retained an empty log and did
not retain its exit status. It is UNKNOWN. A subsequent run with explicit
terminal-status capture passes all 105 cases. Earlier prospective-tree corpus
and schema runs also pass; their original logs remain separately retained.

The whole staged diff whitespace check exits 2 on intentionally byte-sensitive
conformance inputs. For example:

```text
conformance/format/013-crlf-same-content-digest/input/docs/records/product/password-reset.md:1: trailing whitespace.
```

CRLF, trailing blanks and trailing fence spaces are the inputs those cases test.
They were preserved. The check excluding the conformance input directories
passes. The complete warning output is retained, not relabeled as a pass.

Format 1.0 and OW-ADR-0032 remain proposed. The accepted legacy SAS and signed
records are not superseded by this integration. This does not accept the new
standard, run a model, provide real-user sessions or grant release permission.
The required full gate for the final combined candidate and all remaining
provider/operator/independent/human qualification requirements remain open.

After main merged PR235 at `a8304460`, the integration branch was refreshed
through `7782810f`. Explicit terminal-status capture reports compile and record
checks both exit zero, with 2,029 PASS, 630 WARN and zero ERROR/UNKNOWN. The
preceding final-check attempt retained no JSON and remains UNKNOWN; its empty
file is preserved. These observations do not establish the required hosted gate.

Hosted run `38174561914` failed on candidate `970dc7ee`: the exact corpus-payload equality test rejected stale generated gate explanations. Read-only local comparison found 99 changed explanations: 97 source-path counts moved from 2,379 to 2,385 and two from 2,543 to 2,549, after six source files entered the tracked set. No other payload fields differed. The original hosted log and run metadata are retained here; their captured output does not establish every later gate step. The fix stages authored inputs before recompiling and keeps the test’s exact comparison, with a bounded failure message. Required CI for the corrected candidate remains pending. OW129’s separate local compiler test timed out during compilation (exit 124); it did not execute and remains UNKNOWN.

The corrected worktree’s cache-disabled public `status --json` payload equals the regenerated projection exactly. The focused `json_envelope::the_corpus_projection_is_the_result_payload_verbatim` test passes (1 passed, 0 failed, 268 filtered out; Rust 1.97.1, default features). Its original command log and exit status are retained. This is the corrected local equality check, not a full gate result or an independent assurance verdict. All authored evidence and ticket files must be staged before the final compile so their path counts enter the committed projection.
