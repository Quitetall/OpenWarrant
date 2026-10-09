# Governing ADR context omission: reproduced, not repaired

Observed against main 9791ea9ed132c29757e455366826ef913ebfa8e6 using the immutable tested binary /mnt/2tb/ow-main-9791e-war. The public CLI returns a verifier packet that omits a synthetic accepted ADR governing the exact Warrant UUID. With a pinned SAS it includes the SAS but still omits the ADR. The assertion fails in both retained logs. No independent disposition, live adoption, or authority signature was created.

Run in a disposable fixture with Python 3.11 or newer and an explicit war binary:

```sh
mkdir -p /tmp/ow-governing-repro-output
python3 docs/tickets/t-b9610/evidence/governing-adr-20261009/reproduce.py "$PWD" /absolute/path/to/war /tmp/ow-governing-repro-output --minimal
python3 docs/tickets/t-b9610/evidence/governing-adr-20261009/reproduce.py "$PWD" /absolute/path/to/war /tmp/ow-governing-repro-output --minimal --pin
```

The script creates and removes its own temporary repository. Its synthetic accepted ADR is fixture input, not a claim of authorization. `sas propose` creates only a draft. It does not run a model or ingest a verifier response. The script exits unsuccessfully until the required governing ADR is delivered. This evidence records a bug; it does not complete ticket i-37a7.
