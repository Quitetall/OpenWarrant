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
