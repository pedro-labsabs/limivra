# Issue #29 — Evidência de launcher e apps permitidos

**Data:** 2026-10-21. **Base:** `f6bb7ad2239b8cf15574fe67a393bb95fa07b8ba`. **Branch:** `agent/29-launcher-app-controls`.

**Resultado de desenvolvimento:** seleção/lançamento no core implementados; dois apps observados no display virtual com confirmação humana. Auditoria independente pelo limivra-qa permanece pendente. `Accepted` é confirmação do comando; imagem, destino e foco têm evidências próprias abaixo.

## Ambiente e escopo

- Linux Mint 22.3, kernel 7.0.0-30-generic, X11. Wayland não testado nesta slice.
- Rust 1.88.0 (6b00bc388), Cargo 1.88.0 (873a06493), ADB 1.0.41, scrcpy 5.0.1.
- Xiaomi Redmi Note 13 4G / HyperOS Android 15 autorizado por USB, seleção explícita do aparelho em memória. Observador confirmou disponibilidade e apps sem dados pessoais antes do experimento. Serial e caminhos pessoais omitidos.
- Sem APK/companion instalado, root, alteração de settings persistentes, captura de tela, logcat ou inventário global de apps. O servidor transitório padrão do scrcpy é o backend existente, não companion instalado.
- Prova via APIs reais do core; wrapper local acrescentou `--no-control --no-audio --no-clipboard-autosync`. Não houve prova de input nesta slice. Ambos os apps foram lançados sequencialmente, sem pedido de freeform.
- #22 e #25 permaneciam OPEN na consulta; contratos presentes na base não significam aprovação de integração. ADRs 0001–0003 continuam PROPOSED. Nenhum merge/fechamento de issue faz parte desta entrega.

## Implementação e decisões de interface

- `AllowedApp::{Settings, Calculator}` restringe pacotes: Settings somente `com.android.settings`; Calculator tenta MIUI, depois Google somente se o backend declarar ausência. Erros de segurança abortam.
- Package/activity têm gramática ASCII, limites 255/512 bytes e campos privados. Componentes retornados devem corresponder ao pacote consultado. `$` de classes internas é protegido com aspas simples na passagem por `adb shell`; não se confia apenas em argv separados.
- Runner fixa executável absoluto e alvo; comandos não recebem shell do host. Timeout de 5 s por subprocesso e teto combinado de 16 KiB de stdout/stderr. Cancelamento mata/recolhe somente seu cliente; respostas desconhecidas falham fechadas. Não abre nem anuncia portas no host; usa o servidor ADB já disponível.
- Resolução restrita usa `cmd package resolve-activity --brief -a android.intent.action.MAIN -c android.intent.category.LAUNCHER -p <pacote>`. Cabeçalho conhecido de cinco campos é validado, sem aceitar texto arbitrário.
- Displays são consultados com `cmd display get-displays --ids-only --type VIRTUAL` antes de `cmd display get-displays --ids-only`. Apenas IDs numéricos únicos são aceitos. A segunda consulta exclui IDs desaparecidos; ID recém-aparecido permanece sem tipo até a próxima observação. Não se coletam nomes.
- Provider captura IDs anteriores ao start, exige um único novo virtual não zero, com prazo de 5 s para a descoberta após start, e vincula geração/alvo/display. Revalida antes de lançar; desconexão, restart ou desaparecimento observado invalidam a vinculação. Respostas de operação cancelada não são reaplicadas.
- APIs estáticas permissivas `build_am_start_command`, `launch_app_via_am` e `find_virtual_display_id` foram removidas. Não havia callers de produto no repositório além do próprio provider. Migração externa: usar `start_launcher_session` e `launch_allowed_app`; o caminho legado de config valida a allowlist e, no backend ADB, usa o fluxo tipado.
- `HostControls::request_app` requer Active/VirtualDisplay. `ReturnToSession` é intenção local, sem HOME/BACK, clique ou foco Android sintetizado. Adaptador gráfico para executá-la fica fora desta slice, conforme o plano.
- Correção necessária de ciclo de vida em `session.rs`: o monitor anterior tomava o handle do filho e impedia `stop()` de matá-lo. Agora o handle permanece compartilhado e o monitor usa `try_wait` a cada 20 ms, com ordem state→child e geração para impedir atualização atrasada após restart. Teste com PID real reproduziu e comprovou o reparo.

