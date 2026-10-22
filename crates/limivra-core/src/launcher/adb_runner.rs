//! Bounded, explicitly targeted ADB clients. No raw output is logged or retained.
use super::*;
use std::{
    path::PathBuf,
    process::Stdio,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt},
    process::{Child, Command},
};

const OUTPUT_LIMIT: usize = 16 * 1024;
pub struct AdbLauncherRunner {
    adb: PathBuf,
    target: String,
}
impl AdbLauncherRunner {
    /// Resolve an existing system binary without installing or starting a daemon.
    pub fn system(target: &str) -> Result<Self, LaunchError> {
        let path = std::env::var_os("PATH").ok_or(LaunchError::UnsafeConfiguration)?;
        for dir in std::env::split_paths(&path).filter(|dir| dir.is_absolute()) {
            let adb = dir.join("adb");
            if adb.is_file() {
                return Self::new(adb, target);
            }
        }
        Err(LaunchError::UnsafeConfiguration)
    }
    pub fn new(adb: PathBuf, target: &str) -> Result<Self, LaunchError> {
        if !adb.is_absolute() || !adb.is_file() || !valid_target(target) {
            return Err(LaunchError::UnsafeConfiguration);
        }
        Ok(Self {
            adb,
            target: target.into(),
        })
    }
}
fn valid_target(target: &str) -> bool {
    !target.is_empty()
        && target.len() <= 255
        && !target.starts_with('-')
        && target
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._:-".contains(&c))
}
// Moving a child into this guard keeps cancellation safe even if the caller
// drops the run future. Drop kills only this child and delegates its reap.
struct OwnedChild(Option<Child>);
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.start_kill();
            if let Ok(runtime) = tokio::runtime::Handle::try_current() {
                runtime.spawn(async move {
                    let _ = child.wait().await;
                });
            }
        }
    }
}
async fn read_bounded<R: AsyncRead + Unpin>(
    mut stream: R,
    count: Arc<AtomicUsize>,
) -> Result<Vec<u8>, LaunchError> {
    let mut data = Vec::new();
    let mut chunk = [0u8; 1024];
    loop {
        let n = stream
            .read(&mut chunk)
            .await
            .map_err(|_| LaunchError::CommandFailed)?;
        if n == 0 {
            return Ok(data);
        }
        if count.fetch_add(n, Ordering::Relaxed) + n > OUTPUT_LIMIT {
            return Err(LaunchError::OutputLimit);
        }
        data.extend_from_slice(&chunk[..n]);
    }
}
impl LauncherRunner for AdbLauncherRunner {
    fn run<'a>(
        &'a self,
        target: &'a str,
        request: LauncherRequest,
    ) -> Pin<Box<dyn Future<Output = Result<LauncherReply, LaunchError>> + Send + 'a>> {
        Box::pin(async move {
            if target != self.target {
                return Err(LaunchError::UnsafeConfiguration);
            }
            let mut args: Vec<String> = ["-s", target, "shell"]
                .into_iter()
                .map(str::to_owned)
                .collect();
            match &request {
                LauncherRequest::Resolve { package } => {
                    if AllowedApp::from_package(package.as_str()).is_none() {
                        return Err(LaunchError::NotAllowed);
                    }
                    args.extend(
                        [
                            "cmd",
                            "package",
                            "resolve-activity",
                            "--brief",
                            "-a",
                            "android.intent.action.MAIN",
                            "-c",
                            "android.intent.category.LAUNCHER",
                            "-p",
                            package.as_str(),
                        ]
                        .into_iter()
                        .map(str::to_owned),
                    );
                }
                LauncherRequest::Start {
                    component,
                    display_id,
                } => {
                    if AllowedApp::from_package(component.package()).is_none() {
                        return Err(LaunchError::NotAllowed);
                    }
                    if *display_id == 0 {
                        return Err(LaunchError::DisplayDenied);
                    }
                    // ADB joins shell arguments remotely. Preserve literal '$' in
                    // inner class names; validation excludes single quotes.
                    let component = if component.as_str().contains('$') {
                        format!("'{}'", component.as_str())
                    } else {
                        component.as_str().to_owned()
                    };
                    args.extend([
                        "am".into(),
                        "start".into(),
                        "--display".into(),
                        display_id.to_string(),
                        "-n".into(),
                        component,
                    ]);
                }
                LauncherRequest::Displays => {
                    let mut all_args = args.clone();
                    all_args.extend(
                        ["cmd", "display", "get-displays", "--ids-only"]
                            .into_iter()
                            .map(str::to_owned),
                    );
                    let mut virtual_args = all_args.clone();
                    virtual_args.extend(["--type".into(), "VIRTUAL".into()]);
                    // Query type first: a newly appearing display in the second
                    // query remains untyped until the next observation. An ID
                    // that disappeared between queries is excluded entirely.
                    let virtuals = parse_ids(&self.execute(virtual_args).await?)?;
                    let all = parse_ids(&self.execute(all_args).await?)?;
                    if all.is_empty() {
                        return Err(LaunchError::UnexpectedReply);
                    }
                    return Ok(LauncherReply::Displays(
                        all.into_iter()
                            .map(|id| ObservedDisplay {
                                id,
                                is_virtual: virtuals.contains(&id),
                            })
                            .collect(),
                    ));
                }
            }
            let stdout = self.execute(args).await?;
            match request {
                LauncherRequest::Resolve { package } => {
                    let mut lines: Vec<_> = stdout
                        .lines()
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .collect();
                    if lines == ["No activity found"] {
                        return Ok(LauncherReply::NotInstalled);
                    }
                    if lines.len() == 2 && brief_header(lines[0]) {
                        lines.remove(0);
                    }
                    if lines.len() != 1 {
                        return Err(LaunchError::UnexpectedReply);
                    }
                    let component =
                        ComponentName::parse(lines[0]).map_err(|_| LaunchError::UnexpectedReply)?;
                    if component.package() != package.as_str() {
                        return Err(LaunchError::UnexpectedReply);
                    }
                    Ok(LauncherReply::Resolved(component))
                }
                LauncherRequest::Start { component, .. } => {
                    let mut starting = false;
                    for line in stdout.lines().map(str::trim).filter(|s| !s.is_empty()) {
                        let expected = format!("cmp={}", component.as_str());
                        if line.starts_with("Starting: Intent {") && line.ends_with('}')
                            && line.split_whitespace().any(|token| token == expected) {
                            starting = true;
                        } else if ![
                            "Warning: Activity not started, intent has been delivered to currently running top-most instance.",
                            "Warning: Activity not started, its current task has been brought to the front",
                        ].contains(&line) {
                            return Err(LaunchError::UnexpectedReply);
                        }
                    }
                    if starting {
                        Ok(LauncherReply::Accepted)
                    } else {
                        Err(LaunchError::UnexpectedReply)
                    }
                }
                LauncherRequest::Displays => unreachable!("display query handled above"),
            }
        })
    }
}
impl AdbLauncherRunner {
    async fn execute(&self, args: Vec<String>) -> Result<String, LaunchError> {
        let child = Command::new(&self.adb)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| LaunchError::CommandFailed)?;
        let mut owned = OwnedChild(Some(child));
        let child = owned.0.as_mut().expect("owned child");
        let stdout = child.stdout.take().expect("piped stdout");
        let stderr = child.stderr.take().expect("piped stderr");
        let count = Arc::new(AtomicUsize::new(0));
        let result = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::try_join!(
                read_bounded(stdout, count.clone()),
                read_bounded(stderr, count),
                async { child.wait().await.map_err(|_| LaunchError::CommandFailed) }
            )
        })
        .await;
        let (stdout, stderr, status) = match result {
            Ok(Ok(value)) => value,
            failure => {
                let child = owned.0.as_mut().expect("owned child");
                let _ = child.kill().await;
                let _ = child.wait().await;
                return Err(match failure {
                    Err(_) => LaunchError::Timeout,
                    Ok(Err(error)) => error,
                    _ => unreachable!(),
                });
            }
        };
        owned.0.take(); // already reaped
        let stdout = String::from_utf8(stdout).map_err(|_| LaunchError::UnexpectedReply)?;
        let stderr = String::from_utf8(stderr).map_err(|_| LaunchError::UnexpectedReply)?;
        if let Some(error) = classify_error(&stdout, &stderr) {
            return Err(error);
        }
        if !status.success() {
            return Err(LaunchError::CommandFailed);
        }
        if !stderr.trim().is_empty() {
            return Err(LaunchError::UnexpectedReply);
        }
        Ok(stdout)
    }
}

