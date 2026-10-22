# Issue #31 — Threat model, sanitização de logs e política de segurança

**Branch:** `agent/31-threat-model-security`
**Base isolada da Issue #31:** `agent/30-contract-tests-fakes` em `6fd1fbdeb3fb4cdb14572f8d51fc9295c854e56d`.
**Base cumulativa da PR #57:** `origin/main` em `76d5294de19dafac3daa4a26b0a9ff00af85f228`.
**Escopo:** threat model, política de disclosure, testes de contrato de segurança e hardening mínimo das superfícies cobertas. Nenhum teste físico foi necessário ou executado.

## Escopos do diff

A slice original da Issue #31 continha 10 arquivos quando comparada isoladamente com `agent/30-contract-tests-fakes` (`6fd1fbdeb3fb4cdb14572f8d51fc9295c854e56d`). A revisão de QA adiciona `crates/limivra-core/src/process.rs` à slice; o diff isolado atualizado contém 11 arquivos, incluindo esta evidência.

A PR #57 usa `origin/main` (`76d5294de19dafac3daa4a26b0a9ff00af85f228`) como base e apresenta o diff cumulativo de 55 arquivos dos branches M3/M4 ainda aguardando integração sequencial. Esse escopo é distinto do diff isolado da Issue #31.

## Evidência de regressão

Antes do hardening, `CARGO_NET_OFFLINE=true cargo test -p limivra-core --test security_contracts --locked --offline` terminou com exit 101: o teste de modo de diretório aceitou `0500`, `AdbDevice::Debug` imprimiu o serial sentinela e o erro de spawn preservou o serial inserido no caminho do executável. O teste de perda de foco já passou e registrou a expectativa funcional existente.

Após as correções, os quatro testes da integração `security_contracts` passaram. O teste unitário direcionado `pairing_code_drop_zeroizes_its_owned_buffer` também passou, observando em hook restrito a `cfg(test)` que o buffer já está zerado durante a execução do `Drop` (sem leitura após desalocação).

## Cobertura automatizada

| Contrato | Prova automatizada |
|---|---|
| Argumentos com `-s`, `--serial`, forma `=`, título e texto arbitrário redigem serial | `security_contracts::serial_is_redacted_from_all_transport_argument_forms_and_debug_models` |
| `Debug` e serde de `AdbDevice`, `DeviceProfile` e `SessionState` não expõem serial | Mesmo teste de modelos, com serial sentinela `LIMIVRA-TEST-SERIAL-XYZ` |
| Trace, `CoreError` e `SessionEvent::Error` de falha real de spawn não expõem serial | `security_contracts::spawn_failure_redacts_serial_from_trace_error_and_session_event` |
| CLI e argumentos de scrcpy | `cli_session_output_redacts_serials`, `devices_uses_only_fake_adb_and_redacts_serial_sentinel`, `probe_failure_has_bounded_non_sensitive_output` |
| Probe/log de display | `display::tests::display_probe_log_redacts_serial` |
| Buffer de `PairingCode` zerado no `Drop` | `wifi::pairing_code_tests::pairing_code_drop_zeroizes_its_owned_buffer` |
| Diretórios privados de ADB aceitam `0700` e rejeitam `0500` | `security_contracts::private_adb_socket_directories_require_exact_0700_permissions` |
| Perda de foco emite liberação de todas as teclas, botões e modificadores | `security_contracts::losing_focus_releases_every_pressed_key_button_and_modifier`; suíte anterior também cobre eventos de sessão/consentimento |

O monitor de foco mantém controle IDs somente enquanto pressionados; isso não equivale a texto digitado. Releases são intenções do core; a futura camada de UI/adapter continua responsável por entregar os eventos ou destruir o dispositivo de input próprio em perda do transporte.

## Correção da corrida no teardown da fixture

O teardown mantém o `pidfd` aberto e agora repete a leitura de `/proc/<pid>/environ` por até 10 tentativas, aguardando 5 ms entre leituras sem correspondência. O `SIGKILL` via `pidfd_send_signal` só é enviado depois que o ambiente confirma o marcador com o caminho exato da fixture. O teste `fixture_drop_kills_its_marked_child_and_child_handle_reaps_it` também passa `LIMIVRA_PROCESS_FIXTURE_ROOT` explicitamente no ambiente inicial do `Command`, para que o marcador esteja presente desde o `execve` inicial.

O teste unitário `fixture_marker_check_retries_after_transient_missing_environment` simula uma primeira leitura indisponível seguida pelo marcador correto. Ele falhou com a implementação de uma só tentativa e passou após o retry. Os 100 ciclos do teste de teardown original executados antes da correção passaram; essa execução não reproduziu a janela rara identificada pela auditoria e não é usada como prova de estabilidade da correção.

## Documentos alterados

- `docs/security/threat-model.md`: objetivos, limites, fronteiras de confiança, ativos, ameaças, controles e riscos residuais.
- `SECURITY.md`: canal privado preferido, contingência honesta enquanto o recurso GitHub está desabilitado, SLA de resposta, escopo e regras para pesquisadores.
- `crates/limivra-core/tests/security_contracts.rs` e testes unitários de `PairingCode`.
- `adb.rs`, `display.rs`, `lib.rs`, `session.rs` e `wifi/adb_runner.rs`: redaction em representações/erros, serde privado, zeroização volátil e validação `0700`.

## Gates executados

Executados no worktree `/tmp/limivra-31` em 2026-10-21 com `rustc 1.88.0 (6b00bc388 2025-06-23)` e `cargo 1.88.0 (873a06493 2025-05-10)`:

| Comando | Resultado |
|---|---|
| `CARGO_NET_OFFLINE=true cargo test --workspace --locked --offline` | Validação original: exit 0; 142 testes passaram. Após a correção: 10/10 execuções completas passaram, cada uma com 143 testes, 0 falhas e 0 doctests. |
| `cargo fmt --all -- --check` | exit 0 após a correção |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | exit 0 após a correção; zero warnings |
| `git diff --check origin/main...HEAD` | exit 0; sem erros de whitespace no diff cumulativo, repetido após a atualização final desta evidência. |

Em 5 das 10 execuções completas, o stderr imprimiu `sleep: invalid option -- '-s'`, sem falha de teste ou alteração do exit code. A origem provável é o teste preexistente `provider::tests::test_provider_fallback_when_host_unsupported`, que usa `sleep` como mock de scrcpy enquanto o provider fornece argumentos reais de scrcpy. Isso é ruído do mock, não falha intermitente do teardown; nenhuma alteração foi feita nesse teste fora do escopo da revisão.

## Limitações, risco e reversão

Esta mudança não altera configurações do repositório GitHub; em 2026-10-21, `GET /repos/pedro-labsabs/limivra/private-vulnerability-reporting` retornou `enabled: false`. Por isso SECURITY.md orienta solicitar canal privado ao mantenedor sem publicar detalhes até o recurso ser habilitado ou um canal privado ser combinado.

Não foram inspecionadas portas TCP em um host externo, capturados dados de aparelho nem validados clipboard/GUI. A garantia de serial cobre a CLI e representações estruturadas atualmente exercitadas; campos operacionais públicos continuam contendo serial enquanto a sessão precisa dele. Zeroização não cobre cópias externas, runtime, processo ADB ou kernel. Para reverter o hardening sem perder os documentos, rever apenas as alterações de código neste commit; a documentação de política/threat model deve permanecer alinhada com o estado vigente.
