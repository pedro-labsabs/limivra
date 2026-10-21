use crate::{CoreError, DisplayMode};
use tracing::{info, warn};

/// Versão analisada do scrcpy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrcpyVersion {
    pub major: u32,
    pub minor: u32,
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

        Some(Self {
            major,
            minor,
            raw: ver_str.to_string(),
        })
    }

    /// Verifica se a versão suporta a flag `--new-display` (introduzida na versão 2.2).
    pub fn supports_virtual_display(&self) -> bool {
        self.major > 2 || (self.major == 2 && self.minor >= 2)
    }
}

/// Adaptador para orquestração e construção de argumentos do scrcpy.
pub struct ScrcpyAdapter;

impl ScrcpyAdapter {
    /// Detecta a versão do scrcpy instalado no sistema executando `scrcpy --version`.
    pub async fn detect_version() -> Result<ScrcpyVersion, CoreError> {
        let output = tokio::process::Command::new("scrcpy")
            .arg("--version")
            .output()
            .await
            .map_err(|e| CoreError::DependencyMissing(format!("scrcpy não encontrado: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        ScrcpyVersion::parse(&stdout)
            .ok_or_else(|| CoreError::DependencyMissing("Saída de versão do scrcpy irreconhecível".into()))
    }

    /// Constrói a lista segura de argumentos para invocar o scrcpy sem privilégios de root.
    pub fn build_arguments(
        serial: &str,
        mode: DisplayMode,
        version: &ScrcpyVersion,
        stay_awake: bool,
    ) -> Result<Vec<String>, CoreError> {
        let mut args = vec![
            "-s".to_string(),
            serial.to_string(),
            "--window-title".to_string(),
            format!("Limivra Desktop ({})", serial),
        ];

        if stay_awake {
            args.push("--stay-awake".to_string());
        }

        match mode {
            DisplayMode::Mirror => {
                info!("Configurando scrcpy em modo espelhamento (Mirror)");
            }
            DisplayMode::VirtualDisplay { width, height, dpi } => {
                if !version.supports_virtual_display() {
                    warn!(
                        "scrcpy versão {} não suporta --new-display (exige >= 2.2). Realizando fallback para Mirror.",
                        version.raw
                    );
                    return Ok(args);
                }

                info!(
                    "Configurando display virtual secundário: {}x{} @ {} DPI",
                    width, height, dpi
                );
                args.push(format!("--new-display={}x{}/{}", width, height, dpi));
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
        assert!(!ver.supports_virtual_display());

        let output_v2 = "scrcpy 2.2 <https://github.com/Genymobile/scrcpy>";
        let ver2 = ScrcpyVersion::parse(output_v2).expect("deve parsear v2.2");
        assert_eq!(ver2.major, 2);
        assert_eq!(ver2.minor, 2);
        assert!(ver2.supports_virtual_display());

        let output_v3 = "scrcpy 3.0";
        let ver3 = ScrcpyVersion::parse(output_v3).expect("deve parsear v3");
        assert!(ver3.supports_virtual_display());
    }

    #[test]
    fn test_build_mirror_args() {
        let ver = ScrcpyVersion {
            major: 1,
            minor: 25,
            raw: "1.25".into(),
        };
        let args = ScrcpyAdapter::build_arguments("serial123", DisplayMode::Mirror, &ver, true)
            .expect("deve construir args");

        assert_eq!(args, vec![
            "-s", "serial123",
            "--window-title", "Limivra Desktop (serial123)",
            "--stay-awake"
        ]);
    }

    #[test]
    fn test_fallback_on_unsupported_virtual_display() {
        let ver = ScrcpyVersion {
            major: 1,
            minor: 25,
            raw: "1.25".into(),
        };
        let mode = DisplayMode::VirtualDisplay {
            width: 1920,
            height: 1080,
            dpi: 240,
        };
        // Deve fazer fallback para espelhamento sem quebrar a execução
        let args = ScrcpyAdapter::build_arguments("serial123", mode, &ver, false)
            .expect("deve construir args com fallback");

        assert!(!args.iter().any(|a| a.starts_with("--new-display")));
    }
}
