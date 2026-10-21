use serde::{Deserialize, Serialize};

/// Estado de autorização e prontidão de um dispositivo Android via ADB.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceStatus {
    /// Autorizado e pronto para interação.
    Device,
    /// Pendente de confirmação na tela do telefone.
    Unauthorized,
    /// Dispositivo em estado offline ou inacessível.
    Offline,
    /// Dispositivo em modo sideload/recovery.
    Recovery,
    /// Estado desconhecido reportado pelo daemon.
    Unknown,
}

/// Descritor de um dispositivo Android descoberto localmente.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdbDevice {
    /// Identificador/Serial do dispositivo (mantido apenas em memória).
    pub serial: String,
    /// Estado atual da autorização.
    pub status: DeviceStatus,
    /// Modelo comercial reportado (ex: "23129RA5FL").
    pub model: Option<String>,
    /// Produto reportado pelo sistema Android (ex: "sapphire_global").
    pub product: Option<String>,
    /// ID de transporte no subsistema ADB.
    pub transport_id: Option<String>,
}

impl AdbDevice {
    /// Retorna `true` se o dispositivo estiver plenamente autorizado e utilizável.
    pub fn is_ready(&self) -> bool {
        self.status == DeviceStatus::Device
    }
}

/// Analisa a saída em texto do comando `adb devices -l`.
pub fn parse_adb_devices_output(output: &str) -> Vec<AdbDevice> {
    let mut devices = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        // Ignora linhas vazias ou cabeçalhos do daemon
        if trimmed.is_empty()
            || trimmed.starts_with("List of devices")
            || trimmed.starts_with("* daemon")
        {
            continue;
        }

        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        let serial = parts[0].to_string();
        let status = if parts.len() > 1 {
            match parts[1] {
                "device" => DeviceStatus::Device,
                "unauthorized" => DeviceStatus::Unauthorized,
                "offline" => DeviceStatus::Offline,
                "recovery" | "sideload" => DeviceStatus::Recovery,
                _ => DeviceStatus::Unknown,
            }
        } else {
            DeviceStatus::Unknown
        };

        let mut model = None;
        let mut product = None;
        let mut transport_id = None;

        for part in &parts[2..] {
            if let Some(val) = part.strip_prefix("model:") {
                model = Some(val.to_string());
            } else if let Some(val) = part.strip_prefix("product:") {
                product = Some(val.to_string());
            } else if let Some(val) = part.strip_prefix("transport_id:") {
                transport_id = Some(val.to_string());
            }
        }

        devices.push(AdbDevice {
            serial,
            status,
            model,
            product,
            transport_id,
        });
    }

    devices
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_empty_devices() {
        let output = "List of devices attached\n\n";
        let devices = parse_adb_devices_output(output);
        assert!(devices.is_empty());
    }

    #[test]
    fn test_parse_unauthorized_and_ready() {
        let output = r#"
* daemon not running; starting now at tcp:5037
* daemon started successfully
List of devices attached
674d6cf9               device usb:1-4 product:sapphire_global model:23129RA5FL device:sapphire transport_id:1
emulator-5554          unauthorized transport_id:2
offline-dev            offline transport_id:3
"#;
        let devices = parse_adb_devices_output(output);
        assert_eq!(devices.len(), 3);

        assert_eq!(devices[0].serial, "674d6cf9");
        assert_eq!(devices[0].status, DeviceStatus::Device);
        assert_eq!(devices[0].model.as_deref(), Some("23129RA5FL"));
        assert_eq!(devices[0].product.as_deref(), Some("sapphire_global"));
        assert!(devices[0].is_ready());

        assert_eq!(devices[1].serial, "emulator-5554");
        assert_eq!(devices[1].status, DeviceStatus::Unauthorized);
        assert!(!devices[1].is_ready());

        assert_eq!(devices[2].serial, "offline-dev");
        assert_eq!(devices[2].status, DeviceStatus::Offline);
        assert!(!devices[2].is_ready());
    }
}
