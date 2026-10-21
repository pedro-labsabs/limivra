//! Limivra CLI - Interface de linha de comando para controle e diagnóstico do Limivra.

use clap::{Parser, Subcommand};
use limivra_core::{
    AppLaunchBackend, DeviceDiscovery, DeviceStatus, DisplayMode, DisplayProber, ScrcpyAdapter,
    SessionController, SessionEvent, VirtualDisplayConfig, VirtualDisplayProvider,
    VirtualDisplaySupport,
};
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "limivra")]
#[command(
    about = "Acesse e controle dispositivos Android a partir do Linux como uma área de trabalho."
)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Lista todos os dispositivos Android conectados via ADB e seus estados
    Devices,

    /// Sonda displays ativos no dispositivo e avalia compatibilidade de VirtualDisplay
    Probe {
        /// Número de série do dispositivo Android (opcional se houver apenas um)
        #[arg(short, long)]
        serial: Option<String>,
    },

    /// Inicia uma sessão de exibição
    Start {
        /// Número de série do dispositivo Android (opcional se houver apenas um)
        #[arg(short, long)]
        serial: Option<String>,

        /// Habilitar modo de tela virtual independente (scrcpy --new-display)
        #[arg(long, default_value_t = false)]
        virtual_display: bool,

        /// Pacote ou componente de aplicativo Android a ser lançado no display virtual (ex: com.android.settings)
        #[arg(long)]
        app: Option<String>,

        /// Backend de lançamento do aplicativo ('scrcpy' via flag --start-app ou 'am' via ADB shell am start --display <id>)
        #[arg(long, default_value = "scrcpy")]
        app_backend: String,

        /// Manter o dispositivo acordado enquanto conectado
        #[arg(long, default_value_t = true)]
        stay_awake: bool,
    },

    /// Verifica a disponibilidade e compatibilidade das ferramentas no sistema host
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
                        DeviceStatus::Unauthorized => {
                            "[Não autorizado — confirme na tela do celular]"
                        }
                        DeviceStatus::Offline => "[Offline]",
                        DeviceStatus::Recovery => "[Recovery/Sideload]",
                        DeviceStatus::Unknown => "[Desconhecido]",
                    };
                    let model_desc = d.model.as_deref().unwrap_or("Modelo desconhecido");
                    println!("  • {} ({model_desc}) — {status_desc}", d.serial);
                }
            }
            Err(e) => {
                eprintln!("Erro ao listar dispositivos ADB: {e}");
            }
        },
        Commands::Probe { serial } => {
            println!("Localizando dispositivo Android...");
            let device = match DeviceDiscovery::resolve_target(serial.as_deref()).await {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Falha na seleção do dispositivo: {e}");
                    return;
                }
            };

            let caps = match ScrcpyAdapter::detect_capabilities().await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Erro ao avaliar scrcpy: {e}");
                    return;
                }
            };

            println!("Sondando dispositivo {}...", device.serial);
            match DisplayProber::probe_device(&device.serial, &caps).await {
                Ok(report) => {
                    println!("\n=== Relatório de Compatibilidade Limivra ===");
                    println!("Dispositivo: {}", report.device.serial);
                    println!(
                        "  • Fabricante / Modelo: {} / {}",
                        report
                            .device
                            .manufacturer
                            .as_deref()
                            .unwrap_or("Desconhecido"),
                        report.device.model.as_deref().unwrap_or("Desconhecido")
                    );
                    println!(
                        "  • Android: {} (SDK {})",
                        report.device.release_version.as_deref().unwrap_or("?"),
                        report
                            .device
                            .sdk_version
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| "?".into())
                    );
                    if let Some(inc) = &report.device.incremental {
                        println!("  • Build incremental: {inc}");
                    }

                    println!("\nDisplays detectados no Android (dumpsys):");
                    if report.displays.is_empty() {
                        println!("  (Nenhum display retornado via dumpsys)");
                    } else {
                        for d in &report.displays {
                            let disp_type = if d.is_virtual {
                                "Virtual/Secundário"
                            } else if d.is_default {
                                "Físico Principal (Default)"
                            } else {
                                "Secundário"
                            };
                            let fps = d
                                .refresh_rate
                                .map(|f| format!(" @ {f:.0} Hz"))
                                .unwrap_or_default();
                            println!(
                                "  • ID {}: \"{}\" — {}x{} ({} DPI){fps}",
                                d.id, d.name, d.width, d.height, d.dpi
                            );
                            println!(
                                "    Tipo: {disp_type} | Confiável (TRUSTED): {}",
                                d.is_trusted
                            );
                        }
                    }

                    println!("\nCapacidades scrcpy instaladas:");
                    let ver_str = caps
                        .version
                        .as_ref()
                        .map(|v| format!("v{}", v.raw))
                        .unwrap_or_else(|| "Desconhecida".into());
                    println!("  • Versão: {ver_str}");
                    println!("  • Fonte de inspeção: {:?}", caps.probe_source);
                    println!(
                        "  • Flag --new-display: {}",
                        if caps.supports_new_display {
                            "Sim [OK]"
                        } else {
                            "Não [Indisponível]"
                        }
                    );
                    println!(
                        "  • Flag --start-app: {}",
                        if caps.supports_start_app {
                            "Sim [OK]"
                        } else {
                            "Não [Indisponível]"
                        }
                    );
                    println!(
                        "  • Flag --display-ime-policy: {}",
                        if caps.supports_display_ime_policy {
                            "Sim [OK]"
                        } else {
                            "Não [Indisponível]"
                        }
                    );
                    println!(
                        "  • Flag --no-vd-system-decorations: {}",
                        if caps.supports_vd_system_decorations {
                            "Sim [OK]"
                        } else {
                            "Não [Indisponível]"
                        }
                    );

                    println!("\nResultado da Avaliação:");
                    match &report.virtual_display_support {
                        VirtualDisplaySupport::Supported => {
                            println!(
                                "  [SUPORTADO] O ambiente atende aos requisitos documentados para VirtualDisplay."
                            );
                        }
                        VirtualDisplaySupport::Unsupported { reason } => {
                            println!("  [NÃO SUPORTADO] {reason}");
                        }
                        VirtualDisplaySupport::Unknown { reason } => {
                            println!("  [UNKNOWN / REQUER PROVA] {reason}");
                            println!(
                                "  (Regra: UNKNOWN não vira suportado sem evidência empírica)"
                            );
                        }
                    }

                    println!("  • Modo recomendado: {:?}", report.recommended_mode);
                    for diag in &report.diagnostics {
                        println!("    -> {diag}");
                    }
                }
                Err(e) => {
                    eprintln!("Erro ao sondar displays: {e}");
                }
            }
        }
        Commands::Start {
            serial,
            virtual_display,
            app,
            app_backend,
            stay_awake,
        } => {
            println!("Localizando dispositivo Android...");
            let device = match DeviceDiscovery::resolve_target(serial.as_deref()).await {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("Falha na seleção do dispositivo: {e}");
                    return;
                }
            };

            println!(
                "Dispositivo selecionado: {} ({})",
                device.serial,
                device.model.as_deref().unwrap_or("Modelo desconhecido")
            );

            let caps = match ScrcpyAdapter::detect_capabilities().await {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Erro de dependência: {e}");
                    return;
                }
            };

            if let Some(v) = &caps.version {
                println!(
                    "scrcpy detectado: v{} (inspeção: {:?})",
                    v.raw, caps.probe_source
                );
            }

            let is_virtual = virtual_display || app.is_some();

            if is_virtual {
                if !caps.supports_new_display {
                    println!("[AVISO] scrcpy instalado não suporta criação de display virtual (--new-display).");
                    println!("[FALLBACK] Executando em modo espelhamento (Mirror).");
                    run_mirror_session(&device.serial, &caps, stay_awake).await;
                    return;
                }

                let launch_backend = if app_backend.eq_ignore_ascii_case("am") {
                    AppLaunchBackend::AdbAmStart
                } else {
                    AppLaunchBackend::ScrcpyStartApp
                };

                let config = VirtualDisplayConfig {
                    width: 1920,
                    height: 1080,
                    dpi: 240,
                    app: app.clone(),
                    launch_backend,
                    ..Default::default()
                };

                if let Some(ref a) = app {
                    println!("Configurando lançamento do app: '{a}' (backend: {launch_backend:?})");
                }

                let provider = Arc::new(VirtualDisplayProvider::new());
                let mut rx = provider.subscribe();

                let prov_clone = Arc::clone(&provider);
                tokio::spawn(async move {
                    tokio::signal::ctrl_c().await.ok();
                    println!(
                        "\nRecebido sinal de interrupção (Ctrl+C). Encerrando sessão de forma limpa..."
                    );
                    prov_clone.stop().await.ok();
                });

                println!("Inicializando VirtualDisplayProvider...");
                if let Err(e) = provider
                    .start(&device.serial, &config, &caps, stay_awake)
                    .await
                {
                    eprintln!("Erro ao iniciar display virtual: {e}");
                    return;
                }

                while let Ok(event) = rx.recv().await {
                    match event {
                        SessionEvent::StateChanged(s) => {
                            println!("Estado da Sessão: {s:?}");
                        }
                        SessionEvent::ProcessExited(code) => {
                            println!("Sessão concluída (código de saída: {code:?})");
                            break;
                        }
                        SessionEvent::Error(err) => {
                            eprintln!("Erro na sessão: {err}");
                            break;
                        }
                    }
                }
            } else {
                run_mirror_session(&device.serial, &caps, stay_awake).await;
            }
        }
        Commands::Doctor => {
            println!("Executando diagnóstico de ambiente Limivra...");
            check_tool("adb");
            check_scrcpy().await;
        }
    }
}

