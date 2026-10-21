use crate::{CoreError, DisplayMode};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Versão analisada do scrcpy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScrcpyVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: Option<u32>,
    pub raw: String,
}

impl ScrcpyVersion {
    /// Faz o parse da saída de `scrcpy --version`.
    pub fn parse(output: &str) -> Option<Self> {
        let first_line = output.lines().next()?;
        let parts: Vec<&str> = first_line.split_whitespace().collect();
        if parts.len() < 2 || !parts[0].eq_ignore_ascii_case("scrcpy") {
            return None;
        }

        let ver_str = parts[1];
        let mut num_parts = ver_str.split('.');
        let major = num_parts.next()?.parse().ok()?;
        let minor = num_parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let patch = num_parts.next().and_then(|s| s.parse().ok());

        Some(Self {
            major,
            minor,
            patch,
            raw: ver_str.to_string(),
        })
    }

    /// Verifica heurística básica de versão para suporte a display virtual (>= 2.2 / 3.0).
    pub fn supports_virtual_display(&self) -> bool {
        self.major > 2 || (self.major == 2 && self.minor >= 2)
    }
}

/// Origem da determinação de capacidades do scrcpy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityProbeSource {
    /// Inspecionado diretamente pela saída de `scrcpy --help`.
    HelpInspection,
    /// Inferido a partir da versão sem inspeção direta de flags.
    VersionHeuristic,
    /// Desconhecido ou incompatível; nenhum recurso avançado é presumido.
    Unknown,
}

/// Capacidades e flags suportadas pelo binário scrcpy instalado.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScrcpyCapabilities {
    pub version: Option<ScrcpyVersion>,
    /// Suporte verificado a criação de display virtual secundário (`--new-display`).
    pub supports_new_display: bool,
    /// Suporte a lançamento direcionado de aplicativo (`--start-app`).
    pub supports_start_app: bool,
    /// Suporte a desabilitar decorações de sistema no display virtual (`--no-vd-system-decorations`).
    pub supports_vd_system_decorations: bool,
    /// Suporte a manter conteúdo ao destruir o display virtual (`--no-vd-destroy-content`).
    pub supports_vd_destroy_content: bool,
    /// Suporte a política de posicionamento de IME (`--display-ime-policy`).
    pub supports_display_ime_policy: bool,
    /// Suporte a listar displays do dispositivo (`--list-displays`).
    pub supports_list_displays: bool,
    /// Suporte a manter o dispositivo acordado (`--stay-awake`).
    pub supports_stay_awake: bool,
    /// Suporte a redimensionamento flexível de display virtual (`--flex-display` ou `-x`).
    pub supports_flex_display: bool,
    /// Modo de prova da capacidade.
    pub probe_source: CapabilityProbeSource,
}

impl ScrcpyCapabilities {
    /// Constrói um descritor seguro de capacidades desconhecidas (tudo desabilitado).
    /// "UNKNOWN não vira suportado sem prova."
    pub fn unknown() -> Self {
        Self {
            version: None,
            supports_new_display: false,
            supports_start_app: false,
            supports_vd_system_decorations: false,
            supports_vd_destroy_content: false,
            supports_display_ime_policy: false,
            supports_list_displays: false,
            supports_stay_awake: false,
            supports_flex_display: false,
            probe_source: CapabilityProbeSource::Unknown,
        }
    }

    /// Inspeciona diretamente a saída de `scrcpy --help` para comprovar cada flag suportada.
    pub fn from_help_output(help_output: &str, version: Option<ScrcpyVersion>) -> Self {
        let supports_new_display = help_output.contains("--new-display");
        let supports_start_app = help_output.contains("--start-app");
        let supports_vd_system_decorations = help_output.contains("--no-vd-system-decorations");
        let supports_vd_destroy_content = help_output.contains("--no-vd-destroy-content");
        let supports_display_ime_policy = help_output.contains("--display-ime-policy");
        let supports_list_displays = help_output.contains("--list-displays");
        let supports_stay_awake = help_output.contains("--stay-awake");
        let supports_flex_display = help_output.contains("--flex-display");

        Self {
            version,
            supports_new_display,
            supports_start_app,
            supports_vd_system_decorations,
            supports_vd_destroy_content,
            supports_display_ime_policy,
            supports_list_displays,
            supports_stay_awake,
            supports_flex_display,
            probe_source: CapabilityProbeSource::HelpInspection,
        }
    }

