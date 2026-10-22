# Issue #25 — plano da camada de controle do host

Estado: plano aprovado pelo usuário como Orquestrador em 2026-10-21;
slice implementada, aguardando auditoria independente do limivra-qa.

**Objetivo:** implementar no `limivra-core` o processamento de atalhos e ações
de sessão solicitado pelo usuário, com testes unitários. O executor codifica;
`limivra-qa` realiza a auditoria independente.

**Base:** `origin/agent/24-compatibility-matrix-qa`, `c11cec5`.
Branch: `agent/25-host-desktop-ux`. Worktree isolada para preservar alterações
preexistentes de outro trabalho.

**Fontes:** PRD, arquitetura e ADRs do projeto; Issue #25; solicitação do
usuário de 2026-10-21. A especificação geral permanece PROPOSED.

## Escopo e contrato proposto

Camada Rust independente de Qt, sem dependências novas e sem acesso ao aparelho.
Toolbar futura e atalhos devem produzir as mesmas ações tipadas:
`Disconnect`, `SwitchDisplay`, `ToggleVideoPause`, `ReloadVirtualDisplay`.
Atalhos locais propostos: Ctrl+Shift+D, Ctrl+Shift+S, Ctrl+Shift+P e
Ctrl+Shift+R, respectivamente. Ignorar repetição automática, teclas não
mapeadas e combinações com modificadores adicionais.

Eventos de `SessionEvent` alimentam uma projeção de status sem serial ou
conteúdo de logs. Ações só ficam disponíveis em sessão ativa; recarga requer
modo virtual observado. O processamento retorna intenção de controle ao
adaptador, sem afirmar que o efeito já ocorreu. Pausa, alternância e recarga
somente podem alterar o status confirmado após resposta do backend. Falhas
devem ser observáveis e não produzir confirmação fictícia.

Esta slice não implementa uma toolbar Qt, captura global de teclado, transporte
de comandos scrcpy ou Wi-Fi. A integração executável e os aceites visuais da
Issue #25 continuam pendentes; o PR desta camada deve usar `Refs #25`, sem
fechamento automático. Confirmar esse limite na aprovação.

## Arquivos esperados

- Criar `crates/limivra-core/src/host_controls.rs`: ações, atalhos e projeção.
- Alterar `crates/limivra-core/src/lib.rs`: exportação do módulo.
- Criar `crates/limivra-core/tests/host_controls.rs`: testes de contrato.
- Criar `docs/testing/issue-25-host-controls.md`: comandos, resultados e limites.

## Execução após aprovação

1. Escrever testes de mapeamento das quatro combinações e rejeição de
   repetição/modificadores adicionais; observar RED antes de implementar.
2. Implementar o parser tipado mínimo; executar os mesmos testes até GREEN.
3. Escrever testes de ações equivalentes por toolbar/atalho, bloqueio fora de
   Active, recarga apenas virtual, mudança de conexão e fallback para mirror.
   Confirmar que emitir intenção não modifica estado confirmado e que erros
   preservam essa distinção. Observar RED e implementar a projeção mínima.
4. Executar `cargo test --workspace --locked --offline`,
   `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
   --locked --offline -- -D warnings` e `git diff --check`.
5. Registrar resultados reais, fazer commit e push no branch autorizado e
   abrir PR para `main`, explicitando a ancestralidade M1/M2 ainda não integrada.
   Reservar revisão e validação independente ao `limivra-qa`.

## Riscos e rollback

Atalhos podem conflitar com apps Android; a camada não captura teclas globais.
As limitações do backend não devem virar capacidades presumidas. A supervisão
de processos existente requer prova própria antes de prometer saída sem órfãos;
não refatorar esse subsistema implicitamente nesta slice. Rollback: reverter
apenas o commit desta camada, sem modificar dispositivo ou matar processos.

## Evidência inicial e prontidão

Consulta GitHub: #25 OPEN, M3, `status:needs-evidence`; dependências #18 e #19
OPEN. A base alternativa foi explicitamente autorizada pelo usuário, mas isso
não comprova os gates dessas issues nem os aceites visuais da #25.

Baseline na worktree: Rust 1.88.0, Cargo 1.88.0;
`cargo test --workspace --locked --offline`: exit 0, 34 testes passaram
(3 CLI e 31 core). Isso é verificação de desenvolvimento da base, não auditoria
independente ou prova em hardware. Wayland/X11, resize/foco e aparelho:
UNTESTED nesta execução.

Aprovação recebida: contrato restrito à camada core e execução apesar das
dependências abertas. A UI Qt e o gate total da Issue #25 permanecem pendentes.
Resultados de desenvolvimento: `docs/testing/issue-25-host-controls.md`.
