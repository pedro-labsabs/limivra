use limivra_core::input_policy::*;
use limivra_core::{DisplayMode, SessionEvent, SessionState};

fn active(p: &mut InputPolicy) {
    p.observe_session(&SessionEvent::StateChanged(SessionState::Active {
        serial: "private-device".into(),
        mode: DisplayMode::Mirror,
    }));
}
fn caps() -> InputCapabilities {
    InputCapabilities {
        sdk: Support::Supported,
        uhid: Support::Supported,
        host_to_android: Support::Supported,
        android_to_host: Support::Supported,
    }
}
fn ready() -> InputPolicy {
    let mut p = InputPolicy::default();
    active(&mut p);
    p.set_capabilities(caps());
    p.select_mode(InputMode::Sdk).unwrap();
    p.set_focus(true);
    p.set_input_consent(true);
    p
}
fn press(control: Control) -> InputEvent {
    InputEvent {
        control,
        pressed: true,
    }
}
fn release(control: Control) -> InputEvent {
    InputEvent {
        control,
        pressed: false,
    }
}
const KEY: Control = Control::Key(30);

#[test]
fn input_requires_active_focus_consent_and_explicit_supported_mode() {
    let mut p = InputPolicy::default();
    assert_eq!(p.route(press(KEY)), Err(InputDenied::NotActive));
    // Grants made before a session must not carry into it.
    p.set_focus(true);
    p.set_input_consent(true);
    active(&mut p);
    assert_eq!(p.route(press(KEY)), Err(InputDenied::NotFocused));
    p.set_focus(true);
    assert_eq!(p.route(press(KEY)), Err(InputDenied::NoConsent));
    p.set_input_consent(true);
    assert_eq!(p.route(press(KEY)), Err(InputDenied::NoMode));
    assert_eq!(
        p.select_mode(InputMode::Sdk),
        Err(InputDenied::UnsupportedMode)
    );
    p.set_capabilities(caps());
    p.select_mode(InputMode::Sdk).unwrap();
    assert_eq!(p.route(press(KEY)), Ok(Some(press(KEY))));
}

#[test]
fn duplicate_presses_and_releases_are_not_forwarded() {
    let mut p = ready();
    assert_eq!(p.route(release(KEY)), Ok(None));
    assert_eq!(p.route(press(KEY)), Ok(Some(press(KEY))));
    assert_eq!(p.route(press(KEY)), Ok(None));
    assert_eq!(p.route(release(KEY)), Ok(Some(release(KEY))));
    assert_eq!(p.route(release(KEY)), Ok(None));
    assert!(p.set_focus(false).is_empty());
}

#[test]
fn every_revocation_drains_keys_and_buttons_before_modifiers_once() {
    for trigger in 0..6 {
        let mut p = ready();
        let controls = [
            Control::Modifier(Modifier::Shift),
            Control::Modifier(Modifier::Ctrl),
            KEY,
            Control::MouseButton(1),
        ];
        for control in controls {
            p.route(press(control)).unwrap();
        }
        let releases = match trigger {
            0 => p.set_focus(false),
            1 => p.set_input_consent(false),
            2 => p.observe_session(&SessionEvent::ProcessExited(Some(0))),
            3 => p.observe_session(&SessionEvent::StateChanged(SessionState::Disconnected)),
            4 => p.select_mode(InputMode::Uhid).unwrap(),
            _ => p.set_capabilities(InputCapabilities::default()),
        };
        assert_eq!(releases.len(), 4);
        assert_eq!(releases[0], release(KEY));
        assert_eq!(releases[1], release(Control::MouseButton(1)));
        assert!(releases.iter().all(|e| !e.pressed));
        for control in controls {
            assert!(releases.contains(&release(control)));
        }
        assert!(p.set_focus(false).is_empty());
    }
}

#[test]
fn session_transitions_invalidate_grants_capabilities_and_pressed_state() {
    for state in [
        SessionState::Connecting,
        SessionState::Disconnected,
        SessionState::Terminated,
        SessionState::Active {
            serial: "another-private-device".into(),
            mode: DisplayMode::VirtualDisplay {
                width: 800,
                height: 600,
                dpi: 160,
            },
        },
    ] {
        let mut p = ready();
        p.set_clipboard_opt_in(true);
        p.route(press(KEY)).unwrap();
        assert_eq!(
            p.observe_session(&SessionEvent::StateChanged(state)),
            vec![release(KEY)]
        );
        active(&mut p);
        assert_eq!(p.route(press(KEY)), Err(InputDenied::NotFocused));
        p.set_focus(true);
        assert_eq!(p.route(press(KEY)), Err(InputDenied::NoConsent));
        p.set_input_consent(true);
        assert_eq!(
            p.select_mode(InputMode::Sdk),
            Err(InputDenied::UnsupportedMode)
        );
        p.set_capabilities(caps());
        p.select_mode(InputMode::Sdk).unwrap();
        assert_eq!(p.route(release(KEY)), Ok(None));
        assert_eq!(
            p.transfer_clipboard(
                ClipboardDirection::HostToAndroid,
                Protection::Allowed,
                || panic!("must not read")
            ),
            Err(ClipboardDenied::OptOut)
        );
        assert!(!format!("{p:?}").contains("private-device"));
    }
}

