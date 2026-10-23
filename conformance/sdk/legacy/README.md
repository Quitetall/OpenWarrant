# Legacy preservation fixtures

`resolved.json` retains the exact supplied files under `docs/warrants/OW-WAR-0002/`
from commit `faaf3a4aa70029600ecfed58aab53177cefa8c10`, excluding the later
implementation report directory. Paths retain their original structure. Hashes
were independently computed with Python hashlib. The actual correction DSSE,
authorization, resolution and original generated JSON remain byte-identical.

This is a bounded historical inventory, not a complete historical artifact backup
or proof of signature authenticity. Externally referenced delivered files and
external identities are not captured. Missing or unsupported meaning remains explicit.

## Reproduce current observations

```sh
cargo test --locked -p openwarrant-core --test document_legacy
cargo run --locked -p openwarrant-core --example sdk_probe -- --scope 84 --fixtures conformance/sdk
python3 conformance/sdk/legacy/check_observations.py \
  --war /absolute/path/to/war \
  --fixtures conformance/sdk/legacy \
  --repo /absolute/path/to/OpenWarrant \
  --historical-revision faaf3a4aa70029600ecfed58aab53177cefa8c10 \
  --record-dir /absolute/path/to/new-observation-directory
```

The observer requires that exact commit locally. It compares all 25 retained
files with Git, then supplies Git's actual bytes to the public SDK CLI. It keeps
their original paths and checks signature-byte tampering and missing-file refusal.
Fresh successor cases preserve Unicode, CRLF, binary data and an archived original
signature. Keeping that signature does not transfer its authority to the successor.
Duplicate, unchanged, missing and self-successor paths refuse.

The new output directory retains exact requests, responses, exit codes and Git
source identities. Only a fully passing observation publishes its summary. Failed
runs remain failed. The observer changes no historical file and authenticates no
signature. It checks the named inventory, not every historical external artifact.
