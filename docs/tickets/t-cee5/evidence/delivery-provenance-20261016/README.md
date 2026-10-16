# Delivery provenance for known new work

The CLI can now state an observed producing attempt and replace input digests explicitly when recording changed bytes. It can also state that a production had no inputs. Existing defaults and already-current records remain unchanged. Unknown attempts must not be invented. This records claims and byte identity; it does not grant ownership, authorize, independently verify or resolve.

The public CLI first refused the producing-attempt option. The first implementation exposed that the old text writer deliberately preserved input lists. The replacement now uses the TOML parser's byte spans, preserving text outside the replaced value, and verifies the full parsed result before the existing guarded atomic write. No new canonicalizer, dependency, digest domain or schema was introduced.

Rust 1.97.1: three public controls pass for changed-byte metadata, multiline/Unicode/comment preservation and explicit zero inputs. Each includes unknown-id refusal without writes, a no-write dry run, byte-preserving replay and unchanged authority records. Five legacy delivery unit controls and all-target Clippy pass. One test-fixture string-concatenation compile error and prior red controls remain in the logs. Aggregate and full CI remain required.

A separate two-file proposal matches current OW134/D001 and D005 bytes exactly to our retained ab29cfe4 and 50a16044 modification commits and their input bases. The original delivery manifest is preserved under the evidence root in its original relative path. Both delivery dry runs pass. Actual recording is not claimed at this checkpoint; recorded attempts remain explicitly unrecorded and complete production history is not inferred.

Raw proof: `/mnt/4tb/artifacts/OpenWarrant/keep/conformance-continuation-20261014/delivery-provenance-20261016/`.
