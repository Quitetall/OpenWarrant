# 59-webui-lan: the bound-only check reads every listener on the machine, so a parallel battery holding the same port on 127.0.0.1 fails it

## Notes

- **2026-09-26 21:07 UTC, claude:** Seen in war prepare --all --jobs 3: OW-WAR-0136 STAGE-004, 1211/1 — FAIL bound only to the given address 127.0.0.2:45601 127.0.0.1:45601. Fix: read only the server pid's listeners (ss -p). Refusal kept: own loopback, own wildcard, wrong pid each fail on synthetic ss lines; plant alone 41/0.
