# Issue #26 — Input, foco e privacidade de clipboard

Estado: aprovado na íntegra pelo Orquestrador em 2026-10-21; implementado
com evidência de desenvolvimento, aguardando auditoria independente.
Executor: limivra-dev. Auditor independente: limivra-qa, convocado pelo
Orquestrador após a entrega. Refs #26.

## Objetivo e base

Implementar uma camada de política no limivra-core para entrada local consentida,
liberação de teclas/modificadores/botões e transferência explícita de clipboard.
Branch `agent/26-input-focus-privacy`, base exata
`260d34effe27b56c990cc815e9f2bab3934c3668` de `agent/25-host-desktop-ux`.
Worktree `/tmp/limivra-26`. Não modificar os checkouts de desenvolvimento ou QA
anteriores. A Issue #26 está OPEN, M3, `status:needs-evidence`; depende da #25,
cujo PR #51 ainda está aberto. O usuário autorizou a base dependente.

Fontes: Issue #26; PRD, arquitetura e estratégia de testes; pesquisa em
`docs/research/findings/aosp-scrcpy-public-surfaces.md`, seção 4.3;
`docs/research/SPIKE-001.md` e `docs/research/capability-matrix.md`.
Os registros de hardware são herdados: esta execução não os reproduziu.

## Desenho proposto e limites

Criar módulo `input_policy` independente de Qt, APIs de clipboard e processos.
A camada recebe eventos/observações do adaptador e devolve decisões tipadas.
Não capturar teclado global, ler clipboard do sistema, acessar telefone,
alterar políticas Android ou adicionar dependências nesta slice.

A abordagem escolhida é uma máquina de política determinística no core.
Integração direta com Qt/Wayland e backend scrcpy exigiria outra slice de
adaptação; um simples conjunto de flags não cobriria a liberação de teclas.

### Foco, consentimento e liberação

- Inicialmente desconectado, sem foco, sem consentimento e sem modo selecionado.
- Roteamento requer sessão ativa, foco local, consentimento explícito e modo
  com suporte observado. Usar códigos tipados para teclas, modificadores e
  botões; não armazenar texto digitado ou histórico de eventos.
- Manter somente o conjunto transitório de controles pressionados. Press/release
  duplicados não devem gerar estado preso nem liberação espúria.
- Perda de foco, desconexão, revogação de consentimento, troca de modo ou
  invalidação da capacidade drenam o conjunto e retornam eventos de liberação
  idempotentes, com teclas/botões antes dos modificadores.
- Desconexão também revoga foco, consentimento e clipboard opt-in. Reconexão
  requer novas decisões explícitas e nunca reenvia teclas antigas.
- O adaptador é responsável por enviar as liberações enquanto houver transporte
  ou limpar o dispositivo de input que possui quando o transporte já caiu.
  Retornar intents não comprova liberação física nem entrega ao Android.
- Expor transição a partir de SessionEvent: Connecting, Disconnected,
  Terminated e ProcessExited desativam roteamento; Active não concede foco ou
  consentimento automaticamente. Error não deve inventar desconexão de sessão
  Active; a correção de spawn de #25 fornece o evento confirmado.

### Clipboard opt-in e retenção

- Desabilitado por padrão, sem observador em background ou sincronização
  automática. Cada transferência tem direção explícita (host→Android ou
  Android→host) e deve ser iniciada pelo usuário com foco local.
- Exigir sessão ativa, foco, opt-in, capacidade observada para a direção e
  observação atual de que a operação é permitida pelo sistema e pelo contexto.
- Contexto seguro/restrito, inclusive FLAG_SECURE informado pelo adaptador,
  bloqueia a operação. Estado de proteção desconhecido também bloqueia.
  Não inferir ausência de proteção por falta de erro ou pela versão da ROM.
- A política não recebe nem guarda texto: recebe somente metadados e executa
  callback síncrono de transferência apenas quando a política permite. O
  callback é o ponto em que o adaptador poderá ler e transferir; negar deve
  impedir até mesmo a leitura. Não retornar permissões reutilizáveis que possam
  sobreviver à perda de foco.
- Nenhum campo, log, Debug, serialização, fila, cache ou arquivo conterá texto
  de clipboard. Retenção zero refere-se à camada core; não prometer apagar o
  clipboard do sistema, a memória do backend ou cópias feitas pelo consumidor.
- Não aceitar sucesso unitário como prova de FLAG_SECURE real ou privacidade
  end-to-end: essas observações ficam para integração e QA em aparelho.

### SDK Input e UHID

- `InputMode::{Sdk,Uhid}`; capacidades individuais com valores
  Supported/Unsupported/Unknown/Blocked, inicialmente Unknown.
- A escolha é explícita e só aceita Supported observado para a sessão atual,
  dispositivo e display alvo; fabricante e SDK level não concedem suporte.
