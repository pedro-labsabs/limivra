use limivra_core::launcher::{adb_runner::AdbLauncherRunner, *};
use std::{
    os::unix::fs::PermissionsExt,
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

static NEXT: AtomicUsize = AtomicUsize::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new(body: &str) -> Self {
        let dir = std::env::temp_dir().join(format!(
            "limivra-launcher-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&dir).unwrap();
        let bin = dir.join("adb");
        std::fs::write(&bin, format!("#!/bin/sh\n{body}\n")).unwrap();
        std::fs::set_permissions(&bin, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self(dir)
    }
    fn runner(&self) -> AdbLauncherRunner {
        AdbLauncherRunner::new(self.0.join("adb"), "test-target").unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn resolve() -> LauncherRequest {
    LauncherRequest::Resolve {
        package: PackageName::parse("com.android.settings").unwrap(),
    }
}
fn start() -> LauncherRequest {
    LauncherRequest::Start {
        component: ComponentName::parse("com.android.settings/.Settings").unwrap(),
        display_id: 7,
    }
}

#[tokio::test]
async fn exact_resolve_and_start_arguments() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    let f = Fixture::new(
        r#"
if [ "$4" = cmd ]; then
 [ "$*" = '-s test-target shell cmd package resolve-activity --brief -a android.intent.action.MAIN -c android.intent.category.LAUNCHER -p com.android.settings' ] || exit 99
 printf 'com.android.settings/.Settings\n'
else
 [ "$*" = '-s test-target shell am start --display 7 -n com.android.settings/.Settings' ] || exit 98
 printf 'Starting: Intent { cmp=com.android.settings/.Settings }\n'
fi"#,
    );
    assert!(matches!(
        f.runner().run("test-target", resolve()).await,
        Ok(LauncherReply::Resolved(_))
    ));
    assert_eq!(
        f.runner().run("test-target", start()).await,
        Ok(LauncherReply::Accepted)
    );
}

#[tokio::test]
async fn reject_unsafe_config_target_and_packages_without_execution() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    assert!(AdbLauncherRunner::new("adb".into(), "test-target").is_err());
    let f = Fixture::new("exit 99");
    assert!(AdbLauncherRunner::new(f.0.join("adb"), "-bad").is_err());
    assert_eq!(
        f.runner().run("other-target", resolve()).await,
        Err(LaunchError::UnsafeConfiguration)
    );
    let foreign = LauncherRequest::Start {
        component: ComponentName::parse("com.private.app/.Main").unwrap(),
        display_id: 7,
    };
    assert_eq!(
        f.runner().run("test-target", foreign).await,
        Err(LaunchError::NotAllowed)
    );
}

#[tokio::test]
async fn errors_on_either_stream_override_zero_exit_and_success_prefix() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    for (message, expected) in [
        (
            "Error type 3: Activity class does not exist",
            LaunchError::ActivityNotFound,
        ),
        (
            "android.content.ActivityNotFoundException",
            LaunchError::ActivityNotFound,
        ),
        (
            "SecurityException: Permission Denial",
            LaunchError::SecurityException,
        ),
        (
            "SecurityException: Permission Denial: not allowed to launch on display 7",
            LaunchError::DisplayDenied,
        ),
        (
            "Error: Display 7 does not exist",
            LaunchError::DisplayDenied,
        ),
        ("error: device unauthorized", LaunchError::Unauthorized),
        ("error: device offline", LaunchError::Offline),
    ] {
        for redirect in ["", " >&2"] {
            let f=Fixture::new(&format!("printf 'Starting: Intent {{ cmp=com.android.settings/.Settings }}\\n'\nprintf '%s\\n' '{message}'{redirect}"));
            assert_eq!(
                f.runner().run("test-target", start()).await,
                Err(expected),
                "{message}"
            );
        }
    }
}

#[tokio::test]
async fn resolution_fails_closed_for_ambiguous_foreign_and_malformed_output() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    for output in [
        "com.private.app/.Main",
        "com.android.settings/.A;id",
        "com.android.settings/.A\ncom.android.settings/.B",
        "unknown",
        "",
        "com.android.settings/.Settings\nprivate text",
    ] {
        let f = Fixture::new(&format!("printf '%s\\n' '{output}'"));
        assert_eq!(
            f.runner().run("test-target", resolve()).await,
            Err(LaunchError::UnexpectedReply),
            "fixture output: {output:?}"
        );
    }
    let f = Fixture::new("printf 'No activity found\\n'");
    assert_eq!(
        f.runner().run("test-target", resolve()).await,
        Ok(LauncherReply::NotInstalled)
    );
    let f = Fixture::new("printf 'Starting: nonsense\\n'");
    assert_eq!(
        f.runner().run("test-target", start()).await,
        Err(LaunchError::UnexpectedReply)
    );
}

#[tokio::test]
async fn invalid_utf8_reply_fails_closed_without_accepting_prefixes() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    for body in ["printf '\\377'", "printf '\\377' >&2"] {
        let f = Fixture::new(body);
        assert_eq!(
            f.runner().run("test-target", resolve()).await,
            Err(LaunchError::UnexpectedReply)
        );
    }
}

