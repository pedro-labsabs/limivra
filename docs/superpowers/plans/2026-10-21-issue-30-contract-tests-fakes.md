# Issue #30 — Suíte de contratos, fakes e falhas de processo — Plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans para implementar este plano tarefa por tarefa. Cada passo usa caixas de seleção (`- [ ]`).

**Goal:** Consolidar no `limivra-core` e no `limivra-cli` uma suíte determinística e offline que fixe os contratos de subprocessos, parsers, concorrência e teardown, sem telefone físico nem rede externa.

**Architecture:** Reutilizar os runners fakes existentes para ADB, scrcpy e transporte; completar os pontos de teste sem seam com executáveis falsos por caminho absoluto e variáveis de ambiente privadas do processo filho. Separar contratos de parsing, ciclo de vida de processos e invocação da CLI, usando marcadores de prontidão, barreiras/eventos e limites de tempo como sincronização. Saídas excessivas ou inválidas devem falhar de forma explícita e nunca produzir estado de sucesso parcial.

**Tech Stack:** Rust workspace, Tokio, `std::process`, `tracing`, executáveis fakes locais e Cargo em modo `--locked --offline`; `libc` já presente no lockfile é usado para sinalizar grupos Unix próprios.

**Spec:** Issue #30; `docs/testing/strategy.md`; `docs/architecture/system.md`; `docs/planning/roadmap.md`; `AGENTS.md`.

## Global Constraints

- Manter todo teste independente de telefone físico, daemon ADB real, rede externa e credenciais.
- Executar a suíte com Cargo `--locked --offline`; não adicionar dependências sem necessidade demonstrada e aprovada.
- Sinalizar ou encerrar apenas processos filhos criados pelo próprio teste; nunca usar `killall`, PID global ou daemon compartilhado.
- Não registrar nem persistir seriais reais, chaves, códigos de pareamento, saída bruta de dispositivo ou dados pessoais.
- Erros e logs de subprocesso não devem incorporar argumentos crus, caminho pessoal, stdout ou stderr; mensagens de falha permanecem tipadas e genéricas.
- Limitar tempo, quantidade de saída e recursos consumidos por cada fake; limpar arquivos e processos próprios mesmo após cancelamento ou falha.
- Não usar sleeps arbitrários como sincronização; usar canal/evento, barreira ou arquivo de prontidão e um timeout apenas como limite de segurança.
- Não afirmar ausência matemática de flakiness: registrar execuções repetidas e exigir zero falhas observadas no gate definido abaixo.
- Reconciliação aprovada pelo Orquestrador: os critérios técnicos de #15 estão satisfeitos na base desde o PR #42, e #30 pertence formalmente ao Épico #6 / M4. A consulta atual ao GitHub ainda mostra #30 em M1 e #15 aberta; registrar a divergência, sem editar esses metadados nesta slice.

## Review Focus

- ADB, dumpsys ou scrcpy com bytes UTF-8 inválidos, conteúdo truncado ou linha acima de 16 KiB não pode gerar panic nem registro confiável inventado; o teste de parsing/saída excedida da Task 2 fixa esse comportamento.
- Processo bloqueado ou cancelado não pode exceder o timeout contratado nem deixar descendente órfão; os testes de timeout, cancelamento e PID da Task 1 fixam isso.
- Resposta antiga de probe ou inicialização após stop/reconnect não pode restaurar estado de sessão anterior; os testes com barreira da Task 3 fixam isso.
- Código de saída não zero, SIGTERM ou SIGKILL deve produzir a transição/erro documentado, sem falso sucesso e sem afetar processo alheio; os testes da Task 1 fixam isso.
- A CLI deve invocar apenas os executáveis fakes sob o ambiente isolado, redigir sentinelas e terminar deterministicamente mesmo com ferramenta ausente ou falha; os testes da Task 4 fixam isso.

---

## Estado inicial verificado

- Worktree: `/tmp/limivra-30`, branch `agent/30-contract-tests-fakes`, base `2ba92849e532f8551e70fc723b912117691478c5`.
- Baseline executado: `cargo test --workspace --locked --offline` — passou com 108 testes.
- Já existem testes com executáveis fakes em `crates/limivra-core/tests/launcher_runner.rs` e `wifi_runner.rs`; o primeiro limita a saída do subprocesso a 16 KiB e o segundo a 8192 bytes por stream, além de exercitarem timeout/cancelamento e reap. Os parsers ADB/dumpsys/scrcpy têm testes locais, mas `DeviceDiscovery`, `DisplayProber` e `ScrcpyAdapter` ainda usam `Command::output()` sem limite próprio. A CLI atualmente não tem testes de integração sob um `PATH` fake.
- A árvore principal contém alterações locais preexistentes e permanece intocada; esta implementação deve permanecer exclusivamente na worktree indicada.

