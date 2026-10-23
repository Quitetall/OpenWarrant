# FOOTER-08 parser readback evidence

The scope-75 parser observations did not show the author/edit readback
required by FOOTER-08. These additional actual observations fill that
coverage gap. Earlier review and transport history is retained separately;
it is not evidence that this subject meets an obligation.

Additional actual observations now exercise the existing approved author
fixtures through the public SDK. The title/CRLF roundtrip produces exactly
`title-crlf.md`, updates visible and metadata titles together, and preserves
the untouched unit tail. The revision roundtrip produces exactly
`revision.md` and preserves the whole unedited unit. Both results are
parsed and validated again. The conflict fixture returns its actual named
refusal; a seeded output file remains byte-identical on failure.

The existing core probe also ran with `--scope 76`: all 22 document cases
and all 10 author/edit cases passed, including both FOOTER08 cases. This
uses the authoring component as a readback dependency; it does not claim
that W75 implemented W76's authoring operations. No protected fixture or
expected result was changed.

These files are captured execution data and deterministic summaries. They
do not provide a human signature, independent acceptance or an assurance
mark. Raw SDK requests, responses, stderr and the direct probe stdout are
retained under the artifact path in `execution.json`.
