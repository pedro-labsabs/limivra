use crate::{scrcpy::ScrcpyCapabilities, CoreError, DisplayMode};
use serde::{Deserialize, Serialize, Serializer};
use std::{ffi::OsString, path::Path, time::Duration};
use tracing::{info, warn};

const MAX_STRUCTURED_LINE_BYTES: usize = 16 * 1024;

/// Informações de um display detectado no dispositivo Android.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AndroidDisplayInfo {
    pub id: u32,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub dpi: u32,
    pub refresh_rate: Option<f32>,
    pub is_default: bool,
    pub is_virtual: bool,
    pub is_secure: bool,
    pub is_trusted: bool,
    pub raw_flags: Vec<String>,
}

/// Perfil de hardware, fabricante e versão do sistema do dispositivo alvo.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct DeviceProfile {
    #[serde(serialize_with = "serialize_redacted_serial")]
    pub serial: String,
    pub manufacturer: Option<String>,
    pub model: Option<String>,
    pub sdk_version: Option<u32>,
    pub release_version: Option<String>,
    pub incremental: Option<String>,
}

fn serialize_redacted_serial<S>(_: &str, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str("[REDACTED]")
}

impl std::fmt::Debug for DeviceProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DeviceProfile")
            .field("serial", &"[REDACTED]")
            .field("manufacturer", &self.manufacturer)
            .field("model", &self.model)
            .field("sdk_version", &self.sdk_version)
            .field("release_version", &self.release_version)
            .field("incremental", &self.incremental)
            .finish()
    }
}

/// Nível de suporte e certeza para criação de displays virtuais secundários.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VirtualDisplaySupport {
    /// Suporte confirmado documental e tecnicamente.
    Supported,
    /// Incompatibilidade técnica estrita.
    Unsupported { reason: String },
    /// Comportamento desconhecido ou restrito por políticas da fabricante sem prova empírica.
    /// "UNKNOWN não vira suportado sem prova."
    Unknown { reason: String },
}

/// Relatório consolidado de sondagem de display e compatibilidade.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayProbeReport {
    pub device: DeviceProfile,
    pub displays: Vec<AndroidDisplayInfo>,
    pub scrcpy_capabilities: ScrcpyCapabilities,
    pub virtual_display_support: VirtualDisplaySupport,
    pub recommended_mode: DisplayMode,
    pub diagnostics: Vec<String>,
}

/// Módulo de sondagem de displays e avaliação de compatibilidade Android.
pub struct DisplayProber;

impl DisplayProber {
    /// Analisa a saída bruta do comando `dumpsys display` do Android.
    pub fn parse_dumpsys_display_output(output: &str) -> Vec<AndroidDisplayInfo> {
        let mut displays = Vec::new();
        let mut current_id = 0u32;

        for line in output.lines() {
            if line.len() > MAX_STRUCTURED_LINE_BYTES || line.contains('\u{fffd}') {
                continue;
            }
            let trimmed = line.trim();

            // Rastreamento contextual de IDs de display se disponível
            if let Some(id_str) = trimmed.strip_prefix("Display Id=") {
                if let Ok(id) = id_str.trim().parse::<u32>() {
                    current_id = id;
                }
            } else if trimmed.starts_with("Display id ") {
                if let Some(rest) = trimmed.strip_prefix("Display id ") {
                    if let Some(token) = rest.split(':').next() {
                        if let Ok(id) = token.trim().parse::<u32>() {
                            current_id = id;
                        }
                    }
                }
            }

            // Identificação de blocos DisplayDeviceInfo{...}
            if let Some(start_idx) = trimmed.find("DisplayDeviceInfo{") {
                let info_content = &trimmed[start_idx + "DisplayDeviceInfo{".len()..];
                if let Some(end_idx) = info_content.rfind('}') {
                    if let Some(parsed_display) =
                        Self::parse_display_device_info(current_id, &info_content[..end_idx])
                    {
                        displays.push(parsed_display);
                    }
                }
                current_id = current_id.saturating_add(1);
            }
        }

        displays
    }

