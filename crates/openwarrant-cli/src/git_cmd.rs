// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bounded, non-interactive execution for read-only Git plumbing commands.

use std::ffi::{OsStr, OsString};
use std::io::{self, Read, Write};
use std::path::Path;
use std::process::{Command, ExitStatus, Output, Stdio};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[cfg(windows)]
use process_wrap::std::JobObject;
#[cfg(unix)]
use process_wrap::std::ProcessGroup;
use process_wrap::std::{ChildWrapper, CommandWrap};

const GIT_TIMEOUT: Duration = Duration::from_secs(30);
const PROCESS_CLEANUP_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_STDERR_BYTES: usize = 1024 * 1024;
const WAIT_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Run Git with a bounded capture and no ambient repository or prompt state.
///
/// `max_stdout_bytes` is caller-owned because a command that returns one line
/// and a command that returns a frozen object have very different legitimate
/// output sizes. Stderr is diagnostic-only and is capped here at 1 MiB.
pub(crate) fn output(cwd: &Path, args: &[&str], max_stdout_bytes: usize) -> io::Result<Output> {
    output_command(
        git_command(cwd, args),
        max_stdout_bytes,
        MAX_STDERR_BYTES,
        GIT_TIMEOUT,
        "Git",
    )
}

/// Run one local command with process-tree cleanup, a shared deadline, and
/// bounded concurrent capture of both output streams.
pub(crate) fn output_command(
    command: Command,
    stdout_limit: usize,
    stderr_limit: usize,
    timeout: Duration,
    command_name: &'static str,
) -> io::Result<Output> {
    run_bounded(command, stdout_limit, stderr_limit, timeout, command_name)
}

/// Run one local command while supplying exact owned bytes on standard input.
///
/// Input, stdout, and stderr move concurrently. A child that fills stdout
/// before reading stdin therefore cannot deadlock this runner. Returning
/// success also proves the writer delivered every input byte and closed stdin.
pub(crate) fn output_command_with_input(
    command: Command,
    input: Vec<u8>,
    stdout_limit: usize,
    stderr_limit: usize,
    timeout: Duration,
    command_name: &'static str,
) -> io::Result<Output> {
    run_bounded_inner(
        command,
        Some(input),
        stdout_limit,
        stderr_limit,
        timeout,
        command_name,
    )
}

fn git_command(cwd: &Path, args: &[&str]) -> Command {
    let mut command = Command::new("git");

    // Git has many environment variables that can redirect the repository,
    // object database, config, executable path, replacement namespace, or
    // network transport. Removing the whole namespace is less brittle than a
    // hand-maintained subset. The few settings this runner needs are restored
    // below with fixed values.
    let inherited_names = std::env::vars_os().map(|(name, _)| name);
    for name in environment_names_to_remove(inherited_names) {
        command.env_remove(name);
    }
    command
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_PAGER", "cat")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GCM_INTERACTIVE", "Never")
        .env("SSH_ASKPASS_REQUIRE", "never")
        // Git's own diagnostics become stable input to the caller's error.
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Replacement refs and filesystem monitors are host-local overlays.
        // Neither may alter or execute while reading a named frozen object.
        .arg("--no-replace-objects")
        .args(["-c", "core.fsmonitor=false"])
        // Suppress configured credential UI as well as terminal prompting.
        .args(["-c", "credential.interactive=never"])
        .args(["-c", "core.askPass="])
        .arg("-C")
        .arg(cwd)
        .args(args);
    command
}

fn environment_names_to_remove(names: impl IntoIterator<Item = OsString>) -> Vec<OsString> {
    names
        .into_iter()
        .filter(|name| {
            is_git_environment_name(name)
                || os_str_eq_ignore_ascii_case(name, "GCM_INTERACTIVE")
                || os_str_eq_ignore_ascii_case(name, "SSH_ASKPASS")
                || os_str_eq_ignore_ascii_case(name, "SSH_ASKPASS_REQUIRE")
        })
        .collect()
}

fn is_git_environment_name(name: &OsStr) -> bool {
    name.to_string_lossy()
        .as_bytes()
        .get(..4)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"GIT_"))
}

fn os_str_eq_ignore_ascii_case(value: &OsStr, expected: &str) -> bool {
    value
        .to_string_lossy()
        .as_bytes()
        .eq_ignore_ascii_case(expected.as_bytes())
}

