# Keep the full integration gate usable within its declared CI budget

PR 136 hosted gate repeatedly reaches its 20-minute bound; prior exact local gate passed all 14 steps and 1275 checks in about 35 minutes on a busy host. Do not remove checks, fake dispositions, bypass required CI or blindly raise the cost bound. Work in this isolated checkout while the candidate gate remains live.

## Notes

- **2026-10-01 00:26 UTC, codex:** Observed real overview snapshot control failure: 1138 repeated tree scans across six distinct Git queries; caller index unchanged was not yet reached. Hosted web failure is now only project inventory HTTP 503; board checks passed. Warm full check consumed about 4.6 CPU seconds and returned identical bytes in two runs (~5.4 seconds), so disk cold-start timing alone is not a sufficient profile. Full local candidate first run had UNKNOWN setup failures from missing target/debug link, not attestation or corpus refusals; corrected layout retry remains live.
- **2026-10-01 00:38 UTC, codex:** Fixed one-shot overview scan repetition and duplicate corpus assessment via existing roadmap view_with. Live serve remains outside the one-shot memo. Full CI budget item remains open: current hosted gate was cancelled at its twenty-minute bound. First local attempt COULD NOT RUN three steps due missing target link; corrected retry is still active. No required checks or deadlines were removed.
- **2026-10-01 00:56 UTC, codex:** SHA profile experiment retains identical checker bytes across six balanced pairs, 76 compiler tests pass, real CLI digest-drift and missing-artifact controls refuse correctly. Local median checker improvement is about ten percent only; full hosted gate budget remains unestablished. Evidence saved under ticket/evidence.
