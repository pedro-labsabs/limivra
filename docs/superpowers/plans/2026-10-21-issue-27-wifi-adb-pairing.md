# Issue #27 — plano de implementação: transporte Wi-Fi ADB

**Estado:** aprovado na íntegra pelo Orquestrador em 2026-10-21; implementado
com evidência de desenvolvimento. Auditoria independente PENDENTE.
**Objetivo:** pareamento e conexão consentidos de um alvo local explícito, com
recuperação limitada, observabilidade sanitizada e testes determinísticos.
**Executor:** limivra-dev, nesta sessão; auditoria independente pelo limivra-qa
convocado pelo Orquestrador, após a futura entrega.
**Stack:** Rust 2021, Tokio e std já presentes; sem dependências novas previstas.
**Base:** `cc314c0325ab86a5fe616aaf8faa5bdf403c6f28`.
**Branch/worktree:** `agent/27-wifi-adb-pairing`, `/tmp/limivra-27`.
**Spec:** instruções do Orquestrador para #27, PRD FR-07/08/09,
`docs/architecture/system.md`, ADR-0001/0003 e estratégia de testes.

## Prontidão e escopo

Issue #27 OPEN, M3, `status:needs-evidence`; dependências #18 e #26 OPEN na
consulta de 2026-10-21. A base dependente foi autorizada, mas não constitui
aprovação independente desses gates. Arquitetura geral continua PROPOSED.

Implementar biblioteca de transporte e runner de subprocessos ADB injetável,
com alvo manual. Não implementar UI, descoberta mDNS, Wi-Fi legado `adb tcpip`,
configuração do telefone, integração automática com scrcpy, acesso ao aparelho,
instalação de software ou E2E nesta slice. Testes não usam ADB real nem rede.

As decisões abaixo são propostas para aprovação; não são alegações de suporte
já implementado. Após aprovação: usar execução inline com TDD, sem autoauditoria.

## Contratos e divisão de arquivos

- Criar `crates/limivra-core/src/wifi.rs`: endpoint, consentimento, código
  transitório, estados/eventos, controlador e política de recuperação.
- Criar `crates/limivra-core/src/wifi/adb_runner.rs`: execução sem shell, ambiente
  controlado, stdin sensível, limites de saída, timeout/cancelamento e reap.
- Alterar `crates/limivra-core/src/lib.rs`: exportar o módulo.
- Criar `crates/limivra-core/tests/wifi.rs`: mock injetável de runner e relógio
  Tokio pausado; verificar comandos, estados e resultados observáveis.
- Criar `crates/limivra-core/tests/wifi_runner.rs`: executáveis ADB falsos
  em diretórios temporários para provar comportamento real do runner.
- Criar `docs/testing/issue-27-wifi-adb-pairing.md`: comandos, versões,
  RED/GREEN, limitações, pendências e rollback; atualizar estado deste plano.

Interfaces públicas propostas:

- `LocalEndpoint::parse(&str) -> Result<LocalEndpoint, WifiError>`.
- `PairingCode::parse(&str) -> Result<PairingCode, WifiError>`: tipo opaco,
  consumido no pairing; sem Clone, Serialize ou Debug que revele conteúdo.
- `WifiController<R: AdbRunner>::new(runner, allowed_target: LocalEndpoint)`.
- `pair(pair_endpoint, code, consent)`, `connect(consent)`, `check_link()`,
  `recover(consent)` e `disconnect()` assíncronos, com estado consultável e
  emissão de `WifiEvent`. `cancel()` invalida operação, mata/recolhe filho
  próprio e cancela timers antes de retornar.
- `WifiState::{Disconnected, Pairing, Connecting, Connected, Recovering, Failed}`;
  `WifiError`/eventos com categorias fixas, nunca saída ADB ou segredos.
- `AdbRunner::run(request, deadline, cancellation)` retorna futuro de resultado
  tipado; `AdbRequest` diferencia Pair/Connect/Probe/Disconnect e carrega endpoint
  validado. Código segue canal privado de stdin, não argumentos públicos.

A implementação deve escolher assinaturas Rust compiláveis desses contratos
sem mudar seu significado; nenhum objeto de consentimento é criado por default.

