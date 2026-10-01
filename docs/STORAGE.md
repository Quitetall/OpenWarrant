# Storage: how records are written, what a crash leaves, and what is kept

OW-WAR-0121. SAS §86 says file-native commands use temporary files, fsync,
atomic rename and prestate digest checks, and publish no partial generated
parent. §87.2 says controlled writes avoid symlink races. This document says
how `war` meets that for the records that carry authority, what is on disk
after a crash at each point, what `war check` reports then, and the rule for
records kept across schema versions.

## Which writes this covers

One module, `crates/openwarrant-cli/src/atomic.rs`, writes every record
listed here:

| record | written by |
| --- | --- |
| `authorization.toml`, `judgments.toml` | `authorize.rs` (`war sign`, `war authorize --response`) |
| `resolution.toml` | `resolution_cmd.rs` (`war sign`, `war resolve --response`) |
| `verifications/<OBL>.toml` | `verify.rs` (`war verify --response`) |
| gate receipts (`*.receipt.json`) and gate runs (`*.run.toml`) | `gate_cmd.rs` (`war evidence record`, `war gate --run`) |
| every generated view and corpus projection | `compile.rs` (`war compile`) |
| the response draft `war sign` signs, and the public-key file it hands `ssh-keygen` | `sign.rs` |

`journal.jsonl` is appended, not replaced; it has its own rule below. Other
writers in the tool are not covered by this document. Some of them have their
own atomic code (`authority_cmd/store.rs`, `projects.rs` and others); the rest
use a plain write. Tests, `init` scaffolding and one-off outputs are among
them. They move to this module only when they write a record.

## The write protocol

`atomic::write_if(path, bytes, prestate)`:

1. **Refuse a symlink.** If `path` is a symlink, the write is refused with
   `storage.symlink-target`. Nothing is written, and the link's target is not
   opened.
2. **Create the temp file.** The name is `.<name>.<pid>.<nonce>.war-tmp`, in
   the same directory as `path`. It is opened with `O_CREAT|O_EXCL`, so the
   module never opens a file that another process placed there, a symlink
   included.
3. **Write and fsync the temp file.**
4. **Look again.** The target is read again. The write is refused if the
   target became a symlink, or if its SHA-256 is not the prestate the caller
   read (`storage.prestate-moved`). A target that exists when the prestate
   said "absent" is also refused. In each case the temp file is removed and
   the other writer's bytes stay.
5. **Rename.** The temp file is renamed over `path`. Within one directory
   this is atomic (20-basis A-001).
6. **Fsync the directory**, so that the rename survives a power loss.

`atomic::write(path, bytes)` is `write_if` with the prestate read at the
start of the call. It catches a change made while the write is in progress.
The authority acts read their prestate earlier. `war sign` and
`war authorize --response` read `authorization.toml` and `judgments.toml`
before they read anything else. `war resolve --response` does the same for
`resolution.toml`. A record changed during the act, for example during the
ssh-agent dialog, is therefore refused rather than overwritten. `war compile`
uses the bytes it read to decide whether a view changed as that view's
prestate.

The window between step 4 and step 5 is not closed. A writer that does not use
this module and writes in that window is overwritten. The window lasts a few
microseconds. The window the prestate check does close is the time an act
spends between reading a record and writing the next one.

## Crash points

"Crash" means the process stops: it is killed, it panics, or the machine loses
power. The table assumes that fsync and rename behave as documented on the
filesystem (Linux ext4, btrfs and xfs are the ones the plants run on).

| the process stops … | on disk afterwards | `war check` says |
| --- | --- | --- |
| before step 2 | nothing changed | nothing new |
| during step 2 or 3 (temp file partly written) | the target is unchanged: its old bytes, or absent. A partial temp file is left | `storage.stray-temp` (warning) naming the temp file and the record it was meant to replace |
| after step 3, before step 5 (`OPENWARRANT_FAULT=after-temp`) | the target is unchanged. The whole temp file is left | `storage.stray-temp`, as above |
| a refusal at step 4 | the target holds the other writer's bytes. The temp file is removed | the act exits non-zero with `storage.prestate-moved` or `storage.symlink-target` |
| after step 5, before step 6 | after a process crash: the new record, whole. After a power loss: either the new record or the old one, whole, and possibly the temp file | nothing new, or `storage.stray-temp` |
| after step 6 | the new record, whole and durable | nothing new |

