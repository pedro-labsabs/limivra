# Pacote de decisão de licenciamento — Limivra (M0)

**Identificador:** `docs/governance/license-decision-packet.md`
**Status:** `PROPOSED / NEEDS_HUMAN_DECISION`
**Data da análise:** 2026-10-18
**Issue relacionada:** [#7](https://github.com/pedro-labsabs/limivra/issues/7) (Refs #7)
**Épico:** [#1 — M0 Foundation & Feasibility](https://github.com/pedro-labsabs/limivra/issues/1)
**Autor técnico:** Agente Executor Técnico (Limivra)
**Decisor / Mantenedor:** Mantenedor Humano / ChatGPT Mantenedor
**Natureza do documento:** Análise técnica e de conformidade de código aberto. **Não constitui parecer jurídico formal ou homologação legal definitiva.**

---

## 1. Contexto, objetivos e limites

### 1.1 Contexto do projeto Limivra
O Limivra é uma solução open source para Linux focada em produtividade em tela grande acionada por celulares Android (com foco inicial em aparelhos sem modo desktop de fábrica, como o Redmi Note 13 4G), sem exigência de root no fluxo base.

A arquitetura proposta ([`docs/architecture/system.md`](../architecture/system.md) e [ADR-0001](../architecture/decisions/ADR-0001-scrcpy-external.md)) prevê a utilização de:
- Interface de usuário em Linux (planejada em Qt 6 / QML);
- Núcleo de orquestração de sessões e probing em Rust (`SessionCore`);
- Descoberta e ponte de comunicação via Android Debug Bridge (`adb`);
- Espelhamento, streaming e injeção de eventos via subprocesso controlado do `scrcpy`.

Conforme estabelecido em [`docs/governance/security-and-legal.md`](security-and-legal.md), **"repositório público não significa licença open source"**. A ausência de uma licença formal impede a colaboração segura e a redistribuição confiável. A seleção da licença do Limivra é um requisito prévio essencial para a maturidade do projeto e para os marcos de empacotamento e distribuição ([#33](https://github.com/pedro-labsabs/limivra/issues/33)).

### 1.2 Objetivo deste pacote
Fornecer ao mantenedor humano e aos revisores do projeto uma base estruturada, reproduzível e fundamentada em fontes oficiais para:
1. Avaliar 3 licenças de código aberto candidatas (**MIT**, **Apache-2.0** e **GPL-3.0-or-later**);
2. Compreender os impactos jurídicos e operacionais da integração com **scrcpy**, **Qt 6**, **ADB/Android Platform-Tools**, o ecossistema **Rust** e formatos futuros de **packaging**;
3. Analisar 3 cenários práticos de distribuição e consumo;
4. Submeter a pergunta de decisão explícita ao mantenedor e traçar o checklist técnico subsequente à escolha.

### 1.3 Limites do executor (governança)
- O executor técnico **NÃO aprova nem homologa licenças**;
- O executor técnico **NÃO cria nem modifica arquivos `LICENSE` ou `NOTICE`** neste slice;
- Nenhuma ADR proposta é convertida em `ACCEPTED` neste slice;
- A stack técnica (Qt/Rust) e as ADRs permanecem como `PROPOSED`, sujeitas à revisão independente em andamento ([#8](https://github.com/pedro-labsabs/limivra/issues/8) / [#35](https://github.com/pedro-labsabs/limivra/pull/35) / [#11](https://github.com/pedro-labsabs/limivra/issues/11)).

---

## 2. Comparação aprofundada das licenças candidatas

Para a escolha da licença de código aberto do Limivra, foram analisadas três das licenças mais amplamente adotadas no ecossistema de software livre e de código aberto (FOSS): **MIT**, **Apache-2.0** e **GPL-3.0-or-later**.

### 2.1 MIT (`MIT`)
- **Identificador SPDX:** `MIT`
- **Classificação:** Permissiva minimalista (OSI Approved, FSF Free Software).
- **Texto e termos fundamentais:** Permite uso, cópia, modificação, mesclagem, publicação, distribuição, sublicenciamento e venda de cópias do software, sem restrições substanciais além da preservação do aviso de copyright e da declaração de isenção de garantia.
- **Permissões e restrições:** Máxima flexibilidade para uso comercial, privado e redistribuição em formatos abertos ou fechados.
- **Obrigações de redistribuição:** O texto da licença e o aviso de direitos autorais devem acompanhar todas as cópias ou partes substanciais do software.
- **Copyleft:** Inexistente. Não exige disponibilização do código-fonte de trabalhos derivados ou combinados.
- **Patentes:** A licença MIT original não contém cláusula expressa de concessão de licença de patentes nem cláusula de retaliação defensiva contra litígios patentários.
- **Compatibilidade com componentes:** É compatível com código Apache-2.0 e GPLv3. Se combinada em um mesmo executável estático ou trabalho conjunto com código GPLv3/LGPLv3, o trabalho derivado conjunto passa a ser regido pelos termos mais restritivos (GPLv3), embora o código original MIT mantenha seus termos na distribuição isolada.
- **Impacto na distribuição Linux:** Facilmente aceita em todas as distribuições Linux (Debian, Fedora, Arch, Alpine, etc.).
- **Impacto em contribuições:** Muito simples para novos contribuidores, mas não oferece proteção contra entidades proprietárias que fechem trabalhos derivados ou reivindiquem patentes posteriormente.
- **Riscos específicos para o Limivra:** Falta de proteção de patente expressa. Se módulos Qt sob GPLv3 entrarem no projeto, o produto final distribuído não poderá ser puramente MIT sem aderir aos deveres do copyleft do Qt.

### 2.2 Apache 2.0 (`Apache-2.0`)
- **Identificador SPDX:** `Apache-2.0`
- **Classificação:** Permissiva moderna com concessão explícita de patentes e proteção defensiva (OSI Approved, FSF Free Software).
- **Texto e termos fundamentais:** Concede direito de reproduzir, preparar trabalhos derivados, exibir e distribuir o software e trabalhos derivados em código-fonte ou binário. Exige preservação de copyright, patente, marcas e notas de atribuição, além de indicar alterações relevantes em arquivos modificados (Seção 4).
- **Permissões e restrições:** Permite uso comercial e integração em projetos proprietários ou livres. Proíbe uso indevido de marcas registradas do projeto (Seção 6).
- **Obrigações de redistribuição:** Obrigatoriedade de inclusão do arquivo de licença, notices existentes em arquivo `NOTICE`, cópia das notificações de copyright e menção a arquivos modificados.
- **Copyleft:** Inexistente. Trabalhos derivados podem ser licenciados sob outros termos (inclusive proprietários), desde que cumpridas as condições de atribuição da Seção 4.
- **Patentes:** Contém concessão expressa de licença de patentes perpétua, mundial e livre de royalties por parte de cada contribuidor (Seção 3), com cláusula de rescisão automática se o receptor ajuizar litígio de violação de patente contra o projeto.
- **Compatibilidade com componentes:** Alinhamento nativo direto com o **scrcpy** (que é licenciado sob Apache-2.0). A FSF reconhece compatibilidade unilateral: código Apache-2.0 pode ser incorporado a um projeto GPLv3 (a GPLv3 absorve o trabalho combinado). Contudo, código puramente GPLv2 (sem "or later") é incompatível com Apache-2.0 (situação irrelevante para o Limivra, que não mira GPLv2).
- **Impacto na distribuição Linux:** Excelente acolhimento nas distribuições Linux e no ecossistema Rust (onde o padrão de facto da comunidade é a licença dual MIT / Apache-2.0).
- **Impacto em contribuições:** Proporciona segurança jurídica robusta contra "patent trolls" e emboscadas de patentes de colaboradores corporativos.
- **Riscos específicos para o Limivra:** Exige manutenção formal e disciplinada do arquivo `NOTICE`. Se o Limivra vier a linkar estaticamente componentes Qt com partes sob GPLv3, o binário final distribuído deverá atender à GPLv3.

### 2.3 GNU General Public License v3.0 or later (`GPL-3.0-or-later`)
- **Identificador SPDX:** `GPL-3.0-or-later`
- **Classificação:** Copyleft forte (Strong Copyleft) com cláusulas anti-tivoização e patentes (OSI Approved, FSF Free Software).
- **Texto e termos fundamentais:** Concede liberdade para executar, estudar, modificar e redistribuir o software. Garante que qualquer trabalho que contenha o programa ou seja dele derivado seja distribuído sob os mesmos termos da GPLv3 (ou versão posterior, se utilizada a cláusula "or-later").
- **Permissões e restrições:** Permite distribuição e uso comercial, mas proíbe terminantemente fechar o código-fonte de versões modificadas ou combinadas distribuídas a terceiros.
- **Obrigações de redistribuição:** Ao distribuir binários, é obrigatório disponibilizar o código-fonte correspondente completo (*Corresponding Source*), incluindo scripts de controle de compilação e instalação.
- **Copyleft:** Forte e recíproco. Impede que terceiros apropriem o código do Limivra em ferramentas proprietárias sem retribuir o código-fonte.
- **Patentes e Anti-tivoização:** Cláusula explícita de licenciamento de patentes com rescisão por litígio (Seção 11) e proibição expressa de restrições de hardware/assinatura digital que impeçam o usuário de executar versões modificadas do software em dispositivos de consumo (*Installation Information* / anti-tivoização, Seção 6).
- **Compatibilidade com componentes:** Altamente compatível com o ecossistema GNU/Linux e componentes Qt licenciados sob LGPLv3 ou GPLv3. Pode incorporar dependências sob MIT e Apache-2.0 sem violação.
- **Impacto na distribuição Linux:** O padrão histórico e nativo de aplicativos de desktop do ecossistema Linux livre (KDE, GNOME, etc.).
- **Impacto em contribuições:** Garante que todas as melhorias e correções feitas pela comunidade ou por fabricantes permaneçam públicas e abertas. Pode desincentivar empresas que desejem embutir a base do Limivra em produtos proprietários fechados.
- **Riscos específicos para o Limivra:** Se no futuro for cogitado um modelo de licenciamento duplo comercial ou reutilização de partes do código em plataformas móveis ou ambientes proprietários restritos, a GPLv3 impõe limites rígidos.

---

## 3. Matriz comparativa das três licenças candidatas

| Critério | MIT | Apache-2.0 | GPL-3.0-or-later |
| :--- | :--- | :--- | :--- |
| **Identificador SPDX** | `MIT` | `Apache-2.0` | `GPL-3.0-or-later` |
| **Tipo de Licença** | Permissiva minimalista | Permissiva estruturada | Copyleft forte |
| **Obrigação de fornecer fonte** | Não | Não | Sim (na distribuição de binários) |
| **Concessão explícita de patentes** | Não (silente) | Sim (expressa, Seção 3) | Sim (expressa, Seção 11) |
| **Retaliação defensiva de patentes** | Não | Sim | Sim |
| **Cláusula anti-tivoização** | Não | Não | Sim (Seção 6) |
| **Preservação de avisos / Notices** | Sim (no cabeçalho/arquivo) | Sim (requer arquivo `NOTICE` se presente) | Sim (avisos de copyright e GPL intactos) |
| **Rastreamento de alterações** | Não exige | Sim (declarar arquivos alterados) | Sim (destacar alterações e datas) |
| **Compatibilidade com scrcpy** | Compatível (scrcpy como processo ou lib) | Compatibilidade direta e idêntica | Compatível (GPLv3 absorve permissivas) |
| **Compatibilidade com Qt 6 (LGPLv3)** | Compatível (com link dinâmico e relink) | Compatível (com link dinâmico e relink) | Compatibilidade total (mesma família v3) |
| **Compatibilidade com Qt 6 (GPLv3)** | Incompatível se linkado estaticamente em binário puramente MIT | Incompatível se linkado em binário puramente Apache-2.0 | Totalmente compatível (unificação sob GPLv3) |
| **Ecossistema Rust** | Padrão dual (`MIT OR Apache-2.0`) | Padrão dual (`MIT OR Apache-2.0`) | Compatível com crates permissivas |
| **Acolhimento em distros Linux** | Universal | Universal | Universal |
| **Apropriação proprietária por terceiros** | Possível (código pode ser fechado) | Possível (código pode ser fechado) | Proibida (obrigação de reciprocidade aberta) |

---

## 4. Matriz de dependências e obrigações jurídicas

O projeto Limivra planeja interagir com múltiplos componentes de naturezas de licenciamento distintas. A tabela a seguir e a análise subsequente detalham os fatos comprovados, distinções técnicas e obrigações decorrentes.

### 4.1 Síntese por componente

| Componente | Papel no Limivra | Licença oficial | Fonte primária verificada | Modalidade de integração | Obrigações principais |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **scrcpy** | Backend de captura e controle | Apache-2.0 | `https://github.com/Genymobile/scrcpy/blob/master/LICENSE` | Subprocesso independente (`ADR-0001`) | Preservar notices; se redistribuído em bundle, incluir licença Apache-2.0 e `NOTICE`. |
| **Qt 6 (Core/GUI/Quick)** | Interface gráfica (planejada) | LGPL-3.0 / GPL-3.0 / Comercial | `https://doc.qt.io/qt-6/licensing.html` e `https://www.qt.io/development/open-source-lgpl-obligations` | Link dinâmico (`shared library`) | Fornecer fonte da biblioteca Qt; permitir relink/substituição pelo usuário; fornecer instruções de instalação; cumprir notices. |
| **Qt 6 (Módulos GPL)** | Ex: Qt Wayland Compositor, Qt Quick 3D | GPL-3.0 / Comercial | `https://doc.qt.io/qt-6/licensing.html` | Não planejado no MVP (`UNKNOWN` / condicional) | Se qualquer módulo GPLv3 for linkado, a aplicação inteira deve ser licenciada sob GPLv3. |
| **ADB / Platform-Tools** | Descoberta e ponte USB/Wi-Fi | Apache-2.0 (código AOSP) / Termos do SDK Google | `https://developer.android.com/studio/terms` e AOSP | Invocação como utilitário externo do host | Termos do SDK proíbem redistribuição de binários do SDK sem permissão (Cláusula 3.4), mas componentes AOSP open source seguem licença própria (Cláusula 3.5). |
| **Rust Crates** | `SessionCore` e adaptadores | Proposta (`UNKNOWN` até criação do `Cargo.toml`) | Comunidade crates.io / `docs/planning/backlog.md` | Compilação estática no binário nativo | Auditar `cargo-deny` / `cargo-license` na criação de [#15](https://github.com/pedro-labsabs/limivra/issues/15); cumprir avisos de licenças MIT/Apache/BSD. |
| **Packaging Linux** | Distribuição em distro, Flatpak ou AppImage | Formatos abertos | `docs/planning/backlog.md` ([#33](https://github.com/pedro-labsabs/limivra/issues/33)) | Pacote declarativo ou bundle conteinerizado | Flatpak/AppImage contendo bibliotecas exigem agregação de licenças e fontes correspondentes (LGPLv3). |

---

### 4.2 Análise pormenorizada por componente

#### A. scrcpy
- **Licença do repositório oficial:** Apache License 2.0.
- **Execução como processo externo (`ADR-0001`):** O Limivra propõe invocar o scrcpy como binário separado (`Process::spawn`), comunicando-se por flags de linha de comando, pipes e controle de ciclo de vida (`PID`). No âmbito jurídico tradicional, processos desacoplados que operam no nível do sistema operacional comunicando-se via IPC padrão não formam um trabalho derivado único restritivo.
- **Distribuição independente vs. conjunta:**
  - *Consumo de ferramenta externa pré-instalada:* O Limivra não embute nem redistribui o binário do scrcpy. Não há obrigação de redistribuir o código do scrcpy, bastando referenciar o software nas atribuições de terceiros.
  - *Redistribuição de pacote conjunto (bundle):* Caso um instalador do Limivra (ex: Flatpak com extensão ou pacote "all-in-one") redistribua o binário compilado do `scrcpy` e seu `scrcpy-server.jar`, devem ser integralmente respeitadas as condições da Seção 4 do Apache-2.0 (cópia da licença, preservação de avisos de copyright e menção ao `NOTICE` original do Genymobile).

#### B. Qt 6
- **Modularidade de licenças:** A The Qt Company adota um modelo de licenciamento misto (multi-licensing):
  - *Módulos Base essenciais (Qt Core, Qt GUI, Qt Widgets, Qt Network, Qt QML, Qt Quick):* Disponíveis sob **LGPLv3**, **GPLv3** e licença comercial.
  - *Módulos e extensões exclusivamente sob GPLv3 (ou comercial):* Conforme documentado oficialmente em `https://doc.qt.io/qt-6/licensing.html`, módulos como `Qt Wayland Compositor`, `Qt Network Authorization`, `Qt Quick 3D`, `Qt Quick 3D Physics`, `Qt Quick Timeline` e ferramentas de compilação QML proprietárias **NÃO estão disponíveis sob LGPLv3**, mas apenas sob **GPLv3** (ou comercial).
  - *Atenção arquitetural crítica:* O Limivra planeja atuar como cliente gráfico em Wayland, consumindo protocolos Wayland existentes, e **não como compositor Wayland**. Portanto, o módulo `Qt Wayland Compositor` não é necessário. O cliente Qt conecta-se ao compositor do sistema via plugin de plataforma `libqwayland` (que acompanha o Qt Base sob LGPLv3).
- **Obrigações da LGPLv3 para a aplicação:**
  - **Dynamic Linking:** O Limivra deve compilar e linkar dinamicamente contra as bibliotecas compartilhadas do Qt (`.so`).
  - **Re-link / Substituição pelo usuário:** O usuário final tem o direito garantido de substituir ou atualizar as bibliotecas Qt no seu ambiente e rodar o aplicativo com essas bibliotecas modificadas.
  - **Informações de Instalação (Anti-tivoização):** Devem ser fornecidas instruções suficientes de como o aplicativo executa com bibliotecas relinkadas.
  - **Código-fonte correspondente:** O distribuidor da aplicação deve fornecer o código-fonte correspondente das bibliotecas Qt utilizadas (ou oferta formal por escrito), mesmo que não tenham sido alteradas.
  - **Avisos de copyright e licença:** A UI do Limivra ou sua documentação deve exibir que o software utiliza bibliotecas Qt sob LGPLv3 e fornecer link para o texto da licença.
- **Impacto se a licença do Limivra for permissiva (MIT ou Apache-2.0):** É juridicamente viável manter o código do Limivra sob MIT ou Apache-2.0 e linkar dinamicamente ao Qt sob LGPLv3, desde que atendidos todos os requisitos de relink, dynamic linking e entrega do fonte do Qt. Se a licença do Limivra for GPL-3.0-or-later, a conformidade é facilitada porque todo o conjunto opera na mesma filosofia copyleft.

#### C. ADB (Android Debug Bridge) e Android Platform-Tools
- **Distinção fundamental entre código AOSP e SDK Binário do Google:**
  - O código-fonte do `adb` faz parte do *Android Open Source Project* (AOSP) e é mantido majoritariamente sob a licença **Apache License 2.0**. Distribuições Linux (Debian, Ubuntu, Fedora, Arch) compilam seus próprios pacotes `android-tools` ou `adb` a partir do código aberto do AOSP.
  - Os binários pré-compilados do `platform-tools` baixados diretamente do site do Google (`developer.android.com`) estão sujeitos aos **Termos e Condições do Android SDK** (`Android Software Development Kit License Agreement`).
- **Restrições dos Termos do Android SDK Google:**
  - Cláusula 3.4: *"Except to the extent required by applicable third party licenses, you may not copy (except for backup purposes), modify, adapt, redistribute, decompile, reverse engineer, disassemble, or create derivative works of the SDK or any part of the SDK."*
  - Cláusula 3.5: *"Use, reproduction and distribution of components of the SDK licensed under an open source software license are governed solely by the terms of that open source software license and not the License Agreement."*
- **Conclusão para o Limivra:**
  - O Limivra **NÃO deve empacotar nem redistribuir os binários proprietários do Google SDK Platform-Tools**.
  - O Limivra deve se apoiar no ADB fornecido pelo sistema operacional do usuário (pacote da distribuição Linux, derivado do AOSP sob Apache-2.0) ou instruir o download direto pelo usuário.
  - Invocação do `adb` como subprocesso externo não gera vinculação derivada no Limivra.

#### D. Ecossistema Rust (`crates.io`)
- Como a stack e o projeto Rust ainda não foram implementados (não há `Cargo.toml`), **nenhuma dependência crate está fixada**.
- Quando o manifesto for criado na issue [#15](https://github.com/pedro-labsabs/limivra/issues/15), o mantenedor e os executores deverão:
  - Adotar uma ferramenta de auditoria de licenças em CI (ex: `cargo-deny` ou `cargo-license`);
  - Rejeitar crates com licenças ambíguas, ausentes ou com termos copyleft conflitantes com a licença escolhida para o Limivra;
  - Lembrar que, no Rust, as dependências são tipicamente compiladas de forma **estática** no binário nativo. Portanto, qualquer dependência incorporada influencia diretamente as obrigações de licença do binário final.

---

## 5. Análise comparativa dos cenários de distribuição

As obrigações legais variam radicalmente conforme o modelo de entrega do software. Foram modelados 3 cenários de referência:

```text
+---------------------------------------------------------------------------------------+
| CENÁRIO A: Consumo de Subprocessos Locais (Baseline M0 / M1)                          |
| [Limivra Binary (Rust + Qt dinâmico)] ---> executa subprocessos locais independentes  |
|                                       ---> adb (instalado pelo usuário)               |
|                                       ---> scrcpy (instalado pelo usuário)            |
+---------------------------------------------------------------------------------------+
| CENÁRIO B: Pacote de Distribuição Linux Nativa (ex: .deb / .rpm / Arch pkg)           |
| [Metadados do Pacote Limivra] ---> Dependências declaradas: 'scrcpy', 'android-tools' |
|                               ---> Bibliotecas do sistema: 'libQt6Core.so', etc.      |
+---------------------------------------------------------------------------------------+
| CENÁRIO C: Bundle Conteinerizado / Autossuficiente (ex: Flatpak / AppImage)           |
| [Container / Runtime Limivra] ---> Contém binário do Limivra                          |
|                               ---> Contém bibliotecas Qt compartilhadas (.so)         |
|                               ---> Pode conter scrcpy-server e binários auxiliares    |
+---------------------------------------------------------------------------------------+
```

### 5.1 Cenário A — Subprocessos locais do usuário (Baseline do Limivra)
- **Topologia:** O usuário instala o `scrcpy` e o `adb` pelos meios próprios do seu sistema operacional. O Limivra executa no host e apenas invoca `adb` e `scrcpy` via linha de comando (`fork/exec`).
- **Obrigações conhecidas:**
  - O Limivra não redistribui os binários de terceiros;
  - Obrigações limitadas ao binário do Limivra e às suas bibliotecas dinâmicas (Qt e Rust);
  - Exigência de exibir avisos de copyright do Limivra e reconhecimentos de terceiros (Third-Party Notices).
- **Complexidade jurídica:** Mínima. Isolamento de processos garante máxima segurança de licença.

### 5.2 Cenário B — Pacote de distribuição Linux nativa (Debian, Fedora, Arch)
- **Topologia:** O pacote `.deb`, `.rpm` ou PKGBUILD compila o Limivra e declara em seus metadados (`Depends:`) as dependências `scrcpy`, `android-tools` e os pacotes de runtime do `qt6-base`.
- **Obrigações conhecidas:**
  - A distribuição Linux é responsável por gerenciar e empacotar o código-fonte correspondente das bibliotecas Qt e do adb/scrcpy;
  - O mantenedor do Limivra só precisa fornecer o código-fonte limpo do Limivra com a licença declarada (`LICENSE`) e metadados SPDX nos manifestos de empacotamento;
  - A substituição de bibliotecas Qt é nativa pelo gestor de pacotes da distribuição (`apt`, `dnf`, `pacman`).
- **Complexidade jurídica:** Baixa a moderada. Segue o padrão estabelecido da comunidade Linux.

### 5.3 Cenário C — Bundle autocontido / conteinerizado (Flatpak, AppImage)
- **Topologia:** O pacote distribui uma sandbox com todas as bibliotecas necessárias para executar em qualquer distribuição, incluindo Qt 6, runtime C/C++, e eventualmente ferramentas de captura.
- **Obrigações conhecidas:**
  - **Atenção crítica à LGPLv3 do Qt:** Como as bibliotecas do Qt estão empacotadas no bundle/runtime, o fornecedor do pacote DEVE permitir a substituição dessas bibliotecas pelo usuário ou prover mecanismo para re-execução (no Flatpak, isso é resolvido usando o KDE/GNOME Application Runtime padrão sob LGPLv3 oficial; no AppImage, o usuário deve ser capaz de descompactar o AppImage, substituir os `.so` e remontar);
  - Obrigação de fornecer o código-fonte correspondente completo de todas as bibliotecas LGPLv3 contidas no pacote;
  - Inclusão de um diretório consolidado de licenças e `NOTICE` para todos os componentes bundled.
- **Complexidade jurídica:** Alta. Exige auditoria rigorosa de cada arquivo binário embutido na imagem final.

---

## 6. Riscos residuais e questões não respondidas

1. **Risco de contaminação por módulos proprietários/GPL do Qt:**
   Se no desenvolvimento futuro alguém importar inadvertidamente um módulo Qt que não seja LGPLv3 (como o `Qt Wayland Compositor` ou bibliotecas 3D específicas), uma licença permissiva (MIT ou Apache-2.0) no Limivra entraria em conflito direto na distribuição binária.
   *Mitigação:* CI determinístico de auditoria de dependências Qt antes do merge de qualquer código UI.

2. **Propriedade intelectual e ausência de CLA/DCO:**
   Atualmente, o projeto Limivra não possui Contributor License Agreement (CLA) nem Developer Certificate of Origin (DCO / `Signed-off-by`).
   *Mitigação:* Recomenda-se a adoção de DCO nas diretrizes de contribuição ([`CONTRIBUTING.md`](../../CONTRIBUTING.md)) no momento em que a licença for oficializada.

3. **Incerteza sobre o Companion APK Android (Futuro condicional):**
   Se em fases posteriores (ex: M3) for necessário desenvolver um pequeno APK de apoio para rodar no Android sem root, esse APK será compilado contra o SDK do Android. O código desse app pode ser Apache-2.0 ou GPLv3, mas exigirá atenção específica às restrições de publicação da Google Play Store (caso venha a ser distribuído por lá).

---

## 7. Recomendação técnica não vinculante

> [!NOTE]
> Esta recomendação é puramente técnica, de caráter não vinculante, formulada com base no estado da arte da engenharia de software aberto e na arquitetura proposta para o Limivra. A prerrogativa e autoridade final de seleção cabem exclusivamente ao mantenedor humano.

### Recomendação: **`GPL-3.0-or-later`** ou **`Apache-2.0`**

1. **Se o objetivo primário for proteção do bem comum e fortalecimento do ecossistema Linux livre:**
   A opção recomendada é a **`GPL-3.0-or-later`**.
   *Justificativa:* O Limivra é uma ferramenta de desktop Linux projetada para libertar aparelhos Android de limitações artificiais de fabricantes de hardware. A GPLv3 impede que fabricantes ou agentes comerciais incorporem o código do Limivra em ferramentas proprietárias sem retribuir as melhorias à comunidade; resolve nativamente a conformidade com o ecossistema Qt (mesma versão e família v3); e oferece proteção sólida contra litígios de patentes e tivoização de hardware.

2. **Se o objetivo for paridade ecossistêmica direta com o scrcpy e o ecossistema Rust:**
   A opção recomendada é a **`Apache-2.0`** (ou dual license `MIT OR Apache-2.0` para bibliotecas Rust do núcleo).
   *Justificativa:* Compartilha a mesma licença do `scrcpy` e do código aberto do AOSP/ADB; conta com concessão e defesa explícita de patentes; permite flexibilidade para adoção comercial ampla. Exige, no entanto, rigor na preservação de link dinâmico com o Qt LGPLv3 para evitar atritos de copyleft.

A licença **MIT pura não é recomendada como opção preferencial** para o produto Limivra completo devido à ausência total de salvaguardas de patentes e desproteção contra fechamento de código por terceiros em um projeto que envolve fortes interfaces de hardware e sistemas operacionais.

---

## 8. Pergunta de decisão explícita ao mantenedor

Para concluir a governança da issue [#7](https://github.com/pedro-labsabs/limivra/issues/7), o mantenedor humano deve responder às seguintes perguntas:

> **Pergunta 1:** Qual das licenças candidatas deve ser a licença oficial do projeto Limivra?
> - **Opção 1:** `GPL-3.0-or-later` (Copyleft forte, proteção comunitária, conformidade direta com Qt)
> - **Opção 2:** `Apache-2.0` (Permissiva com proteção de patente, alinhamento com scrcpy/AOSP)
> - **Opção 3:** `MIT` (Permissiva minimalista clássica)
> - **Opção 4:** Modelo misto (ex: `Apache-2.0` para o núcleo Rust e `GPL-3.0-or-later` para o frontend/aplicação final)

> **Pergunta 2:** Qual é o nome ou entidade de copyright que deve constar no cabeçalho do arquivo de licença (ex: `Copyright (c) 2026 Pedro Goulart Espinola and Limivra Contributors`)?

> **Pergunta 3:** Fica autorizada a exigência de DCO (Developer Certificate of Origin com `git commit -s`) para futuras contribuições externas?

---

## 9. Checklist de ações posteriores à aprovação humana

Estas etapas **NÃO fazem parte deste slice documental** e devem ser executadas em uma nova branch/PR dedicada após a decisão humana registrada:

- [ ] **1. Criação do arquivo `LICENSE`:** Adicionar o texto oficial integral da licença aprovada na raiz do repositório.
- [ ] **2. Criação do arquivo `NOTICE`:** Se a licença selecionada for Apache-2.0 (ou se mantido arquivo de atribuições para GPLv3), documentar direitos autorais e créditos ao scrcpy, Qt e Android Open Source Project.
- [ ] **3. Registro formal em ADR:** Elaborar e submeter a `ADR-0004-project-license.md` registrando a motivação, alternativas rejeitadas e consequências da escolha aprovada.
- [ ] **4. Atualização de documentação:**
  - Atualizar [`README.md`](../../README.md) informando a licença SPDX oficial;
  - Atualizar [`docs/governance/security-and-legal.md`](security-and-legal.md);
  - Atualizar [`CONTRIBUTING.md`](../../CONTRIBUTING.md) com as regras de submissão e DCO.
- [ ] **5. Configuração de checagens em CI:** Adicionar checagem automatizada de cabeçalhos de licença SPDX e regras no `cargo-deny` quando o workspace de código for estruturado ([#15](https://github.com/pedro-labsabs/limivra/issues/15)).
- [ ] **6. Fechamento da issue #7:** Somente após a revisão humana e aprovação formal da PR decorrente destas ações.

---

## 10. Fontes primárias consultadas

1. **scrcpy Repository License (Apache-2.0):**
   - URL: `https://github.com/Genymobile/scrcpy/blob/master/LICENSE`
   - Contexto: Licença oficial do projeto mantido pela Genymobile.
   - Consulta: 2026-10-18 (Status HTTP 200).
2. **Qt 6 Licensing Overview:**
   - URL: `https://doc.qt.io/qt-6/licensing.html`
   - Contexto: Relação oficial de módulos sob LGPLv3, GPLv3 e comercial no Qt 6.
   - Consulta: 2026-10-18 (Status HTTP 200).
3. **Qt Open Source LGPL Obligations:**
   - URL: `https://www.qt.io/development/open-source-lgpl-obligations`
   - Contexto: Requisitos de relink, dynamic linking, anti-tivoização e entrega de código-fonte.
   - Consulta: 2026-10-18 (Status HTTP 200).
4. **Android Software Development Kit License Agreement:**
   - URL: `https://developer.android.com/studio/terms`
   - Contexto: Termos oficiais de uso e redistribuição do SDK do Google (Cláusulas 3.4 e 3.5).
   - Consulta: 2026-10-18 (Status HTTP 200).
5. **SPDX License List:**
   - URL: `https://spdx.org/licenses/`
   - Contexto: Registros canônicos SPDX para `MIT`, `Apache-2.0` e `GPL-3.0-or-later`.
   - Consulta: 2026-10-18 (Status HTTP 200).
6. **GNU Operating System / FSF Licenses:**
   - URL: `https://www.gnu.org/licenses/`
   - Contexto: Textos e compatibilidade das licenças GPLv3 e LGPLv3.
   - Consulta: 2026-10-18 (Status HTTP 200).
