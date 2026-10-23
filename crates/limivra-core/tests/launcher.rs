use limivra_core::launcher::{AllowedApp, ComponentName, PackageName};

#[test]
fn only_explicit_apps_are_allowed() {
    assert_eq!(AllowedApp::Settings.packages(), &["com.android.settings"]);
    assert_eq!(
        AllowedApp::Calculator.packages(),
        &["com.miui.calculator", "com.google.android.calculator"]
    );
    assert_eq!(
        AllowedApp::from_package("com.android.settings"),
        Some(AllowedApp::Settings)
    );
    assert_eq!(AllowedApp::from_package("com.private.app"), None);
}

#[test]
fn package_validation_rejects_shell_and_malformed_identifiers() {
    for bad in [
        "",
        "settings",
        "com..app",
        "com.1bad",
        "-com.app",
        "com.app;id",
        "com.app\n",
        "com.app x",
        "com.é",
        "com.a\0",
        "com.a'",
        "com.`id`",
        "com.$(id)",
        "com.a/b",
    ] {
        assert!(PackageName::parse(bad).is_err(), "accepted {bad:?}");
    }
    assert!(PackageName::parse(&format!("com.{}", "a".repeat(252))).is_err());
    assert_eq!(
        PackageName::parse("com.android.settings").unwrap().as_str(),
        "com.android.settings"
    );
}

#[test]
fn component_validation_allows_inner_classes_but_no_shell_expansion() {
    for good in [
        "com.android.settings/.Settings",
        "com.android.settings/com.android.settings.Settings$Network",
        "com.miui.calculator/.Calculator",
    ] {
        assert_eq!(ComponentName::parse(good).unwrap().as_str(), good);
    }
    for bad in [
        "com.a",
        "com.a/",
        "com.a/Main",
        "com.a//Main",
        "com.a/.Main;id",
        "com.a/.$Bad",
        "com.a/.$(id)",
        "com.a/.Main\n",
        "com.a/é",
        "com.a/.A B",
    ] {
        assert!(ComponentName::parse(bad).is_err(), "accepted {bad:?}");
    }
    assert!(ComponentName::parse(&format!("com.a/.{}", "A".repeat(506))).is_err());
}

use limivra_core::{
    launcher::*, DisplayMode, ScrcpyCapabilities, ScrcpySessionOptions, SessionState,
    VirtualDisplayConfig, VirtualDisplayProvider,
};
use std::{
    collections::VecDeque, future::Future, os::unix::fs::PermissionsExt, path::PathBuf, pin::Pin,
    sync::Mutex,
};
struct Mock {
    replies: Mutex<VecDeque<Result<LauncherReply, LaunchError>>>,
    calls: Mutex<Vec<LauncherRequest>>,
}
impl Mock {
    fn new(replies: Vec<Result<LauncherReply, LaunchError>>) -> Self {
        Self {
            replies: Mutex::new(replies.into()),
            calls: Mutex::new(vec![]),
        }
    }
}
impl LauncherRunner for Mock {
    fn run<'a>(
        &'a self,
        _: &'a str,
        request: LauncherRequest,
    ) -> Pin<Box<dyn Future<Output = Result<LauncherReply, LaunchError>> + Send + 'a>> {
        self.calls.lock().unwrap().push(request);
        let reply = self
            .replies
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected backend call");
        Box::pin(async move { reply })
    }
}
fn displays(ids: &[u32]) -> Result<LauncherReply, LaunchError> {
    Ok(LauncherReply::Displays(
        ids.iter()
            .map(|id| ObservedDisplay {
                id: *id,
                is_virtual: *id != 0,
            })
            .collect(),
    ))
}
fn resolved(package: &str) -> Result<LauncherReply, LaunchError> {
    Ok(LauncherReply::Resolved(
        ComponentName::parse(&format!("{package}/.Main")).unwrap(),
    ))
}
struct Transport(PathBuf);
impl Transport {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "limivra-launcher-transport-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::write(&path, "#!/bin/sh\nexec sleep 30\n").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
}
impl Drop for Transport {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}
async fn active(
    provider: &VirtualDisplayProvider,
    runner: &impl LauncherRunner,
    transport: &Transport,
) -> Result<(), LaunchError> {
    let mut caps = ScrcpyCapabilities::unknown();
    caps.supports_new_display = true;
    let options = ScrcpySessionOptions {
        stay_awake: false,
        virtual_config: Some(VirtualDisplayConfig::new(1280, 720, 160)),
    };
    provider
        .start_launcher_session(
            transport.0.to_str().unwrap(),
            "test-target",
            &options,
            &caps,
            runner,
        )
        .await
}

