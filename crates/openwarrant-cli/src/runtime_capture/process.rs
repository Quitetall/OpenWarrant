// SPDX-License-Identifier: Apache-2.0
//! Shared bounded process transport. This supervises an explicit executable;
//! it does not authenticate that executable or establish a sandbox.
use openwarrant_core::document::runtime::ProviderFailure;
use serde_json::Value;
use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};
fn unavailable(e: impl ToString) -> ProviderFailure {
    ProviderFailure::Unavailable(e.to_string())
}
fn rejected(e: impl ToString) -> ProviderFailure {
    ProviderFailure::Rejected(e.to_string())
}
pub(crate) struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
impl Scratch {
    pub(crate) fn create(root: &std::path::Path, provider: &str) -> Result<Self, ProviderFailure> {
        let path = root.join(format!(
            "ow-{provider}-verify-{}",
            openwarrant_core::WarUuid::mint()
        ));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&path).map_err(unavailable)?;
        Ok(Self(path))
    }
    pub(crate) fn write(&self, name: &str, bytes: &[u8]) -> Result<PathBuf, ProviderFailure> {
        let path = self.0.join(name);
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        options
            .open(&path)
            .and_then(|mut f| f.write_all(bytes))
            .map_err(unavailable)?;
        Ok(path)
    }
}

