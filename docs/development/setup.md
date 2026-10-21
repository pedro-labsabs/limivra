# Guia de Configuração e Ambiente — Limivra

Este documento descreve como configurar o ambiente de desenvolvimento e executar o build do Limivra no Linux.

## Pré-requisitos do Sistema

1. **Rust Toolchain:**
   - Versão mínima recomendada: `1.80+` (testado e validado com Rust `1.88.0`).
   - Para instalar ou atualizar: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`

2. **Dependências Externas em Runtime (Subprocessos):**
   - **ADB (`adb`):** Necessário para detecção, autorização e encaminhamento de portas (`apt install adb`).
   - **scrcpy (`scrcpy`):** Utilizado como backend de vídeo e injeção de input.
     - *Requisito para Display Virtual Independente:* `scrcpy >= 2.2`.

## Estrutura do Workspace

O projeto é organizado como um Cargo Workspace em Rust:

- `crates/limivra-core`: Biblioteca do núcleo de sessão, orquestração de subprocessos e abstração de displays.
- `crates/limivra-cli`: Interface CLI para controle, diagnóstico (`doctor`) e inicialização de sessões.

## Comandos Determinísticos de Build e Teste

Execute estes comandos a partir da raiz do repositório:

```bash
# Verificar compilação de todos os crates
cargo check

# Executar a suíte de testes unitários e de integração
cargo test

# Executar o diagnóstico de ferramentas do ambiente
cargo run -- doctor

# Iniciar sessão básica em modo espelhamento
cargo run -- start

# Iniciar sessão com display virtual independente
cargo run -- start --virtual-display
```
