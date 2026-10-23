//! Explicit app selection; never a device-wide app inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllowedApp {
    Settings,
    Calculator,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum LaunchError {
    #[error("Invalid launcher identifier")]
    InvalidIdentifier,
    #[error("App not permitted")]
    NotAllowed,
    #[error("Activity unavailable")]
    ActivityNotFound,
    #[error("Android denied access")]
    SecurityException,
    #[error("Display unavailable or denied")]
    DisplayDenied,
    #[error("Device unauthorized")]
    Unauthorized,
    #[error("Device offline")]
    Offline,
    #[error("Launcher timeout")]
    Timeout,
    #[error("Launcher cancelled")]
    Cancelled,
    #[error("Unexpected backend reply")]
    UnexpectedReply,
    #[error("Launcher command failed")]
    CommandFailed,
    #[error("Output limit exceeded")]
    OutputLimit,
    #[error("Unsafe launcher configuration")]
    UnsafeConfiguration,
    #[error("No active virtual session")]
    NotActive,
    #[error("Session context invalidated")]
    StaleContext,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageName(String);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentName(String);
impl AllowedApp {
    pub fn packages(self) -> &'static [&'static str] {
        match self {
            Self::Settings => &["com.android.settings"],
            Self::Calculator => &["com.miui.calculator", "com.google.android.calculator"],
        }
    }
    pub fn from_package(package: &str) -> Option<Self> {
        [Self::Settings, Self::Calculator]
            .into_iter()
            .find(|app| app.packages().contains(&package))
    }
}
impl PackageName {
    pub fn parse(value: &str) -> Result<Self, LaunchError> {
        if value.len() > 255 || value.split('.').count() < 2 || !identifier(value, false) {
            return Err(LaunchError::InvalidIdentifier);
        }
        Ok(Self(value.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
impl ComponentName {
    pub fn parse(value: &str) -> Result<Self, LaunchError> {
        let (package, activity) = value
            .split_once('/')
            .ok_or(LaunchError::InvalidIdentifier)?;
        PackageName::parse(package)?;
        if value.len() > 512
            || (!activity.starts_with('.') && activity.split('.').count() < 2)
            || !identifier(activity.strip_prefix('.').unwrap_or(activity), true)
        {
            return Err(LaunchError::InvalidIdentifier);
        }
        Ok(Self(value.into()))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn identifier(value: &str, inner: bool) -> bool {
    value.split('.').all(|segment| {
        let mut chars = segment.bytes();
        chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
            && chars.all(|c| c.is_ascii_alphanumeric() || c == b'_' || (inner && c == b'$'))
    })
}

pub mod adb_runner;
use std::{future::Future, pin::Pin};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedDisplay {
    pub id: u32,
    pub is_virtual: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LauncherRequest {
    Resolve {
        package: PackageName,
    },
    Start {
        component: ComponentName,
        display_id: u32,
    },
    Displays,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LauncherReply {
    Resolved(ComponentName),
    NotInstalled,
    Accepted,
    Displays(Vec<ObservedDisplay>),
}
pub trait LauncherRunner: Send + Sync {
    fn run<'a>(
        &'a self,
        target: &'a str,
        request: LauncherRequest,
    ) -> Pin<Box<dyn Future<Output = Result<LauncherReply, LaunchError>> + Send + 'a>>;
}
impl ComponentName {
    pub fn package(&self) -> &str {
        self.0.split_once('/').expect("validated").0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LaunchOutcome {
    Accepted { display_id: u32 },
}

// Constructed only by VirtualDisplayProvider after before/after observation.
#[derive(Clone)]
pub(crate) struct LaunchContext {
    pub(crate) generation: u64,
    pub(crate) target: String,
    pub(crate) display_id: u32,
}
