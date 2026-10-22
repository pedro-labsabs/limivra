# SPIKE-028 — janelas Android livres no display virtual

**Estado:** aprovado pelo Orquestrador e executado em 2026-10-21; finding FAIL
no cenário de duas freeform, com floating individual observado. Auditoria PENDENTE.
**Data:** 2026-10-21. **Issue:** #28, OPEN, M3, status:needs-evidence.
**Base:** `5d47205481a740b33fa033c7287c4fec0a272c80`.
**Worktree:** `/tmp/limivra-28`.
**Branch:** `agent/28-freeform-windowing-spike`.
**Executor:** limivra-dev; auditoria independente reservada ao limivra-qa.

## Pergunta, hipótese e resultado esperado

Verificar se o Xiaomi Redmi Note 13 4G / HyperOS / Android 15 indicado pelo
Orquestrador permite duas tarefas Android simultaneamente visíveis, móveis e
redimensionáveis no MESMO display virtual secundário, efetivamente em
`WINDOWING_MODE_FREEFORM = 5`; ou se mantém uma tarefa em tela cheia por vez,
converte o modo solicitado ou rejeita a operação por política OEM.

Hipótese H1: o modo 5 é respeitado e duas janelas coexistem com foco e resize.
Hipótese alternativa H0: launch aceito não produz esse comportamento, ou o
sistema o bloqueia. Nenhuma hipótese é assumida como resultado.

MIRROR, VIRTUAL_DISPLAY e DESKTOP_WINDOWING são capacidades distintas. Duas
janelas scrcpy do Linux, alternar apps em fullscreen, split-screen ou apenas
mostrar `am start` exit 0 não demonstram H1.

A dependência #24 continua OPEN na consulta atual. SPIKE-001/capability-matrix
registram display virtual e ausência de freeform observada em teste anterior;
essa evidência é herdada, não foi reproduzida agora nem prova impossibilidade
universal da ROM. A existência/conexão/autorização atual do alvo informado
pelo usuário também só será verificada depois da aprovação deste plano.

## Escopo, consentimento e limites

Execução posterior por USB no alvo exato fornecido pelo Orquestrador; o serial
será variável local `SPIKE028_SERIAL`, nunca copiado para evidência nova ou PR.
Não escolher automaticamente outro aparelho, alvo Wi-Fi ou primeiro device.
Confirmar observador humano local, telefone desbloqueado e consentimento para
criar display temporário, abrir dois apps locais aprovados e testar foco/resize.
Aprovação deste plano autoriza o experimento descrito, sem ampliar esses limites.

Sem root, unlock, instalação de APK/launcher, alteração de ROM, overlay,
`setprop`, `settings put/delete`, reboot, modo desktop experimental global,
ativação de opções de segurança OEM, bypass de FLAG_SECURE/DRM ou comandos em
apps privados. Apenas LEITURA de enable_freeform_support e
force_resizable_activities. Não alterar essas flags mesmo que sejam zero.
Não fazer RE/decompilação de componentes proprietários.

Aparelhos/configurações/apps distintos exigem outra decisão. Se for necessário
mudar flags persistentes para tentar H1, terminar esta rodada e propor uma
investigação separada; não tratar mudança/restauração posterior como autorizada.

Timebox: até 45 minutos após consentimento e prontidão. No máximo duas rodadas
iguais para confirmar resultado. Sem tentativas indefinidas de contornar OEM.
Teardown obrigatório mesmo se o timebox vencer ou ocorrer erro.

## Preparação de privacidade e evidência, antes do aparelho

Preparar um filtro local em memória, testado com dados sintéticos, ANTES de
qualquer dumpsys. ADB stdout/stderr devem passar pelo filtro antes de aparecer
em ferramentas, terminal capturado, arquivo ou contexto de agente. Não capturar
primeiro em arquivo bruto para redigir depois; desativar shell tracing.

O filtro deve publicar somente campos autorizados: display/task IDs temporários,
windowing mode solicitado/efetivo, bounds numéricos, visibilidade/foco e os dois
componentes de teste aprovados. Não copiar linhas inteiras contendo intents,
extras, URLs, títulos, notifications, user IDs, seriais, tokens ou outros apps.
Contexto de parser deve restringir a captura ao display criado e às tarefas
reconhecidas do experimento; não usar um grep genérico sobre todos os displays.
Se a extração não for confiável, não persistir o dump e classificar evidência
como insuficiente/BLOCKED. Descartar bytes brutos em memória após extração.

