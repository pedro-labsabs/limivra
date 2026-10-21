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

## Revisão de QA-27-01 e QA-27-02 — 2026-10-21

Relatório independente recebido do Orquestrador: REQUEST_CHANGES sobre
`e9a3a3e`; reprodução lida em `/tmp/limivra-qa-27-evidence/audit-report.md`.
Os artefatos e a worktree do QA foram preservados. Esta seção registra reparo
e verificação de desenvolvimento, não reauditoria ou aprovação independente.

### QA-27-01: recuperação com transporte ausente/recusa transitória

O runner agora classifica como Offline somente mensagens completas reconhecidas
para o endpoint exato da requisição:

- Probe: `error: device '<endpoint>' not found`, além de `error: device offline`.
- Connect: `failed to connect to '<endpoint>': Connection refused`.

Essas classificações funcionam no caminho de exit não zero e no caminho de
resposta semântica com exit zero. Cada mensagem deve ocupar um único stream,
sem outro diagnóstico ou conteúdo adicional. Não foi adicionado match genérico
por `not found` ou `failed to connect`. Outro endpoint, razão desconhecida e
texto extra continuam falhando fechados. Unauthorized tem precedência sobre
qualquer classificação transitória e nunca habilita retry.

No controlador, um único predicado de elegibilidade mantém a admissão em
Recovering e as tentativas alinhadas: somente Offline/Timeout. A política de
consentimento, orçamento e máximo de três tentativas permanece restrita.

Regressões:

- `disappeared_target_and_exact_connection_refusal_are_recoverable` cobre alvo
  ausente e recusa em stdout exit 0 / stderr exit 1.
- `unknown_other_target_and_authorization_failures_are_not_retried_as_offline`
  cobre alvo diferente, mensagem desconhecida/adicional e Unauthorized combinado
  com texto de recusa. A API existente representa Unauthorized em resposta com
  exit zero como `Ok(AdbReply::Unauthorized)`; o controlador rejeita essa resposta.
- `actual_runner_recovers_disappeared_target_with_backoff_without_pair_replay`
  usa WifiController + PrivateAdbRunner + subprocesso falso: pair → conexão
  confirmada → desaparecimento → Recovering → duas recusas → reconexão na
  terceira tentativa. Verifica eventos de tentativas 1/2/3, quatro connects no
  total e somente um pair. Esperas reais acumulam pelo menos 7 s, com timeout de
  teste 20 s; o teste existente com relógio pausado comprova os atrasos exatos
  1/2/4 s, sem misturar relógio pausado com I/O do sistema operacional.

RED: `cargo test -p limivra-core --test wifi_runner disappeared --locked --offline`
retornou 101, dois testes falharam: CommandFailed em vez de Offline e Failed em
vez de Recovering. GREEN após reparo: mesmo comando exit 0, dois testes passaram.

### QA-27-02: preflight obrigatório em cada lançamento

Presença, arquivo regular sem symlink, proprietário e permissões da chave
`.android/adbkey` agora são revalidados em um preflight central. Todos os
comandos, inclusive help, daemon inicial/reinício, clientes e cleanup via
subprocesso, passam por esse gate imediatamente antes de serem lançados.
A verificação no início de ensure_server também cobre reutilização de daemon.

A configuração registra somente metadados da chave aprovada (device/inode,
tamanho, mtime/ctime com nanossegundos), sem bytes, hash ou derivação de chave.
Troca de arquivo/alteração em uso observada nesses metadados causa
UnsafeConfiguration: é necessário criar outro runner e efetuar novo pareamento
consentido. Não há adoção automática de identidade nova, regeneração ou store
alternativo. Isso não é selo criptográfico nem lock do OS: o consumidor deve
impedir alterações concorrentes enquanto o processo externo usa os caminhos.

Se o armazenamento se tornar inseguro, nem o cliente kill-server é lançado.
Cleanup usa kill/wait do handle existente e verifica/removerá somente seu socket
sem listener. Falha de coleta/verificação continua CleanupFailed; um cleanup
bem-sucedido conserva UnsafeConfiguration como causa da operação bloqueada.

Regressões:

- `mandatory_key_is_revalidated_after_construction_before_any_process`: remoção,
  chmod inseguro, substituição, alteração in-place e troca por diretório após
  new(); contador de lançamentos permanece ausente.
- `key_disappearing_between_help_daemon_and_client_blocks_next_launch`: remoção
  por help impede daemon; remoção antes do listener ficar pronto impede cliente.
  Nenhum cliente de cleanup é lançado sem chave; daemon próprio é recolhido.
- `key_removed_before_reuse_or_restart_denies_client_and_new_daemon`: remoção com
  daemon ativo ou após shutdown impede reutilização/reinício sem novo subprocesso.

RED: `cargo test -p limivra-core --test wifi_runner key_ --locked --offline`
retornou 101: três regressões receberam Ok(Device) em vez de UnsafeConfiguration.
GREEN após reparo: mesmo comando exit 0, quatro testes passaram (inclui um
preexistente). O log sintético de lançamentos existe somente nas fixtures,
fora do produto, e não contém código de pareamento.

### Verificação final desta revisão

| Comando | Resultado |
| --- | --- |
| `cargo test --workspace --locked --offline` | exit 0; 80 testes passaram, incluindo 18 wifi_runner (seis novos) |
| `cargo check --workspace --all-targets --locked --offline` | exit 0 |
| `cargo clippy -p limivra-core --lib --test wifi --test wifi_runner --locked --offline -- -D warnings` | exit 0 |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | exit 101; mesmos seis lints herdados em provider.rs |
| `cargo fmt --all -- --check` | exit 0 |
| `git diff --check` | exit 0 |

Registro de execução intermediária: `false_success_and_echoed_secrets_never_escape_as_errors`
retornou Io uma vez durante execução paralela dos alvos wifi/wifi_runner; a
reexecução isolada e a suíte workspace final passaram. Causa desse Io não foi
determinada; não foi suprimido nem alterado o teste para aceitar Io. Uma asserção
nova sobre Unauthorized também foi ajustada à representação tipada existente,
sem tornar autorização negada elegível para retry. Reauditoria deve considerar
a observação intermitente, além dos dois reparos funcionais.

Arquivos de produto alterados: somente wifi.rs e wifi/adb_runner.rs. Testes e
esta evidência atualizados; provider.rs preservado, sem supressões de Clippy.
Nenhum ADB/telefone/rede real foi utilizado. Gates físicos e limitações de
backend permanecem UNTESTED conforme seção anterior. Pronto para reauditoria
independente pelo limivra-qa; nenhum parecer de aprovação foi emitido pelo autor.
Rollback desta revisão: reverter somente seu commit de reparo, preservando a
entrega original, os ancestrais e os artefatos independentes de QA.