## TDD e verificações automatizadas

Comandos executados na worktree isolada, com lockfile e cache offline:

| Comando / etapa | Resultado observado |
|---|---|
| `cargo test --workspace --locked --offline` antes do código | exit 0; 80 testes |
| `cargo test -p limivra-core --test launcher --locked --offline`, interfaces mínimas | RED: 3 falhas comportamentais; depois GREEN 3/3 |
| `cargo test -p limivra-core --test launcher_runner --locked --offline`, runner stub | RED: 0/8; depois GREEN para argv, erros, resolução, limites e cancelamento |
| Host/provider com métodos stub | RED: intenção permitida fora de Active e provider sem lançamento; GREEN após implementação |
| `provider_transport_stop_after_monitor_start_reaps_actual_child` | RED: processo sobrevive a stop; GREEN: PID recolhido |
| `observed_display_removal_invalidates_binding_even_if_id_reappears` | RED: Accepted no ID reaparecido; GREEN: NotActive após perda da vinculação |
| `android_brief_resolution_header_is_strictly_parsed` | RED: UnexpectedReply para cabeçalho real; GREEN após parsing estrito |
| Atividade relativa ou qualificada | RED: componente com atividade simples `com.a/Main` aceito; GREEN após exigir forma relativa ou qualificada |
| Consulta `--ids-only` e `concurrent_display_creation_does_not_bind_an_untyped_id` | RED: CommandFailed/UnexpectedReply com protocolo anterior; GREEN com protocolo numérico e ordem defensiva |
| Suítes dedicadas atuais | 12 launcher, 13 launcher_runner e 9 host_controls; cobertura adicional de restart, saída de transporte, cancelamento de resolução/start, orçamento combinado e respostas desconhecidas |

Os executáveis fake inicialmente produziram `ETXTBSY` intermitente (errno 26) em testes concorrentes. Diagnóstico isolou a corrida entre escrita de fixtures e fork; os testes com executáveis temporários foram serializados. Não foi adicionado retry ao código de produto. Dois testes antigos de builder foram substituídos pela prova dos argumentos reais no runner; por isso o total resulta em 105, não apenas 80 + arquivos novos.

Gates finais sobre o código de entrega (todos realmente executados):

| Comando | Resultado |
|---|---|
| `cargo test --workspace --locked --offline` | exit 0; 105 testes, zero falhas |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | exit 0; zero warnings |
| `cargo fmt --all -- --check` | exit 0 |
| `git diff --check origin/main` (candidato antes do commit) | exit 0 |

A verificação cumulativa `git diff --check origin/main...HEAD` foi executada após o commit de implementação `9a6bb12`: **exit 0**, sem erros. A mesma verificação será repetida no HEAD de publicação após este registro documental.

## Prova física observável

Sequência de pedidos (serial substituído por `<alvo>`):

```text
adb -s <alvo> get-state
adb -s <alvo> shell cmd display get-displays --ids-only --type VIRTUAL
adb -s <alvo> shell cmd display get-displays --ids-only
scrcpy --no-control --no-audio --no-clipboard-autosync -s <alvo> --window-title "Limivra Desktop (<alvo>)" --new-display=1280x720/160
adb -s <alvo> shell cmd package resolve-activity --brief -a android.intent.action.MAIN -c android.intent.category.LAUNCHER -p com.android.settings
adb -s <alvo> shell am start --display 10 -n com.android.settings/.MainSettings
adb -s <alvo> shell cmd package resolve-activity --brief -a android.intent.action.MAIN -c android.intent.category.LAUNCHER -p com.miui.calculator
adb -s <alvo> shell am start --display 10 -n com.miui.calculator/.cal.CalculatorActivity
adb -s <alvo> shell dumpsys activity activities
adb -s <alvo> shell dumpsys window displays
```

