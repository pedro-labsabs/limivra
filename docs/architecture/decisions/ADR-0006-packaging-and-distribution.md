# ADR-0006 — Empacotamento e distribuição Linux

**Status:** ACCEPTED (2026-10-21) — formato de distribuição selecionado para a primeira entrega Linux.
**Issue:** [#33 — Packaging Linux, dependências e notices](https://github.com/pedro-labsabs/limivra/issues/33)

## Contexto

O primeiro pacote Linux precisa ser verificável, reversível e instalável pelo usuário sem `sudo` ou alteração de arquivos do sistema. A aplicação ainda invoca `adb` e `scrcpy` como ferramentas externas do host; o primeiro pacote não deve incorporá-las nem presumir que estão instaladas. O artefato deve carregar a licença do Limivra e o inventário de licenças Cargo.

## Alternativas

| Formato | Benefícios | Custos e limites para o Limivra |
|---|---|---|
| Flatpak | Integra sandbox, runtime e atualização pelo ecossistema Flatpak. | Por padrão, o sandbox não tem acesso à rede, a nós de dispositivo nem a arquivos do host. O ADB no host precisa alcançar o daemon/local endpoint e o telefone USB precisa ser exposto; isso exige permissões explícitas de rede, filesystem/socket ou USB/portal. A concessão de permissões reduz o isolamento e varia conforme a instalação. |
| AppImage | Um arquivo executável pode conter a aplicação e seu AppDir e pode ser distribuído sem instalação via gerenciador de pacotes. | O runtime Type 2 monta SquashFS por FUSE; FUSE pode não estar configurado no host. Para reduzir incompatibilidade, ainda exige decisão sobre bibliotecas do sistema e testes por distribuição. Não resolve descoberta de `adb`/`scrcpy` externos nem atualizações automáticas. |
| Pacotes nativos (`.deb`, `.rpm`, etc.) | Integra dependências, metadados, atualização e remoção aos gerenciadores da distribuição. | Exige receitas, dependências, assinatura/publicação e validação por família de distribuição; os nomes de pacotes e ABIs disponíveis variam. O caminho de instalação do sistema normalmente é administrado pelo gerenciador, enquanto o primeiro requisito Limivra é uma instalação uniforme e sem root. |
| Tarball standalone | Distribuição simples de um arquivo versionado, checksum verificável e scripts user-space para `~/.local/bin` ou `$PREFIX/bin`; sem sandbox ou montagem FUSE. | Não instala dependências do sistema, não atualiza automaticamente e o binário continua sujeito à ABI do host. O pacote inicial é somente Linux x86_64; o usuário precisa fornecer `adb` e `scrcpy` no `PATH`. |

As limitações do Flatpak decorrem do modelo de permissões do sandbox: a documentação oficial descreve ausência padrão de rede, arquivos do host e dispositivos, e expõe permissões específicas para USB e caminhos do host ([Sandbox Permissions](https://docs.flatpak.org/en/latest/sandbox-permissions.html)). A documentação do AppImage descreve o runtime Type 2 baseado em SquashFS/FUSE e seus requisitos de FUSE ([Architecture](https://docs.appimage.org/reference/architecture.html), [FUSE troubleshooting](https://docs.appimage.org/user-guide/troubleshooting/fuse.html)). O empacotamento nativo requer metadados e dependências específicos do ecossistema de pacotes, como ilustrado pelo [Packaging FAQ do Debian](https://wiki.debian.org/PackagingFAQ).

## Decisão

Distribuir a primeira entrega como `limivra-linux-x86_64.tar.gz`, acompanhado de `dist/SHA256SUMS`. O arquivo contém o executável renomeado para `limivra`, `install.sh`, `uninstall.sh`, `LICENSE`, `NOTICES.md` e `README.md`.

O script `scripts/package-linux.sh` compila em modo release com `cargo build -p limivra-cli --release --locked --offline`, monta o diretório de distribuição e gera o SHA-256 do tarball. O timestamp, ordem e ownership do arquivo tar são fixados para tornar o contêiner do pacote estável para a mesma entrada e ferramenta. A reprodutibilidade byte a byte do binário entre toolchains ainda não é um compromisso de release.

`scripts/install.sh` instala o binário com modo `0755` em `$PREFIX/bin/limivra`; sem `PREFIX`, usa `~/.local/bin/limivra`. `scripts/uninstall.sh` remove somente esse binário e tenta remover `bin` quando estiver vazio. Os scripts nunca invocam `sudo`, não alteram o PATH do usuário e não removem outros arquivos. Prefixos não graváveis falham como qualquer operação de arquivo sem permissão.

O checksum detecta corrupção ou divergência em relação ao arquivo `SHA256SUMS` recebido junto com o tarball; sem assinatura digital, ele não autentica o autor nem protege contra substituição conjunta do arquivo e do manifesto. `adb`, `scrcpy`, glibc e ferramentas do sistema não são incorporados. A matriz de hosts e a prova USB física continuam gates independentes.

## Consequências

- A instalação e remoção podem ser exercitadas em um `PREFIX` temporário sem root e sem tocar no restante do host.
- O pacote fornece notices de todas as 65 dependências de registry no `Cargo.lock` desta slice. A auditoria é baseada em metadados SPDX de Cargo e precisa ser refeita quando o lockfile mudar; não constitui parecer jurídico.
- A Issue #20 (prova USB física) permanece aberta. Este pacote não prova funcionamento USB, compatibilidade em distribuições adicionais ou prontidão para release.
- Formatos Flatpak, AppImage e pacotes nativos podem ser reconsiderados por ADR futura se a necessidade de sandbox, gestão de atualizações ou integração de distro superar os custos documentados.
