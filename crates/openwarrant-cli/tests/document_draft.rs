// SPDX-License-Identifier: Apache-2.0
use serde_json::{Value, json};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::atomic::{AtomicUsize, Ordering},
};
static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p = std::env::temp_dir().join(format!(
            "ow88-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&p).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn command(root: &Path, resume: bool) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_war"));
    c.current_dir(root).args([
        "--json",
        "document",
        "draft",
        "--draft-dir",
        "draft",
        "--output",
        "result.md",
    ]);
    if resume {
        c.arg("--resume");
    }
    c.stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    c
}
fn run(root: &Path, resume: bool, input: &str) -> (bool, Value, String) {
    let mut child = command(root, resume).spawn().unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    (
        output.status.success(),
        serde_json::from_slice(&output.stdout).unwrap(),
        String::from_utf8(output.stderr).unwrap(),
    )
}
fn latest(root: &Path) -> Value {
    let mut paths: Vec<_> = fs::read_dir(root.join("draft"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|s| s == "json"))
        .collect();
    paths.sort();
    serde_json::from_slice(&fs::read(paths.last().unwrap()).unwrap()).unwrap()
}
#[test]
fn interactive_save_matches_noninteractive_sdk_author_bytes() {
    let root = Temp::new();
    let (ok, report, preview) = run(
        &root.0,
        false,
        "title\nReset password\nid\nexample:reset\nunit outcome binding\n# Reset password\n\nSend a reset link.\n.\nunit scope binding\n## Scope\n\nDo not change signup.\n.\npreview\nsave\n",
    );
    assert!(ok, "{report}");
    assert_eq!(report["result"]["saved"], true);
    assert_eq!(report["result"]["qualified"], false);
    assert!(preview.contains("# Reset password"));
    let state = latest(&root.0);
    let request = json!({"schema":"oh.war/sdk-request/v1","operation":"author","metadata":state["metadata"],"units":state["units"]});
    let mut child = Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(&root.0)
        .args(["sdk", "--request", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(&serde_json::to_vec(&request).unwrap())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let result: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        fs::read_to_string(root.0.join("result.md")).unwrap(),
        result["result"]["source"].as_str().unwrap()
    );
}
#[test]
fn cancel_and_midunit_eof_retain_draft_and_resume_without_authority() {
    let root = Temp::new();
    let (ok, report, _) = run(
        &root.0,
        false,
        "title\nResumable\nunit scope binding\n## Scope\nKeep these lines.\n",
    );
    assert!(ok);
    assert_eq!(report["result"]["saved"], false);
    assert!(!root.0.join("result.md").exists());
    assert!(
        latest(&root.0)["units"][1]["text"]
            .as_str()
            .unwrap()
            .contains("Keep these lines.")
    );
    let (ok, report, _) = run(&root.0, true, "cancel\n");
    assert!(ok);
    assert_eq!(report["result"]["saved"], false);
    let (ok, report, _) = run(&root.0, true, "save\n");
    assert!(ok, "{report}");
    assert_eq!(report["result"]["saved"], true);
}
#[test]
fn false_maturity_and_existing_output_refuse_without_losing_draft() {
    let root = Temp::new();
    let (ok, report, stderr) = run(&root.0, false, "meta state\n\"verified\"\nsave\ncancel\n");
    assert!(ok);
    assert_eq!(report["result"]["saved"], false);
    assert!(stderr.contains("Invalid draft"));
    assert!(!root.0.join("result.md").exists());
    fs::write(root.0.join("result.md"), "original").unwrap();
    let (ok, report, _) = run(&root.0, true, "meta state\n\"draft\"\nsave\n");
    assert!(!ok);
    assert_eq!(report["result"]["saved"], false);
    assert_eq!(
        fs::read_to_string(root.0.join("result.md")).unwrap(),
        "original"
    );
    assert_eq!(latest(&root.0)["metadata"][5][1], "draft");
}
#[test]
fn malformed_resume_and_excessive_input_refuse() {
    let root = Temp::new();
    let (ok, _, _) = run(&root.0, false, "cancel\n");
    assert!(ok);
    fs::write(root.0.join("draft/draft-000001.json"), "{bad}").unwrap();
    let (ok, _, _) = run(&root.0, true, "");
    assert!(!ok);
    let other = Temp::new();
    let (ok, _, _) = run(&other.0, false, &format!("title\n{}\n", "x".repeat(65536)));
    assert!(!ok);
    assert_eq!(latest(&other.0)["metadata"][4][1], "Untitled Warrant");
}
#[test]
fn concurrent_writer_refuses_and_lock_releases_when_process_ends() {
    let root = Temp::new();
    let mut child = command(&root.0, false).spawn().unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !root.0.join("draft/draft-000000.json").exists() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    let (ok, _, _) = run(&root.0, true, "");
    assert!(!ok);
    child.kill().unwrap();
    child.wait().unwrap();
    let (ok, report, _) = run(&root.0, true, "save\n");
    assert!(ok, "{report}");
    assert_eq!(report["result"]["saved"], true);
}
#[cfg(unix)]
#[test]
fn output_and_snapshot_symlinks_refuse_without_changing_targets() {
    use std::os::unix::fs::symlink;
    let root = Temp::new();
    fs::write(root.0.join("original"), "keep").unwrap();
    symlink("original", root.0.join("result.md")).unwrap();
    let (ok, _, _) = run(&root.0, false, "save\n");
    assert!(!ok);
    assert_eq!(fs::read_to_string(root.0.join("original")).unwrap(), "keep");
    symlink("../original", root.0.join("draft/draft-000001.json")).unwrap();
    let (ok, _, _) = run(&root.0, true, "");
    assert!(!ok);
    assert_eq!(fs::read_to_string(root.0.join("original")).unwrap(), "keep");
}
#[test]
fn title_does_not_rewrite_scope_after_outcome_is_removed() {
    let root = Temp::new();
    let (ok, _, _) = run(&root.0, false, "drop outcome\ntitle\nNew title\ncancel\n");
    assert!(ok);
    assert_eq!(
        latest(&root.0)["units"][0]["text"],
        "## Scope\n\nDefine the work boundary.\n"
    );
}
#[test]
fn duplicate_metadata_and_checkpoint_keys_refuse_without_silent_replacement() {
    let root = Temp::new();
    let (ok, _, stderr) = run(
        &root.0,
        false,
        "meta extensions\n{\"test:x\":{\"policy\":\"must\",\"policy\":\"may\"}}\ncancel\n",
    );
    assert!(ok);
    assert!(stderr.contains("duplicate"));
    assert!(
        !latest(&root.0)["metadata"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field[0] == "extensions")
    );
    let mut state = latest(&root.0);
    state["metadata"]
        .as_array_mut()
        .unwrap()
        .push(json!(["extensions",{"test:x":{"policy":"must"}}]));
    let text = serde_json::to_string(&state).unwrap().replace(
        "\"policy\":\"must\"",
        "\"policy\":\"must\",\"policy\":\"may\"",
    );
    fs::write(root.0.join("draft/draft-000001.json"), text).unwrap();
    let (ok, _, _) = run(&root.0, true, "save\n");
    assert!(!ok);
    assert!(!root.0.join("result.md").exists());
}
#[test]
fn oversized_history_and_output_inside_session_refuse() {
    let root = Temp::new();
    assert!(run(&root.0, false, "cancel\n").0);
    for n in 1..=4096 {
        fs::write(root.0.join(format!("draft/draft-{n:06}.json")), "{}").unwrap();
    }
    assert!(!run(&root.0, true, "").0);
    let other = Temp::new();
    let output = Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(&other.0)
        .args([
            "--json",
            "document",
            "draft",
            "--draft-dir",
            "draft",
            "--output",
            "draft/result.md",
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        report["diagnostics"][0]["message"]
            .as_str()
            .unwrap()
            .contains("outside")
    );
    assert!(!other.0.join("draft/result.md").exists());
    assert_eq!(latest(&other.0)["metadata"][4][1], "Untitled Warrant");
}
#[test]
fn aggregate_checkpoint_depth_refuses_before_publishing_unresumable_state() {
    let root = Temp::new();
    let value = format!("{}0{}", "[".repeat(63), "]".repeat(63));
    let (ok, _, _) = run(&root.0, false, &format!("meta custom\n{value}\ncancel\n"));
    assert!(!ok);
    assert!(
        !latest(&root.0)["metadata"]
            .as_array()
            .unwrap()
            .iter()
            .any(|field| field[0] == "custom")
    );
    let (ok, report, _) = run(&root.0, true, "save\n");
    assert!(ok, "{report}");
    assert_eq!(report["result"]["saved"], true);
}
