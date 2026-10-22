# Issue #33 — Packaging Linux, dependências e notices

**Branch:** `agent/33-packaging-dependencies-notices`
**Worktree:** `/tmp/limivra-33`
**Base isolada da slice:** `agent/32-ci-linux-graphics-regression` em `b234beb2809647cc5a907c855a48f388cbf857d4`.
**Base cumulativa da PR:** `origin/main` em `76d5294de19dafac3daa4a26b0a9ff00af85f228`.

## Escopo e artefatos

A slice isolada contém 9 arquivos sobre `b234beb`: `.gitignore`, `README.md`, `NOTICES.md`, os três scripts em `scripts/`, ADR-0006, o teste `packaging_contracts.rs` e esta evidência. O diff cumulativo da PR contra `origin/main` contém 66 caminhos: os 59 anteriores mais 7 arquivos novos desta slice; `README.md` e `.gitignore` já existiam no diff cumulativo.

A slice implementa ADR-0006 com comparação Flatpak/AppImage/pacotes nativos/tarball, tarball standalone x86_64, SHA-256, instalação e remoção por usuário, inventário de licenças Cargo e teste de lifecycle do pacote. O arquivo gerado é `dist/limivra-linux-x86_64.tar.gz`, acompanhado de `dist/SHA256SUMS`.

Esta revisão de QA altera quatro dos nove arquivos da slice (empacotador, teste de contrato, `NOTICES.md` e esta evidência); não adiciona arquivos à slice nem ao diff cumulativo de 66 caminhos.

O tarball contém `limivra`, `install.sh`, `uninstall.sh`, `LICENSE`, `NOTICES.md` e `README.md`. `dist/` é saída gerada e ignorada pelo Git; o PR publica o empacotador e os contratos, não o binário de release.

## Licença e dependências

`LICENSE` foi conferido como MIT, coerente com o ADR-0004 aceito. `NOTICES.md` é o inventário de compilação e triagem de expressões SPDX para os 65 pacotes de registry travados, obtido por `cargo metadata --format-version 1 --locked --offline`; 0/65 (0%) das expressões declaradas contêm GPL, AGPL ou copyleft forte, e não há `BSD-3-Clause` neste lockfile. A agregação dos textos completos de copyright dos autores individuais está rastreada para a fase pré-release de distribuição pública e ainda não é declarada completa. Algumas expressões incluem alternativas permissivas e o termo adicional `Unicode-3.0`; o documento preserva as expressões declaradas pelos pacotes.

Esta é uma triagem técnica de metadados, não uma auditoria jurídica completa de arquivos de copyright/NOTICE upstream. `adb` e `scrcpy` são ferramentas externas instaladas pelo usuário e não são redistribuídas no tarball.

## Testes e gates

O teste `packaging_contracts` executa o script com `CARGO_NET_OFFLINE=true`, destino de build e diretório `dist` temporários. Valida os seis itens do arquivo, o manifesto por `sha256sum --check`, extração, instalação em um `PREFIX` isolado com espaços, permissão `0755`, execução de `limivra --version`, desinstalação e ausência de arquivos no prefixo após a remoção. Esse lifecycle roda no host Linux de build; o `PREFIX` temporário isola os arquivos instalados, mas não simula uma distribuição Linux limpa nem valida a ABI e as bibliotecas disponíveis em outra distro.

O empacotador agora compila com `RUSTFLAGS="${RUSTFLAGS:-} --remap-path-prefix=${REPO_ROOT}=limivra"`, preservando flags Rust fornecidas pelo ambiente e remapeando o caminho absoluto da worktree para um prefixo estável. O teste de contrato também inspeciona o binário extraído e falha se encontrar nele o caminho absoluto da worktree atual.

Ambiente observado em 2026-10-21: Linux x86_64, `rustc 1.88.0`, `cargo 1.88.0`.

| Comando/ação | Resultado observado |
|---|---|
| `cargo metadata --format-version 1 --locked --offline` + inventário de SPDX | exit 0; 65 pacotes de registry; zero expressões GPL/LGPL/AGPL/MPL/SSPL |
| `CARGO_NET_OFFLINE=true cargo test --workspace --locked --offline` (baseline antes da slice) | exit 0; 148 testes passaram, 0 falharam |
| `CARGO_NET_OFFLINE=true cargo test -p limivra-cli --test packaging_contracts --locked --offline` | exit 0; teste de checksum, archive, install, `--version`, uninstall e prefixo limpo passou |
| `CARGO_NET_OFFLINE=true cargo test -p limivra-cli --test packaging_contracts --locked --offline` (regressão de caminho, antes do remapeamento) | exit 101 como esperado; o binário empacotado continha `/tmp/limivra-33` |
| `CARGO_NET_OFFLINE=true cargo test -p limivra-cli --test packaging_contracts --locked --offline` (após remapeamento) | exit 0; 1 teste passou, incluindo a verificação de ausência do caminho absoluto da worktree |
| Empacotamento de cópias em dois caminhos diferentes, com diretórios de build separados; comparação via `cmp` e SHA-256 | exit 0; archives idênticos, SHA-256 `61590e7ef7d80c8ac574f68ab0d447a671d633326f00b174c3ad9a83c7b54218` em ambos |
| `CARGO_NET_OFFLINE=true cargo test --workspace --locked --offline` (revalidação QA após remapeamento) | exit 0; 149 testes listados e passaram, 0 falharam; doctests: 0 |
| `cargo fmt --all -- --check` (revalidação QA) | exit 0 |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` (revalidação QA) | exit 0, sem warnings |
| `sh -n scripts/package-linux.sh scripts/install.sh scripts/uninstall.sh` | exit 0 |
| `scripts/package-linux.sh` | exit 0; criou `dist/limivra-linux-x86_64.tar.gz` e `dist/SHA256SUMS` |
| `sha256sum --check dist/SHA256SUMS` | exit 0; tarball corresponde ao manifesto |
| Geração repetida do tarball no mesmo host/target | os dois hashes foram `77821007c18def73b04d0fbbb5bbdeec996185f0709b7cbe26598094cb69ca49` |
| `git diff --check origin/main...HEAD` | exit 0 após o commit da slice; nenhum erro de whitespace no diff cumulativo |

O archive foi listado com exatamente seis arquivos no topo: `LICENSE`, `NOTICES.md`, `README.md`, `install.sh`, `limivra` e `uninstall.sh`. O teste usa `PREFIX` temporário com espaços e executa os scripts em ambiente limpo, sem Android, ADB, scrcpy, rede ou elevação de privilégio. Ele comprova o lifecycle no host atual, não um smoke em instalação Linux limpa; esse teste de distribuição permanece não executado.

## Limites operacionais

O pacote é Linux x86_64 e não inclui ADB, scrcpy, bibliotecas de sistema ou instalador de dependências. `ldd` no host de build mostrou dependências dinâmicas em `libgcc_s`, `libm` e `libc`; o requisito mínimo de ABI não foi validado em outra distribuição. A instalação padrão é `~/.local/bin`; um `PREFIX` customizado precisa ser gravável pelo usuário. O SHA-256 não é assinatura digital.

A Issue #20, que requer a prova USB física, permanece `OPEN`; este teste hermético não usa telefone, daemon ADB, rede ou monitor e não substitui essa evidência. A prova em distribuição Linux limpa também não foi executada. Nenhuma declaração de compatibilidade física ou de release final é feita nesta slice.
