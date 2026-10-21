# SPIKE-001 — prova Android virtual display no Linux

**Status:** EXECUTADO COM SUCESSO (2026-10-21). **Meta:** decidir se Redmi Note 13 4G com HyperOS pode exibir/controlar uma tela secundária Android sem root.

## Pré-condições e consentimento
- PC Linux Ubuntu 24.04 LTS; Xiaomi Redmi Note 13 4G (`23129RA5FL`); cabo USB de dados; depuração USB autorizada.
- Nenhum root, unlock, mudança permanente de sistema, scripts de inicialização, alteração em apps bancários ou permissões elevadas.
- Comandos sem `sudo`; identificadores e caminhos pessoais sanitizados.
- scrcpy v5.0.1 (SDL 3.4.18 + VA-API) e ADB 1.0.41 (platform-tools 37.0.1).

## A — inventário e baseline
```bash
adb version # 1.0.41 / platform-tools 37.0.1
scrcpy --version # scrcpy 5.0.1
adb devices -l # 674d6cf9 device usb:1-4 product:sapphire_global model:23129RA5FL device:sapphire
adb shell getprop ro.product.model # 23129RA5FL
adb shell getprop ro.build.version.release # 15
adb shell getprop ro.build.version.sdk # 35
adb shell getprop ro.build.version.incremental # OS2.0.213.0.VNGMIXM
```
- Fabricante/Modelo: Xiaomi / 23129RA5FL (Redmi Note 13 4G)
- Sistema: Android 15 (SDK 35), HyperOS 2 (`OS2.0.213.0.VNGMIXM`)
- Host: Linux kernel 7.0.0-30-generic, X11, Mesa 25.2.8 (OpenGL 4.6 + VA-API)

## B — controle do display físico
```bash
scrcpy -s 674d6cf9 --no-audio
```
- Vídeo estabelecido em 864x1920 acelerado por hardware via VA-API.
- Encerramento limpo sem processos órfãos.

## C — segundo display
```bash
scrcpy -s 674d6cf9 --new-display=1280x720 --start-app=com.android.settings --no-audio
```
- Display virtual criado com sucesso pelo servidor (`[server] INFO: New display: 1280x720/234 (id=4)`).
- Aplicativo Configurações iniciado na tela secundária (`Starting app "Configurações" on display 4`).
- Tela física continuou independente.
- Input via SDK bloqueado por restrição OEM Xiaomi (`INJECT_EVENTS` exige opção de desenvolvedor *"Depuração USB - Configurações de Segurança"*).
- Input alternativo via UHID Kernel (`-K -M`) funciona sem restrição do SDK.
- Resolução de 1920x1080 apresenta limite de MediaCodec no Snapdragon 685, tornando 1280x720 a resolução recomendada.

## D — multijanela (exploração de janelas livres)
- O display virtual não disponibilizou gerenciador de janelas flutuantes/livres AOSP (`WINDOWING_MODE_FREEFORM`).
- Não declarar suporte a freeform no produto.

## Resultado e classificação por modo
| Prova | PASS / FAIL / BLOCKED / UNTESTED | Evidência |
|---|---|---|
| ADB autorizado | PASS | `674d6cf9 device transport_id:1` |
| Mirroring + input | PASS | `docs/testing/evidence/2026-10-21-usb-session-qa-evidence.md` |
| Display virtual criado | PASS | `[server] INFO: New display: 1280x720/234 (id=4)` |
| App lançado no display virtual | PASS | `Starting app "Configurações" on display 4` |
| Input no display virtual | BLOCKED (SDK) / PASS (UHID) | SDK exige opção de segurança Xiaomi; UHID (`-K -M`) contorna |
| Tela física independente | PASS | Tela integrada (id 0) segue operando sem espelhar |
| Freeform real (opcional) | FAIL / UNPROVEN | Sem janelas livres observáveis na ROM HyperOS |

## Gates atingidos
- **PASS Nível 1:** baseline mirror+input funcional.
- **PASS Nível 2 (Parcial):** display virtual + app e tela física independente funcional; input restrito no SDK por política da Xiaomi ou contornável via UHID.
- **PASS Nível 3:** NÃO ATINGIDO (Freeform ausente).
- **FALLBACK MIRROR:** Validado e implementado em `VirtualDisplayProvider`.

## Fontes oficiais
- https://github.com/Genymobile/scrcpy
- https://github.com/Genymobile/scrcpy/blob/master/doc/virtual_display.md
- https://source.android.com/docs/core/display/multi_display
