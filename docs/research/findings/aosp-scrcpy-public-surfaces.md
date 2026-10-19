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
|   atividades independentes sem     |   Android se configurado com as  |   virtual display se o    |
|   afetar a tela local.             |   flags e permissões exigidas.   |   WindowManager suportar. |
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
- **Comportamento no AOSP:** Opera como um display secundário lógico. Se criado com as flags e privilégios pertinentes pelo servidor (`VIRTUAL_DISPLAY_FLAG_PUBLIC`, `VIRTUAL_DISPLAY_FLAG_TRUSTED`, `VIRTUAL_DISPLAY_FLAG_OWN_CONTENT_ONLY`), pode hospedar pilhas de atividades Android (`ActivityStack` / `Task`), permitindo que aplicativos Android rodem na resolução remota sem interferir no display físico primário.
- **Relação com IME e Decorações:** O AOSP permite configurar decorações de sistema e políticas de entrada (`DisplayWindowSettings#setShouldShowSystemDecorsLocked`, `DisplayWindowSettings#setShouldShowImeLocked` / `setDisplayImePolicy`), mas a presença efetiva dos componentes visuais varia por elemento e versão:
  - *Barra de Navegação:* Suportada em displays secundários configurados com decorações de sistema a partir do Android 10 (via `NavigationBarController` e affordances de *Back* e *Home*).
  - *Status Bar:* **Não suportada em displays secundários no Android 10** (AOSP documenta expressamente que a status bar, por conter Notification Shade, Quick Settings e Lock Screen, permanece restrita ao display primário).
  - *Launcher (Home):* Instanciado em displays secundários configurados com decorações caso exista atividade compatível com a categoria `android.intent.category.SECONDARY_HOME`; por segurança, launchers em displays virtuais só são renderizados em displays de posse do sistema.
  - *IME (Teclado Virtual):* Configurado independentemente das demais decorações de sistema.
  - *Habilitação de flag vs. Componentes Reais:* A ativação da flag de decorações (`VIRTUAL_DISPLAY_FLAG_SHOULD_SHOW_SYSTEM_DECORATIONS` ou configuração em `display_settings.xml`) sinaliza a intenção ao framework, mas não instancia automaticamente componentes inexistentes na ROM; componentes de System UI e launchers precisam existir e fornecer implementações aptas para o display secundário.

### 2.3 Desktop Windowing (Modo Desktop / Freeform)
- **Definição técnica:** Modo de gerenciamento de janelas (`WINDOWING_MODE_FREEFORM`) em que as atividades Android são dispostas em retângulos redimensionáveis com barra de título e controles de janela, em vez do modo tela cheia (`WINDOWING_MODE_FULLSCREEN`) ou tela dividida (`WINDOWING_MODE_MULTI_WINDOW`).
- **Comportamento no AOSP:** Suportado formalmente pelo `WindowManagerService`. No AOSP base, requer ativação de opções de desenvolvedor (`enable_freeform_support`) ou configurações de sistema. O fato de um display virtual existir **não ativa automaticamente** janelas livres se a ROM sobrepuser o gerenciador de janelas padrão com sua própria UI proprietária.

---

## 3. Mapeamento das APIs AOSP, Permissões e Restrições de Nível

A arquitetura de display e injeção do AOSP é estratificada em níveis rígidos de isolamento e segurança.

