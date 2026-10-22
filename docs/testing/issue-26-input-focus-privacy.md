# Issue #26 — evidência de desenvolvimento: input, foco e clipboard

Refs #26. Plano aprovado na íntegra pelo Orquestrador em 2026-10-21.
Implementador: limivra-dev. Auditoria independente limivra-qa: PENDENTE.

## Ambiente, base e escopo

Rust 1.88.0, Cargo 1.88.0, Linux Mint 22.3; worktree isolada.
Branch `agent/26-input-focus-privacy`, base autorizada
`260d34effe27b56c990cc815e9f2bab3934c3668` (Issue #25).
Os onze commits ancestrais fora de origin/main não pertencem a esta slice;
a comparação de implementação deve usar essa base.

Arquivos: módulo `input_policy`, exportação em lib.rs, suíte dedicada,
plano aprovado e esta evidência. Nenhuma dependência nova ou alteração de
provider, scrcpy, CLI, Qt, pesquisa herdada ou configuração de dispositivo.

## Contrato implementado

- Roteamento exige Active, foco, consentimento e seleção explícita de modo
  Supported. Só o conjunto transitório de controles pressionados é guardado;
  duplicatas são ignoradas. Sem texto digitado, captura global ou histórico.
- Perda de foco/consentimento, troca de modo, mudança de capacidade e transições
  de sessão drenam os controles; keys/buttons antecedem modificadores. As
  liberações são idempotentes e retornadas como intents, sem entrega presumida.
- Cada StateChanged invalida observações e grants, inclusive Active repetido
  ou mudança de display/dispositivo. ProcessExited também reinicia a política.
  Grants/observações feitos antes de Active são descartados. Error e anúncio
  de fallback não inventam transição de sessão.
- SDK mapeia para `--keyboard=sdk --mouse=sdk`; UHID para `-K -M`. Unknown,
  Unsupported e Blocked não habilitam seleção. SDK bloqueado não seleciona
  UHID automaticamente. Alteração de capacidades invalida o modo, exigindo
  nova seleção. Selecionar o mesmo modo não drena teclas desnecessariamente.
- Clipboard opt-in é local à sessão e separado de consentimento de teclado.
  Transferência explícita exige Active, foco, opt-in, suporte da direção e
  proteção Allowed atual fornecida pelo adaptador. Unknown/Restricted falham
  fechados, incluindo restrições de sistema/FLAG_SECURE observadas.
- O callback síncrono só é chamado após o gate; ele é onde a leitura deve
  ocorrer. A política não recebe texto, não guarda callback, não tem fila,
  cache, serialização ou I/O. Ok significa callback invocado, não sucesso de
  transporte. O consumidor deve relatar seus próprios erros de transferência.

## TDD observado

Comando dedicado: `cargo test -p limivra-core --test input_policy --locked --offline`.

1. RED com API esqueleto: exit 101; seis dos sete testes falharam, por aceitar
   input desconectado, encaminhar release espúrio, não gerar liberações,
   argumentos de modo vazios, grants não invalidados e spawn sem revogação.
2. Após implementar input/modos: seis passaram; teste de reconexão continuou
   RED porque o callback de clipboard ainda era invocado sem novo opt-in.
3. Adicionados testes de clipboard: exit 101, seis passaram e três falharam.
   Confirmados callback chamado desconectado, chamada sem foco e chamada após
   reconexão sem novo opt-in. Nenhum hardware envolvido.
4. Implementado gate de clipboard: exit 0, nove testes dedicados passaram.
5. Clippy apontou uma asserção constante no teste de callback; substituída por
   contagem de bytes sintéticos transferidos. Reexecutadas suíte e verificações.

Suíte dedicada cobre gates de input; duplicatas; liberação ordenada e idempotente
em seis causas de revogação; reset entre estados/displays; modos defensivos;
falha de spawn real via SessionController; preservação em Error de Active;
negativas de clipboard para duas direções com contador de leitura zero; callback
permitido explicitamente, opt-out/perda de foco e Debug sem payload sintético.
O executável inválido `/dev/null/not-an-executable` garante falha sem depender
de PATH e sem processo de produto ou aparelho. Não são testes E2E de transporte.

## Verificações finais

| Comando | Resultado observado |
| --- | --- |
| `cargo check --workspace --all-targets --locked --offline` | exit 0 |
| `cargo test --workspace --locked --offline` | exit 0: 50 testes passaram (3 CLI, 31 core, 7 host_controls, 9 input_policy) |
| `cargo clippy -p limivra-core --lib --test input_policy --locked --offline -- -D warnings` | exit 0 |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | exit 101: seis lints herdados em provider.rs |
| `cargo fmt --all -- --check` | exit 0 |
| `git diff --check` | exit 0 |

Clippy global: `uninlined_format_args` em provider.rs:422,445,456,503,565 e
`single_match` em provider.rs:468. Os mesmos lints já estão documentados na
base em `docs/testing/issue-25-host-controls.md`; provider.rs não foi alterado.
Não há supressões de lint nem correção fora de escopo.

## Limitações e risco

A camada recebe observações, não detecta FLAG_SECURE ou suporte da ROM sozinha.
O adaptador deve fornecer proteção atual por operação e capacidades da sessão/
dispositivo/display atuais, separar atalhos locais e normalizar modificadores.
Eventos devem chegar ordenados; após lag, reiniciar a política antes de input.
Liberações devem ser entregues antes de perder transporte ou o adaptador deve
limpar seus dispositivos de input próprios. Nenhum sucesso físico é presumido.

O callback não pode fazer leitura antecipada ou enfileirar leitura assíncrona;
o core não impede código externo de reter dados nem apaga clipboard do OS,
memória de backend ou buffers do consumidor. Retenção zero é o contrato desta
camada sem payload, não uma promessa de apagamento físico de memória externa.

Qt/Wayland/X11, IME real, SDK/UHID em aparelho, liberação remota, clipboard real
e FLAG_SECURE end-to-end: UNTESTED nesta execução. Sem telemetria, acesso ao
telefone, alteração de segurança ou fallback automático adicionados.
O produto/Issue #26 não é declarado COMPLETE com estes testes unitários.

Rollback: reverter apenas o commit desta slice, preservando ancestrais e aparelho.
Próximo gate: Orquestrador despacha limivra-qa para auditoria independente.
