# Native human signing transport checkpoint

Unverified OW149 implementation; no actual human acceptance or independent verdict.

The experimental command is:

```sh
war sign native-resolve <alias> --response <relative-proposal> --native-store <protected-store> --ssh-sign
```

Without the final flag, it checks and displays the exact prepared response without
writing or asking a key. Actual signing renders an escaped review summary and
the original proposal digest before the key request, checks for review-time
changes, signs a private copy and runs fresh native acceptance checks. Existing
resolutions are refused before asking a key. Conflicting parent signing options
are refused by the parser or dispatch guard.

The normal protected human principal supplies the SSH key. Actual authenticated
presence is required where protected or repository policy requires it; ordinary
SSH confirmation does not establish hardware presence. Unsigned prototype work
keeps its existing workflow. This command does not set up a production account
or relax the native host's account, capability or custody requirements.

A complete immutable JSON packet preserves original signed bytes and signature
in `.openwarrant/state/native-signatures/<packet-sha256>.json`. Changed proposals
or delivered bytes during signing refuse recording and retain that observation.
The private snapshot is removed. Retention is not acceptance, and a failure after
recording begins requires checking the record before retry: record and journal
publication are not one transaction.

Real `ssh-keygen` used an in-memory SSH protocol agent holding public-seed test
material. No private key file or actual user's agent was used. Full Katana and
BLUT fixture round trips passed under separate operator/executor UIDs, including
preview with no writes, review-time changes with no key, signing-time source and
artifact changes with retained signatures, successful exact recording, and
replay refusal before any key request. Normal namespace controls, formatting and
all-target/all-feature CLI Clippy passed.

One test initially expected a JSON refusal from an invalid parent/subcommand
combination. The parser correctly refused before dispatch and returned a syntax
error; the test was corrected to require that specific refusal and no signing
output. The failed log and final reruns are retained and hashed.

These observations do not establish real human review, hardware touch, production
key custody, host-account qualification or release qualification. Native format
publication and actual workbench-to-retained-Dispatch mapping remain. OW95 has no
participants; no sessions or human signatures were invented for this project.