#[derive(Debug)]
struct Capture {
    bytes: Vec<u8>,
    exceeded_limit: bool,
}

fn drain_bounded(mut reader: impl Read, limit: usize) -> io::Result<Capture> {
    let mut bytes = Vec::new();
    let mut exceeded_limit = false;
    let mut allocation_failed = false;
    let mut buffer = [0_u8; 16 * 1024];

    loop {
        let read = match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(read) => read,
            Err(source) if source.kind() == io::ErrorKind::Interrupted => continue,
            Err(source) => return Err(source),
        };

        let keep = limit.saturating_sub(bytes.len()).min(read);
        if keep < read {
            exceeded_limit = true;
        }
        if keep > 0 && !allocation_failed {
            if bytes.try_reserve(keep).is_err() {
                // Keep draining after allocation refusal so the child cannot
                // deadlock on a full pipe. Report the fixed error at EOF.
                allocation_failed = true;
            } else {
                bytes.extend_from_slice(&buffer[..keep]);
            }
        }
    }

    if allocation_failed {
        return Err(io::Error::other(
            "could not allocate bounded command output buffer",
        ));
    }
    Ok(Capture {
        bytes,
        exceeded_limit,
    })
}

fn spawn_bounded_drain<R>(
    reader: R,
    limit: usize,
    thread_name: &'static str,
) -> io::Result<JoinHandle<io::Result<Capture>>>
where
    R: Read + Send + 'static,
{
    thread::Builder::new()
        .name(thread_name.to_owned())
        .spawn(move || drain_bounded(reader, limit))
        .map_err(|source| io_with_context("starting command output reader", source))
}

fn join_drain(
    handle: JoinHandle<io::Result<Capture>>,
    stream: &'static str,
) -> io::Result<Capture> {
    handle
        .join()
        .map_err(|_| io::Error::other(format!("command {stream} reader thread panicked")))?
        .map_err(|source| io_with_context(&format!("reading command {stream}"), source))
}

fn spawn_input_writer(
    mut writer: std::process::ChildStdin,
    input: Vec<u8>,
) -> io::Result<JoinHandle<io::Result<()>>> {
    thread::Builder::new()
        .name("bounded-stdin".to_owned())
        .spawn(move || {
            writer.write_all(&input)?;
            writer.flush()?;
            Ok(())
        })
        .map_err(|source| io_with_context("starting command input writer", source))
}

fn join_input_writer(handle: JoinHandle<io::Result<()>>) -> io::Result<()> {
    handle
        .join()
        .map_err(|_| io::Error::other("command stdin writer thread panicked"))?
        .map_err(|source| io_with_context("writing command stdin", source))
}

fn deadline_after(duration: Duration) -> io::Result<Instant> {
    Instant::now()
        .checked_add(duration)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "command timeout is too large"))
}

fn wait_for_exit_until(
    child: &mut dyn ChildWrapper,
    deadline: Instant,
) -> io::Result<Option<ExitStatus>> {
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(Some(status)),
            Ok(None) => {}
            Err(source) => return Err(io_with_context("waiting for command", source)),
        }

        let now = Instant::now();
        if now >= deadline {
            return Ok(None);
        }
        thread::sleep(WAIT_POLL_INTERVAL.min(deadline.saturating_duration_since(now)));
    }
}

fn io_threads_finished(
    readers: &[&JoinHandle<io::Result<Capture>>],
    writer: Option<&JoinHandle<io::Result<()>>>,
) -> bool {
    readers.iter().all(|reader| reader.is_finished()) && writer.is_none_or(JoinHandle::is_finished)
}

fn wait_for_io_until(
    readers: &[&JoinHandle<io::Result<Capture>>],
    writer: Option<&JoinHandle<io::Result<()>>>,
    deadline: Instant,
) -> bool {
    loop {
        if io_threads_finished(readers, writer) {
            return true;
        }
        let now = Instant::now();
        if now >= deadline {
            return false;
        }
        thread::sleep(WAIT_POLL_INTERVAL.min(deadline.saturating_duration_since(now)));
    }
}

