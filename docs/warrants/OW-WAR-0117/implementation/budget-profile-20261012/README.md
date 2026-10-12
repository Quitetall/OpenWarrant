# Review packet budget diagnostics

The OW-WAR-0063 packet generated from source 38c7dbed has a maximum
407450-token estimate against the unchanged 48000-token budget. Its largest
fixed fields are the request (1072971 canonical JSON bytes), required sources
(540814 bytes) and atoms (7997 bytes). The request includes 7485 reviewed-subject
gate input bindings. Required sources include the governing SAS (149780 bytes)
and the exact gate stdout (382075 bytes). These measurements do not prove that
any binding can safely be removed.

The CLI now lists the three largest fixed fields on over-budget warnings and
refusals. It prints names and byte counts, not source contents. The actual
OW63 CLI output confirmed those values. The packet, estimate, limit, schema,
reviewed-subject digest and admission decision remain unchanged.

The registered dispatch plant passed two checks with zero failures: the clean
corpus and the paired control proving that over-budget dispatch makes no adapter
or verdict write, while an in-budget packet reaches the adapter. Offline CLI
build and Clippy with warnings denied passed on Rust 1.97.1. Formatting passed.
No model call or paid call was made.

This completes only the diagnostic ticket item. The actual oversized review
packet remains over budget; independent review and parent Warrant qualification
remain open. Bytes are not model tokenizer counts or spend accounting.
