# W146 UTF-8 tail bound defect

A public `war verify --bundle` reproduction over an isolated synthetic corpus
observed a one-byte capture limit producing three bytes of text. The captured
file held UTF-8 `é` (two bytes). Slicing its last raw byte before lossy decoding
produced U+FFFD, which encodes as three bytes. The record correctly retained the
whole-file digest and marked truncation, but violated the requested text bound.

[The observation](observation.json) records the exact input/output counts,
original source archive digest and executable hash. The command exited zero;
its output is retained beside this report. The complete generated bundle and
fixture configuration are retained under
`/mnt/4tb/artifacts/OpenWarrant/keep/ow146-stream-20261024/`.
The executable's source identity is in the sibling W68 evidence root under
`context-final-tool/identity.json`. Its `bundle.rs` bytes match protected CI's
`5a5f7ce6f3b0f6758c7971f6f4ef34ea42011508` source.

The fixture reused a verified archived scratch program. Only its excerpt limit
and captured output file were changed. No real authority or acceptance was
created. The scratch copy was removed after capture. This is a defect
reproduction, not independent verification or a passing acceptance result.

Next: bound the rendered UTF-8 tail, preserve the full raw digest, distinguish
absent capture from present-but-omitted text, and add a public regression for
multibyte boundaries and invalid input. Keep ASCII and zero-limit behavior.
