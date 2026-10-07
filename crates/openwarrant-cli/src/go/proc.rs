// SPDX-License-Identifier: Apache-2.0
//! A performer process `war evidence go` started (OW-WAR-0148 M15): its own
//! process group, its packet on stdin, its answer read from stdout under a
//! cap, and a kill that reaches everything it started.
//!
//! The bounds are `war evidence perform`'s (OW-WAR-0131): the group kill is
//! the deadline, an answer past the cap is no answer, and output that has not
//! arrived a few seconds after the group is gone is output this run does not
//! have.

use std::io::{Read as _, Write as _};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use camino::Utf8Path;

/// A Stage Submission is a page of JSON; an answer past this is no answer.
const STDOUT_CAP: u64 = 8 * 1024 * 1024;
const STDERR_TAIL: usize = 16 * 1024;
const READ_GRACE: Duration = Duration::from_secs(5);

/// A running performer.
pub struct Running {
    child: Child,
    pub pid: u32,
    pub started: Instant,
    output: Receiver<(String, Vec<u8>)>,
    capped: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

/// How a performer ended.
#[derive(Debug)]
pub struct Ended {
    /// `None` when it was killed (deadline, cap, cancellation).
    pub status: Option<std::process::ExitStatus>,
    pub stdout: String,
    pub stderr_tail: String,
    pub oversized: bool,
    pub seconds: u64,
}

impl Ended {
    /// The last non-blank stderr line, for a note.
    #[must_use]
    pub fn last_stderr(&self) -> Option<&str> {
        self.stderr_tail.lines().rfind(|l| !l.trim().is_empty())
    }

    /// How it exited, as a person says it.
    #[must_use]
    pub fn how(&self) -> String {
        match self.status {
            None => "was killed".to_owned(),
            Some(s) => {
                #[cfg(unix)]
                if let Some(sig) = std::os::unix::process::ExitStatusExt::signal(&s) {
                    return format!("was killed by signal {sig}");
                }
                match s.code() {
                    Some(0) => "exited 0".to_owned(),
                    Some(c) => format!("exited {c}"),
                    None => "exited".to_owned(),
                }
            }
        }
    }
}

/// Start `argv` in `cwd`, in a process group of its own, with `stdin` written
/// to it and `env` set.
pub fn spawn(
    argv: &[String],
    cwd: &Utf8Path,
    stdin: &[u8],
    env: &[(String, String)],
) -> Result<Running, String> {
    let Some((program, args)) = argv.split_first() else {
        return Err("the argv is empty".to_owned());
    };
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in env {
        command.env(k, v);
    }
    #[cfg(unix)]
    std::os::unix::process::CommandExt::process_group(&mut command, 0);
    let mut child = command
        .spawn()
        .map_err(|e| format!("could not run {program}: {e}"))?;
    let pid = child.id();
    if let Some(mut pipe) = child.stdin.take() {
        // A performer that exits before reading is reported by its status.
        let bytes = stdin.to_vec();
        std::thread::spawn(move || {
            let _ = pipe.write_all(&bytes);
        });
    }
    let stdout = child.stdout.take().ok_or("no stdout pipe")?;
    let mut stderr = child.stderr.take().ok_or("no stderr pipe")?;
    let (tx, rx) = std::sync::mpsc::channel();
    let capped = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = std::sync::Arc::clone(&capped);
    std::thread::spawn(move || {
        let mut out = String::new();
        let _ = stdout.take(STDOUT_CAP + 1).read_to_string(&mut out);
        if out.len() as u64 > STDOUT_CAP {
            flag.store(true, std::sync::atomic::Ordering::Release);
        }
        let mut err = Vec::new();
        let _ = stderr.read_to_end(&mut err);
        let _ = tx.send((out, err));
    });
    Ok(Running {
        child,
        pid,
        started: Instant::now(),
        output: rx,
        capped,
    })
}

impl Running {
    /// `Some(status)` once it has exited, without waiting.
    pub fn exited(&mut self) -> Option<std::process::ExitStatus> {
        self.child.try_wait().ok().flatten()
    }

    /// Whether it wrote past the cap on stdout (it is then killed).
    #[must_use]
    pub fn flooded(&self) -> bool {
        self.capped.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Kill its process group and reap it.
    pub fn kill(&mut self) {
        crate::perform::kill_group(&mut self.child);
    }

    /// Collect what it wrote, once it has exited or been killed.
    pub fn finish(mut self, status: Option<std::process::ExitStatus>) -> Ended {
        // A performer that exited and left a child writing is still a writer:
        // the group is killed before its output is read.
        let gone = crate::perform::group_gone_within(self.pid, Duration::from_millis(200));
        if !gone {
            crate::perform::kill_group(&mut self.child);
        }
        let (out, err) = self.output.recv_timeout(READ_GRACE).unwrap_or_default();
        let oversized = out.len() as u64 > STDOUT_CAP || self.flooded();
        let tail_start = err.len().saturating_sub(STDERR_TAIL);
        Ended {
            status,
            stdout: if oversized { String::new() } else { out },
            stderr_tail: String::from_utf8_lossy(&err[tail_start..]).into_owned(),
            oversized,
            seconds: self.started.elapsed().as_secs(),
        }
    }
}

/// Run `argv` to completion within `bound`, with `stdin`: for an external
/// executor's dispatch and poll, which answer at once. `Err` says why there
/// is no answer.
pub fn run_bounded(
    argv: &[String],
    cwd: &Utf8Path,
    stdin: &[u8],
    env: &[(String, String)],
    bound: Duration,
) -> Result<Ended, String> {
    let mut running = spawn(argv, cwd, stdin, env)?;
    loop {
        if let Some(status) = running.exited() {
            return Ok(running.finish(Some(status)));
        }
        if running.flooded() || running.started.elapsed() > bound {
            running.kill();
            return Ok(running.finish(None));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// `argv` with each `{name}` replaced by its value; an unknown placeholder is
/// left as written.
#[must_use]
pub fn fill(argv: &[String], values: &[(&str, &str)]) -> Vec<String> {
    argv.iter()
        .map(|a| {
            let mut s = a.clone();
            for (k, v) in values {
                s = s.replace(&format!("{{{k}}}"), v);
            }
            s
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_fill_and_unknown_ones_stay() {
        let argv: Vec<String> = ["gh", "issue", "view", "{ref}", "{nope}"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        assert_eq!(
            fill(&argv, &[("ref", "https://x/1")]),
            ["gh", "issue", "view", "https://x/1", "{nope}"]
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_bounded_run_is_killed_at_its_bound_with_its_children() {
        let dir = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir()).unwrap();
        let argv: Vec<String> = ["sh", "-c", "sleep 30 & sleep 30"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        let started = Instant::now();
        let ended = run_bounded(&argv, &dir, b"", &[], Duration::from_millis(300)).unwrap();
        assert!(ended.status.is_none(), "killed, not exited");
        assert!(started.elapsed() < Duration::from_secs(10));
        let argv: Vec<String> = ["sh", "-c", "cat; echo done >&2"]
            .iter()
            .map(|s| (*s).to_owned())
            .collect();
        let ended = run_bounded(&argv, &dir, b"{\"a\":1}", &[], Duration::from_secs(10)).unwrap();
        assert!(ended.status.is_some_and(|s| s.success()));
        assert_eq!(ended.stdout, "{\"a\":1}");
        assert_eq!(ended.last_stderr(), Some("done"));
    }
}
