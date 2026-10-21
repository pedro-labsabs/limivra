use limivra_core::wifi::*;

#[test]
fn endpoints_are_canonical_private_ipv4_with_explicit_port() {
    for address in [
        "10.0.0.1:1",
        "172.16.0.1:65535",
        "172.31.255.254:1234",
        "192.168.1.9:4321",
    ] {
        assert!(LocalEndpoint::parse(address).is_ok(), "{address}");
    }
    for address in [
        "8.8.8.8:5555",
        "127.0.0.1:5555",
        "0.0.0.0:1",
        "169.254.1.2:1",
        "172.32.0.1:1",
        "224.0.0.1:1",
        "255.255.255.255:1",
        "10.1:5555",
        "10.0.0.1",
        "10.0.0.1:0",
        "10.0.0.1:65536",
        "10.0.0.1:01",
        "10.00.0.1:12",
        "10.0.0.1:+12",
        "10.0.0.1:１２",
        "10.0.0.1:12\n",
        " 10.0.0.1:12",
        "10.0.0.1:12;id",
        "--help",
        "https://10.0.0.1:12",
        "localhost:12",
        "[fd00::1]:12",
        "[::ffff:10.0.0.1]:12",
        "10.0.0.1:12:13",
    ] {
        assert!(LocalEndpoint::parse(address).is_err(), "{address}");
    }
}

