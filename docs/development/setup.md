# Configuração do workspace Limivra

## Escopo da Issue #15

Este slice fornece um workspace Cargo mínimo, uma CLI de ajuda/versão e uma fronteira de supervisão para subprocessos locais iniciados pelo próprio núcleo. Ele não conecta a dispositivos Android nem requer ADB ou scrcpy para compilar e testar.

A opção `--new-display` do scrcpy surgiu na versão **3.0**. Uma futura utilização desse recurso depende de verificar previamente `scrcpy --help` na versão instalada e confirmar que a opção está listada. Este slice não executa esse comando.

## Ambiente

É necessário ter Rust e Cargo disponíveis em um host Linux. Os testes da supervisão usam um filho fake local; nenhum aparelho, ADB ou scrcpy é usado. Este slice não exige comandos de instalação de ferramentas externas.

## Estrutura do workspace

- `crates/limivra-core`: supervisão e teardown do processo filho que iniciou.
- `crates/limivra-cli`: apenas ajuda e versão.

## Comandos determinísticos

Execute na raiz do repositório:

```bash
rustc --version
cargo --version
cargo fmt --all -- --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo build --workspace --locked
cargo run -p limivra-cli -- --help
cargo run -p limivra-cli -- --version
```
