# Modelo de ameaças do Limivra

**Versão:** 1.0 — 2026-10-21
**Escopo:** contrato de segurança do núcleo Rust e CLI atuais, além de requisitos obrigatórios para adaptadores/UI ainda não implementados.
**Estado do produto:** sem release; a arquitetura geral permanece proposta. Este documento não certifica comportamento de todos os binários ADB, sistemas Android ou fabricantes.

## Objetivos

- Operar como usuário Linux comum, sem `root`, `sudo`, bootloader desbloqueado ou privilégios persistentes no Android.
- Exigir seleção/consentimento ADB explícitos, aceitar somente endpoints Wi-Fi locais validados e limitar a comunicação ao aparelho autorizado.
- Não instalar APK/companion, modificar persistentemente a ROM, contornar controles OEM, DRM ou `FLAG_SECURE`.
- Evitar retenção de conteúdo de teclas: a política de input mantém somente identificadores de controles atualmente pressionados e os limpa ao perder foco, consentimento, modo ou sessão. Não armazena texto digitado nem histórico.
- Tratar código de pareamento como segredo transitório: não pô-lo em argumentos de processo, logs ou arquivos; limpar o buffer próprio de `PairingCode` no `Drop`.
- Manter conteúdo de clipboard fora do core. Qualquer transferência futura requer opt-in por direção, sessão ativa e focada, capacidade observada e autorização atual de proteção; `FLAG_SECURE`, restrições do sistema e observação desconhecida devem negar a transferência.
- Conter subprocessos próprios com argumentos separados (sem shell), timeouts, limites de saída, cancelamento, encerramento e reaping.
- Redigir identificadores de dispositivos nas saídas de CLI, logs, erros, representações `Debug` e serializações estruturadas por padrão.

## Não objetivos e limites

- O Limivra não protege contra o próprio usuário Linux, `root`, administradores, malware executando com o mesmo UID, keyloggers do host, captura de memória privilegiada ou comprometimento do kernel.
- Não corrige vulnerabilidades do Android, ROM, ADB, scrcpy ou dependências. Integração deve respeitar os canais upstream.
- Não garante entrega de eventos de liberação ao Android após perda abrupta do transporte. O adaptador deve enviar as liberações antes do teardown ou destruir seu dispositivo de input próprio.
- O core não implementa atualmente a UI, adaptador gráfico de clipboard nem política de retenção da área de transferência do sistema. A ausência de texto no core não garante a memória do processo GUI, compositor, clipboard manager ou OS.
- Volatile zeroization cobre apenas o buffer pertencente a `PairingCode`; não apaga cópias anteriores fornecidas pelo chamador, buffers internos do runtime/ADB, memória do kernel, swap ou dumps do sistema.
- Testes herméticos e inspeção de código não substituem auditoria do host, validação física em cada ROM, análise de binários externos ou prova de ausência de listeners em toda configuração possível.

## Fronteiras de confiança

| Fronteira | Dados/ações atravessados | Premissa e controle |
|---|---|---|
| Usuário do host ↔ processos locais desprivilegiados | Comandos, sessão, socket privado do ADB | O diretório de runtime e o store Android devem pertencer ao UID atual e ter modo exatamente `0700`; a chave ADB exige owner atual e nenhum bit de grupo/outros. A conexão de controle do ADB usa socket Unix em `localfilesystem:<caminho>`. Outro processo com o mesmo UID continua dentro da mesma fronteira de confiança e pode acessar esses recursos. |
| Host ↔ rede externa | Wi-Fi ADB | O core não oferece listener TCP nem serviço de descoberta/anúncio. O runner direciona o servidor ADB local a socket Unix e inicia conexões de saída somente para endpoint IPv4 privado explicitamente validado. Pareamento e conexão não devem abrir porta de entrada no host. Isso é uma propriedade da configuração do runner, não uma varredura de portas de cada sistema do usuário. |
| Host ↔ Android | ADB, scrcpy, input, display, clipboard futuro | O usuário autoriza depuração no próprio aparelho; o core não instala APK nem eleva privilégio. Códigos de pareamento vão por stdin do subprocesso. Restrições de ROM, autorização, display, input e conteúdo seguro são resultados a respeitar, nunca contornar. |
| Aplicação ↔ subprocessos externos | ADB/scrcpy e respostas não confiáveis | Argumentos separados sem shell, ambiente limpo quando definido pelo runner, timeouts, cancelamento e limites de bytes; parsing rejeita UTF-8 inválido, linhas acima do limite e respostas desconhecidas. Os executáveis e a cadeia de atualização continuam sob controle do sistema/usuário. |
| Política de input ↔ adaptador de input/clipboard | Controles pressionados, foco, consentimento e callbacks | A política emite intenções de liberação e não recebe texto do clipboard. O adaptador deve obedecer à intenção, entregar releases ou destruir recursos próprios, consultar proteção atual e não guardar texto em cache/fila/log. |

