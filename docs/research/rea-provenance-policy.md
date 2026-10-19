# Política de Proveniência, Limites Legais e Clean-Room na Pesquisa REA

**Status:** PROPOSED / NEEDS_HUMAN_REVIEW
**Data:** 19/10/2026
**Contexto:** Épico #2 (Pesquisa REA e interoperabilidade Android), Milestone M0, Issue #12.
**Alinhamento normativo:** [AGENTS.md](../../AGENTS.md), [README.md](../../README.md), [PRD.md](../product/PRD.md), [docs/research/rea-plan.md](rea-plan.md), [docs/governance/security-and-legal.md](../governance/security-and-legal.md), [ADR-0001](../architecture/decisions/ADR-0001-scrcpy-external.md), [ADR-0002](../architecture/decisions/ADR-0002-capability-first.md), [ADR-0003](../architecture/decisions/ADR-0003-no-root.md), [ADR-0004](../architecture/decisions/ADR-0004-project-license.md).

---

## 1. Finalidade e Princípios Gerais

Esta política estabelece os critérios operacionais, éticos, técnicos e legais para orientar qualquer atividade de pesquisa de interoperabilidade e engenharia reversa no ecossistema do Limivra, com foco especial no uso de ferramental REA ([morluto/rea](https://github.com/morluto/rea)).

### 1.1 Premissas e Limites Legais
1. **Natureza estrita de interoperabilidade:** Qualquer atividade investigativa possui finalidade exclusiva de viabilizar a interoperabilidade do Linux com funcionalidades de exibição, controle e entrada do Android, conforme exceções e limites legais aplicáveis em cada jurisdição relevante.
2. **Ausência de consultoria jurídica:** Este documento constitui governança operacional técnica interna e **não** substitui análise jurídica qualificada nem concede autorização legal genérica. Interpretações contratuais (EULAs, ToS) e legais dependem de contexto territorial específico e avaliação humana qualificada.
3. **Mera posse não confere autorização:** A disponibilidade técnica, posse de dispositivo, download público ou extração de binário/APK **não** autorizam automaticamente decompilação, desmontagem, análise estática/dinâmica ou evasão de proteções.
4. **Sem concessão implícita ou automática:** Nenhuma investigação sobre software proprietário, binários de terceiros ou protocolos fechados pode ocorrer sem aprovação humana expressa e registrada.
5. **Independência do MVP de MIRROR:** A pesquisa REA é uma trilha paralela e exploratória. O MVP de espelhamento básico (Épico #3 / Milestone M1, contendo as issues #15 a #20) independe de qualquer resultado ou execução de engenharia reversa de componentes proprietários (baseia-se em APIs públicas AOSP e `scrcpy`, conforme [ADR-0001](../architecture/decisions/ADR-0001-scrcpy-external.md)). O Épico #1 refere-se à governança e arquitetura do Milestone M0.
6. **Não aprovação de arquitetura ou capacidades:** A presente proposta documental não homologa stacks de implementação (ex.: Qt/Rust), não comprova suporte de ROMs específicas (HyperOS, One UI, AOSP puro) e não autoriza transição para marcos subsequentes (M1/#15).

---

## 2. Classificação de Fontes e Níveis de Autorização

Toda fonte de informação, ferramenta ou artefato consultado na pesquisa do Limivra deve ser rigorosamente enquadrado em uma das categorias abaixo:

| Categoria | Definição e Exemplos | Nível de Autorização Exigido | Ações Permitidas |
| :--- | :--- | :--- | :--- |
| **CAT-A: Documentação Pública e Aberta** | Especificações oficiais do Android Open Source Project (AOSP), documentação de desenvolvedor Android (`developer.android.com`), documentação de protocolos públicos (Wayland, VNC, RFCs), manuais públicos do Linux. | **Permissão Padrão (Sem aprovação prévia)** | Leitura, citação com link/data, reprodução conceitual de contratos públicos de API. |
| **CAT-B: Código Aberto com Licença FOSS Conhecida** | Código-fonte sob licenças abertas compatíveis (ex.: `scrcpy` sob Apache-2.0, utilitários AOSP sob Apache-2.0/GPLv2, `rea` sob licença aberta pública). | **Permissão Condicionada à Licença** | Estudo de implementação pública, conformidade estrita com termos de licença e atribuição (manutenção de avisos de copyright). |
| **CAT-C: Artefato Proprietário de Terceiros (Acesso Legítimo / Comercial)** | APKs, binários de sistema, serviços OEM (ex.: Samsung DeX, Motorola Ready For, Xiaomi Desktop), obtidos em dispositivos físicos ou imagens de fábrica de forma legítima, porém sujeitos a EULA/ToS restritivos. | **Exige Revisão Formal e Autorização Humana Explícita (`NEEDS_APPROVAL`)** | Proibido acesso ou análise até submissão de plano e aprovação humana. Análise restrita a superfícies de rede/ADB observáveis externamente se autorizada; decompilação apenas com parecer legal afirmativo. |
| **CAT-D: Artefato com Restrições de Proteção Tecnológica (TPM / DRM / FLAG_SECURE)** | Componentes protegidos por DRM (Widevine, PlayReady), sinalizadores `FLAG_SECURE`, keystores de hardware, contramedidas anti-tamper, mecanismos de integridade/atestação (Play Integrity, Knox). | **PROIBIDO (`STOP`)** | Nenhuma tentativa de contorno, bypass, quebra de criptografia, hooking evasivo ou exploração de vulnerabilidade é permitida. |
| **CAT-E: Origem, Licença ou Proveniência Desconhecida / Duvidosa** | Dumps vazados, APKs baixados de repositórios de pirataria, bibliotecas sem autor identificado, materiais sob sigilo ou obtidos sem cadeia de custódia verificável. | **PROIBIDO (`STOP`)** | Proibido download, armazenamento, inspeção ou referência no repositório. Rejeição imediata. |

---

## 3. Registro Privado de Proveniência e Cadeia de Custódia

Para qualquer análise autorizada sobre artefatos de terceiros (Categorias B e C), é obrigatório estabelecer uma cadeia de custódia auditável mantida exclusivamente em ambiente privado e controlado.

### 3.1 Informações Mínimas Obrigatórias
Antes de qualquer manipulação de artefato, o pesquisador deve registrar privadamente:
- **Identificação do artefato:** Nome exato do arquivo, versão declarada, build ID, target SDK.
- **Titular de direitos:** Entidade jurídica ou autor detentor dos direitos autorais conhecidos.
- **Modo e data de aquisição:** Data precisa (ISO 8601), método lícito de obtenção (ex.: extração via `adb pull` de partição pública de sistema em aparelho próprio homologado, ou download de release oficial pública).
- **Termos e licenças associados:** Termos de serviço, licença de software ou EULA aplicável ao canal de distribuição.
- **Checksum criptográfico:** SHA-256 do arquivo original inalterado.
- **Finalidade restrita:** Declaração explícita do objetivo de interoperabilidade que motiva a inspeção.
- **Pessoa responsável e autorizador:** Identificação do pesquisador executor e do mantenedor que aprovou a análise.

### 3.2 Higiene de Repositório e Proteção contra Vazamento (Anti-Leak)
- **Zero artefato sensível no Git:** É expressamente proibido fazer commit, criar PR ou armazenar no GitHub quaisquer binários, APKs, odex, classes `.dex`, dumps de memória, dumps de ROM, arquivos de configuração proprietários ou capturas de tráfego bruto contendo tokens.
- **Ocultação de hashes de material proprietário não público:** Hashes SHA-256 e nomes de pacotes internos não documentados publicamente não devem ser publicados no repositório para evitar infração ou rastreamento de vazamentos. O registro é mantido em planilha/arquivo de custódia interno privado.
- **Sanitização de logs e relatórios:** Todo relatório público de findings deve passar por triagem prévia para remoção de caminhos locais de arquivos, identificadores de hardware (IMEI, números de série, endereços MAC reais), chaves privadas, cookies e assinaturas binárias proprietárias.
- **Custódia, Acesso e Retenção Condicionada:** O mantenedor do projeto é o responsável primário pela custódia do registro privado e pelo controle estrito de acesso. Cópias de trabalho e metadados de artefatos de pesquisa autorizados devem ser armazenados exclusivamente em diretório local efêmero, isolado fora da árvore do repositório (`.git`). O acesso é restrito ao pesquisador designado e revogável a qualquer momento por determinação do mantenedor. O período de retenção é rigorosamente proporcional à finalidade demonstrada da pesquisa e às obrigações legais aplicáveis, sendo reavaliado formalmente ao término da elaboração da especificação comportamental (sem adoção de prazo universal arbitrário).
- **Gate de Aprovação para Descarte e Exclusão Destrutiva:** Nenhuma exclusão definitiva, descarte destrutivo de ambiente ou eliminação de registros de custódia pode ocorrer de forma silenciosa ou automática por script/agente. Todo descarte exige registro prévio de aprovação humana expressa do mantenedor, garantindo a preservação auditável de evidências de proveniência e cadeia de custódia pelo período legalmente exigido.
- **Triagem, Contenção Segura e Resposta a Incidentes (PII / Segredos):** Caso dados pessoais identificáveis (PII), credenciais ou segredos sejam capturados incidentalmente durante testes autorizados, aplica-se protocolo estrito de contenção imediata: proibição de publicação no repositório, proibição de transmissão externa e proibição de duplicação desnecessária. Os dados afetados devem ser imediatamente isolados em ambiente local protegido. O incidente e o escopo do material capturado devem ser reportados formalmente ao mantenedor para deliberação de descarte seguro, em conformidade com as legislações de proteção de dados e diretrizes de privacidade do projeto.

---

## 4. Metodologia de Separação Clean-Room (Sala Limpa)

A técnica de sala limpa é mandatória para garantir que nenhum direito autoral, segredo comercial ou código de terceiros contamine a base de código do Limivra.

```
+-------------------------------------------------------------+
|                     TRILHA DE PESQUISA                      |
| (Analista / Pesquisador em ambiente isolado)                |
|                                                             |
|  - Analisa documentação AOSP / tráfego observável / APIs   |
|  - Observa entradas, saídas, intents, flags e retornos      |
|  - Formula hipóteses e realiza testes funcionais externos   |
+-------------------------------------------------------------+
                               |
                               | Produz exclusivamente:
                               v
+-------------------------------------------------------------+
|             ESPECIFICAÇÃO COMPORTAMENTAL FUNCIONAL          |
|  (docs/research/findings/<slug>.md)                         |
|                                                             |
|  - Descreve protocolos abstratos em linguagem natural       |
|  - Tabelas de mensagens / eventos / coordenadas neutras     |
|  - Diagramas de sequência e fluxogramas funcionais          |
|  - ZERO linhas de código decompilado, binários ou nomes de  |
|    estruturas internas não públicas                          |
+-------------------------------------------------------------+
                               |
                               | Consome exclusivamente:
                               v
+-------------------------------------------------------------+
|                   TRILHA DE IMPLEMENTAÇÃO                   |
| (Engenheiro de Produto Limivra - Rust / Qt)                 |
|                                                             |
|  - Escreve código original em conformidade com MIT          |
|  - Implementa clientes e parsers baseados na especificação  |
|  - Nunca tem acesso a ferramentas REA, decompiladores ou    |
|    binários proprietários durante a implementação           |
+-------------------------------------------------------------+
```

### 4.1 Regras de Isolamento Funcional
1. **Separação de papéis:** Sempre que possível, o agente ou pesquisador responsável pela dissecação/observação comportamental de um artefato proprietário não deve ser o mesmo que escreve os módulos de produção correspondentes, ou deve haver um marco temporal e documental nítido de descontinuidade entre o término da análise e o início do código.
2. **Proibições Absolutas de Transferência:**
   - Proibido copiar e colar qualquer trecho de código (Java, Smali, C/C++, Rust, Assembly) oriundo de decompilação ou desmontagem.
   - Proibido replicar nomes de classes internas, variáveis privadas ofuscadas, identidades de pacotes não expostos em APIs públicas ou estruturas de dados proprietárias não padronizadas.
   - Proibido transferir assets visuais, ícones, fontes, sons, animações ou marcas comerciais de terceiros para o repositório ou distribuição do Limivra.
3. **Efeito Não Mágico:** A adoção do procedimento clean-room é uma salvaguarda técnica e de boas práticas, mas **não afasta por si só** eventuais obrigações legais, patentes ativas ou disposições contratuais aplicáveis. Nenhuma imunidade jurídica automática deve ser presumida.

---

## 5. Matriz Operacional de Portões: GO / REVIEW / STOP

Antes de iniciar qualquer atividade investigativa ou teste de interoperabilidade, aplique a seguinte matriz:

| Cenário Identificado | Decisão Operacional | Ação Imediata do Agente / Pesquisador |
| :--- | :---: | :--- |
| **Pesquisa baseada em documentação AOSP pública, RFCs ou repositório `scrcpy` (Apache-2.0)** | **GO** | Executar conforme plano; registrar referências com URLs e datas de acesso. |
| **Execução de comandos públicos documentados via ADB em dispositivo próprio autorizado** | **GO** | Proceder com logs mínimos sanitizados, respeitando privacidade do usuário. |
| **Inspeção de artefato de terceiro legítimo sujeito a termos contratuais ou licença restritiva** | **REVIEW** | Interromper execução imediata; documentar proveniência e submeter ao mantenedor (`NEEDS_APPROVAL`). |
| **Dúvida sobre a natureza proprietária ou escopo de licença de uma biblioteca** | **REVIEW** | Registrar lacuna em issue/documento; suspender uso até validação humana. |
| **Artefato sem cadeia de custódia, origem anônima ou fonte de download não verificável** | **STOP** | Rejeição irrevogável; não baixar, não transferir, não analisar. |
| **Exigência de quebra de DRM, evasão de `FLAG_SECURE` ou contorno de criptografia** | **STOP** | Bloqueio imediato; respeitar restrição de segurança da plataforma. |
| **Exigência de exploração de vulnerabilidade, root forçado ou modificação de ROM base** | **STOP** | Bloqueio imediato; alinhamento obrigatório com [ADR-0003](../architecture/decisions/ADR-0003-no-root.md). |
| **Acesso a credenciais de usuário, dados pessoais (PII), fotos, mensagens ou contas protegidas** | **STOP** | Bloqueio imediato; contenção e isolamento seguro do material (não publicar, não duplicar); encaminhar incidente ao mantenedor para descarte seguro aprovado. |

---

## 6. Modelo de Registro de Findings (Achados de Pesquisa)

Todo resultado ou hipótese formulada a partir da pesquisa REA ou de testes de interoperabilidade deve ser documentado em `docs/research/findings/<slug>.md`, utilizando a estrutura obrigatória abaixo:

```markdown
# Finding: [Título Curto do Comportamento Investigado]

**Status:** [PROPOSED / VERIFIED / REFUTED / NEEDS_REVIEW]
**Data da Análise:** [AAAA-MM-DD]
**Pesquisador Responsável:** [Nome / Agente]
**Rastreabilidade:** Issue #[N], Marco [M0/M1/M2]
**Classificação da Fonte:** [CAT-A / CAT-B / CAT-C (com aprovação)]

## 1. Pergunta Focal de Pesquisa
[Qual comportamento exato de exibição, entrada, ciclo de vida ou transporte está sendo investigado?]

## 2. Hipótese
[O que se espera que a plataforma ou serviço realize em resposta ao estímulo ou evento?]

## 3. Fontes Consultadas e Proveniência
- Fonte pública 1: [URL completa, seção, data de acesso]
- Referência de código aberto: [Repositório, licença, tag/commit, arquivo de referência]
- Registro de autorização (se aplicável): [Referência da decisão de aprovação humana]

## 4. Ambiente e Ferramentas
- Ferramenta utilizada: [ex.: REA vX.Y.Z, adb version X.Y.Z, scrcpy vX.Y]
- Dispositivo / Ambiente: [ex.: Emulador Android 14 API 34 / Hardware de bancada com consentimento]
- Variáveis de controle: [Configurações específicas da sessão]

## 5. Observações e Comportamento Mensurado
[Descrição objetiva e independente dos estímulos fornecidos e das respostas observadas externamente.]
- Estímulo A: [Ex.: Envio de comando via console de depuração]
- Resposta observada: [Ex.: Mudança de estado no gerenciador de telas, broadcast documentado recebido]

## 6. Especificação Comportamental Abstrata (Clean-Room)
[Formulação em linguagem natural ou modelo conceitual de como reproduzir a funcionalidade de forma limpa.]
[ATENÇÃO: Proibida a inclusão de código decompilado, bytecode ou símbolos proprietários não públicos.]

## 7. Grau de Confiança e Limitações
- Grau de confiança: [ALTO / MÉDIO / PRELIMINAR]
- Limitações técnicas observadas: [Ex.: Funciona apenas em resoluções específicas, depende de permissões não concedidas por padrão]
- Diferenciação de status: [O que é fato comprovado vs. inferência técnica vs. hipótese em aberto]

## 8. Avaliação de Riscos e Impacto no Produto
- Risco de regressão / quebra de API: [Baixo / Médio / Alto]
- Risco de conformidade e propriedade intelectual: [Parecer do pesquisador sobre salvaguardas adotadas]

## 9. Próximo Gate / Experimento Autorizado
[Qual é o próximo passo acionável? Ex.: Propor ADR arquitetural, elaborar issue de implementação independente, ou arquivar rota.]
```

---

## 7. Matriz de Riscos, Mitigações e Governança

| Risco Identificado | Severidade | Medida de Mitigação Obrigatória | Evidência Exigida | Responsável pela Validação |
| :--- | :---: | :--- | :--- | :--- |
| **Contaminação da codebase com código decompilado** | **CRÍTICA** | Implementação estrita da metodologia clean-room; auditoria de PRs; revisão independente de código. | Diff da PR limpo de qualquer código proprietário; especificação formal intermediária registrada. | Mantenedor do Projeto |
| **Violação de Termos de Serviço (ToS/EULA) de fabricantes** | **ALTA** | Prioridade obrigatória para APIs AOSP e comportamento público; submissão prévia de qualquer análise de binário proprietário. | Parecer ou aprovação explícita registrada na issue antes do início da análise. | Mantenedor / Revisor Legal |
| **Vazamento de dados privados ou identificadores do dispositivo em issues públicas** | **ALTA** | Sanitização pré-commit automática ou manual; política de logs mínimos; proibição de raw logcat. | Verificação estática de logs sanitizados e ausência de strings sensíveis nas PRs. | Pesquisador e Revisor |
| **Dependência de funcionalidades proprietárias instáveis ou exclusivas de um OEM** | **MÉDIA** | Princípio *Capability-First* ([ADR-0002](../architecture/decisions/ADR-0002-capability-first.md)); fallback transparente para espelhamento básico. | Teste de degradação graciosa em dispositivos AOSP padrão. | Engenheiro de Produto |
| **Suposição não comprovada sobre suporte de ROM (ex.: HyperOS/One UI)** | **MÉDIA** | Rotulagem rigorosa como `UNPROVEN` até teste observável direto documentado conforme [SPIKE-001.md](SPIKE-001.md). | Evidência de execução em log sanitizado e matriz de compatibilidade atualizada. | Pesquisador Responsável |

---

## 8. Integração com o Roadmap e Relações entre Issues

```
       TRILHA REA (Épico #2)                   TRILHA DISPOSITIVO / CORE
 ------------------------------------     ------------------------------------
             [Issue #12]
    (Política de Proveniência REA)
                  |
    (Revisão e Aprovação Humana)
                  v
             [Issue #13]                               [Issue #9]
    (Investigação AOSP / scrcpy)                   (Hardware SPIKE-001)
     - Foco estrito em fontes públicas                  |
                  |                                     |
                  +------------------+------------------+
                                     |
                                     v
                                [Issue #14]
                        (Gaps e Pesquisa Aprofundada)
                        - Depende formalmente de #9 e #13
                        - Análise proprietária CAT-C exige
                          autorização humana específica

 =============================================================================
              TRILHA PARALELA INDEPENDENTE: MVP MIRROR (Milestone M1)
 -----------------------------------------------------------------------------
             [Épico #3] (Transporte & Sessão Linux USB MVP)
             - Folhas de implementação: Issues #15 a #20
             - Independente de #12, #13 e #14
             - Baseia-se exclusivamente em APIs AOSP e scrcpy (ADR-0001)
```

1. **Relação com a Issue #13:** A execução da Issue #13 (Investigação de displays virtuais e comportamento multitelas sob o Épico #2) é formalmente dependente da aprovação prévia desta política (#12). A Issue #13 **deve obrigatoriamente iniciar por documentação pública AOSP e código aberto (`scrcpy`)**. Nenhuma análise de artefatos de fabricantes (CAT-C) poderá ser realizada na #13 sem abertura de solicitação específica e autorização humana prévia.
2. **Relação com a Issue #14 e Marco M2:** A Issue #14 (Estudo de gaps e pesquisa aprofundada) depende formalmente de duas frentes: os resultados observáveis do teste de hardware real ([Issue #9](https://github.com/pedro-labsabs/limivra/issues/9) / [SPIKE-001.md](SPIKE-001.md)) e a consolidação das APIs públicas da [Issue #13](https://github.com/pedro-labsabs/limivra/issues/13).
3. **Não interferência e ausência de dependência para o MVP MIRROR:** Reitera-se que o desenvolvimento do MVP de espelhamento USB (Épico #3 / Milestone M1, composto pelas issues #15 a #20) ocorre em trilha paralela e independente. Não existe qualquer dependência de fluxo da pesquisa REA (#12, #13, #14) para o MVP de MIRROR, garantindo a entrega do produto sem bloqueios decorrentes de investigações exploratórias. O Épico #1 coordena a governança fundacional de M0, não se confundindo com o MVP de transporte.

---

## 9. Referências Públicas Primárias

As seguintes fontes primárias abertas foram consultadas na formulação desta política e servem de guia para as atividades de pesquisa:

1. **Repositório do Ferramental REA:**
   - URL: `https://github.com/morluto/rea`
   - Data de consulta: 19/10/2026
   - Finalidade: Compreensão da arquitetura de instrumentação, limites de inspeção de binários e escopo da ferramenta de apoio.
2. **Documentação de Multi-Display do AOSP:**
   - URL: `https://source.android.com/docs/core/display/multi_display`
   - Data de consulta: 19/10/2026
   - Finalidade: Definição dos contratos oficiais do Android para criação e gerenciamento de displays virtuais secundários.
3. **Repositório do scrcpy (Genymobile):**
   - URL: `https://github.com/Genymobile/scrcpy`
   - Licença: Apache-2.0
   - Data de consulta: 19/10/2026
   - Finalidade: Avaliação dos métodos abertos de captura de vídeo, injeção de eventos via ADB e gerenciamento de sessão sem root.
