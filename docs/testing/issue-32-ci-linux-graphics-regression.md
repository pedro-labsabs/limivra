# Issue #32 — CI Linux e regressão Wayland/X11

**Branch:** `agent/32-ci-linux-graphics-regression`
**Worktree:** `/tmp/limivra-32`
**Base isolada da slice:** `agent/31-threat-model-security` em `9cbc200f3e1332fa1dd9df865e99f5f1e7276ef5`.
**Base cumulativa da PR:** `origin/main` em `76d5294de19dafac3daa4a26b0a9ff00af85f228`.

## Escopos do diff

A slice isolada da Issue #32 é comparada com o commit `9cbc200` da Issue #31 e contém 6 arquivos: workflow de CI, badge/estado do README, detector e subcommand CLI, teste de contrato CLI e esta evidência.

A PR para `main` contém o diff cumulativo dos branches M3/M4 que ainda aguardam integração sequencial. Em relação a `origin/main` (`76d5294`), o diff cumulativo tem 59 arquivos; esse total não representa apenas a slice #32.

## Contrato de detecção e smoke

O comando `limivra-cli host-graphics-smoke` classifica variáveis de ambiente não vazias. `WAYLAND_DISPLAY` tem precedência quando ambas estão definidas; caso contrário, `DISPLAY` indica X11. Se ambas estiverem ausentes ou vazias, retorna exit 0 e diagnóstico explícito `SKIP`. Os diagnósticos não imprimem os valores das variáveis.

A presença da variável é apenas um indício de ambiente gráfico: o comando não tenta conectar a um socket Wayland nem a um servidor X11. A CI testa headless com ambas as variáveis removidas e testa X11/Wayland com valores controlados. Esses mocks validam o classificador e a saída do CLI, não iniciam servidor gráfico nem comprovam uma sessão real.

## Evidência local

Ambiente observado em 2026-10-21 no worktree `/tmp/limivra-32`:

- `rustc 1.88.0 (6b00bc388 2025-06-23)`; `cargo 1.88.0 (873a06493 2025-05-10)`.
- `CARGO_NET_OFFLINE=true cargo test --workspace --locked --offline`: exit 0; 148 testes passaram, 0 falharam; doctests: 0.
- `cargo fmt --all -- --check`: exit 0.
- `CARGO_NET_OFFLINE=true cargo clippy --workspace --all-targets --locked --offline -- -D warnings`: exit 0, sem warnings.
- `git diff --check origin/main...HEAD`: exit 0; sem erros de whitespace no diff cumulativo após o commit da slice.

Smokes executados localmente contra o binário real da CLI, com ambientes de processo controlados:

| Entrada | Saída | Resultado |
|---|---|---|
| `DISPLAY` e `WAYLAND_DISPLAY` removidos | `SKIP: host headless; WAYLAND_DISPLAY and DISPLAY are unset or empty.` | exit 0 |
| `DISPLAY=:limivra-ci-mock`, sem `WAYLAND_DISPLAY` | `DETECTED: X11 environment indicated; server connection not verified.` | exit 0 |
| `WAYLAND_DISPLAY=wayland-limivra-ci-mock` e `DISPLAY=:limivra-ci-mock` | `DETECTED: Wayland environment indicated; server connection not verified.` | exit 0 |

Os quatro testes de integração `host_graphics_smoke` passaram, cobrindo headless/skip, variáveis vazias, X11, precedência de Wayland e ausência de eco dos valores simulados. O teste unitário do classificador para valores vazios também passou.

## CI e badge

O workflow existente mantém jobs separados de Format, Clippy estrito e testes herméticos do workspace. O novo job Linux executa o CLI com variáveis gráficas removidas e com marcadores X11/Wayland simulados, exigindo os diagnósticos esperados. O badge no README aponta para o workflow oficial `ci.yml`.

O job usa apenas ferramentas Rust e mocks de ambiente; não instala Xvfb nem dependências gráficas. Isso deixa explícita a natureza sintética dos testes e mantém a execução sem telefone, emulador ou conexão de rede externa iniciada pelos testes.

## Limites, dependências e reversão

Não foi realizada prova em monitor físico, sessão Wayland/X11 real, compositor, GPU ou telefone. Um `DETECTED` deste smoke não confirma que o servidor existe ou aceita conexão; confirma somente a classificação das variáveis fornecidas. A matriz de teste não substitui as evidências físicas requeridas pela estratégia do projeto.

As Issues #20 e #30 permanecem `OPEN` no GitHub no momento desta alteração. O branch-base contém os artefatos cumulativos anteriores, mas esta slice não fecha nem altera o estado dessas issues e não reivindica conclusão de seus critérios físicos.

Reversão: reverter o commit da Issue #32 restaura o workflow, README, detector, testes e esta evidência ao estado do branch-base. Não há dependências ou configurações externas adicionadas.
