//! Limivra Core - Núcleo de controle de sessão e gerenciamento de display Android.

pub mod adb;
pub mod display;
pub mod provider;
pub mod scrcpy;
pub mod session;

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub use adb::{AdbDevice, DeviceStatus};
pub use display::{
    AndroidDisplayInfo, DeviceProfile, DisplayProbeReport, DisplayProber, VirtualDisplaySupport,
};
pub use provider::VirtualDisplayProvider;
pub use scrcpy::{
    AppLaunchBackend, CapabilityProbeSource, ScrcpyAdapter, ScrcpyCapabilities,
    ScrcpySessionOptions, ScrcpyVersion, VirtualDisplayConfig,
};
pub use session::{SessionController, SessionEvent};

/// Erros fundamentais do núcleo Limivra.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Ferramenta de sistema ausente ou incompatível: {0}")]
    DependencyMissing(String),

    #[error("Falha ao comunicar com daemon ADB: {0}")]
    AdbError(String),

    #[error("Nenhum dispositivo Android autorizado encontrado")]
    NoDeviceFound,

    #[error("Múltiplos dispositivos encontrados, selecione um explicitamente via serial")]
    AmbiguousDeviceSelection,

    #[error("Subprocesso scrcpy encerrou com erro: {0}")]
    ScrcpyError(String),

    #[error("Erro de I/O: {0}")]
    Io(#[from] std::io::Error),
}

/// Modos de operação do display gráfico Limivra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisplayMode {
    /// Espelhamento direto do display primário do Android.
    Mirror,
    /// Criação de um VirtualDisplay independente (secundário).
    VirtualDisplay { width: u32, height: u32, dpi: u32 },
}

/// Estado da sessão de conexão.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionState {
    Disconnected,
    Connecting,
    Active { serial: String, mode: DisplayMode },
    Terminated,
}

/// Serviços de descoberta de dispositivos.
pub struct DeviceDiscovery;

impl DeviceDiscovery {
    /// Executa `adb devices -l` de forma assíncrona e retorna os dispositivos detectados.
    pub async fn list_devices() -> Result<Vec<AdbDevice>, CoreError> {
        let output = tokio::process::Command::new("adb")
            .arg("devices")
            .arg("-l")
            .output()
            .await
            .map_err(|e| CoreError::AdbError(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CoreError::AdbError(stderr.to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(adb::parse_adb_devices_output(&stdout))
    }

    /// Seleciona o dispositivo de forma determinística e segura:
    /// - Se um serial foi fornecido, busca exatamente esse serial.
    /// - Se nenhum serial foi fornecido, exige que haja exatamente um dispositivo autorizado.
    pub async fn resolve_target(requested_serial: Option<&str>) -> Result<AdbDevice, CoreError> {
        let devices = Self::list_devices().await?;

        if let Some(serial) = requested_serial {
            devices
                .into_iter()
                .find(|d| d.serial == serial)
                .ok_or(CoreError::NoDeviceFound)
        } else {
            let ready_devices: Vec<AdbDevice> =
                devices.into_iter().filter(|d| d.is_ready()).collect();
            match ready_devices.len() {
                0 => Err(CoreError::NoDeviceFound),
                1 => Ok(ready_devices.into_iter().next().unwrap()),
                _ => Err(CoreError::AmbiguousDeviceSelection),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_display_mode_serialization() {
        let mode = DisplayMode::VirtualDisplay {
            width: 1920,
            height: 1080,
            dpi: 240,
        };
        let serialized = serde_json::to_string(&mode).expect("deve serializar");
        let deserialized: DisplayMode =
            serde_json::from_str(&serialized).expect("deve deserializar");
        assert_eq!(mode, deserialized);
    }
}
