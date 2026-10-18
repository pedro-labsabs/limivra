# Estratégia de testes e evidências

**Estado:** proposta (não existe suíte implementada). Priorizar provas determinísticas e não simular sucesso.

## Pirâmide de testes
- Unitários: parsing ADB, seleção 0/1/N aparelhos, estados UNKNOWN/BLOCKED, filtros de logs, mensagens de erro, transições e cleanup idempotente.
- Integração: executáveis fake para ADB/scrcpy, stderr, exit codes, timeouts, version gating, concorrência e encerramento.
- E2E físico: em Redmi Note 13 4G (e depois mais ROMs), execução de SPIKE-001 e reprodução de ações com observador humano.
- UI: foco em Wayland e X11, teclas e janela desacoplada; não inferir suporte Wayland a partir de teste X11.
- Segurança: ausência de root/sudo, logs redigidos, conexão Wi-Fi só autorizada, nenhum token em artefato.
- Packaging: validação de dependências/licenças, instalação e desinstalação num Linux limpo.

## Matriz obrigatória
`Linux distro/version | Wayland/X11 | scrcpy version | adb version | vendor/model | Android/ROM version | USB/Wi-Fi | mirror | virtual | freeform | input | logs/evidence`.
`UNTESTED` e `UNKNOWN` não contam como PASS.

## Qualidade para cada PR
1. Teste RED onde mudança corrige bug.
2. GREEN local com comandos e outputs de verdade.
3. Testes de integração pertinentes.
4. Evidência manual se a issue afirma comportamento em hardware.
5. Revisão de segurança/licença, `git diff --check`.
6. Risco, rollback e o que *não* foi testado.

## Gates de release
Nenhum release enquanto houver: licença não definida, caminho básico sem-root não comprovado, falha crítica de privacidade, processo órfão reproduzido sem mitigação, CI inexistente, ausência de documentação de instalação ou claims de compatibilidade sem matriz.

## Modelo de relato
```text
Ambiente:
Comando/etapas:
Esperado:
Observado:
PASS/FAIL/BLOCKED/UNTESTED:
Evidência sanitizada:
Limitações:
Proposta de próximo teste:
```