fn kill_and_reap_until(child: &mut dyn ChildWrapper, deadline: Instant) -> io::Result<()> {
    let kill_error = child.start_kill().err();
    let wait_result = wait_for_exit_until(child, deadline);

    match wait_result {
        Ok(Some(_)) => {}
        Ok(None) => {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                "process did not exit after process-tree termination",
            ));
        }
        Err(source) => return Err(io_with_context("reaping process after termination", source)),
    }
    if let Some(source) = kill_error
        && !matches!(
            source.kind(),
            io::ErrorKind::InvalidInput | io::ErrorKind::NotFound
        )
    {
        return Err(io_with_context("terminating process tree", source));
    }
    Ok(())
}

fn kill_and_reap(child: &mut dyn ChildWrapper) -> io::Result<()> {
    kill_and_reap_until(child, deadline_after(PROCESS_CLEANUP_TIMEOUT)?)
}

fn terminate_process_tree_and_io(
    child: &mut dyn ChildWrapper,
    readers: &[&JoinHandle<io::Result<Capture>>],
    writer: Option<&JoinHandle<io::Result<()>>>,
) -> io::Result<()> {
    let deadline = deadline_after(PROCESS_CLEANUP_TIMEOUT)?;
    let process_cleanup = kill_and_reap_until(child, deadline);
    let io_cleanup = wait_for_io_until(readers, writer, deadline)
        .then_some(())
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::TimedOut,
                "command I/O threads did not close after process-tree termination",
            )
        });

    match (process_cleanup, io_cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(process), Ok(())) => Err(process),
        (Ok(()), Err(readers)) => Err(readers),
        (Err(process), Err(readers)) => Err(io::Error::new(
            process.kind(),
            format!("{process}; {readers}"),
        )),
    }
}

fn spawn_process_tree(command: Command) -> io::Result<Box<dyn ChildWrapper>> {
    let mut command = CommandWrap::from(command);
    #[cfg(unix)]
    command.wrap(ProcessGroup::leader());
    #[cfg(windows)]
    command.wrap(JobObject);
    command.spawn()
}

fn io_with_optional_cleanup(primary: io::Error, cleanup: io::Result<()>) -> io::Error {
    match cleanup {
        Ok(()) => primary,
        Err(cleanup) => io::Error::new(
            primary.kind(),
            format!("{primary}; process cleanup failed: {cleanup}"),
        ),
    }
}

fn io_with_context(context: &str, source: io::Error) -> io::Error {
    io::Error::new(source.kind(), format!("{context}: {source}"))
}

fn run_bounded(
    command: Command,
    stdout_limit: usize,
    stderr_limit: usize,
    timeout: Duration,
    command_name: &'static str,
) -> io::Result<Output> {
    run_bounded_inner(
        command,
        None,
        stdout_limit,
        stderr_limit,
        timeout,
        command_name,
    )
}

