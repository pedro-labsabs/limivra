# Issue #29 — Launcher e apps permitidos: plano de implementação

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans para executar este plano após aprovação. Etapas usam checkboxes; desenvolvimento por limivra-dev e auditoria independente por limivra-qa.

**Goal:** Permitir selecionar e lançar explicitamente apps autorizados no display virtual da sessão, com resolução segura, erros observáveis e retorno ao controle do host.

**Architecture:** Um módulo `launcher` valida seleção e contexto; um runner ADB injetável resolve atividades MAIN/LAUNCHER e executa componentes explícitos. `HostControls` valida a intenção e `VirtualDisplayProvider` vincula o lançamento à sessão e ao display observado. Confirmação de comando não equivale a confirmação de foco ou compatibilidade multijanela.

**Tech Stack:** Rust, Tokio e dependências existentes; ADB e scrcpy externos, sem nova dependência nem APK.

**Spec:** [Issue #29](https://github.com/pedro-labsabs/limivra/issues/29), [PRD](../../product/PRD.md), [arquitetura](../../architecture/system.md), [estratégia de testes](../../testing/strategy.md) e instruções do Orquestrador nesta sessão.

## Estado e autorização

- Plano aprovado integralmente pelo Orquestrador nesta sessão, incluindo implementação, experimento USB, commit, push e PR. Evidência de execução: [issue-29-launcher-app-controls.md](../../testing/issue-29-launcher-app-controls.md).
- Worktree `/tmp/limivra-29`, branch `agent/29-launcher-app-controls`, base exata `f6bb7ad2239b8cf15574fe67a393bb95fa07b8ba`.
- Issue #29 OPEN, milestone M3, etiqueta `status:needs-evidence`, consultados no GitHub em 2026-10-21.
- Dependências #22 e #25 ainda OPEN: seus contratos estão presentes nesta base, mas isso não equivale a encerramento ou aprovação de integração. Registrar essa pendência no PR e revalidar antes de implementar.
- ADRs 0001–0003 permanecem PROPOSED; este plano autoriza somente a slice após aprovação humana, sem promovê-las a ACCEPTED. ADR-0004 aceita MIT para código original.
- Checkout principal possui alterações preexistentes e foi preservado; a execução ocorreu na worktree isolada.

## Global Constraints

- Sem root, instalação de companion, alteração persistente da ROM, elevação de privilégios ou bypass de restrições de apps/displays.
- Ações explícitas e reversíveis; seleção não executa apps automaticamente. Sem inventário global de apps, ícones ou histórico pessoal.
- Sem logs brutos de ADB/dumpsys, seriais completos, caminhos pessoais ou conteúdo de tela nos artefatos publicados.
- Não declarar desktop/freeform: SPIKE-028 comprovou floating individual e falha de duas janelas livres no cenário ensaiado.
- Sem shell host; argumentos separados ainda exigem validação porque `adb shell` atravessa shell remoto.
- Sem fallback silencioso para display físico, outro app ou outro dispositivo; sem retry automático de lançamento.

## Alternativas e decisão proposta

1. **Recomendada:** enum de apps permitidos + resolução MAIN/LAUNCHER restrita aos pacotes autorizados. Evita componentes OEM fixos e limita acesso.
2. Componentes fixos: mais simples, mas frágeis entre ROMs e versões.
3. Catálogo completo via PackageManager: amplia coleta e complexidade sem necessidade nesta slice; excluído.

## Review Focus

1. Componentes retornados por ADB podem conter lixo/injeção ou pertencer a outro pacote: rejeitar antes de lançar (tarefa 1/2).
2. Exit code zero pode acompanhar erro Android: classificar stdout e stderr, sem sucesso por código isolado (tarefa 2).
3. Display pode sumir ou outra sessão assumir o alvo durante resolução: invalidar contexto antigo (tarefa 3).
4. ROM pode reutilizar tarefa no display físico: verificar destino e reportar divergência sem mover/fechar apps privados (tarefas 3/4).
5. Foco pode permanecer desconhecido mesmo com lançamento aceito: não sintetizar confirmação de foco nem input (tarefas 3/4).

## Arquivos previstos

- Criar `crates/limivra-core/src/launcher.rs`: tipos validados, allowlist, contexto e orquestração.
- Criar `crates/limivra-core/src/launcher/adb_runner.rs`: runner real, comandos e parsing limitado.
- Modificar `crates/limivra-core/src/lib.rs`: exportar módulo.
- Modificar `crates/limivra-core/src/host_controls.rs`: intenção tipada de seleção/lançamento, sem executar processos.
- Modificar `crates/limivra-core/src/provider.rs`: integração e substituição do caminho permissivo de lançamento ADB desta slice; preservar mirror/fallback existentes.
- Criar `crates/limivra-core/tests/launcher.rs` e `tests/launcher_runner.rs`; ampliar `tests/host_controls.rs` para a intenção nova.
- Criar `docs/testing/issue-29-launcher-app-controls.md`: RED/GREEN, verificações e evidência física sanitizada.

## Tarefa 1 — Seleção validada e contexto (TDD)

**Interfaces propostas:** `AllowedApp::{Settings, Calculator}`; `PackageName::parse(&str) -> Result<Self, LaunchError>`; `ComponentName::parse(&str) -> Result<Self, LaunchError>`; `LaunchContext` com geração da sessão, alvo selecionado e display observado. Campos de componentes/contexto privados, construção controlada pelo provider; nenhum identificador de dispositivo em Debug público.

- [x] Escrever testes em `tests/launcher.rs`: Settings permite somente `com.android.settings`; Calculator permite `com.miui.calculator` ou `com.google.android.calculator`, nesta ordem, sem seleção silenciosa após erro de permissão.
- [x] Cobrir pacote com segmentos ASCII `[A-Za-z_][A-Za-z0-9_]*`, mínimo dois segmentos e limite de 255 bytes; atividade relativa `.Settings` ou qualificada, com segmentos equivalentes e `$` permitido apenas no corpo do segmento para classes internas; componente com exatamente uma `/`, limite de 512 bytes. Rejeitar espaços, Unicode, NUL, newline, `;`, aspas, backticks, `$(`, barras extras e prefixos de opção. Exigir que pacote retornado corresponda ao pacote consultado/permitido.
- [x] Introduzir somente interfaces mínimas para compilar; executar `cargo test -p limivra-core --test launcher --locked --offline` e guardar falha comportamental RED antes da implementação.
- [x] Implementar validação/allowlist; repetir suíte até GREEN. Não fornecer API pública de lançamento de componente arbitrário.

## Tarefa 2 — Resolução e subprocessos ADB (TDD)

**Interfaces propostas:** `LauncherRequest::{Resolve { package: PackageName }, Start { component: ComponentName, display_id: u32 }}`; `LauncherReply::{Resolved(ComponentName), NotInstalled, Accepted}`; `LauncherRunner::run(&self, target: &str, request: LauncherRequest) -> Pin<Box<dyn Future<Output = Result<LauncherReply, LaunchError>> + Send + '_>>`. O lifetime de `target` acompanha o de `self`. Mock armazena pedidos e devolve respostas predeterminadas. Runner real recebe caminho absoluto aprovado do ADB e alvo selecionado, nunca escolhido por saída de descoberta.

- [x] Escrever testes RED de argumentos exatos para `adb -s <alvo> shell cmd package resolve-activity --brief -a android.intent.action.MAIN -c android.intent.category.LAUNCHER -p <pacote>` e `adb -s <alvo> shell am start --display <id> -n <componente>`.
- [x] Cobrir resolução ausente, ambígua, componente inválido/externo e saída desconhecida. Aceitar somente um componente validado, com cabeçalhos conhecidos opcionais; demais formatos falham fechados. Não usar `monkey` nem instalar ferramentas no aparelho.
- [x] Cobrir `ActivityNotFound`, `SecurityException`, `DisplayDenied`, `Unauthorized`, `Offline`, `Timeout`, `UnexpectedReply` e falha de processo; verificar ambas as streams inclusive quando exit=0. `DisplayDenied` exige marcador de display reconhecido; SecurityException genérica continua erro de segurança. Ausência de app permite tentar a segunda variante da Calculadora; outros erros abortam.
- [x] Implementar runner com timeout de 5 segundos por subprocesso, teto de 16 KiB combinado de stdout/stderr e encerramento/reap do próprio filho em timeout, excesso ou cancelamento. `kill_on_drop` como proteção adicional; sem daemon privado novo nem kill-server.
- [x] Usar executáveis fake locais para provar timeout, limites, argv e cleanup, sem ADB/aparelho real nos testes. Mensagens públicas retornam somente erros tipados; descartar saída bruta sem tracing.
- [x] Executar `cargo test -p limivra-core --test launcher_runner --locked --offline`, registrar RED comportamental e GREEN.

## Tarefa 3 — Integração do host e provider (TDD)

**Interfaces propostas:** `HostControls::request_app(app: AllowedApp) -> Result<AllowedApp, ControlError>` exige Active/VirtualDisplay; `VirtualDisplayProvider::launch_allowed_app<R: LauncherRunner>(&self, app: AllowedApp, runner: &R) -> Result<LaunchOutcome, LaunchError>`. `LaunchOutcome::Accepted { display_id: u32 }` significa comando aceito, não foco comprovado. `ReturnToSession` é intenção local de devolver foco à janela da sessão pelo futuro adaptador UI; não envia HOME/BACK ao telefone.

- [x] Testar RED bloqueios em Disconnected/Connecting/Terminated/Mirror, sem chamada ao runner. Seleção permitida exige clique/intenção explícita; não criar atalhos globais.
- [x] Substituir no caminho de lançamento ADB do provider o aceite de strings não validadas pelo fluxo tipado. Preservar compatibilidade pública apenas com wrappers que validem e restrinjam à mesma allowlist; atualizar testes/call sites afetados sem conservar bypass permissivo.
- [x] Vincular display à sessão: capturar IDs anteriores ao start e exigir um único novo display virtual observado após criação, com prazo de 5 segundos. Não usar o primeiro ID não zero como prova de propriedade. Se ambíguo/ausente, retornar erro explícito e não lançar app.
- [x] Guardar geração de sessão e display confirmado; invalidar no stop, saída de processo, fallback ou restart. Revalidar imediatamente antes do start após resolução; descartar resposta atrasada. Testar troca de sessão e remoção de display via mock. Corrida externa após envio permanece limitação documentada, não sucesso inferido.
- [x] Testar que erro de app preserva sessão/stream e permite nova escolha/retorno local; não pausa vídeo, não injeta teclas nem modifica `input_policy`. Perda de sessão invalida contexto e libera entrada pelo fluxo existente.
- [x] Expor resultado/erro tipado ao chamador sem afirmar foco observado. Testar coexistência com controles atuais, fallback e ausência de estado Active artificial.
- [x] Rodar as suítes dedicadas e regressões do provider/host; guardar RED/GREEN. UI gráfica, launcher Android completo e transporte Wi-Fi novo ficam fora do escopo.

## Tarefa 4 — Prova física, evidências e gates

- [x] Após aprovação deste plano, revalidar USB autorizado no alvo indicado pelo Orquestrador, observador local e apps permitidos sem dados pessoais. Não publicar serial. Se indisponível, registrar BLOCKED para hardware, mantendo distinção dos testes unitários.
- [x] Executar harness local temporário usando as APIs reais desta slice, sem instalar APK. Abrir sessão virtual própria, registrar ID confirmado, resolver Configurações e Calculadora e lançar sequencialmente no mesmo display usando componentes resolvidos. Não fixar IDs/componentes descobertos no spike anterior.
- [x] Extrair em memória de `dumpsys window displays` e `dumpsys activity activities` somente componentes permitidos, displayId, visibilidade, modo, bounds e foco; descartar restante antes de escrever. Sem screenshots, logcat bruto ou dumps completos em disco.
- [x] Para cada app registrar pedido, resultado e destino real; tarefa reutilizada fora do alvo é FAIL/limitação explícita, mesmo com exit=0. Pedir relato local de imagem e retorno ao controle host. Não exigir coexistência freeform nem promover suporte desktop. Foco programático e foco humano têm evidências separadas.
- [x] Encerrar somente subprocessos e tarefas comprovadamente criados pelo experimento; não force-stop de pacote nem remoção de tarefa preexistente. Validar desaparecimento do display e ausência de processos próprios órfãos. Sem alterar settings persistentes.
- [x] Rodar `cargo test --workspace --locked --offline`, `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`, `cargo fmt --all -- --check` e `git diff --check`; registrar comandos, versões e exit codes reais no documento de evidência. Baseline será executada antes do código, sem confundir os 80 testes reportados na auditoria anterior com execução nesta worktree.
- [x] Conferir links relativos, sanitização, diff restrito e riscos. Após autorização de execução/publicação, commit/push da branch e PR para main com `Refs #29`, base, mudanças, RED/GREEN, resultado físico, pendências de integração e rollback. Não fazer merge nem fechar issues por conta própria.

## Critérios de aceite e falhas

- Seleção restrita; entradas/saídas maliciosas não chegam a subprocessos; resolução de app inexistente/ROM incompatível informa bloqueio explícito.
- Dois apps permitidos lançados e observados no display alvo por este fluxo, ou falha física documentada sem alegação de aceite total.
- Desconexão/invalidação não permite novo lançamento com contexto antigo; erro de app não encerra sessão válida nem captura foco em segredo.
- Gates automatizados verdes e evidência sanitizada; auditoria independente continua exclusiva de limivra-qa.
- Rollback: reverter commits da slice; parar filhos próprios e destruir display temporário. Nenhuma mudança persistente no Android a desfazer.

## Registro de execução

Aprovação recebida antes do código e do experimento. Etapas executadas pelo desenvolvedor; resultados, decisões de implementação e limites estão no documento de evidência. Auditoria independente será convocada pelo Orquestrador; este registro não a substitui.
