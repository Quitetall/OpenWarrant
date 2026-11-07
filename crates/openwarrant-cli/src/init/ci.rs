// SPDX-License-Identifier: Apache-2.0
//! `war init --ci` (t-3c16c W1.5): one opt-in fast pull-request gate, as one
//! workflow file the repository owns.
//!
//! The workflow runs two things: `war check --changed`, which is this
//! project's affected-tests idea applied where it fits any repository, the
//! records a pull request touched; and the repository's own tests, whole. A
//! map that narrows a user's tests would be a claim about their code that only
//! they can make, so nothing here narrows them.
//!
//! Nothing runs until someone asks: plain `war init` writes no workflow, and
//! deleting the file turns the gate off. An existing workflow that differs is
//! never overwritten.

use std::fs;

use camino::{Utf8Path, Utf8PathBuf};

use super::InitError;

/// Where the gate lives, relative to the repository root.
pub const WORKFLOW: &str = ".github/workflows/openwarrant.yml";

/// The checkout action, pinned as docs/INSTALL.md pins it.
const CHECKOUT: &str = "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7";

/// The test commands `root` shows, from the files that name a build system. A
/// repository with several gets each; one with none gets none, and is told.
#[must_use]
pub fn detected_tests(root: &Utf8Path) -> Vec<String> {
    let has = |name: &str| root.join(name).is_file();
    let mut found = Vec::new();
    if has("Cargo.toml") {
        found.push(if has("Cargo.lock") {
            "cargo test --locked".to_owned()
        } else {
            "cargo test".to_owned()
        });
    }
    if fs::read_to_string(root.join("package.json"))
        .ok()
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .is_some_and(|v| v["scripts"]["test"].is_string())
    {
        found.push(if has("package-lock.json") {
            "npm ci && npm test".to_owned()
        } else {
            "npm install && npm test".to_owned()
        });
    }
    if has("go.mod") {
        found.push("go test ./...".to_owned());
    }
    if has("pyproject.toml") || has("pytest.ini") {
        found.push("python -m pytest".to_owned());
    }
    let makefile = fs::read_to_string(root.join("Makefile")).unwrap_or_default();
    if found.is_empty() && makefile.lines().any(|l| l.starts_with("test:")) {
        found.push("make test".to_owned());
    }
    found
}

/// The workflow's bytes: a pure function of the commands and `war`'s version.
#[must_use]
pub fn workflow(tests: &[String], version: &str) -> String {
    let mut out = format!(
        "# Written by `war init --ci`. Yours to edit; delete it to turn the gate off.\n\
         # Fast by design: the records a pull request changed, and your tests.\n\
         name: openwarrant\n\
         on:\n  pull_request:\n\
         permissions:\n  contents: read\n\
         jobs:\n  check:\n    runs-on: ubuntu-latest\n    steps:\n\
         \x20     - uses: {CHECKOUT}\n\
         \x20       with:\n\
         \x20         fetch-depth: 0\n\
         \x20     - name: Install war\n\
         \x20       run: |\n\
         \x20         curl -fsSL \"https://github.com/Quitetall/OpenWarrant/releases/download/v{version}/install.sh\" -o install.sh\n\
         \x20         OW_VERSION=v{version} bash install.sh\n\
         \x20         echo \"$HOME/.local/bin\" >> \"$GITHUB_PATH\"\n\
         \x20     - name: Check what changed\n\
         \x20       env:\n\
         \x20         BASE: ${{{{ github.event.pull_request.base.sha }}}}\n\
         \x20       run: war check --changed --since \"$BASE\"\n"
    );
    for command in tests {
        out.push_str("      - name: Test\n        run: |\n");
        out.push_str(&format!("          {command}\n"));
    }
    if tests.is_empty() {
        out.push_str(
            "      # No test command was found. Add yours here:\n\
             \x20     # - run: make test\n",
        );
    }
    out
}

