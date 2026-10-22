# ADR-0005 — Núcleo Rust/CLI e interface futura Qt 6/QML

**Status:** PROPOSED — NEEDS_HUMAN_APPROVAL
**Data:** 2026-10-22
**Issue:** [#11 — Viabilidade Qt/QML + Rust + scrcpy no Linux](https://github.com/pedro-labsabs/limivra/issues/11)

## Contexto e problema técnico

A Issue #11 pede uma comparação de custos e dependências antes de qualquer estruturação do produto. O Limivra precisa separar a coordenação de sessões Android da interface Linux, conservar o espelhamento como modo básico e tratar display virtual e janelas livres como capacidades distintas e condicionais. A stack em docs/architecture/system.md ainda é proposta; esta ADR não a ratifica nem libera a Issue #15.

A decisão em avaliação é se faz sentido manter um núcleo de sessão em Rust, testável e operável por CLI, com uma interface Qt 6/QML futura, enquanto ADB e scrcpy permanecem ferramentas externas. A alternativa é concentrar núcleo e interface em outra linguagem ou no próprio Qt. Nenhuma alternativa foi implementada ou medida por esta ADR.

A proposta preserva três modos com evidências separadas:

- **MIRROR:** captura e controle do display físico.
- **VIRTUAL_DISPLAY:** criação de display secundário e abertura de aplicativo nele.
- **DESKTOP_WINDOWING:** múltiplas janelas Android livres; não decorre da existência de um display virtual.

## Direção proposta para revisão humana

Manter como candidata uma arquitetura com o núcleo de sessão e sua CLI em Rust, executados em processo próprio, e uma futura interface Qt 6/QML em processo separado. A UI conversaria com o núcleo por um canal IPC local versionado, ainda sem formato ou contrato aprovado. O núcleo chamaria os executáveis ADB e scrcpy como subprocessos externos.

Essa divisão permite testar o núcleo sem interface gráfica e manter Qt fora das regras de sessão. Em contrapartida, introduz um protocolo IPC, tratamento de encerramento entre processos e empacotamento de mais de um executável. A proposta não inclui integração de vídeo dentro de Qt nem protocolo próprio de streaming; isso exigiria uma decisão e evidência separadas.

~~~text
[Qt 6/QML — processo futuro da interface]
                 ⇅ IPC local versionado (contrato pendente)
[Rust SessionCore + CLI — processo do núcleo]
                 ├── subprocesso externo: cliente ADB
                 └── subprocesso externo: scrcpy
~~~

O núcleo só deve supervisionar processos que iniciou e encerrar esses processos de forma idempotente. Não deve usar encerramento global por nome de processo. O servidor ADB e o serviço remoto que scrcpy eventualmente inicia no aparelho exigem tratamento de ciclo de vida próprio; esta proposta não afirma que foram exercitados.

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

A avaliação abaixo é uma hipótese arquitetural para discussão, não resultado de benchmark nem de protótipo.

| Alternativa | Vantagens para este problema | Custos e riscos |
|---|---|---|
| **Rust para núcleo/CLI + Qt 6/QML para UI futura** (direção proposta) | Núcleo independente de UI, CLI útil para diagnósticos, testes sem janela e controle explícito de subprocessos. Permite evoluir a interface sem embutir regras de sessão no toolkit. | Dois runtimes/ecossistemas, canal IPC a especificar, publicação de executáveis separados e manutenção coordenada. A integração com Qt deve ser medida antes de ratificar. |
| **C** | Baixa camada de runtime e ABI C simples para interoperabilidade entre linguagens. | Gestão manual de memória e recursos, maior carga de segurança e manutenção para estados, concorrência e erros de subprocesso; não remove o custo da UI Qt ou do protocolo entre processos. |
| **C++** | Ecossistema nativo de Qt, acesso direto às APIs de interface e ausência de fronteira Rust/C++ se todo o produto usar C++. | Núcleo e UI tendem a compartilhar toolchain e padrões Qt; lógica de sessão fica mais acoplada à plataforma de UI e exige disciplina adicional para testes headless e fronteiras de propriedade. |
| **Go** | Binário de aplicação único por alvo e modelo direto para coordenação concorrente de subprocessos. | UI Qt 6/QML requer avaliar bindings e a ponte C/C++ disponível; cgo e toolchain cruzada acrescentam manutenção e podem reduzir a simplicidade de empacotamento esperada. |
| **Python** | Iteração rápida e bindings Qt disponíveis para construir interfaces. | Requer runtime e ambiente de pacotes ou bundling, com maior variação entre distribuições; tipagem e erros em runtime elevam o trabalho de validação do supervisor de processos. |
| **Qt puro (C++/QML para UI e núcleo)** | Integração mais direta entre QML, widgets/serviços Qt e ciclo de vida da janela; um ecossistema principal e sem FFI Rust. | A lógica de domínio fica ligada ao toolkit e o caminho CLI/headless precisa ser construído deliberadamente; não oferece a separação linguística que motiva o núcleo Rust. |

A escolha proposta não afirma que Rust seja requisito do produto nem que Qt 6/QML seja a única UI viável. Uma decisão diferente é aceitável se um protótipo comparável demonstrar menor custo total de manutenção, teste e distribuição.

## Custos de FFI, dependências e distribuição

### FFI e fronteira entre linguagens

A direção proposta evita FFI obrigatório ao manter UI e núcleo em processos separados. Isso transfere o custo para IPC: definir mensagens e versões compatíveis, autenticação local apropriada, timeouts, cancelamento, erros, encerramento e comportamento quando um dos processos cai.

Se futuramente for preferida uma biblioteca Rust carregada dentro do processo Qt/C++, haverá custo de uma ABI C estável e versionada, conversão de tipos, propriedade e duração de memória, callbacks e threads, propagação de erros, prevenção de panic através da fronteira e testes nos dois lados. Exceções C++ e tipos de Qt não devem atravessar diretamente uma ABI Rust. A eventual ponte FFI seria uma decisão própria, não parte desta ADR.

### Dependências

O núcleo Rust exigiria toolchain para cada alvo, dependências Rust pinadas e revisão de licenças. Qt 6/QML acrescentaria os módulos selecionados, ferramentas de compilação e runtime Qt. O ADB/Platform-Tools e o scrcpy continuariam executáveis externos com versões e licenças próprias; o núcleo precisaria descobrir sua disponibilidade e reportar ausência ou incompatibilidade sem instalar ferramentas por conta própria.

ADR-0004 resolve MIT para o código original do Limivra, não as obrigações de dependências distribuídas. Seleção de módulos Qt, termos de distribuição, avisos e composição de pacotes precisam de revisão específica antes de distribuição.

### Empacotamento

Um pacote futuro precisaria decidir como entregar ou localizar o núcleo e a UI, módulos/runtime Qt e executáveis ADB/scrcpy, bem como plugins QML, caminhos de dados, atualização e avisos de licença. Pacotes por distribuição, Flatpak e AppImage têm implicações diferentes de dependência e integração com ferramentas externas. O host verificado não tem Qt 6 detectável via pkg-config, portanto não há evidência de build, deploy ou runtime Qt neste trabalho.

### Teste e manutenção

A divisão de processos permite testes unitários do núcleo, executáveis fake para ADB/scrcpy, testes do contrato IPC e testes de falhas de processo sem UI. Em seguida seriam necessários testes da UI em X11 e Wayland separadamente, e testes físicos autorizados para USB, mirror, input e display virtual. Uma verificação só em X11 não pode ser reportada como suporte Wayland.

A manutenção passa a cobrir Rust, Qt/QML, o protocolo IPC e diferenças entre versões empacotadas de ADB/scrcpy. Fixar versões e registrar a saída de --version/--help reduz ambiguidades, mas não substitui testes de comportamento. O custo de FFI descrito acima desaparece somente enquanto a fronteira continuar entre processos.

## Princípio capability-first e fallback

A detecção deve separar MIRROR, VIRTUAL_DISPLAY e DESKTOP_WINDOWING, usando estes estados:

| Estado | Significado proposto |
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
- C, C++, Go, Python e Qt puro não foram selecionados para a direção proposta, mas permanecem alternativas abertas se protótipos e empacotamento evidenciarem menor custo global.
- Rollback desta proposta é rejeitar ou substituir esta ADR antes de aprovar a arquitetura; ela não altera host, dispositivo nem produto.

## Gates sequenciais e decisões humanas pendentes

A progressão proposta é **#8 → #9 → #15 → #19**, sem que esta ADR marque qualquer gate como concluído:

1. **#8 — revisão do design:** decisão humana sobre PRD/arquitetura e plano. Esta ADR é apenas subsídio; seu status continua PROPOSED — NEEDS_HUMAN_APPROVAL.
2. **#9 — SPIKE-001:** requer telefone disponível e consentimento específico para depuração USB e observação. O estado de capacidades permanece UNKNOWN até haver prova; o preflight do host desta ADR não substitui esse experimento.
3. **#15 — estrutura do projeto:** só pode começar após aprovação humana de #8 e da direção de stack da #11, além de plano aprovado. Esta ADR não autoriza scaffold nem implementação.
4. **#19 — interface Qt:** gate posterior dependente também das Issues #16 e #17. Esta ADR não cria código de UI nem escolhe definitivamente Qt.

A Issue #9 atualmente não declara #8 como dependência formal; a sequência acima descreve os checkpoints de progressão, não altera o grafo de dependências publicado. Permanecem pendentes a aprovação humana desta ADR, a validação física do SPIKE-001 e as decisões de IPC, Qt, empacotamento e licenças de dependências.

## Referências

- docs/architecture/system.md e ADR-0001, ADR-0002, ADR-0003 e ADR-0004.
- docs/product/PRD.md, docs/planning/roadmap.md e docs/testing/strategy.md.
- [scrcpy v3.0 — notas oficiais de lançamento](https://github.com/Genymobile/scrcpy/releases/tag/v3.0).
