use limivra_core::{DisplayMode, SessionController, SessionEvent};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::instrument::WithSubscriber;
use tracing_subscriber::fmt::MakeWriter;

#[cfg(target_os = "linux")]
static PROCESS_FIXTURE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[cfg(target_os = "linux")]
mod common;
#[cfg(target_os = "linux")]
use common::process::Fixture;

#[derive(Clone, Default)]
struct LogBuffer(Arc<Mutex<Vec<u8>>>);

impl<'a> MakeWriter<'a> for LogBuffer {
    type Writer = Self;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

impl Write for LogBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0
            .lock()
            .map_err(|_| io::Error::other("log buffer poisoned"))?
            .extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn test_subscriber(writer: LogBuffer) -> impl tracing::Subscriber + Send + Sync {
    tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_writer(writer)
        .finish()
}

#[tokio::test]
async fn transport_log_redacts_serial_arguments() {
    const SERIAL: &str = "LIMIVRA-TEST-SERIAL-XYZ";

    let captured = LogBuffer::default();
    let controller = SessionController::new();
    let owned_args = [
        "-s".to_owned(),
        SERIAL.to_owned(),
        "--serial".to_owned(),
        SERIAL.to_owned(),
        format!("--serial={SERIAL}"),
        format!("-s={SERIAL}"),
        "--window-title".to_owned(),
        format!("Limivra Desktop ({SERIAL})"),
    ];
    let args = owned_args.iter().map(String::as_str).collect::<Vec<_>>();

    let result = controller
        .start(
            "limivra-missing-test-executable",
            &args,
            SERIAL,
            DisplayMode::Mirror,
        )
        .with_subscriber(test_subscriber(captured.clone()))
        .await;

    assert!(
        result.is_err(),
        "missing executable should fail after logging"
    );
    let output = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    assert!(
        output.contains("[REDACTED]"),
        "expected a redacted argument: {output}"
    );
    assert!(
        !output.contains(SERIAL),
        "transport log leaked the serial sentinel: {output}"
    );
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn session_child_exit_codes_are_reported_without_false_active_state() {
    let _fixture_lock = PROCESS_FIXTURE_LOCK.lock().await;
    for code in [1, 7, 127] {
        let fixture = Fixture::new(&format!(
            "printf '%s\\n' \"$$\" > \"$LIMIVRA_FIXTURE_ROOT/child-{code}\"; exit {code}"
        ));
        let controller = SessionController::new();
        let mut events = controller.subscribe();
        controller
            .start(
                fixture.path().to_str().unwrap(),
                &[],
                "synthetic-device",
                DisplayMode::Mirror,
            )
            .with_subscriber(test_subscriber(LogBuffer::default()))
            .await
            .unwrap();
        let pid = fixture
            .wait_for_pid(&format!("child-{code}"), Duration::from_secs(2))
            .await
            .unwrap();

        let observed = tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                if let SessionEvent::ProcessExited(actual) = events.recv().await.unwrap() {
                    break actual;
                }
            }
        })
        .await
        .expect("child exit event timed out");

        assert_eq!(observed, Some(code));
        assert_eq!(
            controller.current_state().await,
            limivra_core::SessionState::Terminated
        );
        fixture.assert_reaped(pid);
    }
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn sigterm_terminated_child_reaches_terminated_state() {
    let _fixture_lock = PROCESS_FIXTURE_LOCK.lock().await;
    let fixture = Fixture::new(
        "printf '%s\\n' \"$$\" > \"$LIMIVRA_FIXTURE_ROOT/term-child\"; kill -TERM \"$$\"",
    );
    let controller = SessionController::new();
    let mut events = controller.subscribe();
    controller
        .start(
            fixture.path().to_str().unwrap(),
            &[],
            "synthetic-device",
            DisplayMode::Mirror,
        )
        .with_subscriber(test_subscriber(LogBuffer::default()))
        .await
        .unwrap();
    let pid = fixture
        .wait_for_pid("term-child", Duration::from_secs(2))
        .await
        .unwrap();

    let observed = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            if let SessionEvent::ProcessExited(code) = events.recv().await.unwrap() {
                break code;
            }
        }
    })
    .await
    .expect("signal exit event timed out");

    assert_eq!(observed, None);
    assert_eq!(
        controller.current_state().await,
        limivra_core::SessionState::Terminated
    );
    fixture.assert_reaped(pid);
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn session_stop_kills_and_reaps_only_its_child_and_is_idempotent() {
    let _fixture_lock = PROCESS_FIXTURE_LOCK.lock().await;
    let owned_fixture = Fixture::new(
        "printf '%s\\n' \"$$\" > \"$LIMIVRA_FIXTURE_ROOT/owned-child\"; exec /bin/sleep 30",
    );
    let sentinel_fixture = Fixture::new(
        "printf '%s\\n' \"$$\" > \"$LIMIVRA_FIXTURE_ROOT/sentinel-child\"; exec /bin/sleep 30",
    );
    let owned = SessionController::new();
    let sentinel = SessionController::new();

    for (controller, fixture) in [(&owned, &owned_fixture), (&sentinel, &sentinel_fixture)] {
        controller
            .start(
                fixture.path().to_str().unwrap(),
                &[],
                "synthetic-device",
                DisplayMode::Mirror,
            )
            .with_subscriber(test_subscriber(LogBuffer::default()))
            .await
            .unwrap();
    }
    let owned_pid = owned_fixture
        .wait_for_pid("owned-child", Duration::from_secs(2))
        .await
        .unwrap();
    let sentinel_pid = sentinel_fixture
        .wait_for_pid("sentinel-child", Duration::from_secs(2))
        .await
        .unwrap();
    assert!(std::path::Path::new(&format!("/proc/{sentinel_pid}")).exists());

    owned.stop().await.unwrap();
    owned.stop().await.unwrap();
    assert_eq!(
        owned.current_state().await,
        limivra_core::SessionState::Disconnected
    );
    owned_fixture.assert_reaped(owned_pid);
    assert!(
        std::path::Path::new(&format!("/proc/{sentinel_pid}")).exists(),
        "stopping one session must not kill a separate fixture process"
    );

    sentinel.stop().await.unwrap();
    sentinel_fixture.assert_reaped(sentinel_pid);
}

