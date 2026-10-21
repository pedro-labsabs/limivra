use crate::DisplayMode;
use serde::{Deserialize, Serialize};

/// Classificação explicativa da causa de fallback de VirtualDisplay para Mirror.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FallbackReason {
    /// O binário scrcpy instalado no host não possui suporte à flag `--new-display`.
    HostScrcpyUnsupported { detail: String },
    /// A versão do Android ou política de segurança da fabricante (OEM) bloqueou a criação de tela secundária.
    RomPolicyDenied { detail: String },
    /// Falha ao instanciar o display virtual secundário em tempo de execução (crash ou encerramento precoce).
    VirtualDisplaySpawnFailed { detail: String },
    /// Falha ao lançar o aplicativo direcionado na tela secundária ou launcher indisponível.
    LaunchAppFailed { detail: String },
}

impl FallbackReason {
    /// Mensagem detalhada, amigável e transparente para exibição ao usuário final.
    /// "Nunca alternar invisivelmente nem declarar freeform."
    pub fn user_friendly_message(&self) -> String {
        match self {
            Self::HostScrcpyUnsupported { detail } => {
                format!(
                    "O scrcpy instalado no host não possui suporte a displays virtuais ({detail}). \
                    Alternando explicitamente para espelhamento (Mirror)."
                )
            }
            Self::RomPolicyDenied { detail } => {
                format!(
                    "A ROM ou opções de desenvolvedor do Android negaram a criação do display virtual: {detail}. \
                    Alternando explicitamente para espelhamento (Mirror)."
                )
            }
            Self::VirtualDisplaySpawnFailed { detail } => {
                format!(
                    "O processo de exibição virtual falhou ao iniciar no Android ({detail}). \
                    Alternando explicitamente para espelhamento (Mirror) para manter a sessão ativa."
                )
            }
            Self::LaunchAppFailed { detail } => {
                format!(
                    "O aplicativo solicitado não pôde ser iniciado na tela secundária ({detail}). \
                    Alternando explicitamente para espelhamento (Mirror)."
                )
            }
        }
    }
}

/// Registro estruturado de um evento de fallback ocorrido.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FallbackRecord {
    pub reason: FallbackReason,
    pub original_mode: DisplayMode,
    pub fallback_mode: DisplayMode,
    pub user_message: String,
}

impl FallbackRecord {
    /// Constrói um novo registro de fallback para o modo Mirror.
    pub fn new(reason: FallbackReason, original_mode: DisplayMode) -> Self {
        let user_message = reason.user_friendly_message();
        Self {
            reason,
            original_mode,
            fallback_mode: DisplayMode::Mirror,
            user_message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fallback_reason_user_friendly_messages() {
        let r1 = FallbackReason::HostScrcpyUnsupported {
            detail: "versão 1.25 sem --new-display".into(),
        };
        assert!(r1.user_friendly_message().contains("não possui suporte"));
        assert!(r1.user_friendly_message().contains("versão 1.25"));

        let r2 = FallbackReason::RomPolicyDenied {
            detail: "MIUI security permissions disabled".into(),
        };
        assert!(r2.user_friendly_message().contains("A ROM ou opções"));

        let r3 = FallbackReason::VirtualDisplaySpawnFailed {
            detail: "código de saída 1".into(),
        };
        assert!(r3.user_friendly_message().contains("falhou ao iniciar"));

        let r4 = FallbackReason::LaunchAppFailed {
            detail: "ActivityNotFound".into(),
        };
        assert!(r4.user_friendly_message().contains("não pôde ser iniciado"));
    }

    #[test]
    fn test_fallback_record_creation() {
        let original = DisplayMode::VirtualDisplay {
            width: 1920,
            height: 1080,
            dpi: 240,
        };
        let reason = FallbackReason::HostScrcpyUnsupported {
            detail: "scrcpy v1.25".into(),
        };
        let record = FallbackRecord::new(reason.clone(), original);

        assert_eq!(record.reason, reason);
        assert_eq!(record.original_mode, original);
        assert_eq!(record.fallback_mode, DisplayMode::Mirror);
        assert!(!record.user_message.is_empty());
    }
}