As duas últimas consultas foram processadas em memória; nunca se gravou o dump bruto. `<alvo>` e display 10 são ilustrativos da reprodução: selecionar o aparelho autorizado e usar o ID vinculado pelo provider, nunca fixar o ID histórico.

| Prova | Observado |
|---|---|
| Configurações | 18:53:05 UTC: Accepted no display 10; tarefa 12266 fullscreen, visibleRequested=true, bounds [280,0,1000,720]; WM mFocusedApp aponta para Settings nesse display |
| Calculadora MIUI | 18:53:53 UTC: Accepted no display 10; tarefa 12267 fullscreen, visibleRequested=true, bounds [0,0,720,1280]; Settings passa a visibleRequested=false |
| Observador humano | Respostas nesta sessão: “Sim, Configurações está visível” e “Confirmo Calculadora e alternância manual de foco” |
| Retorno local | 18:53:57 UTC: core retorna Ok(ReturnToSession); não afirma execução automática de foco pela UI |
| Stop | 18:55:35 UTC: stop retorna Ok, estado Disconnected; harness exit 0 |
| Convergência | Consulta posterior de IDs: somente 0 e nenhum probe/scrcpy no host; às 18:56:18 UTC AM/WM somente display 0, tarefas 12266/12267 ausentes e zero processos de servidor scrcpy no Android |

O cache/estado Android ainda mostrava display 10 e tarefas em 18:55:46 UTC, depois de stop. A remoção foi confirmada posteriormente, em até 44 s da chamada de stop; não se afirma destruição instantânea. Nenhum pacote foi force-stopped nem tarefa preexistente removida manualmente.

Tentativas anteriores também foram registradas: o primeiro start falhou fechado em UnexpectedReply antes de lançar o transporte por incompatibilidade do parser textual de displays; a segunda tentativa encontrou a corrida das consultas. Testes de regressão foram escritos antes das correções. Uma sessão diagnóstica seguinte abriu display 9 e encerrou sem apps. O backend convergiu após teardown; display 9 ainda aparecia no snapshot anterior à sessão 10 e foi excluído pela diferença de IDs. O wrapper Python da primeira tentativa recebeu BrokenPipe ao escrever stop após o harness já ter encerrado (harness exit 0); isso não foi registrado como sucesso físico. A prova positiva acima usa o runner com a correção da corrida.

### Registros sanitizados reais da prova positiva e cleanup

Somente componentes permitidos, IDs de display/tarefa, modo, bounds, booleanos, enums e timestamps. Resumos aninhados do dumpsys podem repetir IDs; a conclusão usa as tarefas raiz visíveis 12266/12267, sem confundir resumos com janelas adicionais. Campos de foco fora da allowlist são representados por `none-or-other-redacted`.

