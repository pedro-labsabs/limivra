use limivra_core::process_supervisor::{ProcessSupervisor, WaitOutcome};
use std::ffi::OsStr;
use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

fn fake_child() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake-child.sh")
}

fn spawn_fake(args: &[&OsStr]) -> ProcessSupervisor {
    ProcessSupervisor::spawn(fake_child(), args.iter().copied()).expect("fake child deve iniciar")
}

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn spawn_reports_a_missing_executable() {
    let result = ProcessSupervisor::spawn(
        "/definitely-not-a-real-limivra-executable",
        std::iter::empty::<&OsStr>(),
    );

    assert!(matches!(result, Err(error) if error.kind() == std::io::ErrorKind::NotFound));
}

#[test]
fn spontaneous_exit_is_observed_and_reaped() {
    let mut supervisor = spawn_fake(&[OsStr::new("exit"), OsStr::new("23")]);

    match supervisor
        .wait_timeout(Duration::from_secs(2))
        .expect("espera deve concluir")
    {
        WaitOutcome::Exited(status) => assert_eq!(Some(23), status.code()),
        WaitOutcome::TimedOut(_) => panic!("filho deveria encerrar espontaneamente"),
    }

    assert!(supervisor
        .try_wait()
        .expect("consulta deve funcionar")
        .is_some());
}

#[test]
fn timeout_terminates_and_reaps_the_managed_child() {
    let mut supervisor = spawn_fake(&[OsStr::new("sleep"), OsStr::new("30")]);

    match supervisor
        .wait_timeout(Duration::from_millis(75))
        .expect("timeout deve encerrar o filho")
    {
        WaitOutcome::TimedOut(status) => assert!(!status.success()),
        WaitOutcome::Exited(_) => panic!("filho de longa duração não encerrou espontaneamente"),
    }

    assert!(supervisor
        .try_wait()
        .expect("filho deve ter sido recolhido")
        .is_some());
}

#[test]
fn voluntary_cancel_only_stops_the_managed_child() {
    let mut supervisor = spawn_fake(&[OsStr::new("sleep"), OsStr::new("30")]);
    let mut sentinel = ChildGuard(
        Command::new("sleep")
            .arg("30")
            .spawn()
            .expect("sentinela independente deve iniciar"),
    );

    let status = supervisor
        .stop()
        .expect("cancelamento deve encerrar o filho gerenciado");

    assert!(!status.success());
    assert!(supervisor
        .try_wait()
        .expect("filho gerenciado deve ter sido recolhido")
        .is_some());
    assert!(sentinel
        .0
        .try_wait()
        .expect("consulta à sentinela deve funcionar")
        .is_none());
}

#[test]
fn repeated_teardown_is_idempotent() {
    let mut supervisor = spawn_fake(&[OsStr::new("sleep"), OsStr::new("30")]);

    let first = supervisor.stop().expect("primeiro teardown deve funcionar");
    let second = supervisor.stop().expect("segundo teardown deve funcionar");

    assert_eq!(first.code(), second.code());
    assert_eq!(first.success(), second.success());
}

#[test]
fn dropping_supervisor_kills_and_reaps_its_child() {
    let pid_file = unique_temp_path();
    let pid_file_arg = pid_file.as_os_str();
    let supervisor = ProcessSupervisor::spawn(
        fake_child(),
        [OsStr::new("record-pid"), pid_file_arg, OsStr::new("30")],
    )
    .expect("fake child deve iniciar");
    let deadline = Instant::now() + Duration::from_secs(2);

    let pid = loop {
        if let Ok(contents) = fs::read_to_string(&pid_file) {
            break contents
                .trim()
                .parse::<u32>()
                .expect("PID deve ser numérico");
        }
        assert!(
            Instant::now() < deadline,
            "fake child não publicou PID a tempo"
        );
        thread::sleep(Duration::from_millis(10));
    };

    assert!(PathBuf::from(format!("/proc/{pid}")).exists());
    drop(supervisor);
    assert!(
        !PathBuf::from(format!("/proc/{pid}")).exists(),
        "Drop deve encerrar e recolher o filho administrado"
    );
    let _ = fs::remove_file(pid_file);
}

fn unique_temp_path() -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("relógio deve estar após epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "limivra-process-test-{}-{nonce}",
        std::process::id()
    ))
}
