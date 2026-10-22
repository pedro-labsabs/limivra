# Issue #31 — Threat model, sanitização de logs e política de segurança

**Branch:** `agent/31-threat-model-security`
**Base:** `agent/30-contract-tests-fakes` em `6fd1fbdeb3fb4cdb14572f8d51fc9295c854e56d`.
**Escopo:** threat model, política de disclosure, testes de contrato de segurança e hardening mínimo das superfícies cobertas. Nenhum teste físico foi necessário ou executado.

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

## Documentos alterados

- `docs/security/threat-model.md`: objetivos, limites, fronteiras de confiança, ativos, ameaças, controles e riscos residuais.
- `SECURITY.md`: canal privado preferido, contingência honesta enquanto o recurso GitHub está desabilitado, SLA de resposta, escopo e regras para pesquisadores.
- `crates/limivra-core/tests/security_contracts.rs` e testes unitários de `PairingCode`.
- `adb.rs`, `display.rs`, `lib.rs`, `session.rs` e `wifi/adb_runner.rs`: redaction em representações/erros, serde privado, zeroização volátil e validação `0700`.

## Gates executados

Executados no worktree `/tmp/limivra-31` em 2026-10-21 com `rustc 1.88.0 (6b00bc388 2025-06-23)` e `cargo 1.88.0 (873a06493 2025-05-10)`:

| Comando | Resultado |
|---|---|
| `CARGO_NET_OFFLINE=true cargo test --workspace --locked --offline` | exit 0; 142 testes passaram, 0 falharam; doctests: 0 |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | exit 0; zero warnings |
| `git diff --check origin/main...HEAD` | exit 0; sem erros de whitespace no diff cumulativo, executado após `184753d` |

Durante uma execução completa, o stderr imprimiu uma vez `sleep: invalid option -- '-s'`. A origem provável foi o teste preexistente `provider::tests::test_provider_fallback_when_host_unsupported`, que usa `sleep` como mock de scrcpy enquanto o provider fornece argumentos reais de scrcpy. A execução isolada desse teste passou sem reproduzir a linha; o exit code da suíte completa permaneceu 0. Nenhuma alteração foi feita nesse teste fora do escopo da Issue #31.

## Limitações, risco e reversão

Esta mudança não altera configurações do repositório GitHub; em 2026-10-21, `GET /repos/pedro-labsabs/limivra/private-vulnerability-reporting` retornou `enabled: false`. Por isso SECURITY.md orienta solicitar canal privado ao mantenedor sem publicar detalhes até o recurso ser habilitado ou um canal privado ser combinado.

Não foram inspecionadas portas TCP em um host externo, capturados dados de aparelho nem validados clipboard/GUI. A garantia de serial cobre a CLI e representações estruturadas atualmente exercitadas; campos operacionais públicos continuam contendo serial enquanto a sessão precisa dele. Zeroização não cobre cópias externas, runtime, processo ADB ou kernel. Para reverter o hardening sem perder os documentos, rever apenas as alterações de código neste commit; a documentação de política/threat model deve permanecer alinhada com o estado vigente.
