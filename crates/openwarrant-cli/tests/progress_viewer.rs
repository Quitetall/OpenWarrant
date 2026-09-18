// SPDX-License-Identifier: Apache-2.0
use std::{path::PathBuf, process::Command};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
#[test]
fn offline_view_contains_live_data_and_refuses_source_overwrite() {
    let output = std::env::temp_dir().join(format!("ow-progress-{}.html", std::process::id()));
    let result = Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(repo())
        .args(["progress", "--html"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let html = std::fs::read_to_string(&output).unwrap();
    assert!(html.starts_with("<!doctype html>"));
    assert!(html.contains("OpenWarrant progress"));
    assert!(html.contains("OW-WAR-0076"));
    assert!(html.contains("application/json"));
    assert!(!html.contains("<script src="));
    std::fs::remove_file(output).unwrap();
    let before = std::fs::read(repo().join("README.md")).unwrap();
    let denied = Command::new(env!("CARGO_BIN_EXE_war"))
        .current_dir(repo())
        .args(["progress", "--html", "README.md"])
        .output()
        .unwrap();
    assert!(!denied.status.success());
    assert_eq!(std::fs::read(repo().join("README.md")).unwrap(), before);
}

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "ow-viewer-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        let fixture = Self(path);
        fixture.war(&["init", "--namespace", "VIEW", "--program", "Viewer tests"]);
        fixture
    }
    fn war(&self, args: &[&str]) {
        let result = Command::new(env!("CARGO_BIN_EXE_war"))
            .args(args)
            .current_dir(&self.0)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{} {}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }
    fn report(&self, state: &str) -> PathBuf {
        let path = self
            .0
            .join("docs/warrants/VIEW-WAR-0001/implementation/progress.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path,serde_json::to_vec(&serde_json::json!({
            "schema":"oh.war/viewer-work-report/v1","warrant":"VIEW-WAR-0001","work_state":state,
            "reported_by":"agent://test","revision":"0000000000000000000000000000000000000000",
            "summary":"</script><script>globalThis.injected=true</script> __LIVE__ __INTERVAL__",
            "next_steps":["Review outcome"]
        })).unwrap()).unwrap();
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
struct Server {
    child: Child,
    host: String,
}
impl Server {
    fn start(fixture: &Fixture) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_war"))
            .args([
                "progress",
                "--serve",
                "--port",
                "0",
                "--refresh-secs",
                "1",
                "--json",
            ])
            .current_dir(&fixture.0)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let stdout = child.stdout.take().unwrap();
        let mut server = Self {
            child,
            host: String::new(),
        };
        let mut reader = BufReader::new(stdout);
        let mut json = String::new();
        let result: serde_json::Value = loop {
            assert!(
                reader.read_line(&mut json).unwrap() > 0,
                "Server exited: {json}"
            );
            if let Ok(value) = serde_json::from_str(&json) {
                break value;
            }
            assert!(json.len() < 65536, "Invalid startup envelope");
        };
        server.host = result["result"]["url"]
            .as_str()
            .unwrap()
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_string();
        server
    }
    fn request(&self, method: &str, path: &str, host: &str, extra: &str) -> String {
        let mut stream = TcpStream::connect(&self.host).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(6)))
            .unwrap();
        write!(
            stream,
            "{method} {path} HTTP/1.1\r\nHost: {host}\r\n{extra}\r\n"
        )
        .unwrap();
        let mut result = String::new();
        stream.read_to_string(&mut result).unwrap();
        result
    }
    fn data(&self) -> serde_json::Value {
        let result = self.request("GET", "/api/progress", &self.host, "");
        assert!(result.starts_with("HTTP/1.1 200"));
        serde_json::from_str(result.split_once("\r\n\r\n").unwrap().1).unwrap()
    }
    fn until(&self, check: impl Fn(&serde_json::Value) -> bool) -> serde_json::Value {
        let start = Instant::now();
        loop {
            let data = self.data();
            if check(&data) {
                return data;
            }
            assert!(
                start.elapsed() < Duration::from_secs(12),
                "Timed out: {data}"
            );
            std::thread::sleep(Duration::from_millis(150));
        }
    }
}
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn live_refresh_keeps_last_good_snapshot_and_refuses_writes_and_bad_origins() {
    let fixture = Fixture::new();
    let report = fixture.report("in-progress");
    let server = Server::start(&fixture);
    assert_eq!(
        server.data()["snapshot"]["reports"]["VIEW-WAR-0001"]["report"]["work_state"],
        "in-progress"
    );
    let html = server.request("GET", "/", &server.host, "");
    assert!(!html.contains("</script><script>globalThis.injected=true"));
    assert!(html.contains("__LIVE__ __INTERVAL__"));
    fixture.report("completed");
    let completed = server.until(|v| {
        v["snapshot"]["reports"]["VIEW-WAR-0001"]["report"]["work_state"] == "completed"
    });
    let config = fixture.0.join("openwarrant.toml");
    let original = std::fs::read(&config).unwrap();
    std::fs::write(&config, "invalid = [").unwrap();
    let stale = server.until(|v| v["error"].is_string());
    assert_eq!(
        stale["snapshot"]["record_digest"],
        completed["snapshot"]["record_digest"]
    );
    std::fs::write(config, original).unwrap();
    server.until(|v| v["error"].is_null());
    std::fs::write(&report, "{bad json").unwrap();
    server.until(|v| v["snapshot"]["reports"]["VIEW-WAR-0001"]["error"].is_string());
    for (method, path, host, extra, code) in [
        ("POST", "/api/progress", server.host.as_str(), "", "405"),
        ("GET", "/api/progress", "attacker.example", "", "403"),
        (
            "GET",
            "/api/progress",
            server.host.as_str(),
            "Origin: https://attacker.example\r\n",
            "403",
        ),
        (
            "GET",
            "/source/../../etc/passwd",
            server.host.as_str(),
            "",
            "404",
        ),
    ] {
        assert!(
            server
                .request(method, path, host, extra)
                .starts_with(&format!("HTTP/1.1 {code}"))
        );
    }
    assert_eq!(std::fs::read_to_string(report).unwrap(), "{bad json");
}

