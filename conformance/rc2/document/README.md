# RC.2 document fixtures: first parser slice

These inputs prepare the `parse_document` and `validate_document` public seams
planned by OW-WAR-0075. They are not evidence that either primitive exists or
passes. No Warrant authorization, SAS acceptance, or independent verification
is asserted here.

`cases.json` is a test inventory, not an OpenWarrant wire format. Its fixed
expectations come from RC.2 format contract F1/F2. Future tests must read these
files through the production public boundary; selecting results by fixture name
is not an implementation.

| Input | Required observation |
| --- | --- |
| `minimal.md` | Valid minimal draft; exact identity, title, three binding units and byte ranges; no authority inferred |
| `duplicate-title.md` | Duplicate TOML key produces `source-invalid`, with location, and no partial document |
| `missing-header-close.md` | Missing header delimiter produces `source-invalid`, with location, and no partial document |

The positive file preserves the candidate's authored minimal example byte for
byte. The two negative files each change only the stated framing/header property.
Offsets are zero-based UTF-8 byte ranges with an exclusive end. Tests must compare
returned slices with these original bytes, not normalize Markdown or line endings.
Locations describe what the diagnostic must identify; they do not prescribe a
Rust error type or a display-message spelling.

This prepares T01 and the two mutations in T02. T03–T10 remain unprepared here.
The remaining Phase 1 features and T11–T56 are not covered. A fixture integrity
audit proves only that the inputs match this inventory. Production conformance
requires observed results through the new parser, validator and `rc2_probe`.