fn drain(
    stream: impl Read + Send + 'static,
    limit: usize,
) -> mpsc::Receiver<std::io::Result<Vec<u8>>> {
    let (send, recv) = mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stream
            .take(limit as u64 + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = send.send(result);
    });
    recv
}
fn stop(child: &mut Child) {
    #[cfg(unix)]
    if let Some(pid) = i32::try_from(child.id())
        .ok()
        .and_then(rustix::process::Pid::from_raw)
    {
        let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
    }
    let _ = child.kill();
    let until = Instant::now() + Duration::from_millis(250);
    while Instant::now() < until {
        if !matches!(child.try_wait(), Ok(None)) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}
fn run(
    command: Command,
    timeout: Duration,
    limit: usize,
) -> Result<(bool, Vec<u8>), ProviderFailure> {
    run_with_stdin(command, timeout, limit, Stdio::null())
}

/// Input comes from a bounded local file, avoiding a blocking pipe write.
pub(super) fn run_with_stdin(
    command: Command,
    timeout: Duration,
    limit: usize,
    input: Stdio,
) -> Result<(bool, Vec<u8>), ProviderFailure> {
    let observed = observe_with_stdin(command, timeout, limit, input)?;
    Ok((observed.success, observed.stdout))
}

pub(crate) struct Completed {
    pub success: bool,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// Preserve both bounded streams so protocol adapters can distinguish a verdict
/// from a verifier unable to produce one. Transport itself assigns no verdict.
pub(crate) fn observe_with_stdin(
    mut command: Command,
    timeout: Duration,
    limit: usize,
    input: Stdio,
) -> Result<Completed, ProviderFailure> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command
        .stdin(input)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // A failed ETXTBSY exec did not start the verifier. Retry only that launch
    // condition, under one deadline shared with the subsequent actual execution.
    // Never retry a started program or turn inability to run into a verdict.
    let deadline = Instant::now() + timeout;
    let mut last_busy_error = None;
    let mut child = loop {
        if Instant::now() >= deadline {
            return Err(unavailable(format!(
                "native verifier startup deadline exceeded; {}; no result established",
                last_busy_error.as_deref().unwrap_or("no launch completed")
            )));
        }
        match command.spawn() {
            Ok(child) => break child,
            Err(error)
                if error.raw_os_error() == Some(rustix::io::Errno::TXTBSY.raw_os_error()) =>
            {
                last_busy_error = Some(error.to_string());
                std::thread::sleep(
                    Duration::from_millis(5)
                        .min(deadline.saturating_duration_since(Instant::now())),
                );
            }
            Err(error) => return Err(unavailable(error)),
        }
    };
    let out = drain(child.stdout.take().expect("piped stdout"), limit);
    let err = drain(child.stderr.take().expect("piped stderr"), limit);
    let mut stdout = None;
    let mut stderr = None;
    loop {
        for (recv, slot) in [(&out, &mut stdout), (&err, &mut stderr)] {
            if slot.is_none() {
                match recv.try_recv() {
                    Ok(Ok(bytes)) if bytes.len() <= limit => *slot = Some(bytes),
                    Ok(Ok(_)) => {
                        stop(&mut child);
                        return Err(rejected("native verifier output byte budget"));
                    }
                    Ok(Err(e)) => {
                        stop(&mut child);
                        return Err(unavailable(e));
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        stop(&mut child);
                        return Err(unavailable("native verifier output unavailable"));
                    }
                    Err(mpsc::TryRecvError::Empty) => {}
                }
            }
        }
        match child.try_wait() {
            Ok(Some(status)) if stdout.is_some() && stderr.is_some() => {
                return Ok(Completed {
                    success: status.success(),
                    stdout: stdout.expect("checked output"),
                    stderr: stderr.expect("checked error output"),
                });
            }
            Ok(_) => {}
            Err(e) => {
                stop(&mut child);
                return Err(unavailable(e));
            }
        }
        if Instant::now() >= deadline {
            stop(&mut child);
            return Err(unavailable(
                "native verifier deadline exceeded; no result established",
            ));
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

pub(crate) fn verify_response(
    command: Command,
    timeout: Duration,
    limit: usize,
    schema: &str,
) -> Result<Value, ProviderFailure> {
    let (success, bytes) = run(command, timeout, limit)?;
    decode_response(success, &bytes, schema)
}

pub(super) fn decode_response(
    success: bool,
    bytes: &[u8],
    schema: &str,
) -> Result<Value, ProviderFailure> {
    let response: Value = crate::sdk::wire::decode_value(bytes).map_err(rejected)?;
    if response["schema"] != schema {
        return Err(ProviderFailure::Unsupported(
            "unsupported native verifier response schema".into(),
        ));
    }
    if response["assurance"] != "not-established" {
        return Err(rejected(
            "native verifier cannot issue OpenWarrant assurance",
        ));
    }
    if response.as_object().is_none_or(|fields| {
        fields
            .keys()
            .any(|k| !["schema", "status", "assurance", "native", "reason"].contains(&k.as_str()))
    }) {
        return Err(rejected("unknown native response fields"));
    }
    let reason = response["reason"].as_str().filter(|s| !s.trim().is_empty());
    match response["status"].as_str() {
        Some("unavailable") if !success && reason.is_some() && response.get("native").is_none() => {
            Err(unavailable(reason.unwrap()))
        }
        Some("rejected") if !success && reason.is_some() && response.get("native").is_none() => {
            Err(rejected(reason.unwrap()))
        }
        Some("validated")
            if success && response.get("reason").is_none() && response.get("native").is_some() =>
        {
            Ok(response["native"].clone())
        }
        _ => Err(rejected(
            "contradictory native verifier status or exit code",
        )),
    }
}

#[cfg(all(test, target_os = "linux"))]
mod exec_contention_tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new(body: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "ow-exec-busy-{}",
                openwarrant_core::WarUuid::mint()
            ));
            fs::create_dir(&root).unwrap();
            let program = root.join("verifier.sh");
            fs::write(&program, format!("#!/bin/sh\n{body}\n")).unwrap();
            fs::set_permissions(&program, fs::Permissions::from_mode(0o700)).unwrap();
            Self(root)
        }
        fn writer(&self) -> fs::File {
            fs::OpenOptions::new()
                .write(true)
                .open(self.0.join("verifier.sh"))
                .unwrap()
        }
        fn command(&self) -> Command {
            Command::new(self.0.join("verifier.sh"))
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn transient_busy_exec_reaches_the_actual_program_after_writer_release() {
        let fixture = Fixture::new("/usr/bin/cat");
        let payload = fixture.0.join("request.txt");
        fs::write(&payload, "actual-verifier-input").unwrap();
        let writer = fixture.writer();
        let release = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            drop(writer);
        });
        let observed = observe_with_stdin(
            fixture.command(),
            Duration::from_secs(1),
            4096,
            Stdio::from(fs::File::open(payload).unwrap()),
        );
        release.join().unwrap();
        let observed = observed
            .unwrap_or_else(|e| panic!("busy executable must run after its writer closes: {e:?}"));
        assert!(observed.success);
        assert_eq!(observed.stdout, b"actual-verifier-input");
    }

    #[test]
    fn retained_writer_expires_as_unknown_without_any_fabricated_result() {
        let fixture = Fixture::new("printf 'must-not-execute'");
        let _writer = fixture.writer();
        let observed = observe_with_stdin(
            fixture.command(),
            Duration::from_millis(100),
            4096,
            Stdio::null(),
        );
        match observed {
            Err(ProviderFailure::Unavailable(why)) => assert!(
                why.contains("deadline") && why.contains("Text file busy"),
                "{why}"
            ),
            Err(other) => panic!("busy launch must remain an unavailable observation: {other:?}"),
            Ok(_) => panic!("an executable with a retained writer must not run"),
        }
    }

    #[test]
    fn exec_retry_and_running_child_share_one_deadline() {
        let fixture = Fixture::new("printf started > \"$0.started\"\nsleep 1.6\nprintf late");
        let writer = fixture.writer();
        let release = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(600));
            drop(writer);
        });
        let observed = observe_with_stdin(
            fixture.command(),
            Duration::from_secs(2),
            4096,
            Stdio::null(),
        );
        release.join().unwrap();
        assert!(
            fixture.0.join("verifier.sh.started").exists(),
            "the controlled program must actually start before its run deadline"
        );
        match observed {
            Err(ProviderFailure::Unavailable(why)) => {
                assert!(why.contains("native verifier deadline"), "{why}")
            }
            Err(other) => panic!("running timeout must remain unavailable: {other:?}"),
            Ok(_) => panic!("startup retry must not reset the execution time allowance"),
        }
    }

    #[test]
    fn nonbusy_spawn_errors_remain_the_original_unavailable_observation() {
        let fixture = Fixture::new("printf unused");
        fs::remove_file(fixture.0.join("verifier.sh")).unwrap();
        let observed = observe_with_stdin(
            fixture.command(),
            Duration::from_secs(1),
            4096,
            Stdio::null(),
        );
        match observed {
            Err(ProviderFailure::Unavailable(why)) => {
                assert!(why.contains("No such file"), "{why}")
            }
            Err(other) => panic!("missing executable must remain unavailable: {other:?}"),
            Ok(_) => panic!("missing executable must not yield a result"),
        }
    }
}
