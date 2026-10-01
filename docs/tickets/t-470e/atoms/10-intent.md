# 59-webui-lan reads the act log before the remedy is written, under load

On OW-WAR-0120's evidence run (parallel batteries, loaded machine) 59-webui-lan.sh failed 'OBL-003 an auto remedy with a fresh nonce': the POST returned 202 but the act log had 1 line where the check expected the remedy's line too. 202 means accepted; the remedy runs after. The plant must wait for the act (poll the act log / its completion with a bound) rather than read immediately — or the server must record the act before answering. Decide which is the contract and make the plant hold it deterministically.

## Notes

- **2026-09-26 01:34 UTC, claude:** Correction to the description: the act log DID have its one line and the POST was 202. The failing condition is the next one — 'war check' no longer reports relations.child-listed — tested after a fixed 'sleep 1'. The remedy is a full 'war compile' (~20 s under load, seconds when idle), so the plant must wait for the remedy to finish (poll war check or the act's completion line, with a bound), not sleep 1.
