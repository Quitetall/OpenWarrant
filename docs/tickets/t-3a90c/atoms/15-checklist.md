# Checklist

- [x] Reproduce duplicate gate execution through the public prepare command (i-79c5) — done by agent://codex, 2026-10-14: Public war evidence prepare fixture selects IX-WAR-0001 and IX-WAR-0003 and counts actual gate child starts outside source inputs. Baseline 379acb09 binary runs the shared gate twice; red-v2.stderr preserves expected assertion. Synthetic fixture only, no production authority or verification.
- [ ] Record one pre-bound shared observation and retain per-Warrant attachments without changing receipt format (i-1398)
- [ ] Add refusal and replay controls, run scoped checks, and submit the feature for CI (i-7486)
