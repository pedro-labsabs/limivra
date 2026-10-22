# SPIKE-028 — evidência empírica de freeform (2026-10-21)

**Finding: FAIL para duas janelas livres no cenário testado.**
Floating individual: OBSERVADO em Calculadora, inclusive resize programático.
Isso não comprova desktop multijanela e não significa ausência total de freeform.
Plano e experimento aprovados pelo Orquestrador; observador local e apps sem
conteúdo privado confirmados antes da abertura. Auditoria independente: PENDENTE.
Executor: limivra-dev. Base: `5d47205481a740b33fa033c7287c4fec0a272c80`.

## Ambiente e método

Host Linux Mint 22.3, X11; ADB 1.0.41 / platform-tools 37.0.1; scrcpy 5.0.1.
Telefone autorizado USB, modelo 23129RA5FL, Android 15 / SDK 35,
HyperOS build OS2.0.213.0.VNGMIXM. Serial e caminhos pessoais redigidos.
Janela de medição: 18:08:49–18:17:57 UTC (14:08:49–14:17:57 UTC-04).
Nenhum aparelho alternativo ou alvo Wi-Fi foi selecionado.

Consentimento adicional recebido durante a execução para fechar somente a tarefa
Configurações criada pelo experimento e relançá-la sem a ambiguidade de reuse.
Apps autorizados/resolvidos:

- `com.android.settings/.MainSettings`.
- `com.miui.calculator/.cal.CalculatorActivity`.
- `com.google.android.calculator`: resolução exit 0 sem componente; não utilizado.

Filtros foram exercitados com sentinelas sintéticas antes de dumpsys. Bytes
brutos foram capturados apenas em memória e descartados após extração, sem
arquivo bruto, logcat, screenshot, screenrecord, áudio ou leitura de clipboard.
Somente campos de teste são publicados; não há intents, extras ou dados de apps.
As amostras JSONL abaixo são saídas reais das projeções permitidas, não fixtures.

Limitação corrigida do filtro: a primeira extração confundiu sufixos de UID com
nomes de componente, inclusive em saída transitória de ferramenta/JSON local.
A regra foi restringida a nomes de classes e ganhou testes negativos para UID.
Os pseudo-componentes foram excluídos dos artefatos finais. O checkpoint corrigido
às 18:14:09 confirmou os mesmos modos/bounds antes da segunda tentativa.
Registros de tarefas auxiliares aninhadas não foram usados para inferir duas
janelas: o finding usa roots identificados e suas configurações efetivas.
Não afirmar que o filtro inicial era completo nem que esses metadados eram
uma prova de visibilidade ou de conteúdo da tela.

## Resultados principais

| Etapa | Exit | Observado |
| --- | ---: | --- |
| get-state/modelo/build | 0 | device; aparelho de referência confirmado |
| settings get global enable_freeform_support | 0 | 0 antes e após |
| settings get global force_resizable_activities | 0 | 0 antes e após |
| pm list features (filtrado) | 0 | freeform_window_management ausente |
| scrcpy --new-display=1280x720 | criação confirmada | único ID novo 7, correlacionado com anúncio do scrcpy; não presumido de testes anteriores |
| launch baseline Configurações | 0 | Status ok; task 12247 fullscreen, bounds [0,0,720,1280] |
| modo 5 em Configurações existente | 0 | Status ok, aviso de reuse; permaneceu fullscreen |
| modo 5 em Calculadora | 0 | Status ok, sem reuse; task 12248 freeform, bounds [17,94,737,1374] |
| remoção autorizada da task 12247 | 0 | task ausente no checkpoint seguinte; nenhum force-stop de pacote |
| novo launch Configurações modo 5 | 0 | Status ok, SEM reuse; nova task 12249 ainda fullscreen, bounds [0,0,720,1280] |
| am task resize da Calculadora | 0 | bounds efetivos mudaram para [40,80,680,1120]; continuou freeform |
| foco programático por launch | 0 | WindowManager focou Configurações após relaunch e Calculadora após bring-to-front; não é prova de foco por mouse |
| saída do scrcpy próprio | 0 | handle recolhido, alive=false |
| inventário após teardown | 0 | somente display 0; tasks criadas não encontradas na tela física |

O display solicitado como 1280x720 apresentou configuração efetiva de tarefas
fullscreen em 720x1280. Não inferir orientação/dimensões efetivas só pela flag
solicitada. Bounds originais de floating excederam a área fullscreen medida;
o resize foi confirmado por estado, sem presumir que cada pixel estava visível.

Na primeira tentativa, Configurações tinha visible=true e visibleRequested=false
após abrir Calculadora. Após o relaunch novo, ambas tinham visible=true e
visibleRequested=true, mas só a Calculadora estava freeform. Tarefas existentes
ou flags de visibilidade não demonstram duas janelas livres simultâneas.