Não executar logcat, screencap/screenrecord ou coleta de clipboard/dados de apps.
Observação visual local não implica autorização para screenshot. Usar relato
estruturado do observador, sem conteúdo privado; imagem só com nova autorização
e revisão visual local prévia. Sem captura automática da tela física.

Arquivos esperados após execução aprovada:

- `docs/research/findings/SPIKE-028-freeform-windowing.md`: finding e método.
- `docs/testing/evidence/2026-10-21-spike-028-freeform.md`: comandos redigidos,
  versões reais, exit codes, campos extraídos, observação e cleanup. Ajustar a
  data se a execução ocorrer em outro dia; não inventar datas de experimento.
- Atualizar somente a linha de freeform da capability-matrix, vinculando a
  evidência e preservando as provas históricas; nenhuma mudança de runtime.

Nenhum desses findings/resultados será criado como se já tivesse sido executado.

## Procedimento observável após aprovação

### 1. Preflight e baseline

Registrar HEAD, status da worktree e versões reais no host:
`adb version`, `scrcpy --version`, `scrcpy --help`, identificação da distro e
Wayland/X11 atual. Não inferir ambiente atual a partir de registros anteriores.

No alvo explícito, sempre com `adb -s "$SPIKE028_SERIAL"`:

```sh
adb -s "$SPIKE028_SERIAL" get-state
adb -s "$SPIKE028_SERIAL" shell getprop ro.product.model
adb -s "$SPIKE028_SERIAL" shell getprop ro.build.version.release
adb -s "$SPIKE028_SERIAL" shell getprop ro.build.version.sdk
adb -s "$SPIKE028_SERIAL" shell getprop ro.build.version.incremental
adb -s "$SPIKE028_SERIAL" shell settings get global enable_freeform_support
adb -s "$SPIKE028_SERIAL" shell settings get global force_resizable_activities
adb -s "$SPIKE028_SERIAL" shell am help
```

Esses exemplos listam os comandos, não constituem um script para execução cega;
stdout/stderr terão o filtro descrito. Timeout de leitura/consulta: 10 s; após
timeout encerrar e recolher somente o cliente próprio. Unauthorized, offline,
modelo divergente, indisponibilidade ou ausência de observador → parar.

Consultar somente a feature `android.software.freeform_window_management`
por saída filtrada de `pm list features`. Ausência de feature, flag `null` ou
valor 0 é metadado da configuração, não prova isolada de suporte/limite OEM.

Selecionar dois apps JÁ instalados e aprovados localmente, de pacotes distintos,
com telas sem conteúdo pessoal. Preferir ferramentas de sistema simples, por
exemplo Configurações + Calculadora somente se o usuário confirmar ausência de
histórico/conteúdo privado. Não instalar app de teste nem apagar histórico.
Resolver apenas os componentes dos dois pacotes aprovados com
`cmd package resolve-activity --brief` e MAIN/LAUNCHER; usar o componente exato
retornado, não adivinhar atividades Xiaomi. Se faltarem dois apps adequados,
BLOCKED por fixture, sem atribuir a limitação à ROM.

### 2. Criar e identificar o display secundário

Coletar baseline sanitizado dos IDs/modos de displays antes de criar a sessão.
Depois de confirmar as flags na ajuda instalada, iniciar um único subprocesso
scrcpy próprio com:

```sh
scrcpy -s "$SPIKE028_SERIAL" --new-display=1280x720 --no-audio --no-clipboard-autosync
```

Somente acrescentar `-K -M` se disponíveis na versão instalada, autorizados e
observados como funcionais no aparelho para interação local. É modo público
alternativo condicionado à ROM, não autorização para alterar controles OEM.
Não adicionar flags que preservem display, desliguem telefone ou mudem segurança.

Manter handle/PID do subprocesso próprio para encerramento. Aguardar até 15 s
por criação; filtrar stdout/stderr antes de armazenamento. Cruzar anúncio de
novo display com diferença de inventário e estado do Android. Definir
`SPIKE028_DISPLAY_ID` como esse ID observado; validar inteiro positivo !=0.
Não reutilizar IDs 3/4 dos relatos anteriores. Se existirem displays/sessões
alheios, deixá-los intactos; ID ambíguo → BLOCKED.

### 3. Baseline de uso e lançamento freeform

