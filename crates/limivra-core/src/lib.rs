//! Limivra Core - Núcleo de controle de sessão e gerenciamento de display Android.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Erros fundamentais do núcleo Limivra.
#[derive(Debug, Error)]
pub enum CoreError {
    #[error("Ferramenta de sistema ausente ou incompatível: {0}")]
    DependencyMissing(String),

    #[error("Falha ao comunicar com daemon ADB: {0}")]
    AdbError(String),

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
