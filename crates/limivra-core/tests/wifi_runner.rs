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
static FIXTURE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
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
with (root/'launches').open('a') as out: out.write(' '.join(a)+'\n')
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
        let pid = std::fs::read_to_string(self.root.join(name))
            .unwrap_or_else(|error| panic!("missing process marker {name}: {error}"));
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
    let _fixture_lock = FIXTURE_LOCK.lock().await;
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
    let _fixture_lock = FIXTURE_LOCK.lock().await;
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
async fn invalid_utf8_reply_fails_closed() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    let f = Fixture::new("sys.stdout.buffer.write(bytes([255]))");
    let runner = f.runner();
    assert_eq!(
        runner
            .run(
                request(RequestKind::Connect),
                Instant::now() + Duration::from_secs(3),
                Cancellation::default(),
            )
            .await,
        Err(WifiError::UnexpectedReply)
    );
    runner.shutdown().await.unwrap();
    f.assert_reaped("client.pid");
    f.assert_reaped("server.pid");
}

#[tokio::test]
async fn bounded_output_and_timeouts_kill_and_reap_only_owned_processes() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    for (body, expected, timeout) in [
        (
            "print('x'*9000)",
            WifiError::OutputLimit,
            Duration::from_secs(3),
        ),
        ("time.sleep(60)", WifiError::Timeout, Duration::from_secs(3)),
    ] {
        let f = Fixture::new(body);
        let runner = f.runner();
        assert_eq!(
            runner
                .run(
                    request(RequestKind::Connect),
                    Instant::now() + timeout,
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
    let _fixture_lock = FIXTURE_LOCK.lock().await;
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
    let _fixture_lock = FIXTURE_LOCK.lock().await;
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
    let _fixture_lock = FIXTURE_LOCK.lock().await;
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
    let _fixture_lock = FIXTURE_LOCK.lock().await;
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
    let _fixture_lock = FIXTURE_LOCK.lock().await;
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
    let _fixture_lock = FIXTURE_LOCK.lock().await;
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
    let _fixture_lock = FIXTURE_LOCK.lock().await;
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
    let _fixture_lock = FIXTURE_LOCK.lock().await;
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
    let _fixture_lock = FIXTURE_LOCK.lock().await;
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

#[tokio::test]
async fn disappeared_target_and_exact_connection_refusal_are_recoverable() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    for (kind, body) in [
        (RequestKind::Probe, "print(\"error: device '192.168.1.9:4321' not found\",file=sys.stderr); sys.exit(1)"),
        (RequestKind::Connect, "print(\"failed to connect to '192.168.1.9:4321': Connection refused\")"),
        (RequestKind::Connect, "print(\"failed to connect to '192.168.1.9:4321': Connection refused\",file=sys.stderr); sys.exit(1)"),
    ] {
        let f = Fixture::new(body);
        let runner = f.runner();
        let result = runner.run(request(kind), Instant::now() + Duration::from_secs(3), Cancellation::default()).await;
        runner.shutdown().await.unwrap();
        assert_eq!(result, Err(WifiError::Offline));
    }
}

#[tokio::test]
async fn unknown_other_target_and_authorization_failures_are_not_retried_as_offline() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    for (kind, body, expected) in [
        (RequestKind::Probe, "print(\"error: device '192.168.1.10:4321' not found\",file=sys.stderr); sys.exit(1)", WifiError::CommandFailed),
        (RequestKind::Connect, "print(\"failed to connect to '192.168.1.10:4321': Connection refused\")", WifiError::UnexpectedReply),
        (RequestKind::Connect, "print(\"failed to connect to '192.168.1.9:4321': mystery error\")", WifiError::UnexpectedReply),
        (RequestKind::Connect, "print(\"failed to connect to '192.168.1.9:4321': Connection refused\\nextra output\")", WifiError::UnexpectedReply),
        (RequestKind::Connect, "print(\"failed to connect to '192.168.1.9:4321': Connection refused\"); print('unauthorized',file=sys.stderr)", WifiError::Unauthorized),
    ] {
        let f = Fixture::new(body); let runner = f.runner();
        let result = runner.run(request(kind), Instant::now() + Duration::from_secs(3), Cancellation::default()).await;
        runner.shutdown().await.unwrap();
        assert_eq!(result, if expected == WifiError::Unauthorized {
            Ok(AdbReply::Unauthorized)
        } else { Err(expected) });
    }
}

#[tokio::test]
async fn actual_runner_recovers_disappeared_target_with_backoff_without_pair_replay() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    let f = Fixture::new(
        r#"if a[0] == 'pair':
    assert sys.stdin.readline() == '123456\n'
    with (root/'pairs').open('a') as out: out.write('pair\n')
    print('Successfully paired to 192.168.1.9:1234 [guid=fixture]')
elif a[0] == 'connect':
    marker=root/'connects'
    count=int(marker.read_text())+1 if marker.exists() else 1
    marker.write_text(str(count))
    if count in [2,3]: print("failed to connect to '192.168.1.9:4321': Connection refused")
    else: print('connected to 192.168.1.9:4321')
elif a[0] == '-s':
    marker=root/'probes'
    count=int(marker.read_text())+1 if marker.exists() else 1
    marker.write_text(str(count))
    if count == 2:
        print("error: device '192.168.1.9:4321' not found",file=sys.stderr); sys.exit(1)
    print('device')
"#,
    );
    let c = std::sync::Arc::new(WifiController::new(
        f.runner(),
        LocalEndpoint::parse("192.168.1.9:4321").unwrap(),
    ));
    c.pair(
        LocalEndpoint::parse("192.168.1.9:1234").unwrap(),
        PairingCode::parse("123456").unwrap(),
        Consent::Granted,
    )
    .await
    .unwrap();
    c.connect(Consent::Granted, Consent::Granted).await.unwrap();
    let result = c.check_link().await;
    let state = c.state();
    if state != WifiState::Recovering {
        c.cancel().await.unwrap();
        panic!("expected Recovering, got {state:?}, probe {result:?}");
    }
    assert_eq!(result, Err(WifiError::Offline));
    // Keep the real I/O clock here. The mock-clock test in wifi.rs separately
    // pins exact 1/2/4 delays without mixing paused time and OS subprocess I/O.
    let start = Instant::now();
    let mut events = c.subscribe();
    let result = tokio::time::timeout(Duration::from_secs(20), c.recover(Consent::Granted))
        .await
        .unwrap();
    assert!(Instant::now() - start >= Duration::from_secs(7));
    let attempts: Vec<_> = std::iter::from_fn(|| events.try_recv().ok())
        .filter_map(|e| {
            if let WifiEvent::RecoveryAttempt(n) = e {
                Some(n)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(attempts, [1, 2, 3]);
    let state = c.state();
    c.cancel().await.unwrap();
    assert_eq!(result, Ok(()));
    assert_eq!(state, WifiState::Connected);
    assert_eq!(
        std::fs::read_to_string(f.root.join("pairs")).unwrap(),
        "pair\n"
    );
    assert_eq!(
        std::fs::read_to_string(f.root.join("connects")).unwrap(),
        "4"
    );
}

#[tokio::test]
async fn mandatory_key_is_revalidated_after_construction_before_any_process() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    for change in 0..5 {
        let f = Fixture::new("print('device')");
        let runner = f.runner();
        let key = f.home.join(".android/adbkey");
        match change {
            0 => std::fs::remove_file(&key).unwrap(),
            1 => std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o644)).unwrap(),
            2 => {
                std::fs::rename(&key, f.root.join("saved-fixture-key")).unwrap();
                std::fs::write(&key, "replacement fixture identity").unwrap();
                std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
            }
            3 => std::fs::write(&key, "changed in place").unwrap(),
            _ => {
                std::fs::remove_file(&key).unwrap();
                std::fs::create_dir(&key).unwrap();
                std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o700)).unwrap();
            }
        }
        let result = runner
            .run(
                request(RequestKind::Probe),
                Instant::now() + Duration::from_secs(3),
                Cancellation::default(),
            )
            .await;
        runner.shutdown().await.unwrap();
        assert_eq!(
            result,
            Err(WifiError::UnsafeConfiguration),
            "change {change}"
        );
        assert!(
            !f.root.join("launches").exists(),
            "a subprocess was launched for change {change}"
        );
    }
}

