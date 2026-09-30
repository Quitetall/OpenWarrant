// SPDX-License-Identifier: AGPL-3.0-or-later

use std::fs;
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

fn scratch() -> std::path::PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let path = std::env::temp_dir().join(format!(
        "war-legacy-dispositions-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&path).expect("scratch");
    path
}

#[test]
fn malformed_manifest_is_refused_by_the_shipped_command() {
    let root = scratch();
    let manifest = root.join("manifest.json");
    let source_import = root.join("import.json");
    fs::write(&manifest, "{}\n").expect("manifest");
    fs::write(&source_import, "{}\n").expect("import");

    let git = |args: &[&str]| {
        let output = Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(args)
            .output()
            .expect("run git");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["config", "user.name", "OpenWarrant Test"]);
    git(&["config", "user.email", "test@example.invalid"]);
    git(&["add", "."]);
    git(&["commit", "-q", "-m", "fixture"]);

    let output = Command::new(env!("CARGO_BIN_EXE_war"))
        .arg("legacy-dispositions")
        .arg("--manifest")
        .arg("manifest.json")
        .arg("--source-repo")
        .arg(&root)
        .arg("--source-import")
        .arg("import.json")
        .arg("--warrant-repo")
        .arg(&root)
        .output()
        .expect("run war");

    let _ = fs::remove_dir_all(&root);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("legacy disposition manifest")
            && stderr.contains("is not valid JSON")
            && stderr.contains("missing field `schema`"),
        "malformed manifest failed for the wrong reason: {stderr}"
    );
}
