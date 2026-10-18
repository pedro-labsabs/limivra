# SPIKE-001 — prova Android virtual display no Linux

**Status:** NÃO EXECUTADO; versão da ROM e do scrcpy ainda desconhecidas. **Prioridade:** primeira atividade técnica. **Meta:** decidir se Redmi Note 13 4G com HyperOS pode exibir/controlar uma tela secundária Android sem root.

## Pré-condições e consentimento
- PC Linux próprio; celular próprio; cabo USB de dados; acesso local ao aparelho para aceitar pedido ADB.
- Nenhum root, unlock, mudança permanente de sistema, scripts de inicialização, alteração em apps bancários ou permissões elevadas.
- Usar comandos sem `sudo`; não gravar logcat, screenshots de conversas, nem serial IMEI ou tokens no repositório.
- Validar flags pela versão: `scrcpy --help`, `scrcpy --version`.

## A — inventário e baseline
```bash
command -v adb; adb version
command -v scrcpy; scrcpy --version
adb devices -l
adb shell getprop ro.product.model
adb shell getprop ro.build.version.release
adb shell getprop ro.build.version.sdk
adb shell getprop ro.build.version.incremental
```
Registre fabricante/modelo, Android/SDK, versão HyperOS (sem identificador único), kernel Linux, Wayland/X11, scrcpy, ADB, se aparecia "unauthorized" e resultado da autorização.

## B — controle do display físico
```bash
scrcpy --no-audio
```
Observar vídeo, clique, rolagem, digitação, encerramento via fechar janela. Se bloqueio Xiaomi de input, registrar sem habilitar opções de segurança automaticamente. Opcionalmente avaliar com o proprietário mudança reversível e documentada, mas não é requisito para continuar.

## C — segundo display
**Após fechar a instância anterior:**
```bash
scrcpy --new-display=1280x800 --start-app=com.android.settings --no-audio
```
Observar: (1) cria janela Linux? (2) o app Configurações aparece? (3) input funciona? (4) display físico segue independente? (5) display some ao encerrar? (6) que erro aparece quando falha? Não confundir tela preta por launcher ausente com ausência de suporte a display virtual; documentar logs de execução sanitizados.

**Não presumir** que o Android habilita freeform. Não usar `settings put`, mudanças de `wm`, root ou opções de desenvolvedor intrusivas neste spike.

## D — multijanela (exploração opcional e separada)
Com a sessão ativa, tentar abrir outro app autorizado pelo usuário e observar comportamento. Critério positivo: **duas janelas Android livres simultâneas, com redimensionamento verificável**, não mera alternância de apps.

## Resultado e classificação por modo
| Prova | PASS / FAIL / BLOCKED / UNTESTED | Evidência |
|---|---|---|
| ADB autorizado | UNTESTED | |
| Mirroring + input | UNTESTED | |
| Display virtual criado | UNTESTED | |
| App lançado no display virtual | UNTESTED | |
| Input no display virtual | UNTESTED | |
| Tela física independente | UNTESTED | |
| Freeform real (opcional) | UNTESTED | |

Anotar comandos, versão, stdout/stderr sem dados privados, comportamento visual descrito, limitações e hipótese. Se virtual falhar, validar `MIRROR` como fallback viável, não exigir root. Testes físicos só podem ser marcados feitos com evidência real.

## Gates
- **PASS Nível 1:** baseline mirror+input funcional.
- **PASS Nível 2:** display virtual + app e input, display físico independente.
- **PASS Nível 3:** mais de uma janela livre observável.
- **BLOCKED:** sem acesso/consentimento físico ou ferramentas essenciais indisponíveis.
- **FAIL:** experimento tentado corretamente e funcionalidade negada; registrar causa, opções de fallback.

## Fontes oficiais
https://github.com/Genymobile/scrcpy
https://github.com/Genymobile/scrcpy/blob/master/doc/virtual_display.md
https://source.android.com/docs/core/display/multi_display
