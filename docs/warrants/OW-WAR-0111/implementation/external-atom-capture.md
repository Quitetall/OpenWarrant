# Explicit shared atom preservation

Archive export resolves manifest atom paths relative to the Warrant directory,
including leading parent segments that remain inside the repository. It preserves
the original manifest reference and stores bytes under their normalized repository
path. Reads reject symlinks. Interior dot segments, absolute references and paths
escaping the repository are refused before the legacy loader reads the source.

Inspection reconstructs IR using the original reference and checks that each basis
record path matches that reference. An equal-byte alternate record cannot silently
replace the declared source. Sources may be removed after export.

Validation: eleven preservation tests passed; all-target CLI Clippy with warnings
denied passed (Rust 1.97.1). A real OW-WAR-0003 export retained exact bytes of its
shared OW-ADR-0001 atom and passed archive inspection. The compressed archive and
reports are retained beside this note.

This captures explicitly referenced current atoms. It does not establish complete
ADR history, resolve authority-bound URIs, or grant acceptance/export permission.
The ADR coverage category remains unavailable until its full inventory and retained
revision obligations are established. OW-WAR-0111 remains incomplete.