#[tokio::test]
async fn key_disappearing_between_help_daemon_and_client_blocks_next_launch() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    for boundary in ["help", "daemon"] {
        let f = Fixture::new("print('device')");
        let script = std::fs::read_to_string(&f.adb).unwrap();
        let script = if boundary == "help" {
            script.replace(
                "if a == ['help']:",
                "if a == ['help']:\n    (root/'home/.android/adbkey').unlink()",
            )
        } else {
            script.replace(
                "s.listen()",
                "(root/'home/.android/adbkey').unlink(); s.listen()",
            )
        };
        std::fs::write(&f.adb, script).unwrap();
        let runner = f.runner();
        let result = runner
            .run(
                request(RequestKind::Probe),
                Instant::now() + Duration::from_secs(3),
                Cancellation::default(),
            )
            .await;
        runner.shutdown().await.unwrap();
        assert_eq!(
            result,
            Err(WifiError::UnsafeConfiguration),
            "boundary {boundary}"
        );
        let launches = std::fs::read_to_string(f.root.join("launches")).unwrap();
        assert!(!launches.contains("get-state") && !launches.contains("kill-server"));
        if boundary == "help" {
            assert_eq!(launches, "help\n");
        } else {
            f.assert_reaped("server.pid");
        }
    }
}

#[tokio::test]
async fn key_removed_before_reuse_or_restart_denies_client_and_new_daemon() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    for restart in [false, true] {
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
        if restart {
            runner.shutdown().await.unwrap();
        }
        let before = std::fs::read_to_string(f.root.join("launches")).unwrap();
        std::fs::remove_file(f.home.join(".android/adbkey")).unwrap();
        let result = runner
            .run(
                request(RequestKind::Probe),
                Instant::now() + Duration::from_secs(3),
                Cancellation::default(),
            )
            .await;
        runner.shutdown().await.unwrap();
        assert_eq!(result, Err(WifiError::UnsafeConfiguration));
        assert_eq!(
            std::fs::read_to_string(f.root.join("launches")).unwrap(),
            before
        );
        f.assert_reaped("server.pid");
    }
}