## Endereços e rede local

Primeira slice: IPv4 literal RFC1918 (10/8, 172.16/12, 192.168/16), porta
explícita decimal 1..65535. IPv6 e nomes DNS ficam UNSUPPORTED nesta slice;
essa limitação deve aparecer no erro/documentação, sem resolução ou fallback.
Aceitar apenas forma canônica `a.b.c.d:port`; rejeitar espaços, controles,
newlines, sinais, zeros à esquerda ambíguos, URL, usuário/senha, shell syntax,
flags, ausência de porta, porta zero, overflow e dados após a porta.

Rejeitar IP público, loopback, unspecified, multicast, broadcast limitado,
link-local e formas IPv4-mapped/abreviadas. RFC1918 não prova adjacência física
nem confiança: exigir seleção/consentimento do endpoint exato. Não fazer scan,
DNS, escolha do primeiro device, fallback para USB ou troca silenciosa de alvo.
Não inferir broadcast dirigido por último octeto sem conhecer máscara de rede.

Guardar somente endpoints autorizados em memória. Pair e connect têm portas
independentes, fornecidas pelo usuário, mas devem ter o mesmo IP autorizado.
Alvo novo, IP/porta de conexão alterado, falha de autenticação ou autorização
revogada exigem nova ação humana; recovery nunca tenta endereço alternativo.

## Pareamento, credenciais e servidor ADB

Usar equivalente funcional de `adb pair <ip>:<pair-port> <code>`, porém executar
`adb pair <ip>:<pair-port>` com código e newline em stdin privado. AOSP confirma
esse caminho; evitar exposição do código em argv/process list. Validar seis
dígitos ASCII, sem trim silencioso ou eco; não registrar tamanho/derivações.
Descartar código e saída sensível após a operação, sem arquivo, env, Debug,
tracing, replay ou retenção no estado. Não prometer apagamento físico de todas
as cópias dentro do ADB/OS; não manter buffer secreto na aplicação após retorno.

Consentimento explícito distingue pairing, connect e recovery habilitado por
sessão. Pair nunca é repetido automaticamente. Pairing bem-sucedido retorna a
Disconnected com registro transitório do alvo pareado; não implica Connected.
Connect exige sucesso de pairing nesta instância e o alvo previamente autorizado;
conexões pré-pareadas herdadas serão recusadas nesta primeira slice.

O ADB administra chaves/registro de pareamento próprios: não alegar que o
protocolo é sem persistência. A implementação não copia chaves nem administra
senhas. Usar armazenamento ADB do usuário explicitamente aprovado e protegido;
se ausente/inseguro, falhar antes da operação, sem criar armazenamento alternativo.
Disconnect não é unpair e não remove confiança/chaves do telefone. Revogação
permanente é ação manual no Android, fora do escopo e sem execução automática.

Para não depender de um daemon compartilhado com bind/auto-connect desconhecidos,
runner de produção deve usar instância própria de ADB com socket Unix privado
no Linux e processo em foreground sob supervisão. Diretório 0700, socket sem
acesso público; falhar em socket preexistente não pertencente à instância.
Verificar suporte das opções com fixtures antes de fixar a invocação na implementação;
se versão instalada não suportar esse contrato, DependencyUnsupported, sem fallback.
Não usar `-a`, servidor TCP em 0.0.0.0/::, `forward`, `reverse`, `tcpip`, firewall,
mDNS discovery ou anúncios implementados pelo Limivra. Desabilitar auto-connect
mDNS na instância própria e testar a configuração aceita pelo backend escolhido.

Subprocessos recebem ambiente mínimo: caminho absoluto de adb aprovado, locale
fixo, armazenamento aprovado e configuração explícita de socket/auto-connect.
Não herdar ADB_TRACE, ANDROID_SERIAL, ADB_SERVER_SOCKET, ANDROID_ADB_SERVER_*,
ADB_VENDOR_KEYS ou redirecionamentos de servidor do ambiente pai sem validação.
Nunca encerrar/reconfigurar servidor ADB de outra aplicação ou killall.
Apenas o daemon privado e clientes próprios são encerrados e recolhidos.
E2E futuro deve observar sockets reais para comprovar ausência de bind/anúncio
público do ADB; mocks não certificam configuração real do software externo.

