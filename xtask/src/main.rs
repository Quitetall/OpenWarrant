// SPDX-License-Identifier: AGPL-3.0-or-later
//! `cargo xtask gate` — the aggregate gate (SAS §92).
//!
//! §92: "The final command SHALL exit zero only when every positive fixture
//! passes and every planted violation is rejected by the intended control."
//!
//! Two halves, and the second is the one that matters. Running the test suite
//! proves the positive fixtures pass. It does NOT prove that any control would
//! have caught a violation — a validator that returns `Ok(())` unconditionally
//! passes every positive fixture ever written. The planted-violation half exists
//! because this fleet has three times shipped a green gate that compared nothing.
//!
//! Planted violations live in two places, and both are needed:
//!
//! - `#[test]` cases asserting a refusal from a function given a value
//!   (`non_v7_uuid_is_refused`, `duplicate_ordinal_is_refused`, and the rest);
//! - `conformance/plant.sh`, which mutates real files and runs the SHIPPED
//!   BINARY, then asserts the right rule fired for the right reason.
//!
//! The second exists because the first cannot catch a control that is correct in
//! isolation and never reached in the real code path.

use std::path::Path;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("gate") => gate(),
        Some(other) => {
            eprintln!("xtask: unknown task {other:?}; known tasks: gate");
            ExitCode::FAILURE
        }
        None => {
            eprintln!("usage: cargo xtask gate");
            ExitCode::FAILURE
        }
    }
}

/// One step of the gate: a label and the command that decides it.
struct Step {
    label: &'static str,
    program: &'static str,
    args: &'static [&'static str],
}

/// The SPDX identifier every Rust source file must declare.
///
/// Checked mechanically because the Apache-2.0 relicense rewrites exactly this
/// line in every file (see RELICENSING.md). A file that never carried a header
/// would be silently skipped by that rewrite and would keep asserting the old
/// licence — or none at all — after the flip.
const EXPECTED_SPDX: &str = "// SPDX-License-Identifier: AGPL-3.0-or-later";

/// Walk `.rs` files and report every one missing its SPDX header.
fn check_spdx() -> Result<usize, std::io::Error> {
    fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) -> Result<(), std::io::Error> {
        for entry in std::fs::read_dir(dir)? {
            let path = entry?.path();
            if path.is_dir() {
                if path.file_name().is_some_and(|n| n == "target") {
                    continue;
                }
                walk(&path, out)?;
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    for root in ["crates", "xtask"] {
        let dir = Path::new(root);
        if dir.is_dir() {
            walk(dir, &mut files)?;
        }
    }
    files.sort();

    let mut missing = 0usize;
    for file in &files {
        let text = std::fs::read_to_string(file)?;
        if !text.starts_with(EXPECTED_SPDX) {
            // Report every one, not the first: otherwise the fix is one
            // re-run per file.
            println!("   missing SPDX header: {}", file.display());
            missing += 1;
        }
    }
    if missing == 0 {
        println!("   {} file(s) carry the SPDX header", files.len());
    }
    Ok(missing)
}

fn gate() -> ExitCode {
    let steps = [
        Step {
            label: "format",
            program: "cargo",
            args: &["fmt", "--all", "--check"],
        },
        Step {
            label: "clippy",
            program: "cargo",
            args: &[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        },
        Step {
            label: "tests (positive fixtures + planted violations)",
            program: "cargo",
            args: &["test", "--workspace", "--no-fail-fast"],
        },
        // The license gate is not hygiene here. This repository ships
        // AGPL-3.0-or-later and is intended to relicense to Apache-2.0; a
        // copyleft dependency adopted today would make that impossible later.
        Step {
            label: "licenses (permissive-only; protects the Apache-2.0 path)",
            program: "cargo",
            args: &["deny", "check", "licenses"],
        },
        // §92's second half, and the reason the first half means anything: every
        // planted violation must be rejected BY ITS INTENDED CONTROL. The unit
        // tests above prove the code does what it says; this proves the shipped
        // binary refuses what it should, on real files, for the stated reason.
        Step {
            label: "planted violations (§92 — each rejected by its intended control)",
            program: "bash",
            args: &["conformance/plant.sh"],
        },
    ];

    let mut failed = Vec::new();

    // An in-process step, run first because it is instant and its failure is
    // always actionable.
    println!("== spdx headers ==");
    match check_spdx() {
        Ok(0) => println!("   ok"),
        Ok(n) => {
            println!("   FAILED ({n} file(s) missing a header)");
            failed.push("spdx headers");
        }
        Err(err) => {
            println!("   COULD NOT RUN: {err}");
            failed.push("spdx headers");
        }
    }

    for step in &steps {
        println!("== {} ==", step.label);
        let status = Command::new(step.program).args(step.args).status();
        match status {
            Ok(status) if status.success() => println!("   ok"),
            Ok(status) => {
                println!("   FAILED ({status})");
                failed.push(step.label);
            }
            Err(err) => {
                // "Could not run" is not "failed". Collapsing them is exactly
                // what SAS §96.4 forbids for gate results, and the same honesty
                // applies to the gate that enforces it.
                println!("   COULD NOT RUN: {err}");
                failed.push(step.label);
            }
        }
    }

    // Report every failing step, not the first. A gate that stops at the first
    // failure makes the operator re-run it once per defect.
    if failed.is_empty() {
        println!("\ngate: PASS — {} step(s) green", steps.len() + 1);
        ExitCode::SUCCESS
    } else {
        println!(
            "\ngate: FAIL — {}/{} step(s) failed:",
            failed.len(),
            steps.len() + 1
        );
        for label in &failed {
            println!("  - {label}");
        }
        ExitCode::FAILURE
    }
}