    /// Infere capacidades baseadas no número da versão caso `--help` esteja indisponível.
    pub fn from_version_heuristic(version: ScrcpyVersion) -> Self {
        let supports_new_display = version.supports_virtual_display();
        let supports_start_app = version.major >= 2;
        let supports_vd_system_decorations = version.major >= 3;
        let supports_vd_destroy_content = version.major >= 3;
        let supports_display_ime_policy = version.major >= 3;
        let supports_list_displays =
            version.major >= 2 || (version.major == 1 && version.minor >= 25);
        let supports_stay_awake = version.major > 1 || (version.major == 1 && version.minor >= 13);
        let supports_flex_display = version.major >= 3;

        Self {
            supports_new_display,
            supports_start_app,
            supports_vd_system_decorations,
            supports_vd_destroy_content,
            supports_display_ime_policy,
            supports_list_displays,
            supports_stay_awake,
            supports_flex_display,
            probe_source: CapabilityProbeSource::VersionHeuristic,
            version: Some(version),
        }
    }
}

/// Configurações avançadas opcionais para sessões com display virtual.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct VirtualDisplayConfig {
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
    #[serde(default)]
    pub start_app: Option<String>,
    #[serde(default)]
    pub disable_system_decorations: bool,
    #[serde(default)]
    pub keep_content_on_destroy: bool,
    #[serde(default)]
    pub ime_policy: Option<String>,
    #[serde(default)]
    pub flex_display: bool,
}

/// Opções globais para execução da sessão scrcpy.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScrcpySessionOptions {
    pub stay_awake: bool,
    pub virtual_config: Option<VirtualDisplayConfig>,
}

/// Adaptador para orquestração, inspeção de flags e construção de argumentos do scrcpy.
pub struct ScrcpyAdapter;

impl ScrcpyAdapter {
    /// Detecta a versão do scrcpy instalado executando `scrcpy --version`.
    pub async fn detect_version() -> Result<ScrcpyVersion, CoreError> {
        let output = tokio::process::Command::new("scrcpy")
            .arg("--version")
            .output()
            .await
            .map_err(|e| CoreError::DependencyMissing(format!("scrcpy não encontrado: {e}")))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        ScrcpyVersion::parse(&stdout).ok_or_else(|| {
            CoreError::DependencyMissing("Saída de versão do scrcpy irreconhecível".into())
        })
    }

