# Interactive SDK authoring

`war document draft --draft-dir <new-directory> --output <new-document.md>`
works outside a repository. Choose `title`, `id`, `kind` or `revision`, then enter
its value. `meta KEY` accepts one JSON value. `unit ID binding|background` selects
a unit and replaces its Markdown; include its heading and end with a single dot.
`drop ID`, `preview`, `save`, `cancel` and `help` are also available. Title changes
update the `outcome` heading only; other units remain unchanged.

Preview and save call the same SDK author operation as `war sdk`. Invalid source
is not published. `save` publishes a new Markdown file and never overwrites an
existing file or symlink. Output must remain outside the checkpoint directory.
Prompts and previews go to stderr; `--json` returns the standard report envelope
on stdout, with explicit `saved` and `qualified: false` fields. No model, database,
service, repository discovery, authorization or verification runs.

Each accepted edit line creates a new checkpoint. Cancel, end of input or process
termination leaves prior complete checkpoints. Resume with the same arguments
plus `--resume`. A file lock allows one cooperating writer and releases when its
process exits. Keep the checkpoint directory private to this session; unrelated
files or malformed snapshots refuse. This is not a sandbox against another
process that can rewrite its files, nor a guarantee against storage hardware loss.

Limits: 64 KiB per input line, 1 MiB per checkpoint, 16 MiB total checkpoint bytes,
4096 checkpoints and 8192 scanned directory entries. JSON uses the same duplicate,
depth and node checks as the SDK CLI. A rejected edit leaves its prior checkpoint.
Drafts can be incomplete; source validity and execution readiness stay separate.

`author.stdin` is a reproducible input transcript. `run.py --war <built-war>
--output <new-receipt.json>` runs it, compares saved bytes with the SDK's
noninteractive author response, then exercises canceled/resumed publication and
no-clobber refusal. Public Rust integration tests additionally cover interruption,
concurrent writers, false maturity, duplicate metadata, title preservation,
malformed state, symlinks and resource limits. These are input-pipe and process
observations; they do not measure human effort or real-user usability.