## Estados, confirmação e desconexão

| Origem/evento | Destino e comportamento |
| --- | --- |
| Disconnected + pair consentido | Pairing; sucesso validado → Disconnected com alvo pareado; erro → Failed |
| Disconnected + connect consentido | Connecting; sucesso semântico + probe autorizado → Connected; falha → Failed |
| Connected + probe offline/ausente/timeout | Recovering se recovery foi habilitado; caso contrário Failed |
| Recovering + tentativa confirmada | Connected, com aviso de recuperação |
| Recovering + limite/autorização perdida | Failed, sem novos comandos |
| cancel/disconnect | invalidar operação/timers; disconnect específico; confirmar teardown ou reportar Failed |

Só marcar Connected após resposta de connect reconhecida E
`adb -s <endpoint> get-state` retornar `device`. Offline/unauthorized/unknown,
stdout vazio e exit 0 com mensagem `failed to connect` não são sucesso.
Usar locale fixo e parser conservador: mensagem desconhecida → falha tipada.
Pair exige confirmação reconhecida, não apenas exit code 0. Erros não vazam
endpoint, GUID, código, stdout/stderr ou detalhes do ambiente.

Executar sempre `adb disconnect <endpoint>` específico e nunca disconnect sem
alvo. Disconnect idempotente sem conexão não inicia daemon nem toca outros alvos.
Na falha/timeout de disconnect, parar clientes/daemon privados e reportar estado
Failed/cleanup pendente em vez de declarar limpeza completa indevidamente.
Cancelar pairing não desfaz confiança que já possa ter sido gravada pelo ADB.

Uma operação por controlador; rejeitar chamadas concorrentes com Busy. Geração
monotônica invalida respostas tardias após cancel/revoke/novo alvo, de modo que
nenhum callback antigo possa ressuscitar Connected. Usar ownership/cancelamento
explícitos; timeout exige kill E wait do filho, não só abandono do futuro.

## Queda de link e recuperação limitada

Monitoramento iniciado explicitamente pelo consumidor, sem daemon de aplicação
oculto. Enquanto Connected, probes a cada 5 s, um por vez, deadline 2 s. Parar
monitor ao disconnect/cancel e fornecer notificação para consumidor liberar
input/foco via contrato da #26; não religar input ou clipboard automaticamente.
A integração UI/scrcpy/input não faz parte desta slice.

Pair timeout 30 s; connect 5 s; disconnect 5 s. Recovery opt-in limitado a
três tentativas com atrasos determinísticos de 1, 2 e 4 s, sem jitter nesta slice,
e deadline total de 30 s (inclui esperas, connect e confirmação). Usar relógio
monotônico; reduzir timeout de cada operação ao saldo restante. Nunca aguardar
além do deadline nem manter retry pendente após disconnect/revogação.

Recuperar apenas o endpoint de conexão já autorizado, sem replay do código,
novo pair ou busca de IP/porta. Unauthorized/rejeição de confiança interrompe
imediatamente; timeout/offline pode consumir orçamento. Após esgotamento → Failed;
novo ciclo exige ação humana. Sem loops infinitos ou retries implícitos do core.

## Plano TDD e revisão de entradas críticas

1. **Validação e privacidade:** escrever testes RED de endpoints válidos e
   rejeitados, overflow/injeção/Unicode/IPv6 e IP público. Implementar parser
   mínimo; GREEN. Testar portas separadas de pair/connect e mesmo IP obrigatório.
2. **Pareamento/runner:** RED com mock que verifica argv sem código, stdin
   privado e ausência de persistência. Implementar runner e parser; GREEN.
   Fixtures reais de subprocesso falso devem ecoar segredo em stdout/stderr para
   demonstrar que erro/evento/Debug público não o repassa. Saída limitada a
   8 KiB por stream; excesso → OutputLimit com kill/reap, sem log bruto.
