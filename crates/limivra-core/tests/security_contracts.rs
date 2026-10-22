use limivra_core::{
    input_policy::{
        Control, InputCapabilities, InputEvent, InputMode, InputPolicy, Modifier, Support,
    },
    session::redact_transport_arguments,
    AdbDevice, DeviceProfile, DeviceStatus, DisplayMode, SessionController, SessionEvent,
    SessionState,
};
use std::{
    fs,
    io::{self, Write},
    os::unix::fs::PermissionsExt,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, Mutex,
    },
};
use tracing::instrument::WithSubscriber;
use tracing_subscriber::fmt::MakeWriter;

const SERIAL: &str = "LIMIVRA-TEST-SERIAL-XYZ";
static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

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

fn private_dir(path: &std::path::Path) {
    fs::create_dir(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

#[test]
fn serial_is_redacted_from_all_transport_argument_forms_and_debug_models() {
    let args = [
        "-s",
        SERIAL,
        "--serial",
        SERIAL,
        "--serial=LIMIVRA-TEST-SERIAL-XYZ",
        "-s=LIMIVRA-TEST-SERIAL-XYZ",
        "--window-title",
        "Limivra Desktop (LIMIVRA-TEST-SERIAL-XYZ)",
    ];
    let redacted = redact_transport_arguments(&args, SERIAL).join(" ");
    assert!(redacted.contains("[REDACTED]"));
    assert!(
        !redacted.contains(SERIAL),
        "arguments leaked serial: {redacted}"
    );

    let device = AdbDevice {
        serial: SERIAL.into(),
        status: DeviceStatus::Device,
        model: None,
        product: None,
        transport_id: None,
    };
    let profile = DeviceProfile {
        serial: SERIAL.into(),
        ..DeviceProfile::default()
    };
    let state = SessionState::Active {
        serial: SERIAL.into(),
        mode: DisplayMode::Mirror,
    };
    for formatted in [
        format!("{device:?}"),
        format!("{profile:?}"),
        format!("{state:?}"),
    ] {
        assert!(
            !formatted.contains(SERIAL),
            "debug output leaked serial: {formatted}"
        );
    }

    for encoded in [
        serde_json::to_string(&device).unwrap(),
        serde_json::to_string(&profile).unwrap(),
        serde_json::to_string(&state).unwrap(),
    ] {
        assert!(
            !encoded.contains(SERIAL),
            "serialized model leaked serial: {encoded}"
        );
        assert!(encoded.contains("[REDACTED]"));
    }
}

#[tokio::test]
async fn spawn_failure_redacts_serial_from_trace_error_and_session_event() {
    let captured = LogBuffer::default();
    let controller = SessionController::new();
    let mut events = controller.subscribe();
    let executable = format!("/definitely/missing/{SERIAL}");
    let args = ["--serial", SERIAL, "--window-title", SERIAL];
    let result = controller
        .start(&executable, &args, SERIAL, DisplayMode::Mirror)
        .with_subscriber(
            tracing_subscriber::fmt()
                .with_ansi(false)
                .without_time()
                .with_writer(captured.clone())
                .finish(),
        )
        .await;

    let error = result.expect_err("the synthetic executable must not exist");
    let log = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
    assert!(!log.contains(SERIAL), "tracing leaked serial: {log}");
    assert!(
        !error.to_string().contains(SERIAL),
        "error leaked serial: {error}"
    );

    while let Ok(event) = events.try_recv() {
        let rendered = format!("{event:?}");
        assert!(
            !rendered.contains(SERIAL),
            "session event leaked serial: {rendered}"
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
fn private_adb_socket_directories_require_exact_0700_permissions() {
    use limivra_core::wifi::{adb_runner::PrivateAdbRunner, WifiError};

    let root = std::env::temp_dir().join(format!(
        "limivra-security-{}-{}",
        std::process::id(),
        NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
    ));
    private_dir(&root);
    let home = root.join("home");
    let android = home.join(".android");
    let runtime = root.join("runtime");
    private_dir(&home);
    private_dir(&android);
    private_dir(&runtime);
    let key = android.join("adbkey");
    fs::write(&key, b"synthetic test key").unwrap();
    fs::set_permissions(&key, fs::Permissions::from_mode(0o600)).unwrap();
    let adb = root.join("fake-adb");
    fs::write(&adb, b"#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&adb, fs::Permissions::from_mode(0o700)).unwrap();

    let accepted = PrivateAdbRunner::new(adb.clone(), home.clone(), runtime.clone());
    assert!(
        accepted.is_ok(),
        "private 0700 directories should be accepted"
    );
    drop(accepted);

    for directory in [&root, &home, &android, &runtime] {
        assert_eq!(
            fs::metadata(directory).unwrap().permissions().mode() & 0o777,
            0o700,
            "directory must be exactly owner-only: {}",
            directory.display()
        );
    }

    for unsafe_mode in [0o500, 0o1700] {
        fs::set_permissions(&runtime, fs::Permissions::from_mode(unsafe_mode)).unwrap();
        assert!(matches!(
            PrivateAdbRunner::new(adb.clone(), home.clone(), runtime.clone()),
            Err(WifiError::UnsafeConfiguration)
        ));
    }
    fs::set_permissions(&runtime, fs::Permissions::from_mode(0o700)).unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn losing_focus_releases_every_pressed_key_button_and_modifier() {
    let mut policy = InputPolicy::default();
    policy.observe_session(&SessionEvent::StateChanged(SessionState::Active {
        serial: SERIAL.into(),
        mode: DisplayMode::Mirror,
    }));
    policy.set_capabilities(InputCapabilities {
        sdk: Support::Supported,
        ..InputCapabilities::default()
    });
    policy.select_mode(InputMode::Sdk).unwrap();
    policy.set_focus(true);
    policy.set_input_consent(true);

    let controls = [
        Control::Modifier(Modifier::Ctrl),
        Control::Modifier(Modifier::Shift),
        Control::Key(30),
        Control::Key(31),
        Control::MouseButton(1),
    ];
    for control in controls {
        assert_eq!(
            policy.route(InputEvent {
                control,
                pressed: true
            }),
            Ok(Some(InputEvent {
                control,
                pressed: true
            }))
        );
    }

    let releases = policy.set_focus(false);
    assert_eq!(releases.len(), controls.len());
    assert!(releases.iter().all(|event| !event.pressed));
    for control in controls {
        assert!(releases.contains(&InputEvent {
            control,
            pressed: false
        }));
    }
    assert!(matches!(
        releases.last().map(|event| event.control),
        Some(Control::Modifier(_))
    ));
}
