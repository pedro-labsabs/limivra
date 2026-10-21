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
pub(crate) enum ProcessError {
    #[error("subprocess spawn failed")]
    Spawn,
    #[error("subprocess timed out")]
    Timeout,
    #[error("subprocess output exceeded its limit")]
    OutputLimit,
    #[error("subprocess I/O failed")]
    Io,
}

pub(crate) async fn run_bounded(
    executable: &Path,
    args: &[OsString],
    timeout: Duration,
    per_stream_limit: usize,
) -> Result<Output, ProcessError> {
    let deadline = tokio::time::Instant::now() + timeout;
    let mut child = Command::new(executable)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| ProcessError::Spawn)?;

    let stdout = child.stdout.take().ok_or(ProcessError::Io)?;
    let stderr = child.stderr.take().ok_or(ProcessError::Io)?;
    let streams = tokio::select! {
        biased;
        _ = tokio::time::sleep_until(deadline) => {
            terminate_and_reap(&mut child).await;
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
            terminate_and_reap(&mut child).await;
            return Err(error);
        }
    };

    let status = tokio::select! {
        biased;
        _ = tokio::time::sleep_until(deadline) => {
            terminate_and_reap(&mut child).await;
            return Err(ProcessError::Timeout);
        }
        result = child.wait() => match result {
            Ok(status) => status,
            Err(_) => {
                terminate_and_reap(&mut child).await;
                return Err(ProcessError::Io);
            }
        },
    };

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

async fn terminate_and_reap(child: &mut tokio::process::Child) {
    let _ = child.start_kill();
    let _ = child.wait().await;
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::{
        fs,
        os::unix::fs::PermissionsExt,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

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
            let marker = root.join("pid");
            let script = format!(
                "set -eu\nprintf '%s\\n' \"$$\" > '{}'\n{}\n",
                marker.display(),
                body
            );
            Self {
                root,
                executable: PathBuf::from("/bin/sh"),
                args: vec![OsString::from("-c"), OsString::from(script)],
            }
        }

        async fn wait_for_pid(&self) -> u32 {
            tokio::time::timeout(Duration::from_secs(3), async {
                loop {
                    if let Ok(value) = fs::read_to_string(self.root.join("pid")) {
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

    impl Drop for Fixture {
        fn drop(&mut self) {
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
        let result = run_bounded(
            &fixture.executable,
            &fixture.args,
            Duration::from_millis(100),
            1024,
        )
        .await;
        assert_eq!(result.unwrap_err(), ProcessError::Timeout);
        fixture.assert_reaped(fixture.wait_for_pid().await).await;
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
}