## Arquivos e responsabilidades

- Criar `crates/limivra-core/tests/common/mod.rs` para expor `process.rs` e `crates/limivra-core/tests/common/process.rs` com utilitários mínimos para diretório temporário exclusivo, fake executável, PID/readiness marker, leitura de saída e asserção de reaping. Cada teste sinaliza apenas PIDs conhecidos que pertencem ao fixture.
- Criar `crates/limivra-core/tests/parsing_contracts.rs`: contratos públicos de `adb::parse_adb_devices_output`, `DisplayProber::parse_dumpsys_display_output`, `DisplayProber::parse_scrcpy_list_displays_output` e `ScrcpyVersion::parse`.
- Atualizar `crates/limivra-core/tests/session.rs`, `launcher_runner.rs`, `wifi_runner.rs`, `launcher.rs`, `wifi.rs` e `host_controls.rs`: comportamento de processo, cancelamento e corridas que pertencem aos contratos existentes.
- Criar `crates/limivra-core/src/process.rs`: executor de probes com limite por stream, timeout, cancelamento, encerramento do grupo Unix e reaping do filho direto, com erro sem saída bruta; expor a API limitada usada pelo CLI.
- Ajustar `crates/limivra-core/src/lib.rs`, `adb.rs`, `display.rs` e `scrcpy.rs` para usar o executor limitado nos probes genéricos, preservando assinaturas públicas quando possível.
- Criar `crates/limivra-cli/tests/cli_process_contracts.rs`: executar `CARGO_BIN_EXE_limivra-cli` com `PATH`, `HOME` e diretório temporário isolados e executáveis fake.
- Criar `docs/testing/issue-30-contract-tests-fakes.md`: matriz de contratos, fakes usados, comandos executados, resultados, dependências locais necessárias e limites do que a suíte não prova.
- Atualizar `docs/testing/strategy.md`: substituir o status inicial “não existe suíte implementada” somente depois de registrar a suíte e seus resultados reais, mantendo E2E físico claramente separado dos testes offline.

### Task 1: Fakes e contratos de ciclo de vida de subprocesso

**Files:**
- Create: `crates/limivra-core/tests/common/process.rs`
- Create: `crates/limivra-core/tests/common/mod.rs`
- Modify: `crates/limivra-core/tests/session.rs`
- Modify: `crates/limivra-core/tests/host_controls.rs`

**Interfaces:**
- `common::process::Fixture::new(script: &str) -> Fixture` cria executável em diretório temporário privado e exclusivo.
- `Fixture::path(&self) -> &Path`, `Fixture::pid_file(&self, name: &str) -> PathBuf`, `Fixture::wait_for_pid(&self, name: &str, timeout: Duration) -> impl Future<Output = io::Result<u32>>` e `Fixture::assert_reaped(&self, pid: u32)` fornecem controle verificável do filho.
- O fixture não exporta estado global nem usa nome de arquivo compartilhado; scripts/saídas recebem apenas valores sintéticos.

- [x] **Step 1: Criar o helper isolado de processos de teste**

Implementar `tests/common/process.rs` com diretório exclusivo, fake executável, PID marker/readiness, espera limitada e verificação de reaping. Esta etapa é só suporte de teste; não muda código de produção e não envia sinais a processos que não tenham PID registrado pelo fixture.

- [x] **Step 2: Escrever regressões de saída inesperada e sinalização em `session.rs`**

Adicionar testes `session_child_exit_codes_are_reported_without_false_active_state` e `sigterm_terminated_child_reaches_terminated_state`. Usar filhos que encerram com código 1, 7 ou 127 e um fake que grava o próprio PID e se termina com SIGTERM via `kill` builtin do shell. Afirmar `ProcessExited(Some(code))` para códigos normais, `ProcessExited(None)` para término por sinal e `SessionState::Terminated` em todos os términos inesperados.

- [x] **Step 3: Executar os testes de caracterização antes de mudar produção**

