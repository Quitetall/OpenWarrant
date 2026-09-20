// SPDX-License-Identifier: Apache-2.0
//! `--root` names the repository a command acts on.
//!
//! Every one of these runs from a directory that is NOT the repository. That
//! is the whole point: before the flag, the repository was whatever the
//! process happened to be standing in, so a test, a plant or the MCP server
//! could only reach another tree by moving the process into it.
use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn scratch(tag: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "ow-root-{tag}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q", "."])
            .current_dir(&root)
            .status()
            .unwrap()
            .success(),
        "git init"
    );
    root
}

/// Run `war` from a directory that is deliberately not the repository.
fn war_from(cwd: &Path, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(cwd)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn a_scaffolded_program_is_reachable_from_outside_it() {
    let root = scratch("ok");
    let elsewhere = std::env::temp_dir();
    let root_s = root.to_str().unwrap();

    // `war init` took its own --root before the global one existed; the
    // spelling is unchanged, so this is also the regression guard for that.
    let out = war_from(
        &elsewhere,
        &[
            "init",
            "--program",
            "Root Flag",
            "--namespace",
            "RF",
            "--root",
            root_s,
        ],
    );
    assert!(
        out.status.success(),
        "init: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = war_from(&elsewhere, &["--root", root_s, "compile"]);
    assert!(
        out.status.success(),
        "compile: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let out = war_from(&elsewhere, &["--root", root_s, "check", "--generated"]);
    assert!(
        out.status.success(),
        "check --generated on a fresh scaffold should pass: {}",
        String::from_utf8_lossy(&out.stdout)
    );

    let out = war_from(&elsewhere, &["--root", root_s, "--json", "status"]);
    let v: serde_json::Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|_| panic!("status stdout: {}", String::from_utf8_lossy(&out.stdout)));
    assert_eq!(v["schema"], "oh.war/report/v1");

    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_root_with_no_repository_names_that_root_and_not_the_cwd() {
    let empty = scratch("empty");
    let empty_s = empty.to_str().unwrap();
    // Run from a real repository, so a fallback to the cwd would SUCCEED and
    // the assertion below is the thing that catches it.
    let out = war_from(
        Path::new(env!("CARGO_MANIFEST_DIR")),
        &["--root", empty_s, "check"],
    );
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(!out.status.success(), "an empty root must not pass: {text}");
    assert!(
        text.contains(empty_s),
        "the refusal should name the root it was given, got: {text}"
    );
    std::fs::remove_dir_all(&empty).ok();
}

#[test]
fn an_sdk_caller_still_gets_an_envelope_behind_the_root_flag() {
    // `main`'s pre-clap heuristic looks for the subcommand by stepping over
    // the global flags. `--root` takes a value, so a bare `--root <path>`
    // hides `sdk` from it unless both tokens are skipped — and an SDK caller
    // then receives clap's help, which it cannot parse.
    let root = scratch("sdk");
    let root_s = root.to_str().unwrap();
    for args in [
        vec!["--root", root_s, "sdk", "--request", "-"],
        vec!["--json", "--root", root_s, "sdk", "--request", "-"],
    ] {
        let out = war_from(Path::new(env!("CARGO_MANIFEST_DIR")), &args);
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            stdout.contains("oh.war/report/v1"),
            "args={args:?} expected an envelope, got stdout={stdout} stderr={}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    std::fs::remove_dir_all(&root).ok();
}
