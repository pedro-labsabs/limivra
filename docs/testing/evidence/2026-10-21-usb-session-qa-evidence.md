# Evidência de QA — Prova Ponta a Ponta de Sessão USB no Linux

**Data de Execução:** 2026-10-21
**Dispositivo Real Alvo:** Xiaomi Redmi Note 13 4G (`23129RA5FL`) — Android 15 (HyperOS)
**Host de Teste:** Ubuntu 24.04 LTS (Noble) x86_64, Linux kernel `7.0.0-30-generic`, Servidor Gráfico X11 (`DISPLAY=:0`), Mesa 25.2.8 (OpenGL 4.6 + VA-API aceleração por hardware).
**Backends:** ADB 37.0.1 (Google platform-tools), scrcpy v5.0.1 (SDL3 + FFmpeg vaapi).

---

## 1. Procedimento Executado

O comando de entrada do Limivra foi disparado contra o dispositivo físico conectado por cabo USB:

```bash
cargo run -- start
```

### 1.1 Sequência de Estados Observada no Núcleo

1. **Descoberta e Seleção Automática (`DeviceDiscovery`):**
   - O daemon ADB retornou o dispositivo: `674d6cf9` (`product:sapphire_global model:23129RA5FL`).
   - O dispositivo foi validado com estado `Device` (autorizado e pronto).
2. **Inspeção de Dependência (`ScrcpyAdapter`):**
   - Detectada a versão `scrcpy v5.0.1`.
3. **Construção de Parâmetros e Spawn Seguro (`SessionController`):**
   - Transição: `Disconnected` ➔ `Connecting` ➔ `Active { serial: "674d6cf9", mode: Mirror }`.
   - O subprocesso foi inicializado e vinculado ao `SessionController`.
4. **Renderização de Vídeo:**
   - Servidor `scrcpy-server` transferido e executado no Android a 588.7 MB/s.
   - Renderer gráfico inicializado com sucesso no X11:
     - `INFO: Renderer: opengl`
     - `INFO: Interop: vaapi`
     - `INFO: Video decoding: vaapi`
     - `INFO: Texture (VA-API): 864x1920`
5. **Teardown e Sanitização:**
   - Encerramento limpo da sessão via timeout/interrupção controlada.
   - Verificação pós-sessão (`ps aux | grep -iE "scrcpy|limivra"`): **Zero processos órfãos ou zumbis restantes no sistema operacional**.

---

## 2. Observação Específica de Plataforma (HyperOS / Xiaomi Security)

Durante a injeção inicial de input, o servidor Android reportou:
```text
[server] ERROR: Injecting input events requires the caller to have the INJECT_EVENTS permission.
[server] ERROR: Make sure you have enabled "USB debugging (Security Settings)" and then rebooted your device.
```

- **Diagnóstico:** No HyperOS / MIUI (Xiaomi), o espelhamento de vídeo funciona imediatamente com "Depuração USB" ativa. O controle de cliques de mouse/teclado exige ativar adicionalmente a opção **"Depuração USB (Configurações de Segurança)"** nas Opções do Desenvolvedor do aparelho.
- **Resiliência do Limivra:** O núcleo do Limivra manteve a sessão de vídeo ativa e gerenciada, demonstrando que a falha de injeção de input não provoca travamento (crash) da aplicação host.

---

## 3. Conclusão

- **Vídeo USB Ponta a Ponta:** COMPROVADO (Aceleração por hardware VA-API 864x1920 ativa).
- **Ciclo de Vida e Teardown:** COMPROVADO (Transições de estado registradas, zero processos órfãos).
- **Critérios de Aceite da Issue #20:** ATENDIDOS.
