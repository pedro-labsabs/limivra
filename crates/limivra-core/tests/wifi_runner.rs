#![cfg(target_os = "linux")]
use limivra_core::wifi::{adb_runner::PrivateAdbRunner, *};
use std::{
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};
use tokio::time::Instant;
static NEXT: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    root: PathBuf,
    home: PathBuf,
    runtime: PathBuf,
    adb: PathBuf,
}
impl Fixture {
    fn new(body: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "limivra-wifi-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&root).unwrap();
        std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700)).unwrap();
        let home = root.join("home");
        let runtime = root.join("runtime");
        for p in [&home, &home.join(".android"), &runtime] {
            std::fs::create_dir(p).unwrap();
            std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let key = home.join(".android/adbkey");
        std::fs::write(&key, "synthetic fixture key").unwrap();
        std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
        let adb = root.join("fake-adb");
        let script = format!(
            r#"#!/usr/bin/python3
import sys,os,socket,time,signal
from pathlib import Path
root=Path(__file__).parent
a=sys.argv[1:]
assert os.environ.get('LC_ALL') == 'C'
assert os.environ.get('ADB_MDNS_AUTO_CONNECT') == '0'
assert os.environ.get('ADB_MDNS') == '0'
assert os.environ.get('ADB_USB') == '0'
assert os.environ.get('ADB_EMU') == '0'
assert not any(k in os.environ for k in ['ADB_TRACE','ANDROID_SERIAL','ADB_SERVER_SOCKET','ADB_VENDOR_KEYS','ANDROID_ADB_SERVER_PORT'])
assert os.environ.get('HOME') == str(root/'home')
assert not any('123456' in s for s in a)
if a == ['help']:
    print('-L SOCKET pair server nodaemon')
    sys.exit(0)
assert a[:1] == ['-L'] and a[1] == 'localfilesystem:'+str(root/'runtime'/'adb.sock')
a=a[2:]
if a == ['kill-server']:
    (root/'private-stop').write_text('owned socket only')
    try: os.kill(int((root/'server.pid').read_text()),signal.SIGKILL)
    except ProcessLookupError: pass
    sys.exit(0)
if a == ['server','nodaemon']:
    (root/'server.pid').write_text(str(os.getpid()))
    s=socket.socket(socket.AF_UNIX); s.bind(str(root/'runtime'/'adb.sock')); s.listen()
    while True:
        c,_=s.accept(); c.close()
(root/'client.pid').write_text(str(os.getpid()))
{body}
"#
        );
        std::fs::write(&adb, script).unwrap();
        std::fs::set_permissions(&adb, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self {
            root,
            home,
            runtime,
            adb,
        }
    }
    fn runner(&self) -> PrivateAdbRunner {
        PrivateAdbRunner::new(self.adb.clone(), self.home.clone(), self.runtime.clone()).unwrap()
    }
    fn assert_reaped(&self, name: &str) {
        let pid = std::fs::read_to_string(self.root.join(name)).unwrap();
        assert!(
            !PathBuf::from(format!("/proc/{pid}")).exists(),
            "process not reaped"
        );
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}
fn request(kind: RequestKind) -> AdbRequest {
    AdbRequest {
        kind,
        endpoint: LocalEndpoint::parse("192.168.1.9:4321").unwrap(),
        code: if kind == RequestKind::Pair {
            Some(PairingCode::parse("123456").unwrap())
        } else {
            None
        },
    }
}
#[tokio::test]
async fn private_server_stdin_and_exact_target_commands() {
    let f = Fixture::new(
        r#"if a == ['pair','192.168.1.9:4321']:
    assert sys.stdin.readline() == '123456\n'
    print('Enter pairing code: Successfully paired to 192.168.1.9:4321 [guid=synthetic]')
elif a == ['connect','192.168.1.9:4321']:
    print('connected to 192.168.1.9:4321')
elif a == ['-s','192.168.1.9:4321','get-state']:
    print('device')
elif a == ['disconnect','192.168.1.9:4321']:
    print('disconnected 192.168.1.9:4321')
else:
    sys.exit(4)"#,
    );
    let runner = f.runner();
    for (kind, expected) in [
        (RequestKind::Pair, AdbReply::Paired),
        (RequestKind::Connect, AdbReply::Connected),
        (RequestKind::Probe, AdbReply::Device),
        (RequestKind::Disconnect, AdbReply::Disconnected),
    ] {
        assert_eq!(
            runner
                .run(
                    request(kind),
                    Instant::now() + Duration::from_secs(3),
                    Cancellation::default()
                )
                .await,
            Ok(expected)
        );
    }
    runner.shutdown().await.unwrap();
    f.assert_reaped("server.pid");
    f.assert_reaped("client.pid");
    assert!(!f.runtime.join("adb.sock").exists());
}
#[tokio::test]
async fn false_success_and_echoed_secrets_never_escape_as_errors() {
    for body in [
        "print('failed to connect to 192.168.1.9:4321')",
        "print('123456 private-output'); print('123456 private-stderr',file=sys.stderr)",
    ] {
        let f = Fixture::new(body);
        let runner = f.runner();
        let error = runner
            .run(
                request(RequestKind::Connect),
                Instant::now() + Duration::from_secs(3),
                Cancellation::default(),
            )
            .await
            .unwrap_err();
        assert_eq!(error, WifiError::UnexpectedReply);
        assert!(!format!("{error:?} {error}").contains("123456"));
        runner.shutdown().await.unwrap();
    }
}
#[tokio::test]
async fn bounded_output_and_timeouts_kill_and_reap_only_owned_processes() {
    for (body, expected) in [
        ("print('x'*9000)", WifiError::OutputLimit),
        ("time.sleep(60)", WifiError::Timeout),
    ] {
        let f = Fixture::new(body);
        let runner = f.runner();
        assert_eq!(
            runner
                .run(
                    request(RequestKind::Connect),
                    Instant::now() + Duration::from_millis(300),
                    Cancellation::default()
                )
                .await,
            Err(expected)
        );
        runner.shutdown().await.unwrap();
        f.assert_reaped("client.pid");
        f.assert_reaped("server.pid");
    }
}
#[tokio::test]
async fn rejects_unsafe_storage_and_preexisting_socket_without_touching_it() {
    let f = Fixture::new("print('device')");
    std::fs::set_permissions(&f.home, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(
        PrivateAdbRunner::new(f.adb.clone(), f.home.clone(), f.runtime.clone()),
        Err(WifiError::UnsafeConfiguration)
    ));
    std::fs::set_permissions(&f.home, std::fs::Permissions::from_mode(0o700)).unwrap();
    std::fs::write(f.runtime.join("adb.sock"), "do not touch").unwrap();
    assert!(matches!(
        PrivateAdbRunner::new(f.adb.clone(), f.home.clone(), f.runtime.clone()),
        Err(WifiError::UnsafeConfiguration)
    ));
    assert_eq!(
        std::fs::read_to_string(f.runtime.join("adb.sock")).unwrap(),
        "do not touch"
    );
    assert!(!f.root.join("server.pid").exists());
}

async fn wait_for(path: &std::path::Path) {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !path.exists() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn cancellation_during_each_client_operation_reaps_clients_and_server() {
    for kind in [RequestKind::Pair, RequestKind::Connect, RequestKind::Probe] {
        let f = Fixture::new("time.sleep(60)");
        let runner = f.runner();
        let cancellation = Cancellation::default();
        let token = cancellation.clone();
        let worker = runner.clone();
        let task = tokio::spawn(async move {
            worker
                .run(
                    request(kind),
                    Instant::now() + Duration::from_secs(10),
                    token,
                )
                .await
        });
        wait_for(&f.root.join("client.pid")).await;
        cancellation.cancel();
        assert_eq!(task.await.unwrap(), Err(WifiError::Cancelled));
        runner.shutdown().await.unwrap();
        f.assert_reaped("client.pid");
        f.assert_reaped("server.pid");
    }
}
#[tokio::test]
async fn dropping_the_call_future_still_runs_supervised_cleanup() {
    let f = Fixture::new("time.sleep(60)");
    let runner = f.runner();
    let worker = runner.clone();
    let task = tokio::spawn(async move {
        worker
            .run(
                request(RequestKind::Connect),
                Instant::now() + Duration::from_secs(10),
                Cancellation::default(),
            )
            .await
    });
    wait_for(&f.root.join("client.pid")).await;
    task.abort();
    let _ = task.await;
    runner.shutdown().await.unwrap();
    f.assert_reaped("client.pid");
    f.assert_reaped("server.pid");
}
#[tokio::test]
async fn unsupported_backend_and_stderr_overflow_fail_closed() {
    let f = Fixture::new("print('device')");
    let script = std::fs::read_to_string(&f.adb).unwrap().replace(
        "print('-L SOCKET pair server nodaemon')",
        "print('legacy backend')",
    );
    std::fs::write(&f.adb, script).unwrap();
    let runner = f.runner();
    assert_eq!(
        runner
            .run(
                request(RequestKind::Probe),
                Instant::now() + Duration::from_secs(3),
                Cancellation::default()
            )
            .await,
        Err(WifiError::DependencyUnsupported)
    );
    assert!(!f.root.join("server.pid").exists());
    let f = Fixture::new("print('x'*9000,file=sys.stderr)");
    let runner = f.runner();
    assert_eq!(
        runner
            .run(
                request(RequestKind::Probe),
                Instant::now() + Duration::from_secs(3),
                Cancellation::default()
            )
            .await,
        Err(WifiError::OutputLimit)
    );
    runner.shutdown().await.unwrap();
    f.assert_reaped("server.pid");
    f.assert_reaped("client.pid");
}
#[tokio::test]
async fn hostile_parent_environment_is_not_inherited() {
    if std::env::var_os("LIMIVRA_WIFI_TEST_CHILD").is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "hostile_parent_environment_is_not_inherited"])
            .env("LIMIVRA_WIFI_TEST_CHILD", "1")
            .env("ADB_TRACE", "all")
            .env("ANDROID_SERIAL", "unapproved-target")
            .env("ADB_VENDOR_KEYS", "/not-approved")
            .env("ADB_SERVER_SOCKET", "tcp:unapproved:1234")
            .env("ANDROID_ADB_SERVER_PORT", "1234")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "isolated test failed: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        return;
    }
    let f = Fixture::new("print('device')");
    let runner = f.runner();
    assert_eq!(
        runner
            .run(
                request(RequestKind::Probe),
                Instant::now() + Duration::from_secs(3),
                Cancellation::default()
            )
            .await,
        Ok(AdbReply::Device)
    );
    runner.shutdown().await.unwrap();
}
#[tokio::test]
async fn key_store_symlinks_and_public_files_are_rejected() {
    let f = Fixture::new("print('device')");
    let path = f.home.join(".android/extra");
    std::os::unix::fs::symlink("/dev/null", &path).unwrap();
    assert!(matches!(
        PrivateAdbRunner::new(f.adb.clone(), f.home.clone(), f.runtime.clone()),
        Err(WifiError::UnsafeConfiguration)
    ));
    std::fs::remove_file(&path).unwrap();
    let path = f.home.join(".android/adbkey");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(matches!(
        PrivateAdbRunner::new(f.adb.clone(), f.home.clone(), f.runtime.clone()),
        Err(WifiError::UnsafeConfiguration)
    ));
}