#[test]
fn unknown_report_and_symlink_sources_never_become_completed() {
    let fixture = Fixture::new();
    let report = fixture.report("verified");
    let output = fixture.0.join("overview.html");
    fixture.war(&["progress", "--html", output.to_str().unwrap()]);
    let html = std::fs::read_to_string(&output).unwrap();
    assert!(html.contains("unknown variant"));
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
    value["work_state"] = "completed".into();
    value["notes"] = "../outside.md".into();
    std::fs::write(report, serde_json::to_vec(&value).unwrap()).unwrap();
    fixture.war(&["progress", "--html", output.to_str().unwrap()]);
    let html = std::fs::read_to_string(&output).unwrap();
    assert!(html.contains("without traversal"));
    #[cfg(unix)]
    {
        let link = fixture.0.join("linked.html");
        std::os::unix::fs::symlink(&output, &link).unwrap();
        let denied = Command::new(env!("CARGO_BIN_EXE_war"))
            .current_dir(&fixture.0)
            .args(["progress", "--html"])
            .arg(link)
            .output()
            .unwrap();
        assert!(!denied.status.success());
    }
}

#[cfg(unix)]
#[test]
fn cached_evidence_replaced_by_fifo_or_symlink_refuses_without_hanging() {
    let fixture = Fixture::new();
    let report = fixture.report("completed");
    let note = fixture.0.join("note.md");
    std::fs::write(&note, "Fixture evidence").unwrap();
    let mut data: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&report).unwrap()).unwrap();
    data["notes"] = "note.md".into();
    std::fs::write(&report, serde_json::to_vec(&data).unwrap()).unwrap();
    let server = Server::start(&fixture);
    let snapshot = server.data();
    let url = snapshot["snapshot"]["links"]["note.md"].as_str().unwrap();
    assert!(
        server
            .request("GET", url, &server.host, "")
            .contains("Fixture evidence")
    );
    std::fs::remove_file(&note).unwrap();
    assert!(
        Command::new("mkfifo")
            .arg(&note)
            .status()
            .unwrap()
            .success()
    );
    let reply = server.request("GET", url, &server.host, "");
    assert!(reply.starts_with("HTTP/1.1 404"));
    assert!(server.data().is_object());
    std::fs::remove_file(&note).unwrap();
    std::os::unix::fs::symlink("/etc/passwd", &note).unwrap();
    let reply = server.request("GET", url, &server.host, "");
    assert!(reply.starts_with("HTTP/1.1 403") || reply.starts_with("HTTP/1.1 404"));
    assert!(!reply.contains("root:"));
    assert!(server.data().is_object());
}

