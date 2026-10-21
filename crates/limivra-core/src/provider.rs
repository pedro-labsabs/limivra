use crate::launcher::{
    AllowedApp, LaunchContext, LaunchError, LaunchOutcome, LauncherReply, LauncherRequest,
    LauncherRunner, ObservedDisplay, PackageName,
};
use crate::{
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
use tokio::sync::{watch, Mutex};
use tracing::{info, warn};

/// Provedor e gerenciador de ciclo de vida de VirtualDisplay secundário para Android.
///
/// Não requer privilégios de root nem APK proprietário pré-instalado.
/// Utiliza o subsistema de display virtual do scrcpy (executando sob UID 2000 shell)
/// com suporte a inicialização direcionada de aplicações Android e fallback gracioso.
pub struct VirtualDisplayProvider {
    controller: Arc<SessionController>,
    operation: Mutex<()>,
    launch_context: Mutex<Option<LaunchContext>>,
    launch_generation: watch::Sender<u64>,
}

impl VirtualDisplayProvider {
    /// Start an app-neutral virtual session and bind only its uniquely new display.
    /// Call launch_allowed_app separately after an explicit host selection.
    pub async fn start_launcher_session<R: LauncherRunner>(
        &self,
        executable: &str,
        serial: &str,
        options: &ScrcpySessionOptions,
        caps: &ScrcpyCapabilities,
        runner: &R,
    ) -> Result<(), LaunchError> {
        let _operation = self.operation.lock().await;
        if matches!(
            self.current_state().await,
            SessionState::Active { .. } | SessionState::Connecting
        ) {
            return Err(LaunchError::NotActive);
        }
        let config = options
            .virtual_config
            .as_ref()
            .ok_or(LaunchError::DisplayDenied)?;
        if config.app.is_some() || config.keep_content_on_destroy {
            return Err(LaunchError::NotAllowed);
        }
        if !caps.supports_new_display {
            return Err(LaunchError::DisplayDenied);
        }
        self.launch_generation
            .send_modify(|n| *n = n.wrapping_add(1));
        *self.launch_context.lock().await = None;
        let mut generation = self.launch_generation.subscribe();
        let epoch = *generation.borrow();
        let before = observed_displays(runner, serial).await?;
        let mode = DisplayMode::VirtualDisplay {
            width: config.width,
            height: config.height,
            dpi: config.dpi,
        };
        let args = ScrcpyAdapter::build_arguments_advanced(serial, mode, caps, options)
            .map_err(|_| LaunchError::CommandFailed)?;
        let args_ref: Vec<_> = args.iter().map(String::as_str).collect();
        // stop may have invalidated while the preflight was running.
        if *generation.borrow() != epoch {
            return Err(LaunchError::StaleContext);
        }
        self.controller
            .start(executable, &args_ref, serial, mode)
            .await
            .map_err(|_| LaunchError::CommandFailed)?;
        let mut events = self.subscribe();
        let bind = async {
            loop {
                let after = observed_displays(runner, serial).await?;
                let candidates: Vec<_> = after
                    .iter()
                    .filter(|d| {
                        d.is_virtual && d.id != 0 && !before.iter().any(|old| old.id == d.id)
                    })
                    .collect();
                match candidates.as_slice() {
                    [display] => return Ok(display.id),
                    [] => tokio::time::sleep(std::time::Duration::from_millis(250)).await,
                    _ => return Err(LaunchError::DisplayDenied),
                }
            }
        };
        let result = tokio::select! {
            biased;
            _ = generation.changed() => Err(LaunchError::StaleContext),
            _ = events.recv() => Err(LaunchError::StaleContext),
            result = tokio::time::timeout(std::time::Duration::from_secs(5), bind) => result.unwrap_or(Err(LaunchError::Timeout)),
        };
        let same_generation = *generation.borrow() == epoch;
        match result {
            Ok(display_id)
                if same_generation
                    && self.current_state().await
                        == (SessionState::Active {
                            serial: serial.into(),
                            mode,
                        }) =>
            {
                *self.launch_context.lock().await = Some(LaunchContext {
                    generation: epoch,
                    target: serial.into(),
                    display_id,
                });
                Ok(())
            }
            failure => {
                self.launch_generation
                    .send_modify(|n| *n = n.wrapping_add(1));
                let _ = self.controller.stop().await;
                Err(failure.err().unwrap_or(LaunchError::StaleContext))
            }
        }
    }

    /// Accepted means ActivityManager accepted the command, not proof of focus,
    /// task placement or windowing support. The UI must report those separately.
    pub async fn launch_allowed_app<R: LauncherRunner>(
        &self,
        app: AllowedApp,
        runner: &R,
    ) -> Result<LaunchOutcome, LaunchError> {
        let _operation = self.operation.lock().await;
        let context = self
            .launch_context
            .lock()
            .await
            .clone()
            .ok_or(LaunchError::NotActive)?;
        let mut generation = self.launch_generation.subscribe();
        let mut events = self.subscribe();
        self.check_launch_context(&context).await?;
        let launch = async {
            let mut component = None;
            for package in app.packages() {
                match runner
                    .run(
                        &context.target,
                        LauncherRequest::Resolve {
                            package: PackageName::parse(package)?,
                        },
                    )
                    .await?
                {
                    LauncherReply::Resolved(found) if found.package() == *package => {
                        component = Some(found);
                        break;
                    }
                    LauncherReply::NotInstalled => continue,
                    _ => return Err(LaunchError::UnexpectedReply),
                }
            }
            let component = component.ok_or(LaunchError::ActivityNotFound)?;
            self.check_launch_context(&context).await?;
            let displays = observed_displays(runner, &context.target).await?;
            if !displays
                .iter()
                .any(|d| d.id == context.display_id && d.is_virtual)
            {
                *self.launch_context.lock().await = None;
                return Err(LaunchError::DisplayDenied);
            }
            self.check_launch_context(&context).await?;
            match runner
                .run(
                    &context.target,
                    LauncherRequest::Start {
                        component,
                        display_id: context.display_id,
                    },
                )
                .await?
            {
                LauncherReply::Accepted => {
                    self.check_launch_context(&context).await?;
                    Ok(LaunchOutcome::Accepted {
                        display_id: context.display_id,
                    })
                }
                _ => Err(LaunchError::UnexpectedReply),
            }
        };
        tokio::select! {
            biased;
            _ = generation.changed() => Err(LaunchError::StaleContext),
            _ = events.recv() => Err(LaunchError::StaleContext),
            result = launch => result,
        }
    }

    async fn check_launch_context(&self, context: &LaunchContext) -> Result<(), LaunchError> {
        if *self.launch_generation.borrow() != context.generation {
            return Err(LaunchError::StaleContext);
        }
        match self.current_state().await {
            SessionState::Active {
                serial,
                mode: DisplayMode::VirtualDisplay { .. },
            } if serial == context.target => Ok(()),
            _ => Err(LaunchError::StaleContext),
        }
    }

    /// Cria uma nova instância do provedor de display virtual.
    pub fn new() -> Self {
        Self {
            controller: Arc::new(SessionController::new()),
            operation: Mutex::new(()),
            launch_context: Mutex::new(None),
            launch_generation: watch::channel(0).0,
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
        if let Some(package) = &config.app {
            let app = AllowedApp::from_package(package)
                .ok_or_else(|| CoreError::AdbError(LaunchError::NotAllowed.to_string()))?;
            if config.launch_backend == AppLaunchBackend::AdbAmStart {
                let runner = crate::launcher::adb_runner::AdbLauncherRunner::system(serial)
                    .map_err(|e| CoreError::AdbError(e.to_string()))?;
                let mut neutral = config.clone();
                neutral.app = None;
                let options = ScrcpySessionOptions {
                    stay_awake,
                    virtual_config: Some(neutral),
                };
                self.start_launcher_session(executable, serial, &options, caps, &runner)
                    .await
                    .map_err(|e| CoreError::AdbError(e.to_string()))?;
                self.launch_allowed_app(app, &runner)
                    .await
                    .map_err(|e| CoreError::AdbError(e.to_string()))?;
                return Ok(());
            }
        }
        let _operation = self.operation.lock().await;
        if matches!(
            self.current_state().await,
            SessionState::Active { .. } | SessionState::Connecting
        ) {
            return Err(CoreError::AdbError("Session already active".into()));
        }
        self.launch_generation
            .send_modify(|n| *n = n.wrapping_add(1));
        *self.launch_context.lock().await = None;
        if !caps.supports_new_display {
            return Err(CoreError::ScrcpyError(
                "O scrcpy instalado não suporta --new-display".into(),
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
        let args_ref: Vec<_> = args.iter().map(String::as_str).collect();
        self.controller
            .start(executable, &args_ref, serial, mode)
            .await
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
            let reason = FallbackReason::ScrcpyUnsupported {
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
            let reason = FallbackReason::ProcessError {
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
            let reason = FallbackReason::ProcessError { detail };
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
        self.launch_generation
            .send_modify(|n| *n = n.wrapping_add(1));
        let _operation = self.operation.lock().await;
        *self.launch_context.lock().await = None;
        self.controller.stop().await?;
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
        self.launch_generation
            .send_modify(|n| *n = n.wrapping_add(1));
        let _operation = self.operation.lock().await;
        *self.launch_context.lock().await = None;
        info!("Encerrando VirtualDisplayProvider e liberando recursos de tela...");
        self.controller.stop().await
    }
}

impl Default for VirtualDisplayProvider {
    fn default() -> Self {
        Self::new()
    }
}

async fn observed_displays<R: LauncherRunner>(
    runner: &R,
    target: &str,
) -> Result<Vec<ObservedDisplay>, LaunchError> {
    match runner.run(target, LauncherRequest::Displays).await? {
        LauncherReply::Displays(displays) if !displays.is_empty() => Ok(displays),
        _ => Err(LaunchError::UnexpectedReply),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scrcpy::CapabilityProbeSource;

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
            other => panic!("Esperava ScrcpyError, obteve: {other:?}"),
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
            FallbackReason::ScrcpyUnsupported { detail } => {
                assert!(detail.contains("não comprovadas"));
            }
            other => panic!("Esperava ScrcpyUnsupported, obteve: {other:?}"),
        }
        assert_eq!(record.fallback_mode, DisplayMode::Mirror);

        // Verifica que o estado atual da sessão virou Mirror
        let current_state = provider.current_state().await;
        match current_state {
            SessionState::Active { serial, mode } => {
                assert_eq!(serial, "serial_host_fail");
                assert_eq!(mode, DisplayMode::Mirror);
            }
            other => panic!("Esperava estado Active com Mirror, obteve: {other:?}"),
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
                if let FallbackReason::ScrcpyUnsupported { .. } = reason {
                    saw_fallback_event = true;
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

        let fallback_opt = provider
            .start_with_fallback_command("false", "serial_crash", &config, &caps, false)
            .await;

        if let Ok(Some(record)) = fallback_opt {
            match record.reason {
                FallbackReason::ProcessError { .. } => {}
                other => panic!("Esperava ProcessError, obteve: {other:?}"),
            }
            assert_eq!(record.fallback_mode, DisplayMode::Mirror);
        }

        provider.stop().await.ok();
    }

    #[test]
    fn test_fallback_explicit_request_and_rom_denied() {
        let r_rom = FallbackReason::RomDenied {
            detail: "Xiaomi HyperOS sem autorização de injeção".into(),
        };
        assert!(r_rom.user_friendly_message().contains("A ROM ou opções"));

        let r_exp = FallbackReason::ExplicitRequest {
            detail: "Configuração do usuário solicitou espelhamento".into(),
        };
        assert!(r_exp
            .user_friendly_message()
            .contains("solicitado explicitamente"));
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
            other => panic!("Esperava Active, obteve: {other:?}"),
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