## Ativos críticos e proteções

| Ativo | Retenção/uso | Proteções presentes e limite |
|---|---|---|
| Serial/identificador ADB | Necessário transitoriamente para direcionar comandos e estado | CLI, tracing, erro de spawn, `Debug` e serialização de `AdbDevice`, `DeviceProfile` e `SessionState` redigem para `[REDACTED]`. Os campos operacionais em memória permanecem crus; callers externos não devem persistir nem imprimir esses campos. |
| Código de pareamento Wi-Fi | Necessário somente durante `adb pair` | `PairingCode` não implementa `Clone`, tem `Debug` redigido, é enviado em stdin (não argv) e seu buffer próprio recebe escritas voláteis de zero seguidas por compiler fence no `Drop`. A cópia fonte do chamador e buffers externos não são apagados pelo core. |
| Chave ADB existente | Arquivo `~/.android/adbkey` usado pelo ADB aprovado | O Limivra não cria nem copia a chave. Preflight exige arquivo regular, owner atual, sem permissões de grupo/outros e identidade de metadata estável; revalida antes de cada subprocesso. O ADB lê a chave fora do espaço controlado pelo core. |
| Conteúdo protegido/`FLAG_SECURE` | Imagem e ações de clipboard em contextos seguros | Não há bypass. Parser guarda `is_secure`; a política de clipboard nega proteção `Unknown`/`Restricted`. A integração de captura/transferência precisa manter o bloqueio; a UI/adapter ainda não foi implementada. |
| Keystrokes e modificadores | Identificadores de controles pressionados, sem caracteres | `InputPolicy` mantém apenas o conjunto transitório de controles pressionados. Perda de foco, consentimento, capacidade ou sessão limpa o conjunto e emite release de todas as teclas/botões/modificadores. Teclas nunca são registradas como texto. |
| Texto de clipboard | Não entra na política do core | Nenhum texto é lido ou guardado pelo core. Transferência futura só em callback síncrono, focado, ativa, opt-in e com proteção observada como permitida; área de transferência do SO e adapter permanecem fora da garantia atual. |
| Socket e chave de controle local | Socket Unix do servidor ADB e dados em `~/.android` | Diretórios validados em modo `0700`, chave ADB sem acesso por grupo/outros; socket não é publicado via TCP. Processos do mesmo UID não são isolados por esse modo. |
| Recursos de subprocessos | Handles de processos, streams e respostas | `run_bounded` limita cada stream a 1 MiB; parsers estruturados descartam linhas maiores que 16 KiB; runner Wi-Fi limita saídas a 8 KiB. Filhos próprios têm timeout/cancelamento e reaping. Fixtures de teste que sinalizam PID usam `pidfd` e identidade de processo/marker para impedir reutilização acidental; isso não é apresentado como mecanismo de PID de produção. |

## Ameaças e mitigações

