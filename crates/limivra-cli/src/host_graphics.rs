use std::{env, ffi::OsStr};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HostGraphicsEnvironment {
    Wayland,
    X11,
    Headless,
}

impl HostGraphicsEnvironment {
    pub(crate) fn detect() -> Self {
        let wayland_display = env::var_os("WAYLAND_DISPLAY");
        let display = env::var_os("DISPLAY");
        Self::from_values(wayland_display.as_deref(), display.as_deref())
    }

    fn from_values(wayland_display: Option<&OsStr>, display: Option<&OsStr>) -> Self {
        if wayland_display.is_some_and(|value| !value.is_empty()) {
            Self::Wayland
        } else if display.is_some_and(|value| !value.is_empty()) {
            Self::X11
        } else {
            Self::Headless
        }
    }

    pub(crate) fn diagnostic(self) -> &'static str {
        match self {
            Self::Wayland => {
                "DETECTED: Wayland environment indicated; server connection not verified."
            }
            Self::X11 => "DETECTED: X11 environment indicated; server connection not verified.",
            Self::Headless => {
                "SKIP: host headless; WAYLAND_DISPLAY and DISPLAY are unset or empty."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_graphics_environment_values_are_headless() {
        assert_eq!(
            HostGraphicsEnvironment::from_values(Some(OsStr::new("")), Some(OsStr::new(""))),
            HostGraphicsEnvironment::Headless
        );
    }
}