3. **Ciclo de vida:** RED para cada linha da tabela, no consent, Busy,
   mensagens desconhecidas, exit 0 falso, probe não-device e disconnect específico.
   Implementar controlador mínimo; GREEN. Mock prova comandos esperados, não
   substitui asserções de estado/eventos nem cria sucesso físico fictício.
4. **Timeout/cancelamento:** RED com executável falso bloqueante; comprovar
   kill/reap, cancel durante pairing/connect/probe, interrupção de stdin e nenhuma
   mudança tardia de estado. GREEN; não matar servidores externos de teste.
5. **Recovery:** relógio Tokio pausado; RED em atrasos 1/2/4, máximo três
   tentativas, deadline de 30 s, autorização perdida, cancel durante backoff e
   resposta atrasada após disconnect. GREEN sem sleeps reais de segundos.
6. **Isolamento:** fixtures de env hostil e socket ocupado/sem suporte; provar
   que não há shell, endpoint alternativo, broadcast ou servidor compartilhado.
   Daemon falso deve confirmar configuração privada e encerramento por ownership.
7. **Verificações finais:** `cargo check --workspace --all-targets --locked --offline`,
   `cargo test --workspace --locked --offline`, `cargo fmt --all -- --check`,
   `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`,
   `git diff --check`. Se persistirem os seis lints herdados de provider.rs,
   reportar e rodar Clippy estrito dos novos alvos sem supressão/refatoração lateral.
8. **Entrega futura, após aprovação e execução:** registrar versões/resultados
   reais e limitações, commit/push somente da slice e PR com `Refs #27` para main,
   ancestralidade explícita, risco/rollback e auditoria independente pendente.

## Critérios da slice, pendências e rollback

Aceite técnico local: testes comprovam comandos, consentimento, política de alvo,
segredos fora de argv/logs/estado, estados confirmados, limites, cancelamento e
cleanup do runner usando fakes. Sem root ou interação não autorizada com aparelho.

E2E Wi-Fi real, persistência efetiva do ADB, recuperação física, certificado/alvo,
sockets/anúncios reais, integração scrcpy e consumidor de eventos: UNTESTED até
experimento autorizado e QA independente. Código unitário não fecha a #27.

Rollback: reverter somente commits desta slice após a futura entrega; encerrar
somente processos/socket privados próprios. Não apagar chaves ADB, desfazer
pareamento no telefone ou alterar rede sem autorização humana específica.

## Fontes oficiais consultadas (2026-10-21)

- Manual ADB 1.0.41: https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/docs/user/adb.1.md
  (pair/connect/disconnect, bind padrão e variável de auto-connect mDNS).
- Cliente AOSP, blob `5cd3ab66dd0a7ba0173b01f954fbe76c21269483`:
  https://android.googlesource.com/platform/packages/modules/adb/+/refs/heads/main/client/commandline.cpp
  (pair sem terceiro argumento lê código por stdin). Não foi copiado código.

## Registro de execução e decisões de implementação

- Execução inline com RED/GREEN; nenhum auditor/subagente foi convocado pelo
  implementador. Resultados: `docs/testing/issue-27-wifi-adb-pairing.md`.
- Decisão: consentimentos tipados Granted/Denied; connect recebe separadamente
  autorização de conexão e habilitação de recovery. Monitor é futuro explícito
  e retorna na queda; recover é nova chamada consentida, sem tarefa escondida.
- Decisão: clientes ADB podem reiniciar o daemon local. Cleanup usa também
  `-L <socket Unix próprio> kill-server`, seguido de kill/wait do handle próprio
  e verificação de listener encerrado antes de remover o socket. Nunca usa
  kill-server no socket padrão/compartilhado. Custo se backend divergir:
  CleanupFailed e necessidade de validação E2E antes de uso em aparelho.
- Decisão: cleanup/reap continua obrigatório após timeout; o deadline limita
  a operação/retries, não permite abandonar processos ainda não recolhidos.
- A fonte AOSP confirmou ADB_MDNS=0, ADB_MDNS_AUTO_CONNECT=0, ADB_USB=0 e
  ADB_EMU=0 para impedir descoberta/conexão automática na instância dedicada.
  Isso não substitui inspeção real da versão/binário configurados.
