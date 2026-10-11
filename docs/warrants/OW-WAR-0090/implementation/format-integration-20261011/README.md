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
