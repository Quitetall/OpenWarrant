// SPDX-License-Identifier: Apache-2.0
//! Every integration test of `war`, in one test binary.
//!
//! Each file under tests/ was its own binary, and each linked the whole CLI:
//! about 30 links and several gigabytes written per `cargo test`. They are
//! now modules of this one binary (`autotests = false` in Cargo.toml), and
//! keep their paths and bytes, because Warrant deliverables cite them by
//! path. Run one file's tests by its module name:
//! `cargo test -p openwarrant-cli --test all sdk_cli::`.
//!
//! A new test file is a module here, or `cargo test` will not run it.

#[path = "authority_transition_cli.rs"]
mod authority_transition_cli;
#[path = "board_cli.rs"]
mod board_cli;
#[path = "diff_target.rs"]
mod diff_target;
#[path = "dispatch_bundle_cli.rs"]
mod dispatch_bundle_cli;
#[path = "doctor_cli.rs"]
mod doctor_cli;
#[path = "document_draft.rs"]
mod document_draft;
#[path = "eval_subject_cli.rs"]
mod eval_subject_cli;
#[path = "frontier_integrity_cli.rs"]
mod frontier_integrity_cli;
#[path = "inbox_classify.rs"]
mod inbox_classify;
#[path = "inbox_cli.rs"]
mod inbox_cli;
#[path = "json_envelope.rs"]
mod json_envelope;
#[path = "mcp_stdio.rs"]
mod mcp_stdio;
#[path = "model_cli.rs"]
mod model_cli;
#[path = "normative_completeness_cli.rs"]
mod normative_completeness_cli;
#[path = "overview.rs"]
mod overview;
#[path = "preflight_cli.rs"]
mod preflight_cli;
#[path = "preservation.rs"]
mod preservation;
#[path = "program_phases.rs"]
mod program_phases;
#[path = "progress_viewer.rs"]
mod progress_viewer;
#[path = "question_integrity_cli.rs"]
mod question_integrity_cli;
#[path = "records_cli.rs"]
mod records_cli;
#[path = "render_cli.rs"]
mod render_cli;
#[path = "root_flag.rs"]
mod root_flag;
#[path = "runtime_capture.rs"]
mod runtime_capture;
#[path = "sdk_cli.rs"]
mod sdk_cli;
#[path = "submission_binding_cli.rs"]
mod submission_binding_cli;
#[path = "suggested_commands.rs"]
mod suggested_commands;
#[path = "telemetry_integrity_cli.rs"]
mod telemetry_integrity_cli;
#[path = "tick_ladder_cli.rs"]
mod tick_ladder_cli;
#[path = "tickets_cli.rs"]
mod tickets_cli;
#[path = "verification_subject_cli.rs"]
mod verification_subject_cli;

/// A file under tests/ that is not a module here would never run. This test
/// fails until it is added.
#[test]
fn every_test_file_is_a_module_of_this_binary() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    let all = std::fs::read_to_string(dir.join("all.rs")).unwrap();
    let mut missing = Vec::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let name = entry.unwrap().file_name().into_string().unwrap();
        let Some(stem) = name.strip_suffix(".rs") else {
            continue;
        };
        if stem != "all" && !all.contains(&format!("\nmod {stem};\n")) {
            missing.push(name);
        }
    }
    assert!(missing.is_empty(), "add to tests/all.rs: {missing:?}");
}

#[path = "collector_loading.rs"]
mod collector_loading;
#[path = "collector_signature.rs"]
mod collector_signature;