```jsonl
{"allowed_tasks": [], "display_ids": [0, 9], "exit": 0, "kind": "activity", "stage": "before", "utc": "2026-10-21T18:52:06.200960+00:00"}
{"allowed_focus": [{"component": "none-or-other-redacted", "display_id": 0, "field": "mCurrentFocus"}, {"component": "none-or-other-redacted", "display_id": 0, "field": "mFocusedApp"}, {"component": "none-or-other-redacted", "display_id": 9, "field": "mCurrentFocus"}, {"component": "none-or-other-redacted", "display_id": 9, "field": "mFocusedApp"}], "display_ids": [0, 9], "exit": 0, "kind": "window", "stage": "after_stop", "utc": "2026-10-21T18:52:06.203623+00:00"}
{"allowed_focus": [{"component": "none-or-other-redacted", "display_id": 0, "field": "mCurrentFocus"}, {"component": "none-or-other-redacted", "display_id": 0, "field": "mFocusedApp"}, {"component": "none-or-other-redacted", "display_id": 9, "field": "mCurrentFocus"}, {"component": "none-or-other-redacted", "display_id": 9, "field": "mFocusedApp"}], "display_ids": [0, 9], "exit": 0, "kind": "window", "stage": "before", "utc": "2026-10-21T18:52:06.263955+00:00"}
{"kind": "displays", "result": "Ok(Displays([ObservedDisplay { id: 0, is_virtual: false }, ObservedDisplay { id: 9, is_virtual: true }]))", "stage": "runner_observation", "utc": "2026-10-21T18:52:06.383969+00:00"}
{"kind": "displays", "result": "Ok(Displays([ObservedDisplay { id: 0, is_virtual: false }, ObservedDisplay { id: 9, is_virtual: true }]))", "stage": "runner_observation", "utc": "2026-10-21T18:52:06.498384+00:00"}
{"kind": "displays", "result": "Ok(Displays([ObservedDisplay { id: 0, is_virtual: false }, ObservedDisplay { id: 9, is_virtual: true }]))", "stage": "runner_observation", "utc": "2026-10-21T18:52:06.881955+00:00"}
{"kind": "displays", "result": "Ok(Displays([ObservedDisplay { id: 0, is_virtual: false }, ObservedDisplay { id: 9, is_virtual: true }, ObservedDisplay { id: 10, is_virtual: true }]))", "stage": "runner_observation", "utc": "2026-10-21T18:52:07.281113+00:00"}
{"result": "Ok(())", "stage": "session_start", "utc": "2026-10-21T18:52:07.281224+00:00"}
{"kind": "resolve", "result": "Ok(Resolved(ComponentName(\"com.android.settings/.MainSettings\")))", "stage": "runner_observation", "utc": "2026-10-21T18:53:05.322857+00:00"}
{"kind": "displays", "result": "Ok(Displays([ObservedDisplay { id: 0, is_virtual: false }, ObservedDisplay { id: 10, is_virtual: true }]))", "stage": "runner_observation", "utc": "2026-10-21T18:53:05.463643+00:00"}
{"kind": "start", "result": "Ok(Accepted)", "stage": "runner_observation", "utc": "2026-10-21T18:53:05.614290+00:00"}
{"app": "Settings", "result": "Ok(Accepted { display_id: 10 })", "stage": "app_launch", "utc": "2026-10-21T18:53:05.614377+00:00"}
{"allowed_tasks": [{"components": ["com.android.settings/.MainSettings"], "configurations": [], "display_id": 0, "parent_task_id": 12253, "task_id": 12265}, {"components": ["com.android.settings/.MainSettings", "com.android.settings/.MiuiSettings"], "configurations": [{"bounds": [280, 0, 1000, 720], "mode": "fullscreen", "source": "mOverrideConfig"}], "display_id": 10, "parent_task_id": null, "task_id": 12266, "task_mode": "fullscreen", "visible": true, "visibleRequested": true}, {"components": ["com.android.settings/.MainSettings"], "configurations": [], "display_id": 10, "parent_task_id": 12264, "task_id": 12265, "task_mode": "fullscreen", "visible": false, "visibleRequested": false}], "display_ids": [0, 10], "exit": 0, "kind": "activity", "stage": "after_settings", "utc": "2026-10-21T18:53:06.706853+00:00"}
{"allowed_focus": [{"component": "none-or-other-redacted", "display_id": 0, "field": "mCurrentFocus"}, {"component": "none-or-other-redacted", "display_id": 0, "field": "mFocusedApp"}, {"component": "none-or-other-redacted", "display_id": 10, "field": "mCurrentFocus"}, {"component": "com.android.settings/.MainSettings", "display_id": 10, "field": "mFocusedApp"}], "display_ids": [0, 10], "exit": 0, "kind": "window", "stage": "after_settings", "utc": "2026-10-21T18:53:06.759758+00:00"}
{"kind": "resolve", "result": "Ok(Resolved(ComponentName(\"com.miui.calculator/.cal.CalculatorActivity\")))", "stage": "runner_observation", "utc": "2026-10-21T18:53:53.289013+00:00"}
{"kind": "displays", "result": "Ok(Displays([ObservedDisplay { id: 0, is_virtual: false }, ObservedDisplay { id: 10, is_virtual: true }]))", "stage": "runner_observation", "utc": "2026-10-21T18:53:53.401730+00:00"}
{"kind": "start", "result": "Ok(Accepted)", "stage": "runner_observation", "utc": "2026-10-21T18:53:53.589750+00:00"}
{"app": "Calculator", "result": "Ok(Accepted { display_id: 10 })", "stage": "app_launch", "utc": "2026-10-21T18:53:53.589849+00:00"}
{"allowed_tasks": [{"components": ["com.miui.calculator/.cal.CalculatorActivity", "com.miui.calculator/com.miui.calculator.cal.CalculatorActivity"], "configurations": [{"bounds": [0, 0, 720, 1280], "mode": "fullscreen", "source": "mOverrideConfig"}], "display_id": 10, "package": "com.miui.calculator", "parent_task_id": null, "task_id": 12267, "task_mode": "fullscreen", "visible": true, "visibleRequested": true}, {"components": ["com.android.settings/.MainSettings", "com.android.settings/.MiuiSettings", "com.android.settings/com.android.settings.MainSettings"], "configurations": [{"bounds": [280, 0, 1000, 720], "mode": "fullscreen", "source": "mOverrideConfig"}], "display_id": 10, "parent_task_id": null, "task_id": 12266, "task_mode": "fullscreen", "visible": false, "visibleRequested": false}, {"components": ["com.miui.calculator/.cal.CalculatorActivity"], "configurations": [], "display_id": 10, "parent_task_id": 12264, "task_id": 12265, "task_mode": "fullscreen", "visible": false, "visibleRequested": false}, {"components": ["com.android.settings/.MainSettings"], "configurations": [], "display_id": 10, "parent_task_id": 12264, "task_id": 12266}], "display_ids": [0, 10], "exit": 0, "kind": "activity", "stage": "after_calculator", "utc": "2026-10-21T18:53:54.676065+00:00"}
{"allowed_focus": [{"component": "none-or-other-redacted", "display_id": 0, "field": "mCurrentFocus"}, {"component": "none-or-other-redacted", "display_id": 0, "field": "mFocusedApp"}, {"component": "none-or-other-redacted", "display_id": 10, "field": "mCurrentFocus"}, {"component": "com.miui.calculator/.cal.CalculatorActivity", "display_id": 10, "field": "mFocusedApp"}], "display_ids": [0, 10], "exit": 0, "kind": "window", "stage": "after_calculator", "utc": "2026-10-21T18:53:54.717913+00:00"}
{"result": "Ok(ReturnToSession)", "stage": "return_to_session_intent", "ui_focus_confirmed": false, "utc": "2026-10-21T18:53:57.785806+00:00"}
{"disconnected": true, "ok": true, "stage": "session_stop", "utc": "2026-10-21T18:55:35.010559+00:00"}
{"exit": 0, "raw_output_discarded": {"discarded_stderr": 3, "discarded_stdout": 2}, "stage": "harness_exit", "utc": "2026-10-21T18:55:45.013752+00:00"}
{"allowed_tasks": [{"components": ["com.miui.calculator/.cal.CalculatorActivity", "com.miui.calculator/com.miui.calculator.cal.CalculatorActivity"], "configurations": [{"bounds": [0, 0, 720, 1280], "mode": "fullscreen", "source": "mOverrideConfig"}], "display_id": 10, "package": "com.miui.calculator", "parent_task_id": null, "task_id": 12267, "task_mode": "fullscreen", "visible": true, "visibleRequested": true}, {"components": ["com.android.settings/.MainSettings", "com.android.settings/.MiuiSettings", "com.android.settings/com.android.settings.MainSettings"], "configurations": [{"bounds": [280, 0, 1000, 720], "mode": "fullscreen", "source": "mOverrideConfig"}], "display_id": 10, "parent_task_id": null, "task_id": 12266, "task_mode": "fullscreen", "visible": false, "visibleRequested": false}, {"components": ["com.miui.calculator/.cal.CalculatorActivity"], "configurations": [], "display_id": 10, "parent_task_id": 12264, "task_id": 12265, "task_mode": "fullscreen", "visible": false, "visibleRequested": false}], "display_ids": [0, 10], "exit": 0, "kind": "activity", "stage": "after_stop", "utc": "2026-10-21T18:55:46.147709+00:00"}
{"allowed_focus": [{"component": "none-or-other-redacted", "display_id": 0, "field": "mCurrentFocus"}, {"component": "none-or-other-redacted", "display_id": 0, "field": "mFocusedApp"}, {"component": "com.miui.calculator/com.miui.calculator.cal.CalculatorActivity", "display_id": 10, "field": "mCurrentFocus"}, {"component": "com.miui.calculator/.cal.CalculatorActivity", "display_id": 10, "field": "mFocusedApp"}], "display_ids": [0, 10], "exit": 0, "kind": "window", "stage": "after_stop", "utc": "2026-10-21T18:55:46.228170+00:00"}
{"display_ids": [0], "display_query_exit": 0, "remaining_probe_or_scrcpy_processes": [], "stage": "final_display_and_process_check"}
{"activity": {"created_task_ids_still_present": [], "display_ids": [0], "exit": 0}, "remote_scrcpy_server_processes": 0, "stage": "final_android_cleanup", "utc": "2026-10-21T18:56:18.998813+00:00", "window": {"display_ids": [0], "exit": 0}}
```

