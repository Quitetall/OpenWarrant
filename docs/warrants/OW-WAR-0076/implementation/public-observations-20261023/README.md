# W76 public author/edit observations — 2026-10-23

These are performer observations. They do not establish independent review,
secure human acceptance, qualification or resolution.

`wire.jsonl` preserves the exact request, stdout, stderr and exit status of all
47 actual public SDK calls. The ten `case-<content-sha256>.json` files decode
those observations against unchanged author fixture inputs and expected outputs.
`summary.json` indexes the cases and binds the runtime and original wire bytes.
`multi-unit-controls.json` contains extra CRLF/wrapped-footer operations: change
one binding unit, preserve the other unit and metadata exactly, explicitly change
revision and identity, and refuse an absent unit without partial output.
`controls.json` records fresh unlisted content and refusal to overwrite an
existing output file. The actual random text and temporary output path are kept;
these are observed results, not a claim of byte-identical future replay.

The public runtime is the sealed PR326 CLI, not a binary built in this checkout.
`runtime-identity.json` describes its actual source state; the caller/source
comparison is in `sdk-caller-source-equality.json`. The fresh native probe build
is separately described by `probe-process.json`. The seven actual Core tests
are in `core-author.stdout`, with command and source hashes in
`core-author-result.json`. They include explicit RC.2 editing, resource refusal,
extension preservation, injection refusals and no partial result on failure.

The source implementation was reused from its original Git history, not authored
again in this evidence run. Original gate and review reports remain historical.
These new observations add no independent verdict or human act.
