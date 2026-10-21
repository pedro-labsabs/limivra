# Issue #27 — evidência de desenvolvimento: transporte Wi-Fi ADB

Refs #27. Plano aprovado na íntegra pelo Orquestrador em 2026-10-21.
Implementador: limivra-dev. Auditoria independente limivra-qa: PENDENTE.

## Ambiente e base

Rust 1.88.0, Cargo 1.88.0; Linux Mint 22.3; Python 3 do host apenas como
executável de fixture. Worktree `/tmp/limivra-27`, branch
`agent/27-wifi-adb-pairing`, base `cc314c0325ab86a5fe616aaf8faa5bdf403c6f28`.
Os doze commits ancestrais ainda fora de origin/main não pertencem à slice #27.

Somente biblioteca core, exportação, dois arquivos de testes, plano e evidência.
Sem dependências novas, alteração de CLI/Qt/scrcpy/input, acesso ao telefone,
ADB real, pareamento real, tráfego TCP ou consentimento presumido do aparelho.
Os fakes abrem exclusivamente sockets Unix em diretórios de fixture 0700.

## Comportamento implementado

- Endpoint IPv4 canônico RFC1918 e porta explícita 1..65535. Rejeita DNS,
  IPv6, públicos, loopback, link-local, zero/overflow, whitespace/Unicode/shell,
  URLs e formatos ambíguos. RFC1918 não prova mesma LAN; alvo exige seleção
  explícita. Pair tem porta independente, mas IP deve ser o alvo autorizado.
- Código opaco de seis dígitos ASCII, sem Clone/serialização, Debug redigido.
  É consumido e enviado ao stdin do cliente, nunca em argv/env/logs/arquivos.
  A aplicação descarta o código e buffers ao terminar; não promete apagamento
  físico de cópias internas do processo ADB/OS ou do input original do chamador.
- Consentimento separado de pair, connect e habilitação de recovery; pair não
  é repetido automaticamente. Somente pair confirmado nesta instância habilita
  connect. Connected exige mensagem reconhecida e probe `device` do alvo exato.
- Estados/eventos tipados: Disconnected, Pairing, Connecting, Connected,
  Recovering e Failed. Eventos/erros públicos não contêm IP, GUID, código ou
  saída bruta. Operações concorrentes retornam Busy. Cancel invalida geração
  e token, espera operação/monitor e cleanup antes de retornar.
- Monitor é chamada explícita, probes a cada 5 s com prazo 2 s e retorna na
  queda. Emite LinkLost, sem religar input/clipboard ou fazer recovery oculto.
- Recovery explicitamente autorizado: até três tentativas, atrasos 1/2/4 s,
  orçamento total 30 s, connect <=5 s e confirmação <=2 s limitados pelo saldo.
  Pair <=30 s, disconnect <=5 s. Offline/timeout permitem tentativa limitada;
  Unauthorized ou resposta desconhecida interrompem. Nunca muda endpoint.
- Disconnect sempre específico, idempotente sem contato, e revoga registro
  transitório/grants. Não desfaz pareamento persistente no Android/ADB.

## Runner e isolamento

`PrivateAdbRunner::new` exige binário absoluto aprovado/executável sem escrita
de grupo/outros, HOME e runtime privados 0700 pertencentes ao usuário, store
`.android` existente/protegido e `adbkey` existente. Rejeita symlinks e arquivos
públicos no store; não cria/copia credenciais, não corrige permissões, não
limpa arquivos de outros processos. O chamador deve aprovar esses caminhos.

Foreground ADB dedicado com `-L localfilesystem:<runtime>/adb.sock server nodaemon`.
Socket preexistente é recusado. Ambiente limpo com HOME aprovado, LC_ALL=C,
ADB_MDNS=0, ADB_MDNS_AUTO_CONNECT=0, ADB_USB=0, ADB_EMU=0. Sem shell, mDNS
implementado pelo Limivra, daemon TCP compartilhado, forward/reverse ou tcpip.
Help precisa anunciar -L, pair e nodaemon; incompatibilidade falha fechada.
A presença dessas strings não certifica a implementação de flags/env de todo
binário/versão: cabe ao gate E2E validar o backend efetivamente configurado.

Um supervisor Tokio possui clientes mesmo se o futuro do chamador for descartado.
Timeout/cancelamento/limite de saída causam kill e wait. Captura limitada a
8192 bytes por stream; stdout/stderr não saem do runner. Daemon em foreground
usa stdout/stderr nulos; não é lançado em modo de daemonização/log padrão.

