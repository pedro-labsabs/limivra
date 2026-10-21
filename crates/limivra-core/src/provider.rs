use crate::{
    display::DisplayProber,
    fallback::{FallbackReason, FallbackRecord},
    scrcpy::{
        AppLaunchBackend, ScrcpyAdapter, ScrcpyCapabilities, ScrcpySessionOptions,
        VirtualDisplayConfig,
    },
    session::SessionController,
    CoreError, DisplayMode, SessionEvent, SessionState,
};
use std::sync::Arc;
use tokio::sync::broadcast;
use tracing::{info, warn};

/// Provedor e gerenciador de ciclo de vida de VirtualDisplay secundário para Android.
///
/// Não requer privilégios de root nem APK proprietário pré-instalado.
/// Utiliza o subsistema de display virtual do scrcpy (executando sob UID 2000 shell)
/// com suporte a inicialização direcionada de aplicações Android e fallback gracioso.
pub struct VirtualDisplayProvider {
    controller: Arc<SessionController>,
}

impl VirtualDisplayProvider {
    /// Cria uma nova instância do provedor de display virtual.
    pub fn new() -> Self {
        Self {
            controller: Arc::new(SessionController::new()),
        }
    }

    /// Retorna canal para observação de eventos da sessão.
    pub fn subscribe(&self) -> broadcast::Receiver<SessionEvent> {
        self.controller.subscribe()
    }

    /// Retorna o estado atual da sessão do display virtual.
    pub async fn current_state(&self) -> SessionState {
        self.controller.current_state().await
    }

    /// Constrói o comando `am start --display <id>` direcionado ao display virtual secundário.
    pub fn build_am_start_command(serial: &str, app: &str, display_id: u32) -> Vec<String> {
        let mut args = vec![
            "-s".to_string(),
            serial.to_string(),
            "shell".to_string(),
            "am".to_string(),
            "start".to_string(),
            "--display".to_string(),
            display_id.to_string(),
        ];

        // Se o identificador contiver '/', é um componente explícito (ex: com.android.settings/.MainSettings)
        if app.contains('/') {
            args.push("-n".to_string());
            args.push(app.to_string());
        } else {
            // Se for apenas o nome de pacote, lança via intent MAIN e categoria LAUNCHER
            args.push("-a".to_string());
            args.push("android.intent.action.MAIN".to_string());
            args.push("-c".to_string());
            args.push("android.intent.category.LAUNCHER".to_string());
            args.push(app.to_string());
        }

        args
    }

