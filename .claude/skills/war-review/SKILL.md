---
name: war-review
description: "War review: inspect exact changes against repository standards and Warrant obligations while preserving independent verification."
---

# war-review

Read [shared workflow](../openwarrant/SKILL.md) and
[verification](../openwarrant/references/verification.md).
Method: Matt Pocock's two-axis review; [provenance](../openwarrant/ADAPTATIONS.md).

1. Pin base and candidate revisions. Inspect actual diff, contract and required
   evidence. Include working-tree changes explicitly if those are the review subject.
2. Standards axis: check applicable repository rules, correctness and maintainability.
   Report actionable findings with file/line and reproduced evidence; tooling output
   needs interpretation, not repetition. Filter false positives before requesting fixes.
3. Obligations axis: an independent verifier gets exact approved constraints,
   candidate code, protected fixtures and evidence in a separate context/workspace.
   It must be able to inspect code and rerun checks. A bundle-only review without
   executable access cannot claim observations it never made.
4. Keep each axis separate. State PASS/FAIL/UNKNOWN and evidence for each bounded
   obligation. If independent execution is unavailable, report that gap and continue
   any permitted ordinary review; performer self-review is never independent proof.
5. Return findings, exact subject and qualification gaps. Store a received verifier
   response through the supported tool without altering actor identity or verdicts.
   Human secure acceptance remains separate from an agent's review report.