fn run_bounded_inner(
    mut command: Command,
    input: Option<Vec<u8>>,
    stdout_limit: usize,
    stderr_limit: usize,
    timeout: Duration,
    command_name: &'static str,
) -> io::Result<Output> {
    // Set these here as well as in `git_command`: tests exercise this generic
    // seam, and no future caller can accidentally revert to unbounded capture.
    command
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let deadline = deadline_after(timeout)?;
    let mut child = spawn_process_tree(command)
        .map_err(|source| io_with_context(&format!("starting {command_name}"), source))?;

    let input_writer = if let Some(input) = input {
        let stdin = match child.stdin().take() {
            Some(stdin) => stdin,
            None => {
                let cleanup = kill_and_reap(&mut *child);
                return Err(io_with_optional_cleanup(
                    io::Error::other(format!("{command_name} stdin pipe was not created")),
                    cleanup,
                ));
            }
        };
        match spawn_input_writer(stdin, input) {
            Ok(writer) => Some(writer),
            Err(source) => {
                let cleanup = kill_and_reap(&mut *child);
                return Err(io_with_optional_cleanup(source, cleanup));
            }
        }
    } else {
        None
    };

    let stdout = match child.stdout().take() {
        Some(stdout) => stdout,
        None => {
            let cleanup = terminate_process_tree_and_io(&mut *child, &[], input_writer.as_ref());
            return Err(io_with_optional_cleanup(
                io::Error::other(format!("{command_name} stdout pipe was not created")),
                cleanup,
            ));
        }
    };
    let stderr = match child.stderr().take() {
        Some(stderr) => stderr,
        None => {
            drop(stdout);
            let cleanup = terminate_process_tree_and_io(&mut *child, &[], input_writer.as_ref());
            return Err(io_with_optional_cleanup(
                io::Error::other(format!("{command_name} stderr pipe was not created")),
                cleanup,
            ));
        }
    };

    let stdout_reader = match spawn_bounded_drain(stdout, stdout_limit, "bounded-stdout") {
        Ok(reader) => reader,
        Err(source) => {
            drop(stderr);
            let cleanup = terminate_process_tree_and_io(&mut *child, &[], input_writer.as_ref());
            return Err(io_with_optional_cleanup(source, cleanup));
        }
    };
    let stderr_reader = match spawn_bounded_drain(stderr, stderr_limit, "bounded-stderr") {
        Ok(reader) => reader,
        Err(source) => {
            let cleanup = terminate_process_tree_and_io(
                &mut *child,
                &[&stdout_reader],
                input_writer.as_ref(),
            );
            return Err(io_with_optional_cleanup(source, cleanup));
        }
    };

    let readers = [&stdout_reader, &stderr_reader];
    let status = match wait_for_exit_until(&mut *child, deadline) {
        Ok(Some(status)) => status,
        Ok(None) => {
            let primary = io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "{command_name} command exceeded {} ms timeout; process-tree termination requested",
                    timeout.as_millis()
                ),
            );
            let cleanup =
                terminate_process_tree_and_io(&mut *child, &readers, input_writer.as_ref());
            return Err(io_with_optional_cleanup(primary, cleanup));
        }
        Err(primary) => {
            let cleanup =
                terminate_process_tree_and_io(&mut *child, &readers, input_writer.as_ref());
            return Err(io_with_optional_cleanup(primary, cleanup));
        }
    };
    if !wait_for_io_until(&readers, input_writer.as_ref(), deadline) {
        let primary = io::Error::new(
            io::ErrorKind::TimedOut,
            format!(
                "{command_name} output remained open past the shared {} ms timeout; process-tree termination requested",
                timeout.as_millis()
            ),
        );
        let cleanup = terminate_process_tree_and_io(&mut *child, &readers, input_writer.as_ref());
        return Err(io_with_optional_cleanup(primary, cleanup));
    }

    // Both readers are known finished before either blocking join. A helper
    // process retaining an inherited pipe therefore cannot hold this function.
    let stdout = join_drain(stdout_reader, "stdout");
    let stderr = join_drain(stderr_reader, "stderr");
    let input = input_writer.map(join_input_writer).transpose();

    let stdout = stdout?;
    let stderr = stderr?;
    input?;
    if stdout.exceeded_limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{command_name} stdout exceeded {stdout_limit}-byte limit"),
        ));
    }
    if stderr.exceeded_limit {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{command_name} stderr exceeded {stderr_limit}-byte limit"),
        ));
    }

    Ok(Output {
        status,
        stdout: stdout.bytes,
        stderr: stderr.bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::io::Cursor;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountedReader {
        inner: Cursor<Vec<u8>>,
        bytes_read: Arc<AtomicUsize>,
    }

    impl Read for CountedReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let read = self.inner.read(buffer)?;
            self.bytes_read.fetch_add(read, Ordering::Relaxed);
            Ok(read)
        }
    }

    #[test]
    fn bounded_capture_keeps_a_prefix_but_drains_the_entire_stream() {
        let input = vec![b'x'; 128 * 1024];
        let bytes_read = Arc::new(AtomicUsize::new(0));
        let capture = drain_bounded(
            CountedReader {
                inner: Cursor::new(input.clone()),
                bytes_read: Arc::clone(&bytes_read),
            },
            31,
        )
        .expect("drain");

        assert_eq!(capture.bytes, &input[..31]);
        assert!(capture.exceeded_limit);
        assert_eq!(bytes_read.load(Ordering::Relaxed), input.len());
    }

    #[test]
    fn exact_capture_limit_is_accepted() {
        let capture = drain_bounded(Cursor::new(b"1234"), 4).expect("drain");
        assert_eq!(capture.bytes, b"1234");
        assert!(!capture.exceeded_limit);
    }

    #[cfg(unix)]
    #[test]
    fn kill_and_reap_waits_for_child_exit() {
        let mut child = std::process::Command::new("sleep")
            .arg("100")
            .spawn()
            .expect("spawn");

        kill_and_reap(&mut child).expect("reap");
        assert!(child.try_wait().expect("try_wait").is_some());
    }

    #[test]
    fn every_git_environment_override_is_removed_before_safe_values_are_set() {
        let removals: BTreeSet<OsString> = environment_names_to_remove([
            OsString::from("PATH"),
            OsString::from("GIT_DIR"),
            OsString::from("git_config_key_0"),
            OsString::from("GIT_SSH_COMMAND"),
            OsString::from("GCM_INTERACTIVE"),
            OsString::from("SSH_ASKPASS"),
            OsString::from("SSH_ASKPASS_REQUIRE"),
        ])
        .into_iter()
        .collect();

        assert!(!removals.contains(OsStr::new("PATH")));
        for unsafe_name in [
            "GIT_DIR",
            "git_config_key_0",
            "GIT_SSH_COMMAND",
            "GCM_INTERACTIVE",
            "SSH_ASKPASS",
            "SSH_ASKPASS_REQUIRE",
        ] {
            assert!(removals.contains(OsStr::new(unsafe_name)), "{unsafe_name}");
        }
    }

    #[test]
    fn git_command_has_the_hardened_prefix_and_noninteractive_environment() {
        let command = git_command(Path::new("repo"), &["cat-file", "-t", "deadbeef"]);
        let args: Vec<&OsStr> = command.get_args().collect();
        assert_eq!(
            args,
            [
                "--no-replace-objects",
                "-c",
                "core.fsmonitor=false",
                "-c",
                "credential.interactive=never",
                "-c",
                "core.askPass=",
                "-C",
                "repo",
                "cat-file",
                "-t",
                "deadbeef",
            ]
            .map(OsStr::new)
        );

        let environment: Vec<(&OsStr, Option<&OsStr>)> = command.get_envs().collect();
        for (name, expected) in [
            ("GIT_NO_LAZY_FETCH", "1"),
            ("GIT_TERMINAL_PROMPT", "0"),
            ("GIT_PAGER", "cat"),
            ("GIT_OPTIONAL_LOCKS", "0"),
            ("GCM_INTERACTIVE", "Never"),
            ("SSH_ASKPASS_REQUIRE", "never"),
            ("LC_ALL", "C"),
        ] {
            assert!(
                environment
                    .iter()
                    .any(|(key, value)| *key == OsStr::new(name)
                        && *value == Some(OsStr::new(expected))),
                "missing {name}={expected}"
            );
        }
    }

    #[cfg(unix)]
    #[test]
    fn process_capture_drains_stdout_and_stderr_concurrently() {
        let mut command = Command::new("/bin/sh");
        command.args([
            "-c",
            "i=0; while [ \"$i\" -lt 4096 ]; do printf '0123456789abcdef0123456789abcdef\\n'; printf 'fedcba9876543210fedcba9876543210\\n' >&2; i=$((i + 1)); done",
        ]);

        let output = run_bounded(
            command,
            256 * 1024,
            256 * 1024,
            Duration::from_secs(5),
            "Git",
        )
        .expect("capture");
        assert!(output.status.success());
        assert_eq!(output.stdout.len(), 4096 * 33);
        assert_eq!(output.stderr.len(), 4096 * 33);
    }

    #[cfg(unix)]
    #[test]
    fn process_input_delivers_exact_owned_bytes() {
        let input = (0..256 * 1024)
            .map(|index| (index % 251) as u8)
            .collect::<Vec<_>>();
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "cat"]);

        let output = run_bounded_inner(
            command,
            Some(input.clone()),
            input.len(),
            16,
            Duration::from_secs(5),
            "test",
        )
        .expect("exact stdin");

        assert!(output.status.success());
        assert_eq!(output.stdout, input);
        assert!(output.stderr.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn process_input_and_output_move_concurrently_without_pipe_deadlock() {
        let input = vec![b'i'; 256 * 1024];
        let output_prefix = b"0123456789abcdef".repeat(16 * 1024);
        let mut command = Command::new("/bin/sh");
        command.args([
            "-c",
            "i=0; while [ \"$i\" -lt 16384 ]; do printf '0123456789abcdef'; i=$((i + 1)); done; cat",
        ]);

        let output = run_bounded_inner(
            command,
            Some(input.clone()),
            output_prefix.len() + input.len(),
            16,
            Duration::from_secs(5),
            "test",
        )
        .expect("concurrent stdin/stdout");

        assert!(output.status.success());
        assert_eq!(&output.stdout[..output_prefix.len()], output_prefix);
        assert_eq!(&output.stdout[output_prefix.len()..], input);
    }

    #[cfg(unix)]
    #[test]
    fn process_capture_reports_a_deterministic_stream_limit_error() {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "printf '0123456789'; printf 'abcdefghij' >&2"]);

        let error = run_bounded(command, 4, 10, Duration::from_secs(5), "Git")
            .expect_err("stdout must exceed its limit");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(error.to_string(), "Git stdout exceeded 4-byte limit");
    }

    #[cfg(unix)]
    #[test]
    fn process_timeout_kills_and_reaps_the_child() {
        let mut pid_file = std::env::temp_dir();
        pid_file.push(format!(
            "openwarrant-cli-git-cmd-{}.pid",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&pid_file);

        let mut command = Command::new("/bin/sh");
        let script = format!(
            "printf '%s\\n' $$ > {} ; while :; do :; done",
            pid_file.display()
        );
        command.args(["-c", &script]);
        let started = Instant::now();

        let error = run_bounded(command, 16, 16, Duration::from_millis(50), "Git")
            .expect_err("child must time out");
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert_eq!(
            error.to_string(),
            "Git command exceeded 50 ms timeout; process-tree termination requested"
        );
        assert!(started.elapsed() < Duration::from_secs(2));

        let pid = std::fs::read_to_string(&pid_file)
            .expect("pid")
            .trim()
            .parse::<u32>()
            .expect("pid parse");
        let still_running = std::process::Command::new("kill")
            .arg("-0")
            .arg(pid.to_string())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        assert!(!still_running);
        let _ = std::fs::remove_file(&pid_file);
    }

    #[cfg(unix)]
    #[test]
    fn process_timeout_kills_descendant_that_keeps_output_pipes_open() {
        let mut pid_file = std::env::temp_dir();
        pid_file.push(format!(
            "openwarrant-cli-git-cmd-descendant-{}.pid",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&pid_file);

        let mut command = Command::new("/bin/sh");
        command.args([
            "-c",
            "sleep 100 & printf '%s\\n' \"$!\" > \"$1\"; exit 0",
            "openwarrant-git-test",
            pid_file.to_str().expect("UTF-8 temp path"),
        ]);
        let started = Instant::now();

        let error = run_bounded(command, 16, 16, Duration::from_millis(100), "Git")
            .expect_err("descendant-held pipes must hit the shared deadline");
        assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        assert!(
            error.to_string().contains("Git output remained open"),
            "unexpected timeout: {error}"
        );
        assert!(started.elapsed() < Duration::from_secs(3));

        let pid = std::fs::read_to_string(&pid_file)
            .expect("descendant pid")
            .trim()
            .parse::<u32>()
            .expect("descendant pid parse");
        let process_stopped = (0..100).any(|_| {
            let running = std::process::Command::new("kill")
                .arg("-0")
                .arg(pid.to_string())
                .stderr(std::process::Stdio::null())
                .status()
                .is_ok_and(|status| status.success());
            if running {
                thread::sleep(Duration::from_millis(10));
            }
            !running
        });
        assert!(
            process_stopped,
            "descendant {pid} survived tree termination"
        );
        let _ = std::fs::remove_file(&pid_file);
    }

    #[cfg(unix)]
    #[test]
    fn process_capture_reports_stderr_limit_error() {
        let mut command = Command::new("/bin/sh");
        command.args(["-c", "printf '0123456789' >&2; printf 'abcdefghij'"]);

        let error = run_bounded(command, 10, 4, Duration::from_secs(5), "Git")
            .expect_err("stderr must exceed its limit");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData);
        assert_eq!(error.to_string(), "Git stderr exceeded 4-byte limit");
    }

    #[test]
    fn production_timeout_is_thirty_seconds() {
        assert_eq!(GIT_TIMEOUT, Duration::from_secs(30));
    }

    // Keep bounded drain coverage so a future refactor cannot silently replace
    // bounded drains with an unbounded `read_to_end` implementation.
    #[test]
    fn generic_drain_helper_is_bounded_too() {
        let handle =
            spawn_bounded_drain(Cursor::new(b"ok"), 1024 * 1024, "git-test").expect("spawn");
        let capture = join_drain(handle, "test").expect("join");
        assert_eq!(capture.bytes, b"ok");
        assert!(!capture.exceeded_limit);
    }
}
