//! Local host control intents; this module never captures keys or executes commands.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlAction {
    Disconnect,
    ReturnToSession,
    SwitchDisplay,
    ToggleVideoPause,
    ReloadVirtualDisplay,
}

/// A key-down event normalized by the host UI, not a global keyboard hook.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyStroke {
    pub key: char,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
    pub meta: bool,
    pub repeat: bool,
}

impl KeyStroke {
    pub fn action(self) -> Option<ControlAction> {
        if !self.ctrl || !self.shift || self.alt || self.meta || self.repeat {
            return None;
        }
        match self.key.to_ascii_lowercase() {
            'd' => Some(ControlAction::Disconnect),
            's' => Some(ControlAction::SwitchDisplay),
            'p' => Some(ControlAction::ToggleVideoPause),
            'r' => Some(ControlAction::ReloadVirtualDisplay),
            _ => None,
        }
    }
}

use crate::{DisplayMode, SessionEvent, SessionState};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionStatus {
    #[default]
    Disconnected,
    Connecting,
    Active,
    Terminated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlError {
    NotActive,
    RequiresVirtualDisplay,
}

/// Privacy-preserving projection for a single ordered session event stream.
/// Stores neither device identifiers nor raw error/fallback messages.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HostControls {
    connection: ConnectionStatus,
    mode: Option<DisplayMode>,
    video_paused: Option<bool>,
    has_error: bool,
    fallback_announced: bool,
}

impl HostControls {
    pub fn request_app(
        &self,
        app: crate::launcher::AllowedApp,
    ) -> Result<crate::launcher::AllowedApp, ControlError> {
        self.request(ControlAction::ReloadVirtualDisplay)?;
        Ok(app)
    }
    pub fn connection(&self) -> ConnectionStatus {
        self.connection
    }
    pub fn mode(&self) -> Option<DisplayMode> {
        self.mode
    }
    pub fn video_paused(&self) -> Option<bool> {
        self.video_paused
    }
    pub fn has_error(&self) -> bool {
        self.has_error
    }
    pub fn fallback_announced(&self) -> bool {
        self.fallback_announced
    }
    /// Apply events in order from one session subscription. After lag or reconnect,
    /// replace this projection and seed it with the current session state.
    /// Fallback notifications announce intent; only StateChanged confirms a mode.
    pub fn observe(&mut self, event: &SessionEvent) {
        match event {
            SessionEvent::StateChanged(state) => {
                let (connection, mode) = match state {
                    SessionState::Disconnected => (ConnectionStatus::Disconnected, None),
                    SessionState::Connecting => (ConnectionStatus::Connecting, None),
                    SessionState::Active { mode, .. } => (ConnectionStatus::Active, Some(*mode)),
                    SessionState::Terminated => (ConnectionStatus::Terminated, None),
                };
                self.connection = connection;
                self.mode = mode;
                self.video_paused = None;
                self.has_error = false;
                self.fallback_announced = false;
            }
            SessionEvent::ProcessExited(_) => {
                self.connection = ConnectionStatus::Terminated;
                self.mode = None;
                self.video_paused = None;
                self.fallback_announced = false;
            }
            SessionEvent::FallbackTriggered { .. } => self.fallback_announced = true,
            SessionEvent::Error(_) => self.has_error = true,
        }
    }
    /// Validate a toolbar action and return an intent for the adapter to execute.
    /// Availability is based on lifecycle/mode, not proof of backend capability.
    /// No command is executed and no confirmed state changes here.
    pub fn request(&self, action: ControlAction) -> Result<ControlAction, ControlError> {
        if self.connection != ConnectionStatus::Active {
            return Err(ControlError::NotActive);
        }
        if action == ControlAction::ReloadVirtualDisplay
            && !matches!(self.mode, Some(DisplayMode::VirtualDisplay { .. }))
        {
            return Err(ControlError::RequiresVirtualDisplay);
        }
        Ok(action)
    }
    pub fn shortcut(&self, key: KeyStroke) -> Option<Result<ControlAction, ControlError>> {
        key.action().map(|action| self.request(action))
    }
    /// Accept an authoritative backend observation for this active session.
    /// The adapter must discard stale replies from previous sessions/commands.
    /// None means video pause state has not been observed, not "playing".
    pub fn confirm_video_paused(&mut self, paused: bool) -> Result<(), ControlError> {
        if self.connection != ConnectionStatus::Active {
            return Err(ControlError::NotActive);
        }
        self.video_paused = Some(paused);
        Ok(())
    }
}
