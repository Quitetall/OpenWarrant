# OW-WAR-0088 implementation

`war document draft --draft-dir DIR --output FILE [--resume]` adds offline field
selection, Markdown unit editing, preview and publication through the existing SDK
author operation. It works outside a repository. `--json` emits the existing report
envelope; prompts and preview use stderr. Saved documents remain drafts, without
authorization, verification or an assurance mark.

Ten public authoring tests and eleven existing SDK CLI tests pass. They compare
interactive output with actual noninteractive SDK bytes, exercise cancel/EOF and
process termination, preserve prior checkpoints, refuse concurrent writers and
existing output/symlinks, reject duplicate metadata/checkpoint keys and false
maturity, and enforce history/input/aggregate JSON limits. A fixed transcript and
a real terminal session also saved and previewed the intended document. The
terminal template is a demonstration draft, not an approved implementation plan.

Independent reviews found and verified repairs for wrong-unit title editing,
duplicate-key collapse, late history limits and a checkpoint that exceeded resume
depth limits. The complete serialized checkpoint is now checked before publication.
Both independent rechecks pass. These observations do not measure human usability
or establish complete Phase 2 acceptance.

Checkpoints use new files and a cooperating-writer lock; output uses atomic
no-clobber publication. This is not a filesystem sandbox or a storage durability
guarantee. No metadata source, signature, legacy delivery pin or provider interface
is modified by the authoring command.

Repository gate pending. See `conformance/integration/interactive/README.md` for
commands, input transcript, bounds and recovery behavior.

## Completion

The full repository gate passed all 14 steps and 308/308 controls on `fa0ed49`.
OW88 implementation is complete and unverified. Independent specification and
standards review passed; the latter independently reproduced the aggregate-depth
refusal and successful recovery. LAMU review_commit used the local/free model and
ended with PASS. Its temporary-file and lock concerns were retracted after checking
cleanup and exclusive-lock behavior. No additional code change was required.

Next: OW89 candidate packaging rehearsal. Full phase exits, native macOS acceptance,
actual-user workflow observations and owner release permission remain separate.
