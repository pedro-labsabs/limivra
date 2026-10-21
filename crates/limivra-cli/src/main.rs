//! Limivra CLI - Interface de linha de comando para controle e diagnóstico do Limivra.

use clap::{Parser, Subcommand};
use limivra_core::{DeviceDiscovery, DeviceStatus, DisplayMode, SessionState};

#[derive(Parser, Debug)]
#[command(name = "limivra")]
#[command(about = "Acesse e controle dispositivos Android a partir do Linux como uma área de trabalho.")]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Lista todos os dispositivos Android conectados via ADB e seus estados
    Devices,

    /// Inicia uma sessão de exibição
    Start {
        /// Número de série do dispositivo Android (opcional se houver apenas um)
        #[arg(short, long)]
        serial: Option<String>,

        /// Habilitar modo de tela virtual independente (scrcpy --new-display)
        #[arg(long, default_value_t = false)]
        virtual_display: bool,
    },

    /// Verifica a disponibilidade das dependências no sistema host (ADB e scrcpy)
    Doctor,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();

    match cli.command {
        Commands::Devices => match DeviceDiscovery::list_devices().await {
            Ok(devices) if devices.is_empty() => {
                println!("Nenhum dispositivo Android detectado.");
            }
            Ok(devices) => {
                println!("Dispositivos conectados:");
                for d in devices {
                    let status_desc = match d.status {
                        DeviceStatus::Device => "[Pronto / Autorizado]",
                        DeviceStatus::Unauthorized => "[Não autorizado — confirme na tela do celular]",
                        DeviceStatus::Offline => "[Offline]",
                        DeviceStatus::Recovery => "[Recovery/Sideload]",
                        DeviceStatus::Unknown => "[Desconhecido]",
                    };
                    let model_desc = d.model.as_deref().unwrap_or("Modelo desconhecido");
                    println!("  • {} ({}) — {}", d.serial, model_desc, status_desc);
                }
            }
            Err(e) => {
                eprintln!("Erro ao listar dispositivos ADB: {}", e);
            }
        },
        Commands::Start {
            serial,
            virtual_display,
        } => {
            println!("Localizando dispositivo Android...");
            match DeviceDiscovery::resolve_target(serial.as_deref()).await {
                Ok(device) => {
                    let mode = if virtual_display {
                        DisplayMode::VirtualDisplay {
                            width: 1920,
                            height: 1080,
                            dpi: 240,
                        }
                    } else {
                        DisplayMode::Mirror
                    };
                    let state = SessionState::Active {
                        serial: device.serial.clone(),
                        mode,
                    };
                    println!(
                        "Dispositivo selecionado: {} ({:?})\nEstado da Sessão: {:?}",
                        device.serial,
                        device.model.unwrap_or_default(),
                        state
                    );
                }
                Err(e) => {
                    eprintln!("Falha na seleção do dispositivo: {}", e);
                }
            }
        }
        Commands::Doctor => {
            println!("Executando diagnóstico de ambiente Limivra...");
            check_tool("adb");
            check_tool("scrcpy");
        }
    }
}

fn check_tool(tool: &str) {
    match std::process::Command::new(tool).arg("--version").output() {
        Ok(output) if output.status.success() => {
            let first_line = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or("")
                .to_string();
            println!("  [OK] {}: {}", tool, first_line);
        }
        _ => {
            println!("  [FALHA] {}: não encontrado no PATH", tool);
        }
    }
}
