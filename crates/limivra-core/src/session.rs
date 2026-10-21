use crate::{CoreError, DisplayMode, SessionState};
use std::process::ExitStatus;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::{broadcast, Mutex};
use tracing::{info, warn};

/// Eventos emitidos durante as transições de ciclo de vida de uma sessão.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionEvent {
    StateChanged(SessionState),
    ProcessExited(Option<i32>),
    Error(String),
}

/// Gerenciador de ciclo de vida de sessão e controle de processo filho.
pub struct SessionController {
    state: Arc<Mutex<SessionState>>,
    child_process: Arc<Mutex<Option<Child>>>,
    event_sender: broadcast::Sender<SessionEvent>,
}

impl SessionController {
    /// Cria uma nova instância do controlador de sessão em estado desconectado.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(32);
        Self {
            state: Arc::new(Mutex::new(SessionState::Disconnected)),
            child_process: Arc::new(Mutex::new(None)),
            event_sender: tx,
        }
    }

    /// Retorna um receptor para observar eventos e mudanças de estado da sessão.
    pub fn subscribe(&self) -> broadcast::Receiver<SessionEvent> {
        self.event_sender.subscribe()
    }

    /// Retorna o estado atual da sessão.
    pub async fn current_state(&self) -> SessionState {
        self.state.lock().await.clone()
    }

    /// Inicia uma sessão gerenciada lançando um subprocesso (ex: scrcpy ou comando mock em testes).
    pub async fn start(
        &self,
        command: &str,
        args: &[&str],
        serial: &str,
        mode: DisplayMode,
    ) -> Result<(), CoreError> {
        let mut state_guard = self.state.lock().await;

        if let SessionState::Active { .. } | SessionState::Connecting = *state_guard {
            warn!("Tentativa de iniciar uma sessão que já está ativa ou conectando.");
            return Ok(());
        }

        *state_guard = SessionState::Connecting;
        let _ = self
            .event_sender
            .send(SessionEvent::StateChanged(SessionState::Connecting));

        info!("Lançando processo de transporte: {} {:?}", command, args);

        let child = match tokio::process::Command::new(command).args(args).spawn() {
            Ok(c) => c,
            Err(e) => {
                let err_msg = format!("Falha ao iniciar processo '{command}': {e}");
                *state_guard = SessionState::Disconnected;
                let _ = self.event_sender.send(SessionEvent::Error(err_msg.clone()));
                return Err(CoreError::DependencyMissing(err_msg));
            }
        };

        let new_state = SessionState::Active {
            serial: serial.to_string(),
            mode,
        };
        *state_guard = new_state.clone();
        let _ = self
            .event_sender
            .send(SessionEvent::StateChanged(new_state));

        // Armazena o handle do processo filho
        let mut child_guard = self.child_process.lock().await;
        *child_guard = Some(child);
        drop(child_guard);
        drop(state_guard);

        // Monitora o término assíncrono do processo em background
        let state_clone = Arc::clone(&self.state);
        let child_clone = Arc::clone(&self.child_process);
        let event_tx = self.event_sender.clone();

        tokio::spawn(async move {
            let mut proc = {
                let mut guard = child_clone.lock().await;
                guard.take()
            };

            if let Some(ref mut child) = proc {
                let status: Result<ExitStatus, std::io::Error> = child.wait().await;
                let code = status.ok().and_then(|s| s.code());
                info!("Processo filho encerrou com código de saída: {:?}", code);

                let mut state = state_clone.lock().await;
                *state = SessionState::Terminated;
                let _ = event_tx.send(SessionEvent::ProcessExited(code));
                let _ = event_tx.send(SessionEvent::StateChanged(SessionState::Terminated));
            }
        });

        Ok(())
    }

    /// Encerra a sessão ativamente de forma segura, afetando apenas o processo filho de propriedade desta sessão.
    pub async fn stop(&self) -> Result<(), CoreError> {
        let mut child_guard = self.child_process.lock().await;
        if let Some(mut child) = child_guard.take() {
            info!("Enviando sinal de encerramento para o processo da sessão...");
            if let Err(e) = child.kill().await {
                warn!("Aviso ao encerrar processo: {}", e);
            }
        }

        let mut state_guard = self.state.lock().await;
        *state_guard = SessionState::Disconnected;
        let _ = self
            .event_sender
            .send(SessionEvent::StateChanged(SessionState::Disconnected));

        Ok(())
    }
}

impl Default for SessionController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn test_session_lifecycle_with_mock_process() {
        let controller = SessionController::new();
        assert_eq!(controller.current_state().await, SessionState::Disconnected);

        // Inicia processo mock rápido (sleep 0.1)
        controller
            .start("sleep", &["0.1"], "mock-serial", DisplayMode::Mirror)
            .await
            .expect("deve iniciar");

        match controller.current_state().await {
            SessionState::Active { serial, mode } => {
                assert_eq!(serial, "mock-serial");
                assert_eq!(mode, DisplayMode::Mirror);
            }
            other => panic!("Esperava estado Active, obteve: {other:?}"),
        }

        // Aguarda término natural do processo filho
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert_eq!(controller.current_state().await, SessionState::Terminated);
    }

    #[tokio::test]
    async fn test_session_explicit_stop() {
        let controller = SessionController::new();

        // Inicia processo que duraria 10 segundos
        controller
            .start("sleep", &["10"], "mock-serial", DisplayMode::Mirror)
            .await
            .expect("deve iniciar");

        // Encerramento voluntário imediato
        controller.stop().await.expect("deve parar com sucesso");
        assert_eq!(controller.current_state().await, SessionState::Disconnected);
    }

    #[tokio::test]
    async fn test_session_missing_executable_error() {
        let controller = SessionController::new();
        let res = controller
            .start(
                "binario_inexistente_12345",
                &[],
                "mock-serial",
                DisplayMode::Mirror,
            )
            .await;

        assert!(res.is_err());
        assert_eq!(controller.current_state().await, SessionState::Disconnected);
    }
}