At no point does the target hold part of the old bytes and part of the new.
A `.war-tmp` file is never read as a record. `war check` names it so that a
person can decide what to do. The tool does not delete it, because the file
is the only sign that an act stopped.

`war compile` writes each view separately. A crash in the middle of a compile
leaves some views new and some old, and each of them is whole. The next
`war compile` finishes the job. `war check --generated` reports the views that
are still old as drift, which is the same finding as for any view that was not
recompiled. None of them is half written.

## The journal

`journal.jsonl` is append-only (§66). Each append writes the whole line,
newline included, in one `write`, and then fsyncs the file. An append that
returned is on disk.

An append cut short leaves a **torn tail**: a final line that has no newline
and does not parse. `war check` reports it as `journal.torn-tail` (error) and
gives:

- the line number and the byte offset where the unfinished line starts;
- the exact command that removes only the unfinished bytes,
  `truncate -s <offset> docs/warrants/<alias>/journal.jsonl`, run from the
  repository root. Every complete line before the offset is kept.

`war` does not run the truncation (20-basis U-001). Repairing a journal is a
person's act. Any command that must read the journal before it writes, which
includes every append, refuses with the same `journal.torn-tail` message until
the tail is removed.

`journal.torn-tail` is reported only for that shape. A line that does not
parse and does have a newline after it was written whole. It is
`journal.malformed`, as before, whether it is in the middle of the file or at
the end. `journal.rewritten` and `journal.malformed` for committed lines are
unchanged.

If the last line has no newline but does parse, it was written whole by hand
or by an older tool. It is read as an event, and the next append adds the
missing newline before its own line so that the two events do not run
together.

## Crashes between files

An act that writes more than one file can stop between them. Each file is
whole, but the set can disagree (20-basis R-001). This Warrant does not add a
transaction across files. That belongs to Knowledge Fabric or Liminal (§86),
not to a local coordinator.

`war sign <alias> --ssh-sign` for an authorization writes, in order: the
response draft, its signature, the rename to `.response.toml` (the `.sig`
follows), `authorization.toml`, the journal line, and then the attestation
and its journal line. These are the states a crash between two of those
steps leaves:

| stopped after … | `war check` says | what to do |
| --- | --- | --- |
| the draft's temp file, before its rename | `storage.stray-temp` naming the draft | remove the temp file. `war sign` again works |
| the rename of the response, before `authorization.toml` | nothing: the act is still pending | `war sign` again is refused with `sign.response-exists`, which names `war authorize <alias> --response <file>` to ingest the signed response |
| the rename of the response, before its `.sig` followed | nothing: the act is still pending | `war sign` again is refused with `sign.ssh-refused`, because the old `.draft.toml.sig` is still there and does not verify the new draft. Rename it to `<response>.sig`, then ingest as in the row above. If a record already cites a response with no `.sig`, `war check` reports `authority.unsigned` |
| `authorization.toml`, before its journal line | **nothing** | the journal lacks `authorization.recorded`. No rule compares the two |
| the journal line, before the attestation | **nothing** | the act stands without its attestation (`sign.rs` treats a missing attestation as a warning, `attest.not-emitted`). No command writes it afterwards |

For `war compile`, a crash between two views leaves the views written so far
new and the rest old. `war check --generated` reports each old one as
`generated.drift` or `corpus-status.drift`, and reports the temp file as
`storage.stray-temp`. `war compile` again finishes the job.

The two rows marked **nothing** are gaps. No finding detects them today. They
are recorded here and not closed, because closing them would mean a new rule
that compares records with the journal, which is outside this Warrant.
These states were observed on a scratch program on 2026-09-24. The first
three rows used the fault hook, and the third then moved the `.sig` back. The
last two were made by removing the line or file that the crash would not have
written.

