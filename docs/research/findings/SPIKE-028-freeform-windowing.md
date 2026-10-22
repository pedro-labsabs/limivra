# SPIKE-028 — duas janelas livres em display virtual

**Resultado: FAIL para multijanela livre simultânea no cenário testado.**
**Resultado parcial: floating individual observado e redimensionável por ADB.**
Refs #28. Execução: 2026-10-21, sem root ou alteração de configuração persistente.
Executor: limivra-dev. Auditoria independente limivra-qa: PENDENTE.

## O que foi medido

Xiaomi Redmi Note 13 4G, modelo 23129RA5FL, Android 15 / SDK 35, HyperOS build
OS2.0.213.0.VNGMIXM; Linux Mint 22.3/X11, scrcpy 5.0.1, platform-tools 37.0.1.
Alvo USB explícito autorizado pelo usuário, com observador local e apps aprovados
sem conteúdo privado. Serial redigido. Base de desenvolvimento: `5d47205`.

Hipótese: solicitar WINDOWING_MODE_FREEFORM=5 para dois apps distintos deveria
produzir duas janelas livres no mesmo display virtual, com resize e foco reais.
O critério não se confunde com alternância de fullscreen, uma janela flutuante
sobre tela cheia, split-screen ou múltiplas janelas Linux do scrcpy.

Foram usados Configurações (`com.android.settings/.MainSettings`) e Calculadora
(`com.miui.calculator/.cal.CalculatorActivity`). Nenhum app novo instalado.
A outra calculadora aprovada (`com.google.android.calculator`) não foi resolvida.

## Observações e reprodução

O scrcpy criou o display 7, correlacionado entre anúncio do subprocesso e inventário
Android. A flag de criação solicitou 1280x720; bounds fullscreen efetivos observados
foram [0,0,720,1280]. Os valores efetivos, não a flag, fundamentam a análise.

| Checkpoint | Configurações | Calculadora | Foco programático |
| --- | --- | --- | --- |
| Baseline sem modo 5 | task 12247, fullscreen, [0,0,720,1280] | ainda não lançada pelo spike | Configurações |
| Primeiro am start modo 5 para A/B | Configurações reutilizada, fullscreen; visibleRequested=false após B | task 12248, freeform, [17,94,737,1374] | Calculadora |
| Após remoção autorizada de 12247 e novo A com modo 5 | nova task 12249, SEM aviso de reuse, fullscreen, [0,0,720,1280] | permaneceu freeform; ambas com visibleRequested=true | Configurações |
| Após resize público e bring-to-front de B | permaneceu fullscreen | bounds mudaram para [40,80,680,1120], continuou freeform | Calculadora |

Cada launch, remoção de tarefa e resize acima retornou exit 0. Os modos/bounds
foram extraídos de Task/mOverrideConfig em `dumpsys activity activities`; foco
foi extraído de mCurrentFocus/mFocusedApp em `dumpsys window displays`, restritos
ao display criado e componentes aprovados. Não foi inferido sucesso por exit 0.
As tarefas auxiliares aninhadas não foram contadas como segunda janela livre.

O observador/Orquestrador relatou Calculadora freeform e Configurações fullscreen,
autorizando remover somente a tarefa Configurações do spike para eliminar reuse.
Essa segunda tentativa produziu nova tarefa, mas não alterou o resultado de modo.
A prova é de floating individual com resize/foco programáticos; não foi recebida
confirmação de duas janelas livres interativas por mouse, nem de launcher desktop.
Launcher/HOME não foi lançado; input humano e affordances de decorações permanecem
não comprovados. Houve eventos ERROR redigidos do backend sem causa preservada;
não atribuir causa nem certificar input SDK/UHID a partir deles.

Flags globais enable_freeform_support e force_resizable_activities estavam em 0
e assim permaneceram. A feature android.software.freeform_window_management não
foi anunciada. Esses metadados não foram tratados como prova isolada de bloqueio.
Nenhum settings put/delete, setprop, reboot ou contorno OEM foi tentado.

## Interpretação limitada e efeito no MVP

O par de apps não atingiu duas roots livres: Configurações foi mantida em tela
cheia mesmo em tarefa nova, enquanto Calculadora aceitou floating. O finding é
**FAIL do critério deste ensaio**, e não BLOCKED por uma rejeição explícita do
serviço (não foi observada tal rejeição nos comandos de launch/resize).

O limite foi observado nesta ROM/build/configuração/par de apps. Não foi isolada
a causa entre política OEM, propriedades de resizability dos apps e configuração
do sistema; não concluir que toda combinação de apps ou toda ROM HyperOS é incapaz
de freeform. Em particular, a conclusão histórica de ausência total de janelas
livres deve ser refinada: houve uma tarefa freeform individual observável.

Para o produto: não apresentar esse cenário como desktop multijanela AOSP completo
nem habilitar DESKTOP_WINDOWING genericamente. Floating individual é capacidade
condicional e não prova launcher, workspace, duas janelas livres ou input confiável.
O FAIL não bloqueia MVP: preservar Mirror e VirtualDisplay nos limites das provas
próprias, com explicação clara, sem alterar runtime neste spike.

## Segurança, cleanup e evidência

A sessão scrcpy foi encerrada pelo handle próprio e recolhida com exit 0. O
inventário imediato ainda reteve o display 7; a checagem posterior confirmou só
display 0 em WindowManager/DisplayManager/ActivityManager. Nenhuma das tarefas
criadas foi encontrada no display físico. Não houve remoção forçada de display,
killall, kill-server ou encerramento de tarefa alheia. As flags continuaram 0.
Não é uma auditoria geral de processos remotos ou da tela física do usuário.

Nenhum dump bruto, screenshot, logcat ou conteúdo de apps foi versionado. O filtro
inicial teve uma limitação de reconhecimento de UID, corrigida e explicitada na
evidência; os artefatos finais contêm somente metadados autorizados e exemplos
sintéticos marcados como tais. O extrator final e sua impressão SHA-256 estão
incluídos para revisão, com saídas reais sanitizadas e exit codes/horários.

- [Evidência completa e comandos reproduzíveis](../../testing/evidence/2026-10-21-spike-028-freeform.md)
- [Plano aprovado e registro de execução](../../superpowers/plans/2026-10-21-issue-28-freeform-spike.md)
- [Matriz de compatibilidade](../capability-matrix.md)

IDs de display/tarefa são efêmeros: qualquer reprodução deve identificar os
próprios antes de executar launch/remoção/resize. Sem auditoria independente
concluída, sem encerramento da #28 e sem alegação de compatibilidade universal.