#[test]
fn json_snapshot_reuses_report_validation_and_preserves_unknown() {
    let fixture = Fixture::new();
    let path = fixture.report("completed");
    let snapshot = || {
        let output = Command::new(env!("CARGO_BIN_EXE_war"))
            .current_dir(&fixture.0)
            .args(["progress", "--snapshot", "--json"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["result"].clone()
    };
    let valid = snapshot();
    assert_eq!(valid["schema"], "oh.war/progress-view/v1");
    assert_eq!(
        valid["reports"]["VIEW-WAR-0001"]["report"]["work_state"],
        "completed"
    );
    let mut report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    report["reported_by"] = serde_json::json!("");
    std::fs::write(&path, serde_json::to_vec(&report).unwrap()).unwrap();
    let invalid = snapshot();
    assert!(invalid["reports"]["VIEW-WAR-0001"]["report"].is_null());
    assert!(invalid["reports"]["VIEW-WAR-0001"]["error"].is_string());
}

#[test]
fn configured_roadmap_changes_snapshot_digest_and_refuses_dangling_warrants() {
    let fixture = Fixture::new();
    let config = fixture.0.join("docs/roadmap/view.json");
    std::fs::create_dir_all(config.parent().unwrap()).unwrap();
    let capture = || {
        Command::new(env!("CARGO_BIN_EXE_war"))
            .current_dir(&fixture.0)
            .args(["progress", "--snapshot", "--json"])
            .output()
            .unwrap()
    };
    let before = capture();
    assert!(before.status.success());
    let before: serde_json::Value = serde_json::from_slice(&before.stdout).unwrap();
    let mut roadmap = serde_json::json!({"schema":"oh.war/roadmap-view/v1","title":"Release",
        "nodes":[{"id":"feature","parent":null,"title":"<script>bad()</script>","outcome":"Useful",
        "warrants":["VIEW-WAR-0001"]}]});
    std::fs::write(&config, serde_json::to_vec(&roadmap).unwrap()).unwrap();
    let after = capture();
    assert!(after.status.success());
    let after: serde_json::Value = serde_json::from_slice(&after.stdout).unwrap();
    assert_eq!(after["result"]["roadmap"], roadmap);
    assert!(after["result"]["stage_frontier"].is_object());
    assert!(after["result"]["stage_frontier_error"].is_null());
    assert_ne!(
        before["result"]["record_digest"],
        after["result"]["record_digest"]
    );
    let html = fixture.0.join("roadmap.html");
    fixture.war(&["progress", "--html", html.to_str().unwrap()]);
    assert!(
        !std::fs::read_to_string(html)
            .unwrap()
            .contains("<script>bad()</script>")
    );
    roadmap["nodes"][0]["warrants"] = serde_json::json!(["VIEW-WAR-9999"]);
    std::fs::write(&config, serde_json::to_vec(&roadmap).unwrap()).unwrap();
    let refused = capture();
    assert!(!refused.status.success());
    assert!(String::from_utf8_lossy(&refused.stdout).contains("unknown Warrant"));
}

#[test]
fn roadmap_keeps_frontier_error_visible_instead_of_empty_ready_state() {
    let fixture = Fixture::new();
    let path = fixture.0.join("docs/roadmap/view.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, r#"{"schema":"oh.war/roadmap-view/v1","title":"Test","nodes":[{"id":"feature","title":"Feature","outcome":"Scope","warrants":["VIEW-WAR-0001"]}]}"#).unwrap();
    let output = || {
        Command::new(env!("CARGO_BIN_EXE_war"))
            .current_dir(&fixture.0)
            .args(["progress", "--snapshot", "--json"])
            .output()
            .unwrap()
    };
    let before = output();
    assert!(before.status.success());
    let before: serde_json::Value = serde_json::from_slice(&before.stdout).unwrap();
    std::fs::write(
        fixture.0.join("docs/warrants/VIEW-WAR-0001/journal.jsonl"),
        "not a journal\n",
    )
    .unwrap();
    let after = output();
    assert!(
        after.status.success(),
        "{}",
        String::from_utf8_lossy(&after.stdout)
    );
    let after: serde_json::Value = serde_json::from_slice(&after.stdout).unwrap();
    assert!(after["result"]["stage_frontier"].is_null());
    assert!(
        after["result"]["stage_frontier_error"]
            .as_str()
            .unwrap()
            .contains("journal")
    );
    assert_ne!(
        before["result"]["record_digest"],
        after["result"]["record_digest"]
    );
    assert_eq!(after["result"]["roadmap"]["title"], "Test");
}