#[test]
fn modes_require_observation_and_never_fallback_silently() {
    assert_eq!(
        InputMode::Sdk.scrcpy_arguments(),
        ["--keyboard=sdk", "--mouse=sdk"]
    );
    assert_eq!(InputMode::Uhid.scrcpy_arguments(), ["-K", "-M"]);
    for support in [Support::Unknown, Support::Unsupported, Support::Blocked] {
        let mut p = ready();
        p.set_capabilities(InputCapabilities {
            sdk: support,
            uhid: support,
            ..caps()
        });
        assert_eq!(
            p.select_mode(InputMode::Sdk),
            Err(InputDenied::UnsupportedMode)
        );
        assert_eq!(
            p.select_mode(InputMode::Uhid),
            Err(InputDenied::UnsupportedMode)
        );
        assert_eq!(p.route(press(KEY)), Err(InputDenied::NoMode));
    }
    let mut p = ready();
    p.set_capabilities(InputCapabilities {
        sdk: Support::Blocked,
        ..caps()
    });
    assert_eq!(
        p.select_mode(InputMode::Sdk),
        Err(InputDenied::UnsupportedMode)
    );
    assert_eq!(p.route(press(KEY)), Err(InputDenied::NoMode));
    p.select_mode(InputMode::Uhid).unwrap();
    assert_eq!(p.route(press(KEY)), Ok(Some(press(KEY))));
}

#[tokio::test]
async fn real_spawn_failure_revokes_routing() {
    let controller = limivra_core::SessionController::new();
    let mut events = controller.subscribe();
    let mut p = ready();
    p.route(press(KEY)).unwrap();
    assert!(controller
        .start(
            "/dev/null/not-an-executable",
            &[],
            "private-device",
            DisplayMode::Mirror
        )
        .await
        .is_err());
    let mut releases = Vec::new();
    while let Ok(event) = events.try_recv() {
        releases.extend(p.observe_session(&event));
    }
    assert_eq!(releases, vec![release(KEY)]);
    assert_eq!(p.route(press(KEY)), Err(InputDenied::NotActive));
}

#[test]
fn errors_do_not_invent_disconnect_or_release_in_active_session() {
    let mut p = ready();
    p.route(press(KEY)).unwrap();
    assert!(p
        .observe_session(&SessionEvent::Error("private error".into()))
        .is_empty());
    assert_eq!(p.route(release(KEY)), Ok(Some(release(KEY))));
}

#[test]
fn clipboard_denials_never_invoke_the_read_callback() {
    use std::cell::Cell;
    for direction in [
        ClipboardDirection::HostToAndroid,
        ClipboardDirection::AndroidToHost,
    ] {
        for case in 0..8 {
            let mut p = ready();
            p.set_clipboard_opt_in(true);
            let (protection, expected) = match case {
                0 => {
                    p.observe_session(&SessionEvent::ProcessExited(None));
                    (Protection::Allowed, ClipboardDenied::NotActive)
                }
                1 => {
                    p.set_focus(false);
                    (Protection::Allowed, ClipboardDenied::NotFocused)
                }
                2 => {
                    p.set_clipboard_opt_in(false);
                    (Protection::Allowed, ClipboardDenied::OptOut)
                }
                3 => (Protection::Unknown, ClipboardDenied::Restricted),
                4 => (Protection::Restricted, ClipboardDenied::Restricted),
                n => {
                    let support = match n {
                        5 => Support::Unknown,
                        6 => Support::Blocked,
                        _ => Support::Unsupported,
                    };
                    let mut c = caps();
                    match direction {
                        ClipboardDirection::HostToAndroid => c.host_to_android = support,
                        ClipboardDirection::AndroidToHost => c.android_to_host = support,
                    }
                    p.set_capabilities(c);
                    (Protection::Allowed, ClipboardDenied::Unsupported)
                }
            };
            let reads = Cell::new(0);
            assert_eq!(
                p.transfer_clipboard(direction, protection, || reads.set(reads.get() + 1)),
                Err(expected)
            );
            assert_eq!(reads.get(), 0);
        }
    }
    let mut p = InputPolicy::default();
    p.set_clipboard_opt_in(true);
    p.set_capabilities(caps()); // No grants or observations accepted before session.
    active(&mut p);
    p.set_focus(true);
    assert_eq!(
        p.transfer_clipboard(
            ClipboardDirection::HostToAndroid,
            Protection::Allowed,
            || panic!("hidden read")
        ),
        Err(ClipboardDenied::OptOut)
    );
}

#[test]
fn clipboard_is_explicit_directional_and_has_no_payload_in_policy() {
    use std::cell::Cell;
    let mut p = ready();
    p.set_clipboard_opt_in(true);
    // Clipboard permission is independent of keyboard injection permission.
    p.set_input_consent(false);
    let reads = Cell::new(0);
    let transferred_bytes = Cell::new(0);
    for direction in [
        ClipboardDirection::HostToAndroid,
        ClipboardDirection::AndroidToHost,
    ] {
        assert_eq!(
            p.transfer_clipboard(direction, Protection::Allowed, || {
                let transient_text = "synthetic-clipboard-secret";
                transferred_bytes.set(transferred_bytes.get() + transient_text.len());
                reads.set(reads.get() + 1);
            }),
            Ok(())
        );
    }
    assert_eq!(reads.get(), 2);
    assert_eq!(
        transferred_bytes.get(),
        2 * "synthetic-clipboard-secret".len()
    );
    assert!(!format!("{p:?}").contains("synthetic-clipboard-secret"));
    p.set_focus(false);
    assert_eq!(
        p.transfer_clipboard(
            ClipboardDirection::HostToAndroid,
            Protection::Allowed,
            || reads.set(reads.get() + 1)
        ),
        Err(ClipboardDenied::NotFocused)
    );
    p.set_focus(true);
    p.set_clipboard_opt_in(false);
    assert_eq!(
        p.transfer_clipboard(
            ClipboardDirection::HostToAndroid,
            Protection::Allowed,
            || reads.set(reads.get() + 1)
        ),
        Err(ClipboardDenied::OptOut)
    );
    assert_eq!(reads.get(), 2);
}