### 3.1 Fontes Primárias AOSP Consultadas
1. [AOSP Multi-Display Overview](https://source.android.com/docs/core/display/multi_display) (Atualizado para o ciclo trunk AOSP 2026).
2. [AOSP Activity Launch Policy](https://source.android.com/docs/core/display/multi_display/activity-launch) (Seções *Launch behavior*, *Security restrictions* e métodos `isCallerAllowedToLaunchOnDisplay` / `isActivityStartAllowedOnDisplay`).
3. [AOSP Recommended Practices](https://source.android.com/docs/core/display/multi_display/recommended-practices).
4. [AOSP Input Routing](https://source.android.com/docs/core/display/multi_display/input-routing).
5. [AOSP System Decorations](https://source.android.com/docs/core/display/multi_display/system-decorations) (Seções *System decorations*, *System UI decor windows*, *Launcher*, *Wallpapers*).

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
|                   |                                   |   AOSP base, mas passível de restrição      |
|                   |                                   |   em opções de desenvolvedor OEM.           |
+-------------------+-----------------------------------+---------------------------------------------+
| Sistema Privilegi.| UID 1000 (system) / system_server | - Posse de INTERNAL_SYSTEM_WINDOW e         |
|                   |                                   |   ACTIVITY_EMBEDDING irrestritos.           |
|                   |                                   | - Controle total de DisplayWindowSettings   |
|                   |                                   |   e roteamento de topologia de displays.    |
+-------------------+-----------------------------------+---------------------------------------------+
| Fabricante (OEM)  | Framework overlay / ROM vendor    | - Pode customizar ou desativar suporte a    |
|                   |                                   |   displays secundários, permissões do nó    |
|                   |                                   |   /dev/uhid e launcher de SystemUI.         |
+-------------------+-----------------------------------+---------------------------------------------+
```

### 3.3 Regras Críticas de Lançamento de Atividades (Activity Launch Policy)
Conforme a especificação oficial AOSP (*Activity launch policy*), o comportamento de lançamento em múltiplos displays é regido por modos de lançamento no `AndroidManifest.xml`, flags de `Intent` e opções de chamada via `ActivityOptions.setLaunchDisplayId(int displayId)`.

1. **Destino Padrão e Resolução de Contexto:**
   - Por padrão, uma atividade é iniciada no mesmo display do chamador (`caller context`).
   - Se disparada de contexto desvinculado a um display específico (como contexto de `Application` ou shell ADB), a atividade é posicionada no display de topo com o qual o usuário interagiu por último ou de onde a última atividade foi iniciada.
   - Quando um `Intent` resolve para uma instância de atividade pré-existente, ela ressurge no display onde esteve ativa por último, a menos que `ActivityOptions.setLaunchDisplayId(...)` direcione um novo display e as restrições de segurança permitam.

2. **Regime de Permissões e Restrições de Segurança:**
   Para evitar que aplicativos maliciosos capturem dados sensíveis do usuário lendo o framebuffer de um display virtual criado por eles, o AOSP impõe regras diferenciadas por classe de chamador, propriedade do display e atributos da atividade:
   - **Atividades Próprias em Display Próprio:** Um aplicativo padrão pode iniciar **suas próprias atividades** em um display virtual de sua própria criação (Android 10+).
   - **Atividades de Terceiros (`ACTIVITY_EMBEDDING`):** Um chamador só pode iniciar atividades de **outros aplicativos** em seu display se possuir a permissão `android.permission.ACTIVITY_EMBEDDING` **e** se a atividade de destino declarar explicitamente `android:allowEmbedded="true"` no seu manifesto (verificado em código via `ActivityInfo.FLAG_ALLOW_EMBEDDED`).
   - **Componentes de Sistema (`INTERNAL_SYSTEM_WINDOW`):** Componentes de sistema detentores da permissão privilegiada `android.permission.INTERNAL_SYSTEM_WINDOW` podem iniciar atividades em qualquer display sem as restrições acima.
   - **Displays Privados e Públicos:** Lançamentos em displays privados (`Display.FLAG_PRIVATE`) são estritamente restritos ao proprietário do display (`owner UID`) ou a atividades já presentes naquele display. Displays públicos aceitam atividades externas desde que atendidas as políticas de permissão do chamador e da atividade alvo.
   - **Papel da Flag `VIRTUAL_DISPLAY_FLAG_TRUSTED`:** A flag `VIRTUAL_DISPLAY_FLAG_TRUSTED` (introduzida no Android 10 e exposta no AOSP/API 33+) confere ao display a propriedade de ser confiável pelo sistema para hospedar certos componentes protegidos (como decorações de sistema e restrições de toque), **mas não constitui uma isenção universal ou automática** que dispense as regras de `ACTIVITY_EMBEDDING` e `FLAG_ALLOW_EMBEDDED` para aplicativos de terceiros.
   - **Diferenças Relevantes de Versão e Verificação Interna:**
     - No Android 9 e inferiores, tentativas não autorizadas de lançamento em displays restritos resultavam em lançamento de `SecurityException`.
     - No Android 10 (API 29+), o framework adicionou a checagem `ActivityManager.isActivityStartAllowedOnDisplay(Context context, int displayId, Intent intent)` para validação prévia de restrições de segurança antes da tentativa de lançamento.
     - As checagens estruturais de autorização são centralizadas internamente no AOSP em `ActivityStackSupervisor#isCallerAllowedToLaunchOnDisplay()` e `ActivityTaskSupervisor`.

### 3.4 Decorações e Entrada (System Decorations & Input Routing)
- **Decorações do Sistema:** Controladas por `DisplayWindowSettings` (persistido em `/data/system/display_settings.xml`). Define se o display secundário renderiza barra de navegação, papel de parede e IME (`shouldShowSystemDecors="true"`, `shouldShowIme="true"`).
- **Roteamento de Entrada (Android 10+):** A associação estática entre portas de dispositivos físicos e displays é realizada via `/vendor/etc/input-port-associations.xml`. Dispositivos dinâmicos virtuais necessitam de mapeamento no `InputManagerService`.

---

## 4. Mapeamento do scrcpy (Base Versionada: v5.0.1 / Commit a60891a)

Para evitar ambiguidades decorrentes de branches mutáveis (`master`), este relatório fixa a análise na release estável mais recente do scrcpy:

- **Versão:** `v5.0.1`
- **Tag:** `v5.0.1` (`refs/tags/v5.0.1` SHA: `a0c27ff382c411af1301dcc1cd8205c9aeb81996`)
- **Commit:** `a60891aea193d92e7e5c3942700eca63f9d19a5f`
- **Histórico Relevante:** O suporte formal a Virtual Displays foi introduzido no **scrcpy v3.0** (Issue #5370, PR #5506) e refinado nas versões 4.x e 5.0.

### 4.1 Mecanismo de Virtual Display no scrcpy (`--new-display`)
O scrcpy não requer privilégios de `root` para criar displays virtuais; ele executa seu servidor Java (`scrcpy-server.jar`) através do `app_process` sob as credenciais do shell ADB (`UID 2000`).

Conforme verificado em `server/src/main/java/com/genymobile/scrcpy/video/NewDisplayCapture.java` e `server/src/main/java/com/genymobile/scrcpy/wrappers/DisplayManager.java`:
1. **Invocação:** O servidor instancia a classe oculta `android.hardware.display.DisplayManagerGlobal` via reflexão e chama `createNewVirtualDisplay(...)`.
2. **Flags Utilizadas na Criação (`NewDisplayCapture.java`):**
   - Flags base:
     - `VIRTUAL_DISPLAY_FLAG_PUBLIC` (1 << 0)
     - `VIRTUAL_DISPLAY_FLAG_PRESENTATION` (1 << 1)
     - `VIRTUAL_DISPLAY_FLAG_OWN_CONTENT_ONLY` (1 << 3)
     - `VIRTUAL_DISPLAY_FLAG_SUPPORTS_TOUCH` (1 << 6)
     - `VIRTUAL_DISPLAY_FLAG_ROTATES_WITH_CONTENT` (1 << 7)
   - Flags condicionais de opções CLI:
     - Se destruição de conteúdo ativada (padrão): `VIRTUAL_DISPLAY_FLAG_DESTROY_CONTENT_ON_REMOVAL` (1 << 8). Pode ser desabilitada com `--no-vd-destroy-content`.
     - Se decorações de sistema ativadas (padrão): `VIRTUAL_DISPLAY_FLAG_SHOULD_SHOW_SYSTEM_DECORATIONS` (1 << 9). Pode ser desabilitada com `--no-vd-system-decorations`.
   - Flags por versão do Android verificadas no código do servidor (`Build.VERSION.SDK_INT`):
     - No Android 13+ (API 33+, `AndroidVersions.API_33_ANDROID_13`):
       - `VIRTUAL_DISPLAY_FLAG_TRUSTED` (1 << 10)
       - `VIRTUAL_DISPLAY_FLAG_OWN_DISPLAY_GROUP` (1 << 11)
       - `VIRTUAL_DISPLAY_FLAG_ALWAYS_UNLOCKED` (1 << 12)
       - `VIRTUAL_DISPLAY_FLAG_TOUCH_FEEDBACK_DISABLED` (1 << 13)
     - No Android 14+ (API 34+, `AndroidVersions.API_34_ANDROID_14`):
       - `VIRTUAL_DISPLAY_FLAG_OWN_FOCUS` (1 << 14)
       - `VIRTUAL_DISPLAY_FLAG_DEVICE_DISPLAY_GROUP` (1 << 15)
3. **Ciclo de Vida:** O display virtual criado pelo scrcpy é volátil e destruído na desconexão da sessão, exceto se configurado de forma diferente.

### 4.2 Lançamento de Aplicativos (`--start-app`)
- Conforme `doc/virtual-display.md`, se o display virtual não possuir um Launcher do sistema disponível (ou se `--no-vd-system-decorations` for passado), a tela permanece em branco (sem gerar frames de vídeo) até que uma aplicação seja iniciada.
- O scrcpy inicia o app alvo (`--start-app=<package>`) direcionando a janela especificamente para o `displayId` recém-criado, utilizando APIs internas de ActivityManager via ADB Shell.

### 4.3 Mecanismos de Entrada (Mouse e Teclado)
O scrcpy v5.0.1 suporta três modos de entrada, detalhados em `doc/control.md`, `doc/keyboard.md`, `doc/mouse.md` e `server/.../control/UhidManager.java`:

```
+--------------------+-----------------------+-------------------------+----------------------------------------------+
| Modo de Entrada    | Flag scrcpy           | Requisitos de Sistema   | Características e Restrições                 |
+--------------------+-----------------------+-------------------------+----------------------------------------------+
| SDK Input          | --keyboard=sdk        | USB Debugging ativo.    | Injeta MotionEvent/KeyEvent via IPC do       |
| (Padrão)           | --mouse=sdk           | UID 2000 (Shell).       | InputManagerService.                         |
|                    |                       |                         | * Advertência scrcpy: em aparelhos Xiaomi,  |
|                    |                       |                         |   pode falhar se "USB debugging (Security    |
|                    |                       |                         |   settings)" estiver desativado no menu dev. |
+--------------------+-----------------------+-------------------------+----------------------------------------------+
| UHID               | --keyboard=uhid (-K)  | Acesso de escrita ao    | Cria dispositivo virtual no subsistema HID   |
| (Kernel HID)       | --mouse=uhid (-M)     | nó /dev/uhid pelo shell | do kernel Linux (/dev/uhid).                 |
|                    |                       | (UID 2000).             | * Modo público alternativo; sujeito às       |
|                    |                       |                         |   permissões e políticas de segurança da ROM.|
|                    |                       |                         | * No scrcpy (UhidManager.java), faz          |
|                    |                       |                         |   associação por porta em Android 15+.       |
+--------------------+-----------------------+-------------------------+----------------------------------------------+
| AOA (Android Open  | --keyboard=aoa        | Comunicação USB direta  | Emula acessório físico via protocolo USB     |
| Accessory 2.0)     | --mouse=aoa           | (OTG / hardware USB).   | AOAv2 sem passar pelo daemon adbd.           |
|                    |                       | Sem dependência do ADB. | * Requer conexão física por cabo USB.        |
|                    |                       |                         | * Incompatível com modo rede/TCP.            |
+--------------------+-----------------------+-------------------------+----------------------------------------------+
```

---

## 5. Tabela de Capacidades: Fatos Documentais vs Hipóteses Técnicas

A matriz a seguir mapeia cada mecanismo documentado no AOSP e no scrcpy v5.0.1, distinguindo rigorosamente a verdade documental das hipóteses para o Limivra.

> **Regra de Aceite Rigorosa:** Nenhuma funcionalidade é declarada como suportada pelo dispositivo alvo (Redmi Note 13 4G / HyperOS) sem teste físico observável. Todas as linhas referentes a comportamento específico em ROM de fabricante são classificadas como `UNKNOWN / UNTESTED`.

```
+-------------------+----------------------------+-----------------------+---------------------------+---------------------------------------+------------------------------------------+-----------------------+
| Mecanismo         | Fonte Primária e Ref       | Requisito / Versão    | Permissões Requeridas     | Hipótese Técnica Limivra              | Riscos e Restrições de ROM               | Prova Ausente         |
+-------------------+----------------------------+-----------------------+---------------------------+---------------------------------------+------------------------------------------+-----------------------+
| Display Mirroring | AOSP DisplayManager /      | API 21+ (AOSP 5.0+) / | Nenhuma além do shell ADB | Viável para fallback básico de        | Baixa flexibilidade; bloqueio de tela    | Teste observável de   |
| (Default Display) | scrcpy v5.0.1 base         | scrcpy v1.0+          | (UID 2000)                | espelhamento da tela principal.       | primária pode suspender renderização.    | FPS e latência local. |
+-------------------+----------------------------+-----------------------+---------------------------+---------------------------------------+------------------------------------------+-----------------------+
| Virtual Display   | AOSP DisplayManagerGlobal/ | Recurso scrcpy v3.0+; | Shell ADB (UID 2000) com  | Pode hospedar apps Android dedicados  | Risco de a ROM customizar ou restringir  | Teste físico sob      |
| Secundário        | scrcpy NewDisplayCapture   | flags TRUSTED exigem  | acesso a APIs reflexivas  | no display virtual, transmitindo      | o DisplayManagerService; risco de tela   | SPIKE-001 (Issue #9)  |
| (--new-display)   | commit a60891a             | API 33+ no servidor   | ocultas de sistema.       | vídeo para exibição no host Linux.    | preta se launcher decor estiver ausente. | com dumpsys display.  |
+-------------------+----------------------------+-----------------------+---------------------------+---------------------------------------+------------------------------------------+-----------------------+
| Decorações de     | AOSP DisplayWindowSettings | AOSP 10+ (API 29+) /  | Modificação do sistema ou | Exibir barra de tarefas ou launcher   | ROMs OEM podem não instanciar interface  | UNKNOWN / UNTESTED    |
| Sistema Secundárias| scrcpy cli.c / doc:       | scrcpy v3.0+          | flags de criação do VD    | de sistema no display virtual se      | gráfica em displays virtuais criados via | na ROM do aparelho.   |
| (decor / no-decor)| --no-vd-system-decorations |                       | com decors habilitadas.   | decor estiver ativo.                  | shell sem monitor físico HDMI acoplado.  |                       |
+-------------------+----------------------------+-----------------------+---------------------------+---------------------------------------+------------------------------------------+-----------------------+
| Injeção SDK       | AOSP InputManager /        | API 21+ (AOSP)        | INJECT_EVENTS             | Método padrão de interação com touch  | Advertência documentada pelo scrcpy de   | Verificação visual da |
| de Eventos        | scrcpy --mouse=sdk         |                       | (Disponível ao UID 2000   | e eventos básicos de entrada a partir | restrição em aparelhos Xiaomi (exigência | presença da opção no  |
|                   |                            |                       | no AOSP base).            | de eventos capturados no host Linux.  | de autorização de segurança).            | menu de desenvolvedor.|
+-------------------+----------------------------+-----------------------+---------------------------+---------------------------------------+------------------------------------------+-----------------------+
| Injeção UHID      | Linux Kernel UHID /        | scrcpy v2.4+;         | Acesso de escrita ao nó   | Modo público alternativo para teclado | Permissões de SELinux ou nó /dev/uhid    | UNKNOWN / UNTESTED:   |
| (Kernel Device)   | scrcpy --keyboard=uhid     | associação por porta  | /dev/uhid pelo shell      | físico e mouse capturado, sujeito às  | podem ser restritas para o shell UID     | verificar permissões  |
|                   | UhidManager.java (v5.0.1)  | exige API 35+ no srv  | (UID 2000).               | políticas de segurança do sistema.    | 2000 em determinadas variantes de ROM.   | de /dev/uhid via adb. |
+-------------------+----------------------------+-----------------------+---------------------------+---------------------------------------+------------------------------------------+-----------------------+
| Janelas Livres    | AOSP WindowManager         | API 24+ (AOSP 7.0+)   | Flag global do sistema ou | Permitir que aplicações Android rodem | Suporte a freeform AOSP pode estar       | UNKNOWN / UNTESTED:   |
| (Freeform /       | enable_freeform_support    |                       | configuração de janela    | em janelas redimensionáveis sobre o   | inativo ou substituído por janelas       | verificar se a ROM    |
| Floating Windows) |                            |                       | livre da ROM.             | display virtual.                      | flutuantes proprietárias da fabricante.  | expõe a feature.      |
+-------------------+----------------------------+-----------------------+---------------------------+---------------------------------------+------------------------------------------+-----------------------+
```

---

## 6. Diferenciação Factual: Documentação vs Hipótese vs Realidade

A governança do projeto Limivra exige a separação inequívoca das três esferas de conhecimento:

1. **Documentação Oficial (O que o AOSP e o scrcpy descrevem):**
   - O AOSP possui arquitetura documentada para displays múltiplos (`DEFAULT_DISPLAY` vs `secondary`), controle de lançamento de atividades (`ActivityOptions#setLaunchDisplayId`), roteamento de portas físicas (`/vendor/etc/input-port-associations.xml`) e decorações de sistema (`DisplayWindowSettings`).
   - O scrcpy v5.0.1 possui suporte documentado para criar displays virtuais via `DisplayManagerGlobal` (`--new-display`), desativar decorações (`--no-vd-system-decorations`), redimensionamento flexível (`--flex-display`) e inicialização de aplicativos Android direcionados (`--start-app`).
   - O scrcpy documenta formalmente em seu README que dispositivos de determinados fabricantes (como Xiaomi) podem exigir autorização explícita adicional no menu de desenvolvedor para injeção via SDK ("USB debugging (Security Settings)").
   - O scrcpy implementa injeção via kernel em `UhidManager.java`, abrindo `/dev/uhid` como processo do shell, associando a porta ao display via `InputManager` em Android 15+.

2. **Hipótese Técnica do Limivra (Conforme a Constituição do Sistema: Android executa apps → Linux exibe e controla):**
   - É hipotetizado que tarefas e aplicativos **Android** possam rodar em um display virtual secundário sem interferir no display primário do dispositivo, sendo o vídeo transmitido e renderizado no host Linux.
   - É hipotetizado que o host Linux capture eventos de mouse e teclado locais e os transmita para o servidor no Android, que os injetará via SDK ou UHID conforme o suporte e as permissões concedidas pela ROM.
   - O UHID é avaliado estritamente como um **modo público alternativo documentado**, condicionado ao consentimento do usuário e às permissões de kernel da ROM, sem qualquer premissa de contornar ou evadir controles de segurança.

3. **Observação Real em Hardware (O que foi efetivamente provado):**
   - **NADA.** Nenhum comando foi executado no dispositivo físico de referência sob esta issue.
   - Não há evidência observável de que a ROM HyperOS do aparelho aceite as flags `VIRTUAL_DISPLAY_FLAG_TRUSTED` sem redefinir o display para espelhamento forçado.
   - Não há evidência observável de que o nó `/dev/uhid` seja gravável pelo shell UID 2000 ou de que o modo freeform esteja acessível no aparelho sem teste físico.

---

## 7. Proposta de Experimentos Futuros (Subordinados à Issue #9 / SPIKE-001)

Qualquer teste prático decorrente desta pesquisa é uma **proposta futura potencialmente modificadora do estado da sessão** e só poderá ser executado sob autorização humana explícita, consentimento de depuração USB e escopo estrito da [Issue #9](https://github.com/pedro-labsabs/limivra/issues/9) e [docs/research/SPIKE-001.md](../../research/SPIKE-001.md).

### 7.1 Protocolo Experimental Sugerido (Apenas Proposta — Não Executado)
1. **Teste 1 — Dumpsys Inicial de Display:**
   - Comando planejado: `adb shell dumpsys display`
   - Critério de observação: Verificar quais flags, resoluções e displays secundários são reportados nativamente pela ROM em estado padrão.
2. **Teste 2 — Verificação de Permissões do Nó UHID:**
   - Comando planejado: `adb shell ls -l /dev/uhid`
   - Critério de observação: Inspecionar atributos de arquivo (leitura/escrita para UID 2000) sem tentar gravar no dispositivo.
3. **Teste 3 — Verificação Controlada de Display Virtual via scrcpy v5.0.1:**
   - Comando planejado: `scrcpy --new-display=1280x720 --no-audio` (execução interativa pontual com monitoramento de processo).
   - *Critérios de interrupção (STOP) e teardown imediato:*
     - Interromper imediatamente (`Ctrl+C` / SIGINT) se o dispositivo não responder, apresentar travamento de interface ou rejeição de permissão.
     - Garantir o encerramento do processo local do scrcpy e verificar no dispositivo o teardown do display virtual (`adb shell dumpsys display | grep -i scrcpy`).
     - Não solicitar nem armazenar dumps de `logcat` bruto; se logs forem necessários, limitar-se a mensagens de erro sanitizadas da tag `scrcpy`.
4. **Teste 4 — Consulta de Feature de Janelas Livres (Freeform):**
   - Comando planejado: Inspecionar via `adb shell pm list features | grep freeform`.
   - Critério de observação: Verificar se a feature `android.software.freeform_window_management` está presente no sistema.

---

## 8. Conclusões, Riscos, Lacunas e Próximos Gates

### 8.1 Conclusões Documentais
- A documentação pública do AOSP e o código-fonte estável do scrcpy v5.0.1 comprovam a viabilidade conceitual de criação de displays secundários virtuais a partir do shell ADB em sistemas Android com suporte de framework.
- No entanto, a viabilidade real em dispositivos de mercado depende de como a ROM do fabricante implementa ou restringe as classes de `system_server`, permissões de nós de entrada e gerenciamento de janelas.

### 8.2 Principais Riscos Identificados
- **Possibilidade de Restrição de Entrada pelo Sistema:** Risco de o modo SDK ser bloqueado se a ROM exigir permissões adicionais concedidas pelo usuário nas opções de desenvolvedor.
- **Comportamento de Display sem Decorações:** Se a ROM não instanciar uma interface gráfica para telas secundárias, o display virtual pode permanecer em branco até que um aplicativo Android seja explicitamente direcionado a ele.
- **Substituição de Janelas Livres por Soluções Proprietárias:** A ROM pode desativar o gerenciador de janelas livres do AOSP em favor de sistemas próprios de multijanela.

### 8.3 Próximos Gates Formais
1. **Gate SPIKE-001 (Issue #9):** Prova física de viabilidade em hardware real, condicionada à autorização humana e conexão física do aparelho de teste.
2. **Gate de Análise de Lacunas e REA (Issue #14):** Investigação aprofundada de eventuais restrições não resolvidas por documentação pública, formalmente dependente dos achados das Issues #9 e #13.
3. **Gate de Decisão de Arquitetura e Design (Issue #8):** Autoridade soberana para decisões humanas de arquitetura e ratificação final de ADRs do sistema Limivra.

---
*Relatório reparado em estrita conformidade com a política CAT-A/B, sem violação de integridade ou suposições de compatibilidade de hardware.*