#[tokio::test]
async fn inactive_provider_never_calls_adb() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    let provider = VirtualDisplayProvider::new();
    let runner = Mock::new(vec![]);
    assert_eq!(
        provider
            .launch_allowed_app(AllowedApp::Settings, &runner)
            .await,
        Err(LaunchError::NotActive)
    );
    assert!(runner.calls.lock().unwrap().is_empty());
}

#[tokio::test]
async fn launches_two_allowed_apps_on_unique_new_display() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    let p = VirtualDisplayProvider::new();
    let t = Transport::new();
    let r = Mock::new(vec![
        displays(&[0, 2]),
        displays(&[0, 2, 7]),
        resolved("com.android.settings"),
        displays(&[0, 2, 7]),
        Ok(LauncherReply::Accepted),
        Ok(LauncherReply::NotInstalled),
        resolved("com.google.android.calculator"),
        displays(&[0, 2, 7]),
        Ok(LauncherReply::Accepted),
    ]);
    active(&p, &r, &t).await.unwrap();
    for app in [AllowedApp::Settings, AllowedApp::Calculator] {
        assert_eq!(
            p.launch_allowed_app(app, &r).await,
            Ok(LaunchOutcome::Accepted { display_id: 7 })
        );
    }
    let starts: Vec<_> = r
        .calls
        .lock()
        .unwrap()
        .iter()
        .filter_map(|q| {
            if let LauncherRequest::Start { display_id, .. } = q {
                Some(*display_id)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(starts, vec![7, 7]);
    p.stop().await.unwrap();
}

#[tokio::test]
async fn security_error_preserves_session_and_does_not_try_second_calculator() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    let p = VirtualDisplayProvider::new();
    let t = Transport::new();
    let r = Mock::new(vec![
        displays(&[0]),
        displays(&[0, 7]),
        Err(LaunchError::SecurityException),
        resolved("com.android.settings"),
        displays(&[0, 7]),
        Ok(LauncherReply::Accepted),
    ]);
    active(&p, &r, &t).await.unwrap();
    assert_eq!(
        p.launch_allowed_app(AllowedApp::Calculator, &r).await,
        Err(LaunchError::SecurityException)
    );
    assert!(matches!(
        p.current_state().await,
        SessionState::Active {
            mode: DisplayMode::VirtualDisplay { .. },
            ..
        }
    ));
    assert_eq!(
        p.launch_allowed_app(AllowedApp::Settings, &r).await,
        Ok(LaunchOutcome::Accepted { display_id: 7 })
    );
    p.stop().await.unwrap();
}

#[tokio::test]
async fn foreign_resolved_component_and_disappeared_display_never_launch() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    for reply in [
        resolved("com.private.app"),
        resolved("com.android.settings"),
    ] {
        let p = VirtualDisplayProvider::new();
        let t = Transport::new();
        let r = Mock::new(vec![
            displays(&[0]),
            displays(&[0, 7]),
            reply,
            displays(&[0]),
        ]);
        active(&p, &r, &t).await.unwrap();
        assert!(p
            .launch_allowed_app(AllowedApp::Settings, &r)
            .await
            .is_err());
        assert!(!r
            .calls
            .lock()
            .unwrap()
            .iter()
            .any(|q| matches!(q, LauncherRequest::Start { .. })));
        p.stop().await.unwrap();
    }
}

#[tokio::test]
async fn ambiguous_new_displays_fail_closed_and_stop_session() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    let p = VirtualDisplayProvider::new();
    let t = Transport::new();
    let r = Mock::new(vec![displays(&[0]), displays(&[0, 7, 8])]);
    assert_eq!(active(&p, &r, &t).await, Err(LaunchError::DisplayDenied));
    assert_eq!(p.current_state().await, SessionState::Disconnected);
}

struct BlockedRunner {
    inner: Mock,
    entered: tokio::sync::Notify,
    block_start: bool,
}
impl LauncherRunner for BlockedRunner {
    fn run<'a>(
        &'a self,
        target: &'a str,
        request: LauncherRequest,
    ) -> Pin<Box<dyn Future<Output = Result<LauncherReply, LaunchError>> + Send + 'a>> {
        let block = if self.block_start {
            matches!(request, LauncherRequest::Start { .. })
        } else {
            matches!(request, LauncherRequest::Resolve { .. })
        };
        if block {
            self.inner.calls.lock().unwrap().push(request);
            Box::pin(async move {
                self.entered.notify_one();
                std::future::pending().await
            })
        } else {
            self.inner.run(target, request)
        }
    }
}