Relato recebido do observador/Orquestrador: Calculadora flutuante em freeform,
Configurações fullscreen, com autorização de remoção/retry. Não foi recebida
confirmação de duas janelas livres com affordances de resize/foco por mouse.
A medição programática não substitui essa confirmação; nenhum PASS visual foi
inventado. Launcher/HOME não foi lançado, pois o critério de duas freeform já
não foi atendido e essa exploração era opcional.

Foram observadas 16 linhas ERROR do scrcpy, retidas apenas como categoria
redigida (sem causa bruta preservada). Não são exit codes bem-sucedidos de
operações; os eventos correspondentes não têm campo exit no anexo. Não atribuir
causa SDK/UHID nem certificar input humano a partir desses eventos. A sessão não
foi reiniciada com flags extras para tentar contornar a limitação.

## Comandos efetivamente usados (redigidos)

Todos os clientes de leitura/launch usaram alvo explícito e servidor loopback
`adb -H 127.0.0.1 -P 5037 -s <DEVICE>`. Variáveis herdadas de trace/seleção/servidor
foram removidas nesses clientes; nenhum kill-server ou mudança de daemon global.
Timeouts: consultas 10 s, launches 15 s, espera de display 15 s, reap 5 s.

```sh
adb version
scrcpy --version
scrcpy --help
adb -s <DEVICE> get-state
adb -s <DEVICE> shell getprop ro.product.model
adb -s <DEVICE> shell getprop ro.build.version.release
adb -s <DEVICE> shell getprop ro.build.version.sdk
adb -s <DEVICE> shell getprop ro.build.version.incremental
adb -s <DEVICE> shell settings get global enable_freeform_support
adb -s <DEVICE> shell settings get global force_resizable_activities
adb -s <DEVICE> shell am help
adb -s <DEVICE> shell pm list features
adb -s <DEVICE> shell cmd package resolve-activity --brief -a android.intent.action.MAIN -c android.intent.category.LAUNCHER -p <APPROVED_PACKAGE>
scrcpy -s <DEVICE> --new-display=1280x720 --no-audio --no-clipboard-autosync --window-title=Limivra-SPIKE-028
adb -s <DEVICE> shell am start -W --display 7 -n com.android.settings/.MainSettings
adb -s <DEVICE> shell am start -W --windowingMode 5 --display 7 -n com.android.settings/.MainSettings
adb -s <DEVICE> shell am start -W --windowingMode 5 --display 7 -n com.miui.calculator/.cal.CalculatorActivity
adb -s <DEVICE> shell dumpsys window displays
adb -s <DEVICE> shell dumpsys activity activities
adb -s <DEVICE> shell am stack remove 12247
adb -s <DEVICE> shell am start -W --windowingMode 5 --display 7 -n com.android.settings/.MainSettings
adb -s <DEVICE> shell am task resize 12248 40 80 680 1120
adb -s <DEVICE> shell am start -W --windowingMode 5 --display 7 -n com.miui.calculator/.cal.CalculatorActivity
```

Os dumps foram consultados em cada checkpoint com filtro aplicado ANTES de saída
para ferramenta/arquivo. `am help` confirmou stack remove e task resize antes do
uso. IDs acima são resultados efêmeros observados, não constantes para reprodução:
identificar novamente o display/root de cada tarefa em qualquer nova execução.
Não executar stack remove/resize por ID histórico. Nenhum settings put/delete,
setprop, root, reboot, APK/launcher extra, flag de segurança ou dado privado alterado.

## Teardown e limites

O script de controle terminou a sessão por seu handle, esperou e recolheu scrcpy
(exit 0). A leitura ActivityManager imediatamente após a saída ainda mostrou o
display 7 retido: não foi declarado cleanup completo nessa leitura. Em nova
checagem às 18:17:56–57, WindowManager, DisplayManager (`dumpsys display`, também
filtrado só para IDs lógicos) e ActivityManager mostraram apenas display 0.
As tasks 12247/12248/12249 não foram encontradas na tela física. Não foi forçada
remoção de display ou de tarefa alheia. Ambas as flags globais permaneceram 0.

Limite desta prova de cleanup: processo host próprio recolhido e estado do display/
tarefas consultado; não houve auditoria geral de todos os processos do Android.
Não afirmar ausência universal de processos órfãos remotos ou de efeitos privados.
Cargo/testes runtime não executados: alteração somente documental/pesquisa.
Verificação documental e sanitização registradas no relatório; auditoria PENDENTE.

## Anexo: saídas reais filtradas (JSONL)