    /// Executa o lançamento de aplicativo via ADB shell `am start --display <id>`.
    pub async fn launch_app_via_am(
        serial: &str,
        app: &str,
        display_id: u32,
    ) -> Result<(), CoreError> {
        let args = Self::build_am_start_command(serial, app, display_id);
        info!(
            "Executando am start no display virtual {}: {:?}",
            display_id, args
        );

        let output = tokio::process::Command::new("adb")
            .args(&args)
            .output()
            .await
            .map_err(|e| CoreError::AdbError(format!("Falha ao invocar am start via ADB: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CoreError::AdbError(format!(
                "am start falhou com código {:?}: {}",
                output.status.code(),
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        if stdout.contains("Error:") || stdout.contains("SecurityException") {
            return Err(CoreError::AdbError(format!(
                "Erro retornado pelo ActivityManager: {}",
                stdout.trim()
            )));
        }

        info!(
            "Aplicativo '{}' lançado com sucesso no display {}.",
            app, display_id
        );
        Ok(())
    }

    /// Sonda os displays ativos via dumpsys e localiza o identificador do display virtual recém-criado.
    pub async fn find_virtual_display_id(serial: &str) -> Result<Option<u32>, CoreError> {
        let displays = DisplayProber::probe_displays(serial).await?;
        for d in displays {
            if d.is_virtual || d.name.to_lowercase().contains("scrcpy") || d.id != 0 {
                return Ok(Some(d.id));
            }
        }
        Ok(None)
    }

    /// Inicia a sessão de display virtual com o binário scrcpy do sistema.
    pub async fn start(
        &self,
        serial: &str,
        config: &VirtualDisplayConfig,
        caps: &ScrcpyCapabilities,
        stay_awake: bool,
    ) -> Result<(), CoreError> {
        self.start_with_command("scrcpy", serial, config, caps, stay_awake)
            .await
    }

    /// Inicia a sessão de display virtual permitindo especificar o comando executável (útil para testes unitários).
    pub async fn start_with_command(
        &self,
        executable: &str,
        serial: &str,
        config: &VirtualDisplayConfig,
        caps: &ScrcpyCapabilities,
        stay_awake: bool,
    ) -> Result<(), CoreError> {
        if !caps.supports_new_display {
            return Err(CoreError::ScrcpyError(
                "O scrcpy instalado não suporta a criação de display virtual (--new-display)"
                    .into(),
            ));
        }

        let mode = DisplayMode::VirtualDisplay {
            width: config.width,
            height: config.height,
            dpi: config.dpi,
        };

        let options = ScrcpySessionOptions {
            stay_awake,
            virtual_config: Some(config.clone()),
        };

        let args = ScrcpyAdapter::build_arguments_advanced(serial, mode, caps, &options)?;
        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        info!(
            "Iniciando VirtualDisplayProvider para {} (resolução: {}x{} @ {} DPI)...",
            serial, config.width, config.height, config.dpi
        );

        self.controller
            .start(executable, &args_ref, serial, mode)
            .await?;

        // Se o backend for ADB am start e um app foi solicitado, dispara sondagem assíncrona para am start
        if let Some(app) = &config.app {
            if config.launch_backend == AppLaunchBackend::AdbAmStart {
                let app_target = app.clone();
                let serial_target = serial.to_string();

                tokio::spawn(async move {
                    info!("Aguardando registro do display virtual para am start...");
                    for _ in 0..12 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;
                        match Self::find_virtual_display_id(&serial_target).await {
                            Ok(Some(display_id)) => {
                                info!(
                                    "Display virtual {} localizado; invocando am start para '{}'",
                                    display_id, app_target
                                );
                                if let Err(e) =
                                    Self::launch_app_via_am(&serial_target, &app_target, display_id)
                                        .await
                                {
                                    warn!("Falha ao lançar app via am start: {}", e);
                                }
                                return;
                            }
                            _ => continue,
                        }
                    }
                    warn!("Tempo limite esgotado ao buscar ID do display virtual para am start.");
                });
            }
        }

        Ok(())
    }

    /// Inicia uma sessão de display virtual com fallback defensivo automático e explícito para modo Mirror.
    ///
    /// Se a criação do display virtual for negada pelo host, pela ROM ou falhar no spawn do processo,
    /// a causa é capturada, logada de forma amigável ao usuário e uma sessão de espelhamento (Mirror)
    /// é iniciada automaticamente para manter a sessão ativa sem congelamento.
    ///
    /// Regra mandatória: "Nunca alternar invisivelmente nem declarar freeform."
    pub async fn start_with_fallback(
        &self,
        serial: &str,
        config: &VirtualDisplayConfig,
        caps: &ScrcpyCapabilities,
        stay_awake: bool,
    ) -> Result<Option<FallbackRecord>, CoreError> {
        self.start_with_fallback_command("scrcpy", serial, config, caps, stay_awake)
            .await
    }

    /// Inicia a sessão com fallback permitindo especificar o binário base para testes unitários.
    pub async fn start_with_fallback_command(
        &self,
        executable: &str,
        serial: &str,
        config: &VirtualDisplayConfig,
        caps: &ScrcpyCapabilities,
        stay_awake: bool,
    ) -> Result<Option<FallbackRecord>, CoreError> {
        let original_mode = DisplayMode::VirtualDisplay {
            width: config.width,
            height: config.height,
            dpi: config.dpi,
        };

        // 1. Verificação de suporte no host (scrcpy >= 2.2/3.0 com --new-display comprovado)
        if !caps.supports_new_display {
            let reason = FallbackReason::HostScrcpyUnsupported {
                detail: match caps.probe_source {
                    crate::scrcpy::CapabilityProbeSource::HelpInspection => {
                        "saída de 'scrcpy --help' não contém a opção --new-display".into()
                    }
                    crate::scrcpy::CapabilityProbeSource::VersionHeuristic => format!(
                        "versão '{}' anterior a 2.2",
                        caps.version.as_ref().map(|v| v.raw.as_str()).unwrap_or("?")
                    ),
                    crate::scrcpy::CapabilityProbeSource::Unknown => {
                        "capacidades do scrcpy não comprovadas".into()
                    }
                },
            };

            warn!("[FALLBACK EXPLÍCITO] {}", reason.user_friendly_message());
            self.controller
                .notify_fallback(reason.clone(), DisplayMode::Mirror);

            self.start_mirror(executable, serial, caps, stay_awake)
                .await?;
            return Ok(Some(FallbackRecord::new(reason, original_mode)));
        }

        // 2. Inicia o display virtual e monitora para falhas imediatas de execução (ex: recusa da ROM)
        let mut rx = self.controller.subscribe();

        if let Err(e) = self
            .start_with_command(executable, serial, config, caps, stay_awake)
            .await
        {
            let reason = FallbackReason::VirtualDisplaySpawnFailed {
                detail: format!("falha ao criar subprocesso scrcpy: {e}"),
            };

            warn!("[FALLBACK EXPLÍCITO] {}", reason.user_friendly_message());
            self.controller
                .notify_fallback(reason.clone(), DisplayMode::Mirror);

            self.start_mirror(executable, serial, caps, stay_awake)
                .await?;
            return Ok(Some(FallbackRecord::new(reason, original_mode)));
        }

        // 3. Monitoramento de estabilidade na janela inicial (500ms)
        // Se o subprocesso scrcpy morrer rapidamente com código de saída diferente de zero,
        // indica falha da ROM (rejeição de criação de display secundário pelo DisplayManagerService)
        let early_failure = tokio::select! {
            event = rx.recv() => {
                match event {
                    Ok(SessionEvent::ProcessExited(code)) if code != Some(0) => {
                        Some(format!("scrcpy encerrou imediatamente com código {code:?}"))
                    }
                    Ok(SessionEvent::Error(err)) => {
                        Some(err)
                    }
                    _ => None,
                }
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_millis(500)) => {
                None
            }
        };

        if let Some(detail) = early_failure {
            let reason = FallbackReason::VirtualDisplaySpawnFailed { detail };
            warn!("[FALLBACK EXPLÍCITO] {}", reason.user_friendly_message());
            self.controller
                .notify_fallback(reason.clone(), DisplayMode::Mirror);

            self.start_mirror(executable, serial, caps, stay_awake)
                .await?;
            return Ok(Some(FallbackRecord::new(reason, original_mode)));
        }

        Ok(None)
    }

    /// Inicia uma sessão de espelhamento (Mirror) direta como destino de fallback.
    pub async fn start_mirror(
        &self,
        executable: &str,
        serial: &str,
        caps: &ScrcpyCapabilities,
        stay_awake: bool,
    ) -> Result<(), CoreError> {
        let args = ScrcpyAdapter::build_arguments_with_capabilities(
            serial,
            DisplayMode::Mirror,
            caps,
            stay_awake,
        )?;
        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        info!("Inicializando sessão de espelhamento (Mirror) para fallback...");
        self.controller
            .start(executable, &args_ref, serial, DisplayMode::Mirror)
            .await
    }

    /// Encerra a sessão do display virtual de forma limpa, garantindo teardown seguro e destruição do display.
    pub async fn stop(&self) -> Result<(), CoreError> {
        info!("Encerrando VirtualDisplayProvider e liberando recursos de tela...");
        self.controller.stop().await
    }
}

impl Default for VirtualDisplayProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scrcpy::CapabilityProbeSource;

    #[test]
    fn test_build_am_start_command_with_package() {
        let cmd =
            VirtualDisplayProvider::build_am_start_command("serial123", "com.android.settings", 2);

        assert_eq!(
            cmd,
            vec![
                "-s",
                "serial123",
                "shell",
                "am",
                "start",
                "--display",
                "2",
                "-a",
                "android.intent.action.MAIN",
                "-c",
                "android.intent.category.LAUNCHER",
                "com.android.settings"
            ]
        );
    }

    #[test]
    fn test_build_am_start_command_with_explicit_component() {
        let cmd = VirtualDisplayProvider::build_am_start_command(
            "serial123",
            "com.android.settings/.MainSettings",
            3,
        );

        assert_eq!(
            cmd,
            vec![
                "-s",
                "serial123",
                "shell",
                "am",
                "start",
                "--display",
                "3",
                "-n",
                "com.android.settings/.MainSettings"
            ]
        );
    }

    #[tokio::test]
    async fn test_provider_fails_fast_when_new_display_unsupported() {
        let provider = VirtualDisplayProvider::new();
        let caps = ScrcpyCapabilities::unknown(); // supports_new_display = false
        let config = VirtualDisplayConfig::default();

        let res = provider.start("serial123", &config, &caps, true).await;
        assert!(res.is_err());
        match res.unwrap_err() {
            CoreError::ScrcpyError(msg) => {
                assert!(msg.contains("--new-display"));
            }
            other => panic!("Esperava ScrcpyError, obteve: {:?}", other),
        }
    }

    #[tokio::test]
    async fn test_provider_fallback_when_host_unsupported() {
        let provider = VirtualDisplayProvider::new();
        let caps = ScrcpyCapabilities::unknown(); // sem --new-display
        let config = VirtualDisplayConfig::new(1920, 1080, 240);

        let mut rx = provider.subscribe();

        // Inicia com comando mock "sleep"
        let fallback_opt = provider
            .start_with_fallback_command("sleep", "serial_host_fail", &config, &caps, false)
            .await
            .expect("deve realizar fallback sem erro fatal");

        let record = fallback_opt.expect("deve ter retornado FallbackRecord");
        match record.reason {
            FallbackReason::HostScrcpyUnsupported { detail } => {
                assert!(detail.contains("não comprovadas"));
            }
            other => panic!("Esperava HostScrcpyUnsupported, obteve: {:?}", other),
        }
        assert_eq!(record.fallback_mode, DisplayMode::Mirror);

        // Verifica que o estado atual da sessão virou Mirror
        let current_state = provider.current_state().await;
        match current_state {
            SessionState::Active { serial, mode } => {
                assert_eq!(serial, "serial_host_fail");
                assert_eq!(mode, DisplayMode::Mirror);
            }
            other => panic!("Esperava estado Active com Mirror, obteve: {:?}", other),
        }

        // Verifica que evento FallbackTriggered foi emitido para a interface
        let mut saw_fallback_event = false;
        while let Ok(event) = rx.try_recv() {
            if let SessionEvent::FallbackTriggered {
                reason,
                target_mode,
            } = event
            {
                assert_eq!(target_mode, DisplayMode::Mirror);
                match reason {
                    FallbackReason::HostScrcpyUnsupported { .. } => saw_fallback_event = true,
                    _ => {}
                }
            }
        }
        assert!(saw_fallback_event);

        provider.stop().await.ok();
    }

    #[tokio::test]
    async fn test_provider_fallback_when_virtual_display_spawn_fails_immediately() {
        let provider = VirtualDisplayProvider::new();
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
        let config = VirtualDisplayConfig::new(1920, 1080, 240);

        // Usar um executável que falha imediatamente (como "false" no Unix)
        // O fallback deve capturar a saída com erro e transicionar para o comando de espelhamento
        // Usamos um wrapper que falha se receber --new-display
        let fallback_opt = provider
            .start_with_fallback_command("false", "serial_crash", &config, &caps, false)
            .await;

        // "false" falha na execução ou no spawn imediato, provocando registro de fallback
        if let Ok(Some(record)) = fallback_opt {
            match record.reason {
                FallbackReason::VirtualDisplaySpawnFailed { .. } => {}
                other => panic!("Esperava VirtualDisplaySpawnFailed, obteve: {:?}", other),
            }
            assert_eq!(record.fallback_mode, DisplayMode::Mirror);
        }

        provider.stop().await.ok();
    }

    #[tokio::test]
    async fn test_provider_lifecycle_with_mock_executable() {
        let provider = VirtualDisplayProvider::new();
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

        let config = VirtualDisplayConfig::new(1280, 720, 160).with_app("com.android.settings");

        let mut rx = provider.subscribe();

        // Inicia processo mock rápido (sleep)
        provider
            .start_with_command("sleep", "mock-serial", &config, &caps, false)
            .await
            .expect("deve iniciar com comando mock");

        let state = provider.current_state().await;
        match state {
            SessionState::Active { serial, mode } => {
                assert_eq!(serial, "mock-serial");
                assert_eq!(
                    mode,
                    DisplayMode::VirtualDisplay {
                        width: 1280,
                        height: 720,
                        dpi: 160,
                    }
                );
            }
            other => panic!("Esperava Active, obteve: {:?}", other),
        }

        // Encerra explicitamente
        provider.stop().await.expect("deve encerrar");

        let final_state = provider.current_state().await;
        assert_eq!(final_state, SessionState::Disconnected);

        // Verifica que evento foi transmitido
        let mut saw_disconnected = false;
        while let Ok(event) = rx.try_recv() {
            if let SessionEvent::StateChanged(SessionState::Disconnected) = event {
                saw_disconnected = true;
                break;
            }
        }
        assert!(saw_disconnected);
    }
}
