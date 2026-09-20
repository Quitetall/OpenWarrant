// SPDX-License-Identifier: Apache-2.0
//! The `war` binary.
//!
//! Every command lives in the `openwarrant_cli` library; this target is the
//! argv adapter, so the CLI, an integration test and an embedding caller all
//! run the same code instead of the first one reaching it through a process.
//!
//! The two tests below stay HERE rather than moving to the library with
//! everything else. `async_lives_only_in_the_mcp_module` excludes itself from
//! its own walk by `file!()`, and `file!()` in a library is `lib.rs` — which
//! every crate in this workspace also has, so the exclusion would silently
//! spread to `openwarrant-core`, `openwarrant-compiler` and
//! `openwarrant-agent`. The test would keep passing while policing three
//! fewer crates. It also contains the needle literals it searches for, which
//! is the other reason it must live inside the file it skips.

#![forbid(unsafe_code)]

fn main() -> std::process::ExitCode {
    openwarrant_cli::entrypoint()
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    /// §76.4 says EVERY command should support `--json`. This is the ratchet:
    /// the subcommands that still print only for humans are listed here, by
    /// name, and a new subcommand cannot ship without either supporting the
    /// envelope or being added to this list on purpose. The list shrinks; it
    /// does not grow silently.
    /// OW-ADR-0014: async is admitted for the MCP transport only. Every source
    /// file of the workspace outside `openwarrant-cli/src/mcp/` is read here;
    /// the first `tokio`, `rmcp` or `async fn` outside that directory fails.
    #[test]
    fn async_lives_only_in_the_mcp_module() {
        fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).expect("readable dir").flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, out);
                } else if path.extension().is_some_and(|e| e == "rs") {
                    out.push(path);
                }
            }
        }
        let crates = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
        let mut files = Vec::new();
        walk(&crates, &mut files);
        assert!(files.len() > 40, "walked too few files: {}", files.len());
        let this = std::path::Path::new(file!())
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("main.rs");
        let mut offenders = Vec::new();
        for path in files {
            let rel = path
                .strip_prefix(&crates)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if rel.starts_with("openwarrant-cli/src/mcp/")
                || rel.ends_with(this)
                || rel.contains("/target/")
            {
                continue;
            }
            let text = std::fs::read_to_string(&path).expect("readable source");
            for needle in ["tokio", "rmcp", "async fn"] {
                if text
                    .lines()
                    .any(|l| !l.trim_start().starts_with("//") && l.contains(needle))
                {
                    offenders.push(format!("{rel}: {needle}"));
                }
            }
        }
        assert!(
            offenders.is_empty(),
            "async or the MCP SDK outside openwarrant-cli/src/mcp/ (OW-ADR-0014): {offenders:?}"
        );
    }

    #[test]
    fn every_subcommand_supports_json_or_is_listed_as_not_yet() {
        // `mcp` speaks JSON-RPC on stdout; an envelope there would corrupt
        // the transport, so it is listed as not-yet on purpose, not by gap.
        const NOT_YET: &[&str] = &["init", "kf", "telemetry", "migrate", "export", "mcp"];
        let cmd = openwarrant_cli::Cli::command();
        let all: Vec<String> = cmd
            .get_subcommands()
            .map(|c| c.get_name().to_owned())
            .collect();
        assert!(all.len() >= 20, "{all:?}");
        // Every NOT_YET entry must be a real subcommand — a stale name here
        // would make the list look longer than the gap is.
        for n in NOT_YET {
            assert!(
                all.iter().any(|a| a == n),
                "{n} is not a subcommand any more; drop it"
            );
        }
        // The global flag exists and is global.
        let json = cmd
            .get_arguments()
            .find(|a| a.get_id() == "json")
            .expect("--json is a top-level argument");
        assert!(
            json.is_global_set(),
            "--json must be global so it works after the subcommand"
        );
    }
}
