# Issue #25 — evidência de desenvolvimento da camada host controls

Refs #25. Slice aprovada pelo Orquestrador em 2026-10-21. Implementador:
limivra-dev; auditoria independente reservada ao limivra-qa, ainda não realizada.

## Ambiente e base

Linux Mint 22.3, kernel 7.0.0-30-generic, rustc/cargo 1.88.0.
Base `c11cec5`, branch `agent/25-host-desktop-ux`, derivado da alternativa
`origin/agent/24-compatibility-matrix-qa` autorizada. Existem nove commits
ancestrais dessa base fora de `origin/main` no momento da entrega; o diff da
slice deve ser inspecionado contra `c11cec5`, não confundido com os ancestrais
M1/M2. O checkout original permaneceu com suas alterações preexistentes.

## Contrato implementado

`host_controls::KeyStroke::action()` aceita apenas Ctrl+Shift+D/S/P/R,
sem Alt/Meta ou repetição. Letras ASCII maiúsculas/minúsculas são aceitas.
A UI deve fornecer eventos locais de key-down; não há captura global.

`HostControls::request()` aceita ações de toolbar; `shortcut()` usa exatamente
a mesma validação. Só Active aceita ações; ReloadVirtualDisplay requer modo
virtual confirmado. Os resultados são intenções para o adaptador, sem execução,
sem alterar estado e sem prometer que o backend suporta a operação.

`observe(&SessionEvent)` projeta conexão/modo sem guardar serial ou texto bruto
de erros/fallback. FallbackTriggered apenas anuncia fallback; StateChanged
confirma modo. Error marca falha sem alterar modo/pausa confirmados.
ProcessExited invalida modo/pausa. Cada StateChanged invalida a confirmação de
pausa e limpa avisos; video_paused=None significa desconhecido.
`confirm_video_paused(bool)` aceita observação autoritativa somente em Active.

O consumidor deve fornecer eventos ordenados de uma única sessão, descartar
respostas antigas e reconstruir a projeção após lag/reconexão. A camada não
correlaciona respostas de transporte nem implementa scheduler de comandos.

## TDD observado

Comando: `cargo test -p limivra-core --test host_controls --locked --offline`.

1. RED com parser esqueleto: exit 101; `maps_all_local_shortcuts` recebeu None
   em vez de Some(Disconnect). Um teste passou e um falhou.
2. GREEN do parser: exit 0; dois testes passaram.
3. RED com projeção esqueleto: exit 101; três testes falharam por aceitar
   Disconnect desconectado, manter Disconnected após Connecting e não projetar
   anúncio de fallback. Três testes passaram e três falharam.
4. GREEN da projeção: exit 0; seis testes passaram.

Os testes cobrem os quatro atalhos, modificadores, repetição, entradas
ignoradas, equivalência toolbar/teclado, intents sem efeitos fictícios,
restrições de estado/modo, confirmação de pausa, saída, reconexão, fallback e
não retenção de identificadores/texto privado.

## Verificações finais

| Comando | Resultado |
| --- | --- |
| `cargo check --workspace --all-targets --locked --offline` | exit 0 |
| `cargo test --workspace --locked --offline` | exit 0; 40 testes passaram: 3 CLI, 31 core existentes, 6 novos |
| `cargo fmt --all -- --check` | exit 0 |
| `git diff --check` | exit 0 |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | exit 101; seis lints preexistentes em provider.rs |
| `cargo clippy -p limivra-core --lib --test host_controls --locked --offline -- -D warnings` | exit 0 |

Clippy estrito reproduzido em snapshot independente da base `c11cec5`, sem a
slice, com o mesmo comando e exit 101: `uninlined_format_args` nas linhas
422, 445, 456, 503 e 565 de provider.rs; `single_match` na linha 468.
Esse arquivo não foi alterado pela slice. Não foram suprimidos lints.

A suíte existente emitiu `sleep: invalid option -- 's'` em fixtures de provider,
embora todos os testes tenham passado. Esse resultado não constitui prova de
supervisão correta ou teardown sem órfãos.

## Limites, risco e rollback

Toolbar Qt, integração CLI/backend, captura de teclas, ações reais de
pausa/alternância/recarga/desconexão, Wayland/X11, resize/foco e hardware:
UNTESTED/não implementados nesta slice. Sem dependências novas, processos de
produto, acesso ao aparelho ou telemetria adicionados. Atalhos podem conflitar
com apps; o adaptador futuro deve limitar captura ao foco local e comunicar
falhas de execução. Não há aprovação de QA nem fechamento da Issue #25.

Rollback: reverter somente o commit desta slice. Não é necessária alteração
no telefone nem encerramento de processos de terceiros.