/// Write the workflow into `root`, and say what was done, one line each.
pub fn install(root: &Utf8Path, named: &[String]) -> Result<Vec<String>, InitError> {
    if let Some(bad) = named
        .iter()
        .find(|c| c.trim().is_empty() || c.contains(['\r', '\n']))
    {
        return Err(InitError::Ci(format!(
            "--test {bad:?}: a test is one command line; put a longer one in a script"
        )));
    }
    let tests = if named.is_empty() {
        detected_tests(root)
    } else {
        named.to_vec()
    };
    let path: Utf8PathBuf = root.join(WORKFLOW);
    let text = workflow(&tests, env!("CARGO_PKG_VERSION"));
    match fs::read_to_string(&path) {
        Ok(existing) if existing == text => {
            return Ok(vec![format!("{WORKFLOW} is already in place")]);
        }
        Ok(_) => {
            return Err(InitError::Ci(format!(
                "{WORKFLOW} exists and differs, so it was left alone; delete it to write the \
                 gate, or add `war check --changed --since \"$BASE\"` to it yourself"
            )));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(InitError::Io {
                context: format!("could not read {path}"),
                source: e,
            });
        }
    }
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|source| InitError::Io {
            context: format!("could not create {dir}"),
            source,
        })?;
    }
    fs::write(&path, text).map_err(|source| InitError::Io {
        context: format!("could not write {path}"),
        source,
    })?;
    let mut lines = vec![format!(
        "wrote {WORKFLOW}: `war check --changed` and {} on every pull request; opt-in, and \
         deleting the file turns it off",
        if tests.is_empty() {
            "no tests".to_owned()
        } else {
            tests
                .iter()
                .map(|t| format!("`{t}`"))
                .collect::<Vec<_>>()
                .join(", ")
        }
    )];
    if tests.is_empty() {
        lines.push(
            "no test command found: edit the workflow, or rerun with `war init --ci --test \
             \"<command>\"`"
                .to_owned(),
        );
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> (tempfile::TempDir, Utf8PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(tmp.path().to_path_buf()).unwrap();
        (tmp, root)
    }

    #[test]
    fn the_workflow_runs_the_check_and_each_test() {
        let text = workflow(&["cargo test".into(), "make lint".into()], "9.9.9");
        assert!(text.contains("war check --changed --since \"$BASE\""));
        assert!(text.contains("          cargo test\n"));
        assert!(text.contains("          make lint\n"));
        assert!(text.contains("fetch-depth: 0"), "--since needs the base");
        assert!(text.contains("download/v9.9.9/install.sh"), "war is pinned");
        // The ref reaches the shell through the environment, never the script.
        assert!(!text.contains("--since ${{"));
    }

    #[test]
    fn the_tests_are_read_from_what_the_repository_shows() {
        let (_tmp, root) = scratch();
        assert!(detected_tests(&root).is_empty());
        fs::write(root.join("package.json"), r#"{"scripts":{}}"#).unwrap();
        assert!(detected_tests(&root).is_empty(), "no test script, no test");
        fs::write(root.join("package.json"), r#"{"scripts":{"test":"x"}}"#).unwrap();
        fs::write(root.join("Cargo.toml"), "").unwrap();
        assert_eq!(
            detected_tests(&root),
            ["cargo test", "npm install && npm test"]
        );
    }

    #[test]
    fn a_workflow_that_differs_is_never_overwritten() {
        let (_tmp, root) = scratch();
        fs::create_dir_all(root.join(".github/workflows")).unwrap();
        fs::write(root.join(WORKFLOW), "mine\n").unwrap();
        let refused = install(&root, &["true".into()]).unwrap_err();
        assert!(refused.to_string().contains("left alone"));
        assert_eq!(fs::read_to_string(root.join(WORKFLOW)).unwrap(), "mine\n");
    }

    #[test]
    fn writing_it_twice_changes_nothing_and_a_multiline_test_is_refused() {
        let (_tmp, root) = scratch();
        install(&root, &["true".into()]).unwrap();
        let bytes = fs::read(root.join(WORKFLOW)).unwrap();
        let again = install(&root, &["true".into()]).unwrap();
        assert!(again[0].contains("already in place"));
        assert_eq!(fs::read(root.join(WORKFLOW)).unwrap(), bytes);
        assert!(install(&root, &["a\nb".into()]).is_err());
    }
}
