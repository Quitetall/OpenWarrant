# W146 rendered UTF-8 tail fix, unverified

Gate streams are now decoded before their tails are selected. Selection starts
at a UTF-8 character boundary and bounds encoded text bytes. A character that
cannot fit is omitted rather than converted into a replacement character caused
by a raw-byte split. Invalid source bytes still use the existing lossy rendering,
but that rendering also fits the limit. Full-file hashes and raw byte counts
continue to describe the original file.

The public CLI regression first failed with U+FFFD at a one-byte limit. It now
returns empty text, captured true, truncated true, and the original two-byte
file's SHA-256. Missing captures remain captured false with no text or hash.

Two CLI controls pass. Their matrix covers both stdout and stderr, ASCII,
multibyte boundaries, invalid bytes, exact-fit characters, empty streams and
missing files. Each unchanged input is emitted twice: packet path and bytes
match. No receipt digest was supplied by this synthetic fixture; the conditional
mismatch field stays absent. These are formatter observations, not claims that
a production gate ran.

One intermediate matrix failed because it treated configuration zero as an
explicit zero-byte limit. `VerifyPolicy::max_excerpt_bytes` defines zero as the
65536-byte default. The test now preserves that documented behavior; production
configuration semantics were not changed. The failed attempt remains in the logs.

All 11 existing bundle tests and warnings-denied all-target CLI Clippy pass on
Rust 1.97.1. The prior ASCII registered-control evidence remains unchanged;
exact-head full CI is still required. No record schema, receipt, authority,
independent disposition, delivery pin or legacy resolution was changed.