    /// Analisa o conteúdo interno de um bloco `DisplayDeviceInfo{...}`.
    fn parse_display_device_info(fallback_id: u32, content: &str) -> Option<AndroidDisplayInfo> {
        // Extrai o nome entre aspas (ex: "Tela integrada", "scrcpy")
        let first_quote = content.find('"')?;
        let after_first_quote = &content[first_quote + 1..];
        let second_quote = after_first_quote.find('"')?;
        let name = after_first_quote[..second_quote].to_string();
        if name.is_empty() || name.contains('\u{fffd}') {
            return None;
        }

        // Resolução: procura padrão "1080 x 2400" ou similar
        let (width, height) = content.split(',').find_map(|part| {
            let dims = part.split('x').map(str::trim).collect::<Vec<_>>();
            if dims.len() != 2 {
                return None;
            }
            match (dims[0].parse::<u32>(), dims[1].parse::<u32>()) {
                (Ok(width), Ok(height)) if width > 0 && height > 0 => Some((width, height)),
                _ => None,
            }
        })?;

        // Densidade: procura "density 440" ou "density=440"
        let mut dpi = 160u32;
        if let Some(idx) = content.find("density ") {
            let rest = &content[idx + "density ".len()..];
            if let Some(token) = rest.split(&[',', ' '][..]).next() {
                if let Ok(d) = token.parse::<u32>() {
                    dpi = d;
                }
            }
        } else if let Some(idx) = content.find("density=") {
            let rest = &content[idx + "density=".len()..];
            if let Some(token) = rest.split(&[',', ' '][..]).next() {
                if let Ok(d) = token.parse::<u32>() {
                    dpi = d;
                }
            }
        }

        // Taxa de atualização (renderFrameRate)
        let mut refresh_rate = None;
        if let Some(idx) = content.find("renderFrameRate ") {
            let rest = &content[idx + "renderFrameRate ".len()..];
            if let Some(token) = rest.split(&[',', ' '][..]).next() {
                if let Ok(rate) = token.parse::<f32>() {
                    refresh_rate = Some(rate);
                }
            }
        }

        // Flags do display
        let mut raw_flags = Vec::new();
        let is_default = content.contains("FLAG_ALLOWED_TO_BE_DEFAULT_DISPLAY")
            || content.contains("uniqueId=\"local:")
            || fallback_id == 0;
        let is_virtual = content.contains("type VIRTUAL")
            || content.contains("VirtualDisplayAdapter")
            || content.contains("uniqueId=\"virtual:")
            || name.to_lowercase().contains("scrcpy");
        let is_secure = content.contains("FLAG_SECURE");
        let is_trusted = content.contains("FLAG_TRUSTED");

        for token in content.split(&[',', ' ', '}'][..]) {
            let t = token.trim();
            if t.starts_with("FLAG_") {
                raw_flags.push(t.to_string());
            }
        }

        Some(AndroidDisplayInfo {
            id: fallback_id,
            name,
            width,
            height,
            dpi,
            refresh_rate,
            is_default,
            is_virtual,
            is_secure,
            is_trusted,
            raw_flags,
        })
    }

