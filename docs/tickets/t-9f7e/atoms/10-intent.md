# Verification bundles too large for the blind verifier

OW-WAR-0112's bundle is 241,152 tokens; the blind verifier (tools/verifier/claude-verifier.sh, 900 s timeout) timed out on it (2026-09-26). Large Warrants cannot be verified at all. Options: per-obligation bundles carrying only what that obligation's evidence and scope name (its deliverables, its plants, its gate runs); bounded excerpts with digests of the whole; or a verifier that walks obligations one call at a time. Keep the blindness guarantees (no performer narrative, no tools).
