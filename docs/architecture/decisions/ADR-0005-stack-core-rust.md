# ADR-0005 — Avaliação de Stack: Rust para o Núcleo e Desacoplamento de UI

**Status:** PROPOSED (2026-10-21) — Resposta ao SPIKE [#11](https://github.com/pedro-labsabs/limivra/issues/11)

## Contexto

A proposta inicial do Limivra sugeria uma combinação de **Qt 6 / QML + Rust + scrcpy + ADB**. Antes de estruturar o workspace de código ([#15](https://github.com/pedro-labsabs/limivra/issues/15)), foi realizada uma auditoria de viabilidade e dependências no host Linux de referência.

### Evidências Coletadas no Host (Ubuntu Noble x86_64)

1. **Toolchain Rust:**
   - `rustc 1.88.0` / `cargo 1.88.0` instalado e operacional no usuário local (`~/.cargo/bin/`).
   - Suporte maduro a concorrência assíncrona (`tokio`), controle determinístico de subprocessos e manipulação segura de sockets e streams de texto.
2. **Backends de Sistema (ADB e scrcpy):**
   - Ambos operam como binários externos invocados via subprocesso ([ADR-0001](ADR-0001-scrcpy-external.md)).
   - O recurso de telas virtuais independentes (`--new-display`) requer `scrcpy >= 2.2`.
3. **Custos e Atritos Avaliados de Qt6 / QML com Rust:**
   - `qt6-base-dev` não está instalado por padrão e exige compilação pesada com C++, além de bindings FFI complexos (`cxx-qt` ou `qmeta-object`), gerando sobrecarga de build e atrito de empacotamento no Linux.
   - Alternativas avaliadas:
     - **Rust nativo puro para Sessão/Daemon:** O núcleo gerencia ADB, subprocessos scrcpy e IPC sem acoplamento de UI gráfica.
     - **UI desacoplada via IPC (JSON-RPC / UNIX socket / D-Bus):** Permite construir interfaces em Slint, Iced ou Qt sem acoplamento rígido de compilação C++/Rust.

## Decisão Proposta

1. **Rust como Linguagem Primária do Núcleo (`limivra-core`):**
   - O núcleo do sistema, descoberta de dispositivos ADB, ciclo de vida de subprocessos (`scrcpy`) e gestão de telas secundárias serão desenvolvidos exclusivamente em Rust puro.
2. **Desacoplamento Estrito entre Núcleo e Interface:**
   - O `limivra-core` expõe contratos limpos de API (evitando dependências C++ de Qt no crate principal de sessão).
   - O M1 (MVP de sessão USB) focará na biblioteca e CLI determinística em Rust antes de adicionar complexidade de interface gráfica pesada.
3. **Detecção e Validação de Versão do scrcpy:**
   - O daemon/core deve verificar `scrcpy --version` em runtime e rejeitar graciosamente versões `< 2.2` para o recurso de display virtual, fazendo fallback explícito para espelhamento seguro ou emitindo orientação ao usuário.

## Consequências

- **Positivas (+):**
  - Compilação limpa, determinística e veloz via `cargo build` e `cargo test`, sem necessidade de bibliotecas C++ Qt no pipeline inicial de CI.
  - Testabilidade completa via testes unitários e de integração mockando subprocessos sem servidor gráfico (headless).
  - Distribuição facilitada (binário único estaticamente linkado para as dependências Rust).
- **Negativas (-):**
  - Interfaces visuais ricas exigirão definir o protocolo de IPC ou selecionar um framework de UI em Rust leve para fases posteriores (M3).

## Reabrir quando
- Houver necessidade de janela gráfica embutindo widgets complexos de terceiros que exijam explicitamente toolkit Qt6.