    /// Analisa a saída simplificada do comando `scrcpy --list-displays`.
    pub fn parse_scrcpy_list_displays_output(output: &str) -> Vec<AndroidDisplayInfo> {
        let mut displays = Vec::new();

        for line in output.lines() {
            if line.len() > MAX_STRUCTURED_LINE_BYTES || line.contains('\u{fffd}') {
                continue;
            }
            let trimmed = line.trim();
            if !trimmed.starts_with("--display-id=") {
                continue;
            }

            let rest = &trimmed["--display-id=".len()..];
            let mut parts = rest.split_whitespace();
            let id_token = match parts.next() {
                Some(t) => t,
                None => continue,
            };

            let id = match id_token.parse::<u32>() {
                Ok(i) => i,
                Err(_) => continue,
            };

            let Some(res_token) = parts.next() else {
                continue;
            };
            let cleaned = res_token.trim_matches(|c| c == '(' || c == ')');
            let dims: Vec<&str> = cleaned.split('x').collect();
            if dims.len() != 2 {
                continue;
            }
            let (Ok(width), Ok(height)) = (dims[0].parse::<u32>(), dims[1].parse::<u32>()) else {
                continue;
            };
            if width == 0 || height == 0 {
                continue;
            }

            displays.push(AndroidDisplayInfo {
                id,
                name: format!("Display-{id}"),
                width,
                height,
                dpi: 160,
                refresh_rate: None,
                is_default: id == 0,
                is_virtual: id != 0,
                is_secure: false,
                is_trusted: false,
                raw_flags: Vec::new(),
            });
        }

        displays
    }

    /// Sonda as propriedades de hardware e sistema do dispositivo via comandos ADB shell getprop.
    pub async fn probe_device_profile(serial: &str) -> Result<DeviceProfile, CoreError> {
        let get_prop = |prop: &'static str| async move {
            let args = [
                OsString::from("-s"),
                OsString::from(serial),
                OsString::from("shell"),
                OsString::from("getprop"),
                OsString::from(prop),
            ];
            let output = crate::process::run_bounded(
                Path::new("adb"),
                &args,
                Duration::from_secs(5),
                1024 * 1024,
            )
            .await;

            match output {
                Ok(out) if out.status.success() => {
                    let val = std::str::from_utf8(&out.stdout).ok()?.trim().to_string();
                    if val.is_empty() {
                        None
                    } else {
                        Some(val)
                    }
                }
                _ => None,
            }
        };

        let manufacturer = get_prop("ro.product.manufacturer").await;
        let model = get_prop("ro.product.model").await;
        let sdk_str = get_prop("ro.build.version.sdk").await;
        let release = get_prop("ro.build.version.release").await;
        let incremental = get_prop("ro.build.version.incremental").await;

        let sdk_version = sdk_str.as_deref().and_then(|s| s.parse::<u32>().ok());