### Reprodução com APIs do core

O harness temporário foi compilado com `cargo build -p limivra-core --example issue29_probe --locked --offline` (exit 0). Para reproduzir, salvar o código abaixo como example temporário, definir `LIMIVRA_TEST_SERIAL` apenas no ambiente e `LIMIVRA_TEST_SCRCPY` como wrapper local aprovado com as três flags de privacidade acima. Alimentar `settings`, `calculator`, `return`, `stop` no stdin; observar cada app antes do próximo comando. stdout/stderr do backend devem ser consumidos em memória e descartados; só linhas `@ISSUE29` entram na evidência. Este example não integra o produto.

```rust
use limivra_core::{
    host_controls::{ControlAction, HostControls},
    launcher::{adb_runner::AdbLauncherRunner, AllowedApp},
    *,
};
use tokio::io::{AsyncBufReadExt, BufReader};
fn emit(value: serde_json::Value) {
    println!("@ISSUE29 {value}");
}
#[tokio::main]
async fn main() {
    let target = std::env::var("LIMIVRA_TEST_SERIAL").expect("selected target env");
    let executable = std::env::var("LIMIVRA_TEST_SCRCPY").expect("local wrapper env");
    let runner = ObservedRunner(AdbLauncherRunner::system(&target).expect("approved local adb"));
    let caps = ScrcpyAdapter::detect_capabilities()
        .await
        .expect("local scrcpy capabilities");
    let provider = VirtualDisplayProvider::new();
    let options = ScrcpySessionOptions {
        stay_awake: false,
        virtual_config: Some(VirtualDisplayConfig::new(1280, 720, 160)),
    };
    let start = provider
        .start_launcher_session(&executable, &target, &options, &caps, &runner)
        .await;
    emit(serde_json::json!({"stage":"session_start","result":format!("{start:?}")}));
    if start.is_ok() {
        let mut controls = HostControls::default();
        controls.observe(&SessionEvent::StateChanged(provider.current_state().await));
        let mut lines = BufReader::new(tokio::io::stdin()).lines();
        while let Ok(Some(command)) = lines.next_line().await {
            let app = match command.as_str() {
                "settings" => Some(AllowedApp::Settings),
                "calculator" => Some(AllowedApp::Calculator),
                _ => None,
            };
            if let Some(app) = app {
                let intent = controls.request_app(app);
                let result = match intent {
                    Ok(app) => provider.launch_allowed_app(app, &runner).await,
                    Err(_) => Err(launcher::LaunchError::NotActive),
                };
                emit(
                    serde_json::json!({"stage":"app_launch","app":format!("{app:?}"),"result":format!("{result:?}")}),
                );
            } else if command == "return" {
                let intent = controls.request(ControlAction::ReturnToSession);
                emit(
                    serde_json::json!({"stage":"return_to_session_intent","result":format!("{intent:?}"),"ui_focus_confirmed":false}),
                );
            } else if command == "stop" {
                break;
            }
        }
    }
    let stop = provider.stop().await;
    emit(
        serde_json::json!({"stage":"session_stop","ok":stop.is_ok(),"disconnected":provider.current_state().await==SessionState::Disconnected}),
    );
}

struct ObservedRunner(AdbLauncherRunner);
impl launcher::LauncherRunner for ObservedRunner {
    fn run<'a>(
        &'a self,
        target: &'a str,
        request: launcher::LauncherRequest,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<launcher::LauncherReply, launcher::LaunchError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            let kind = match &request {
                launcher::LauncherRequest::Displays => "displays",
                launcher::LauncherRequest::Resolve { .. } => "resolve",
                launcher::LauncherRequest::Start { .. } => "start",
            };
            let result = self.0.run(target, request).await;
            emit(
                serde_json::json!({"stage":"runner_observation","kind":kind,"result":format!("{result:?}")}),
            );
            result
        })
    }
}
```