```jsonl
{"fields": {"ID": "linuxmint", "VERSION_ID": "22.3", "display_configured": true, "session_type": "x11"}, "step": "host environment"}
{"exit": 0, "fields": {"adb": "1.0.41", "platform_tools": "37.0.1"}, "step": "host adb version", "utc": "2026-10-21T18:08:49.538240+00:00"}
{"exit": 0, "fields": {"scrcpy": "5.0.1"}, "step": "host scrcpy version", "utc": "2026-10-21T18:08:49.539715+00:00"}
{"exit": 0, "fields": {"--keyboard": true, "--mouse": true, "--new-display": true, "--no-audio": true, "--no-clipboard-autosync": true}, "step": "host scrcpy flag support", "utc": "2026-10-21T18:08:49.541315+00:00"}
{"exit": 0, "fields": {"state": "device"}, "step": "USB target get-state", "utc": "2026-10-21T18:08:49.543312+00:00"}
{"exit": 0, "fields": {"matches_reference": true, "model": "23129RA5FL"}, "step": "device model", "utc": "2026-10-21T18:08:49.597227+00:00"}
{"exit": 0, "fields": {"value": "15"}, "step": "ro.build.version.release", "utc": "2026-10-21T18:08:49.631725+00:00"}
{"exit": 0, "fields": {"value": "35"}, "step": "ro.build.version.sdk", "utc": "2026-10-21T18:08:49.667001+00:00"}
{"exit": 0, "fields": {"value": "OS2.0.213.0.VNGMIXM"}, "step": "ro.build.version.incremental", "utc": "2026-10-21T18:08:49.704537+00:00"}
{"exit": 0, "fields": {"value": "0"}, "step": "enable_freeform_support", "utc": "2026-10-21T18:08:49.778204+00:00"}
{"exit": 0, "fields": {"value": "0"}, "step": "force_resizable_activities", "utc": "2026-10-21T18:08:49.843399+00:00"}
{"exit": 0, "fields": {"--display": true, "--windowingMode": true, "start": true}, "step": "am syntax", "utc": "2026-10-21T18:08:49.911755+00:00"}
{"exit": 0, "fields": {"freeform_feature_present": false}, "step": "freeform feature", "utc": "2026-10-21T18:08:49.983690+00:00"}
{"exit": 0, "fields": {"component": "com.android.settings/.MainSettings", "package": "com.android.settings"}, "step": "resolve approved app", "utc": "2026-10-21T18:12:27.044840+00:00"}
{"exit": 0, "fields": {"component": "com.miui.calculator/.cal.CalculatorActivity", "package": "com.miui.calculator"}, "step": "resolve approved app", "utc": "2026-10-21T18:12:27.088982+00:00"}
{"exit": 0, "fields": {"component": "unresolved", "package": "com.google.android.calculator"}, "step": "resolve approved app", "utc": "2026-10-21T18:12:27.164423+00:00"}
{"exit": 0, "fields": {"display_ids": [0], "tasks": []}, "step": "baseline display inventory", "utc": "2026-10-21T18:12:27.319435+00:00"}
{"exit": 0, "fields": {"display_ids": [0], "tasks": []}, "step": "wait for display inventory", "utc": "2026-10-21T18:12:27.913317+00:00"}
{"exit": 0, "fields": {"display_ids": [0], "tasks": []}, "step": "wait for display inventory", "utc": "2026-10-21T18:12:28.493526+00:00"}
{"fields": {"category": "redacted-backend-error"}, "step": "scrcpy error", "utc": "2026-10-21T18:12:28.652295+00:00"}
{"fields": {"category": "redacted-backend-error"}, "step": "scrcpy error", "utc": "2026-10-21T18:12:28.652644+00:00"}
{"exit": 0, "fields": {"display_ids": [0, 7], "tasks": []}, "step": "wait for display inventory", "utc": "2026-10-21T18:12:29.059532+00:00"}
{"exit": 0, "fields": {"announced_ids": [7], "display_id": 7}, "step": "display identified", "utc": "2026-10-21T18:12:29.059888+00:00"}
{"exit": 0, "fields": {"component": "com.android.settings/.MainSettings", "display_id": 7, "error_present": false, "launch_status": "ok", "requested_mode": "default", "reused_activity_warning": false, "security_rejection": false}, "step": "baseline launch A", "utc": "2026-10-21T18:12:30.449626+00:00"}
{"exit": 0, "fields": {"display_ids": [0, 7], "tasks": [{"components": ["com.android.settings/.MainSettings", "com.android.settings/.MiuiSettings", "com.android.settings/com.android.settings.MainSettings"], "configurations": [{"bounds": [0, 0, 720, 1280], "mode": "fullscreen", "source": "mOverrideConfig"}], "display_id": 7, "parent_task_id": null, "task_id": 12247, "task_mode": "fullscreen", "visible": true, "visibleRequested": true}, {"components": ["com.android.settings/.MainSettings"], "configurations": [], "display_id": 7, "parent_task_id": 12245, "task_id": 12246, "task_mode": "fullscreen", "visible": false, "visibleRequested": false}]}, "step": "after baseline A activities", "utc": "2026-10-21T18:12:31.545199+00:00"}
{"exit": 0, "fields": {"display_ids": [0, 7], "focus": [{"component": "com.android.settings/com.android.settings.MainSettings", "display_id": 7, "field": "mCurrentFocus"}, {"component": "com.android.settings/.MainSettings", "display_id": 7, "field": "mFocusedApp"}]}, "step": "after baseline A windows", "utc": "2026-10-21T18:12:31.609506+00:00"}
{"exit": 0, "fields": {"component": "com.android.settings/.MainSettings", "display_id": 7, "error_present": false, "launch_status": "ok", "requested_mode": 5, "reused_activity_warning": true, "security_rejection": false}, "step": "request freeform A", "utc": "2026-10-21T18:12:31.723779+00:00"}
{"exit": 0, "fields": {"display_ids": [0, 7], "tasks": [{"components": ["com.android.settings/.MainSettings", "com.android.settings/.MiuiSettings", "com.android.settings/com.android.settings.MainSettings"], "configurations": [{"bounds": [0, 0, 720, 1280], "mode": "fullscreen", "source": "mOverrideConfig"}], "display_id": 7, "parent_task_id": null, "task_id": 12247, "task_mode": "fullscreen", "visible": true, "visibleRequested": true}, {"components": ["com.android.settings/.MainSettings"], "configurations": [], "display_id": 7, "parent_task_id": 12245, "task_id": 12246, "task_mode": "fullscreen", "visible": false, "visibleRequested": false}]}, "step": "after mode5 A activities", "utc": "2026-10-21T18:12:32.813570+00:00"}
{"exit": 0, "fields": {"display_ids": [0, 7], "focus": [{"component": "com.android.settings/com.android.settings.MainSettings", "display_id": 7, "field": "mCurrentFocus"}, {"component": "com.android.settings/.MainSettings", "display_id": 7, "field": "mFocusedApp"}]}, "step": "after mode5 A windows", "utc": "2026-10-21T18:12:32.905194+00:00"}
{"exit": 0, "fields": {"component": "com.miui.calculator/.cal.CalculatorActivity", "display_id": 7, "error_present": false, "launch_status": "ok", "requested_mode": 5, "reused_activity_warning": false, "security_rejection": false}, "step": "request freeform B", "utc": "2026-10-21T18:12:34.782428+00:00"}
{"exit": 0, "fields": {"display_ids": [0, 7], "tasks": [{"components": ["com.miui.calculator/.cal.CalculatorActivity", "com.miui.calculator/com.miui.calculator.cal.CalculatorActivity"], "configurations": [{"bounds": [17, 94, 737, 1374], "mode": "freeform", "source": "mOverrideConfig"}], "display_id": 7, "package": "com.miui.calculator", "parent_task_id": null, "task_id": 12248, "task_mode": "freeform", "visible": true, "visibleRequested": true}, {"components": ["com.android.settings/.MainSettings", "com.android.settings/.MiuiSettings", "com.android.settings/com.android.settings.MainSettings"], "configurations": [{"bounds": [0, 0, 720, 1280], "mode": "fullscreen", "source": "mOverrideConfig"}], "display_id": 7, "parent_task_id": null, "task_id": 12247, "task_mode": "fullscreen", "visible": true, "visibleRequested": false}, {"components": ["com.miui.calculator/.cal.CalculatorActivity"], "configurations": [], "display_id": 7, "parent_task_id": 12245, "task_id": 12246, "task_mode": "fullscreen", "visible": true, "visibleRequested": false}]}, "step": "after mode5 B activities", "utc": "2026-10-21T18:12:35.908200+00:00"}
{"exit": 0, "fields": {"display_ids": [0, 7], "focus": [{"component": "com.miui.calculator/com.miui.calculator.cal.CalculatorActivity", "display_id": 7, "field": "mCurrentFocus"}, {"component": "com.miui.calculator/.cal.CalculatorActivity", "display_id": 7, "field": "mFocusedApp"}]}, "step": "after mode5 B windows", "utc": "2026-10-21T18:12:36.028871+00:00"}
{"exit": 0, "fields": {"apps": ["com.android.settings", "com.miui.calculator"], "display_id": 7, "instruction": "report visible windows, fullscreen replacement, Android resize and focus; send continue to finish or inspect to collect another checkpoint"}, "step": "awaiting local observation", "utc": "2026-10-21T18:12:36.029523+00:00"}
{"exit": 0, "fields": {"display_ids": [0, 7], "structural_labels": [{"indent": 0, "kind": "display"}, {"indent": 0, "kind": "display"}], "tasks": [{"components": ["com.miui.calculator/.cal.CalculatorActivity", "com.miui.calculator/com.miui.calculator.cal.CalculatorActivity"], "configurations": [{"bounds": [17, 94, 737, 1374], "mode": "freeform", "source": "mOverrideConfig"}], "display_id": 7, "package": "com.miui.calculator", "parent_task_id": null, "task_id": 12248, "task_mode": "freeform", "visible": true, "visibleRequested": true}, {"components": ["com.android.settings/.MainSettings", "com.android.settings/.MiuiSettings", "com.android.settings/com.android.settings.MainSettings"], "configurations": [{"bounds": [0, 0, 720, 1280], "mode": "fullscreen", "source": "mOverrideConfig"}], "display_id": 7, "parent_task_id": null, "task_id": 12247, "task_mode": "fullscreen", "visible": true, "visibleRequested": false}, {"components": ["com.miui.calculator/.cal.CalculatorActivity"], "configurations": [], "display_id": 7, "parent_task_id": 12245, "task_id": 12246, "task_mode": "fullscreen", "visible": true, "visibleRequested": false}]}, "step": "corrected-filter activity", "utc": "2026-10-21T18:14:09.039085+00:00"}
{"exit": 0, "fields": {"display_ids": [0, 7], "focus": [{"component": "com.miui.calculator/com.miui.calculator.cal.CalculatorActivity", "display_id": 7, "field": "mCurrentFocus"}, {"component": "com.miui.calculator/.cal.CalculatorActivity", "display_id": 7, "field": "mFocusedApp"}]}, "step": "corrected-filter window", "utc": "2026-10-21T18:14:09.122357+00:00"}
{"fields": {"category": "redacted-backend-error"}, "step": "scrcpy error", "utc": "2026-10-21T18:14:12.048862+00:00"}
{"fields": {"category": "redacted-backend-error"}, "step": "scrcpy error", "utc": "2026-10-21T18:14:12.052041+00:00"}
{"fields": {"category": "redacted-backend-error"}, "step": "scrcpy error", "utc": "2026-10-21T18:15:09.257299+00:00"}
{"fields": {"category": "redacted-backend-error"}, "step": "scrcpy error", "utc": "2026-10-21T18:15:09.257930+00:00"}
{"fields": {"category": "redacted-backend-error"}, "step": "scrcpy error", "utc": "2026-10-21T18:15:34.816819+00:00"}
{"fields": {"category": "redacted-backend-error"}, "step": "scrcpy error", "utc": "2026-10-21T18:15:34.818818+00:00"}
{"fields": {"category": "redacted-backend-error"}, "step": "scrcpy error", "utc": "2026-10-21T18:15:37.823935+00:00"}
{"fields": {"category": "redacted-backend-error"}, "step": "scrcpy error", "utc": "2026-10-21T18:15:37.824536+00:00"}
{"fields": {"category": "redacted-backend-error"}, "step": "scrcpy error", "utc": "2026-10-21T18:15:40.831144+00:00"}
{"fields": {"category": "redacted-backend-error"}, "step": "scrcpy error", "utc": "2026-10-21T18:15:40.833803+00:00"}
{"fields": {"category": "redacted-backend-error"}, "step": "scrcpy error", "utc": "2026-10-21T18:15:53.086537+00:00"}
{"fields": {"category": "redacted-backend-error"}, "step": "scrcpy error", "utc": "2026-10-21T18:15:53.087186+00:00"}
{"exit": 0, "fields": {"display_ids": [0, 7], "security_rejection": false, "tasks": [{"components": ["com.miui.calculator/.cal.CalculatorActivity", "com.miui.calculator/com.miui.calculator.cal.CalculatorActivity"], "configurations": [{"bounds": [17, 94, 737, 1374], "mode": "freeform", "source": "mOverrideConfig"}], "display_id": 7, "package": "com.miui.calculator", "parent_task_id": null, "task_id": 12248, "task_mode": "freeform", "visible": true, "visibleRequested": true}, {"components": ["com.android.settings/.MainSettings", "com.android.settings/.MiuiSettings", "com.android.settings/com.android.settings.MainSettings"], "configurations": [{"bounds": [0, 0, 720, 1280], "mode": "fullscreen", "source": "mOverrideConfig"}], "display_id": 7, "parent_task_id": null, "task_id": 12247, "task_mode": "fullscreen", "visible": true, "visibleRequested": false}, {"components": ["com.miui.calculator/.cal.CalculatorActivity"], "configurations": [], "display_id": 7, "parent_task_id": 12245, "task_id": 12246, "task_mode": "fullscreen", "visible": true, "visibleRequested": false}]}, "step": "before authorized removal", "utc": "2026-10-21T18:16:25.284419+00:00"}
{"exit": 0, "fields": {"error_present": false, "security_rejection": false, "target_task": 12247}, "step": "remove own Settings root task", "utc": "2026-10-21T18:16:25.416805+00:00"}
{"exit": 0, "fields": {"display_ids": [0, 7], "security_rejection": false, "tasks": [{"components": ["com.miui.calculator/.cal.CalculatorActivity", "com.miui.calculator/com.miui.calculator.cal.CalculatorActivity"], "configurations": [{"bounds": [17, 94, 737, 1374], "mode": "freeform", "source": "mOverrideConfig"}], "display_id": 7, "package": "com.miui.calculator", "parent_task_id": null, "task_id": 12248, "task_mode": "freeform", "visible": true, "visibleRequested": true}, {"components": ["com.miui.calculator/.cal.CalculatorActivity"], "configurations": [], "display_id": 7, "parent_task_id": 12245, "task_id": 12246, "task_mode": "fullscreen", "visible": true, "visibleRequested": false}]}, "step": "after authorized removal", "utc": "2026-10-21T18:16:26.016586+00:00"}
{"exit": 0, "fields": {"component": "com.android.settings/.MainSettings", "launch_status": "ok", "requested_mode": 5, "reused_activity_warning": false, "security_rejection": false}, "step": "fresh mode5 Settings", "utc": "2026-10-21T18:16:27.188001+00:00"}
{"exit": 0, "fields": {"display_ids": [0, 7], "security_rejection": false, "tasks": [{"components": ["com.miui.calculator/.cal.CalculatorActivity", "com.miui.calculator/com.miui.calculator.cal.CalculatorActivity"], "configurations": [{"bounds": [17, 94, 737, 1374], "mode": "freeform", "source": "mOverrideConfig"}], "display_id": 7, "package": "com.miui.calculator", "parent_task_id": null, "task_id": 12248, "task_mode": "freeform", "visible": true, "visibleRequested": true}, {"components": ["com.android.settings/.MainSettings", "com.android.settings/.MiuiSettings", "com.android.settings/com.android.settings.MainSettings"], "configurations": [{"bounds": [0, 0, 720, 1280], "mode": "fullscreen", "source": "mOverrideConfig"}], "display_id": 7, "parent_task_id": null, "task_id": 12249, "task_mode": "fullscreen", "visible": true, "visibleRequested": true}, {"components": ["com.android.settings/.MainSettings"], "configurations": [], "display_id": 7, "parent_task_id": 12245, "task_id": 12246, "task_mode": "fullscreen", "visible": false, "visibleRequested": false}]}, "step": "after fresh mode5 Settings", "utc": "2026-10-21T18:16:28.297079+00:00"}
{"exit": 0, "fields": {"focus": [{"component": "com.android.settings/com.android.settings.MainSettings", "display_id": 7, "field": "mCurrentFocus"}, {"component": "com.android.settings/.MainSettings", "display_id": 7, "field": "mFocusedApp"}], "security_rejection": false}, "step": "focus after fresh Settings", "utc": "2026-10-21T18:16:28.365774+00:00"}
{"exit": 0, "fields": {"error_present": false, "requested_bounds": [40, 80, 680, 1120], "security_rejection": false, "target_task": 12248}, "step": "resize observed Calculator freeform task", "utc": "2026-10-21T18:16:28.517787+00:00"}
{"exit": 0, "fields": {"display_ids": [0, 7], "security_rejection": false, "tasks": [{"components": ["com.miui.calculator/.cal.CalculatorActivity", "com.miui.calculator/com.miui.calculator.cal.CalculatorActivity"], "configurations": [{"bounds": [40, 80, 680, 1120], "mode": "freeform", "source": "mOverrideConfig"}], "display_id": 7, "package": "com.miui.calculator", "parent_task_id": null, "task_id": 12248, "task_mode": "freeform", "visible": true, "visibleRequested": true}, {"components": ["com.android.settings/.MainSettings", "com.android.settings/.MiuiSettings", "com.android.settings/com.android.settings.MainSettings"], "configurations": [{"bounds": [0, 0, 720, 1280], "mode": "fullscreen", "source": "mOverrideConfig"}], "display_id": 7, "parent_task_id": null, "task_id": 12249, "task_mode": "fullscreen", "visible": true, "visibleRequested": true}, {"components": ["com.android.settings/.MainSettings"], "configurations": [], "display_id": 7, "parent_task_id": 12245, "task_id": 12246, "task_mode": "fullscreen", "visible": false, "visibleRequested": false}]}, "step": "after Calculator resize", "utc": "2026-10-21T18:16:29.128361+00:00"}
{"exit": 0, "fields": {"focus": [{"component": "com.android.settings/com.android.settings.MainSettings", "display_id": 7, "field": "mCurrentFocus"}, {"component": "com.android.settings/.MainSettings", "display_id": 7, "field": "mFocusedApp"}], "security_rejection": false}, "step": "focus after Calculator resize", "utc": "2026-10-21T18:16:29.217578+00:00"}
{"exit": 0, "fields": {"component": "com.miui.calculator/.cal.CalculatorActivity", "launch_status": "ok", "requested_mode": 5, "reused_activity_warning": true, "security_rejection": false}, "step": "mode5 Calculator focus request", "utc": "2026-10-21T18:16:29.355727+00:00"}
{"exit": 0, "fields": {"display_ids": [0, 7], "security_rejection": false, "tasks": [{"components": ["com.miui.calculator/.cal.CalculatorActivity", "com.miui.calculator/com.miui.calculator.cal.CalculatorActivity"], "configurations": [{"bounds": [40, 80, 680, 1120], "mode": "freeform", "source": "mOverrideConfig"}], "display_id": 7, "package": "com.miui.calculator", "parent_task_id": null, "task_id": 12248, "task_mode": "freeform", "visible": true, "visibleRequested": true}, {"components": ["com.android.settings/.MainSettings", "com.android.settings/.MiuiSettings", "com.android.settings/com.android.settings.MainSettings"], "configurations": [{"bounds": [0, 0, 720, 1280], "mode": "fullscreen", "source": "mOverrideConfig"}], "display_id": 7, "parent_task_id": null, "task_id": 12249, "task_mode": "fullscreen", "visible": true, "visibleRequested": true}, {"components": ["com.miui.calculator/.cal.CalculatorActivity"], "configurations": [], "display_id": 7, "parent_task_id": 12245, "task_id": 12246, "task_mode": "fullscreen", "visible": false, "visibleRequested": false}]}, "step": "final simultaneous checkpoint", "utc": "2026-10-21T18:16:29.951526+00:00"}
{"exit": 0, "fields": {"focus": [{"component": "com.miui.calculator/com.miui.calculator.cal.CalculatorActivity", "display_id": 7, "field": "mCurrentFocus"}, {"component": "com.miui.calculator/.cal.CalculatorActivity", "display_id": 7, "field": "mFocusedApp"}], "security_rejection": false}, "step": "final focus checkpoint", "utc": "2026-10-21T18:16:30.041110+00:00"}
{"fields": {"category": "redacted-backend-error"}, "step": "scrcpy error", "utc": "2026-10-21T18:16:38.650240+00:00"}
{"fields": {"category": "redacted-backend-error"}, "step": "scrcpy error", "utc": "2026-10-21T18:16:38.650899+00:00"}
{"exit": 0, "fields": {"alive": false}, "step": "scrcpy reaped", "utc": "2026-10-21T18:17:24.633021+00:00"}
{"exit": 0, "fields": {"display_ids": [0, 7], "tasks": [{"components": ["com.android.settings/.MainSettings", "com.android.settings/.MiuiSettings", "com.android.settings/com.android.settings.MainSettings"], "configurations": [{"bounds": [0, 0, 720, 1280], "mode": "fullscreen", "source": "mOverrideConfig"}], "display_id": 7, "parent_task_id": null, "task_id": 12249, "task_mode": "fullscreen", "visible": false, "visibleRequested": false}]}, "step": "teardown display inventory", "utc": "2026-10-21T18:17:24.735397+00:00"}
{"exit": 0, "fields": {"created_display_absent": false}, "step": "display teardown verified", "utc": "2026-10-21T18:17:24.736350+00:00"}
{"exit": 0, "fields": {"value": "0"}, "step": "teardown enable_freeform_support", "utc": "2026-10-21T18:17:24.833690+00:00"}
{"exit": 0, "fields": {"value": "0"}, "step": "teardown force_resizable_activities", "utc": "2026-10-21T18:17:24.921914+00:00"}
{"exit": 0, "fields": {"focus_on_created_display": [], "window_display_ids": [0]}, "step": "post-teardown window", "utc": "2026-10-21T18:17:56.785263+00:00"}
{"exit": 0, "fields": {"logical_display_ids": [0]}, "step": "post-teardown display", "utc": "2026-10-21T18:17:56.960602+00:00"}
{"exit": 0, "fields": {"activity_display_ids": [0], "created_tasks_on_physical": [], "created_tasks_on_secondary": []}, "step": "post-teardown activity", "utc": "2026-10-21T18:17:57.079158+00:00"}
```

