# M8: war host, the standalone half of Liminal hosting

## Notes

- **2026-10-07 01:50 UTC, claude:** Design: one implementation via crate::vfs (readers on the model path read through it; std::fs outside war host, an in-memory basis inside). Signature verdicts (ssh-keygen) and git history reads are the two outside observations: a hosted run takes them from the request (assumed, never authenticated; counted in the response) or fails closed and exits 2. Owner decision owed if signatures should be verified inside the boundary. M6 seam: host.rs m6_render_seam_kinds / m6_render_seam.