## The fault hook

Debug builds read `OPENWARRANT_FAULT`:

- `after-temp` exits with status 86 after step 3 and before step 5;
- `pause-before-rename:<ms>` sleeps for that long at the same point.

`OPENWARRANT_FAULT_FILE=<name>` limits the hook to the target whose file name
is `<name>`. Without it, the first write of the process that goes through
this module is the one stopped. The hook is compiled out of release builds by
`cfg(debug_assertions)`, so the variable's name should not appear in a
release binary at all. `conformance/plants.d/53-storage.sh` uses the hook to
observe the crash points above. It also looks for the name in
`target/release/war` when that binary is newer than `atomic.rs`, and reports
`UNKNOWN` when it is not, because an older binary lacks the hook for the
wrong reason.

## Retained artifacts

A signed or resolved record is kept exactly as it was written. Specifically:

- **A signed or resolved record is never rewritten to move it to a new
  schema.** Its bytes are what a signature, an attestation, a
  `deliverables.toml` pin or a resolution covers. Rewriting it would break the
  thing that makes it evidence.
- **A reader accepts every schema version committed in this repository, or
  refuses it by name.** A reader never treats an unknown version as the
  version it knows. A new version adds a reader and keeps the old one.
- Migration tooling, and readers for versions that are not written yet,
  belong to a later Warrant. That work depends on OW-WAR-0032 (schema pack)
  and OW-WAR-0111 (preservation archives).

### Record schemas present at HEAD

These are the `schema` values carried by files tracked under `docs/` at commit
`244f538e`, with the number of files carrying each. A file is counted once
per value. For Markdown, only the front matter is read, because prose quotes
schemas that it does not carry. These values are what the rule above binds
today. The table was produced by this script, run from the repository root:

```python
import collections, re, subprocess
files = subprocess.run(["git", "ls-tree", "-r", "--name-only", "HEAD", "docs"], capture_output=True, text=True).stdout.split()
pat = re.compile(r'(?:^|[\s{,])"?schema"?\s*[:=]\s*"?([A-Za-z][\w./-]*/[\w.-]+)"?', re.M)
count = collections.Counter()
for f in files:
    if not f.endswith((".toml", ".json", ".jsonl", ".yaml", ".md")):
        continue
    text = subprocess.run(["git", "show", f"HEAD:{f}"], capture_output=True).stdout.decode("utf-8", "replace")
    if f.endswith(".md"):  # front matter only; prose quotes schemas it does not carry
        m = re.match(r"---\n(.*?)\n---\n", text, re.S)
        text = m.group(1) if m else ""
    for s in set(pat.findall(text)):
        count[s] += 1
for s, n in sorted(count.items()):
    print(f"{n:5d} {s}")
```