## Anexo: extrator final de campos de dumpsys

Código próprio usado na leitura corrigida, incluído para revisão/reprodução com
saída novamente filtrada. Não contém dump ou identificador do aparelho.
SHA-256 do texto do extrator: `b7d4bb36a50a8bb6eae0adbc585c7525156a16cb74cb33597a25f7d2f15cf44d`.

```python
import re
MODES={'undefined','fullscreen','freeform','pinned','split-screen-primary','split-screen-secondary','multi-window','1','2','3','4','5','6','0'}
def ids(text):
    return sorted(set(int(x) for x in re.findall(r'(?:Display #(\d+)|Display: mDisplayId=(\d+))',text) for x in x if x))
def component(line,approved):
    for pkg in approved:
        match=re.search(r'(?<![\w.])'+re.escape(pkg)+r'/((?:\.[A-Za-z_]|[A-Za-z_][A-Za-z0-9_]*\.)[A-Za-z0-9_.]*)',line)
        if match: return pkg+'/'+match.group(1)
    return None

def activities(text,display,approved):
    current=None; stack=[]; tasks=[]; display_indent=None
    for line in text.splitlines():
        h=re.match(r'\s*Display #(\d+)\b',line)
        if h: current=int(h.group(1));stack=[];display_indent=len(line)-len(line.lstrip());continue
        if line.strip() and display_indent is not None and len(line)-len(line.lstrip())<=display_indent: current=None;stack=[]
        if current!=display: continue
        indent=len(line)-len(line.lstrip())
        t=re.match(r'\s*\*?\s*Task\{[^\n]*?#(\d+)\b',line)
        if t:
            while stack and indent<=stack[-1][0]: stack.pop()
            item={'task_id':int(t.group(1)),'display_id':display,'parent_task_id':stack[-1][1]['task_id'] if stack else None,'components':[],'configurations':[]}
            for pkg in approved:
                if re.search(r'(?<![\w.])'+re.escape(pkg)+r'(?![\w.])',line): item['package']=pkg
            m=re.search(r'\bmode=([\w-]+)',line)
            if m and m.group(1) in MODES: item['task_mode']=m.group(1)
            for key in ['visible','visibleRequested']:
                m=re.search(r'\b'+key+r'=(true|false)',line)
                if m:item[key]=m.group(1)=='true'
            tasks.append(item);stack.append((indent,item));continue
        if not stack:continue
        # Only explicit approved component records authorize activity fields.
        comp=component(line,approved)
        item=stack[-1][1]
        if comp and comp not in item['components']: item['components'].append(comp)
        if item.get('package') or item['components']:
            label=next((key for key in ['mRequestedOverrideConfig','mResolvedOverrideConfig','mOverrideConfig','mFullConfiguration','mLastReportedConfiguration'] if re.match(r'\s*'+key+r'\s*[=:]',line)),None)
            if label:
                config={'source':label}
                mode=re.search(r'\bmWindowingMode=([\w-]+)',line)
                if mode and mode.group(1) in MODES:config['mode']=mode.group(1)
                bounds=re.search(r'\bmBounds=Rect\((-?\d+),\s*(-?\d+)\s*-\s*(-?\d+),\s*(-?\d+)\)',line)
                if bounds:config['bounds']=[int(n) for n in bounds.groups()]
                if len(config)>1:item['configurations'].append(config)
    return [t for t in tasks if t.get('package') or t['components']]

def windows(text,display,approved):
    current=None; rows=[]
    for line in text.splitlines():
        h=re.match(r'\s*Display: mDisplayId=(\d+)\b',line)
        if h:current=int(h.group(1));continue
        if line and not line[0].isspace():current=None
        if current!=display:continue
        for field in ['mCurrentFocus','mFocusedApp']:
            if re.match(r'\s*'+field+r'=',line):
                comp=component(line,approved)
                rows.append({'display_id':display,'field':field,'component':comp or 'none-or-other-redacted'})
    return rows

if __name__=='__main__':
    import json
    secret='SECRET_SENTINEL'
    sample='''ACTIVITY MANAGER ACTIVITIES
Display #0 (activities from top to bottom):
  * Task{abc #10 A=private.pkg mode=freeform visible=true}
    mOverrideConfig={ mBounds=Rect(1, 2 - 3, 4) mWindowingMode=freeform }
Display #7 (activities from top to bottom):
  * Task{abc #20 A=2000000:com.android.settings mode=fullscreen visible=true visibleRequested=true}
    * Hist #0: ActivityRecord{a com.android.settings/.Settings t20}
    intent={ dat=SECRET_SENTINEL }
    mOverrideConfig={ mBounds=Rect(0, 0 - 1280, 720) mWindowingMode=fullscreen }
  * Task{abc #21 A=private.pkg mode=freeform visible=true}
    intent={ dat=SECRET_SENTINEL }
'''
    out=activities(sample,7,['com.android.settings'])
    assert component('com.android.settings/2000000', ['com.android.settings']) is None
    assert component('com.android.settings/u99a99999', ['com.android.settings']) is None
    assert len(out)==1 and out[0]['task_mode']=='fullscreen' and out[0]['configurations'][0]['bounds']==[0,0,1280,720]
    assert secret not in json.dumps(out) and 'private.pkg' not in json.dumps(out)
    assert ids(sample)==[0,7]
    w=windows('''WINDOW MANAGER DISPLAY CONTENTS
  Display: mDisplayId=0
    mCurrentFocus=Window{x private.pkg/SECRET_SENTINEL}
  Display: mDisplayId=7
    mCurrentFocus=Window{x com.android.settings/.Settings}
''',7,['com.android.settings'])
    assert len(w)==1 and secret not in json.dumps(w)
    print('display/task/component privacy filter synthetic checks: PASS')
```

## Verificação documental da entrega

`git diff --check`: exit 0. Links locais dos três artefatos novos verificados;
extrator incorporado recompilado/executado com testes sintéticos: PASS; SHA-256
do código incorporado confere. Os 69 registros JSONL foram decodificados e
inspecionados contra componentes com formato de UID. Scan explícito do serial
do alvo, caminhos pessoais e pseudo-componentes observados: PASS nos artefatos
novos. Somente a linha freeform da matriz foi alterada. Nenhum teste Cargo ou
código de produto foi alterado/executado para validar esta mudança documental.
