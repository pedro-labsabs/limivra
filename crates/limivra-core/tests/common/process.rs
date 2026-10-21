#![cfg(target_os = "linux")]

use std::{
    fs, io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
    time::Duration,
};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

pub struct Fixture {
    root: PathBuf,
    executable: PathBuf,
    active_pids: Mutex<std::collections::HashSet<u32>>,
    reaped_pids: Mutex<std::collections::HashSet<u32>>,
    owned_children: Mutex<Vec<std::process::Child>>,
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
            "#!/bin/sh\nset -eu\nexport LIMIVRA_FIXTURE_ROOT='{}'\nprintf '%s\\n' \"$$\" > '{}/fixture-root-pid'\n{}\n",
            root.display(),
            root.display(),
            script
        );
        fs::write(&executable, script).expect("write fake process executable");
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o700))
            .expect("make fake process executable");

        Self {
            root,
            executable,
            active_pids: Mutex::new(std::collections::HashSet::new()),
            reaped_pids: Mutex::new(std::collections::HashSet::new()),
            owned_children: Mutex::new(Vec::new()),
        }
    }

    pub fn path(&self) -> &Path {
        &self.executable
    }

    // This helper is used by the session integration target; other targets
    // compile the shared module independently and therefore do not call it.
    #[allow(dead_code)]
    pub fn spawn(&self) -> io::Result<u32> {
        let child = std::process::Command::new(&self.executable).spawn()?;
        let pid = child.id();
        self.owned_children.lock().unwrap().push(child);
        self.active_pids.lock().unwrap().insert(pid);
        Ok(pid)
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
                        Ok(pid) => {
                            self.active_pids.lock().unwrap().insert(pid);
                            return Ok(pid);
                        }
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
        self.active_pids.lock().unwrap().remove(&pid);
        self.reaped_pids.lock().unwrap().insert(pid);
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let (Ok(pids), Ok(reaped_pids)) =
            (self.active_pids.get_mut(), self.reaped_pids.get_mut())
        {
            if let Ok(contents) = fs::read_to_string(self.root.join("fixture-root-pid")) {
                if let Ok(pid) = contents.trim().parse::<u32>() {
                    if !reaped_pids.contains(&pid) {
                        pids.insert(pid);
                    }
                }
            }
            for &pid in pids.iter() {
                // SAFETY: fixtures register only PIDs read from their own private markers.
                unsafe {
                    libc::kill(pid as libc::pid_t, libc::SIGKILL);
                }
            }
        }
        if let Ok(children) = self.owned_children.get_mut() {
            for child in children {
                if matches!(child.try_wait(), Ok(None)) {
                    let _ = child.kill();
                }
                let _ = child.wait();
            }
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}
