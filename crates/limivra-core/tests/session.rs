use limivra_core::{DisplayMode, SessionController};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use tracing::instrument::WithSubscriber;
use tracing_subscriber::fmt::MakeWriter;

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

#[tokio::test]
async fn transport_log_redacts_serial_arguments() {
    const SERIAL: &str = "LIMIVRA-TEST-SERIAL-XYZ";

    let captured = LogBuffer::default();
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .without_time()
        .with_writer(captured.clone())
        .finish();
    let controller = SessionController::new();
    let owned_args = [
        "-s".to_owned(),
        SERIAL.to_owned(),
        "--serial".to_owned(),
        SERIAL.to_owned(),
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
        .with_subscriber(subscriber)
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
