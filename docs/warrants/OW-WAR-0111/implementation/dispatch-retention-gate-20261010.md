# Dispatch-retention fixture follow-up

Full gate [38069084165](https://github.com/Quitetall/OpenWarrant/actions/runs/38069084165) failed the planted battery: five plant files left automatically retained Dispatch records behind, and the runtime source-capture fixture attempted to create the existing Dispatch directory. The other thirteen gate steps passed. This is a failed gate, not qualification.

The battery now snapshots pre-existing tracked and untracked Dispatch paths before it starts and removes only new Dispatch outputs during restore. It preserves owner files and retains the final leak check for all other paths. A bounded scratch check observed both removal of a newly created Dispatch and preservation of pre-existing untracked bytes. Shell syntax passed.

The runtime capture fixture now asserts that the CLI retained the exact emitted Dispatch bytes rather than publishing them itself. The full positive/refusal fixture passed against the freshly built candidate on Rust 1.97.1. The full hosted gate must run again. No independent verdict, human act or assurance is claimed.
