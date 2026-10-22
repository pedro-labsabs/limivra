//! Consent and focus policy. No platform input or clipboard APIs are called here.
use crate::{SessionEvent, SessionState};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Support {
    #[default]
    Unknown,
    Unsupported,
    Blocked,
    Supported,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Sdk,
    Uhid,
}
impl InputMode {
    pub fn scrcpy_arguments(self) -> [&'static str; 2] {
        match self {
            Self::Sdk => ["--keyboard=sdk", "--mouse=sdk"],
            Self::Uhid => ["-K", "-M"],
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InputCapabilities {
    pub sdk: Support,
    pub uhid: Support,
    pub host_to_android: Support,
    pub android_to_host: Support,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Modifier {
    Ctrl,
    Shift,
    Alt,
    Meta,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Control {
    Key(u16),
    MouseButton(u8),
    Modifier(Modifier),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputEvent {
    pub control: Control,
    pub pressed: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputDenied {
    NotActive,
    NotFocused,
    NoConsent,
    NoMode,
    UnsupportedMode,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardDirection {
    HostToAndroid,
    AndroidToHost,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Operation-specific system/secure-context observation. Unknown fails closed.
pub enum Protection {
    Unknown,
    Restricted,
    Allowed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardDenied {
    NotActive,
    NotFocused,
    OptOut,
    Unsupported,
    Restricted,
}
/// One local session/display policy. Every StateChanged starts a new trust boundary,
/// including repeated Active events. Reapply observations and grants explicitly.
/// Feed ordered events; after broadcast lag, reset the policy before routing.
/// Returned releases are intents: the adapter must deliver them or destroy its
/// owned input device on transport loss. This core cannot guarantee remote delivery.
#[derive(Debug, Default)]
pub struct InputPolicy {
    active: bool,
    focused: bool,
    consent: bool,
    mode: Option<InputMode>,
    capabilities: InputCapabilities,
    pressed: BTreeSet<Control>,
    clipboard_opt_in: bool,
}
impl InputPolicy {
    fn drain(&mut self) -> Vec<InputEvent> {
        // Enum ordering places ordinary keys/buttons before modifier releases.
        std::mem::take(&mut self.pressed)
            .into_iter()
            .map(|control| InputEvent {
                control,
                pressed: false,
            })
            .collect()
    }

    /// Invalidate session-local observations on every lifecycle transition.
    /// Error and fallback announcements alone do not confirm a transition.
    pub fn observe_session(&mut self, event: &SessionEvent) -> Vec<InputEvent> {
        let active = match event {
            SessionEvent::StateChanged(state) => matches!(state, SessionState::Active { .. }),
            SessionEvent::ProcessExited(_) => false,
            _ => return vec![],
        };
        let releases = self.drain();
        *self = Self {
            active,
            ..Self::default()
        };
        releases
    }
    pub fn set_focus(&mut self, focused: bool) -> Vec<InputEvent> {
        self.focused = self.active && focused;
        if !self.focused {
            self.drain()
        } else {
            vec![]
        }
    }
    pub fn set_input_consent(&mut self, consent: bool) -> Vec<InputEvent> {
        self.consent = self.active && consent;
        if !self.consent {
            self.drain()
        } else {
            vec![]
        }
    }
    /// Observations must apply to this session/device/display. Never infer from OEM.
    /// Any changed observation invalidates the mode; reselection is explicit.
    pub fn set_capabilities(&mut self, capabilities: InputCapabilities) -> Vec<InputEvent> {
        let capabilities = if self.active {
            capabilities
        } else {
            InputCapabilities::default()
        };
        if self.capabilities == capabilities {
            return vec![];
        }
        self.capabilities = capabilities;
        self.mode = None;
        self.drain()
    }
    pub fn select_mode(&mut self, mode: InputMode) -> Result<Vec<InputEvent>, InputDenied> {
        if !self.active {
            return Err(InputDenied::NotActive);
        }
        let support = match mode {
            InputMode::Sdk => self.capabilities.sdk,
            InputMode::Uhid => self.capabilities.uhid,
        };
        if support != Support::Supported {
            return Err(InputDenied::UnsupportedMode);
        }
        if self.mode == Some(mode) {
            return Ok(vec![]);
        }
        let releases = self.drain();
        self.mode = Some(mode);
        Ok(releases)
    }
    /// Caller must route local host shortcuts separately before forwarding.
    /// No text, history, repeats or platform event capture is retained here.
    pub fn route(&mut self, event: InputEvent) -> Result<Option<InputEvent>, InputDenied> {
        if !self.active {
            return Err(InputDenied::NotActive);
        }
        if !self.focused {
            return Err(InputDenied::NotFocused);
        }
        if !self.consent {
            return Err(InputDenied::NoConsent);
        }
        if self.mode.is_none() {
            return Err(InputDenied::NoMode);
        }
        let changed = if event.pressed {
            self.pressed.insert(event.control)
        } else {
            self.pressed.remove(&event.control)
        };
        Ok(changed.then_some(event))
    }
    pub fn set_clipboard_opt_in(&mut self, enabled: bool) {
        self.clipboard_opt_in = self.active && enabled;
    }
    /// Call only for a fresh user-initiated transfer. The adapter must supply a
    /// current protection observation (including FLAG_SECURE/system restrictions).
    /// Unknown is denied. Reading must occur INSIDE this synchronous callback,
    /// never before the gate; do not enqueue asynchronous reads from it.
    ///
    /// The policy receives no text and retains no callback. Ok means the callback
    /// ran, not that platform delivery succeeded. The adapter owns transport errors
    /// and any transient buffers; no guarantee is made about its memory or the OS
    /// clipboard. No reusable permission escapes this call.
    pub fn transfer_clipboard(
        &self,
        direction: ClipboardDirection,
        protection: Protection,
        callback: impl FnOnce(),
    ) -> Result<(), ClipboardDenied> {
        if !self.active {
            return Err(ClipboardDenied::NotActive);
        }
        if !self.focused {
            return Err(ClipboardDenied::NotFocused);
        }
        if !self.clipboard_opt_in {
            return Err(ClipboardDenied::OptOut);
        }
        if protection != Protection::Allowed {
            return Err(ClipboardDenied::Restricted);
        }
        let support = match direction {
            ClipboardDirection::HostToAndroid => self.capabilities.host_to_android,
            ClipboardDirection::AndroidToHost => self.capabilities.android_to_host,
        };
        if support != Support::Supported {
            return Err(ClipboardDenied::Unsupported);
        }
        callback();
        Ok(())
    }
}