Cleanup também solicita `kill-server` SOMENTE no socket Unix privado: clientes
ADB podem reiniciar daemon local. Depois recolhe handle próprio, verifica que
não há listener no socket e remove apenas seu socket. Se não comprovar teardown,
retorna CleanupFailed. Reap/cleanup não são abandonados ao vencer deadline.
Não há killall ou alteração do daemon compartilhado do host.

A fonte consultada indica que clientes ADB podem fazer autostart/version-check.
A configuração restringe esse backend ao socket privado e desliga auto-connect;
os testes não certificam ausência de forks/logs inesperados em ADB real. Exigir
inspeção E2E de processos/sockets/arquivos antes de aprovar operação física.
Drop do runner sem shutdown não substitui cleanup assíncrono: o consumidor deve
aguardar controller.cancel/disconnect ou runner.shutdown antes de descartar.

## TDD observado

1. `cargo test -p limivra-core --test wifi --locked --offline`:
   RED inicial exit 101, dois testes falharam com parsers esqueleto; GREEN após
   validação estrita de endpoints e códigos.
2. RED exit 101: cinco testes de controlador falharam por aceitar conexão sem
   pair/consentimento e não projetar estados/recovery. GREEN: sete testes.
3. `cargo test -p limivra-core --test wifi_runner --locked --offline`:
   RED exit 101, quatro testes falharam com runner esqueleto; GREEN após runner
   privado, stdin, parsers, limites e supervisão.
4. Expansão para concorrência/cancelamento/monitor e orçamento com relógio
   pausado: RED para disconnect que confiava no deadline do backend; retornava
   UnexpectedReply após 60 s virtuais. GREEN após timeout de 5 s no controlador.
5. RED: teste de parada pelo socket próprio não observava a solicitação de
   parada; GREEN após cleanup dirigido ao daemon privado.
6. RED: daemon morto durante Connect ainda produzia Connected; GREEN após
   confirmação de que o handle de daemon permanece vivo antes de aceitar reply.
7. RED: stderr `error: device offline` virava CommandFailed; GREEN após
   classificação Offline para política de recuperação. Unauthorized permanece
   interrupção de autorização, sem retry.

Outros casos verdes: cancel em pair/connect/probe com fakes bloqueantes; aborto
externo do futuro; saída/erro com segredo sintético sem vazamento público;
limite em stdout e stderr; store inseguro/symlink; socket preexistente preservado;
backend sem suporte; ambiente hostil em processo de teste isolado; PIDs de
clientes/daemon fakes recolhidos e socket removido. Nenhum segredo real usado.

## Verificação final

| Comando | Resultado |
| --- | --- |
| `cargo check --workspace --all-targets --locked --offline` | exit 0 |
| `cargo test --workspace --locked --offline` | exit 0, 74 testes passaram: 3 CLI, 31 core, 7 host_controls, 9 input_policy, 12 wifi, 12 wifi_runner |
| `cargo clippy -p limivra-core --lib --test wifi --test wifi_runner --locked --offline -- -D warnings` | exit 0 |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | exit 101; seis lints herdados de provider.rs |
| `cargo fmt --all -- --check` | exit 0 |
| `git diff --check` | exit 0 |

Lints herdados: uninlined_format_args em provider.rs:422,445,456,503,565;
single_match em provider.rs:468. Provider e arquivos herdados não alterados;
sem supressões. Resultados anteriores na evidência #25/#26 da própria base.

## Fontes de protocolo e pendências

Consultas de leitura ao AOSP, sem copiar implementação:

- https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/client/commandline.cpp — pair lê stdin se código não veio em argv; kill-server dirigido ao socket configurado.
- https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/adb_mdns.cpp — valor 0 desabilita auto-connect (string vazia não é garantia).
- https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/client/mdns_utils.cpp — ADB_MDNS=0 desabilita backend mDNS.
- https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/client/main.cpp — foreground, bind configurado, ADB_USB/ADB_EMU.
- https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/client/adb_client.cpp — version-check/autostart de clientes; motivação para shutdown pelo socket próprio.

UNTESTED: pairing real, backend/versão ADB instalado, armazenamento efetivo de
chaves e known-hosts, revogação no Android, queda/recuperação física, Wi-Fi em
LAN, ausência observada de listeners/anúncios públicos/forks/logs reais,
integração scrcpy/UI/input. A aplicação não garante privacidade/isolamento de um
binário ADB arbitrário ou adulterado. Sem promessa de suporte IPv6/DNS.

O implementador não auditou sua própria entrega. A Issue #27 não foi fechada;
próximo gate: convocação de limivra-qa pelo Orquestrador com contexto limpo.
Rollback: reverter somente o commit desta slice; encerrar apenas recursos
privados próprios, sem apagar chaves nem desfazer confiança no telefone.