| schema | files |
| --- | ---: |
| `oh.war/amendment/v1` | 43 |
| `oh.war/atom/v1` | 568 |
| `oh.war/authorization-response/v1` | 164 |
| `oh.war/authorization/v1` | 163 |
| `oh.war/batch/v1` | 4 |
| `oh.war/bonsai-evidence/v1` | 7 |
| `oh.war/bonsai-scope/v1` | 20 |
| `oh.war/candidate-bundle/v1` | 1 |
| `oh.war/compile-request/1.0.0-rc.2` | 6 |
| `oh.war/completion-audit/v1` | 1 |
| `oh.war/context-package/1.0.0-rc.2` | 4 |
| `oh.war/context-packet/1.0.0-rc.2` | 2 |
| `oh.war/corpus-pending/v1` | 1 |
| `oh.war/corpus-status/v1` | 1 |
| `oh.war/corpus-timeline/v1` | 1 |
| `oh.war/correction-request/v1` | 4 |
| `oh.war/correction-response/v1` | 53 |
| `oh.war/correction/v1` | 51 |
| `oh.war/deliverables/v1` | 88 |
| `oh.war/dispatch-queue-record/v1` | 5 |
| `oh.war/document/1.0.0-rc.2` | 8 |
| `oh.war/document/1.0.0-rc.3` | 1 |
| `oh.war/drafter-run/v1` | 1 |
| `oh.war/execution-attempt/v1` | 2 |
| `oh.war/hotline-answer/v1` | 1 |
| `oh.war/implementation-evidence/v1` | 1 |
| `oh.war/implementation-gate-observation/v1` | 1 |
| `oh.war/implementation-observation/v1` | 5 |
| `oh.war/install-observation/v1` | 1 |
| `oh.war/interactive-observation/v1` | 1 |
| `oh.war/judgments/v1` | 59 |
| `oh.war/legacy-closure-observations/v1` | 1 |
| `oh.war/manifest/v1` | 136 |
| `oh.war/manual-observation-draft/v1` | 1 |
| `oh.war/milestones/v1` | 138 |
| `oh.war/native-phase1-comparison/v1` | 1 |
| `oh.war/overview/v1` | 1 |
| `oh.war/phase1-observation/v1` | 4 |
| `oh.war/preservation-archive/v1-draft.1` | 1 |
| `oh.war/preservation-result/v1-draft.1` | 15 |
| `oh.war/question/v1` | 7 |
| `oh.war/rationale/v1` | 88 |
| `oh.war/reconciliation-inventory/v1` | 1 |
| `oh.war/release-review-inventory/v1` | 1 |
| `oh.war/report/v1` | 35 |
| `oh.war/resolution-response/v1` | 29 |
| `oh.war/resolution/v1` | 29 |
| `oh.war/roadmap-acceptance-response/v1` | 2 |
| `oh.war/roadmap-phases/v1` | 1 |
| `oh.war/roadmap-revision/v1` | 2 |
| `oh.war/roadmap-view/v1` | 1 |
| `oh.war/roadmap/v1` | 2 |
| `oh.war/sas-acceptance-response/v1` | 3 |
| `oh.war/sas-normative/v1` | 1 |
| `oh.war/sas-revision/v1` | 5 |
| `oh.war/sdk-request/v1` | 1 |
| `oh.war/spec-source-set/1.0.0-rc.2` | 2 |
| `oh.war/telemetry-baseline/v1` | 2 |
| `oh.war/verification-inventory/v1` | 4 |
| `oh.war/verification-request/v1` | 4 |
| `oh.war/verification-request/v2` | 1 |
| `oh.war/verification-request/v3` | 1 |
| `oh.war/verification-result/v1` | 4 |
| `oh.war/verifier-decision/v1` | 1 |
| `oh.war/verifier-dispute/v1` | 1 |
| `oh.war/verifier-job/v1` | 4 |
| `oh.war/verifier-loop/v1` | 1 |
| `oh.war/verifier-loops/v1` | 1 |
| `oh.war/verifier-observation/v1` | 4 |
| `oh.war/viewer-work-report/v1` | 84 |
| `openwarrant-provider-workspace-isolation-observation/1` | 1 |
| `openwarrant.candidate-delivery-change/v1` | 1 |
| `openwarrant.friction/record/v1` | 1 |
| `openwarrant.rc2-build-roadmap.draft/v1` | 1 |
| `openwarrant.scale/budget-run/v1` | 1 |
| `openwarrant.scale/synth-counts/v1` | 1 |
| `openwarrant.sdk-roadmap.draft/v1` | 1 |

Some records carry no `schema` field. Their format is fixed by the code that
reads them, and the same rule applies to them:

- journal lines (`journal.jsonl`), versioned by `"v": 1` on each line;
- verification records (`verifications/<OBL>.toml`);
- gate runs (`*.run.toml`) and gate receipts (`*.receipt.json`);
- ssh signatures (`*.sig`), in the `ssh-keygen -Y sign` format;
- DSSE attestations (`*.dsse.json`), with `payloadType`
  `application/vnd.in-toto+json`.