Definir `SPIKE028_APP_A` e `SPIKE028_APP_B` como componentes resolvidos,
previamente aprovados. Não interpolar valores externos livres em remote shell:
validar caracteres de package/component (letras ASCII, números, underscore,
ponto e barra), sem espaços, controles ou metacaracteres; encaminhar como
argumentos, não montar shell script. Componentes que exijam caracteres fora
dessa allowlist tornam a fixture indisponível nesta rodada; não improvisar
escaping no shell remoto.

Primeiro lançar A no display secundário sem solicitação de freeform e confirmar
app visível nessa tela. Isso separa falha de launch/display de ausência de H1.
Observar se a tela física continua independente; anotar qualquer efeito nela.

Em seguida, no mesmo display, executar conforme sintaxe confirmada em `am help`:

```sh
adb -s "$SPIKE028_SERIAL" shell am start -W --windowingMode 5 --display "$SPIKE028_DISPLAY_ID" -n "$SPIKE028_APP_A"
adb -s "$SPIKE028_SERIAL" shell am start -W --windowingMode 5 --display "$SPIKE028_DISPLAY_ID" -n "$SPIKE028_APP_B"
```

Timeout de cada launch: 15 s; capturar exit code E resultado semântico filtrado.
Não usar `-S`/force-stop, clear-data, am kill-all ou privilégios adicionais.
Se Activity já existente for apenas trazida à frente/reutilizada e tornar a
medida ambígua, registrar isso; pedir fechamento manual das tarefas de teste
ou classificar BLOCKED por fixture, sem matar apps privados para facilitar prova.

### 4. Estado real e observação de duas janelas

Nos checkpoints (antes, após A, após B e após interações), executar com timeout
10 s e filtro obrigatório:

```sh
adb -s "$SPIKE028_SERIAL" shell dumpsys window displays
adb -s "$SPIKE028_SERIAL" shell dumpsys activity activities
```

Extrair somente do display e tarefas de teste: display ID de cada tarefa,
windowingMode efetivo (nome/número), bounds, visibilidade e foco. Estado solicitado
5 não substitui modo efetivo nos dumps; task registrada/retida não prova janela
visível simultaneamente. O observador confirma ambas na mesma imagem da tela
virtual, sem alternância de fullscreen. Separar tasks/atividades/janelas de apps
de overlays ou decorações OEM.

Se surgirem duas janelas livres: arrastar/redimensionar cada uma, alternar foco
A→B→A e fazer uma ação inócua por app (por exemplo navegar em uma categoria
pública de configurações e fazer cálculo sintético sem salvar). Confirmar que
a outra permanece visível e que o input foi ao app focado. Registrar bounds e
foco antes/depois, sem conteúdo das telas. Resize da janela Linux inteira não
conta como resize de janela Android.

Se não houver decorações/controle de resize, não habilitar flags persistentes
ou instalar launcher para forçar resultado. Registrar janelas sem affordances,
modo efetivo e comportamento; marcar subcritério resize/foco não comprovado.

A Issue também pede avaliar launcher: observar se existe launcher utilizável
nesse display. Opcionalmente resolver/lançar HOME pelo mecanismo público no
mesmo display, somente se houver handler local já instalado e consentimento
para essa navegação. Não forçar o launcher padrão da tela física a mudar de
papel/display. Se a navegação interromper o cenário, não repetir invasivamente;
registrar launcher ausente/não validado separadamente do suporte de janelas.

### 5. Repetição limitada e teardown

No máximo uma repetição após fechar manualmente somente as tarefas do
experimento, mantendo todas as configurações globais inalteradas. Se o modo é
convertido para fullscreen ou há rejeição OEM clara, não testar hacks alternativos.

Encerrar pelo handle da sessão scrcpy iniciada pelo experimento, aguardar e
recolher o processo; se necessário, terminar apenas seu PID com timeout limitado.
Não usar killall, adb kill-server, remover display alheio ou encerrar serviços.
Verificar por inventário filtrado que o display criado desapareceu e que não
restaram filhos próprios; verificar localmente efeito na tela física e nas duas
tarefas de teste. Se houver migração de tarefa para tela física, registrar e
solicitar fechamento manual apenas dessa tarefa. Não forçar remoção/reset geral.

Reler as duas flags globais e comparar ao baseline (incluindo `null`/ausência).
Não executar put/delete para restaurar algo que o plano não alterou. Mudança
externa inesperada, display persistente ou processo não recolhido → registrar
cleanup pendente/BLOCKED e interromper novas tentativas.