        Ok(DeviceProfile {
            serial: serial.to_string(),
            manufacturer,
            model,
            sdk_version,
            release_version: release,
            incremental,
        })
    }

    /// Sonda a lista de displays ativos no dispositivo via `dumpsys display`.
    pub async fn probe_displays(serial: &str) -> Result<Vec<AndroidDisplayInfo>, CoreError> {
        let args = [
            OsString::from("-s"),
            OsString::from(serial),
            OsString::from("shell"),
            OsString::from("dumpsys"),
            OsString::from("display"),
        ];
        let output = crate::process::run_bounded(
            Path::new("adb"),
            &args,
            Duration::from_secs(5),
            1024 * 1024,
        )
        .await
        .map_err(|e| CoreError::AdbError(format!("Falha ao invocar dumpsys display: {e}")))?;

        if !output.status.success() {
            return Err(CoreError::AdbError(
                "dumpsys display retornou status de erro".into(),
            ));
        }

        let stdout = std::str::from_utf8(&output.stdout).map_err(|_| {
            CoreError::AdbError("saída de dumpsys display não é UTF-8 válida".into())
        })?;
        let parsed = Self::parse_dumpsys_display_output(stdout);
        Ok(parsed)
    }

    /// Avalia o suporte a displays virtuais com base nas regras AOSP, restrições scrcpy e perfil OEM.
    ///
    /// Regra fundamental do projeto: "UNKNOWN não vira suportado sem prova!"
    pub fn evaluate_virtual_display_support(
        device: &DeviceProfile,
        caps: &ScrcpyCapabilities,
        _displays: &[AndroidDisplayInfo],
    ) -> VirtualDisplaySupport {
        // Verificação 1: scrcpy suporta a flag --new-display?
        if !caps.supports_new_display {
            return VirtualDisplaySupport::Unsupported {
                reason: "O binário scrcpy instalado não suporta a flag --new-display (exige scrcpy >= 2.2 com suporte comprovado)".to_string(),
            };
        }

        // Verificação 2: versão do Android SDK
        let sdk = match device.sdk_version {
            Some(s) => s,
            None => {
                return VirtualDisplaySupport::Unknown {
                    reason: "Versão da API Android (SDK) desconhecida; incapaz de certificar suporte a VirtualDisplay".to_string(),
                };
            }
        };

        if sdk < 29 {
            return VirtualDisplaySupport::Unsupported {
                reason: format!(
                    "Android SDK {sdk} (< 29 / Android 10) não suporta criação de display secundário via Shell UID 2000"
                ),
            };
        }

        // Verificação 3: Quirk de fabricantes com restrições proprietárias de segurança (Xiaomi / MIUI / HyperOS)
        let is_xiaomi = device
            .manufacturer
            .as_deref()
            .map(|m| {
                m.eq_ignore_ascii_case("xiaomi")
                    || m.eq_ignore_ascii_case("redmi")
                    || m.eq_ignore_ascii_case("poco")
            })
            .unwrap_or(false);

        if is_xiaomi {
            // Em ROMs HyperOS/MIUI, injeção de input e multi-display dependem de autorizações adicionais
            // na interface de desenvolvedor ("Depuração USB (Configurações de segurança)").
            // Portanto, sem prova empírica em tempo de execução, deve ser classificado estritamente como UNKNOWN.
            return VirtualDisplaySupport::Unknown {
                reason: "Dispositivo Xiaomi/HyperOS detectado: criação de tela secundária e injeção de entrada dependem de opções de segurança da fabricante (USB debugging Security settings)".to_string(),
            };
        }

        // Dispositivos genéricos Android 10+
        VirtualDisplaySupport::Supported
    }

    /// Realiza sondagem completa de hardware, display e scrcpy para o dispositivo indicado.
    pub async fn probe_device(
        serial: &str,
        caps: &ScrcpyCapabilities,
    ) -> Result<DisplayProbeReport, CoreError> {
        let device = Self::probe_device_profile(serial).await?;
        let displays = Self::probe_displays(serial).await.unwrap_or_else(|e| {
            warn!("Aviso: falha na sondagem via dumpsys display: {}", e);
            Vec::new()
        });

        let support = Self::evaluate_virtual_display_support(&device, caps, &displays);

        let mut diagnostics = Vec::new();
        let recommended_mode = match &support {
            VirtualDisplaySupport::Supported => {
                diagnostics.push("VirtualDisplay secundário suportado e recomendado.".to_string());
                DisplayMode::VirtualDisplay {
                    width: 1920,
                    height: 1080,
                    dpi: 240,
                }
            }
            VirtualDisplaySupport::Unsupported { reason } => {
                diagnostics.push(format!("VirtualDisplay não suportado: {reason}"));
                diagnostics.push("Recomendado modo de espelhamento (Mirror).".to_string());
                DisplayMode::Mirror
            }
            VirtualDisplaySupport::Unknown { reason } => {
                diagnostics.push(format!(
                    "VirtualDisplay com comportamento desconhecido/não provado: {reason}"
                ));
                diagnostics.push("Recomendado iniciar com espelhamento (Mirror) ou testar VirtualDisplay com salvaguardas.".to_string());
                DisplayMode::Mirror
            }
        };

        log_probe_summary(serial, &support);

        Ok(DisplayProbeReport {
            device,
            displays,
            scrcpy_capabilities: caps.clone(),
            virtual_display_support: support,
            recommended_mode,
            diagnostics,
        })
    }
}

