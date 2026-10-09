# Governing ADR context omission: candidate correction observed

Observed against main 9791ea9ed132c29757e455366826ef913ebfa8e6 using the immutable tested binary /mnt/2tb/ow-main-9791e-war. The public CLI returns a verifier packet that omits a synthetic accepted ADR governing the exact Warrant UUID. With a pinned SAS it includes the SAS but still omits the ADR. The assertion fails in both retained logs. No independent disposition, live adoption, or authority signature was created.

Run in a disposable fixture with Python 3.11 or newer and an explicit war binary:

```sh
mkdir -p /tmp/ow-governing-repro-output
python3 docs/tickets/t-b9610/evidence/governing-adr-20261009/reproduce.py "$PWD" /absolute/path/to/war /tmp/ow-governing-repro-output --minimal
python3 docs/tickets/t-b9610/evidence/governing-adr-20261009/reproduce.py "$PWD" /absolute/path/to/war /tmp/ow-governing-repro-output --minimal --pin
```

The script creates and removes its own temporary repository. Its synthetic accepted ADR is fixture input, not a claim of authorization. `sas propose` creates only a draft. It does not run a model or ingest a verifier response. The script exits unsuccessfully until the required governing ADR is delivered. This evidence records a bug; it does not complete ticket i-37a7.

## Candidate observations

The original failed logs above remain unchanged. The candidate now carries exact applicable accepted ADRs for UUID and retained alias relations, with or without a SAS pin, in whole and split packets. Rehashed omissions, substitutions, duplicates, false kinds and extra instruction-bearing fields are refused without verdict or journal writes. The retained extra-instruction failure shows why checking only source hashes was insufficient. Malformed applicability, ambiguous current identity and unsafe/nonregular sources remain UNKNOWN. Native namespace enumeration retains ignored ADRs instead of allowing Git's source inventory to remove them.

All 41 public CLI regressions, scoped all-target Clippy and formatting pass on Rust 1.97.1. The original minimal and pinned probes pass; the malformed-relation probe now observes UNKNOWN. The extended required-sources conformance control passes using the immutable candidate executable identified in manifest.json. The actual OW-WAR-0007 request now names the existing accepted ADR0003 and its pinned SAS; this is a read-only request, not a verifier response or current assurance. Full gates are pending.

The proposed ADR0030 describes this candidate selection. No signature, disposition, v1 canonicalization/domain change or final context/custody qualification was created. This does not infer transitive dependencies from prose or finish ticket i-37a7.
