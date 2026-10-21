use limivra_core::host_controls::{ControlAction, KeyStroke};

fn shortcut(key: char) -> KeyStroke {
    KeyStroke {
        key,
        ctrl: true,
        shift: true,
        alt: false,
        meta: false,
        repeat: false,
    }
}

#[test]
fn maps_all_local_shortcuts() {
    for (key, action) in [
        ('d', ControlAction::Disconnect),
        ('s', ControlAction::SwitchDisplay),
        ('p', ControlAction::ToggleVideoPause),
        ('r', ControlAction::ReloadVirtualDisplay),
    ] {
        assert_eq!(shortcut(key).action(), Some(action));
        assert_eq!(shortcut(key.to_ascii_uppercase()).action(), Some(action));
    }
}

#[test]
fn ignores_unmapped_repeated_or_nonexact_chords() {
    assert_eq!(shortcut('x').action(), None);
    for event in [
        KeyStroke {
            repeat: true,
            ..shortcut('d')
        },
        KeyStroke {
            ctrl: false,
            ..shortcut('d')
        },
        KeyStroke {
            shift: false,
            ..shortcut('d')
        },
        KeyStroke {
            alt: true,
            ..shortcut('d')
        },
        KeyStroke {
            meta: true,
            ..shortcut('d')
        },
    ] {
        assert_eq!(event.action(), None);
    }
}

use limivra_core::host_controls::{ConnectionStatus, ControlError, HostControls};
use limivra_core::{DisplayMode, FallbackReason, SessionEvent, SessionState};

const VIRTUAL: DisplayMode = DisplayMode::VirtualDisplay {
    width: 1920,
    height: 1080,
    dpi: 240,
};

fn activate(controls: &mut HostControls, mode: DisplayMode) {
    controls.observe(&SessionEvent::StateChanged(SessionState::Active {
        serial: "private-device-identifier".into(),
        mode,
    }));
}

#[test]
fn toolbar_and_keyboard_return_same_intents_without_claiming_effects() {
    let mut controls = HostControls::default();
    activate(&mut controls, VIRTUAL);
    for key in ['d', 's', 'p', 'r'] {
        let before = controls.clone();
        let action = shortcut(key).action().unwrap();
        assert_eq!(controls.request(action), Ok(action));
        assert_eq!(controls.shortcut(shortcut(key)), Some(Ok(action)));
        assert_eq!(controls, before);
    }
    assert_eq!(controls.shortcut(shortcut('x')), None);
    assert!(!format!("{controls:?}").contains("private-device-identifier"));
}

#[test]
fn blocks_controls_outside_active_session_and_reload_in_mirror() {
    let mut controls = HostControls::default();
    for state in [
        SessionState::Disconnected,
        SessionState::Connecting,
        SessionState::Terminated,
    ] {
        controls.observe(&SessionEvent::StateChanged(state));
        for key in ['d', 's', 'p', 'r'] {
            assert_eq!(
                controls.shortcut(shortcut(key)),
                Some(Err(ControlError::NotActive))
            );
        }
    }
    activate(&mut controls, DisplayMode::Mirror);
    assert_eq!(
        controls.request(ControlAction::ReloadVirtualDisplay),
        Err(ControlError::RequiresVirtualDisplay)
    );
    assert_eq!(
        controls.request(ControlAction::SwitchDisplay),
        Ok(ControlAction::SwitchDisplay)
    );
}

#[test]
fn projects_connection_and_resets_video_confirmation_between_sessions() {
    let mut controls = HostControls::default();
    assert_eq!(controls.connection(), ConnectionStatus::Disconnected);
    controls.observe(&SessionEvent::StateChanged(SessionState::Connecting));
    assert_eq!(controls.connection(), ConnectionStatus::Connecting);
    activate(&mut controls, VIRTUAL);
    assert_eq!(controls.connection(), ConnectionStatus::Active);
    assert_eq!(controls.mode(), Some(VIRTUAL));
    controls.confirm_video_paused(true).unwrap();
    assert_eq!(controls.video_paused(), Some(true));
    controls.request(ControlAction::ToggleVideoPause).unwrap();
    assert_eq!(controls.video_paused(), Some(true));
    controls.observe(&SessionEvent::ProcessExited(Some(1)));
    assert_eq!(controls.connection(), ConnectionStatus::Terminated);
    assert_eq!(controls.mode(), None);
    assert_eq!(controls.video_paused(), None);
    assert_eq!(
        controls.confirm_video_paused(false),
        Err(ControlError::NotActive)
    );
    activate(&mut controls, DisplayMode::Mirror);
    assert_eq!(controls.video_paused(), None);
    controls.observe(&SessionEvent::StateChanged(SessionState::Disconnected));
    assert_eq!(controls.connection(), ConnectionStatus::Disconnected);
    assert_eq!(controls.mode(), None);
}

#[test]
fn fallback_notification_waits_for_confirmed_state_and_errors_are_redacted() {
    let mut controls = HostControls::default();
    activate(&mut controls, VIRTUAL);
    controls.confirm_video_paused(true).unwrap();
    controls.observe(&SessionEvent::FallbackTriggered {
        reason: FallbackReason::ExplicitRequest {
            detail: "private reason".into(),
        },
        target_mode: DisplayMode::Mirror,
    });
    assert!(controls.fallback_announced());
    assert_eq!(controls.mode(), Some(VIRTUAL));
    controls.observe(&SessionEvent::Error(
        "private-device-identifier private log".into(),
    ));
    assert!(controls.has_error());
    assert!(!format!("{controls:?}").contains("private"));
    assert_eq!(controls.mode(), Some(VIRTUAL));
    assert_eq!(controls.video_paused(), Some(true));
    activate(&mut controls, DisplayMode::Mirror);
    assert!(!controls.has_error());
    assert!(!controls.fallback_announced());
    assert_eq!(controls.mode(), Some(DisplayMode::Mirror));
    assert_eq!(controls.video_paused(), None);
    assert_eq!(
        controls.request(ControlAction::ReloadVirtualDisplay),
        Err(ControlError::RequiresVirtualDisplay)
    );
}
