# Original OpenWarrant documentation archive

Captured 2026-09-14. Original bytes and relative directory structure are retained.
This is historical material, excluded from default context and repository search.
Read a source only when history, migration or an exact required reference needs it.

| Location | Contents |
| --- | --- |
| [original/](original/) | 1,850 readable files from pre-SDK baseline commit `16f0559db9a267c2123b334224c1b1ab5e645b03`, at their original repository-relative paths. |
| [Original SAS](original/docs/sas/WAR_Software_Architecture_Specification.md) | The baseline specification; its authority is determined by the original records, not its archive location. |
| [Original Warrants](original/docs/warrants/) | Baseline Warrant directory structure, including atoms, generated views and evidence where present. |
| [Original context](original/CONTEXT.md) and [skills guide](original/docs/SKILLS.md) | Earlier terminology and workflow reference. |
| [snapshots/](snapshots/) | 16 separate worktree captures, including uncommitted variants and ignored documentation receipts. Each tar archive retains repository-relative paths; no flattening or overwrite of colliding revisions. |
| [history.bundle](history.bundle) | Self-contained Git history reachable from the captured local refs, including committed files outside the documentation selection. |
| [manifest.json](manifest.json) | Source worktrees, branches, HEADs, capture scope, paths, kinds, hashes and preserved symlink targets. |
| [retained-copies/](retained-copies/) | Two former `docs/agents/legacy-*` references, relocated without changing their bytes or relative paths. |

The `ow-war-0074-source-set` snapshot contains signed authorization revisions,
responses and evidence absent from the SDK amendment checkout. Captures are kept
separate; neither branch recency nor this archive chooses authority between them.

## Check and restore

Run from the repository root:

```sh
python3 archive/legacy-20260914/verify.py --restore-check
```

The verifier checks original bytes, tar member coverage, link targets, modes,
relocated copies and bundle integrity. The restore option imports the bundle into
an empty temporary Git repository, then compares all captured refs and every
baseline document with the retained copy. It leaves source repositories unchanged.
Use a fresh output directory when restoring a worktree snapshot; inspect symlink
targets before extraction. The archive is never an execution source for old skills.

Manifest SHA-256:
`b9f44e81bc1ebbf644985affae3f73ca57afb4d23dab937ca97072326e562465`.
Hashes establish retained byte identity, not human acceptance or signature
authenticity. Git history records publication of this archive. It is not a separate-device backup.

## Scope and compatibility

Current worktree selection covers `docs/**`, root Markdown, `.claude/skills/**`
and `.agents/skills/**`. Ignored documentation receipts are included; Python
bytecode/cache files are excluded. Symlinks are retained without following them.
External evidence, remote-only refs, unreachable/reflog-only objects and local
configuration outside the selection are not claimed captured. File ownership,
ACLs and extended attributes are outside this capture. Reads are consistent per
file; this was not one atomic snapshot of all worktrees.

Live legacy SAS, Warrant and authority paths remain where current tools need
them. These compatibility copies must not be removed before validated migration
supports relocation. Archived optional context stays out of default selection;
an active required rule remains available until its permitted replacement.
Archive preservation does not complete SDK migration, resolve old Warrants or
turn historical signatures into new assurance marks.