Executar `cargo test -p limivra-core --locked --offline --test session`. Os eventos de término podem já passar e, nesse caso, ficam registrados como contrato existente; falha de comportamento precisa ser reproduzível antes de qualquer correção, e falha de compilação/setup não conta como RED.

- [x] **Step 4: Implementar somente correções demonstradas**

Adaptar fixtures locais repetidas apenas onde isso reduzir duplicação sem mudar semântica, e corrigir a menor fronteira de ciclo de vida necessária para estados/eventos e reaping serem coerentes. Não ampliar o escopo para um novo supervisor de processos.

- [x] **Step 5: Fixar SIGKILL, reaping e teardown idempotente do controlador**

Adicionar `session_stop_kills_and_reaps_only_its_child_and_is_idempotent`. Substituir em `host_controls.rs` as pausas fixas de `explicit_stop_stays_idempotent_after_monitor_runs` por PID marker e espera de `ProcessExited`. Afirmar que stop mata/recolhe o filho próprio, uma segunda chamada continua em `Disconnected` e um processo sentinela distinto continua vivo até o próprio teste encerrá-lo. Cobrir SIGKILL via stop e SIGTERM como término inesperado; nunca sinalizar PID fora do fixture. Os timeouts, overflow, cancelamento e reaping já cobertos por `launcher_runner.rs`/`wifi_runner.rs` serão mantidos e reexecutados, sem duplicar esses testes.

- [x] **Step 6: Verificar contratos focados**

Executar `cargo test -p limivra-core --locked --offline --test session --test launcher_runner --test wifi_runner --test host_controls`. Esperado: todos passam; eventos de término permanecem ordenados e nenhum PID de fixture continua vivo após cleanup.

### Task 2: Parsing malformado, truncado e limitado

**Files:**
- Create: `crates/limivra-core/tests/parsing_contracts.rs`
- Create: `crates/limivra-core/src/process.rs`
- Modify: `crates/limivra-core/src/lib.rs`
- Modify: `crates/limivra-core/src/adb.rs`
- Modify: `crates/limivra-core/src/display.rs`
- Modify: `crates/limivra-core/src/scrcpy.rs`
- Modify: `crates/limivra-core/tests/launcher_runner.rs`
- Modify: `crates/limivra-core/tests/wifi_runner.rs`

**Interfaces:**
- `process::run_bounded(executable: &Path, args: &[OsString], timeout: Duration, per_stream_limit: usize) -> Result<std::process::Output, ProcessError>` limita stdout e stderr independentemente, mata o grupo privado e recolhe o filho direto em timeout/cancelamento/overflow e não guarda conteúdo no erro. O CLI usa o mesmo contrato para `adb --version`.
- `ProcessError` distingue `Spawn`, `Timeout`, `OutputLimit` e `Io`; preserva o `ExitStatus` de comandos que terminam normalmente, inclusive status não zero.
- Usar `run_bounded` em `DeviceDiscovery::list_devices`, `DisplayProber::probe_device_profile`, `DisplayProber::probe_displays` e `ScrcpyAdapter::detect_version`/`detect_capabilities`.
- Preservar os parsers tipados já públicos; parsers só aceitam registros completos e válidos e nunca promovem status desconhecido a autorizado.
- Adotar para probes genéricos ADB/dumpsys/scrcpy limite de 1 MiB por stream no executor do processo e limite de 16 KiB por linha estruturada. Manter os limites atuais de 16 KiB do launcher e 8192 bytes por stream do runner Wi-Fi.
- Quando qualquer stream ultrapassar seu limite, a execução retorna erro explícito e não entrega prefixo como resposta válida; bytes UTF-8 inválidos são rejeitados no contrato estruturado, sem panic nem eco do conteúdo bruto.

- [x] **Step 1: Escrever testes de parser com entradas incompletas e corrompidas**

Em `parsing_contracts.rs`, cobrir cabeçalhos duplicados, linhas vazias, status desconhecido, colunas ausentes, registro truncado, campos fora de ordem, `Rect`/display incompleto, versão scrcpy malformada e texto contendo `U+FFFD`. Afirmar que apenas registros completos conhecidos são retornados, status desconhecido nunca é `Device`, bounds incompletos não viram display e nenhuma entrada causa panic.

- [x] **Step 2: Escrever testes do executor, bytes inválidos, linha gigante e limite total**

