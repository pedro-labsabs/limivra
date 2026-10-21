# Samsung DeX Transport & Interoperability Analysis

**Origem do Alvo:** `research/artifacts/dex/dexonpc.apk` (`com.sec.android.app.dexonpc_2.6.01.8-260101008`)  
**Biblioteca Inspecionada:** `lib/arm64-v8a/libSideSyncNative.so` (SHA-256: `b3f9e5701792dbaa63cfd3c0740ac36b057bee7f391ec95d50f2fc02cf61efc7`)  
**Metodologia:** Engenharia reversa via MCP REA (Ghidra 12.1 decompilação e análise estática de símbolos) com restrições *clean-room* documentadas em [AGENTS.md](../../../AGENTS.md) e [rea-plan.md](../rea-plan.md).

---

## 1. Visão Geral da Arquitetura de Transporte (DeX on PC / SideSync)

O componente DeX on PC (`dexonpc.apk`) baseia-se na evolução da pilha Samsung SideSync/DOP (*DeX on PC*). A comunicação física entre o computador (Sink) e o smartphone (Source) opera sobre conexões de rede padrão (TCP/IP), tanto via **USB** (através de `rndis0`, tethering ou `adb forward` para `127.0.0.1`) quanto via **Wi-Fi** (P2P/WLAN).

### 1.1 Portas de Escuta e Canais de Comunicação

A classe nativa `SocketManager` inicializa dois canais de comunicação TCP principais:

| Canal | Porta Padrão | Controlador Nativo | Propósito |
|---|---|---|---|
| **Control / Event Channel** | `30830` (`0x786e`) | `EventSocketController` | Handshake, negociação de protocolo, eventos de mouse/teclado, resolução de tela, clipboard e batimentos cardíacos (heartbeat). |
| **Data / File Channel** | `30831` (`0x786f`) | `FileSocketController` | Transferência de arquivos e buffers de dados pesados (bidirecional). |

Adicionalmente, na camada Android Java (`DOPService` / `ConnDaemonHandler`):
- O serviço interage com o daemon local do sistema Samsung `ss_conn_daemon2` (via socket local UNIX de domínio abstrato `\0ss_conn_daemon2`) para detecção e inicialização da ponte USB.

---

## 2. Detecção e Seleção de Transporte (`SocketManager::determineConntectionType`)

A função nativa `SocketManager::determineConntectionType()` inspeciona o socket conectado (`getsockname` / `inet_ntoa` / `getInterfaceFromIP`):

1. **Loopback / USB Forward (`127.0.0.1`):**
   - Se o IP conectado for `127.0.0.1`, o tipo de conexão é configurado como `0` (USB / Loopback via ADB forward ou RNDIS local).
   - O mecanismo de heartbeat de timeout é desabilitado ou ajustado para transporte cabeado estável.
2. **Interface de Rede USB RNDIS (`rndis`):**
   - Se o nome da interface de rede associada ao IP contiver o prefixo `rndis`, a conexão é também identificada e tratada como USB (`0`).
3. **Interface Wi-Fi / P2P (`wlan0`, `p2p-wlan0-0`, `swlan0`):**
   - Caso não seja loopback nem RNDIS, a conexão recebe o identificador `1` (Conexão via Wi-Fi/Rede sem fio).
   - O flag de segurança (`needSecurity`) e criptografia obrigatória é acionado para proteger a transmissão.

---

## 3. Protocolo de Enquadramento de Pacotes (`PacketParser`)

Todos os pacotes no canal de eventos/controle seguem um cabeçalho fixo binário em formato *Big-Endian*:

