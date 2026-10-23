# Política de segurança e divulgação responsável

**Atualizado:** 2026-10-21. **Estado do produto:** fase inicial, sem release implantado.

## Como reportar uma vulnerabilidade

Não abra issue, discussão, pull request ou comentário público com detalhes de uma vulnerabilidade ainda não corrigida. O reporte privado de vulnerabilidades do GitHub é o canal preferido: [Reportar uma vulnerabilidade](https://github.com/pedro-labsabs/limivra/security/advisories/new). Em 2026-10-21, a consulta às configurações do repositório indicou que esse recurso está **desabilitado**. Até que os mantenedores o habilitem, contate o mantenedor do projeto pelo perfil GitHub [@pedro-labsabs](https://github.com/pedro-labsabs) solicitando um canal privado; não publique os detalhes enquanto esse canal não for combinado.

Envie uma descrição concisa, versão/commit afetado, pré-condições, impacto e passos mínimos de reprodução. Compartilhe uma prova de conceito somente se for necessária para triagem e sem dados reais. Nunca inclua senhas, chaves ADB, códigos de pareamento, IMEI, seriais completos, capturas de apps bancários ou conteúdo pessoal. Se algum segredo real for exposto durante a reprodução, interrompa o teste, não o copie nem o publique e avise o mantenedor pelo canal privado.

## Prazos de resposta

- Confirmação inicial de recebimento em até **5 dias úteis**.
- Triagem inicial, severidade e próximos passos em até **10 dias úteis**.
- Atualização pelo menos a cada **10 dias úteis** enquanto o caso permanecer aberto.

Esses prazos são metas de resposta, não prazo garantido para correção. O cronograma de mitigação será acordado após a triagem, considerando impacto, disponibilidade de mantenedores e dependências upstream. O projeto não oferece programa de recompensa.

## Escopo da revisão

São relevantes vulnerabilidades reproduzíveis na versão atual do Limivra, especialmente em `limivra-core`, `limivra-cli`, workflows, parsing e limites de subprocessos, redaction de diagnósticos, sessões USB/Wi-Fi ADB, socket Unix privado, credenciais, foco/input/clipboard e respeito a conteúdo protegido por `FLAG_SECURE`. Falhas de ADB, scrcpy, Android, ROMs ou bibliotecas devem também ser comunicadas ao projeto upstream responsável quando aplicável; inclua apenas o impacto específico da integração Limivra.

Não autorize testes contra aparelhos, contas, redes ou dados de terceiros. Root, desbloqueio de bootloader, alteração persistente de ROM, instalação de APK/companion, bypass de `FLAG_SECURE`/DRM, engenharia social, exfiltração de dados e negação de serviço em serviços públicos estão fora do escopo.

## Regras para pesquisadores

- Teste apenas em sistemas próprios ou com autorização explícita do proprietário; mantenha as ações reversíveis e sem root.
- Minimize acesso a dados: use aparelhos de teste, aplicativos sem conteúdo pessoal e identificadores sintéticos.
- Não acesse, altere, retenha ou transfira dados além do mínimo necessário para demonstrar o problema.
- Não persista credenciais, códigos de pareamento, keystrokes, clipboard, dumps brutos ou capturas privadas.
- Dê aos mantenedores tempo razoável para triagem e mitigação antes de divulgação coordenada; combine previamente qualquer publicação de detalhes.
- Pare imediatamente se o teste alcançar dados de terceiros, escapar do escopo autorizado ou puder causar alteração persistente.

Esta política descreve o escopo de colaboração do projeto; ela não concede autorização para acessar dispositivos ou serviços de terceiros nem substitui a legislação aplicável.