| Ameaça | Impacto | Mitigação | Risco residual |
|---|---|---|---|
| Injeção de argumentos por serial, endereço, app ou resposta ADB | Executar comandos inesperados no host/aparelho | Invocação sem shell, enum/validação estrita de package/activity, IPv4 privado canônico e porta válida; parser limitado; falha fechada para resposta desconhecida. | Bug futuro num novo runner pode reintroduzir concatenação; revisão e testes de contrato são necessários por subprocesso. |
| Serial em log, erro, estado `Debug`, CLI ou artefato estruturado | Identificar o aparelho do usuário | Redaction central de argumentos, erro de spawn e superfícies CLI; `Debug` e serde das structs/estado redigem; testes com sentinela. | Callers ainda podem ler diretamente campos públicos crus ou gerar outra representação; API externa deve tratar serial como secreto operacional. |
| Código Wi-Fi em log/argv/disco ou memória após uso | Pareamento não autorizado | Código exclusivo em `PairingCode`, `Debug` redigido, stdin, nenhum arquivo e zeroização volátil de buffer no `Drop`. | Cópias do chamador, ADB, kernel, swap e depurador fora do alcance desta limpeza. |
| Acesso indevido ao socket/chave por outro usuário local | Controle da sessão e uso de credenciais ADB | Diretórios e dono verificados; modo exato `0700`; chave sem grupo/outros; caminhos canônicos e preflight imediatamente antes dos processos. | Processo malicioso com o mesmo UID ou `root` pode ler/usar os recursos. Preflight não é lock contra alteração concorrente privilegiada. |
| Exposição de porta/listener no host | Entrada remota e abuso de ADB | Servidor ADB configurado com socket `localfilesystem`, sem protocolo TCP local; sem anúncio/descoberta; conexão Wi-Fi do host é de saída ao endereço explicitamente autorizado. | Executáveis externos podem ter comportamento diferente; configuração/versão deve ser revisitada e observada ao alterar runner ou ADB. |
| Resposta ADB/dumpsys maliciosa, corrompida ou gigantesca | Consumo de memória, travamento ou decisão falsa | Limites de 1 MiB por stream (probes genéricos), 8 KiB (runner Wi-Fi), 16 KiB por linha estruturada, validação UTF-8 e fail-closed. | Limites precisam ser mantidos em novos consumidores e não substituem timeout/cancelamento. |
| PID encerrado e reutilizado durante teardown | Sinal enviado a processo alheio | Código de produto usa handles de filhos próprios, grupo de processo e reaping; fixtures de teste validam identidade com `pidfd` antes de sinalizar. Não se faz `killall` nem sinalização de PID recebido de saída não confiável. | Sem pidfd disponível, qualquer novo caminho de sinalização por PID exige mecanismo equivalente de identidade; código atual não declara pidfd como garantia universal de produção. |
| Tecla/modificador preso ao perder foco ou desconectar | Entrada involuntária posterior no Android | Estado transitório de controles é drenado em perda de foco, consentimento e transição da sessão; teste cobre teclas, botão e modificadores. | Se transporte já caiu, releases podem não chegar; adaptador deve encerrar/dispor seu input device. |
| Clipboard ou imagem segura lidos sem foco/consentimento | Exposição de texto/imagem pessoal | Nenhum texto de clipboard no core; transferência futura opt-in e focada; `Unknown`/`Restricted` e `FLAG_SECURE` negam. Sem bypass de controles Android. | Ainda não há adapter gráfico; a garantia só se aplica quando integrações futuras seguirem este contrato. |
| Comprometimento de binário ADB/scrcpy ou dependência | Execução local/controle do aparelho | Seleção de binários do sistema e contenção de subprocessos; revisão de dependências/licenças e gates offline; ambiente do ADB isolado no runner Wi-Fi. | Integridade/atualização dos binários externos pertence ao sistema do usuário e upstream; o Limivra não os audita em runtime. |

## Revisão e validação

Os contratos automatizados estão documentados em [evidência da Issue #31](../testing/issue-31-threat-model-security.md). A suíte verifica redaction em argumentos, modelos, tracing e erro de spawn, limpeza do buffer de pareamento, modo `0700` dos diretórios privados e liberação completa de controles ao perder foco. Testes de CLI e display existentes cobrem saída de processos, probe/log e serial sentinela. Nenhum desses testes prova comportamento de uma ROM, GUI/clipboard ainda não implementado ou ausência de listener em todo host.

Revisar este modelo quando mudar: processo/socket ADB, transporte Wi-Fi, persistência de configurações, captura de imagem/input, clipboard, logging, política de dependências ou instalação no Android. Vulnerabilidades devem seguir [SECURITY.md](../../SECURITY.md).
