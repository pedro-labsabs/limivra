# Issue #30 — Contratos, fakes e falhas de processo

## Escopo e limites

A suíte valida parsing hostil, limites de subprocessos, códigos de saída, timeout,
cancelamento, sinalização, reaping e corridas de estado em `limivra-core` e
`limivra-cli`. Os cenários usam dados sintéticos e executáveis controlados por
cada teste.

Nenhum teste desta issue usa telefone físico, daemon ADB real ou rede externa.
O runner Wi-Fi exercita somente socket Unix privado em diretório temporário;
isso não é transporte de rede nem evidência de pareamento real. A suíte não
prova compatibilidade de ROM, UI, foco gráfico, Wayland/X11 ou comportamento de
scrcpy real.

## Contratos cobertos

| Contrato | Fake/fixture | Comando focalizado | Resultado esperado |
|---|---|---|---|
| Saída inesperada e término por sinal | Filho shell com PID marker | `cargo test -p limivra-core --locked --offline --test session` | Códigos 1, 7 e 127 preservados; SIGTERM projetado como término; estado `Terminated` |
| Stop/reaping idempotente e sem afetar outro processo | Dois filhos independentes com PID conhecido | `cargo test -p limivra-core --locked --offline --test session` | Stop mata/recolhe apenas o filho próprio; segunda chamada continua desconectada |
| Vinte ciclos rápidos start/stop | Filho identificado por PID e processo sentinela | `cargo test -p limivra-core --locked --offline --test session rapid_start_stop_cycles_reap_every_owned_child` | Todos os filhos próprios recolhidos; sem evento tardio de término; sentinela preservado |
| Parsing ADB, dumpsys e versão scrcpy | Strings sintéticas truncadas, incompletas, fora de ordem, com U+FFFD e linhas longas | `cargo test -p limivra-core --locked --offline --test parsing_contracts` | Nenhum panic; registros incompletos/linhas acima de 16 KiB não viram resposta confiável |
| Executor limitado e status de processo | `/bin/sh`, `/bin/sleep`, `/usr/bin/head`, PID markers | `cargo test -p limivra-core --locked --offline process::tests` | Exit code preservado; timeout, cancelamento e stdout/stderr acima de 1 MiB encerram e recolhem o filho |
| Respostas UTF-8 inválidas e limites existentes | Fakes ADB da suíte launcher/Wi-Fi | `cargo test -p limivra-core --locked --offline --test launcher_runner --test wifi_runner` | Resposta inválida falha fechada; saídas excedentes não são aceitas como prefixo |
| Reconexão durante lançamento e cancelamento durante probe | `Notify`, runners fake e respostas controladas | `cargo test -p limivra-core --locked --offline --test launcher --test wifi` | Geração antiga não altera sessão nova; cancelamento não publica conexão tardia |
| Contrato da CLI sem ferramentas reais | `CARGO_BIN_EXE_limivra-cli`, `PATH`/`HOME` temporários e fakes locais | `cargo test -p limivra-cli --locked --offline --test cli_process_contracts` | Apenas fake ADB/scrcpy executado; serial sentinela redigido; bytes inválidos/saída acima de 1 MiB não vazam |

## Limites e isolamento

- Probes genéricos de ADB, dumpsys e scrcpy usam `process::run_bounded`, timeout
  de 5 segundos e limite independente de 1 MiB por stream.
- Parsers estruturados ignoram linhas acima de 16 KiB e linhas com `U+FFFD`;
  bytes UTF-8 inválidos são rejeitados na fronteira do subprocesso.
- O runner de launcher conserva seu limite combinado atual de 16 KiB. O runner
  Wi-Fi conserva 8192 bytes por stream e credenciais de pareamento entram por
  stdin privado.
- Fixtures de processo usam diretórios temporários com permissões `0700`,
  executáveis locais, PID markers e teardown restrito aos próprios PIDs. A suíte
  de runner Wi-Fi, de sessão e da CLI serializa operações que escrevem e iniciam
  executáveis fake para evitar `ETXTBSY` interteste; concorrência e cancelamento
  continuam exercitados dentro dos cenários dedicados.
