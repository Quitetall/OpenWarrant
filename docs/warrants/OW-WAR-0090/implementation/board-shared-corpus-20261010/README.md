# Share one corpus per read-only board request

The full reference-web CI run on candidate 41f61879 failed four board controls: the existing SDK subprocess exceeded its ten-second limit and returned HTTP 503. `board::build` loaded three independent corpora for frontier, review rows and status despite existing entry points that accept one corpus. It now uses one corpus for all three sections. The wire response, admission, signing queue and timeout remain unchanged; no result is cached across requests.

On the same frozen checkout, old/new binaries returned exactly identical 1,679,220-byte JSON responses, SHA-256 ed0083c5f04fac2c19c4fc462fe6912ac911e971dd8462685cdef631f5c53715. One measured pair took 7.84 seconds before and 5.35 after. This is a bounded observation on this host, not a performance guarantee on every machine.

Existing public board controls passed: each section agrees with its corresponding CLI view, sources stay unchanged, damaged questions/journals refuse and the HTML remains read-only. All four inherited HTTP board controls passed. The first full local web run had six server-startup-wait failures during concurrent builds; all were the same four-second setup assertion, and no board control failed. Without concurrent builds, the unchanged full web suite passed 215 tests in 180.072 seconds. Initial failures are retained.

Rust 1.97.1; formatting and whitespace passed. Strict all-target/all-feature CLI Clippy passed in 20.94 seconds; combined candidate generated integrity is recorded separately when terminal. Full current-main hosted gates still apply. OW90 remains blocked on the rest of Phase 3, including real-user qualification; this report grants no authority or assurance.