Criar `process.rs` com os tipos e assinatura documentados na seção Interfaces e adicionar `nonzero_exit_status_is_preserved`, `timeout_kills_and_reaps_child`, `oversized_stdout_or_stderr_kills_and_reaps_child` e `cancellation_kills_and_reaps_child`; o esqueleto inicial retorna erro sentinela em vez de implementar a execução. Em `launcher_runner.rs`/`wifi_runner.rs`, acrescentar `invalid_utf8_reply_fails_closed`. Em `parsing_contracts.rs`, acrescentar `oversized_structured_line_is_rejected`. Incluir byte `0xFF`, linha de 16 KiB + 1 byte, stream de 1 MiB + 1 byte e resposta truncada; verificar erro, ausência de resposta parcial confiável e ausência de eco de sentinela.

- [x] **Step 3: Rodar testes focados para confirmar RED comportamental**

Executar `cargo test -p limivra-core --locked --offline process::tests` e `cargo test -p limivra-core --locked --offline --test parsing_contracts --test launcher_runner --test wifi_runner`. Esperado: falha nos comportamentos ainda sem limite/validação, sem falhas de setup.

- [x] **Step 4: Implementar parsing fail-closed e limites no ponto de leitura**

Aplicar validação de linha antes de construir valores tipados; implementar `run_bounded` e usar seu timeout/limite nos probes genéricos, encerrando/recolhendo o subprocesso dono ao exceder cap ou ser cancelado. Rejeitar UTF-8 inválido nos formatos estruturados. Não usar `Command::output()` ilimitado para os caminhos exercitados. Preservar erros sem incorporar saídas potencialmente privadas.

- [x] **Step 5: Verificar a matriz de parsing**

Executar os comandos focalizados do Step 3. Esperado: entradas válidas preexistentes mantêm resultado idêntico; entradas malformadas/limites não causam panic, falso positivo ou prefixo aceito.

### Task 3: Corridas de inicialização, reconexão e cancelamento

**Files:**
- Modify: `crates/limivra-core/tests/session.rs`
- Modify: `crates/limivra-core/tests/launcher.rs`
- Modify: `crates/limivra-core/tests/wifi.rs`
- Modify: `crates/limivra-core/tests/host_controls.rs`
- Modify apenas se necessário: `crates/limivra-core/tests/wifi_runner.rs`
- Modify apenas se necessário: `crates/limivra-core/src/session.rs`, `launcher.rs`, `wifi.rs`, `host_controls.rs`

**Interfaces:**
- Fakes assíncronos de teste usam `tokio::sync::Barrier` ou `Notify` para pausar uma resposta exatamente no ponto da corrida; timeout serve apenas para evitar hang.
- Manter contratos de generation/cancellation existentes: resultado de operação iniciada em geração antiga não altera estado, provider ou projeção da sessão corrente.

- [x] **Step 1: Fixar reconnect durante launch pendente em `launcher.rs`**

Estender `stop_cancels_pending_resolution_or_start_and_old_context_cannot_relaunch`: após o fake confirmar que a resolução antiga está pendente, parar a sessão e iniciar uma sessão nova antes de aguardar a operação antiga. Afirmar que a operação antiga retorna `StaleContext`, a nova sessão segue ativa e nenhuma chamada `Start` tardia usa o contexto anterior.

- [x] **Step 2: Manter os contratos existentes de concorrência Wi-Fi**

Manter como cobertura os testes existentes `busy_cancel_and_late_response_cannot_resurrect_connection`, `cancel_during_backoff_leaves_no_retry_pending` e `cancellation_during_each_client_operation_reaps_clients_and_server`. Adicionar `cancel_during_device_probe_cannot_restore_connected`, bloqueando `RequestKind::Probe` durante a confirmação de conexão e cancelando antes de liberar a resposta. Afirmar estado final `Disconnected`, nenhum evento tardio `Connected` e processo ADB do fake recolhido; não duplicar o cenário de cancelamento já coberto pelo runner.

- [x] **Step 3: Escrever `rapid_start_stop_cycles_reap_every_owned_child`**

Executar pelo menos 20 ciclos start/stop com filhos identificados por PID. Afirmar estado final `Disconnected`, ausência de PIDs dos filhos encerrados, ausência de evento tardio que reative sessão parada e preservação de processo sentinela não pertencente ao controlador.

- [x] **Step 4: Rodar testes focados, corrigir o menor defeito de corrida e repetir**

Executar `cargo test -p limivra-core --locked --offline --test launcher --test wifi --test session --test host_controls`. Esperado: completam dentro do timeout do teste, sem `sleep` de sincronização e sem evento atrasado ressuscitar sessão anterior.

