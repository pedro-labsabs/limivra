//! Private Linux foreground ADB server. Requires an approved binary and protected
//! existing HOME/.android store; creates no credentials or alternative key store.
//! Always call shutdown (or controller cancel/disconnect) before dropping a runner.
use super::*;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
};
use tokio::{
    io::{AsyncRead, AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
    process::{Child, Command},
};

const LIMIT: usize = 8192;
struct Configuration {
    adb: PathBuf,
    home: PathBuf,
    runtime: PathBuf,
    socket: PathBuf,
    key: KeyIdentity,
}
// Metadata only: no key bytes, digest or credential derivative is retained.
// Replacement or in-place changes require a new runner and fresh pairing.
#[derive(PartialEq, Eq)]
struct KeyIdentity {
    device: u64,
    inode: u64,
    size: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}
#[derive(Default)]
struct Server {
    child: Option<Child>,
    owns_socket: bool,
    checked: bool,
}
/// No raw subprocess output or pairing credentials are exposed by this runner.
/// The approved key's identity/metadata are pinned until this runner is dropped;
/// replacement or in-place change requires a new runner and fresh pairing.
/// Preflight checks are not an OS lock: the caller must protect the store from
/// concurrent modification while the external ADB process is using it.
#[derive(Clone)]
pub struct PrivateAdbRunner {
    config: Arc<Configuration>,
    server: Arc<Mutex<Server>>,
}
impl PrivateAdbRunner {
    pub fn new(adb: PathBuf, home: PathBuf, runtime: PathBuf) -> Result<Self, WifiError> {
        if !adb.is_absolute() || !home.is_absolute() || !runtime.is_absolute() {
            return Err(WifiError::UnsafeConfiguration);
        }
        let adb = std::fs::canonicalize(adb).map_err(|_| WifiError::UnsafeConfiguration)?;
        let metadata = std::fs::metadata(&adb).map_err(|_| WifiError::UnsafeConfiguration)?;
        if !metadata.is_file()
            || metadata.permissions().mode() & 0o111 == 0
            || metadata.permissions().mode() & 0o022 != 0
        {
            return Err(WifiError::UnsafeConfiguration);
        }
        secure_directory(&home)?;
        secure_directory(&runtime)?;
        secure_directory(&home.join(".android"))?;
        secure_store(&home.join(".android"))?;
        let key = secure_key(&home)?;
        // Canonical paths pin parent symlinks before any subprocess is launched.
        let home = std::fs::canonicalize(home).map_err(|_| WifiError::UnsafeConfiguration)?;
        let runtime = std::fs::canonicalize(runtime).map_err(|_| WifiError::UnsafeConfiguration)?;
        let socket = runtime.join("adb.sock");
        if socket.as_os_str().len() >= 100 || std::fs::symlink_metadata(&socket).is_ok() {
            return Err(WifiError::UnsafeConfiguration);
        }
        Ok(Self {
            config: Arc::new(Configuration {
                adb,
                home,
                runtime,
                socket,
                key,
            }),
            server: Arc::new(Mutex::new(Server::default())),
        })
    }
    fn preflight(&self) -> Result<(), WifiError> {
        secure_directory(&self.config.runtime)?;
        secure_directory(&self.config.home)?;
        secure_store(&self.config.home.join(".android"))?;
        if secure_key(&self.config.home)? != self.config.key {
            return Err(WifiError::UnsafeConfiguration);
        }
        Ok(())
    }
    fn command(&self) -> Result<Command, WifiError> {
        // Every launch, including help, daemon startup/restart and clients,
        // rechecks the required key immediately before building the command.
        self.preflight()?;
        let mut command = Command::new(&self.config.adb);
        command
            .env_clear()
            .env("HOME", &self.config.home)
            .env("LC_ALL", "C")
            .env("ADB_MDNS", "0")
            .env("ADB_MDNS_AUTO_CONNECT", "0")
            .env("ADB_USB", "0")
            .env("ADB_EMU", "0")
            .current_dir(&self.config.runtime)
            .kill_on_drop(true);
        Ok(command)
    }
    fn socket_argument(&self) -> String {
        format!("localfilesystem:{}", self.config.socket.display())
    }
    async fn ensure_server(
        &self,
        state: &mut Server,
        deadline: Instant,
        cancel: &Cancellation,
        abandoned: &Cancellation,
    ) -> Result<(), WifiError> {
        self.preflight()?;
        if let Some(child) = state.child.as_mut() {
            if child.try_wait().map_err(|_| WifiError::Io)?.is_some() {
                return Err(WifiError::Offline);
            }
            return Ok(());
        }
        if std::fs::symlink_metadata(&self.config.socket).is_ok() {
            return Err(WifiError::UnsafeConfiguration);
        }
        if !state.checked {
            let mut help = self.command()?;
            help.arg("help");
            let (out, _) = execute(help, None, deadline, cancel, abandoned, None).await?;
            let text = std::str::from_utf8(&out).map_err(|_| WifiError::DependencyUnsupported)?;
            if !text.contains("-L") || !text.contains("pair") || !text.contains("nodaemon") {
                return Err(WifiError::DependencyUnsupported);
            }
            state.checked = true;
        }
        let child = self
            .command()?
            .args(["-L", &self.socket_argument(), "server", "nodaemon"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| WifiError::Io)?;
        state.child = Some(child);
        state.owns_socket = true;
        loop {
            if state
                .child
                .as_mut()
                .unwrap()
                .try_wait()
                .map_err(|_| WifiError::Io)?
                .is_some()
            {
                return Err(WifiError::DependencyUnsupported);
            }
            tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(WifiError::Cancelled),
                _ = abandoned.cancelled() => return Err(WifiError::Cancelled),
                _ = tokio::time::sleep_until(deadline) => return Err(WifiError::Timeout),
                result = UnixStream::connect(&self.config.socket) => {
                    if result.is_ok() { return Ok(()); }
                }
            }
            tokio::select! {
                biased;
                _ = cancel.cancelled() => return Err(WifiError::Cancelled),
                _ = abandoned.cancelled() => return Err(WifiError::Cancelled),
                _ = tokio::time::sleep_until(deadline) => return Err(WifiError::Timeout),
                _ = tokio::time::sleep(Duration::from_millis(5)) => {}
            }
        }
    }
    async fn run_owned(
        &self,
        request: AdbRequest,
        deadline: Instant,
        cancel: Cancellation,
        abandoned: Cancellation,
    ) -> Result<AdbReply, WifiError> {
        let mut state = tokio::select! {
            biased;
            _ = cancel.cancelled() => return Err(WifiError::Cancelled),
            _ = abandoned.cancelled() => return Err(WifiError::Cancelled),
            _ = tokio::time::sleep_until(deadline) => return Err(WifiError::Timeout),
            guard = self.server.lock() => guard,
        };
        let result = async {
            if (request.kind == RequestKind::Pair) != request.code.is_some() {
                return Err(WifiError::InvalidCode);
            }
            self.ensure_server(&mut state, deadline, &cancel, &abandoned)
                .await?;
            let mut command = self.command()?;
            command.args(["-L", &self.socket_argument()]);
            let endpoint = request.endpoint.argument();
            match request.kind {
                RequestKind::Pair => {
                    command.args(["pair", &endpoint]);
                }
                RequestKind::Connect => {
                    command.args(["connect", &endpoint]);
                }
                RequestKind::Probe => {
                    command.args(["-s", &endpoint, "get-state"]);
                }
                RequestKind::Disconnect => {
                    command.args(["disconnect", &endpoint]);
                }
            }
            let (out, err) = execute(
                command,
                request.code,
                deadline,
                &cancel,
                &abandoned,
                Some((request.kind, &endpoint)),
            )
            .await?;
            if state
                .child
                .as_mut()
                .unwrap()
                .try_wait()
                .map_err(|_| WifiError::Io)?
                .is_some()
            {
                return Err(WifiError::Offline);
            }
            parse_reply(request.kind, &endpoint, &out, &err)
        }
        .await;
        if result.is_err() {
            self.stop_server(&mut state).await?;
        }
        result
    }
    async fn stop_server(&self, state: &mut Server) -> Result<(), WifiError> {
        // ADB clients can restart a dead daemon. Stop by our private socket too,
        // never by the global/default server, then reap the foreground handle.
        let protocol = if state.owns_socket {
            if let Ok(mut stop) = self.command() {
                stop.args(["-L", &self.socket_argument(), "kill-server"]);
                execute(
                    stop,
                    None,
                    Instant::now() + Duration::from_secs(2),
                    &Cancellation::default(),
                    &Cancellation::default(),
                    None,
                )
                .await
                .map(|_| ())
            } else {
                // Unsafe storage must not launch even a cleanup client: only
                // kill/wait the existing owned handle, then verify the socket.
                Ok(())
            }
        } else {
            Ok(())
        };
        force_reap_server(state).await?;
        protocol.map_err(|_| WifiError::CleanupFailed)?;
        if state.owns_socket {
            match tokio::time::timeout(
                Duration::from_millis(200),
                UnixStream::connect(&self.config.socket),
            )
            .await
            {
                Ok(Err(_)) => {}
                _ => return Err(WifiError::CleanupFailed),
            }
            match std::fs::symlink_metadata(&self.config.socket) {
                Ok(metadata) if metadata.file_type().is_socket() => {
                    std::fs::remove_file(&self.config.socket)
                        .map_err(|_| WifiError::CleanupFailed)?;
                }
                Ok(_) => return Err(WifiError::CleanupFailed),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => return Err(WifiError::CleanupFailed),
            }
            state.owns_socket = false;
        }
        Ok(())
    }
}
impl AdbRunner for PrivateAdbRunner {
    async fn run(
        &self,
        request: AdbRequest,
        deadline: Instant,
        cancellation: Cancellation,
    ) -> Result<AdbReply, WifiError> {
        // The supervisor owns child handles independently of the caller's future.
        // Dropping that future signals abandonment; kill/wait still run to completion.
        let runner = self.clone();
        let abandoned = Cancellation::default();
        let _on_drop = CancelOnDrop(abandoned.clone());
        let task = tokio::spawn(async move {
            runner
                .run_owned(request, deadline, cancellation, abandoned)
                .await
        });
        task.await.map_err(|_| WifiError::CleanupFailed)?
    }
    async fn shutdown(&self) -> Result<(), WifiError> {
        let mut state = self.server.lock().await;
        self.stop_server(&mut state).await
    }
}
struct CancelOnDrop(Cancellation);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.cancel();
    }
}