#[tokio::test]
async fn display_observation_uses_typed_ids_without_names() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    let f = Fixture::new(
        r#"
case "$*" in
 '-s test-target shell cmd display get-displays --ids-only') printf '0\n7\n';;
 '-s test-target shell cmd display get-displays --ids-only --type VIRTUAL') printf '7\n';;
 *) exit 99;;
esac
"#,
    );
    assert_eq!(
        f.runner()
            .run("test-target", LauncherRequest::Displays)
            .await,
        Ok(LauncherReply::Displays(vec![
            ObservedDisplay {
                id: 0,
                is_virtual: false
            },
            ObservedDisplay {
                id: 7,
                is_virtual: true
            }
        ]))
    );
}

#[tokio::test]
async fn inner_class_dollar_is_quoted_for_android_shell() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    let f = Fixture::new(
        r#"
[ "$9" = "'com.android.settings/.Settings\$Network'" ] || exit 99
printf 'Starting: Intent { cmp=com.android.settings/.Settings$Network }\n'
"#,
    );
    let request = LauncherRequest::Start {
        component: ComponentName::parse("com.android.settings/.Settings$Network").unwrap(),
        display_id: 7,
    };
    assert_eq!(
        f.runner().run("test-target", request).await,
        Ok(LauncherReply::Accepted)
    );
}

#[tokio::test]
async fn timeout_and_overflow_reap_only_owned_process() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    let f = Fixture::new("exec sleep 30");
    let result = f.runner().run("test-target", resolve()).await;
    assert_eq!(result, Err(LaunchError::Timeout));
    let f = Fixture::new("head -c 17000 /dev/zero");
    assert_eq!(
        f.runner().run("test-target", resolve()).await,
        Err(LaunchError::OutputLimit)
    );
}

#[tokio::test]
async fn cancellation_reaps_the_running_client() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    let f = Fixture::new("printf '%s' $$ > \"$(dirname \"$0\")/pid\"\nexec sleep 30");
    let runner = f.runner();
    let task = tokio::spawn(async move { runner.run("test-target", resolve()).await });
    let pidfile = f.0.join("pid");
    for _ in 0..200 {
        if pidfile.exists() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    let pid = std::fs::read_to_string(pidfile).unwrap();
    task.abort();
    let _ = task.await;
    for _ in 0..200 {
        if !PathBuf::from(format!("/proc/{pid}")).exists() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    panic!("owned client survived cancellation");
}

#[tokio::test]
async fn provider_transport_stop_after_monitor_start_reaps_actual_child() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    let f = Fixture::new("printf '%s' $$ > \"$(dirname \"$0\")/pid\"\nexec sleep 30");
    let session = limivra_core::SessionController::new();
    session
        .start(
            f.0.join("adb").to_str().unwrap(),
            &[],
            "test-target",
            limivra_core::DisplayMode::Mirror,
        )
        .await
        .unwrap();
    let pidfile = f.0.join("pid");
    for _ in 0..200 {
        if pidfile.exists() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    }
    let pid = std::fs::read_to_string(pidfile).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    session.stop().await.unwrap();
    let exists = PathBuf::from(format!("/proc/{pid}")).exists();
    if exists {
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &pid])
            .status();
    }
    assert!(
        !exists,
        "owned transport survives stop after monitor takes child"
    );
}

