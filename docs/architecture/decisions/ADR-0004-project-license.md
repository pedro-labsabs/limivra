# ADR-0004 — Escolha da licença do projeto (MIT)

**Status:** ACCEPTED (2026-10-18) — restrito à decisão de licenciamento do código original do Limivra.

## Contexto

O Limivra necessitava de uma definição formal de licença de código aberto para seu repositório e código original antes do início da implementação do produto (Milestone M0, Issue [#7](https://github.com/pedro-labsabs/limivra/issues/7)). A ausência de arquivo de licença mantinha o projeto sob regime padrão de direitos autorais exclusivos, inviabilizando contribuições externas estruturadas e clareza jurídica.

O ecossistema previsto para o Limivra interage com ferramentas e bibliotecas sob diferentes licenças:
- `scrcpy` (Apache-2.0, executado como subprocesso conforme [ADR-0001](ADR-0001-scrcpy-external.md));
- `Qt 6` (LGPLv3 no conjunto base, com certos módulos sob GPLv3 ou comercial);
- `ADB / Platform-Tools` (Apache-2.0 no AOSP / termos do SDK Google para binários);
- Ecossistema `Rust` (predominantemente dual `MIT OR Apache-2.0`).

Uma análise aprofundada de requisitos, obrigações de linking, concessão de patentes e compatibilidade de distribuição foi consolidada em [docs/governance/license-decision-packet.md](../../governance/license-decision-packet.md).

## Alternativas consideradas

Conforme documentado no pacote de decisão (`docs/governance/license-decision-packet.md`), três licenças candidatas principais foram avaliadas:

1. **MIT (SPDX: `MIT`):** Permissiva minimalista, amplamente aceita no ecossistema Rust e em distribuições Linux, com excelente interoperabilidade, baixa barreira de adoção e regras simples de preservação de copyright.
2. **Apache-2.0 (SPDX: `Apache-2.0`):** Permissiva estruturada com concessão e retaliação explícita de patentes e regras formais de atribuição/`NOTICE`, mas com atrito histórico pontual em combinação com copyleft estrito GPLv2 e maior formalismo de distribuição.
3. **GPL-3.0-or-later (SPDX: `GPL-3.0-or-later`):** Copyleft forte com reciprocidade obrigatória e proteção anti-tivoização, assegurando que derivados permaneçam abertos, mas impondo restrições rígidas à reutilização de código e exigindo que qualquer aplicação combinada seja 100% GPLv3.

## Decisão

Adotar a **Licença MIT** (SPDX: `MIT`) para todo o código original do projeto Limivra contido neste repositório.

- **Texto da licença:** Canônico conforme padronizado pela OSI e SPDX.
- **Titularidade inicial do copyright:** `Copyright (c) 2026 renyennos`.
- **DCO / CLA:** Não exigido neste momento; contribuições enviadas ao repositório submetem-se aos termos da licença MIT.

Esta aceitação refere-se **estritamente à decisão de licenciamento do projeto**. As ADRs de arquitetura existentes ([ADR-0001](ADR-0001-scrcpy-external.md), [ADR-0002](ADR-0002-capability-first.md), [ADR-0003](ADR-0003-no-root.md)) permanecem como `PROPOSED` sujeitas a testes empíricos de implementação e comprovação técnica.

## Consequências

### Para o projeto e desenvolvedores
+ **Simplicidade e clareza:** Máxima facilidade de compreensão, empacotamento e distribuição nas principais distribuições Linux.
+ **Harmonia com Rust:** Total alinhamento com as convenções e tolerâncias de licença padrão da comunidade Rust (`crates.io`).
+ **Interoperabilidade:** O código original do Limivra pode ser consumido, empacotado ou integrado por outros projetos livres com mínimas restrições, bastando reter o aviso de copyright e o texto da licença MIT.

### Consequências para dependências e terceiros
- **Não extensão a terceiros:** A licença MIT aplica-se estritamente ao código original do Limivra. Componentes externos, ferramentas invocadas e bibliotecas linkadas continuam sujeitos às suas respectivas licenças de origem (ex.: Apache-2.0 do scrcpy, LGPLv3/GPLv3 do Qt 6).
- **Obrigações da LGPLv3 (Qt):** A adoção de MIT no código da aplicação não isenta a distribuição binária combinada de cumprir as exigências da LGPLv3 do Qt (fornecimento de código-fonte das bibliotecas Qt utilizadas, instruções de substituição e mecanismo de relink dinâmico ou estático com objetos compilados).
- **Módulos Qt GPLv3-only:** O uso de qualquer módulo Qt exclusivo sob GPLv3 (como `Qt Quick 3D` ou `Qt Wayland Compositor`) exigiria que a aplicação distribuída como um todo cumprisse os termos da GPLv3. A arquitetura do Limivra deve priorizar módulos LGPLv3 para manter a aplicação distribuível nos termos pretendidos.
- **Distribuição de bundles:** Pacotes agregados (Flatpak, AppImage ou instaladores) que redistribuam binários de terceiros (como scrcpy ou bibliotecas Qt) devem agregar as licenças e cumprir integralmente os avisos de copyright e obrigações correspondentes de cada componente redistribuído.
