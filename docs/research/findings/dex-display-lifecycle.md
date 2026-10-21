# Samsung DeX Display Lifecycle, Freeform Mode & App Launch Mechanics

**Origem dos Alvos:**  
- `research/artifacts/dex/uiservice.apk` (`com.sec.android.desktopmode.uiservice_4.7.16-471600000`)  
- `research/artifacts/dex/desktoplauncher.apk` (`com.sec.android.app.desktoplauncher_3.0.16.34-301601034`)  
**Metodologia:** Engenharia reversa em ambiente *clean-room* (inspeção de bytecode DEX e tabelas de símbolos/métodos com base em [AGENTS.md](../../../AGENTS.md) e [rea-plan.md](../rea-plan.md)).

---

## 1. Ciclo de Vida do Display Secundário (VirtualDisplay / Display de Desktop)

### 1.1 Identificação e Categoria do Display
Diferente de displays de espelhamento padrão (*mirroring*), o Samsung DeX registra displays de desktop associados a uma categoria de hardware proprietária:

- **Categoria de Display Samsung:** `com.samsung.android.hardware.display.category.DESKTOP`
- **Consulta de Displays Ativos:**
  Tanto o `uiservice` (`Ll4/i;->d`) quanto o `desktoplauncher` (`Lc3/h;->k`) obtêm o display alvo através da API oculta do `DisplayManager`:
  ```java
  DisplayManager dm = (DisplayManager) context.getSystemService(Context.DISPLAY_SERVICE);
  Display[] desktopDisplays = dm.getDisplays("com.samsung.android.hardware.display.category.DESKTOP");
  ```
  Se displays com essa categoria existirem, o `DisplayId` correspondente é selecionado para servir como root display do launcher de desktop. Caso contrário, ocorre fallback para a lista geral de displays secundários (`dm.getDisplays()`).

### 1.2 Transição de Espelhamento para Desktop (Runtime Mirroring Swap)
O framework DeX suporta transição a quente entre modo clone e desktop estendido:
- **Categoria de Troca em Runtime:** `com.samsung.android.hardware.display.category.RUNTIME_MIRRORING_SWAP`
- **Controle de Wi-Fi Display:**
  O `uiservice` invoca `DisplayManager.semConnectWifiDisplay(SemWifiDisplayConfig)` passando parâmetros dedicados (`wfd_sec_dex_support`, `wfd_sec_dex_mouse_support`).

---

## 2. Ativação do Modo Desktop / Freeform

### 2.1 Configurações de Sistema (Settings.System / Global / Secure)

A ativação, persistência e estado do DeX dependem dos seguintes registros:

| Chave de Configuração | Escopo | Tipo | Descrição e Comportamento |
|---|---|---|---|
| `dex_mode` | `Settings.System` | `int` (`0` ou `1`) | Flag principal do Samsung DeX. `1` indica que o subsistema DeX está ativo. |
| `new_dex` | `Settings.System` | `int` (`0` ou `1`) | Flag do "Novo DeX" (modo desktop moderno no próprio tablet ou em tela externa). Monitorado via `SettingsObserver` em `uiservice` e `desktoplauncher`. |
| `dexonpc_connection_state` | `Settings.System` | `int` | Estado da conexão com o cliente PC (`0` desconectado, `1` em negociação, `2` conectado). |
| `wifi_display_on` | `Settings.Global` | `int` (`0` ou `1`) | Habilitação do serviço de Wi-Fi Display para projeção sem fio. |
| `dual_mode_screen_zoom` | `Settings.System` | `int` | Fator de zoom/escala de DPI aplicado exclusivamente ao display DeX. |
| `dex_screen_timeout` | `Settings.System` | `int` | Timeout independente de suspensão da tela para a sessão DeX. |
| `audio_output_to_display` | `Settings.System` / `Secure` | `int` (`0` ou `1`) | Roteamento do áudio multimídia para a tela externa / PC. |
| `dex_lockscreen_wallpaper_transparency` | `Settings.System` | `int` | Transparência do papel de parede da tela de bloqueio no desktop. |
| `dex_system_wallpaper_transparency` | `Settings.System` | `int` | Transparência do papel de parede do desktop. |

---

## 3. Intents de Inicialização e Broadcasts

### 3.1 Broadcasts de Mudança de Estado do DeX
O framework emite e escuta as seguintes ações de Intent para orquestrar transições:

- **Atualização de Modo Desktop:**
  - `com.samsung.android.desktopmode.action.DESKTOP_MODE_UPDATE_REQUEST`
  - Extras associados:
    - `com.samsung.android.desktopmode.extra.DESKTOP_MODE_STATE`:
      - `0`: Desativado (`DISABLED`)
      - `1`: Ativando/Desativando (`UPDATE`)
      - `2`: Ativado (`ENABLED`)
    - `com.samsung.android.desktopmode.extra.DESKTOP_MODE_SOURCE`: Identifica o gatilho da ativação:
      - `EXTRA_DESKTOP_MODE_SOURCE_HDMI_MODE` (Cabo USB-C / HDMI conectado)
      - `EXTRA_DESKTOP_MODE_SOURCE_CONNECTIVITY_ACTIVITY` (Conexão sem fio iniciada pelo usuário)
      - `EXTRA_DESKTOP_MODE_SOURCE_DEX_PANEL` (Painel rápido / Quick Settings)
      - `EXTRA_DESKTOP_MODE_SOURCE_TILE` (Toggle do Tile no SystemUI)
      - `EXTRA_DESKTOP_MODE_SOURCE_KEYBOARD_SHORTCUT` (Atalho de teclado físico Meta+W / Meta+D)
    - `com.samsung.android.desktopmode.extra.NEW_DEX_MODE_STATE`: Estado específico do layout de janelas livres.

