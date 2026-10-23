# Limivra M1 Foundation / Skeleton — Implementation Plan

> **For future agentic workers:** this minimum plan was approved by the maintainer on 23/10/2026 ([Issue #8 confirmation](https://github.com/pedro-labsabs/limivra/issues/8#issuecomment-6367240150)). That approval grants no implementation authorization by itself; satisfy all start gates below before executing it.

**Status:** ACCEPTED / APROVADO COMO PLANO MÍNIMO — 23/10/2026, pelo mantenedor, conforme [comentário na Issue #8](https://github.com/pedro-labsabs/limivra/issues/8#issuecomment-6367240150).

**Goal:** propor o menor alicerce reproduzível para a Issue [#15](https://github.com/pedro-labsabs/limivra/issues/15): workspace Linux com esqueleto Session Core/CLI em Rust e prova determinística da supervisão de subprocessos próprios, sem sessão Android funcional.

**Architecture:** Propõe pacotes Rust separados para Session Core e CLI apenas como organização de workspace; isso não define a topologia de processos da futura UI Qt. O núcleo poderá supervisionar subprocessos ADB/scrcpy próprios com timeout, cancelamento e encerramento seguro; os testes usarão somente processos auxiliares locais. IPC, FFI, esquema de mensagens, integração Qt e embedding de vídeo continuam decisões futuras.

**Tech Stack:** Rust para Session Core/CLI e checks Linux. Versão do toolchain, runtime assíncrono e dependências ainda dependem da conclusão documentada de #11 e de revisão humana; este plano não os escolhe.

**Spec:** [especificação de design](../specs/2026-10-18-limivra-design.md), [PRD](../../product/PRD.md), [ADRs aceitas no escopo limitado](../../architecture/decisions/ADR-0005-stack-core-rust.md) e [reconciliação M0](../../planning/m0-design-decision-packet.md).

## Limites globais

- Sem root, `sudo`, desbloqueio de bootloader, bypass ou alteração persistente de controles de segurança.
- Não executar `adb devices`, não invocar ADB/scrcpy contra aparelho, não abrir conexão USB/Wi-Fi e não alegar compatibilidade de ROM.
- O supervisor só pode encerrar processos filhos que o próprio Limivra iniciou; sem encerramento global por nome de processo.
- Não implementar UI Qt/QML, embedding de vídeo, comandos de sessão, descoberta de dispositivo, MIRROR, VIRTUAL_DISPLAY ou DESKTOP_WINDOWING neste slice.
- Não escolher topologia UI/Core, IPC, FFI, transporte, serialização, esquema de mensagens ou runtime assíncrono.
- Fixar e auditar qualquer dependência e toolchain antes de CI; não adicionar dependências sem decisão documentada.
- Os testes locais de subprocesso devem usar executáveis auxiliares falsos, nunca os binários ADB/scrcpy instalados.

## Arquivos previstos para uma implementação futura

Os caminhos abaixo compõem o plano mínimo aprovado para a futura Issue #15; nenhum deles será criado nesta entrega documental.

| Caminho futuro | Responsabilidade proposta |
|---|---|
| `Cargo.toml` | Workspace Cargo mínimo para os pacotes Core e CLI. |
| `Cargo.lock` | Reprodutibilidade de dependências, se houver dependências. |
| `rust-toolchain.toml` | Fixar a versão aprovada após a evidência de stack de #11. |
| `crates/limivra-core/Cargo.toml` | Pacote Rust do Session Core. |
| `crates/limivra-core/src/lib.rs` | Raiz compilável do Core; sem contrato público de sessão adicional nesta slice. |
| `crates/limivra-core/src/process_supervisor.rs` | Fronteira mínima para iniciar e supervisionar somente subprocessos filhos próprios. |
| `crates/limivra-core/tests/process_supervisor.rs` | Testes de integração com processos auxiliares locais para término, timeout, cancelamento e teardown. |
| `crates/limivra-core/tests/fixtures/fake-child.sh` | Processo auxiliar determinístico para testes locais Linux, sem ADB, scrcpy ou dispositivo. |
| `crates/limivra-cli/Cargo.toml` | Pacote binário CLI em Rust. |
| `crates/limivra-cli/src/main.rs` | Entrada mínima com ajuda/versão; sem comando que acesse dispositivo. |
| `crates/limivra-cli/tests/help.rs` | Smoke test de `--help` e código de saída, sem efeitos externos. |
| `.github/workflows/ci.yml` | Checks reproduzíveis em Linux, após aprovação do toolchain e das dependências. |
| `docs/development/linux-build.md` | Pré-requisitos e comandos locais de build/test do workspace. |

## Review Focus

- Falha ao iniciar um executável auxiliar: reportar falha sem registrar processo inexistente nem deixar estado órfão; coberto por `spawn_failure_is_reported` em `process_supervisor.rs`.
- Filho encerra por conta própria: observar o resultado e recolher o processo sem afetar outro; coberto por `child_exit_is_reaped`.
- Timeout: retornar resultado limitado no tempo e limpar apenas o filho criado pela operação; coberto por `timeout_cleans_owned_child`.
- Cancelamento: encerrar apenas o filho da operação cancelada, deixando um processo sentinela independente vivo; coberto por `cancel_ends_only_owned_child`.
- Teardown repetido ou concorrente: manter o encerramento idempotente e não atingir processos externos; coberto por `teardown_is_idempotent`.

---

## Tarefas do plano mínimo aprovado — execução condicionada aos gates

### Task 1: Definir gates e criar o workspace compilável

**Arquivos:** `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `crates/limivra-core/Cargo.toml`, `crates/limivra-core/src/lib.rs`, `crates/limivra-cli/Cargo.toml`, `crates/limivra-cli/src/main.rs`.

- [ ] Antes de alterar arquivos, confirmar o registro de aprovação humana do PRD/especificação e deste plano na #8. A aprovação documental não libera M1: #9 requer consentimento específico e observação humana; #10 depende da evidência de #9; e #11 precisa concluir a avaliação de viabilidade. Se qualquer gate estiver aberto ou sem evidência, STOP.
- [ ] Criar o workspace e os dois pacotes vazios, sem acoplar CLI e Core nem definir a integração futura com a UI.
- [ ] Registrar a versão Rust após evidência de #11 e aprovação humana; produzir o lockfile e não introduzir dependência não aprovada.
- [ ] Rodar `cargo build --workspace --locked` e `cargo test --workspace --locked`; exigir ambos com exit code 0 em Linux limpo.

### Task 2: Provar o limite de ciclo de vida de subprocessos

**Arquivos:** `crates/limivra-core/src/process_supervisor.rs`, `crates/limivra-core/tests/process_supervisor.rs`, `crates/limivra-core/tests/fixtures/fake-child.sh`.

- [ ] Escrever primeiro os testes de falha e comportamento com um processo auxiliar local: inicialização recusada, saída espontânea, timeout, cancelamento, processo sentinela independente e teardown repetido.
- [ ] Rodar `cargo test -p limivra-core --test process_supervisor`; confirmar RED por falha comportamental dos casos do supervisor, não por erro de setup ou compilação.
- [ ] Implementar a menor supervisão que cumpra ownership do filho, timeout, cancelamento e cleanup seguro; não definir aqui duração padrão, política específica de sinais ou dependência assíncrona sem aprovação.
- [ ] Repetir o teste direcionado e exigir GREEN; o sentinela independente deve continuar vivo após timeout/cancelamento do filho administrado.

### Task 3: Manter CLI inerte e documentar build local

**Arquivos:** `crates/limivra-cli/src/main.rs`, `crates/limivra-cli/tests/help.rs`, `docs/development/linux-build.md`.

- [ ] Escrever teste de `--help` que exija saída de ajuda e exit code 0 sem criar subprocesso ADB/scrcpy.
- [ ] Rodar `cargo test -p limivra-cli --test help`; confirmar RED por comportamento ausente.
- [ ] Implementar somente a ajuda/versão mínima, sem descoberta, sessão ou conexão Android; documentar setup e comandos locais reproduzíveis.
- [ ] Repetir o teste direcionado e exigir GREEN.

### Task 4: Integrar os checks reproduzíveis de Linux

**Arquivos:** `.github/workflows/ci.yml`, `Cargo.lock`, `docs/development/linux-build.md`.

- [ ] Configurar CI Linux com a versão aprovada e dependências bloqueadas pelo lockfile.
- [ ] Executar `cargo fmt --all --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --locked` e `cargo build --workspace --locked`.
- [ ] Exigir exit code 0 para cada comando em runner Linux limpo e manter as mesmas instruções na documentação.
- [ ] Registrar evidência, risco, rollback e limitações na PR de implementação futura; não usar CI como prova de comportamento em aparelho.

## Plano RED → GREEN futuro

1. RED comportamental dos testes de spawn, saída espontânea, timeout, cancelamento e teardown com executáveis auxiliares locais; falhas devem identificar comportamento ausente, nunca erro de compilação ou setup.
2. GREEN com a implementação mínima de supervisão de processos próprios e limpeza limitada ao filho gerenciado.
3. RED/GREEN separado para CLI `--help`; depois executar build, fmt, Clippy e testes do workspace.
4. Os comandos ADB/scrcpy reais e a sessão USB pertencem a slices posteriores e autorizadas; nenhum teste deste plano pode tocar aparelho.

## Gates e critérios de aceite

**Gates antes de iniciar:**

- Aprovação humana do PRD/especificação e deste plano mínimo registrada na #8, subordinada à segurança e aos escopos das ADRs aceitas. Essa aprovação documental não dispensa os gates seguintes.
- #9 concluída com consentimento específico e observação humana; seu estado permanece BLOCKED enquanto isso não ocorrer.
- #10 concluída com matriz sustentada por evidência e dependente da evidência de #9.
- #11 concluída com evidência conclusiva de viabilidade.
- Gates de saída M0 do roadmap satisfeitos antes de iniciar M1.
- Nenhuma mudança de `origin/main` que invalide a baseline exige continuar automaticamente: parar e revalidar escopo/base.

**Aceite técnico futuro da #15:**

- Workspace Rust Core/CLI compila e testa de forma reproduzível em Linux com versões/dependências fixadas.
- Testes falsos comprovam falha de spawn observável, saída recolhida, timeout, cancelamento limitado ao filho próprio e teardown idempotente.
- CLI de ajuda funciona sem acesso a dispositivo; setup Linux está documentado.
- Não há root, execução de ADB/scrcpy contra telefone, alegação de compatibilidade de ROM, UI, IPC, FFI ou embedding de vídeo nesta slice.
- PR futura contém comandos/saídas, revisão de permissões, risco e rollback; QA independente permanece separado do executor.

## Dependências, riscos e condições de STOP

- **Dependências e estado consultado em 2026-10-23:** #15 depende formalmente de #8 e #11; o início de M1 também requer os gates #9 e #10. #9 está aberta com `status:blocked`; #10 está aberta e depende da evidência de #9; #11 está aberta e requer conclusão de viabilidade; #8 e #15 permanecem abertas. A aprovação do plano não altera esses estados.
- **Risco de contrato:** assinatura e semântica interna do supervisor, valores de timeout e política de encerramento não foram aceitos; devem ser apresentados antes da implementação se alterarem limites de segurança.
- **Risco de escopo:** ligar UI e Core pode exigir topologia, IPC, FFI ou embedding de vídeo; qualquer necessidade disso suspende esta slice até decisão humana separada.
- **STOP:** falta de aprovação escrita, dependência #11 sem evidência, gate M0 ainda pendente, mudança de base, necessidade de root, acesso a dispositivo, encerramento por nome de processo, ou necessidade de escolher um contrato de IPC/FFI/vídeo.

**Rollback futuro:** reverter somente os arquivos do slice #15 em sua branch dedicada, sem reset ou limpeza de checkout compartilhado; preservar logs sanitizados e evidências de CI necessárias à revisão.

**Estado:** plano mínimo aprovado em 23/10/2026, documental e não executado. #9, #10 e #11 continuam sendo gates para o início de M1; nenhuma issue teve seu status alterado por esta aprovação.
