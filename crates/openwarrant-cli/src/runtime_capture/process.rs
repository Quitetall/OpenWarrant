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
    mut command: Command,
    timeout: Duration,
    limit: usize,
) -> Result<(bool, Vec<u8>), ProviderFailure> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(unavailable)?;
    let out = drain(child.stdout.take().expect("piped stdout"), limit);
    let err = drain(child.stderr.take().expect("piped stderr"), limit);
    let deadline = Instant::now() + timeout;
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
                return Ok((status.success(), stdout.expect("checked output")));
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
    let response: Value = crate::sdk::wire::decode_value(&bytes).map_err(rejected)?;
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
