use std::{
    ffi::OsString,
    path::Path,
    process::{Output, Stdio},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::Command,
};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProcessError {
    #[error("subprocess spawn failed")]
    Spawn,
    #[error("subprocess timed out")]
    Timeout,
    #[error("subprocess output exceeded its limit")]
    OutputLimit,
    #[error("subprocess I/O failed")]
    Io,
}

pub async fn run_bounded(
    executable: &Path,
    args: &[OsString],
    timeout: Duration,
    per_stream_limit: usize,
) -> Result<Output, ProcessError> {
    let deadline = tokio::time::Instant::now() + timeout;
    let mut command = Command::new(executable);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    command.process_group(0);
    let mut child = command.spawn().map_err(|_| ProcessError::Spawn)?;
    let mut process_group = ProcessGroupGuard::new(child.id());

    let stdout = child.stdout.take().ok_or(ProcessError::Io)?;
    let stderr = child.stderr.take().ok_or(ProcessError::Io)?;
    let streams = tokio::select! {
        biased;
        _ = tokio::time::sleep_until(deadline) => {
            terminate_and_reap(&mut child, &mut process_group).await;
            return Err(ProcessError::Timeout);
        }
        result = async {
            tokio::try_join!(
                read_bounded(stdout, per_stream_limit),
                read_bounded(stderr, per_stream_limit),
            )
        } => result,
    };

    let (stdout, stderr) = match streams {
        Ok(streams) => streams,
        Err(error) => {
            terminate_and_reap(&mut child, &mut process_group).await;
            return Err(error);
        }
    };

    let status = tokio::select! {
        biased;
        _ = tokio::time::sleep_until(deadline) => {
            terminate_and_reap(&mut child, &mut process_group).await;
            return Err(ProcessError::Timeout);
        }
        result = child.wait() => match result {
            Ok(status) => status,
            Err(_) => {
                terminate_and_reap(&mut child, &mut process_group).await;
                return Err(ProcessError::Io);
            }
        },
    };

    process_group.disarm();

    Ok(Output {
        status,
        stdout,
        stderr,
    })
}

async fn read_bounded<R: AsyncRead + Unpin>(
    mut reader: R,
    limit: usize,
) -> Result<Vec<u8>, ProcessError> {
    let mut output = Vec::new();
    let mut buffer = [0u8; 8192];
    loop {
        let remaining = limit.saturating_sub(output.len());
        let read_len = buffer.len().min(remaining.saturating_add(1));
        let count = reader
            .read(&mut buffer[..read_len])
            .await
            .map_err(|_| ProcessError::Io)?;
        if count == 0 {
            return Ok(output);
        }
        if count > remaining {
            return Err(ProcessError::OutputLimit);
        }
        output.extend_from_slice(&buffer[..count]);
    }
}

async fn terminate_and_reap(
    child: &mut tokio::process::Child,
    process_group: &mut ProcessGroupGuard,
) {
    process_group.terminate();
    process_group.disarm();
    let _ = child.start_kill();
    let _ = child.wait().await;
}

struct ProcessGroupGuard {
    id: Option<u32>,
    armed: bool,
}

impl ProcessGroupGuard {
    fn new(id: Option<u32>) -> Self {
        Self { id, armed: true }
    }