#[test]
fn pairing_code_is_strict_and_debug_is_redacted() {
    let code = PairingCode::parse("123456").unwrap();
    assert!(!format!("{code:?}").contains("123456"));
    for value in [
        "",
        "12345",
        "1234567",
        "１２３４５６",
        "123456\n",
        "123 56",
        "-12345",
    ] {
        assert!(PairingCode::parse(value).is_err());
    }
}

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::time::Instant;
#[derive(Clone, Default)]
struct Mock {
    replies: Arc<Mutex<VecDeque<Result<AdbReply, WifiError>>>>,
    calls: Arc<Mutex<Vec<RequestKind>>>,
    times: Arc<Mutex<Vec<Instant>>>,
}
impl Mock {
    fn queue(&self, replies: impl IntoIterator<Item = Result<AdbReply, WifiError>>) {
        self.replies.lock().unwrap().extend(replies);
    }
}
impl AdbRunner for Mock {
    async fn run(
        &self,
        request: AdbRequest,
        _: Instant,
        _: Cancellation,
    ) -> Result<AdbReply, WifiError> {
        self.calls.lock().unwrap().push(request.kind);
        self.times.lock().unwrap().push(Instant::now());
        self.replies
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected command")
    }
    async fn shutdown(&self) -> Result<(), WifiError> {
        Ok(())
    }
}
fn endpoint(port: u16) -> LocalEndpoint {
    LocalEndpoint::parse(&format!("192.168.1.9:{port}")).unwrap()
}
async fn paired(mock: &Mock) -> WifiController<Mock> {
    let c = WifiController::new(mock.clone(), endpoint(4321));
    mock.queue([Ok(AdbReply::Paired)]);
    c.pair(
        endpoint(1234),
        PairingCode::parse("123456").unwrap(),
        Consent::Granted,
    )
    .await
    .unwrap();
    c
}
async fn connected(mock: &Mock, recovery: Consent) -> WifiController<Mock> {
    let c = paired(mock).await;
    mock.queue([Ok(AdbReply::Connected), Ok(AdbReply::Device)]);
    c.connect(Consent::Granted, recovery).await.unwrap();
    c
}
#[tokio::test]
async fn consent_and_pairing_are_required_before_any_network_command() {
    let mock = Mock::default();
    let c = WifiController::new(mock.clone(), endpoint(4321));
    assert_eq!(
        c.connect(Consent::Granted, Consent::Denied).await,
        Err(WifiError::NotPaired)
    );
    assert_eq!(
        c.pair(
            endpoint(1234),
            PairingCode::parse("123456").unwrap(),
            Consent::Denied
        )
        .await,
        Err(WifiError::NoConsent)
    );
    assert_eq!(
        c.pair(
            LocalEndpoint::parse("10.0.0.1:1234").unwrap(),
            PairingCode::parse("123456").unwrap(),
            Consent::Granted
        )
        .await,
        Err(WifiError::WrongTarget)
    );
    assert!(mock.calls.lock().unwrap().is_empty());
    c.disconnect().await.unwrap();
    assert!(mock.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn pairing_does_not_imply_connection_and_connect_requires_device_probe() {
    let mock = Mock::default();
    let c = paired(&mock).await;
    assert_eq!(c.state(), WifiState::Disconnected);
    assert_eq!(
        c.connect(Consent::Denied, Consent::Denied).await,
        Err(WifiError::NoConsent)
    );
    let mut events = c.subscribe();
    mock.queue([
        Ok(AdbReply::Connected),
        Ok(AdbReply::Device),
        Ok(AdbReply::Disconnected),
    ]);
    c.connect(Consent::Granted, Consent::Granted).await.unwrap();
    assert_eq!(c.state(), WifiState::Connected);
    assert_eq!(
        events.try_recv().unwrap(),
        WifiEvent::StateChanged(WifiState::Connecting)
    );
    assert_eq!(
        events.try_recv().unwrap(),
        WifiEvent::StateChanged(WifiState::Connected)
    );
    c.disconnect().await.unwrap();
    assert_eq!(c.state(), WifiState::Disconnected);
    assert_eq!(
        *mock.calls.lock().unwrap(),
        [
            RequestKind::Pair,
            RequestKind::Connect,
            RequestKind::Probe,
            RequestKind::Disconnect
        ]
    );
    assert_eq!(
        c.connect(Consent::Granted, Consent::Denied).await,
        Err(WifiError::NotPaired)
    );
}
#[tokio::test]
async fn failed_confirmation_never_exposes_connected_state() {
    for reply in [
        Ok(AdbReply::Offline),
        Ok(AdbReply::Unauthorized),
        Err(WifiError::UnexpectedReply),
    ] {
        let mock = Mock::default();
        let c = paired(&mock).await;
        let mut events = c.subscribe();
        mock.queue([Ok(AdbReply::Connected), reply]);
        assert!(c.connect(Consent::Granted, Consent::Denied).await.is_err());
        assert_eq!(c.state(), WifiState::Failed);
        while let Ok(event) = events.try_recv() {
            assert_ne!(event, WifiEvent::StateChanged(WifiState::Connected));
        }
    }
}
#[tokio::test(start_paused = true)]
async fn recovery_has_exact_backoff_three_attempts_and_no_pair_replay() {
    let mock = Mock::default();
    let c = connected(&mock, Consent::Granted).await;
    mock.queue([Ok(AdbReply::Offline)]);
    assert_eq!(c.check_link().await, Err(WifiError::Offline));
    assert_eq!(c.state(), WifiState::Recovering);
    let start = Instant::now();
    mock.queue([
        Err(WifiError::Offline),
        Err(WifiError::Timeout),
        Ok(AdbReply::Connected),
        Ok(AdbReply::Device),
    ]);
    c.recover(Consent::Granted).await.unwrap();
    assert_eq!(c.state(), WifiState::Connected);
    let times = mock.times.lock().unwrap();
    assert_eq!(times[4] - start, Duration::from_secs(1));
    assert_eq!(times[5] - start, Duration::from_secs(3));
    assert_eq!(times[6] - start, Duration::from_secs(7));
    assert_eq!(
        mock.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|&&k| k == RequestKind::Pair)
            .count(),
        1
    );
}
#[tokio::test(start_paused = true)]
async fn recovery_exhaustion_and_unauthorized_stop_without_further_retries() {
    for unauthorized in [false, true] {
        let mock = Mock::default();
        let c = connected(&mock, Consent::Granted).await;
        mock.queue([Ok(AdbReply::Offline)]);
        c.check_link().await.unwrap_err();
        assert_eq!(c.recover(Consent::Denied).await, Err(WifiError::NoConsent));
        mock.queue(if unauthorized {
            vec![Ok(AdbReply::Unauthorized)]
        } else {
            vec![Err(WifiError::Offline); 3]
        });
        assert_eq!(
            c.recover(Consent::Granted).await,
            Err(if unauthorized {
                WifiError::Unauthorized
            } else {
                WifiError::RecoveryExhausted
            })
        );
        assert_eq!(c.state(), WifiState::Failed);
        assert!(mock.replies.lock().unwrap().is_empty());
    }
    let mock = Mock::default();
    let c = connected(&mock, Consent::Denied).await;
    mock.queue([Ok(AdbReply::Offline)]);
    c.check_link().await.unwrap_err();
    assert_eq!(c.state(), WifiState::Failed);
    assert_eq!(
        c.recover(Consent::Granted).await,
        Err(WifiError::InvalidState)
    );
}

#[derive(Clone, Default)]
struct Slow {
    mock: Mock,
    blocked: Arc<Mutex<Option<RequestKind>>>,
    started: Arc<tokio::sync::Notify>,
    release: Arc<tokio::sync::Notify>,
}
impl AdbRunner for Slow {
    async fn run(
        &self,
        request: AdbRequest,
        deadline: Instant,
        cancellation: Cancellation,
    ) -> Result<AdbReply, WifiError> {
        let block = *self.blocked.lock().unwrap() == Some(request.kind);
        if block {
            self.mock.calls.lock().unwrap().push(request.kind);
            self.started.notify_one();
            self.release.notified().await;
            return Ok(match request.kind {
                RequestKind::Probe => AdbReply::Device,
                _ => AdbReply::Connected,
            });
        }
        self.mock.run(request, deadline, cancellation).await
    }
    async fn shutdown(&self) -> Result<(), WifiError> {
        Ok(())
    }
}
async fn slow_paired() -> (Arc<WifiController<Slow>>, Slow) {
    let runner = Slow::default();
    runner.mock.queue([Ok(AdbReply::Paired)]);
    let c = Arc::new(WifiController::new(runner.clone(), endpoint(4321)));
    c.pair(
        endpoint(1234),
        PairingCode::parse("123456").unwrap(),
        Consent::Granted,
    )
    .await
    .unwrap();
    (c, runner)
}
#[tokio::test(start_paused = true)]
async fn busy_cancel_and_late_response_cannot_resurrect_connection() {
    let (c, runner) = slow_paired().await;
    *runner.blocked.lock().unwrap() = Some(RequestKind::Connect);
    let worker = Arc::clone(&c);
    let task =
        tokio::spawn(async move { worker.connect(Consent::Granted, Consent::Granted).await });
    runner.started.notified().await;
    assert_eq!(
        c.connect(Consent::Granted, Consent::Granted).await,
        Err(WifiError::Busy)
    );
    c.cancel().await.unwrap();
    assert_eq!(task.await.unwrap(), Err(WifiError::Cancelled));
    tokio::time::advance(Duration::from_secs(120)).await;
    assert_eq!(c.state(), WifiState::Disconnected);
    assert_eq!(
        c.connect(Consent::Granted, Consent::Denied).await,
        Err(WifiError::NotPaired)
    );
}

#[tokio::test(start_paused = true)]
async fn cancel_during_device_probe_cannot_restore_connected() {
    let runner = Slow::default();
    runner.mock.queue([Ok(AdbReply::Paired)]);
    let c = Arc::new(WifiController::new(runner.clone(), endpoint(4321)));
    c.pair(
        endpoint(1234),
        PairingCode::parse("123456").unwrap(),
        Consent::Granted,
    )
    .await
    .unwrap();
    runner.mock.queue([Ok(AdbReply::Connected)]);
    *runner.blocked.lock().unwrap() = Some(RequestKind::Probe);
    let mut events = c.subscribe();
    let worker = Arc::clone(&c);
    let task =
        tokio::spawn(async move { worker.connect(Consent::Granted, Consent::Granted).await });
    runner.started.notified().await;
    assert_eq!(c.state(), WifiState::Connecting);

    c.cancel().await.unwrap();
    assert_eq!(task.await.unwrap(), Err(WifiError::Cancelled));
    runner.release.notify_one();
    tokio::time::advance(Duration::from_secs(60)).await;

    assert_eq!(c.state(), WifiState::Disconnected);
    assert_eq!(
        *runner.mock.calls.lock().unwrap(),
        [RequestKind::Pair, RequestKind::Connect, RequestKind::Probe]
    );
    while let Ok(event) = events.try_recv() {
        assert_ne!(event, WifiEvent::StateChanged(WifiState::Connected));
    }
}
#[tokio::test(start_paused = true)]
async fn disconnect_enforces_timeout_even_when_backend_ignores_deadline() {
    let (c, runner) = slow_paired().await;
    runner
        .mock
        .queue([Ok(AdbReply::Connected), Ok(AdbReply::Device)]);
    c.connect(Consent::Granted, Consent::Denied).await.unwrap();
    *runner.blocked.lock().unwrap() = Some(RequestKind::Disconnect);
    let start = Instant::now();
    assert_eq!(c.disconnect().await, Err(WifiError::Timeout));
    assert_eq!(Instant::now() - start, Duration::from_secs(5));
    assert_eq!(c.state(), WifiState::Failed);
}
#[tokio::test(start_paused = true)]
async fn cancel_during_backoff_leaves_no_retry_pending() {
    let mock = Mock::default();
    let c = Arc::new(connected(&mock, Consent::Granted).await);
    mock.queue([Ok(AdbReply::Offline)]);
    c.check_link().await.unwrap_err();
    let worker = Arc::clone(&c);
    let task = tokio::spawn(async move { worker.recover(Consent::Granted).await });
    tokio::task::yield_now().await;
    c.cancel().await.unwrap();
    assert_eq!(task.await.unwrap(), Err(WifiError::Cancelled));
    tokio::time::advance(Duration::from_secs(120)).await;
    assert_eq!(mock.calls.lock().unwrap().len(), 4);
    assert_eq!(c.state(), WifiState::Disconnected);
}
#[tokio::test(start_paused = true)]
async fn monitor_waits_five_seconds_and_notifies_link_loss() {
    let mock = Mock::default();
    let c = connected(&mock, Consent::Granted).await;
    let start = Instant::now();
    mock.queue([Ok(AdbReply::Offline)]);
    let mut events = c.subscribe();
    assert_eq!(c.monitor().await, Err(WifiError::Offline));
    assert_eq!(Instant::now() - start, Duration::from_secs(5));
    assert_eq!(c.state(), WifiState::Recovering);
    assert_eq!(events.try_recv().unwrap(), WifiEvent::LinkLost);
}
#[tokio::test(start_paused = true)]
async fn recovery_timeouts_remain_inside_global_budget() {
    let (c, runner) = slow_paired().await;
    runner.mock.queue([
        Ok(AdbReply::Connected),
        Ok(AdbReply::Device),
        Ok(AdbReply::Offline),
    ]);
    c.connect(Consent::Granted, Consent::Granted).await.unwrap();
    c.check_link().await.unwrap_err();
    *runner.blocked.lock().unwrap() = Some(RequestKind::Connect);
    let start = Instant::now();
    assert_eq!(
        c.recover(Consent::Granted).await,
        Err(WifiError::RecoveryExhausted)
    );
    assert_eq!(Instant::now() - start, Duration::from_secs(22));
    assert!(Instant::now() - start < Duration::from_secs(30));
    assert_eq!(
        runner
            .mock
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|&&k| k == RequestKind::Connect)
            .count(),
        4
    );
}
