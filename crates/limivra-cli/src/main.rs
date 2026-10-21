//! Limivra CLI - Interface de linha de comando para controle e diagnóstico do Limivra.

use clap::{Parser, Subcommand};
use limivra_core::{
    DeviceDiscovery, DeviceStatus, DisplayMode, ScrcpyAdapter, SessionController, SessionEvent,
};
use std::sync::Arc;

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

        /// Manter o dispositivo acordado enquanto conectado
        #[arg(long, default_value_t = true)]
        stay_awake: bool,
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
            stay_awake,
        } => {
            println!("Localizando dispositivo Android...");
            let device = match DeviceDiscovery::resolve_target(serial.as_deref()).await {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Falha na seleção do dispositivo: {}", e);
                    return;
                }
            };

            println!(
                "Dispositivo selecionado: {} ({})",
                device.serial,
                device.model.as_deref().unwrap_or("Modelo desconhecido")
            );

            let scrcpy_version = match ScrcpyAdapter::detect_version().await {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("Erro de dependência: {}", e);
                    return;
                }
            };
            println!("scrcpy detectado: v{}", scrcpy_version.raw);

            let mode = if virtual_display {
                DisplayMode::VirtualDisplay {
                    width: 1920,
                    height: 1080,
                    dpi: 240,
                }
            } else {
                DisplayMode::Mirror
            };

            let args = match ScrcpyAdapter::build_arguments(&device.serial, mode, &scrcpy_version, stay_awake) {
                Ok(a) => a,
                Err(e) => {
                    eprintln!("Erro ao construir argumentos: {}", e);
                    return;
                }
            };

            let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
            let controller = Arc::new(SessionController::new());
            let mut rx = controller.subscribe();

            let ctrl_clone = Arc::clone(&controller);
            tokio::spawn(async move {
                tokio::signal::ctrl_c().await.ok();
                println!("\nRecebido sinal de interrupção (Ctrl+C). Encerrando sessão de forma limpa...");
                ctrl_clone.stop().await.ok();
            });

            println!("Inicializando sessão de vídeo...");
            if let Err(e) = controller.start("scrcpy", &args_ref, &device.serial, mode).await {
                eprintln!("Erro ao iniciar sessão: {}", e);
                return;
            }

            while let Ok(event) = rx.recv().await {
                match event {
                    SessionEvent::StateChanged(s) => {
                        println!("Estado da Sessão: {:?}", s);
                    }
                    SessionEvent::ProcessExited(code) => {
                        println!("Sessão concluída (código de saída: {:?})", code);
                        break;
                    }
                    SessionEvent::Error(err) => {
                        eprintln!("Erro na sessão: {}", err);
                        break;
                    }
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
