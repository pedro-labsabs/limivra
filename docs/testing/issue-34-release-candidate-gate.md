# Issue #34 — Gate do release candidate e relatório de limites

**Branch:** `agent/34-release-candidate-gate`
**Base isolada da slice:** `agent/33-packaging-dependencies-notices` em `9df11b47a8f6d702e8113fc121f6a6d1108dbc1e`.
**Base cumulativa da PR:** `origin/main` em `76d5294de19dafac3daa4a26b0a9ff00af85f228`.
**Issue #34:** `OPEN`, milestone `M4 — Hardening & Release`, label `status:needs-evidence` na consulta de 2026-10-21.

## Decisão do gate

**Resultado: HOLD / NEEDS_EVIDENCE.** Os artefatos técnicos e os gates automatizados desta slice podem passar, mas isso não autoriza publicar o tarball como release, declarar M0–M4 encerrados ou substituir a aprovação do mantenedor. A Issue #20 permanece aberta, o aplicativo desktop completo e o smoke em distro limpa não existem, e a cadeia de PRs M3/M4 ainda aguarda integração sequencial.

Esta revisão é uma atestação técnica do executor limitada às evidências citadas. Não é auditoria independente, assinatura criptográfica, decisão jurídica, aprovação de integração ou autorização de distribuição.

**Reconciliação do tracker:** a consulta ao GitHub em 2026-10-21 mostra Issues #30 e #15 no milestone M1, Issue #31 no M0, e Issues #32–#34 no M4. O contexto de execução do projeto agrupa os artefatos de #30/#31 em M4; esta slice não altera milestones/labels externos e registra a divergência para reconciliação pelo mantenedor.

## Checklist assinado por milestone

| Milestone | Evidência consultada | Prontidão técnica nesta revisão |
|---|---|---|
| M0 — Fundação e viabilidade | `ADR-0004` aceita MIT; `ADR-0001` a `ADR-0003` permanecem `PROPOSED`; SPIKE-001 está documentado; `ADR-0005` não existe no branch-base; Issue #9 permanece `OPEN`. | **PARTIAL / NEEDS_RECONCILIATION** — não declarar a série ADR-0001 a ADR-0005 completa. |
| M1 — Sessão Linux via USB | O registro USB descreve mirror e teardown observados; Issues #15 e #20 permanecem `OPEN`. A execução deste gate foi offline e não repetiu o teste físico. | **NEEDS_EVIDENCE** — conservar a evidência histórica; obter fechamento/revisão da Issue #20 antes de declarar o gate de fase encerrado. |
| M2 — Display virtual e fallback | SPIKE-001 registra display virtual, lançamento de app e fallback mirror; Issue #23 permanece `OPEN`. | **PARTIAL** — capacidade observada no dispositivo ensaiado, sem generalização de compatibilidade ou fechamento do issue. |
| M3 — Desktop UX e Wi-Fi | Core e CLI contêm contratos de host controls, input policy, Wi-Fi ADB e launcher. Issues #25–#29 permanecem `OPEN`; não existe aplicativo desktop completo. SPIKE-028 falha o critério de duas janelas livres no cenário testado. | **PARTIAL** — contratos e observações presentes; integração de GUI, pareamento Wi-Fi físico e multijanela livre não são aprovados por este gate. |
| M4 — Hardening e release | Testes herméticos, Threat Model, `SECURITY.md`, CI Linux, tarball e notices estão na cadeia de branches. Issues #30–#34 e PR #59 permanecem `OPEN`; licença/assinatura integral de notices e distribuição em distro limpa estão pendentes. | **PASS dos artefatos e gates locais quando indicados abaixo; HOLD para fechamento do milestone e publicação.** |

### Assinatura técnica do executor

`limivra-dev` — executor da Issue #34 — 2026-10-21. A assinatura confirma apenas que os estados e resultados observados nesta checklist foram registrados; não aprova as pendências como concluídas.

## Gates automatizados desta slice

O baseline da Issue #33 tinha 149 testes listados. A Issue #34 acrescenta um contrato de release candidate; o total final deve ser confirmado pelo comando workspace abaixo.

| Comando | Resultado nesta revisão |
|---|---|
| `CARGO_NET_OFFLINE=true cargo test --workspace --locked --offline` | PASS — exit 0; 150 testes passaram, 0 falharam; doctests: 0 |
| `cargo fmt --all -- --check` | PASS — exit 0 |
| `cargo clippy --workspace --all-targets --locked --offline -- -D warnings` | PASS — exit 0, sem warnings |
| `git diff --check origin/main...HEAD` | PENDENTE após commit da slice |
| GitHub Actions da PR da Issue #34: Format, Clippy, Workspace tests e host graphics smoke | PENDENTE após publicação da PR |
| GitHub Actions da PR #59 (base da slice) | PASS na consulta de 2026-10-21; PR #59 permanece aberta |

O teste `release_candidate_gate` valida versão do workspace contra `v0.1.0-rc1`, exige que o manifesto SHA-256 cubra exatamente as release notes e esta checklist, verifica os hashes e constrói/abre o binário do tarball para executar `--version`. Isso valida o artefato host Linux x86_64, não uma instalação em distro limpa nem um teste físico.

## Riscos e pendências antes de qualquer distribuição pública

| Risco / pendência | Estado e ação necessária |
|---|---|
| E2E USB e teardown | Issue #20 está `OPEN`. Reconciliar a revisão do registro existente e o fechamento do issue; esta fase offline não repetiu o telefone. |
| ADR-0005 | Ausente no branch-base. Localizar a decisão pretendida ou corrigir a referência M0 antes de alegar fechamento documental. |
| ROM e desktop multijanela | SPIKE-028 é `FAIL` para duas janelas livres simultâneas no par de apps/build ensaiados; manter o produto sem claim de desktop livre. |
| Host gráfico | CI usa marcadores X11/Wayland controlados. Validar conexão real de compositor e UX gráfica numa etapa autorizada. |
| Distribuição Linux | Lifecycle atual usa `PREFIX` temporário no host. Fazer smoke em distro limpa e documentar ABI/dependências do sistema. |
| Integridade/publicação | SHA-256 não autentica o publicador. Definir e testar assinatura criptográfica de pacotes antes de distribuição pública. |
| Copyrights de terceiros | `NOTICES.md` é triagem SPDX dos 65 pacotes; agregar/revisar textos completos de copyright upstream na fase pré-release. |
| Integração | PR #59 e Issues #30–#34 ainda estão abertas na consulta. Aguardar revisão/merge sequencial do mantenedor sem mesclar automaticamente. |

## Rollback

Esta slice adiciona documentação, manifesto e um teste de contrato; não altera o telefone, serviços externos ou runtime de produção. Para reverter, reverta o commit da Issue #34. Regere o manifesto SHA-256 se qualquer documento coberto mudar. Não publique artefato RC enquanto o resultado estiver `HOLD / NEEDS_EVIDENCE`.

## Bases e escopo de diff

A slice isolada da Issue #34 contém quatro arquivos sobre o commit `9df11b47a8f6d702e8113fc121f6a6d1108dbc1e`: release notes, checklist/evidência, manifesto SHA-256 e teste de contrato. A PR para `main` é cumulativa: 70 caminhos contra `origin/main` em `76d5294de19dafac3daa4a26b0a9ff00af85f228` (66 caminhos anteriores mais os quatro desta slice). Esses totais foram medidos depois do commit da slice; não atribuir o diff cumulativo inteiro à Issue #34.