## Critérios de classificação e efeito no produto

| Finding | Evidência necessária | Implicação |
| --- | --- | --- |
| PASS | Dois pacotes/tarefas distintos, simultaneamente visíveis no mesmo display secundário; modo efetivo 5; bounds não fullscreen, movimento/resize e foco A/B observáveis; teardown confirmado | Suporte observado somente nesta ROM/build/configuração/apps/host; não habilitar universalmente nem equiparar a launcher completo |
| FAIL | Ensaio válido e observável, display/launch baseline funcionando, mas modo 5 convertido/ignorado, app B substitui A em fullscreen, ou gestão livre não atende os subcritérios | Exibir comportamento real como app único/alternância/virtual display, sem chamar desktop multijanela; não bloquear MVP |
| BLOCKED por ROM | Operação pública necessária explicitamente rejeitada por política/serviço OEM, sem intervenção permitida; anexar causa sanitizada e ponto da falha | Limite desta configuração, sem bypass; manter Mirror/VirtualDisplay conforme provas próprias |
| BLOCKED por ambiente/evidência | Ferramenta, autorização, observador, app fixture, ID ou inspeção insuficientes | Não atribuir impossibilidade à ROM; registrar precisamente a dependência e manter capacidade UNKNOWN/UNPROVEN |

Não exigir multi-resume de duas Activities como único sinal: foco/resume e
visibilidade são fenômenos distintos; o ensaio mede coexistência visível e input.
Registrar por subcritério display, launch A/B, modo efetivo, simultaneidade,
resize, foco, launcher e cleanup. Launcher não observado é pendência explícita,
sem converter duas janelas em prova de desktop/launcher completo.

FAIL/BLOCKED não bloqueiam MVP: preservar Mirror e VirtualDisplay apenas nos
limites já comprovados; não habilitar DESKTOP_WINDOWING por flag nominal. Não
alterar runtime, capacidade global ou declarar suporte OEM universal neste spike.

## Registro reprodutível e gates de entrega futura

Para cada etapa: horário, ambiente/build/versões reais, comando com serial e
caminhos pessoais redigidos, timeout/exit code, campos observados filtrados,
relato local, esperado vs observado, classe e limitações. Logs sanitizados
finais podem receber SHA-256 após revisão; não gerar hash/derivação de segredo
ou PII nem selar dumps brutos privados.

O estado inicial deste plano foi UNTESTED / NEEDS_APPROVAL. Posteriormente o
Orquestrador aprovou o experimento, os apps, o observador e a publicação via PR.
Execução e finding estão nos artefatos vinculados abaixo; não se trata de plano
interpretado como prova. Testes Cargo não se aplicam à mudança documental.

Fontes de contexto: `AGENTS.md`, PRD, arquitetura/ADRs sem root e capability-first,
roadmap/backlog, estratégia de testes, Issue #28, SPIKE-001 e matriz herdada.
A sintaxe/capacidade real de am/scrcpy será validada nas ajudas instaladas após
aprovação; referências remotas não substituem essa validação nem prova física.


## Registro da execução autorizada

- Pré-condições confirmadas pelo Orquestrador antes de abrir os apps.
- Somente Configurações e Calculadora MIUI usadas; Google Calculator não resolvida.
- Uma sessão/display, tentativa inicial e retry limitado de Configurações, após
  autorização específica para remover sua root de teste. Ajuda instalada validou
  `am stack remove` e `am task resize`, usados somente nas tarefas identificadas.
- Configurações nova permaneceu fullscreen; Calculadora freeform foi
  redimensionada por API pública. Foco programático observado, sem inventar prova
  visual/mouse ou launcher. Não houve alteração de flags globais ou instalação.
- Classificação: FAIL do cenário, sem afirmar impossibilidade universal de ROM;
  floating individual é resultado parcial positivo, não desktop completo.
- Filtro ajustado após reconhecer UID como componente; a correção e os limites
  estão explicitados na evidência. Nenhum dump bruto/imagem foi versionado.
- Scrcpy próprio recolhido; teardown do display confirmado em três inventários
  após retenção inicial transitória; flags globais continuaram 0.
- [Relatório](../../research/findings/SPIKE-028-freeform-windowing.md).
- [Evidência](../../testing/evidence/2026-10-21-spike-028-freeform.md).