// Serialize executable fixture writes against forks in other tests; otherwise a
// concurrent child can briefly inherit a writer FD and cause Linux ETXTBSY.
static FIXTURE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
async fn android_brief_resolution_header_is_strictly_parsed() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    for header in [
        "priority=0 preferredOrder=0 match=0x108000 specificIndex=-1 isDefault=true",
        "priority=-1 preferredOrder=2 match=0x0 specificIndex=0 isDefault=false",
    ] {
        let f = Fixture::new(&format!(
            "printf '%s\\n' '{header}' 'com.android.settings/.MainSettings'"
        ));
        assert_eq!(
            f.runner().run("test-target", resolve()).await,
            Ok(LauncherReply::Resolved(
                ComponentName::parse("com.android.settings/.MainSettings").unwrap()
            ))
        );
    }
    for header in [
        "priority=private data",
        "priority=0 preferredOrder=0 match=0x108000 specificIndex=-1 isDefault=true extra=data",
    ] {
        let f = Fixture::new(&format!(
            "printf '%s\\n' '{header}' 'com.android.settings/.MainSettings'"
        ));
        assert_eq!(
            f.runner().run("test-target", resolve()).await,
            Err(LaunchError::UnexpectedReply)
        );
    }
}

#[tokio::test]
async fn concurrent_display_creation_does_not_bind_an_untyped_id() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    let f = Fixture::new(
        r#"
state="$(dirname "$0")/count"
n=0; [ ! -f "$state" ] || n=$(cat "$state")
n=$((n+1)); printf '%s' "$n" > "$state"
case "$*" in
 *'--type VIRTUAL') [ "$n" -eq 1 ] || printf '7\n';;
 *) printf '0\n'; [ "$n" -eq 1 ] || printf '7\n';;
esac
"#,
    );
    assert_eq!(
        f.runner()
            .run("test-target", LauncherRequest::Displays)
            .await,
        Ok(LauncherReply::Displays(vec![
            ObservedDisplay {
                id: 0,
                is_virtual: false
            },
            ObservedDisplay {
                id: 7,
                is_virtual: false
            }
        ]))
    );
    assert_eq!(
        f.runner()
            .run("test-target", LauncherRequest::Displays)
            .await,
        Ok(LauncherReply::Displays(vec![
            ObservedDisplay {
                id: 0,
                is_virtual: false
            },
            ObservedDisplay {
                id: 7,
                is_virtual: true
            }
        ]))
    );
}

#[tokio::test]
async fn output_budget_is_combined_and_unknown_replies_fail_closed() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    let f = Fixture::new("head -c 9000 /dev/zero\nhead -c 9000 /dev/zero >&2");
    assert_eq!(
        f.runner().run("test-target", resolve()).await,
        Err(LaunchError::OutputLimit)
    );
    for body in [
        "exit 12",
        "printf 'Starting: Intent { cmp=com.android.settings/.Settings }\\nError: unknown\\n'",
        "printf 'Starting: Intent { cmp=com.android.settings/.SettingsOther }\\n'",
    ] {
        let f = Fixture::new(body);
        assert!(f.runner().run("test-target", start()).await.is_err());
    }
}

#[tokio::test]
async fn display_query_rejects_duplicate_or_non_numeric_records() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    for reply in [
        "0\n0",
        "Display id 7",
        "7;id",
        "4294967296",
        "-1",
        "7\nprivate text",
    ] {
        let f = Fixture::new(&format!("printf '%s\\n' '{reply}'"));
        assert_eq!(
            f.runner()
                .run("test-target", LauncherRequest::Displays)
                .await,
            Err(LaunchError::UnexpectedReply)
        );
    }
}