### 3.1 Estrutura do Cabeçalho de Pacote (4 bytes fixos)

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|    Version    |  Packet Type  |      Packet Length (Total)    |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                        Data Buffer...                         |
```

- **Byte 0 — Protocol Version:** `0x03` (ou conforme negociado na inicialização).
- **Byte 1 — Packet Type (MessageType):** Identificador do comando/evento (1 byte).
- **Bytes 2–3 — Total Packet Length (uint16 big-endian):** Comprimento total do pacote (cabeçalho de 4 bytes + payload).
- **Bytes 4+ — Payload:** Dados específicos do pacote.

### 3.2 Tabela de Tipos de Pacotes Observados

| Código (`Type`) | Nome Simbólico | Descrição do Payload |
|---|---|---|
| `0x01` (1) | `HEARTBEAT` | Ping de verificação de conectividade; o dispositivo responde com `sendHeartBeatResponse()`. |
| `0x03` (3) | `CHANGE_WORKING_DEVICE` | Notificação de alternância de dispositivo de controle ativo (1 byte de estado). |
| `0x04` (4) | `CONNECT_CONFIRM` | Confirmação de conexão e alinhamento de versão de protocolo (`getVersion`). |
| `0x14` (20) | `MOUSE_EVENT` | Evento de ponteiro/mouse (14 bytes total). |
| `0x15` (21) | `KEYBOARD_EVENT` | Evento de teclado (11 bytes total). |
| `0x1E` (30) | `DEVICE_NAME_REQ` | Requisição/envio do nome do dispositivo. |
| `0x1F` (31) | `DEVICE_NAME_RESP` | Resposta contendo o nome descritivo do host/dispositivo. |
| `0x20`–`0x22` (32–34) | `CLIPBOARD_DATA` | Início, fragmento e término de sincronização de área de transferência. |
| `0x2C`–`0x2E` (44–46) | `FILE_CONTROL` | Cancelamento (`0x2C`/`0x2D`) e prontidão para transmissão de arquivo (`0x2E`). |
| `0x33`–`0x36` (51–54) | `RESOLUTION_CONFIG`| Negociação e requisição de resolução de tela / densidade DPI. |
| `0x5A` (90) | `SECURITY_HANDSHAKE`| Troca de chaves criptográficas (`sendSecurityHello`, `sendSecurityAck`, SAKEP). |
| `0x5B` (91) | `ENCRYPTED_PACKET`  | Envelope de dados criptografados (descriptografado por `SecurityHandler::decryptData`). |
| `0x64`–`0x69` (100–105)| `COMMON_JSON_MSG`  | Mensagens estruturadas JSON (payload com comprimento e string JSON serializada). |

---

## 4. Handshake de Conexão e Segurança

### 4.1 Sequência de Conexão em Modo Seguro (Wi-Fi)
Quando conectado via Wi-Fi (`connectionType == 1`):
1. **Security Hello (`0x5A` / SAKEP / Cryptopp):**
   - O cliente envia `ClientHello` contendo parâmetros criptográficos SAKEP (*Samsung Authentication & Key Exchange Protocol*) e parâmetros Diffie-Hellman / ECDH baseados em `CryptoPP`.
2. **Server Hello & Key Exchange:**
   - O servidor responde com `GeneratorServerHello` e derivação de `SharedSecret` e `SKPrime`.
3. **Security ACK (`0x5A` subtype ACK):**
   - Validação recíproca dos MACs de integridade. Conclusão da troca de chaves (`isKeyExchanged == true`).
4. **Estabelecimento de Sessão:**
   - O flag de conexão é confirmado (`SocketManager::setConnected(true)`).

### 4.2 Sequência de Conexão em Modo Direto (USB / Loopback)
Quando conectado via USB/RNDIS (`connectionType == 0`):
1. A criptografia pesada no canal pode ser ignorada ou operada em bypass (`needSecurity == false`).
2. O handshake transita diretamente via `CONNECT_CONFIRM` (`0x04`) para confirmação mútua da versão de protocolo suportada.

---

## 5. Mapeamento de Eventos de Entrada (Mouse e Teclado)

### 5.1 Pacote de Mouse (`Type: 0x14`)
Construído por `SocketPacketFactory::mouseEvent(int action, int x, int y, int* length)`:
- **Tamanho total:** 14 bytes (`0x0E`).
- **Cabeçalho:** `[0x03, 0x14, 0x00, 0x0E]`
- **Layout do Payload:**
  - `Byte 4`: Reservado / flags (`0x00`).
  - `Byte 5`: Ação do mouse (`action`: botão pressionado, botão liberado, movimento, clique).
  - `Bytes 6–9`: Coordenada X (inteiro de 32 bits, big-endian).
  - `Bytes 10–13`: Coordenada Y (inteiro de 32 bits, big-endian).
- **Processamento:** Desempacotado em `EventSocketController::receiveMouseEvent` e despachado para a camada de injeção através de `NotifyFromNative::Notify(0xc9, action, -1, [x, y], 2, 2)`.

### 5.2 Pacote de Teclado (`Type: 0x15`)
Construído por `SocketPacketFactory::keyboardEvent(int action, int keyCode, char* charData, int* length)`:
- **Tamanho total:** 11 bytes (`0x0B`).
- **Cabeçalho:** `[0x03, 0x15, 0x00, 0x0B]`
- **Layout do Payload:**
  - `Byte 4`: Reservado (`0x00`).
  - `Byte 5`: Ação da tecla (`action`: key down, key up).
  - `Byte 6`: Código de modificador ou meta-state (`keyCode`).
  - `Bytes 7–10`: Valor do caractere / keycode Android em 32 bits.
- **Processamento:** Desempacotado em `EventSocketController::receiveKeyboardEvent` e notificado à camada do framework via `NotifyFromNative::Notify(200, action, keyCode, [charData], 4, 1)`. Na camada Java, `handleKeyEvent` encaminha os eventos para injeção física/lógica através de `InputManager.semInjectInputEvent()`.

---

## 6. Projeção de Display e Vídeo

A análise do `dexonpc.apk` e suas dependências revela que:
1. **O streaming de vídeo do display NÃO transita na `libSideSyncNative.so`:**
   - A biblioteca `libSideSyncNative.so` limita-se estritamente ao canal de controle, eventos de input, clipboard e sincronização de arquivos.
2. **Camada de Display — WFD / Miracast / KMS:**
   - O streaming de tela do DeX on PC é gerenciado no lado Android por serviços dedicados do framework Samsung (`SemWifiDisplayStatus`, `DisplayManager.semConnectWifiDisplay`, `KMSService` e `WIMPServer`).
   - A sessão de display é vinculada ao subsistema de desktop UI (`uiservice.apk` e `desktoplauncher.apk`), usando um `VirtualDisplay` criado pelo `SemDesktopModeManager` / `IDesktopModeConnectivityService`.

---

## 7. Relevância para a Implementação do Limivra (Clean-Room Client)

Para interoperar como um cliente Linux nativo de DeX on PC sem dependência de drivers proprietários do Windows:
1. **Canal de Controle/Input:**
   - É viável implementar um servidor/cliente TCP padrão escutando na porta `30830` conectando via ADB port forward (`adb forward tcp:30830 tcp:30830`).
   - O envio de eventos de mouse e teclado em conformidade com o enquadramento de 4 bytes e os opcodes `0x14` e `0x15` dispensa a emulação do handshake SAKEP quando executado sobre `127.0.0.1`.
2. **Recepção de Vídeo:**
   - Para o transporte de vídeo, a integração pode se conectar ao receptor Wi-Fi Display / KMS do dispositivo ou alavancar o pipeline Scrcpy / virtual display padrão do Linux, contornando a necessidade de reescrever a pilha WIMP/KMS proprietária no Linux.
