# ADR-0005 — Núcleo Rust/CLI e interface futura Qt 6/QML

**Status:** ACCEPTED (2026-10-22), limitado à direção Session Core/CLI em Rust, interface Linux futura em Qt 6/QML e ADB/scrcpy como subprocessos externos supervisionados. Contratos e implementação permanecem pendentes.
**Data:** 2026-10-22
**Issue:** [#11 — Viabilidade Qt/QML + Rust + scrcpy no Linux](https://github.com/pedro-labsabs/limivra/issues/11)

## Contexto e problema técnico

A Issue #11 pediu a avaliação de custos e dependências antes da estruturação do produto. Em 22/10/2026, o mantenedor ratificou somente a direção delimitada nesta ADR. Essa ratificação não aprova todo o PRD ou a especificação, não confirma comportamento em dispositivo e não libera a Issue #15.

A direção aceita é um Session Core e CLI em Rust, interface gráfica Linux futura em Qt 6/QML e ADB/scrcpy como ferramentas externas supervisionadas em subprocessos. A avaliação das alternativas permanece qualitativa; nenhuma alternativa foi implementada ou medida por esta ADR.

A proposta preserva três modos com evidências separadas:

- **MIRROR:** captura e controle do display físico.
- **VIRTUAL_DISPLAY:** criação de display secundário e abertura de aplicativo nele.
- **DESKTOP_WINDOWING:** múltiplas janelas Android livres; não decorre da existência de um display virtual.

## Direção arquitetural aceita no escopo limitado

Ficam aceitos o núcleo de sessão e CLI em Rust, a interface Linux futura em Qt 6/QML e ADB/scrcpy como ferramentas externas em subprocessos supervisionados. A interface e o núcleo são componentes/processos conceitualmente separados; o mecanismo concreto de integração entre eles não foi escolhido. O Limivra só encerrará processos que pertencem à sua própria sessão.

Essa separação conceitual mantém as regras de sessão fora da interface. Qualquer integração futura terá de definir fronteiras, erros e encerramento, mas este registro não escolhe IPC, FFI ou outro mecanismo. Embedding de vídeo em Qt e protocolo próprio de streaming não foram aprovados.

~~~text
[Qt 6/QML — processo futuro da interface]
                 [mecanismo de integração: não decidido]
[Rust Session Core + CLI — processo do núcleo]
                 ├── subprocesso externo supervisionado: ADB
                 └── subprocesso externo supervisionado: scrcpy
~~~

O núcleo só deve supervisionar processos que iniciou e encerrar esses processos de forma idempotente. Não deve usar encerramento global por nome de processo. O servidor ADB e o serviço remoto que scrcpy eventualmente inicia no aparelho exigem tratamento de ciclo de vida próprio; esta proposta não afirma que foram exercitados.

## Limites explícitos da ratificação humana

A aceitação abrange somente a direção de Session Core/CLI em Rust, UI Linux futura em Qt 6/QML, ferramentas ADB/scrcpy externas supervisionadas e encerramento apenas de processos pertencentes ao Limivra. Continuam expressamente pendentes:

- Qualquer contrato concreto de IPC ou integração: formato, transporte, serialização, autenticação, ABI, versionamento e ciclo de vida.
- Embedding de vídeo em Qt.
- Código de produto, scaffold Rust e testes com telefone físico.
- Ratificação integral do PRD ou da especificação.
- Qualquer afirmação de capacidade do Redmi Note 13 4G/HyperOS sem evidência observável.
- Release, distribuição ou aproveitamento de PRs de produto já abertas.

Esta decisão documental não autoriza instalação, acesso ao aparelho, comandos ADB físicos, implementação ou fechamento das Issues #8/#11.

## Evidência observada no host Linux

As verificações abaixo consultaram versões, ajuda dos executáveis e ambiente local. Não iniciaram uma sessão ADB nem contataram um telefone.

| Verificação | Comando | Observado em 2026-10-22 |
|---|---|---|
| Sistema | cat /etc/os-release; uname -srm | Linux Mint 22.3 (Zena), Linux 7.0.0-30-generic, x86_64. |
| Android Platform-Tools | adb --version | Android Debug Bridge 1.0.41; versão 37.0.1-15733141 (Platform-Tools 37.0.1). |
| scrcpy | scrcpy --version | scrcpy 5.0.1. |
| Flags do scrcpy | scrcpy --help \| rg -n -C 3 -- 'new-display\|display-id\|start-app' | A ajuda local mostrou --new-display[=[<width>x<height>][/<dpi>]], --start-app=name e --display-id=id. A opção --display-id seleciona um display do aparelho; sua presença não prova que um display virtual possa ser criado. |
| Qt 6 | command -v pkg-config qmake6 qmake; pkg-config --modversion Qt6Core Qt6Qml | pkg-config disponível; qmake6/qmake não encontrados e os módulos Qt6Core/Qt6Qml não encontrados via pkg-config (exit 1). Nenhuma instalação foi feita. |
| Sessão gráfica | XDG_SESSION_TYPE, DISPLAY, WAYLAND_DISPLAY | XDG_SESSION_TYPE=x11, DISPLAY=:0, WAYLAND_DISPLAY vazio. Sessão Wayland não verificada. |

A execução de adb --version apenas reportou a versão do cliente. Não foi executado adb devices, nem houve conexão USB/Wi-Fi ou comunicação física com o aparelho. A presença das flags na ajuda local comprova disponibilidade do CLI instalado, não conectividade, injeção de input, UI Qt, criação funcional de display virtual ou suporte da ROM. O ambiente observado cobre X11 somente; não fornece evidência sobre Wayland.

## Histórico e inspeção em runtime do scrcpy

A nota oficial de lançamento do [scrcpy v3.0](https://github.com/Genymobile/scrcpy/releases/tag/v3.0) registra a introdução do recurso de display virtual e documenta o uso de --new-display; a mesma versão introduziu iniciar um aplicativo no início da sessão. Portanto, a versão histórica mínima para --new-display é **v3.0**, não v2.2.

No host observado, scrcpy 5.0.1 e sua saída real de --help confirmam as três opções listadas acima. Em qualquer implementação futura, a disponibilidade deve ser inspecionada no executável encontrado em runtime (versão e --help), e o comportamento precisa ser provado separadamente no aparelho selecionado. Versão e texto de ajuda não demonstram que Android ou uma ROM permita completar a operação.

## Comparação qualitativa de alternativas

A comparação abaixo permanece qualitativa e não resulta de benchmark nem de protótipo; a direção Rust + Qt 6/QML foi ratificada somente no escopo acima.

| Alternativa | Vantagens para este problema | Custos e riscos |
|---|---|---|
| **Rust para núcleo/CLI + Qt 6/QML para UI futura** (direção ratificada) | Núcleo independente da UI por direção arquitetural e CLI útil para diagnóstico. A integração entre componentes continua sem mecanismo ou contrato aprovado. | Dois runtimes/ecossistemas, mecanismo de integração ainda não escolhido, publicação de executáveis separados e manutenção coordenada. Empacotamento e integração com Qt permanecem sem prova. |
| **C** | Baixa camada de runtime e ABI C simples para interoperabilidade entre linguagens. | Gestão manual de memória e recursos, maior carga de segurança e manutenção para estados, concorrência e erros de subprocesso; não remove o custo da UI Qt ou do protocolo entre processos. |
| **C++** | Ecossistema nativo de Qt, acesso direto às APIs de interface e ausência de fronteira Rust/C++ se todo o produto usar C++. | Núcleo e UI tendem a compartilhar toolchain e padrões Qt; lógica de sessão fica mais acoplada à plataforma de UI e exige disciplina adicional para testes headless e fronteiras de propriedade. |
| **Go** | Binário de aplicação único por alvo e modelo direto para coordenação concorrente de subprocessos. | UI Qt 6/QML requer avaliar bindings e a ponte C/C++ disponível; cgo e toolchain cruzada acrescentam manutenção e podem reduzir a simplicidade de empacotamento esperada. |
| **Python** | Iteração rápida e bindings Qt disponíveis para construir interfaces. | Requer runtime e ambiente de pacotes ou bundling, com maior variação entre distribuições; tipagem e erros em runtime elevam o trabalho de validação do supervisor de processos. |
| **Qt puro (C++/QML para UI e núcleo)** | Integração mais direta entre QML, widgets/serviços Qt e ciclo de vida da janela; um ecossistema principal e sem FFI Rust. | A lógica de domínio fica ligada ao toolkit e o caminho CLI/headless precisa ser construído deliberadamente; não oferece a separação linguística que motiva o núcleo Rust. |

A ratificação confirma essa direção como escolha arquitetural limitada, não como prova de viabilidade ou benchmark. Mudança de linguagem ou UI exige decisão humana separada e evidência comparável.

## Custos de FFI, dependências e distribuição

### FFI e fronteira entre linguagens

A decisão humana separa UI e núcleo conceitualmente, mas não escolhe FFI, IPC ou outro mecanismo de integração. Se uma proposta futura usar IPC, será necessário decidir formato, transporte, serialização, autenticação, versionamento, ciclo de vida, timeouts, cancelamento, erros e queda de processo. Se usar FFI, os custos abaixo se aplicam; nenhum desses contratos foi ratificado.

Se futuramente for preferida uma biblioteca Rust carregada dentro do processo Qt/C++, haverá custo de uma ABI C estável e versionada, conversão de tipos, propriedade e duração de memória, callbacks e threads, propagação de erros, prevenção de panic através da fronteira e testes nos dois lados. Exceções C++ e tipos de Qt não devem atravessar diretamente uma ABI Rust. A eventual ponte FFI seria uma decisão própria, não parte desta ADR.

### Dependências

O núcleo Rust exigiria toolchain para cada alvo, dependências Rust pinadas e revisão de licenças. Qt 6/QML acrescentaria os módulos selecionados, ferramentas de compilação e runtime Qt. O ADB/Platform-Tools e o scrcpy continuariam executáveis externos com versões e licenças próprias; o núcleo precisaria descobrir sua disponibilidade e reportar ausência ou incompatibilidade sem instalar ferramentas por conta própria.

ADR-0004 resolve MIT para o código original do Limivra, não as obrigações de dependências distribuídas. Seleção de módulos Qt, termos de distribuição, avisos e composição de pacotes precisam de revisão específica antes de distribuição.

### Empacotamento

Um pacote futuro precisaria decidir como entregar ou localizar o núcleo e a UI, módulos/runtime Qt e executáveis ADB/scrcpy, bem como plugins QML, caminhos de dados, atualização e avisos de licença. Pacotes por distribuição, Flatpak e AppImage têm implicações diferentes de dependência e integração com ferramentas externas. O host verificado não tem Qt 6 detectável via pkg-config, portanto não há evidência de build, deploy ou runtime Qt neste trabalho.

### Teste e manutenção

A divisão de processos permite testes unitários do núcleo, executáveis fake para ADB/scrcpy, testes do contrato IPC e testes de falhas de processo sem UI. Em seguida seriam necessários testes da UI em X11 e Wayland separadamente, e testes físicos autorizados para USB, mirror, input e display virtual. Uma verificação só em X11 não pode ser reportada como suporte Wayland.

A manutenção cobrirá Rust, Qt/QML, o mecanismo de integração que vier a ser aprovado e diferenças entre versões empacotadas de ADB/scrcpy. Registrar versões e opções não substitui testes de comportamento. O custo de FFI só será evitado se uma decisão futura mantiver componentes em processos separados; essa decisão de integração ainda está pendente.

## Princípio capability-first e fallback

A detecção deve separar MIRROR, VIRTUAL_DISPLAY e DESKTOP_WINDOWING, usando estes estados:

| Estado | Significado aceito em princípio |
|---|---|
| **SUPPORTED** | Comportamento demonstrado no aparelho e ambiente identificados, com passo reproduzível e evidência observável. |
| **UNSUPPORTED** | Uma prova válida demonstrou que a capacidade não está disponível naquela combinação testada. |
| **UNKNOWN** | **Padrão** quando falta prova empírica suficiente; ausência de teste, versão Android ou flag no host não muda o estado. |
| **BLOCKED** | O teste não pode prosseguir por pré-condição, consentimento, autorização ou ferramenta ausente; não equivale a UNSUPPORTED. |

O fallback de VIRTUAL_DISPLAY para MIRROR nunca é automático. A interface só pode oferecê-lo com explicação clara da limitação, e a sessão mirror só começa depois de o usuário aceitar expressamente. UNKNOWN e BLOCKED também não autorizam uma troca silenciosa de modo.

## Riscos, alternativas não selecionadas e limites

- O contrato IPC, sua política de autenticação local, compatibilidade e encerramento ainda não foram especificados nem aprovados.
- Qt 6/QML não foi encontrado no host por pkg-config; instalação, build, UI, plugins e empacotamento permanecem sem prova.
- Apenas a existência das opções do scrcpy foi observada. USB, autorização ADB, mirror, input, display virtual, inicialização de app e suporte de ROM não foram testados.
- Wayland não foi verificado. A sessão observada foi X11.
- A coordenação de subprocessos tem riscos de timeout, desconexão, saída inesperada e processos órfãos; o servidor ADB pode ter ciclo de vida distinto do cliente iniciado.
- C, C++, Go, Python e Qt puro não foram selecionados para a direção ratificada; qualquer mudança para essas alternativas requer decisão humana separada.
- Rever ou substituir a direção aceita requer nova decisão humana; esta slice altera somente documentação, sem mudar host, dispositivo ou produto.

## Gates sequenciais e decisões humanas pendentes

A progressão continua sujeita aos gates **#8 → #9 → #15 → #19**; esta aceitação limitada não marca issue ou gate como concluído:

1. **#8 — revisão do design:** esta direção limitada foi registrada como ACCEPTED; a ratificação integral do PRD/especificação e a aprovação do plano continuam pendentes, e a Issue #8 permanece aberta.
2. **#9 — SPIKE-001:** requer telefone disponível e consentimento específico para depuração USB e observação. O estado de capacidades permanece UNKNOWN até haver prova; o preflight do host desta ADR não substitui esse experimento.
3. **#15 — estrutura do projeto:** só pode começar após aprovação humana de #8 e da direção de stack da #11, além de plano aprovado. Esta ADR não autoriza scaffold nem implementação.
4. **#19 — interface Qt:** gate posterior dependente também das Issues #16 e #17. Esta ADR não cria código de UI nem escolhe definitivamente Qt.

A Issue #9 atualmente não declara #8 como dependência formal; a sequência acima descreve os checkpoints de progressão, não altera o grafo de dependências publicado. Permanecem pendentes os contratos concretos de integração/IPC, a decisão sobre embedding de vídeo, a validação física do SPIKE-001, a prova de capacidades de ROM e decisões futuras de implementação, empacotamento e distribuição.

## Referências

- docs/architecture/system.md e ADR-0001, ADR-0002, ADR-0003 e ADR-0004.
- docs/product/PRD.md, docs/planning/roadmap.md e docs/testing/strategy.md.
- [scrcpy v3.0 — notas oficiais de lançamento](https://github.com/Genymobile/scrcpy/releases/tag/v3.0).