async fn run_mirror_session(
    serial: &str,
    caps: &limivra_core::ScrcpyCapabilities,
    stay_awake: bool,
) {
    let args = match ScrcpyAdapter::build_arguments_with_capabilities(
        serial,
        DisplayMode::Mirror,
        caps,
        stay_awake,
    ) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Erro ao construir argumentos: {e}");
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

    println!("Inicializando sessão de espelhamento (Mirror) com argumentos: {args_ref:?}");
    if let Err(e) = controller
        .start("scrcpy", &args_ref, serial, DisplayMode::Mirror)
        .await
    {
        eprintln!("Erro ao iniciar sessão: {e}");
        return;
    }

    while let Ok(event) = rx.recv().await {
        match event {
            SessionEvent::StateChanged(s) => {
                println!("Estado da Sessão: {s:?}");
            }
            SessionEvent::ProcessExited(code) => {
                println!("Sessão concluída (código de saída: {code:?})");
                break;
            }
            SessionEvent::Error(err) => {
                eprintln!("Erro na sessão: {err}");
                break;
            }
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
            println!("  [OK] {tool}: {first_line}");
        }
        _ => {
            println!("  [FALHA] {tool}: não encontrado no PATH");
        }
    }
}

async fn check_scrcpy() {
    match ScrcpyAdapter::detect_capabilities().await {
        Ok(caps) => {
            let ver = caps
                .version
                .as_ref()
                .map(|v| format!("v{}", v.raw))
                .unwrap_or_else(|| "desconhecida".into());
            println!("  [OK] scrcpy: {ver}");
            println!("       • Fonte da prova: {:?}", caps.probe_source);
            println!(
                "       • Suporta --new-display: {}",
                caps.supports_new_display
            );
            println!("       • Suporta --start-app: {}", caps.supports_start_app);
            println!(
                "       • Suporta decorações (--no-vd-system-decorations): {}",
                caps.supports_vd_system_decorations
            );
            println!(
                "       • Suporta política IME (--display-ime-policy): {}",
                caps.supports_display_ime_policy
            );
            println!(
                "       • Suporta --list-displays: {}",
                caps.supports_list_displays
            );
            println!(
                "       • Suporta --stay-awake: {}",
                caps.supports_stay_awake
            );
        }
        Err(e) => {
            println!("  [FALHA] scrcpy: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_parsing_doctor() {
        let cli = Cli::try_parse_from(["limivra", "doctor"]).expect("deve parsear doctor");
        match cli.command {
            Commands::Doctor => {}
            _ => panic!("Esperado comando Doctor"),
        }
    }

    #[test]
    fn test_cli_parsing_probe() {
        let cli = Cli::try_parse_from(["limivra", "probe", "--serial", "device123"])
            .expect("deve parsear probe");
        match cli.command {
            Commands::Probe { serial } => {
                assert_eq!(serial.as_deref(), Some("device123"));
            }
            _ => panic!("Esperado comando Probe"),
        }
    }

    #[test]
    fn test_cli_parsing_start_with_virtual_display_and_app() {
        let cli = Cli::try_parse_from([
            "limivra",
            "start",
            "--virtual-display",
            "--app",
            "com.android.settings",
            "--app-backend",
            "am",
            "--stay-awake",
        ])
        .expect("deve parsear start com app e backend am");
        match cli.command {
            Commands::Start {
                serial,
                virtual_display,
                app,
                app_backend,
                stay_awake,
            } => {
                assert_eq!(serial, None);
                assert!(virtual_display);
                assert_eq!(app.as_deref(), Some("com.android.settings"));
                assert_eq!(app_backend, "am");
                assert!(stay_awake);
            }
            _ => panic!("Esperado comando Start"),
        }
    }
}
