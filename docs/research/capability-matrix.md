# Matriz de Compatibilidade Android & Linux — Limivra

**Última Atualização:** 2026-10-21
**Milestone:** M2 — Virtual Display & Fallback
**Status de Validação:** Baseado em testes empíricos executados em hardware físico e verificação formal de código.

---

## 1. Princípios de Classificação

Conforme estipulado no [`AGENTS.md`](../../AGENTS.md) e na estratégia de teste do projeto:
- **`SUPPORTED`**: Exclusivamente atribuído quando há prova reproduzível em hardware real anexada no repositório.
- **`DEPENDENT_ON_OEM`**: Mecanismo permitido pelas APIs de sistema, porém condicionado a autorizações manuais específicas da fabricante (ex: opções de segurança do desenvolvedor na MIUI/HyperOS).
- **`UNSUPPORTED`**: Funcionalidade comprovadamente bloqueada ou incompatível no ambiente avaliado sem modificação intrusiva de sistema.
- **`UNPROVEN / UNTESTED`**: Recursos não observados empiricamente; proibido assumir funcionamento sem evidência rastreável.

---

## 2. Hardware e Ambientes Avaliados

### 2.1 Perfil de Referência Real (Dispositivo Físico Conectado)
- **Aparelho:** Xiaomi Redmi Note 13 4G (Código comercial: `23129RA5FL`, Produto: `sapphire_global`)
- **Processador / Gráficos:** Qualcomm Snapdragon 685 (SM6225-AD) / Adreno 610
- **Sistema Operacional:** Android 15 (API / SDK 35)
- **ROM / Customização:** Xiaomi HyperOS 2 (`OS2.0.213.0.VNGMIXM`)
- **Condição de Acesso:** Sem root, sem desbloqueio de bootloader, depuração USB padrão autorizada.
- **Host Linux:** Ubuntu 24.04 LTS (Noble) x86_64, Kernel `7.0.0-30-generic`, Servidor Gráfico X11 (`DISPLAY=:0`), Mesa 25.2.8 (OpenGL 4.6 + VA-API aceleração por hardware).
- **Subsubsistemas:** ADB 1.0.41 (platform-tools 37.0.1), scrcpy v5.0.1 (SDL 3.4.18 + FFmpeg VA-API).

---

## 3. Matriz de Capacidades por Modo de Operação

| Recurso / Modo | Classificação | Evidência Documentada | Comportamento Observado e Requisitos |
|---|---|---|---|
| **Sessão USB e Detecção ADB** | `SUPPORTED` | [QA-001](../testing/evidence/2026-10-21-usb-session-qa-evidence.md) | Dispositivo detectado e autorizado automaticamente via `DeviceDiscovery`. Ciclo de vida completo validado. |
| **Sondagem de Hardware (`limivra probe`)** | `SUPPORTED` | [QA-002](../testing/evidence/2026-10-21-m2-virtual-display-qa-evidence.md) | Extrai modelo, Android 15, build incremental HyperOS e displays via `dumpsys display` (1080x2400 @ 60Hz, TRUSTED). |
| **Espelhamento (Mirror Mode)** | `SUPPORTED` | [QA-001](../testing/evidence/2026-10-21-usb-session-qa-evidence.md) | Transmissão de vídeo de baixa latência acelerada por hardware via VA-API (864x1920) estável e sem artefatos. |
| **Criação de Display Virtual (`--new-display`)** | `DEPENDENT_ON_OEM` | [QA-002](../testing/evidence/2026-10-21-m2-virtual-display-qa-evidence.md) | A criação do display secundário via Shell UID 2000 é **permitida** pelo HyperOS 2 (`New display id=3/4`). Resolução de 1280x720 recomendada devido a limites do MediaCodec a 1080p. |
| **Lançamento Direcionado de App (`--start-app` / `am start`)** | `SUPPORTED` | [QA-002](../testing/evidence/2026-10-21-m2-virtual-display-qa-evidence.md) | Comprovado lançamento de `com.android.settings` diretamente no display virtual secundário sem launcher de sistema. |
| **Injeção de Entrada (SDK Input - Toque/Mouse)** | `DEPENDENT_ON_OEM` | [QA-002](../testing/evidence/2026-10-21-m2-virtual-display-qa-evidence.md) | Bloqueado por padrão pela MIUI/HyperOS (`SecurityException: INJECT_EVENTS`). Exige ativação manual de *"Depuração USB (Configurações de Segurança)"* nas Opções de Desenvolvedor. |
| **Injeção de Entrada (Kernel UHID - Teclado/Mouse)** | `SUPPORTED` | [QA-002](../testing/evidence/2026-10-21-m2-virtual-display-qa-evidence.md) | Acesso ao nó `/dev/uhid` permitido via shell (`-K -M`), contornando a restrição de injeção SDK sem elevar privilégios. |
| **Fallback Explícito (Virtual ➔ Mirror)** | `SUPPORTED` | [QA-002](../testing/evidence/2026-10-21-m2-virtual-display-qa-evidence.md) | Caso o display virtual falhe no spawn ou no host, o Limivra emite `FallbackTriggered` com causa transparente e mantém o controle em Mirror. |
| **Janelas Livres / Freeform Desktop (`WINDOWING_MODE_FREEFORM`)** | `FAIL` para duas janelas livres no cenário; floating individual observado | [QA-002](../testing/evidence/2026-10-21-m2-virtual-display-qa-evidence.md), [SPIKE-028](../testing/evidence/2026-10-21-spike-028-freeform.md), [finding](findings/SPIKE-028-freeform-windowing.md) | No HyperOS 2 / Android 15 / build OS2.0.213.0.VNGMIXM, Configurações permaneceu fullscreen mesmo em tarefa nova com modo 5; Calculadora ficou freeform e aceitou resize programático. Não foram comprovadas duas janelas livres nem desktop/launcher completo. Resultado restrito aos apps/configuração ensaiados; refina a leitura histórica de ausência total de freeform, sem habilitar DESKTOP_WINDOWING ou bloquear MVP. |
| **Teardown Seguro e Idempotente** | `SUPPORTED` | [QA-001](../testing/evidence/2026-10-21-usb-session-qa-evidence.md) | Encerramento limpo via `Ctrl+C` com destruição da tela virtual pelo servidor e **zero processos órfãos** no Linux host. |

---

## 4. Recomendações e Políticas de Execução do Limivra

1. **Seleção de Modo Padrão:**
   - Em dispositivos Xiaomi/HyperOS, o modo padrão recomendado é **Mirror**.
   - O modo **VirtualDisplay** deve ser acionado com salvaguardas claras e aviso sobre a necessidade de autorização de injeção de segurança.
2. **Proibição de Afirmações Fictícias:**
   - Nunca sugerir que o sistema suporta multijanela livre de área de trabalho sem teste observável de mais de uma janela móvel simultânea.
3. **Resolução Recomendada para VirtualDisplay:**
   - Devido ao limite de buffers do MediaCodec no SoC Snapdragon 685, telas virtuais secundárias devem utilizar preferencialmente resolução `1280x720` a 160/240 DPI para garantir codificação contínua a 60 FPS.
