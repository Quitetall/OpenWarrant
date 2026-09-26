# Plants pipe grep into grep -q under pipefail (SIGPIPE race)

Found by t-0f24: `grep PATTERN <<<"$OUT" | grep -q X` under set -o pipefail reads false when the first grep's matches exceed one 4 KiB pipe buffer and the second exits at its first match (SIGPIPE → 141). 69-standing is fixed (st_line_has: capture, then one grep). The same pattern remains in 58-invalidation (13), 53-storage (4), 69-idempotency (4), 59-webui-lan (3), 57-teams (2), 45-tickets, 56-acceptance-validity, 69-current (1 each). A lib.sh helper (line_has) and a check refusing the pattern in plants.d, as 99-fixed-tmp does for /tmp paths.

## Notes

- **2026-09-26 15:44 UTC, claude:** Correction to i-5b7e's count: 45 pipes converted on 44 lines (the per-file counts listed are right; the total '50 on 48' was wrong — 48 lines carried the pattern, 3 of them in the held 63-webui.sh, and 83-tokens:38 held two). Latent, preserved exactly: 59-webui-lan OBL-003's source grep passed vacuously when the second grep (ssh-add in *.rs) found nothing, since the group's status is that grep's; 67-pins-next and 69-current's HISTORY.md check pass when war itself exits nonzero. Worth a follow-up.