fn log_probe_summary(_serial: &str, support: &VirtualDisplaySupport) {
    info!(
        "Sondagem de display concluída para dispositivo [REDACTED]: {:?}",
        support
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scrcpy::CapabilityProbeSource;
    use std::io::{self, Write};
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    struct LogBuffer(Arc<Mutex<Vec<u8>>>);

    impl<'a> MakeWriter<'a> for LogBuffer {
        type Writer = Self;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    impl Write for LogBuffer {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            self.0
                .lock()
                .map_err(|_| io::Error::other("log buffer poisoned"))?
                .extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn display_probe_log_redacts_serial() {
        const SERIAL: &str = "LIMIVRA-TEST-SERIAL-XYZ";
        let captured = LogBuffer::default();
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .without_time()
            .with_writer(captured.clone())
            .finish();

        tracing::subscriber::with_default(subscriber, || {
            log_probe_summary(SERIAL, &VirtualDisplaySupport::Supported);
        });

        let output = String::from_utf8(captured.0.lock().unwrap().clone()).unwrap();
        assert!(
            !output.contains(SERIAL),
            "display probe log leaked the serial: {output}"
        );
        assert!(output.contains("[REDACTED]"));
    }

    const SAMPLE_DUMPSYS_OUTPUT: &str = r#"
DISPLAY MANAGER (dumpsys display)
Display States: size=1
  Display Id=0
  Display State=ON

Display Devices: size=1
  DisplayDeviceInfo{"Tela integrada": uniqueId="local:4630946359590559105", 1080 x 2400, modeId 3, renderFrameRate 60.000004, defaultModeId 1, userPreferredModeId -1, supportedModes [{id=1, width=1080, height=2400, fps=120.00001}], density 440, 394.705 x 394.307 dpi, touch INTERNAL, rotation 0, type INTERNAL, FLAG_ALLOWED_TO_BE_DEFAULT_DISPLAY, FLAG_ROTATES_WITH_CONTENT, FLAG_SECURE, FLAG_SUPPORTS_PROTECTED_BUFFERS, FLAG_TRUSTED}
"#;

    const SAMPLE_MULTI_DISPLAY_OUTPUT: &str = r#"
Display Devices: size=2
  DisplayDeviceInfo{"Tela integrada": uniqueId="local:123", 1080 x 2400, renderFrameRate 120.0, density 440, FLAG_ALLOWED_TO_BE_DEFAULT_DISPLAY, FLAG_TRUSTED}
  DisplayDeviceInfo{"scrcpy": uniqueId="virtual:456", 1920 x 1080, renderFrameRate 60.0, density 240, type VIRTUAL, FLAG_PRIVATE}
"#;

    #[test]
    fn test_parse_dumpsys_single_display() {
        let displays = DisplayProber::parse_dumpsys_display_output(SAMPLE_DUMPSYS_OUTPUT);
        assert_eq!(displays.len(), 1);

        let d = &displays[0];
        assert_eq!(d.id, 0);
        assert_eq!(d.name, "Tela integrada");
        assert_eq!(d.width, 1080);
        assert_eq!(d.height, 2400);
        assert_eq!(d.dpi, 440);
        assert_eq!(d.refresh_rate, Some(60.000004));
        assert!(d.is_default);
        assert!(!d.is_virtual);
        assert!(d.is_secure);
        assert!(d.is_trusted);
    }

    #[test]
    fn test_parse_dumpsys_multi_display_with_virtual() {
        let displays = DisplayProber::parse_dumpsys_display_output(SAMPLE_MULTI_DISPLAY_OUTPUT);
        assert_eq!(displays.len(), 2);

        let primary = &displays[0];
        assert_eq!(primary.name, "Tela integrada");
        assert!(!primary.is_virtual);
        assert!(primary.is_default);

        let virtual_disp = &displays[1];
        assert_eq!(virtual_disp.name, "scrcpy");
        assert_eq!(virtual_disp.width, 1920);
        assert_eq!(virtual_disp.height, 1080);
        assert_eq!(virtual_disp.dpi, 240);
        assert!(virtual_disp.is_virtual);
        assert!(!virtual_disp.is_default);
    }

    #[test]
    fn test_parse_scrcpy_list_displays() {
        let output = r#"
[server] INFO: Device: [Xiaomi] Redmi 23129RA5FL (Android 15)
[server] INFO: List of displays:
    --display-id=0    (1080x2400)
    --display-id=2    (1920x1080)
scrcpy 5.0.1 <https://github.com/Genymobile/scrcpy>
"#;
        let displays = DisplayProber::parse_scrcpy_list_displays_output(output);
        assert_eq!(displays.len(), 2);

        assert_eq!(displays[0].id, 0);
        assert_eq!(displays[0].width, 1080);
        assert_eq!(displays[0].height, 2400);
        assert!(displays[0].is_default);

        assert_eq!(displays[1].id, 2);
        assert_eq!(displays[1].width, 1920);
        assert_eq!(displays[1].height, 1080);
        assert!(displays[1].is_virtual);
    }

    #[test]
    fn test_compatibility_unsupported_scrcpy() {
        let device = DeviceProfile {
            serial: "device1".into(),
            manufacturer: Some("Google".into()),
            model: Some("Pixel 8".into()),
            sdk_version: Some(34),
            release_version: Some("14".into()),
            incremental: None,
        };
        let mut caps = ScrcpyCapabilities::unknown();
        caps.supports_new_display = false;

        let res = DisplayProber::evaluate_virtual_display_support(&device, &caps, &[]);
        match res {
            VirtualDisplaySupport::Unsupported { reason } => {
                assert!(reason.contains("--new-display"));
            }
            _ => panic!("Esperado Unsupported devido a scrcpy"),
        }
    }

    #[test]
    fn test_compatibility_unsupported_android_sdk() {
        let device = DeviceProfile {
            serial: "device2".into(),
            manufacturer: Some("Google".into()),
            model: Some("Pixel 3".into()),
            sdk_version: Some(28), // Android 9 Pie
            release_version: Some("9".into()),
            incremental: None,
        };
        let mut caps = ScrcpyCapabilities::unknown();
        caps.supports_new_display = true;

        let res = DisplayProber::evaluate_virtual_display_support(&device, &caps, &[]);
        match res {
            VirtualDisplaySupport::Unsupported { reason } => {
                assert!(reason.contains("SDK 28"));
            }
            _ => panic!("Esperado Unsupported devido a Android SDK < 29"),
        }
    }

    #[test]
    fn test_compatibility_xiaomi_hyperos_is_unknown_without_evidence() {
        // "UNKNOWN não vira suportado sem prova!"
        let device = DeviceProfile {
            serial: "674d6cf9".into(),
            manufacturer: Some("Xiaomi".into()),
            model: Some("23129RA5FL".into()),
            sdk_version: Some(35),
            release_version: Some("15".into()),
            incremental: Some("OS2.0.1.0".into()),
        };
        let mut caps = ScrcpyCapabilities::unknown();
        caps.supports_new_display = true;

        let res = DisplayProber::evaluate_virtual_display_support(&device, &caps, &[]);
        match res {
            VirtualDisplaySupport::Unknown { reason } => {
                assert!(reason.contains("Xiaomi/HyperOS"));
            }
            _ => panic!("Esperado Unknown para Xiaomi/HyperOS sem evidência empírica"),
        }
    }

    #[test]
    fn test_compatibility_generic_android_14_is_supported() {
        let device = DeviceProfile {
            serial: "pixel-device".into(),
            manufacturer: Some("Google".into()),
            model: Some("Pixel 7".into()),
            sdk_version: Some(34),
            release_version: Some("14".into()),
            incremental: None,
        };
        let caps = ScrcpyCapabilities {
            version: None,
            supports_new_display: true,
            supports_start_app: true,
            supports_vd_system_decorations: true,
            supports_vd_destroy_content: true,
            supports_display_ime_policy: true,
            supports_list_displays: true,
            supports_stay_awake: true,
            supports_flex_display: true,
            probe_source: CapabilityProbeSource::HelpInspection,
        };

        let res = DisplayProber::evaluate_virtual_display_support(&device, &caps, &[]);
        assert_eq!(res, VirtualDisplaySupport::Supported);
    }
}
