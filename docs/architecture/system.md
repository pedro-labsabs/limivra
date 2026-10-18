# Arquitetura Limivra (proposta)

**Estado:** PROPOSED, não implementada. Requer revisão do documento em `docs/superpowers/specs/2026-10-18-limivra-design.md`.

## Fluxo e limites
```text
Linux: Qt 6/QML UI (planejado)
       ↓
Rust SessionCore (planejado)
       ├ DeviceProvider: ADB device discovery e auth
       ├ CapabilityProbe: evidência e estados
       ├ DisplayProvider: Mirror / Virtual / Desktop(condicional)
       ├ ScrcpyAdapter: subprocesso + ciclo de vida
       └ Diagnostics: logs sanitizados/erros
       ↓ ADB/scrcpy autorizados, USB/Wi-Fi
Android: display físico + display virtual (se permitido) + apps executados no telefone
```
**Host não executa apps Android.** REA é pesquisa separada, não dependência de runtime. Companion APK é futuro condicional.

## Componentes e contratos conceituais
- `DeviceRef { serial, transport, state }` em memória; logs substituem serial por identificador sanitizado.
- `Capabilities { mirror, virtual_display, desktop_windowing, input, audio, wifi }`. Valores `SUPPORTED/UNSUPPORTED/UNKNOWN/BLOCKED`, mais `reason`, `evidence`, `version`, `timestamp`. Padrão UNKNOWN; versão Android por si só não prova suporte.
- `Session` com estados `IDLE→DISCOVERING→WAITING_FOR_AUTH→READY→STARTING→ACTIVE→STOPPING→IDLE` e `ERROR` com stage e cleanup.
- `start(device, mode, options) → handle|error`, `stop(session_id)`, `probe(device)`, `subscribe(session_id)` apenas contratos conceituais.
- `MirrorProvider` implementado antes de `VirtualDisplayProvider`; `DesktopWindowingProvider` só após testes físicos.
- Seleção de múltiplos dispositivos explícita. No fallback, requer aviso visível; proibir fallback silencioso que surpreenda o usuário.

## Erros observáveis
`ADB_MISSING`, `UNAUTHORIZED`, `DEVICE_OFFLINE`, `SCRCPY_MISSING`, `VERSION_UNSUPPORTED`, `VIRTUAL_DISPLAY_UNAVAILABLE`, `INPUT_RESTRICTED`, `DISCONNECTED`, `PROCESS_EXITED`. Mensagem deve distinguir ação do usuário, falha de ferramenta e limite da ROM.

## Tratamento de processos
Orquestrador é proprietário apenas de subprocessos que iniciou, teardown idempotente, sem `killall adb` ou `killall scrcpy`. Testar corrida desconexão/fechamento, UI cancelável e timeout. Wi-Fi somente pareado/consentido, rede local, não exposição de portas públicas.

## Alternativas rejeitadas inicialmente
1. Codec/protocolo próprio: alto custo sem evidência de benefício.
2. APK privilegiado e root: exclui aparelhos, altera segurança do telefone.
3. Supor DeX/freeform universal: falso em várias ROMs.

## Referências
https://github.com/Genymobile/scrcpy
https://source.android.com/docs/core/display/multi_display
https://github.com/morluto/rea
