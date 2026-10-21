//! Limivra CLI - Interface de linha de comando para controle e diagnóstico do Limivra.

use clap::{Parser, Subcommand};
use limivra_core::{DisplayMode, SessionState};

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
        Commands::Start {
            serial,
            virtual_display,
        } => {
            let mode = if virtual_display {
                DisplayMode::VirtualDisplay {
                    width: 1920,
                    height: 1080,
                    dpi: 240,
                }
            } else {
                DisplayMode::Mirror
            };
            let serial_str = serial.unwrap_or_else(|| "default".to_string());
            let state = SessionState::Active {
                serial: serial_str.clone(),
                mode,
            };
            println!("Inicializando sessão Limivra para {}: {:?}", serial_str, state);
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