- A integração CLI usa `env_clear()`, `PATH` contendo só o diretório de fakes,
  `HOME` temporário, `LC_ALL=C` e variáveis sintéticas de log. O fake registra
  argumentos para comprovar invocação, sem consultar telefone.
- A execução Linux usa utilitários locais `/bin/sh`, `/bin/sleep`,
  `/usr/bin/head` e `/usr/bin/python3` (este último já usado pelos fakes Wi-Fi
  existentes). Nenhum deles acessa rede externa.

## RED/GREEN observado

- A suíte de sessão confirmou como contratos existentes os eventos para saída
  1/7/127 e SIGTERM; não foi necessária mudança de produção nessa fronteira.
- RED dos parsers: 5 testes falharam no código inicial, incluindo panic em linha
  ADB sem coluna de status, aceitação de versão `3.x` e inclusão de displays sem
  bounds completos. GREEN após validação estrita de registros e limite de linha.
- RED do executor esqueleto: os quatro testes falharam por falta de execução,
  timeout, limite de saída e cancelamento/reaping. GREEN após `run_bounded` com
  leitura simultânea, timeout, `kill_on_drop` e `wait` explícito nas falhas
  observadas.
- Durante a verificação foram corrigidas duas corridas em fixtures existentes:
  prazo Wi-Fi insuficiente para inicialização do fake e fechamento de stdin sem
  dados sendo classificado como `Io` quando o fake terminava rápido. Os testes
  agora preservam timeouts explícitos e sincronização por PID/`Notify`.
- Os testes de reconexão/cancelamento e da CLI caracterizam contratos já
  satisfeitos; não exigiram mudança de produção adicional.

## Ambiente e gates

Preencher somente com os resultados dos comandos executados nesta worktree.

| Item | Resultado |
|---|---|
| Worktree / branch | `/tmp/limivra-30` / `agent/30-contract-tests-fakes` |
| Base | `2ba92849e532f8551e70fc723b912117691478c5` |
| Sistema | `Linux 7.0.0-30-generic` |
| Rust / Cargo | `rustc 1.88.0 (6b00bc388 2025-06-23)` / `cargo 1.88.0 (873a06493 2025-05-10)` |
| `cargo test --workspace --locked --offline` | exit 0; 128 testes passaram |
| Validação posterior à correção de isolamento | `CARGO_NET_OFFLINE=true cargo test --workspace --locked --offline`: exit 0; 128 testes |
| Validação final após ajustes de lint | `CARGO_NET_OFFLINE=true cargo test --workspace --locked --offline`: exit 0; 128 testes |
| Repetições herméticas pós-correção | `CARGO_NET_OFFLINE=true cargo test --workspace --locked --offline`, 10/10 rodadas exit 0; cada rodada executou 128 testes. Tabela abaixo. |
| `cargo fmt --all -- --check` | exit 0 |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | exit 0; zero warnings |
| `git diff --check origin/main...HEAD` | exit 0; nenhum erro de whitespace |

Detalhe das dez repetições pós-correção:

| Rodada | Exit code | Testes |
|---:|---:|---:|
| 1 | 0 | 128 |
| 2 | 0 | 128 |
| 3 | 0 | 128 |
| 4 | 0 | 128 |
| 5 | 0 | 128 |
| 6 | 0 | 128 |
| 7 | 0 | 128 |
| 8 | 0 | 128 |
| 9 | 0 | 128 |
| 10 | 0 | 128 |

Na primeira série exploratória, antes do isolamento dos fixtures de sessão e
CLI, a rodada 5/10 falhou com `ETXTBSY` em `rapid_start_stop_cycles_reap_every_owned_child` (91 testes haviam terminado antes da falha). As operações de escrita e execução dos fakes foram serializadas dentro desses test binaries; depois disso, 20 execuções focadas de `tests/session.rs`, uma suíte workspace e as dez repetições completas passaram. A falha inicial não foi omitida desta evidência.

## Prontidão reconciliada

O Orquestrador informou que a dependência técnica da Issue #15 está satisfeita
desde PR #42 e que a Issue #30 pertence formalmente ao Épico #6 / M4. A
visualização atual do GitHub pode continuar desatualizada; esta slice não altera
issue, milestone nem outro metadado externo.
