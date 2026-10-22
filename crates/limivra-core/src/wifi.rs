//! Consented Wi-Fi ADB transport. No discovery or implicit target selection.
pub mod adb_runner;
use std::net::SocketAddrV4;
#[derive(Clone, PartialEq, Eq)]
pub struct LocalEndpoint(SocketAddrV4);
impl LocalEndpoint {
    pub fn parse(value: &str) -> Result<Self, WifiError> {
        let addr: SocketAddrV4 = value.parse().map_err(|_| WifiError::InvalidEndpoint)?;
        if !addr.ip().is_private() || addr.port() == 0 || addr.to_string() != value {
            return Err(WifiError::InvalidEndpoint);
        }
        Ok(Self(addr))
    }
    pub(crate) fn argument(&self) -> String {
        self.0.to_string()
    }
}
pub struct PairingCode([u8; 6]);

#[cfg(test)]
thread_local! {
    static LAST_PAIRING_CODE_DROP_WAS_ZEROIZED: std::cell::Cell<Option<bool>> = const { std::cell::Cell::new(None) };
}

impl PairingCode {
    pub fn parse(value: &str) -> Result<Self, WifiError> {
        if value.len() != 6 || !value.bytes().all(|b| b.is_ascii_digit()) {
            return Err(WifiError::InvalidCode);
        }
        let mut code = [0; 6];
        code.copy_from_slice(value.as_bytes());
        Ok(Self(code))
    }
}
impl std::fmt::Debug for PairingCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PairingCode([REDACTED])")
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum WifiError {
    #[error("Invalid local endpoint")]
    InvalidEndpoint,
    #[error("Invalid pairing code")]
    InvalidCode,
    #[error("Consent required")]
    NoConsent,
    #[error("Target not authorized")]
    WrongTarget,
    #[error("Not paired in this session")]
    NotPaired,
    #[error("Operation busy")]
    Busy,
    #[error("Invalid state")]
    InvalidState,
    #[error("Operation cancelled")]
    Cancelled,
    #[error("Operation timed out")]
    Timeout,
    #[error("Authorization rejected")]
    Unauthorized,
    #[error("Link unavailable")]
    Offline,
    #[error("Unexpected ADB reply")]
    UnexpectedReply,
    #[error("ADB command failed")]
    CommandFailed,
    #[error("Output limit exceeded")]
    OutputLimit,
    #[error("Unsupported dependency")]
    DependencyUnsupported,
    #[error("Unsafe ADB configuration")]
    UnsafeConfiguration,
    #[error("Process I/O failed")]
    Io,
    #[error("Cleanup failed")]
    CleanupFailed,
    #[error("Recovery exhausted")]
    RecoveryExhausted,
}

impl WifiError {
    // Keep link-loss admission and recovery retry decisions aligned. A new
    // backend message must be classified by the runner, never broaden this to
    // CommandFailed/UnexpectedReply or authorization/configuration failures.
    fn is_recoverable(self) -> bool {
        matches!(self, Self::Offline | Self::Timeout)
    }
}

impl std::fmt::Debug for LocalEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("LocalEndpoint([REDACTED])")
    }
}
impl Drop for PairingCode {
    fn drop(&mut self) {
        zeroize_pairing_buffer(&mut self.0);
        #[cfg(test)]
        LAST_PAIRING_CODE_DROP_WAS_ZEROIZED.with(|result| {
            result.set(Some(self.0.iter().all(|byte| *byte == 0)));
        });
    }
}

fn zeroize_pairing_buffer(buffer: &mut [u8]) {
    for byte in buffer {
        // SAFETY: `byte` is a valid, exclusively borrowed location in our owned secret buffer.
        // Volatile writes prevent the compiler from removing the final clearing operation.
        unsafe { std::ptr::write_volatile(byte, 0) };
    }
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}

#[cfg(test)]
mod pairing_code_tests {
    use super::{PairingCode, LAST_PAIRING_CODE_DROP_WAS_ZEROIZED};

    #[test]
    fn pairing_code_drop_zeroizes_its_owned_buffer() {
        LAST_PAIRING_CODE_DROP_WAS_ZEROIZED.with(|result| result.set(None));
        drop(PairingCode::parse("123456").expect("test pairing code should parse"));
        LAST_PAIRING_CODE_DROP_WAS_ZEROIZED.with(|result| {
            assert_eq!(result.get(), Some(true));
        });
    }
}