fn classify_error(stdout: &str, stderr: &str) -> Option<LaunchError> {
    let text = format!("{stdout}\n{stderr}").to_ascii_lowercase();
    if text.contains("unauthorized") {
        Some(LaunchError::Unauthorized)
    } else if text.contains("device offline")
        || (text.contains("error: device") && text.contains("not found"))
    {
        Some(LaunchError::Offline)
    } else if text.contains("not allowed to launch on display")
        || text.contains("invalid display")
        || (text.contains("error: display") && text.contains("does not exist"))
    {
        Some(LaunchError::DisplayDenied)
    } else if text.contains("securityexception") || text.contains("permission denial") {
        Some(LaunchError::SecurityException)
    } else if text.contains("activitynotfound")
        || text.contains("error type 3")
        || text.contains("activity class") && text.contains("does not exist")
    {
        Some(LaunchError::ActivityNotFound)
    } else {
        None
    }
}
fn parse_ids(stdout: &str) -> Result<Vec<u32>, LaunchError> {
    let mut ids = Vec::new();
    for line in stdout.lines() {
        if line.is_empty() || !line.bytes().all(|b| b.is_ascii_digit()) {
            return Err(LaunchError::UnexpectedReply);
        }
        let id = line
            .parse::<u32>()
            .map_err(|_| LaunchError::UnexpectedReply)?;
        if ids.contains(&id) {
            return Err(LaunchError::UnexpectedReply);
        }
        ids.push(id);
    }
    Ok(ids)
}

// PackageManager --brief prints these five fields before the component.
// Do not accept arbitrary headers, additional fields or multiple activities.
fn brief_header(line: &str) -> bool {
    let fields: Vec<_> = line.split_whitespace().collect();
    if fields.len() != 5 {
        return false;
    }
    for (field, key) in fields.iter().zip([
        "priority=",
        "preferredOrder=",
        "match=",
        "specificIndex=",
        "isDefault=",
    ]) {
        let Some(value) = field.strip_prefix(key) else {
            return false;
        };
        let valid = match key {
            "match=" => value
                .strip_prefix("0x")
                .is_some_and(|hex| !hex.is_empty() && u32::from_str_radix(hex, 16).is_ok()),
            "isDefault=" => matches!(value, "true" | "false"),
            _ => value.parse::<i32>().is_ok(),
        };
        if !valid {
            return false;
        }
    }
    true
}