#[cfg(target_os = "linux")]
#[tokio::test(start_paused = true)]
async fn rapid_start_stop_cycles_reap_every_owned_child() {
    let _fixture_lock = PROCESS_FIXTURE_LOCK.lock().await;
    use std::path::Path;

    let controller = SessionController::new();
    let mut events = controller.subscribe();
    let sentinel_fixture = Fixture::new(
        "printf '%s\\n' \"$$\" > \"$LIMIVRA_FIXTURE_ROOT/sentinel\"; exec /bin/sleep 30",
    );
    let sentinel = SessionController::new();
    sentinel
        .start(
            sentinel_fixture.path().to_str().unwrap(),
            &[],
            "synthetic-sentinel",
            DisplayMode::Mirror,
        )
        .await
        .unwrap();
    let sentinel_pid = sentinel_fixture
        .wait_for_pid("sentinel", Duration::from_secs(3))
        .await
        .unwrap();

    for cycle in 0..20 {
        let marker = format!("child-{cycle}");
        let fixture = Fixture::new(&format!(
            "printf '%s\\n' \"$$\" > \"$LIMIVRA_FIXTURE_ROOT/{marker}\"; exec /bin/sleep 30"
        ));
        controller
            .start(
                fixture.path().to_str().unwrap(),
                &[],
                "synthetic-device",
                DisplayMode::Mirror,
            )
            .await
            .unwrap();
        let pid = fixture
            .wait_for_pid(&marker, Duration::from_secs(3))
            .await
            .unwrap();

        controller.stop().await.unwrap();
        assert_eq!(
            controller.current_state().await,
            limivra_core::SessionState::Disconnected
        );
        fixture.assert_reaped(pid);
        assert!(Path::new(&format!("/proc/{sentinel_pid}")).exists());

        // Advance the monitor's timer so each old generation observes its
        // invalidation before the next cycle; no fixed wall-clock sleep needed.
        tokio::time::advance(Duration::from_millis(25)).await;
        tokio::task::yield_now().await;
        while let Ok(event) = events.try_recv() {
            assert_ne!(
                event,
                SessionEvent::StateChanged(limivra_core::SessionState::Terminated),
                "cycle {cycle} emitted a late termination after stop"
            );
        }
    }

    sentinel.stop().await.unwrap();
    sentinel_fixture.assert_reaped(sentinel_pid);
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn dropping_common_fixture_terminates_child_before_pid_registration() {
    let _fixture_lock = PROCESS_FIXTURE_LOCK.lock().await;
    let fixture = Fixture::new(
        "printf '%s\\n' \"$$\" > \"$LIMIVRA_FIXTURE_ROOT/registered-child\"; exec /bin/sleep 30",
    );
    let mut child = std::process::Command::new(fixture.path())
        .spawn()
        .expect("start fixture-owned fake child");
    let marker = fixture.pid_file("fixture-root-pid");
    let pid = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Ok(contents) = std::fs::read_to_string(&marker) {
                if let Ok(pid) = contents.trim().parse::<u32>() {
                    break pid;
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("fake child publishes its PID");

    drop(fixture);
    let _ = child.wait().expect("reap fixture child after teardown");

    assert!(
        !std::path::Path::new(&format!("/proc/{pid}")).exists(),
        "fixture child should be reaped during teardown"
    );
}
