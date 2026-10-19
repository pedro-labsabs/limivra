# Mapeamento Documental AOSP e scrcpy: Secondary Displays, Input e Freeform

**Status:** `RESEARCH / DOCUMENTARY / UNTESTED-ON-DEVICE`
**Data de consolidação:** 19 de outubro de 2026
**Issue de referência:** [Issue #13](https://github.com/pedro-labsabs/limivra/issues/13)
**Política regulatória:** [docs/research/rea-provenance-policy.md](../../research/rea-provenance-policy.md) (ratificada sob [Issue #12](https://github.com/pedro-labsabs/limivra/issues/12) / [PR #38](https://github.com/pedro-labsabs/limivra/pull/38) — escopo CAT-A/B)
**Classificação de evidência:** Documental pública (AOSP Trunk/API Reference e repositório oficial scrcpy versionado). Nenhuma afirmação contida neste relatório certifica comportamento em hardware específico (ex.: Redmi Note 13 4G / HyperOS) sem teste físico observável subordinado à Issue #9.

---

## 1. Contexto, Mandato e Limites

Este documento consolida a investigação estritamente documental e pública sobre as superfícies de sistema operacional Android (AOSP) e ferramentas abertas consolidadas (scrcpy) no tocante ao gerenciamento de displays múltiplos, displays virtuais, injeção/roteamento de entrada e suporte a janelas flutuantes/livres (*freeform windowing*).

### 1.1 Mandato da Issue #13
Conforme deliberado e aprovado na [Issue #12](https://github.com/pedro-labsabs/limivra/issues/12), o presente trabalho é restrito à categoria CAT-A (documentação pública, AOSP e manuais abertos) e CAT-B (análise do código-fonte público do scrcpy).

### 1.2 Limites Inegociáveis (Não Executados)
- **Zero execução em hardware:** Nenhum comando ADB foi disparado contra dispositivos físicos ou emuladores.
- **Zero ferramentas de REA binário:** Nenhuma ferramenta de engenharia reversa binária (`morluto/rea`, Ghidra, Frida, Jeb, Rizin, Cutter) foi executada.
- **Sem binários ou assets proprietários:** Nenhum trecho decompilado, classe privada ou protocolo proprietário (como Samsung DeX) foi inspecionado ou transferido para este documento.
- **Sem alteração de arquitetura ou produto:** Nenhuma linha de código de produto (Rust/Qt) ou ADR de arquitetura foi modificada.
- **Não promessa de compatibilidade:** A existência de uma API no AOSP não implica que a fabricante (Xiaomi/HyperOS) a mantenha ativada, inalterada ou funcional no aparelho de teste.

---

## 2. Taxonomia Rigorosa: Mirror vs Virtual Display vs Desktop Windowing

Um erro conceitual frequente em projetos de integração Android-Linux é tratar espelhamento, display secundário e modo desktop como sinônimos. Para o Limivra, essa distinção técnica é fundamental para delimitar arquitetura e viabilidade.

```
+---------------------------------------------------------------------------------------------------+
|                                       CONCEITOS DE DISPLAY NO AOSP                                 |
+------------------------------------+----------------------------------+---------------------------+
| 1. MIRRORING                       | 2. VIRTUAL DISPLAY               | 3. DESKTOP WINDOWING      |
|    (Espelhamento de Buffer)        |    (Superfície Independente)     |    (Gerenciador Livre)    |
+------------------------------------+----------------------------------+---------------------------+
| * Duplica a saída de exibição      | * Cria uma instância com         | * Modelo de apresentação  |
|   existente (DEFAULT_DISPLAY).     |   Display ID distinto (id != 0). |   com janelas livres,     |
| * Compartilha o mesmo ciclo de     | * Possui métricas próprias:      |   redimensionáveis        |
|   renderização e orientação do     |   resolução, densidade (DPI) e   |   (freeform/floating).    |
|   display físico nativo.           |   orientação.                    | * Pode ocorrer no display |
| * Sem capacidade nativa de rodar   | * Pode hospedar tarefas          |   principal ou em um      |
|   atividades independentes sem     |   próprias se configurado como   |   virtual display se o    |
|   afetar a tela local.             |   público e com flag de toque.   |   WindowManager suportar. |
| * Não requer permissões especiais  | * Criação requer privilégios     | * Depende de flag da ROM  |
|   para exibição padrão de tela.    |   específicos conforme flags.    |   (enable_freeform_support|
+------------------------------------+----------------------------------+---------------------------+
```

### 2.1 Mirroring (Espelhamento)
- **Definição técnica:** Transmissão síncrona ou assíncrona do framebuffer do display primário (`Display.DEFAULT_DISPLAY`, ID `0`) para um receptor externo (ex.: scrcpy padrão via `SurfaceControl` ou `MediaProjection`).
- **Comportamento no AOSP:** O conteúdo visualizado na tela do celular é exatamente o reproduzido no cliente remoto. Alterações de orientação física giram a captura; bloqueio da tela principal pode suspender a renderização.
- **Limitação de produtividade:** Não permite multitarefa desacoplada (o usuário não pode interagir com um aplicativo no PC e outro no celular simultaneamente na mesma tela sem interrupção).

### 2.2 Virtual Display (Display Virtual Secundário)
- **Definição técnica:** Instância de `android.view.Display` criada via `DisplayManager.createVirtualDisplay(...)` associada a uma `android.view.Surface` em memória, recebendo um `displayId` próprio (`displayId != DEFAULT_DISPLAY`).
- **Comportamento no AOSP:** Opera como um display secundário lógico. Se criado com os privilégios corretos (`VIRTUAL_DISPLAY_FLAG_PUBLIC`, `VIRTUAL_DISPLAY_FLAG_TRUSTED`, `VIRTUAL_DISPLAY_FLAG_OWN_CONTENT_ONLY`), pode hospedar pilhas de atividades independentes (`ActivityStack` / `Task`), permitindo que um aplicativo rode na resolução remota sem interferir no display físico primário.
- **Relação com IME e Decorações:** O AOSP permite configurar decorações de sistema (barra de navegação, status bar, launcher dedicado) e política de teclado virtual (`DisplayWindowSettings#shouldShowSystemDecorsLocked`, `setDisplayImePolicy`).

### 2.3 Desktop Windowing (Modo Desktop / Freeform)
- **Definição técnica:** Modo de gerenciamento de janelas (`WINDOWING_MODE_FREEFORM`) em que as atividades são dispostas em retângulos redimensionáveis com barra de título e controles de janela, em vez do modo tela cheia (`WINDOWING_MODE_FULLSCREEN`) ou tela dividida (`WINDOWING_MODE_MULTI_WINDOW`).
- **Comportamento no AOSP:** Suportado formalmente pelo `WindowManagerService`. No AOSP base, requer ativação de opções de desenvolvedor (`enable_freeform_support`) ou configurações de sistema. O fato de um display virtual existir **não ativa automaticamente** janelas flutuantes se a ROM sobrepuser o gerenciador de janelas padrão com sua própria UI proprietária.

---

## 3. Mapeamento das APIs AOSP, Permissões e Restrições de Nível

A arquitetura de display e injeção do AOSP é estratificada em níveis rígidos de isolamento e segurança.

### 3.1 Fontes Primárias AOSP Consultadas
1. [AOSP Multi-Display Overview](https://source.android.com/docs/core/display/multi_display) (Atualizado para o ciclo trunk AOSP 2026).
2. [AOSP Activity Launch Policy](https://source.android.com/docs/core/display/multi_display/activity-launch).
3. [AOSP Recommended Practices](https://source.android.com/docs/core/display/multi_display/recommended-practices).
4. [AOSP Input Routing](https://source.android.com/docs/core/display/multi_display/input-routing).
5. [AOSP System Decorations](https://source.android.com/docs/core/display/multi_display/system-decorations).

### 3.2 Estratificação de Permissões e Privilégios

```
+-------------------+-----------------------------------+---------------------------------------------+
| Nível de Acesso   | Identificador / Contexto          | Capacidades e Restrições                    |
+-------------------+-----------------------------------+---------------------------------------------+
| App Normal        | UID de terceiro (unprivileged)    | - Cria apenas VirtualDisplay privado.       |
| (Sandbox)         |                                   | - Lança apenas suas próprias atividades     |
|                   |                                   |   (ou com ActivityInfo.FLAG_ALLOW_EMBEDDED).|
|                   |                                   | - Proibido de injetar eventos globais de    |
|                   |                                   |   toque/teclado em outras apps.             |
+-------------------+-----------------------------------+---------------------------------------------+
| Shell ADB         | UID 2000 (shell)                  | - Pode invocar comandos do sistema via IPC. |
|                   |                                   | - Criação de VirtualDisplay com flags       |
|                   |                                   |   avançadas se invocado via processo Java   |
|                   |                                   |   executando sob o token do shell.          |
|                   |                                   | - Permissão INJECT_EVENTS disponível no     |
|                   |                                   |   AOSP base, mas frequentemente restrita    |
|                   |                                   |   por fabricantes ("Security Settings").    |
+-------------------+-----------------------------------+---------------------------------------------+
| Sistema Privilegi.| UID 1000 (system) / system_server | - Posse de INTERNAL_SYSTEM_WINDOW e         |
|                   |                                   |   ACTIVITY_EMBEDDING irrestritos.           |
|                   |                                   | - Controle total de DisplayWindowSettings   |
|                   |                                   |   e roteamento de topologia de displays.    |
+-------------------+-----------------------------------+---------------------------------------------+
| Fabricante (OEM)  | Framework overlay / ROM vendor    | - Pode desativar livremente o suporte a     |
|                   |                                   |   displays secundários, restringir HID      |
|                   |                                   |   kernel e substituir o SystemUI launcher.  |
+-------------------+-----------------------------------+---------------------------------------------+
```

### 3.3 Regras Críticas de Lançamento de Atividades (Activity Launch Policy)
1. **Destino Padrão:** Por padrão, novas atividades iniciam no mesmo display do chamador (`caller context`).
2. **Lançamento Direcionado:** O chamador pode especificar `ActivityOptions.setLaunchDisplayId(int displayId)` ao disparar um `Intent`.
3. **Restrições de Segurança (Android 10+ / AOSP):**
   - Para evitar que aplicativos maliciosos leiam dados sensíveis desenhados em superfícies virtuais arbitrárias, o AOSP restringe o lançamento em displays virtuais não pertencentes ao chamador.
   - Chamadores precisam da permissão `ACTIVITY_EMBEDDING` e a atividade alvo precisa declarar `android:allowEmbedded="true"`, **a menos** que a tela seja de posse do sistema ou criada com a flag `VIRTUAL_DISPLAY_FLAG_TRUSTED` por uma entidade privilegiada.
   - Verificação interna do AOSP realizada via `ActivityStackSupervisor#isCallerAllowedToLaunchOnDisplay()` e `ActivityManager#isActivityStartAllowedOnDisplay()`.

### 3.4 Decorações e Entrada (System Decorations & Input Routing)
- **Decorações do Sistema:** Controladas por `DisplayWindowSettings` (persistido em `/data/system/display_settings.xml`). Define se o display secundário renderiza barra de navegação, papel de parede e IME (`shouldShowSystemDecors="true"`, `shouldShowIme="true"`).
- **Roteamento de Entrada (Android 10+):** A associação estática entre portas de dispositivos físicos e displays é realizada via `/vendor/etc/input-port-associations.xml`. Dispositivos dinâmicos (como touch virtual via scrcpy) necessitam de mapeamento no `InputManagerService`.

---

## 4. Mapeamento do scrcpy (Base Versionada: v5.0.1 / Commit a60891a)

Para evitar ambiguidades decorrentes de branches mutáveis (`master`), este relatório fixa a análise na release estável mais recente do scrcpy:

- **Versão:** `v5.0.1`
- **Tag:** `v5.0.1` (`refs/tags/v5.0.1` SHA: `a0c27ff382c411af1301dcc1cd8205c9aeb81996`)
- **Commit:** `a60891aea193d92e7e5c3942700eca63f9d19a5f` (Data: 2025/2026)
- **Histórico Relevante:** O suporte formal a Virtual Displays foi introduzido no **scrcpy v3.0** (Issue #5370, PR #5506) e refinado nas versões 4.x e 5.0.

### 4.1 Mecanismo de Virtual Display no scrcpy (`--new-display`)
O scrcpy não requer privilégios de `root` para criar displays virtuais; ele executa seu servidor Java (`scrcpy-server.jar`) através do `app_process` sob as credenciais do shell ADB (`UID 2000`).

Conforme verificado em `server/src/main/java/com/genymobile/scrcpy/video/NewDisplayCapture.java` e `server/src/main/java/com/genymobile/scrcpy/wrappers/DisplayManager.java`:
1. **Invocação:** O servidor instancia a classe oculta `android.hardware.display.DisplayManagerGlobal` via reflexão e chama `createNewVirtualDisplay(...)`.
2. **Flags Utilizadas na Criação:**
   - `VIRTUAL_DISPLAY_FLAG_PUBLIC` (1 << 0)
   - `VIRTUAL_DISPLAY_FLAG_PRESENTATION` (1 << 1)
   - `VIRTUAL_DISPLAY_FLAG_OWN_CONTENT_ONLY` (1 << 3)
   - `VIRTUAL_DISPLAY_FLAG_SUPPORTS_TOUCH` (1 << 6)
   - `VIRTUAL_DISPLAY_FLAG_ROTATES_WITH_CONTENT` (1 << 7)
   - Condicional (`vdDestroyContent`): `VIRTUAL_DISPLAY_FLAG_DESTROY_CONTENT_ON_REMOVAL` (1 << 8)
   - Condicional (`vdSystemDecorations`): `VIRTUAL_DISPLAY_FLAG_SHOULD_SHOW_SYSTEM_DECORATIONS` (1 << 9)
   - No Android 13+ (API 33+):
     - `VIRTUAL_DISPLAY_FLAG_TRUSTED` (1 << 10)
     - `VIRTUAL_DISPLAY_FLAG_OWN_DISPLAY_GROUP` (1 << 11)
     - `VIRTUAL_DISPLAY_FLAG_ALWAYS_UNLOCKED` (1 << 12)
     - `VIRTUAL_DISPLAY_FLAG_TOUCH_FEEDBACK_DISABLED` (1 << 13)
   - No Android 14+ (API 34+):
     - `VIRTUAL_DISPLAY_FLAG_OWN_FOCUS` (1 << 14)
     - `VIRTUAL_DISPLAY_FLAG_DEVICE_DISPLAY_GROUP` (1 << 15)
3. **Ciclo de Vida:** O display virtual criado pelo scrcpy é volátil e destruído na desconexão da sessão, exceto se configurado de forma diferente.

### 4.2 Lançamento de Aplicativos (`--start-app`)
- Conforme `doc/virtual-display.md`, se o display virtual não possuir um Launcher do sistema disponível (ou se `--no-vd-system-decorations` for passado), a tela permanece em branco (sem gerar frames de vídeo) até que uma aplicação seja iniciada.
- O scrcpy inicia o app alvo (`--start-app=<package>`) direcionando a janela especificamente para o `displayId` recém-criado, utilizando APIs internas de ActivityManager via ADB Shell.

### 4.3 Mecanismos de Entrada (Mouse e Teclado)
O scrcpy v5.0.1 suporta três modos de entrada, detalhados em `doc/control.md`, `doc/keyboard.md` e `doc/mouse.md`:

```
+--------------------+-----------------------+-------------------------+----------------------------------------------+
| Modo de Entrada    | Flag scrcpy           | Requisitos de Sistema   | Características e Restrições                 |
+--------------------+-----------------------+-------------------------+----------------------------------------------+
| SDK Input          | --keyboard=sdk        | USB Debugging ativo.    | Injeta MotionEvent/KeyEvent via IPC do       |
| (Padrão)           | --mouse=sdk           | UID 2000 (Shell).       | InputManagerService.                         |
|                    |                       |                         | * Falha no Xiaomi/HyperOS se a opção         |
|                    |                       |                         |   "USB debugging (Security settings)"        |
|                    |                       |                         |   estiver desativada (INJECT_EVENTS negado). |
+--------------------+-----------------------+-------------------------+----------------------------------------------+
| UHID               | --keyboard=uhid (-K)  | Acesso ao nó de kernel  | Emula dispositivo físico USB HID real        |
| (Kernel HID)       | --mouse=uhid (-M)     | /dev/uhid pelo shell.   | através do módulo de kernel UHID.            |
|                    |                       | Android 5.0+ (geralmente| * Suporta todos os caracteres de teclado e   |
|                    |                       | estável em Android 9+). |   desativa teclado virtual na tela nativa.   |
|                    |                       |                         | * Suporta captura de cursor de mouse.        |
+--------------------+-----------------------+-------------------------+----------------------------------------------+
| AOA (Android Open  | --keyboard=aoa        | Comunicação USB direta  | Emula acessório físico via protocolo USB     |
| Accessory 2.0)     | --mouse=aoa           | (OTG / hardware USB).   | AOAv2 sem passar pelo daemon adbd.           |
|                    |                       | Sem dependência do ADB. | * Funciona mesmo sem depuração USB ativada.  |
|                    |                       |                         | * Não suporta rede (apenas conexão USB).     |
+--------------------+-----------------------+-------------------------+----------------------------------------------+
```

---

## 5. Tabela de Capacidades: Fatos Documentais vs Hipóteses Técnicas

A matriz a seguir mapeia cada mecanismo documentado no AOSP e no scrcpy v5.0.1, distinguindo rigorosamente a verdade documental das hipóteses para o Limivra.

> **Regra de Aceite Rigorosa:** Nenhuma funcionalidade é declarada como suportada pelo dispositivo alvo (Redmi Note 13 4G / HyperOS) sem teste físico observável. Todas as linhas referentes a comportamento específico em ROM de fabricante são classificadas como `UNKNOWN / UNTESTED`.

```
+-------------------+----------------------------+-----------------+---------------------------+---------------------------------------+------------------------------------------+-----------------------+
| Mecanismo         | Fonte Primária e Ref       | Versão Mínima   | Permissões Requeridas     | Hipótese Técnica Limivra              | Riscos e Restrições de ROM               | Prova Ausente         |
+-------------------+----------------------------+-----------------+---------------------------+---------------------------------------+------------------------------------------+-----------------------+
| Display Mirroring | AOSP DisplayManager /      | API 21+         | Nenhuma além do shell ADB | Viável para fallback básico de        | Baixa flexibilidade; bloqueio de tela    | Teste observável de   |
| (Default Display) | scrcpy v5.0.1 (Base)       | (Android 5.0+)  | (UID 2000)                | espelhamento da tela principal.       | primária desliga renderização.           | FPS e latência local. |
+-------------------+----------------------------+-----------------+---------------------------+---------------------------------------+------------------------------------------+-----------------------+
| Virtual Display   | AOSP DisplayManagerGlobal/ | API 30+ (ideal  | Shell ADB (UID 2000) com  | Pode hospedar sessão Linux/Wayland ou | OEMs (especialmente Xiaomi/HyperOS)      | Teste físico sob      |
| Secundário        | scrcpy NewDisplayCapture   | API 33+ para    | acesso a APIs reflexivas  | apps específicos em resolução nativa  | frequentemente customizam ou quebram o   | SPIKE-001 (Issue #9)  |
| (--new-display)   | commit a60891a             | flags TRUSTED)  | ocultas de sistema.       | de monitor de desktop.                | DisplayManagerService; risco de tela     | com logcat e dumpsys. |
|                   |                            |                 |                           |                                       | preta se launcher decor falhar.          |                       |
+-------------------+----------------------------+-----------------+---------------------------+---------------------------------------+------------------------------------------+-----------------------+
| System UI Decor   | AOSP DisplayWindowSettings | API 29+         | Modificação requer shell  | Exibir barra de tarefas ou launcher   | HyperOS comumente omite ou quebra        | UNKNOWN / UNTESTED    |
| no Secundário     | scrcpy --vd-system-decors  | (Android 10+)   | ou flags de criação de VD | dedicado no display externo.          | launchers em telas secundárias virtuais  | na ROM HyperOS.       |
|                   |                            |                 | com decors habilitadas.   |                                       | sem cabo físico conectado.               |                       |
+-------------------+----------------------------+-----------------+---------------------------+---------------------------------------+------------------------------------------+-----------------------+
| Injeção SDK       | AOSP InputManager /        | API 21+         | INJECT_EVENTS             | Método padrão de interação com touch  | No HyperOS/MIUI, bloqueado por padrão;   | Verificação física da |
| de Eventos        | scrcpy --mouse=sdk         |                 | (Concedido ao UID 2000    | e cliques básicos no host.            | exige aprovação manual em "Depuração USB | opção no menu de      |
|                   |                            |                 | no AOSP base).            |                                       | (Configurações de segurança)" com SIM.   | desenvolvedor.        |
+-------------------+----------------------------+-----------------+---------------------------+---------------------------------------+------------------------------------------+-----------------------+
| Injeção UHID      | Linux Kernel UHID /        | API 28+         | Acesso de escrita ao nó   | Permite teclado com layout de desktop | Algumas ROMs restringem permissões de    | UNKNOWN / UNTESTED:   |
| (Kernel Device)   | scrcpy --keyboard=uhid     | (Ideal Android  | /dev/uhid pelo shell      | e cursor de mouse capturado sem       | SELinux no /dev/uhid para o UID do shell | verificar ls -l       |
|                   | PR #2632 / commit a60891a  | 9+)             | (geralmente permitido).   | depender de INJECT_EVENTS do Android. | em dispositivos de linha de entrada.     | /dev/uhid via adb.    |
+-------------------+----------------------------+-----------------+---------------------------+---------------------------------------+------------------------------------------+-----------------------+
| Janelas Livres    | AOSP WindowManager         | API 24+         | Configuração global do    | Permitir que aplicações rodem em      | O suporte a Freeform é frequentemente    | UNKNOWN / UNTESTED:   |
| (Freeform /       | enable_freeform_support    | (Android 7.0+)  | sistema ou flags de janela| janelas arbitrárias sobre o display   | desativado, substituído por "janelas     | testar se HyperOS     |
| Floating Windows) |                            |                 | livre.                    | virtual sem tiling fixo.              | flutuantes" proprietárias da fabricante. | aceita freeform AOSP. |
+-------------------+----------------------------+-----------------+---------------------------+---------------------------------------+------------------------------------------+-----------------------+
```

---

## 6. Diferenciação Factual: Documentação vs Hipótese vs Realidade

A governança do projeto Limivra exige a separação inequívoca das três esferas de conhecimento:

1. **Documentação Oficial (O que o AOSP e o scrcpy descrevem):**
   - O AOSP possui arquitetura documentada para displays múltiplos (`DEFAULT_DISPLAY` vs `secondary`), controle de lançamento (`ActivityOptions#setLaunchDisplayId`), roteamento de portas físicas (`/vendor/etc/input-port-associations.xml`) e decorações de sistema.
   - O scrcpy v5.0.1 possui código testado publicamente para criar displays virtuais via `DisplayManagerGlobal` com suporte a redimensionamento dinâmico (`--flex-display`) e inicialização de processos em displays específicos (`--start-app`).
   - O scrcpy documenta formalmente em seu README que dispositivos Xiaomi exigem autorização adicional para injeção via SDK ("USB debugging (Security Settings)").

2. **Hipótese Técnica do Limivra (O que o projeto projeta como possível):**
   - É hipotetizado que o servidor do Limivra possa utilizar técnica análoga ao scrcpy (ou compor com componentes abertos do scrcpy) para disponibilizar um display virtual secundário sem requerer privilégios de `root`.
   - É hipotetizado que uma sessão de desktop Wayland (ex.: compositor leve em container/chroot proot) possa emitir saída para o display virtual ou, alternativamente, servir de hospedeiro para tarefas Android isoladas.
   - É hipotetizado que a entrada de teclado e mouse seja mais estável via UHID, contornando a restrição de injeção de eventos em ROMs restritivas.

3. **Observação Real em Hardware (O que foi efetivamente provado):**
   - **NADA.** Nenhum comando foi executado no dispositivo físico de referência sob esta issue.
   - Não há evidência observável de que a ROM HyperOS do aparelho aceite as flags `VIRTUAL_DISPLAY_FLAG_TRUSTED` sem redefinir o display para espelhamento forçado.
   - Não há evidência observável de que o modo freeform esteja acessível sem quebra de compositor na ROM local.

---

## 7. Proposta de Experimentos Futuros (Subordinados à Issue #9 / SPIKE-001)

Qualquer teste prático decorrente desta pesquisa só poderá ser executado sob autorização humana explícita, consentimento de depuração USB e escopo estrito da [Issue #9](https://github.com/pedro-labsabs/limivra/issues/9) e [docs/research/SPIKE-001.md](../../research/SPIKE-001.md).

### 7.1 Protocolo Experimental Sugerido (Apenas Proposta — Não Executado)
1. **Teste 1 — Dumpsys Inicial e Permissões:**
   - Comando planejado: `adb shell dumpsys display`
   - Critério de observação: Verificar quais flags e displays secundários são reportados nativamente pela ROM em estado virgem.
2. **Teste 2 — Validação de Nó UHID:**
   - Comando planejado: `adb shell ls -l /dev/uhid`
   - Critério de observação: Verificar permissões de leitura/escrita do nó para o UID 2000 (shell).
3. **Teste 3 — Teste Experimental de Display Virtual via scrcpy v5.0.1:**
   - Comando planejado: `scrcpy --new-display=1280x720 --no-audio --dry-run` ou execução pontual com logcat.
   - Critério de observação: Observar se o servidor scrcpy reporta sucesso na criação do display (`New display: 1280x720/DPI (id=X)`) ou se o `DisplayManagerGlobal` lança `SecurityException` ou trava na renderização de frames.
4. **Teste 4 — Comportamento de Freeform na ROM:**
   - Comando planejado: Inspecionar suporte a `freeform_window_management` via `adb shell dumpsys package features | grep freeform`.
   - Critério de observação: Determinar se a fabricante removeu a feature do manifesto de sistema.

---

## 8. Conclusões, Riscos, Lacunas e Próximos Gates

### 8.1 Conclusões Documentais
- A documentação pública do AOSP e o código-fonte estável do scrcpy v5.0.1 comprovam a viabilidade conceitual de criação de displays secundários virtuais sem `root` a partir do shell ADB em sistemas Android 10+.
- No entanto, o sucesso dessa abordagem no ecossistema Android real é altamente dependente das modificações introduzidas por OEMs na camada de `system_server` e `WindowManager`.

### 8.2 Principais Riscos Identificados
- **Restrição de Injeção em ROMs Xiaomi/HyperOS:** Risco iminente de bloqueio de entrada se o usuário não conceder a permissão especial de segurança (que exige conta Mi e cartão SIM inserido em muitas versões da ROM).
- **Incompatibilidade com Telas Virtuais sem Launcher:** Se a ROM não instanciar uma interface gráfica para telas secundárias, o display virtual fica vazio a menos que uma atividade específica seja mantida ativa.
- **Divergência de Gerenciamento de Janelas:** Impossibilidade de janelas flutuantes AOSP caso a ROM imponha seu próprio framework de multitarefa proprietário.

### 8.3 Próximos Gates
1. **Gate SPIKE-001 (Issue #9):** Aguardar aprovação humana e conexão física do aparelho para executar a bateria mínima de inspeção de capacidades reais.
2. **Gate de Arquitetura (Issue #14):** Avaliar se o Limivra adotará uma arquitetura baseada em display virtual compartilhado ou espelhamento controlado antes de qualquer especificação final em ADR.

---
*Relatório concluído em estrita conformidade com a política CAT-A/B, sem violação de integridade ou suposições de compatibilidade de hardware.*
