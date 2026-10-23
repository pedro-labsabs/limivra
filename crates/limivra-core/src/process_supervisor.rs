use std::ffi::OsStr;
use std::io;
use std::process::{Child, Command, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Resultado de esperar um filho até sair ou alcançar o limite de tempo.
#[derive(Debug)]
pub enum WaitOutcome {
    /// O filho encerrou antes do limite.
    Exited(ExitStatus),
    /// O limite foi alcançado; o filho foi encerrado e recolhido.
    TimedOut(ExitStatus),
}

/// Administra exclusivamente o processo filho criado por esta instância.
///
/// O supervisor não assume propriedade de outros processos do host.
pub struct ProcessSupervisor {
    child: Child,
    exit_status: Option<ExitStatus>,
}

impl ProcessSupervisor {
    /// Inicia um executável local e mantém seu handle para espera e teardown.
    pub fn spawn<I, S>(program: impl AsRef<OsStr>, args: I) -> io::Result<Self>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let child = Command::new(program).args(args).spawn()?;
        Ok(Self {
            child,
            exit_status: None,
        })
    }

    /// Retorna o status quando o filho terminou e foi recolhido.
    pub fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
        if let Some(status) = self.exit_status.as_ref() {
            return Ok(Some(*status));
        }

        let status = self.child.try_wait()?;
        if let Some(status) = status.as_ref() {
            self.exit_status = Some(*status);
        }
        Ok(status)
    }

    /// Espera até o limite; ao vencê-lo, encerra e recolhe o filho.
    pub fn wait_timeout(&mut self, timeout: Duration) -> io::Result<WaitOutcome> {
        let started_at = Instant::now();

        loop {
            if let Some(status) = self.try_wait()? {
                return Ok(WaitOutcome::Exited(status));
            }

            let elapsed = started_at.elapsed();
            if elapsed >= timeout {
                return self.stop().map(WaitOutcome::TimedOut);
            }

            thread::sleep((timeout - elapsed).min(POLL_INTERVAL));
        }
    }

    /// Encerra e recolhe apenas o filho administrado; chamadas repetidas são seguras.
    pub fn stop(&mut self) -> io::Result<ExitStatus> {
        if let Some(status) = self.try_wait()? {
            return Ok(status);
        }

        if let Err(kill_error) = self.child.kill() {
            if let Some(status) = self.try_wait()? {
                return Ok(status);
            }
            return Err(kill_error);
        }

        let status = self.child.wait()?;
        self.exit_status = Some(status);
        Ok(status)
    }
}

impl Drop for ProcessSupervisor {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(Some(_))) {
            return;
        }

        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
