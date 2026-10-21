# Estratégia de testes e evidências

**Estado:** estratégia de testes ativa para a suíte unitária e de integração offline existente. A matriz de compatibilidade e a experiência no aparelho continuam exigindo evidência física separada; testes de fakes não certificam comportamento de ROM.

## Pirâmide de testes
- Unitários: parsing ADB, seleção 0/1/N aparelhos, estados UNKNOWN/BLOCKED, filtros de logs, mensagens de erro, transições e cleanup idempotente.
- Integração: executáveis fake para ADB/scrcpy, stderr, exit codes, timeouts, version gating, concorrência e encerramento.
- Contratos de processo: fakes isolados por teste, PID/readiness markers, grupos Unix privados, limites de stdout/stderr, timeout, cancelamento, sinalização e verificação de reaping de filhos e descendentes; fixtures sinalizam e recolhem filhos que iniciaram diretamente em teardown de falha, e sinalizam apenas PIDs publicados em seu próprio diretório. Não herdar `PATH`, `HOME` ou configuração ADB do operador quando o teste exerce a CLI.
- CLI `doctor`: sondar `adb --version` com timeout e limite de saída; distinguir ferramenta ausente, falha de leitura, timeout, saída excedida e exit não zero.
- Parsing defensivo: ignorar registros incompletos, linhas estruturadas acima de 16 KiB e texto com caractere de substituição; rejeitar bytes UTF-8 inválidos. Probes genéricos limitam cada stream a 1 MiB e não aceitam prefixos como respostas completas.
- Repetibilidade: comandos herméticos usam `--locked --offline`; a evidência da Issue #30 registra execuções consecutivas da suíte workspace antes de atribuir ausência de flakiness.
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

## Evidência da Issue #30

`docs/testing/issue-30-contract-tests-fakes.md` registra fixtures, casos de falha de processo, limites e resultados reproduzíveis da validação offline. Esses testes não iniciam telefone, daemon ADB real nem rede externa; testes de integração física permanecem no fluxo E2E separado.

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