    /// Detecta rigorosamente as capacidades do scrcpy inspecionando `--version` e `--help`.
    pub async fn detect_capabilities() -> Result<ScrcpyCapabilities, CoreError> {
        let version = Self::detect_version().await.ok();

        // Tenta inspecionar o help real
        let help_res = tokio::process::Command::new("scrcpy")
            .arg("--help")
            .output()
            .await;

        match help_res {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                Ok(ScrcpyCapabilities::from_help_output(&stdout, version))
            }
            _ => {
                if let Some(ver) = version {
                    warn!(
                        "Não foi possível executar 'scrcpy --help'; utilizando heurística da versão {}",
                        ver.raw
                    );
                    Ok(ScrcpyCapabilities::from_version_heuristic(ver))
                } else {
                    Err(CoreError::DependencyMissing(
                        "Binário scrcpy ausente ou não responde a --version / --help".into(),
                    ))
                }
            }
        }
    }

    /// Constrói a lista segura de argumentos para invocar o scrcpy baseando-se em `ScrcpyVersion`.
    /// Mantido para compatibilidade direta.
    pub fn build_arguments(
        serial: &str,
        mode: DisplayMode,
        version: &ScrcpyVersion,
        stay_awake: bool,
    ) -> Result<Vec<String>, CoreError> {
        let caps = ScrcpyCapabilities::from_version_heuristic(version.clone());
        Self::build_arguments_with_capabilities(serial, mode, &caps, stay_awake)
    }

    /// Constrói os argumentos do scrcpy utilizando a prova estrita de capacidades (`ScrcpyCapabilities`).
    pub fn build_arguments_with_capabilities(
        serial: &str,
        mode: DisplayMode,
        caps: &ScrcpyCapabilities,
        stay_awake: bool,
    ) -> Result<Vec<String>, CoreError> {
        let options = ScrcpySessionOptions {
            stay_awake,
            virtual_config: match mode {
                DisplayMode::VirtualDisplay { width, height, dpi } => Some(VirtualDisplayConfig {
                    width,
                    height,
                    dpi,
                    ..Default::default()
                }),
                DisplayMode::Mirror => None,
            },
        };
        Self::build_arguments_advanced(serial, mode, caps, &options)
    }

    /// Constrói argumentos completos suportando todas as opções de display virtual e fallbacks controlados.
    pub fn build_arguments_advanced(
        serial: &str,
        mode: DisplayMode,
        caps: &ScrcpyCapabilities,
        options: &ScrcpySessionOptions,
    ) -> Result<Vec<String>, CoreError> {
        let mut args = vec![
            "-s".to_string(),
            serial.to_string(),
            "--window-title".to_string(),
            format!("Limivra Desktop ({})", serial),
        ];

        if options.stay_awake {
            if caps.supports_stay_awake {
                args.push("--stay-awake".to_string());
            } else {
                warn!("scrcpy não suporta --stay-awake; ignorando flag com segurança.");
            }
        }

        match mode {
            DisplayMode::Mirror => {
                info!("Configurando scrcpy em modo espelhamento (Mirror)");
            }
            DisplayMode::VirtualDisplay { width, height, dpi } => {
                if !caps.supports_new_display {
                    let source_desc = match caps.probe_source {
                        CapabilityProbeSource::HelpInspection => "prova direta de flags no --help",
                        CapabilityProbeSource::VersionHeuristic => "heurística de versão",
                        CapabilityProbeSource::Unknown => "capacidade desconhecida",
                    };
                    warn!(
                        "scrcpy instalado não suporta criação de display virtual secundário ({source_desc}). Realizando fallback gracioso para Mirror."
                    );
                    return Ok(args);
                }

                info!(
                    "Configurando display virtual secundário: {}x{} @ {} DPI",
                    width, height, dpi
                );
                args.push(format!("--new-display={width}x{height}/{dpi}"));

                if let Some(cfg) = &options.virtual_config {
                    if let Some(app) = &cfg.start_app {
                        if caps.supports_start_app {
                            args.push(format!("--start-app={app}"));
                        } else {
                            warn!("scrcpy não suporta flag --start-app; ignorando inicialização explícita.");
                        }
                    }

                    if cfg.disable_system_decorations {
                        if caps.supports_vd_system_decorations {
                            args.push("--no-vd-system-decorations".to_string());
                        } else {
                            warn!("scrcpy não suporta --no-vd-system-decorations; ignorando flag.");
                        }
                    }

                    if cfg.keep_content_on_destroy {
                        if caps.supports_vd_destroy_content {
                            args.push("--no-vd-destroy-content".to_string());
                        } else {
                            warn!("scrcpy não suporta --no-vd-destroy-content; ignorando flag.");
                        }
                    }

                    if let Some(policy) = &cfg.ime_policy {
                        if caps.supports_display_ime_policy {
                            args.push(format!("--display-ime-policy={policy}"));
                        } else {
                            warn!("scrcpy não suporta --display-ime-policy; ignorando política.");
                        }
                    }

                    if cfg.flex_display {
                        if caps.supports_flex_display {
                            args.push("--flex-display".to_string());
                        } else {
                            warn!("scrcpy não suporta --flex-display; ignorando flag.");
                        }
                    }
                }
            }
        }

        Ok(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_scrcpy_version() {
        let output = "scrcpy 1.25 <https://github.com/Genymobile/scrcpy>\nDependencies...";
        let ver = ScrcpyVersion::parse(output).expect("deve parsear");
        assert_eq!(ver.major, 1);
        assert_eq!(ver.minor, 25);
        assert_eq!(ver.patch, None);
        assert!(!ver.supports_virtual_display());

        let output_v2 = "scrcpy 2.2 <https://github.com/Genymobile/scrcpy>";
        let ver2 = ScrcpyVersion::parse(output_v2).expect("deve parsear v2.2");
        assert_eq!(ver2.major, 2);
        assert_eq!(ver2.minor, 2);
        assert_eq!(ver2.patch, None);
        assert!(ver2.supports_virtual_display());

        let output_v3 = "scrcpy 3.0";
        let ver3 = ScrcpyVersion::parse(output_v3).expect("deve parsear v3");
        assert_eq!(ver3.major, 3);
        assert_eq!(ver3.minor, 0);
        assert!(ver3.supports_virtual_display());

        let output_v5 = "scrcpy 5.0.1 <https://github.com/Genymobile/scrcpy>";
        let ver5 = ScrcpyVersion::parse(output_v5).expect("deve parsear v5.0.1");
        assert_eq!(ver5.major, 5);
        assert_eq!(ver5.minor, 0);
        assert_eq!(ver5.patch, Some(1));
        assert!(ver5.supports_virtual_display());

        let invalid = "invalid output from unknown binary";
        assert!(ScrcpyVersion::parse(invalid).is_none());
    }

    #[test]
    fn test_capabilities_from_help_output() {
        let help = r#"
Usage: scrcpy [options]
    --display-id=id
        Specify device display id to mirror.
    --display-ime-policy=value
        Set IME policy (local/fallback).
    --flex-display
        Continuously resize virtual display.
    --list-displays
        List device displays.
    --new-display[=[<width>x<height>][/<dpi>]]
        Create a new virtual display.
    --no-vd-destroy-content
        Do not destroy content on removal.
    --no-vd-system-decorations
        Disable virtual display system decorations.
    --start-app=name
        Start specified application.
    --stay-awake
        Keep device awake.
"#;
        let caps = ScrcpyCapabilities::from_help_output(help, None);
        assert_eq!(caps.probe_source, CapabilityProbeSource::HelpInspection);
        assert!(caps.supports_new_display);
        assert!(caps.supports_start_app);
        assert!(caps.supports_vd_system_decorations);
        assert!(caps.supports_vd_destroy_content);
        assert!(caps.supports_display_ime_policy);
        assert!(caps.supports_list_displays);
        assert!(caps.supports_stay_awake);
        assert!(caps.supports_flex_display);
    }

    #[test]
    fn test_capabilities_from_legacy_help_output() {
        let legacy_help = r#"
Usage: scrcpy [options]
    --stay-awake
        Keep device awake.
    --list-displays
        List displays.
"#;
        let caps = ScrcpyCapabilities::from_help_output(legacy_help, None);
        assert_eq!(caps.probe_source, CapabilityProbeSource::HelpInspection);
        assert!(!caps.supports_new_display);
        assert!(!caps.supports_start_app);
        assert!(!caps.supports_vd_system_decorations);
        assert!(!caps.supports_vd_destroy_content);
        assert!(!caps.supports_display_ime_policy);
        assert!(caps.supports_list_displays);
        assert!(caps.supports_stay_awake);
        assert!(!caps.supports_flex_display);
    }

    #[test]
    fn test_capabilities_unknown_strict_no_assumptions() {
        let caps = ScrcpyCapabilities::unknown();
        assert_eq!(caps.probe_source, CapabilityProbeSource::Unknown);
        assert!(!caps.supports_new_display);
        assert!(!caps.supports_start_app);
        assert!(!caps.supports_vd_system_decorations);
        assert!(!caps.supports_vd_destroy_content);
        assert!(!caps.supports_display_ime_policy);
        assert!(!caps.supports_list_displays);
        assert!(!caps.supports_stay_awake);
        assert!(!caps.supports_flex_display);
    }

    #[test]
    fn test_capabilities_version_heuristic_variants() {
        let v1_25 = ScrcpyVersion {
            major: 1,
            minor: 25,
            patch: None,
            raw: "1.25".into(),
        };
        let caps1 = ScrcpyCapabilities::from_version_heuristic(v1_25);
        assert!(!caps1.supports_new_display);
        assert!(!caps1.supports_start_app);
        assert!(caps1.supports_list_displays);
        assert!(caps1.supports_stay_awake);

        let v2_0 = ScrcpyVersion {
            major: 2,
            minor: 0,
            patch: None,
            raw: "2.0".into(),
        };
        let caps2 = ScrcpyCapabilities::from_version_heuristic(v2_0);
        assert!(!caps2.supports_new_display);
        assert!(caps2.supports_start_app);
        assert!(caps2.supports_list_displays);
        assert!(!caps2.supports_vd_system_decorations);

        let v2_2 = ScrcpyVersion {
            major: 2,
            minor: 2,
            patch: None,
            raw: "2.2".into(),
        };
        let caps22 = ScrcpyCapabilities::from_version_heuristic(v2_2);
        assert!(caps22.supports_new_display);
        assert!(caps22.supports_start_app);

        let v3_0 = ScrcpyVersion {
            major: 3,
            minor: 0,
            patch: None,
            raw: "3.0".into(),
        };
        let caps3 = ScrcpyCapabilities::from_version_heuristic(v3_0);
        assert!(caps3.supports_new_display);
        assert!(caps3.supports_vd_system_decorations);
        assert!(caps3.supports_vd_destroy_content);
        assert!(caps3.supports_display_ime_policy);
    }

    #[test]
    fn test_build_advanced_arguments_with_full_caps() {
        let help = r#"
    --new-display[=[<width>x<height>][/<dpi>]]
    --start-app=name
    --no-vd-system-decorations
    --no-vd-destroy-content
    --display-ime-policy=value
    --flex-display
    --stay-awake
"#;
        let caps = ScrcpyCapabilities::from_help_output(help, None);
        let mode = DisplayMode::VirtualDisplay {
            width: 1920,
            height: 1080,
            dpi: 240,
        };
        let options = ScrcpySessionOptions {
            stay_awake: true,
            virtual_config: Some(VirtualDisplayConfig {
                width: 1920,
                height: 1080,
                dpi: 240,
                start_app: Some("com.android.settings".to_string()),
                disable_system_decorations: true,
                keep_content_on_destroy: true,
                ime_policy: Some("local".to_string()),
                flex_display: true,
            }),
        };

        let args = ScrcpyAdapter::build_arguments_advanced("serial456", mode, &caps, &options)
            .expect("deve construir args completos");

        assert_eq!(
            args,
            vec![
                "-s",
                "serial456",
                "--window-title",
                "Limivra Desktop (serial456)",
                "--stay-awake",
                "--new-display=1920x1080/240",
                "--start-app=com.android.settings",
                "--no-vd-system-decorations",
                "--no-vd-destroy-content",
                "--display-ime-policy=local",
                "--flex-display"
            ]
        );
    }

    #[test]
    fn test_build_advanced_arguments_gracefully_drops_unsupported_flags() {
        // Suporta apenas --new-display, sem start-app ou IME policy
        let help = r#"
    --new-display[=[<width>x<height>][/<dpi>]]
"#;
        let caps = ScrcpyCapabilities::from_help_output(help, None);
        let mode = DisplayMode::VirtualDisplay {
            width: 1280,
            height: 720,
            dpi: 160,
        };
        let options = ScrcpySessionOptions {
            stay_awake: true, // Caps não tem stay-awake
            virtual_config: Some(VirtualDisplayConfig {
                width: 1280,
                height: 720,
                dpi: 160,
                start_app: Some("com.android.settings".to_string()),
                disable_system_decorations: true,
                keep_content_on_destroy: true,
                ime_policy: Some("fallback".to_string()),
                flex_display: true,
            }),
        };

        let args = ScrcpyAdapter::build_arguments_advanced("serial789", mode, &caps, &options)
            .expect("deve construir args filtrando flags desconhecidas");

        assert_eq!(
            args,
            vec![
                "-s",
                "serial789",
                "--window-title",
                "Limivra Desktop (serial789)",
                "--new-display=1280x720/160"
            ]
        );
    }

    #[test]
    fn test_fallback_on_unsupported_virtual_display() {
        let caps = ScrcpyCapabilities::unknown();
        let mode = DisplayMode::VirtualDisplay {
            width: 1920,
            height: 1080,
            dpi: 240,
        };
        let options = ScrcpySessionOptions {
            stay_awake: false,
            virtual_config: None,
        };

        let args = ScrcpyAdapter::build_arguments_advanced("serial123", mode, &caps, &options)
            .expect("deve realizar fallback para espelhamento sem erro fatal");

        assert_eq!(
            args,
            vec![
                "-s",
                "serial123",
                "--window-title",
                "Limivra Desktop (serial123)"
            ]
        );
        assert!(!args.iter().any(|a| a.starts_with("--new-display")));
    }
}
