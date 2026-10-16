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

#[test]
fn working_form_progress_follows_records_without_signing_or_hiding_directory_warrants() {
    let fixture = Fixture::new();
    fixture.war(&[
        "create",
        "A prototype outcome",
        "--item",
        "Deliver one result",
    ]);
    let dir = std::fs::read_dir(fixture.0.join("docs/tickets"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(dir.join("manifest.toml")).unwrap()).unwrap();
    let id = manifest["id"].as_str().unwrap();
    let read = |args: &[&str]| {
        let output = Command::new(env!("CARGO_BIN_EXE_war"))
            .args(args)
            .current_dir(&fixture.0)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()["result"].clone()
    };
    let shown = read(&["show", id, "--json"]);
    let target = format!("{id}/{}", shown["items"][0]["id"].as_str().unwrap());
    let open = read(&["progress", "--snapshot", "--json"]);
    assert_eq!(open["working_forms"]["records"][0]["id"], id);
    assert_eq!(open["working_forms"]["records"][0]["state"], "open");
    assert_eq!(open["working_forms"]["completed"], 0);
    assert_eq!(open["working_forms"]["qualification_assessed"], false);
    assert_eq!(open["legacy"]["warrants"][0]["alias"], "VIEW-WAR-0001");
    fixture.war(&["claim", &target, "--as", "agent://fixture"]);
    let active = read(&["progress", "--snapshot", "--json"]);
    assert_eq!(
        active["working_forms"]["records"][0]["state"],
        "in_progress"
    );
    assert_ne!(active["record_digest"], open["record_digest"]);
    fixture.war(&[
        "done",
        &target,
        "--as",
        "agent://fixture",
        "--note",
        "Fixture result delivered",
    ]);
    let done = read(&["progress", "--snapshot", "--json"]);
    assert_eq!(done["working_forms"]["records"][0]["state"], "done");
    assert_eq!(done["working_forms"]["completed"], 1);
    assert_ne!(done["record_digest"], active["record_digest"]);
    let overview = read(&["overview", "--all", "--json"]);
    assert_eq!(overview["working_forms"], done["working_forms"]);
    let remaining = read(&["overview", "--json"]);
    assert!(
        remaining["working_forms"]["records"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(remaining["working_forms"]["completed"], 1);
    assert!(!dir.join("authorization.toml").exists());
    assert!(!dir.join("resolution.toml").exists());
}

#[test]
fn working_form_sources_refresh_and_unreadable_records_remain_visible() {
    let fixture = Fixture::new();
    fixture.war(&["create", "A prototype", "--item", "Deliver a result"]);
    let dir = std::fs::read_dir(fixture.0.join("docs/tickets"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let manifest_path = dir.join("manifest.toml");
    let manifest: toml::Value =
        toml::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
    let id = manifest["id"].as_str().unwrap();
    let server = Server::start(&fixture);
    let before = server.data();
    let record = &before["snapshot"]["working_forms"]["records"][0];
    assert_eq!(record["id"], id);
    let sources = record["sources"].as_array().unwrap();
    for source in sources {
        let source = source.as_str().unwrap();
        let link = before["snapshot"]["links"][source]
            .as_str()
            .unwrap()
            .replacen("/source/", "/api/source/", 1);
        let response = server.request("GET", &link, &server.host, "");
        assert!(response.starts_with("HTTP/1.1 200"));
        assert_eq!(
            response.split_once("\r\n\r\n").unwrap().1.as_bytes(),
            std::fs::read(fixture.0.join(source)).unwrap()
        );
    }
    let intent = sources[1].as_str().unwrap();
    let path = fixture.0.join(intent);
    let mut contents = std::fs::read_to_string(&path).unwrap();
    contents.push_str("\nA newly observed context note.\n");
    std::fs::write(&path, contents).unwrap();
    let refreshed =
        server.until(|v| v["snapshot"]["record_digest"] != before["snapshot"]["record_digest"]);
    assert_ne!(
        refreshed["snapshot"]["working_forms"]["records"][0]["source_digest"],
        record["source_digest"]
    );
    let page = server.request("GET", "/api/progress", &server.host, "");
    let page: serde_json::Value =
        serde_json::from_str(page.split_once("\r\n\r\n").unwrap().1).unwrap();
    assert_eq!(
        page["working_forms"],
        refreshed["snapshot"]["working_forms"]
    );
    std::fs::write(&manifest_path, "not valid TOML = [").unwrap();
    let broken = server.until(|v| v["snapshot"]["working_forms"]["coverage_complete"] == false);
    assert!(
        !broken["snapshot"]["working_forms"]["diagnostics"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(broken["snapshot"]["legacy"]["total"], 1);
    assert_eq!(broken["snapshot"]["working_forms"]["total"], 0);
}

#[cfg(unix)]
#[test]
fn working_form_nonregular_source_is_reported_without_waiting_for_a_writer() {
    let fixture = Fixture::new();
    fixture.war(&["create", "A prototype", "--item", "Deliver a result"]);
    let dir = std::fs::read_dir(fixture.0.join("docs/tickets"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let intent = dir.join("atoms/10-intent.md");
    std::fs::remove_file(&intent).unwrap();
    assert!(
        Command::new("mkfifo")
            .arg(&intent)
            .status()
            .unwrap()
            .success()
    );
    let mut child = Command::new(env!("CARGO_BIN_EXE_war"))
        .args(["progress", "--snapshot", "--json"])
        .current_dir(&fixture.0)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            break;
        }
        if start.elapsed() > Duration::from_secs(4) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("blocked on a FIFO");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["result"]["working_forms"]["coverage_complete"], false);
    assert_eq!(value["result"]["working_forms"]["total"], 0);
    assert!(value["result"]["working_forms"]["diagnostics"][0]["message"].is_string());
}
/// `war progress --serve`, which is `war ui` opened at Progress
/// (OW-WAR-0116): every `/api/` call carries the session token.
struct Server {
    child: Child,
    host: String,
    token: String,
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
            token: String::new(),
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
        let url = result["result"]["url"].as_str().unwrap();
        let rest = url.trim_start_matches("http://");
        server.host = rest.split('/').next().unwrap().to_string();
        server.token = rest
            .split("#t=")
            .nth(1)
            .unwrap()
            .split('&')
            .next()
            .unwrap()
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
            "{method} {path} HTTP/1.1\r\nHost: {host}\r\nAuthorization: Bearer {}\r\n{extra}\r\n",
            self.token
        )
        .unwrap();
        let mut result = String::new();
        stream.read_to_string(&mut result).unwrap();
        result
    }
    fn data(&self) -> serde_json::Value {
        let result = self.request("GET", "/api/snapshot", &self.host, "");
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
    // The page carries no data and no inline script: a record cannot
    // inject into it, because nothing from a record is in it.
    let html = server.request("GET", "/", &server.host, "");
    assert!(!html.contains("globalThis.injected"));
    assert!(!html.contains("<script>"));
    assert!(html.contains("Content-Security-Policy: default-src 'none'; script-src 'self'"));
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
        // A foreign Host is DNS rebinding, refused before the token is read.
        ("GET", "/api/progress", "attacker.example", "", "400"),
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
    let url = &snapshot["snapshot"]["links"]["note.md"]
        .as_str()
        .unwrap()
        .replacen("/source/", "/api/source/", 1);
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
    roadmap["nodes"][0]["documents"] = serde_json::json!(["../outside.md"]);
    std::fs::write(&config, serde_json::to_vec(&roadmap).unwrap()).unwrap();
    let unsafe_link = capture();
    assert!(!unsafe_link.status.success());
    assert!(String::from_utf8_lossy(&unsafe_link.stdout).contains("without traversal"));
    roadmap["nodes"][0]["documents"] = serde_json::json!([]);
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

#[test]
fn roadmap_document_link_serves_exact_source_and_invalid_refresh_stays_stale() {
    let fixture = Fixture::new();
    let path = fixture.0.join("docs/roadmap/view.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        fixture.0.join("architecture.md"),
        "# Architecture\nExact local source.\n",
    )
    .unwrap();
    std::fs::write(&path, r#"{"schema":"oh.war/roadmap-view/v1","title":"Test","nodes":[{"id":"feature","title":"Feature","outcome":"Scope","warrants":["VIEW-WAR-0001"],"documents":["architecture.md"]}]}"#).unwrap();
    let server = Server::start(&fixture);
    let before = server.data();
    let link = before["snapshot"]["links"]["architecture.md"]
        .as_str()
        .unwrap()
        .replacen("/source/", "/api/source/", 1);
    let response = server.request("GET", &link, &server.host, "");
    assert!(response.starts_with("HTTP/1.1 200"));
    assert_eq!(
        response.split_once("\r\n\r\n").unwrap().1,
        "# Architecture\nExact local source.\n"
    );
    std::fs::write(&path, "{broken roadmap").unwrap();
    let stale = server.until(|data| data["error"].is_string());
    assert_eq!(
        stale["snapshot"]["record_digest"],
        before["snapshot"]["record_digest"]
    );
    assert!(stale["error"].as_str().unwrap().contains("Roadmap"));
}