- Mapear SDK para `--keyboard=sdk --mouse=sdk`; UHID para `-K -M`.
  Produzir argumentos tipados/estáticos, sem lançar subprocessos nesta slice.
- SDK bloqueado não seleciona UHID automaticamente. Usuário pode escolher UHID
  somente após observação autorizada de suporte e consentimento. Não elevar
  privilégios, alterar opções de segurança ou contornar FLAG_SECURE/DRM.
- Mudança de sessão/display invalida observações anteriores. IME permanece
  condicionado ao backend e à capacidade observada, sem habilitação implícita.

## Arquivos e interfaces previstos

- Criar `crates/limivra-core/src/input_policy.rs`: modos/capacidades, eventos
  tipados, estado de foco/consentimento e gates de transferência sem payload.
- Alterar `crates/limivra-core/src/lib.rs`: exportar `input_policy`.
- Criar `crates/limivra-core/tests/input_policy.rs`: suíte dedicada de contratos.
- Criar `docs/testing/issue-26-input-focus-privacy.md`: ambiente, RED/GREEN,
  resultados reais, riscos e limites; atualizar o estado deste plano.

Contratos a implementar:

- `InputMode::scrcpy_arguments() -> [&'static str; 2]`.
- `InputPolicy::observe_session(&SessionEvent) -> Vec<InputEvent>`.
- `set_focus(bool)`, `set_input_consent(bool)`, `set_capabilities(InputCapabilities)`
  e `select_mode(InputMode)` retornam liberações, ou erro tipado na seleção.
- `route(InputEvent) -> Result<Option<InputEvent>, InputDenied>` mantém somente
  controles pressionados enquanto a política permite roteamento.
- `set_clipboard_opt_in(bool)` controla opt-in local à sessão.
- `transfer_clipboard(direction, protection, callback) -> Result<(), ClipboardDenied>`
  valida a política antes de executar callback síncrono sem payload no core.
- Tipos privados para estado interno, sem APIs que exponham texto ou concedam
  foco/consentimento por padrão. Callback não terá permissão de mutar a política
  durante a transferência síncrona; não fazer await entre gate e operação.

## Execução TDD após aprovação

1. Escrever testes de defaults e gates de foco/consentimento/capacidade; executar
   para observar falhas comportamentais, depois implementar o mínimo.
2. Escrever testes de press/release, múltiplos modificadores, perda de foco,
   desconexão, revogação, troca de modo, idempotência e reconexão. Observar RED
   e implementar a liberação determinística.
3. Testar projeção com eventos reais de SessionController em falha de spawn,
   sem aparelho e sem executável externo funcional, além de eventos de término.
4. Escrever testes de clipboard negado por padrão, sem foco, desconectado,
   opt-out, direção sem suporte e proteção Unknown/Restricted. Um contador de
   leituras no callback deve permanecer zero em todos os casos negados.
5. Testar transferência explícita permitida em ambas as direções; opt-out e
   perda de foco impedem chamadas posteriores. A política nunca recebe texto;
   testar Debug/estado sem payload e ausência de cache por construção.
6. Escrever testes de SDK/UHID observados, Unsupported/Unknown/Blocked,
   argumentos exatos e ausência de seleção silenciosa após SDK bloqueado.
   Observar RED e implementar seleção defensiva.
7. Rodar `cargo check --workspace --all-targets --locked --offline`,
   `cargo test --workspace --locked --offline`,
   `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`,
   `cargo fmt --all -- --check` e `git diff --check`.
   Caso os seis lints herdados de provider.rs permaneçam, reportá-los e rodar
   também Clippy estrito de `--lib --test input_policy`, sem supressão ou
   refatoração fora do escopo.
8. Registrar evidência real, commit e push do branch autorizado. Abrir PR para
   main com `Refs #26`, descrevendo a ancestralidade ainda não integrada e
   sem fechamento automático. Informar prontidão para limivra-qa; sem merge.

## Riscos e critérios de entrega

Aceite da slice: testes provam política determinística, opt-in, impedimento de
leitura negada, argumentos defensivos e geração de liberações. APIs não retêm
texto. Nenhuma alegação de integração física/Qt/Wayland ou liberação entregue.
A implementação não torna a Issue #26 COMPLETE por si só.

Riscos: observações incorretas/antigas do adaptador, transporte que já caiu,
callback que retém texto fora do core e conflito de atalhos/foco na UI. Devem
constar no contrato público e na evidência. Rollback: reverter só os commits
desta slice, sem alterar telefone, preferências do sistema ou outros branches.

## Registro de execução

Nove testes dedicados implementados; RED/GREEN e verificações registrados em
`docs/testing/issue-26-input-focus-privacy.md`. Sem mudanças de escopo.
A auditoria independente não foi executada pelo implementador.