#[tokio::test]
async fn stop_cancels_pending_resolution_or_start_and_old_context_cannot_relaunch() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    for block_start in [false, true] {
        let p = std::sync::Arc::new(VirtualDisplayProvider::new());
        let t = Transport::new();
        let r = std::sync::Arc::new(BlockedRunner {
            inner: Mock::new(vec![
                displays(&[0]),
                displays(&[0, 7]),
                resolved("com.android.settings"),
                displays(&[0, 7]),
            ]),
            entered: tokio::sync::Notify::new(),
            block_start,
        });
        active(&p, &*r, &t).await.unwrap();
        let p2 = p.clone();
        let r2 = r.clone();
        let task =
            tokio::spawn(async move { p2.launch_allowed_app(AllowedApp::Settings, &*r2).await });
        r.entered.notified().await;
        p.stop().await.unwrap();
        let replacement_runner = Mock::new(vec![
            displays(&[0]),
            displays(&[0, 9]),
            resolved("com.android.settings"),
            displays(&[0, 9]),
            Ok(LauncherReply::Accepted),
        ]);
        active(&p, &replacement_runner, &t).await.unwrap();
        assert!(matches!(
            p.current_state().await,
            SessionState::Active {
                mode: DisplayMode::VirtualDisplay { .. },
                ..
            }
        ));
        assert_eq!(task.await.unwrap(), Err(LaunchError::StaleContext));
        let old_starts = r
            .inner
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter(|call| matches!(call, LauncherRequest::Start { .. }))
            .count();
        assert_eq!(old_starts, usize::from(block_start));
        assert_eq!(
            p.launch_allowed_app(AllowedApp::Settings, &replacement_runner)
                .await,
            Ok(LaunchOutcome::Accepted { display_id: 9 })
        );
        p.stop().await.unwrap();
    }
}

#[tokio::test]
async fn observed_display_removal_invalidates_binding_even_if_id_reappears() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    let p = VirtualDisplayProvider::new();
    let t = Transport::new();
    let r = Mock::new(vec![
        displays(&[0]),
        displays(&[0, 7]),
        resolved("com.android.settings"),
        displays(&[0]),
        resolved("com.android.settings"),
        displays(&[0, 7]),
        Ok(LauncherReply::Accepted),
    ]);
    active(&p, &r, &t).await.unwrap();
    assert_eq!(
        p.launch_allowed_app(AllowedApp::Settings, &r).await,
        Err(LaunchError::DisplayDenied)
    );
    assert_eq!(
        p.launch_allowed_app(AllowedApp::Settings, &r).await,
        Err(LaunchError::NotActive)
    );
    p.stop().await.unwrap();
}

// Serialize executable fixture writes against forks in other tests; otherwise a
// concurrent child can briefly inherit a writer FD and cause Linux ETXTBSY.
static FIXTURE_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[tokio::test]
async fn restart_binds_new_generation_and_never_reuses_previous_display() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    let p = VirtualDisplayProvider::new();
    let t = Transport::new();
    let first = Mock::new(vec![displays(&[0]), displays(&[0, 7])]);
    active(&p, &first, &t).await.unwrap();
    p.stop().await.unwrap();
    let second = Mock::new(vec![
        displays(&[0, 7]),
        displays(&[0, 7, 8]),
        resolved("com.android.settings"),
        displays(&[0, 7, 8]),
        Ok(LauncherReply::Accepted),
    ]);
    active(&p, &second, &t).await.unwrap();
    assert_eq!(
        p.launch_allowed_app(AllowedApp::Settings, &second).await,
        Ok(LaunchOutcome::Accepted { display_id: 8 })
    );
    p.stop().await.unwrap();
}

#[tokio::test]
async fn transport_exit_cancels_pending_resolution() {
    let _fixture_lock = FIXTURE_LOCK.lock().await;
    let p = std::sync::Arc::new(VirtualDisplayProvider::new());
    let t = Transport::new();
    std::fs::write(&t.0, "#!/bin/sh\nexec sleep 0.2\n").unwrap();
    let r = std::sync::Arc::new(BlockedRunner {
        inner: Mock::new(vec![displays(&[0]), displays(&[0, 7])]),
        entered: tokio::sync::Notify::new(),
        block_start: false,
    });
    active(&p, &*r, &t).await.unwrap();
    let p2 = p.clone();
    let r2 = r.clone();
    let task = tokio::spawn(async move { p2.launch_allowed_app(AllowedApp::Settings, &*r2).await });
    r.entered.notified().await;
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(2), task)
            .await
            .unwrap()
            .unwrap(),
        Err(LaunchError::StaleContext)
    );
    assert_eq!(
        p.launch_allowed_app(AllowedApp::Settings, &*r).await,
        Err(LaunchError::StaleContext)
    );
    p.stop().await.unwrap();
}