### Task 4: Contratos de processo da CLI

**Files:**
- Create: `crates/limivra-cli/tests/cli_process_contracts.rs`
- Modify somente se necessário: `crates/limivra-cli/src/main.rs`

**Interfaces:**
- O teste executa `env!("CARGO_BIN_EXE_limivra-cli")` como processo filho.
- Cada caso prepara `PATH` contendo apenas `adb`/`scrcpy` fakes, `HOME` temporário e locale `C`; remover `ANDROID_SERIAL`, `ADB_SERVER_SOCKET`, `ADB_TRACE` e variáveis de chaves/servidor.
- O fake escreve apenas fixture sintético; nenhum comando real `adb`/`scrcpy` deve ser encontrado ou usado.

- [x] **Step 1: Escrever testes CLI para execução fake, falha de ferramenta e saída hostil**

Adicionar `devices_uses_only_fake_adb_and_redacts_serial_sentinel`, `doctor_handles_missing_and_nonzero_tools`, `doctor_bounds_version_output`, `doctor_times_out_hanging_tool`, `invalid_utf8_or_oversized_adb_output_does_not_panic` e `probe_failure_has_bounded_non_sensitive_output`. Verificar código de saída/comportamento documentado, stdout/stderr, argumentos observados pelo fake, redaction do serial `LIMIVRA-TEST-SERIAL-XYZ` e que o fake prova a chamada sem consultar aparelho.

- [x] **Step 2: Rodar os testes focalizados em modo offline e observar RED**

Executar `cargo test -p limivra-cli --locked --offline --test cli_process_contracts`. Esperado: falhas comportamentais se o CLI ou a descoberta ainda expuser saída ou tratar limites de forma incompleta; sem acesso a ferramenta instalada.

- [x] **Step 3: Aplicar correção mínima somente nas fronteiras observadas**

Manter a invocação por argumentos separados, preservar redaction existente, limitar/validar saída conforme Task 2 e evitar mensagem de diagnóstico contendo serial, argumento privado ou saída bruta.

- [x] **Step 4: Verificar os contratos CLI**

Reexecutar `cargo test -p limivra-cli --locked --offline --test cli_process_contracts`. Esperado: todos os cenários passam com ambiente externo removido e apenas fakes locais.

### Task 5: Documentar matriz offline e executar gates finais

**Files:**
- Create: `docs/testing/issue-30-contract-tests-fakes.md`
- Modify: `docs/testing/strategy.md`

- [x] **Step 1: Registrar a estratégia e os limites da evidência**

Documentar tabela `contrato | fake/fixture | comando de teste | resultado esperado`, os caps por stream/linha, mecanismos de sincronização, cleanup dos filhos, dependências locais para execução Linux, e asserção de que nenhum teste usa telefone, daemon ADB real ou rede externa. Reservar campos de resultado para os comandos efetivamente executados; não preencher com sucesso antecipado.

- [x] **Step 2: Executar a suíte workspace sem rede e repetição contra flakiness**

Executar `CARGO_NET_OFFLINE=true cargo test --workspace --locked --offline` uma vez e, depois, repetir 10 vezes em sequência com o mesmo comando. Esperado: todas as execuções terminam com código 0, sem processo residual; registrar número de testes e código de saída de cada repetição. Qualquer falha intermitente impede concluir a issue até diagnóstico e correção.

- [x] **Step 3: Executar gates de qualidade e whitespace**

Executar `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` e `git diff --check origin/main...HEAD`. Esperado: cada comando termina com código 0 e Clippy sem warnings.

- [x] **Step 4: Completar o relatório com os resultados reais**

Atualizar `docs/testing/issue-30-contract-tests-fakes.md` com versões do Rust/Cargo, comandos literais, códigos de saída, contagem de testes, resultados das 10 execuções, limites e exclusões. Marcar telefone/rede real como não usados, pois esta issue exige suíte hermética offline.

## Gate de implementação

O Orquestrador aprovou este plano e reconciliou a prontidão: a dependência técnica #15 está satisfeita desde PR #42, e a Issue #30 pertence ao Épico #6 / M4. A divergência restante na visualização do GitHub será registrada, sem alteração externa de milestone/issue. A execução fica limitada à suíte e documentação acima; commit/push/PR seguem a autorização explícita do Orquestrador nesta tarefa e os gates de validação.
