// SPDX-License-Identifier: Apache-2.0
use std::{fs, path::Path, process::Command};

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let dest = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &dest);
        } else {
            fs::copy(entry.path(), dest).unwrap();
        }
    }
}

#[test]
fn damaged_question_store_is_never_an_empty_success() {
    let root = std::env::temp_dir().join(format!("ow-question-integrity-{}", std::process::id()));
    copy_tree(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../conformance/fixtures/inbox/repository"),
        &root,
    );
    let run = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_war"))
            .args(args)
            .arg("--json")
            .current_dir(&root)
            .output()
            .unwrap()
    };
    let good = run(&["questions", "IX-WAR-0002"]);
    assert!(good.status.success());
    let value: serde_json::Value = serde_json::from_slice(&good.stdout).unwrap();
    assert_eq!(value["result"]["blocking_open"], 1);
    for args in [
        vec!["answers", "IX-WAR-0002"],
        vec!["watch", "--once"],
        vec!["console"],
    ] {
        assert!(
            run(&args).status.success(),
            "valid queue must remain readable: {args:?}"
        );
    }
    let path = root.join("docs/warrants/IX-WAR-0002/questions");
    let original = fs::read(path.join("Q-001.toml")).unwrap();
    fs::remove_dir_all(&path).unwrap();
    fs::write(&path, "not a directory").unwrap();
    for args in [
        vec!["questions", "IX-WAR-0002"],
        vec!["answers", "IX-WAR-0002"],
        vec!["watch", "--once"],
        vec!["console"],
    ] {
        let result = run(&args);
        assert!(
            !result.status.success(),
            "{args:?} silently succeeded: {}",
            String::from_utf8_lossy(&result.stdout)
        );
    }
    fs::remove_file(&path).unwrap();
    assert!(
        run(&["questions", "IX-WAR-0002"]).status.success(),
        "missing store is valid"
    );
    assert!(run(&["answers", "IX-WAR-0002"]).status.success());
    fs::create_dir(&path).unwrap();
    fs::write(path.join("Q-001.toml"), original).unwrap();
    fs::write(path.join("Q-002.toml"), "broken TOML").unwrap();
    let damaged = run(&["questions", "IX-WAR-0002"]);
    assert!(!damaged.status.success());
    let value: serde_json::Value = serde_json::from_slice(&damaged.stdout).unwrap();
    assert_eq!(
        value["result"]["blocking_open"], 1,
        "healthy questions remain visible"
    );
    assert!(!run(&["answers", "IX-WAR-0002"]).status.success());
    fs::remove_file(path.join("Q-002.toml")).unwrap();
    fs::create_dir(path.join("Q-002.toml")).unwrap();
    assert!(!run(&["questions", "IX-WAR-0002"]).status.success());
    assert!(!run(&["answers", "IX-WAR-0002"]).status.success());
    fs::remove_dir(path.join("Q-002.toml")).unwrap();
    let q = path.join("Q-001.toml");
    let text = fs::read_to_string(&q).unwrap()
        + "\n[answer]\nanswered_by = \"fixture-human\"\nanswered_at = \"2020-01-02T00:00:00Z\"\nanswer = \"Keep the existing contract\"\n";
    fs::write(q, text).unwrap();
    let answered = run(&["answers", "IX-WAR-0002"]);
    assert!(answered.status.success());
    let value: serde_json::Value = serde_json::from_slice(&answered.stdout).unwrap();
    assert_eq!(
        value["result"][0]["answer"]["answer"],
        "Keep the existing contract"
    );
    #[cfg(unix)]
    {
        let actual = path.with_file_name("saved-questions");
        fs::rename(&path, &actual).unwrap();
        std::os::unix::fs::symlink(&actual, &path).unwrap();
        assert!(!run(&["questions", "IX-WAR-0002"]).status.success());
        assert!(!run(&["answers", "IX-WAR-0002"]).status.success());
        fs::remove_file(&path).unwrap();
        fs::rename(&actual, &path).unwrap();
    }
    fs::remove_dir_all(root).unwrap();
}