### 3.2 Notificações de Interação e Touchpad
- `ACTION_DEX_RUNNING_NOTIFICATION_PRESSED`
- `ACTION_TOUCHPAD_AVAILABLE_NOTIFICATION_PRESSED`
- `ACTION_TOUCHPAD_NOTIFICATION_PRESSED`
- `ACTION_SPEN_NOTIFICATION_PRESSED`

### 3.3 Categorias de Intent do Launcher de Desktop
Para identificar quais aplicações suportam execução ou devem aparecer na gaveta de aplicativos do DeX:
- `android.intent.category.DEXLAUNCHER`
- O `desktoplauncher` consulta os aplicativos compatíveis via:
  ```java
  Intent intent = new Intent(Intent.ACTION_MAIN);
  intent.addCategory("android.intent.category.DEXLAUNCHER");
  pm.semQueryIntentActivitiesAsUser(intent, ...);
  ```

---

## 4. Chamadas AIDL / IPC (`IDesktopModeUiService`)

O serviço `uiservice` atua como bridge de interface entre o servidor central do sistema Android (`DesktopModeService`) e a interface gráfica do usuário.

### 4.1 Interface `IDesktopModeUiService`
Definida no AIDL `com.samsung.android.desktopmode.IDesktopModeUiService`:
- O stub é implementado pela classe interna `Le3/b;` no `uiservice`.
- Métodos do contrato AIDL expostos pelo serviço:
  1. `showOverlay(int type, int where)`: Exibe telas de transição (ex: `OVERLAY_TYPE_LOADING_SCREEN_ENTER`, `OVERLAY_TYPE_LOADING_SCREEN_EXIT`, `OVERLAY_TYPE_BLACK_SCREEN`).
  2. `dismissOverlay(int type, int where)`: Remove overlays de transição.
  3. `showDialog(int type, int displayId, IDesktopModeUiServiceCallback callback)`: Exibe diálogos de aviso (incompatibilidade de carregador, app não suportado em freeform, etc.).
  4. `dismissDialog(int type)`: Fecha diálogos abertos.
  5. `showNotification(int type)`: Dispara notificações de modo touchpad, S-Pen ou DeX ativo.
  6. `dismissNotification(int type)`: Remove as notificações de sistema.

### 4.2 Callback `IDesktopModeUiServiceCallback`
- Interface: `com.samsung.android.desktopmode.IDesktopModeUiServiceCallback` (proxy implementado em `Le3/c;`).
- Códigos de transação observados:
  - `transact(1)`: Confirmação positiva / clique em botão de prosseguir (`onClick` / `onButtonClick`).
  - `transact(2)`: Cancelamento ou recusa do diálogo de modo desktop.

---

## 5. Lançamento de Apps na Tela Secundária e Janelas Freeform

### 5.1 Mecanismo de Alvo de Display (`setLaunchDisplayId`)
Tanto o launcher (`desktoplauncher`) quanto as janelas de serviço (`uiservice`) utilizam o `ActivityOptions` do Android para direcionar o destino da Activity:

```java
ActivityOptions options = ActivityOptions.makeBasic();
int desktopDisplayId = getDesktopDisplayId(); // Obtido via categoria DESKTOP
options.setLaunchDisplayId(desktopDisplayId);
context.startActivity(intent, options.toBundle());
```

### 5.2 Janelas Livres (Freeform Windowing Mode)
Na abertura de tarefas a partir do launcher ou da lista de recentes (`Lx3/c;->z0`):

1. **ActivityOptionsCompat.makeFreeformOptions:**
   - Invocado internamente pelo Launcher (`Lcom/samsung/android/desktopsystemui/sharedlib/system/ActivityOptionsCompat;->makeFreeformOptions()`).
   - Define o bounds inicial da janela (`setLaunchBounds(Rect)`).
   - Configura o display alvo (`setLaunchDisplayId(desktopDisplayId)`).
2. **Definição de Modo de Janela no Task:**
   - O launcher atualiza o modo da janela via `TaskKey.setWindowingMode(WINDOWING_MODE_FREEFORM)` (constante Android AOSP `5`).
   - Dispara a execução via `ActivityManagerWrapper.startActivityFromRecentsAsync()`.

---

## 6. Implicações para a Arquitetura do Limivra

Para que o Limivra implemente uma experiência DeX-like no Linux sem componentes proprietários da Samsung:

1. **Uso de Displays Secundários em AOSP:**
   - O Limivra pode emular a mesma semântica de lançamento configurando um display virtual no Android (via `adb shell appops` / `adb shell am display` ou criação de `VirtualDisplay` em daemon próprio) e despachando apps com `am start -n <component> --display <id>`.
2. **Ativação de Freeform:**
   - Em ROMs AOSP / OneUI, a ativação do modo freeform para telas secundárias pode ser simulada via:
     ```bash
     settings put global enable_freeform_support 1
     settings put global force_resizable_activities 1
     ```
   - Em ROMs Samsung, o chaveamento de `Settings.System.putInt("dex_mode", 1)` e `Settings.System.putInt("new_dex", 1)` habilita respostas de compatibilidade nos launchers.
3. **Controle de Resolução e Escala:**
   - Em vez de depender do protocolo proprietário de display WFD da Samsung, o Limivra pode ajustar dinamicamente a densidade e resolução do display virtual secundário usando comandos do display manager do AOSP (`wm size <W>x<H> -d <id>`, `wm density <DPI> -d <id>`).