fn secure_key(home: &Path) -> Result<KeyIdentity, WifiError> {
    let key = std::fs::symlink_metadata(home.join(".android/adbkey"))
        .map_err(|_| WifiError::UnsafeConfiguration)?;
    let uid = std::fs::metadata("/proc/self")
        .map_err(|_| WifiError::UnsafeConfiguration)?
        .uid();
    if !key.is_file() || key.uid() != uid || key.permissions().mode() & 0o077 != 0 {
        return Err(WifiError::UnsafeConfiguration);
    }
    Ok(KeyIdentity {
        device: key.dev(),
        inode: key.ino(),
        size: key.len(),
        modified: (key.mtime(), key.mtime_nsec()),
        changed: (key.ctime(), key.ctime_nsec()),
    })
}

fn secure_directory(path: &Path) -> Result<(), WifiError> {
    let m = std::fs::symlink_metadata(path).map_err(|_| WifiError::UnsafeConfiguration)?;
    let uid = std::fs::metadata("/proc/self")
        .map_err(|_| WifiError::UnsafeConfiguration)?
        .uid();
    if !m.is_dir() || m.uid() != uid || m.permissions().mode() & 0o077 != 0 {
        return Err(WifiError::UnsafeConfiguration);
    }
    Ok(())
}
fn secure_store(path: &Path) -> Result<(), WifiError> {
    secure_directory(path)?;
    for entry in std::fs::read_dir(path).map_err(|_| WifiError::UnsafeConfiguration)? {
        let entry = entry.map_err(|_| WifiError::UnsafeConfiguration)?;
        let m =
            std::fs::symlink_metadata(entry.path()).map_err(|_| WifiError::UnsafeConfiguration)?;
        if m.is_dir() {
            secure_store(&entry.path())?;
        } else if !m.is_file()
            || m.uid()
                != std::fs::metadata("/proc/self")
                    .map_err(|_| WifiError::UnsafeConfiguration)?
                    .uid()
            || m.permissions().mode() & 0o077 != 0
        {
            return Err(WifiError::UnsafeConfiguration);
        }
    }
    Ok(())
}
async fn force_reap_server(state: &mut Server) -> Result<(), WifiError> {
    if let Some(child) = state.child.as_mut() {
        if child
            .try_wait()
            .map_err(|_| WifiError::CleanupFailed)?
            .is_none()
        {
            child.start_kill().map_err(|_| WifiError::CleanupFailed)?;
        }
        child.wait().await.map_err(|_| WifiError::CleanupFailed)?;
        state.child = None;
    }
    Ok(())
}
async fn read_bounded(mut reader: impl AsyncRead + Unpin) -> Result<Vec<u8>, WifiError> {
    let mut output = Vec::new();
    let mut chunk = [0; 1024];
    loop {
        let count = reader.read(&mut chunk).await.map_err(|_| WifiError::Io)?;
        if count == 0 {
            return Ok(output);
        }
        if output.len() + count > LIMIT {
            return Err(WifiError::OutputLimit);
        }
        output.extend_from_slice(&chunk[..count]);
    }
}
async fn execute(
    mut command: Command,
    code: Option<PairingCode>,
    deadline: Instant,
    cancel: &Cancellation,
    abandoned: &Cancellation,
    context: Option<(RequestKind, &str)>,
) -> Result<(Vec<u8>, Vec<u8>), WifiError> {
    if cancel.is_cancelled() || abandoned.is_cancelled() {
        return Err(WifiError::Cancelled);
    }
    if Instant::now() >= deadline {
        return Err(WifiError::Timeout);
    }
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| WifiError::Io)?;
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let io = async {
        let write = async {
            if let Some(code) = code {
                stdin.write_all(&code.0).await.map_err(|_| WifiError::Io)?;
                stdin.write_all(b"\n").await.map_err(|_| WifiError::Io)?;
            }
            stdin.shutdown().await.map_err(|_| WifiError::Io)
        };
        let wait = async { child.wait().await.map_err(|_| WifiError::Io) };
        let (status, (), out, err) =
            tokio::try_join!(wait, write, read_bounded(stdout), read_bounded(stderr))?;
        if !status.success() {
            if has_unauthorized(&out) || has_unauthorized(&err) {
                return Err(WifiError::Unauthorized);
            }
            if context.is_some_and(|(kind, endpoint)| recoverable_reply(kind, endpoint, &out, &err))
            {
                return Err(WifiError::Offline);
            }
            return Err(WifiError::CommandFailed);
        }
        Ok((out, err))
    };
    let result = tokio::select! {
        biased;
        _ = cancel.cancelled() => Err(WifiError::Cancelled),
        _ = abandoned.cancelled() => Err(WifiError::Cancelled),
        _ = tokio::time::sleep_until(deadline) => Err(WifiError::Timeout),
        result = io => result,
    };
    if result.is_err() {
        if child
            .try_wait()
            .map_err(|_| WifiError::CleanupFailed)?
            .is_none()
        {
            child.start_kill().map_err(|_| WifiError::CleanupFailed)?;
        }
        child.wait().await.map_err(|_| WifiError::CleanupFailed)?;
    }
    result
}
fn has_unauthorized(bytes: &[u8]) -> bool {
    let text = String::from_utf8_lossy(bytes).to_ascii_lowercase();
    text.contains("unauthorized") || text.contains("authentication failed")
}
fn parse_reply(
    kind: RequestKind,
    endpoint: &str,
    out: &[u8],
    err: &[u8],
) -> Result<AdbReply, WifiError> {
    if has_unauthorized(out) || has_unauthorized(err) {
        return Ok(AdbReply::Unauthorized);
    }
    if recoverable_reply(kind, endpoint, out, err) {
        return Err(WifiError::Offline);
    }
    let text = std::str::from_utf8(out)
        .map_err(|_| WifiError::UnexpectedReply)?
        .trim();
    if !err.is_empty() {
        return Err(WifiError::UnexpectedReply);
    }
    match kind {
        RequestKind::Pair => {
            let text = text.strip_prefix("Enter pairing code: ").unwrap_or(text);
            let prefix = format!("Successfully paired to {endpoint} [guid=");
            if text.strip_prefix(&prefix).is_some_and(|guid| {
                guid.ends_with(']') && guid.len() > 1 && !guid.contains(['\n', '\r'])
            }) {
                Ok(AdbReply::Paired)
            } else {
                Err(WifiError::UnexpectedReply)
            }
        }
        RequestKind::Connect
            if text == format!("connected to {endpoint}")
                || text == format!("already connected to {endpoint}") =>
        {
            Ok(AdbReply::Connected)
        }
        RequestKind::Probe if text == "device" => Ok(AdbReply::Device),
        RequestKind::Probe if text == "offline" => Ok(AdbReply::Offline),
        RequestKind::Disconnect if text == format!("disconnected {endpoint}") => {
            Ok(AdbReply::Disconnected)
        }
        _ => Err(WifiError::UnexpectedReply),
    }
}

/// Recognize only single, endpoint-bound backend failures. Unknown text, extra
/// diagnostics, a different target, and pairing/authentication errors stay closed.
fn recoverable_reply(kind: RequestKind, endpoint: &str, out: &[u8], err: &[u8]) -> bool {
    let (Ok(out), Ok(err)) = (std::str::from_utf8(out), std::str::from_utf8(err)) else {
        return false;
    };
    let (out, err) = (out.trim(), err.trim());
    let text = match (out.is_empty(), err.is_empty()) {
        (true, false) => err,
        (false, true) => out,
        _ => return false,
    };
    match kind {
        RequestKind::Probe => {
            text == "error: device offline"
                || text == format!("error: device '{endpoint}' not found")
        }
        RequestKind::Connect => {
            text == format!("failed to connect to '{endpoint}': Connection refused")
        }
        _ => false,
    }
}