use std::{future::Future, sync::Mutex as StdMutex, time::Duration};
use tokio::{
    sync::{broadcast, watch, Mutex},
    time::Instant,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consent {
    Granted,
    Denied,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiState {
    Disconnected,
    Pairing,
    Connecting,
    Connected,
    Recovering,
    Failed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WifiEvent {
    StateChanged(WifiState),
    Error(WifiError),
    LinkLost,
    RecoveryAttempt(u8),
    Recovered,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestKind {
    Pair,
    Connect,
    Probe,
    Disconnect,
}
#[derive(Debug)]
pub struct AdbRequest {
    pub kind: RequestKind,
    pub endpoint: LocalEndpoint,
    pub code: Option<PairingCode>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdbReply {
    Paired,
    Connected,
    Device,
    Disconnected,
    Offline,
    Unauthorized,
}

/// Cancellation is sticky and can be observed without losing a wakeup.
#[derive(Clone, Debug)]
pub struct Cancellation(watch::Sender<bool>);
impl Default for Cancellation {
    fn default() -> Self {
        Self(watch::channel(false).0)
    }
}
impl Cancellation {
    pub fn cancel(&self) {
        self.0.send_replace(true);
    }
    pub fn is_cancelled(&self) -> bool {
        *self.0.borrow()
    }
    pub async fn cancelled(&self) {
        let mut rx = self.0.subscribe();
        loop {
            if *rx.borrow_and_update() {
                return;
            }
            if rx.changed().await.is_err() {
                return;
            }
        }
    }
}
pub trait AdbRunner: Send + Sync {
    fn run(
        &self,
        request: AdbRequest,
        deadline: Instant,
        cancellation: Cancellation,
    ) -> impl Future<Output = Result<AdbReply, WifiError>> + Send;
    fn shutdown(&self) -> impl Future<Output = Result<(), WifiError>> + Send;
}
struct Status {
    state: WifiState,
    paired: bool,
    recovery: bool,
    stopping: bool,
    generation: u64,
    cancellation: Cancellation,
    contacted: bool,
}
pub struct WifiController<R: AdbRunner> {
    runner: R,
    target: LocalEndpoint,
    status: StdMutex<Status>,
    operation: Mutex<()>,
    monitor_lock: Mutex<()>,
    cleanup: Mutex<()>,
    events: broadcast::Sender<WifiEvent>,
}
impl<R: AdbRunner> WifiController<R> {
    pub fn new(runner: R, allowed_target: LocalEndpoint) -> Self {
        Self {
            runner,
            target: allowed_target,
            status: StdMutex::new(Status {
                state: WifiState::Disconnected,
                paired: false,
                recovery: false,
                stopping: false,
                generation: 0,
                cancellation: Cancellation::default(),
                contacted: false,
            }),
            operation: Mutex::new(()),
            monitor_lock: Mutex::new(()),
            cleanup: Mutex::new(()),
            events: broadcast::channel(64).0,
        }
    }
    pub fn state(&self) -> WifiState {
        self.status.lock().unwrap().state
    }
    pub fn subscribe(&self) -> broadcast::Receiver<WifiEvent> {
        self.events.subscribe()
    }
    fn begin(&self) -> Result<Operation<'_, R>, WifiError> {
        let guard = self.operation.try_lock().map_err(|_| WifiError::Busy)?;
        let mut status = self.status.lock().unwrap();
        if status.stopping {
            return Err(WifiError::Busy);
        }
        if status.cancellation.is_cancelled() {
            return Err(WifiError::Cancelled);
        }
        status.generation += 1;
        Ok(Operation {
            controller: self,
            _guard: guard,
            token: status.cancellation.clone(),
            generation: status.generation,
            finished: false,
        })
    }
    fn transition(&self, op: &Operation<'_, R>, state: WifiState) -> Result<(), WifiError> {
        let mut status = self.status.lock().unwrap();
        if status.stopping || status.generation != op.generation || op.token.is_cancelled() {
            return Err(WifiError::Cancelled);
        }
        status.state = state;
        let _ = self.events.send(WifiEvent::StateChanged(state));
        Ok(())
    }
    async fn request(
        &self,
        op: &Operation<'_, R>,
        kind: RequestKind,
        endpoint: LocalEndpoint,
        code: Option<PairingCode>,
        deadline: Instant,
    ) -> Result<AdbReply, WifiError> {
        tokio::select! {
            biased;
            _ = op.token.cancelled() => Err(WifiError::Cancelled),
            _ = tokio::time::sleep_until(deadline) => Err(WifiError::Timeout),
            reply = self.runner.run(AdbRequest { kind, endpoint, code }, deadline, op.token.clone()) => reply,
        }
    }
    async fn finish(
        &self,
        op: &mut Operation<'_, R>,
        result: Result<(), WifiError>,
        state: WifiState,
    ) -> Result<(), WifiError> {
        let result = if result.is_err() {
            self.runner
                .shutdown()
                .await
                .map_err(|_| WifiError::CleanupFailed)
                .and(result)
        } else {
            result
        };
        let effective_state = if result.is_ok() {
            state
        } else {
            WifiState::Failed
        };
        let transition = self.transition(op, effective_state);
        if let Err(error) = result {
            let mut status = self.status.lock().unwrap();
            status.paired = false;
            status.recovery = false;
            let _ = self.events.send(WifiEvent::Error(error));
        }
        op.finished = true;
        transition.and(result)
    }
    pub async fn pair(
        &self,
        endpoint: LocalEndpoint,
        code: PairingCode,
        consent: Consent,
    ) -> Result<(), WifiError> {
        if consent != Consent::Granted {
            return Err(WifiError::NoConsent);
        }
        if endpoint.0.ip() != self.target.0.ip() {
            return Err(WifiError::WrongTarget);
        }
        let mut op = self.begin()?;
        if self.state() != WifiState::Disconnected {
            op.finished = true;
            return Err(WifiError::InvalidState);
        }
        self.status.lock().unwrap().paired = false;
        self.transition(&op, WifiState::Pairing)?;
        let result = expect_reply(
            self.request(
                &op,
                RequestKind::Pair,
                endpoint,
                Some(code),
                Instant::now() + Duration::from_secs(30),
            )
            .await,
            AdbReply::Paired,
        );
        if result.is_ok() && !op.token.is_cancelled() {
            self.status.lock().unwrap().paired = true;
        }
        self.finish(&mut op, result, WifiState::Disconnected).await
    }
    pub async fn connect(&self, consent: Consent, recovery: Consent) -> Result<(), WifiError> {
        if consent != Consent::Granted {
            return Err(WifiError::NoConsent);
        }
        let mut op = self.begin()?;
        {
            let mut status = self.status.lock().unwrap();
            if !status.paired {
                op.finished = true;
                return Err(WifiError::NotPaired);
            }
            if status.state != WifiState::Disconnected {
                op.finished = true;
                return Err(WifiError::InvalidState);
            }
            status.recovery = recovery == Consent::Granted;
            status.contacted = true;
        }
        self.transition(&op, WifiState::Connecting)?;
        let result = self
            .connect_attempt(&op, Instant::now() + Duration::from_secs(7))
            .await;
        self.finish(&mut op, result, WifiState::Connected).await
    }
    async fn connect_attempt(
        &self,
        op: &Operation<'_, R>,
        deadline: Instant,
    ) -> Result<(), WifiError> {
        expect_reply(
            self.request(
                op,
                RequestKind::Connect,
                self.target.clone(),
                None,
                deadline.min(Instant::now() + Duration::from_secs(5)),
            )
            .await,
            AdbReply::Connected,
        )?;
        expect_reply(
            self.request(
                op,
                RequestKind::Probe,
                self.target.clone(),
                None,
                deadline.min(Instant::now() + Duration::from_secs(2)),
            )
            .await,
            AdbReply::Device,
        )
    }
    pub async fn check_link(&self) -> Result<(), WifiError> {
        let mut op = self.begin()?;
        if self.state() != WifiState::Connected {
            op.finished = true;
            return Err(WifiError::InvalidState);
        }
        let result = expect_reply(
            self.request(
                &op,
                RequestKind::Probe,
                self.target.clone(),
                None,
                Instant::now() + Duration::from_secs(2),
            )
            .await,
            AdbReply::Device,
        );
        if result.is_ok() {
            op.finished = true;
            return Ok(());
        }
        let error = result.unwrap_err();
        let _ = self.events.send(WifiEvent::LinkLost);
        let recovery = self.status.lock().unwrap().recovery;
        if recovery && error.is_recoverable() {
            self.transition(&op, WifiState::Recovering)?;
            op.finished = true;
            Err(error)
        } else {
            self.finish(&mut op, Err(error), WifiState::Failed).await
        }
    }
    /// Explicit monitor; returns on link loss. Recovery is a separate consented call.
    /// After a lagged event stream consumers must invalidate input/focus themselves.
    pub async fn monitor(&self) -> Result<(), WifiError> {
        let _monitor = self.monitor_lock.try_lock().map_err(|_| WifiError::Busy)?;
        let token = self.status.lock().unwrap().cancellation.clone();
        loop {
            if self.state() != WifiState::Connected {
                return Err(WifiError::InvalidState);
            }
            tokio::select! {
                biased;
                _ = token.cancelled() => return Err(WifiError::Cancelled),
                _ = tokio::time::sleep(Duration::from_secs(5)) => {}
            }
            self.check_link().await?;
        }
    }
    pub async fn recover(&self, consent: Consent) -> Result<(), WifiError> {
        if consent != Consent::Granted {
            return Err(WifiError::NoConsent);
        }
        let mut op = self.begin()?;
        if self.state() != WifiState::Recovering {
            op.finished = true;
            return Err(WifiError::InvalidState);
        }
        let deadline = Instant::now() + Duration::from_secs(30);
        let mut result = Err(WifiError::RecoveryExhausted);
        for (index, delay) in [1, 2, 4].into_iter().enumerate() {
            tokio::select! {
                biased;
                _ = op.token.cancelled() => { result = Err(WifiError::Cancelled); break; }
                _ = tokio::time::sleep_until(deadline) => { result = Err(WifiError::Timeout); break; }
                _ = tokio::time::sleep(Duration::from_secs(delay)) => {}
            }
            let _ = self
                .events
                .send(WifiEvent::RecoveryAttempt(index as u8 + 1));
            match self.connect_attempt(&op, deadline).await {
                Ok(()) => {
                    result = Ok(());
                    break;
                }
                Err(error) if error.is_recoverable() => {}
                Err(error) => {
                    result = Err(error);
                    break;
                }
            }
            if Instant::now() >= deadline {
                result = Err(WifiError::Timeout);
                break;
            }
        }
        let result = self.finish(&mut op, result, WifiState::Connected).await;
        if result.is_ok() {
            let _ = self.events.send(WifiEvent::Recovered);
        }
        result
    }
    async fn stop(&self, disconnect: bool) -> Result<(), WifiError> {
        let _cleanup = self.cleanup.lock().await;
        {
            let mut status = self.status.lock().unwrap();
            status.stopping = true;
            status.generation += 1;
            status.cancellation.cancel();
        }
        let _operation = self.operation.lock().await;
        let _monitor = self.monitor_lock.lock().await;
        let contacted = self.status.lock().unwrap().contacted;
        let result = if disconnect && contacted {
            let deadline = Instant::now() + Duration::from_secs(5);
            let reply = tokio::time::timeout_at(
                deadline,
                self.runner.run(
                    AdbRequest {
                        kind: RequestKind::Disconnect,
                        endpoint: self.target.clone(),
                        code: None,
                    },
                    deadline,
                    Cancellation::default(),
                ),
            )
            .await
            .unwrap_or(Err(WifiError::Timeout));
            expect_reply(reply, AdbReply::Disconnected)
        } else {
            Ok(())
        };
        let result = self
            .runner
            .shutdown()
            .await
            .map_err(|_| WifiError::CleanupFailed)
            .and(result);
        let mut status = self.status.lock().unwrap();
        status.state = if result.is_ok() {
            WifiState::Disconnected
        } else {
            WifiState::Failed
        };
        status.stopping = false;
        status.paired = false;
        status.recovery = false;
        status.contacted = false;
        status.cancellation = Cancellation::default();
        let _ = self.events.send(WifiEvent::StateChanged(status.state));
        if let Err(error) = result {
            let _ = self.events.send(WifiEvent::Error(error));
        }
        result
    }
    pub async fn disconnect(&self) -> Result<(), WifiError> {
        self.stop(true).await
    }
    pub async fn cancel(&self) -> Result<(), WifiError> {
        self.stop(false).await
    }
}

struct Operation<'a, R: AdbRunner> {
    controller: &'a WifiController<R>,
    _guard: tokio::sync::MutexGuard<'a, ()>,
    token: Cancellation,
    generation: u64,
    finished: bool,
}
impl<R: AdbRunner> Drop for Operation<'_, R> {
    fn drop(&mut self) {
        if !self.finished {
            self.token.cancel();
            let mut status = self.controller.status.lock().unwrap();
            if !status.stopping {
                status.state = WifiState::Failed;
                status.paired = false;
                let _ = self
                    .controller
                    .events
                    .send(WifiEvent::StateChanged(WifiState::Failed));
            }
        }
    }
}
fn expect_reply(reply: Result<AdbReply, WifiError>, expected: AdbReply) -> Result<(), WifiError> {
    match reply? {
        reply if reply == expected => Ok(()),
        AdbReply::Unauthorized => Err(WifiError::Unauthorized),
        AdbReply::Offline => Err(WifiError::Offline),
        _ => Err(WifiError::UnexpectedReply),
    }
}