Filtro dos dumps: mesmo extrator publicado na [evidência do SPIKE-028](evidence/2026-10-21-spike-028-freeform.md), funções `ids`, `activities` e `windows`, aplicado somente aos três pacotes da allowlist. SHA-256 do extrator utilizado: `b7d4bb36a50a8bb6eae0adbc585c7525156a16cb74cb33597a25f7d2f15cf44d`. Saídas desconhecidas são descartadas, sem cópia de texto bruto. As checagens sintéticas desse extrator foram repetidas nesta execução antes da publicação.

## Limites e rollback

- Prova de lançamento sequencial de dois apps nesta ROM, não prova de duas janelas livres ou desktop. Matriz freeform não foi promovida.
- Foco manual confirmado pelo observador; intenção ReturnToSession coberta no core. Sem UX Qt/QML nova, sem foco automático, Wayland, audio, clipboard ou injeção de entrada testados.
- Identificação de display usa diferença de IDs e unicidade observada. Manipulação simultânea de displays por processos externos e a corrida entre última validação e envio do comando não podem ser eliminadas por essas consultas; não se declara isolamento forte. Ausência/ambiguidade observadas abortam sem lançar em display físico.
- ROMs sem `cmd display get-displays --ids-only --type` ou com respostas desconhecidas falham explicitamente; não existe fallback de parsing heurístico ou escolha do primeiro display não zero.
- Cancelamento de um comando já enviado não desfaz o efeito que Android possa ter aplicado; não há retry automático. Chamadores devem encerrar explicitamente a sessão via `stop()` no teardown, inclusive quando cancelarem o fluxo de startup.
- Rollback de produto: reverter os commits da slice. No experimento não houve alteração persistente; display/tarefas/servidor temporários foram removidos e verificados.
- Esta evidência é de desenvolvimento. O Orquestrador convocará limivra-qa para auditoria independente.

## Referência do protocolo de displays

A opção numérica de displays foi conferida na fonte primária [AOSP Android 15 — DisplayManagerShellCommand.getDisplays](https://android.googlesource.com/platform/frameworks/base/+/refs/heads/android15-release/services/core/java/com/android/server/display/DisplayManagerShellCommand.java). A fonte especifica `--ids-only` e `--type`; no aparelho, a combinação foi verificada sem supor suporte pela versão Android isoladamente. Parser desconhecido retorna erro explícito.