    fn terminate(&self) {
        #[cfg(unix)]
        if self.armed {
            if let Some(id) = self.id {
                // SAFETY: run_bounded placed its child in a fresh process group led by this PID.
                unsafe {
                    libc::kill(-(id as libc::pid_t), libc::SIGKILL);
                }
            }
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for ProcessGroupGuard {
    fn drop(&mut self) {
        self.terminate();
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::{ffi::OsStrExt, fs::PermissionsExt},
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
    const FIXTURE_ENV_MARKER: &[u8] = b"LIMIVRA_PROCESS_FIXTURE_ROOT=";

    fn wait_for_fixture_marker(
        root: &Path,
        mut read_environment: impl FnMut() -> Option<Vec<u8>>,
        mut wait: impl FnMut(),
    ) -> bool {
        for attempt in 0..10 {
            let belongs_to_fixture = read_environment().is_some_and(|environment| {
                environment.split(|byte| *byte == 0).any(|entry| {
                    entry
                        .strip_prefix(FIXTURE_ENV_MARKER)
                        .is_some_and(|value| value == root.as_os_str().as_bytes())
                })
            });
            if belongs_to_fixture {
                return true;
            }
            if attempt < 9 {
                wait();
            }
        }
        false
    }

    #[test]
    fn fixture_marker_check_retries_after_transient_missing_environment() {
        let root = Path::new("/tmp/limivra-fixture-marker-retry");
        let marker = [FIXTURE_ENV_MARKER, root.as_os_str().as_bytes()].concat();
        let mut reads = 0;
        let mut waits = 0;

        let found = wait_for_fixture_marker(
            root,
            || {
                reads += 1;
                if reads == 1 {
                    None
                } else {
                    Some(marker.clone())
                }
            },
            || waits += 1,
        );

        assert!(found, "a transiently unavailable environ should be retried");
        assert_eq!(reads, 2);
        assert_eq!(waits, 1);
    }

    fn signal_fixture_process(pid: libc::pid_t, root: &Path) {
        if pid <= 0 {
            return;
        }

        // Pin this process identity before reading /proc so PID reuse cannot
        // redirect the eventual signal to an unrelated process.
        let pidfd = unsafe { libc::syscall(libc::SYS_pidfd_open, pid, 0) };
        if pidfd < 0 {
            return;
        }
        let pidfd = pidfd as libc::c_int;

        let belongs_to_fixture = wait_for_fixture_marker(
            root,
            || fs::read(format!("/proc/{pid}/environ")).ok(),
            || std::thread::sleep(Duration::from_millis(5)),
        );

        if belongs_to_fixture {
            // SAFETY: pidfd pins the process identity, and its environment
            // contains this fixture's exact private root before signaling.
            unsafe {
                libc::syscall(
                    libc::SYS_pidfd_send_signal,
                    pidfd,
                    libc::SIGKILL,
                    std::ptr::null::<libc::siginfo_t>(),
                    0,
                );
            }
        }
        // SAFETY: pidfd is the descriptor returned by pidfd_open above.
        unsafe {
            libc::close(pidfd);
        }
    }

    struct Fixture {
        root: PathBuf,
        executable: PathBuf,
        args: Vec<OsString>,
    }

    impl Fixture {
        fn new(body: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "limivra-process-unit-{}-{}",
                std::process::id(),
                NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&root).unwrap();
            fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
            let script = format!(
                "set -eu\nexport LIMIVRA_PROCESS_FIXTURE_ROOT='{}'\nprintf '%s\\n' \"$$\" > '{}/pid'\n{}\n",
                root.display(),
                root.display(),
                body
            );
            Self {
                root,
                executable: PathBuf::from("/bin/sh"),
                args: vec![OsString::from("-c"), OsString::from(script)],
            }
        }

        async fn wait_for_pid(&self) -> u32 {
            self.wait_for_named_pid("pid").await
        }

        async fn wait_for_named_pid(&self, name: &str) -> u32 {
            tokio::time::timeout(Duration::from_secs(3), async {
                loop {
                    if let Ok(value) = fs::read_to_string(self.root.join(name)) {
                        if let Ok(pid) = value.trim().parse() {
                            return pid;
                        }
                    }
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            })
            .await
            .expect("process did not publish PID marker")
        }

        async fn assert_reaped(&self, pid: u32) {
            tokio::time::timeout(Duration::from_secs(3), async {
                while Path::new(&format!("/proc/{pid}")).exists() {
                    tokio::time::sleep(Duration::from_millis(5)).await;
                }
            })
            .await
            .expect("process was not reaped");
        }
    }

    #[tokio::test]
    async fn fixture_drop_does_not_kill_an_unrelated_pid_marker() {
        let fixture = Fixture::new("exit 0");
        let mut unrelated = std::process::Command::new("/bin/sleep")
            .arg("30")
            .spawn()
            .expect("start unrelated process");
        fs::write(fixture.root.join("pid"), unrelated.id().to_string())
            .expect("write unrelated PID to fixture marker");

        drop(fixture);
        tokio::time::sleep(Duration::from_millis(30)).await;
        let survived = unrelated
            .try_wait()
            .expect("inspect unrelated process")
            .is_none();
        if survived {
            unrelated
                .kill()
                .expect("stop unrelated process after assertion");
        }
        let _ = unrelated.wait().expect("reap unrelated process");

        assert!(survived, "fixture teardown must not signal a foreign PID");
    }

    #[tokio::test]
    async fn fixture_drop_kills_its_marked_child_and_child_handle_reaps_it() {
        let fixture = Fixture::new("exec /bin/sleep 30");
        let mut child = std::process::Command::new(&fixture.executable)
            .args(&fixture.args)
            .env("LIMIVRA_PROCESS_FIXTURE_ROOT", &fixture.root)
            .spawn()
            .expect("start fixture child");
        fixture.wait_for_pid().await;

        drop(fixture);
        let exited = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                match child.try_wait().expect("inspect fixture child") {
                    Some(status) => break status,
                    None => tokio::time::sleep(Duration::from_millis(5)).await,
                }
            }
        })
        .await;
        let status = match exited {
            Ok(status) => status,
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("fixture teardown did not stop its own child");
            }
        };
        let _ = child.wait().expect("reap fixture child after teardown");

        assert!(
            !status.success(),
            "fixture child should be killed on teardown"
        );
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            for name in ["pid", "descendant"] {
                if let Ok(contents) = fs::read_to_string(self.root.join(name)) {
                    if let Ok(pid) = contents.trim().parse::<libc::pid_t>() {
                        signal_fixture_process(pid, &self.root);
                    }
                }
            }
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[tokio::test]
    async fn nonzero_exit_status_is_preserved() {
        let output = run_bounded(
            Path::new("/bin/sh"),
            &[OsString::from("-c"), OsString::from("exit 7")],
            Duration::from_secs(2),
            1024,
        )
        .await
        .unwrap();
        assert_eq!(output.status.code(), Some(7));
    }

    #[tokio::test]
    async fn timeout_kills_and_reaps_child() {
        let fixture = Fixture::new("exec /bin/sleep 30");
        let executable = fixture.executable.clone();
        let args = fixture.args.clone();
        let task = tokio::spawn(async move {
            run_bounded(&executable, &args, Duration::from_millis(500), 1024).await
        });
        let pid = fixture.wait_for_pid().await;
        assert_eq!(task.await.unwrap().unwrap_err(), ProcessError::Timeout);
        fixture.assert_reaped(pid).await;
    }

    #[tokio::test]
    async fn timeout_kills_and_reaps_descendant_holding_pipe() {
        let fixture = Fixture::new(
            "(/bin/sleep 30) & printf '%s\\n' \"$!\" > \"$LIMIVRA_PROCESS_FIXTURE_ROOT/descendant\"; wait",
        );
        let executable = fixture.executable.clone();
        let args = fixture.args.clone();
        let task = tokio::spawn(async move {
            run_bounded(&executable, &args, Duration::from_millis(500), 1024).await
        });
        let descendant = fixture.wait_for_named_pid("descendant").await;
        assert_eq!(task.await.unwrap().unwrap_err(), ProcessError::Timeout);
        fixture.assert_reaped(descendant).await;
    }

    #[tokio::test]
    async fn output_limit_kills_and_reaps_descendant() {
        let fixture = Fixture::new(
            "(/bin/sleep 30) & printf '%s\\n' \"$!\" > \"$LIMIVRA_PROCESS_FIXTURE_ROOT/descendant\"; /usr/bin/head -c 1048577 /dev/zero; wait",
        );
        let result = run_bounded(
            &fixture.executable,
            &fixture.args,
            Duration::from_secs(5),
            1024 * 1024,
        )
        .await;
        assert_eq!(result.unwrap_err(), ProcessError::OutputLimit);
        let descendant = fixture.wait_for_named_pid("descendant").await;
        fixture.assert_reaped(descendant).await;
    }

    #[tokio::test]
    async fn oversized_stdout_or_stderr_kills_and_reaps_child() {
        for body in [
            "exec /usr/bin/head -c 1048577 /dev/zero",
            "exec /usr/bin/head -c 1048577 /dev/zero 1>&2",
        ] {
            let fixture = Fixture::new(body);
            let result = run_bounded(
                &fixture.executable,
                &fixture.args,
                Duration::from_secs(5),
                1024 * 1024,
            )
            .await;
            let error = result.unwrap_err();
            assert_eq!(error, ProcessError::OutputLimit);
            assert!(!format!("{error:?} {error}").contains("1048577"));
            fixture.assert_reaped(fixture.wait_for_pid().await).await;
        }
    }

    #[tokio::test]
    async fn cancellation_kills_and_reaps_child() {
        let fixture = Fixture::new("exec /bin/sleep 30");
        let executable = fixture.executable.clone();
        let args = fixture.args.clone();
        let task = tokio::spawn(async move {
            run_bounded(&executable, &args, Duration::from_secs(30), 1024).await
        });
        let marker = fixture.root.join("pid");
        let mut task = Some(task);
        let pid = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if let Ok(value) = fs::read_to_string(&marker) {
                    if let Ok(pid) = value.trim().parse::<u32>() {
                        return pid;
                    }
                }
                if task
                    .as_ref()
                    .is_some_and(tokio::task::JoinHandle::is_finished)
                {
                    let result = task.take().unwrap().await;
                    panic!("run_bounded exited before publishing the PID: {result:?}");
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
        })
        .await
        .expect("process did not publish PID marker");
        let task = task.expect("process task completed before cancellation");
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        fixture.assert_reaped(pid).await;
    }

    #[tokio::test]
    async fn cancellation_kills_and_reaps_descendant() {
        let fixture = Fixture::new(
            "(/bin/sleep 30) & printf '%s\\n' \"$!\" > \"$LIMIVRA_PROCESS_FIXTURE_ROOT/descendant\"; wait",
        );
        let executable = fixture.executable.clone();
        let args = fixture.args.clone();
        let task = tokio::spawn(async move {
            run_bounded(&executable, &args, Duration::from_secs(30), 1024).await
        });
        let descendant = fixture.wait_for_named_pid("descendant").await;
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        fixture.assert_reaped(descendant).await;
    }
}
