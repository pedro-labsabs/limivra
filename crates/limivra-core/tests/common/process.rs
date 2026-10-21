#![cfg(target_os = "linux")]

use std::{
    fs, io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

pub struct Fixture {
    root: PathBuf,
    executable: PathBuf,
}

impl Fixture {
    pub fn new(script: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "limivra-contract-process-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).expect("create isolated fixture directory");
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))
            .expect("restrict fixture directory permissions");

        let executable = root.join("fake-process");
        let script = format!(
            "#!/bin/sh\nset -eu\nexport LIMIVRA_FIXTURE_ROOT='{}'\n{}\n",
            root.display(),
            script
        );
        fs::write(&executable, script).expect("write fake process executable");
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700))
            .expect("make fake process executable");

        Self { root, executable }
    }

    pub fn path(&self) -> &Path {
        &self.executable
    }

    pub fn pid_file(&self, name: &str) -> PathBuf {
        assert!(
            !name.is_empty()
                && name
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'),
            "fixture PID marker name must be a simple filename"
        );
        self.root.join(name)
    }

    pub async fn wait_for_pid(&self, name: &str, timeout: Duration) -> io::Result<u32> {
        let marker = self.pid_file(name);
        tokio::time::timeout(timeout, async {
            loop {
                match fs::read_to_string(&marker) {
                    Ok(contents) => match contents.trim().parse::<u32>() {
                        Ok(pid) => return Ok(pid),
                        Err(_) => tokio::time::sleep(Duration::from_millis(1)).await,
                    },
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        tokio::time::sleep(Duration::from_millis(1)).await;
                    }
                    Err(error) => return Err(error),
                }
            }
        })
        .await
        .map_err(|_| io::Error::new(io::ErrorKind::TimedOut, "fixture PID marker timed out"))?
    }

    pub fn assert_reaped(&self, pid: u32) {
        assert!(
            !Path::new(&format!("/proc/{pid}")).exists(),
            "fixture process {pid} was not reaped"
        );
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}