#[tokio::test]
async fn shutdown_also_targets_the_owned_socket_to_stop_backend_restarts() {
    let f = Fixture::new("print('device')");
    let runner = f.runner();
    runner
        .run(
            request(RequestKind::Probe),
            Instant::now() + Duration::from_secs(3),
            Cancellation::default(),
        )
        .await
        .unwrap();
    runner.shutdown().await.unwrap();
    assert!(f.root.join("private-stop").exists());
    f.assert_reaped("server.pid");
}

#[tokio::test]
async fn daemon_exit_during_command_cannot_confirm_a_connection() {
    let f = Fixture::new("os.kill(int((root/'server.pid').read_text()),signal.SIGKILL)\ntime.sleep(0.02)\nprint('connected to 192.168.1.9:4321')");
    let runner = f.runner();
    assert_eq!(
        runner
            .run(
                request(RequestKind::Connect),
                Instant::now() + Duration::from_secs(3),
                Cancellation::default()
            )
            .await,
        Err(WifiError::Offline)
    );
    runner.shutdown().await.unwrap();
    f.assert_reaped("client.pid");
    f.assert_reaped("server.pid");
}

#[tokio::test]
async fn offline_and_unauthorized_are_typed_for_recovery_policy() {
    for (body, expected) in [
        (
            "print('error: device offline',file=sys.stderr); sys.exit(1)",
            WifiError::Offline,
        ),
        (
            "print('error: device unauthorized.',file=sys.stderr); sys.exit(1)",
            WifiError::Unauthorized,
        ),
    ] {
        let f = Fixture::new(body);
        let runner = f.runner();
        assert_eq!(
            runner
                .run(
                    request(RequestKind::Probe),
                    Instant::now() + Duration::from_secs(3),
                    Cancellation::default()
                )
                .await,
            Err(expected)
        );
        runner.shutdown().await.unwrap();
    }
}
