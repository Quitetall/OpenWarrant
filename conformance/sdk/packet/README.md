# Packet and package integrity observations

The seven literals in `cases.json` exercise canonical F7 decoding and named
refusals. Run them through `sdk_probe --scope 81 --fixtures conformance/sdk`.

`check_observations.py --war <SDK-CLI> --fixtures conformance/sdk/packet
--transcript <external-probe-stdout> --record-dir <new-directory>` records the
literal cases through the public SDK CLI, then consumes fresh package results
from the external provider probe. The probe caller and execution identity are
retained in W81's evidence. It is an additional private test caller, not an
OpenWarrant dependency or a provider implementation change.

The deliberate counterexample is rehashed and structurally intact but omits
required context. SDK integrity accepts it with semantic coverage unestablished;
provider reconstruction refuses it. This distinction is intentional. Neither
integrity nor provider context coverage grants execution permission, independent
code verification or human acceptance.
