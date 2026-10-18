# Backlog e organização no GitHub

**Autoridade operacional:** issues e milestones atuais em https://github.com/pedro-labsabs/limivra/issues. Este arquivo define convenções, não substitui o estado do GitHub.

## Convenções
- `[EPIC]`: issue pai vinculada a sub-issues nativas do GitHub.
- `[SPIKE]`: investigação com hipótese, comandos e limite de tempo/escopo, entrega de evidência.
- `[TASK]`: trabalho de implementação pequeno e testável.
- `[DECISION]`: decisão humana, ADR ou licença.
- `[QA]`: testes, regressões, release gates.
- Dependências na issue (`Depends on #N`) são *informativas*; confirmar que blockers estão encerrados com provas.
- Etiquetas por tipo, área, prioridade, prontidão e risco.
- Milestone por fase, sem datas até existir compromisso explícito.

## Ordem de execução sugerida
1. Validar licença e arquitetura proposta.
2. Executar SPIKE-001 no Redmi/HyperOS e registrar matriz observada.
3. Confirmar contratos e preparar plano de implementação.
4. Construir núcleo USB com mock/processos controlados e UI mínima.
5. Adicionar virtual display com fallback não enganoso.
6. Expandir UX, Wi-Fi, testes, packaging e release.

## Definition of Ready
Issue tem objetivo mensurável, fontes, dependências, aceites, acesso necessário, plano de teste, riscos e restrições. Investigação de hardware requer disponibilidade e consentimento do usuário. Caso contrário, marcar `status:blocked` ou `status:needs-evidence`.

## Definition of Done
Artefatos versionados + testes executados + evidência real para alegação real + revisão da PR + documentação atualizada. Não fechar issue por texto plausível, CI isolado ou hipótese.

## Ligações úteis
- https://github.com/pedro-labsabs/limivra/milestones
- https://github.com/pedro-labsabs/limivra/labels
- https://github.com/pedro-labsabs/limivra/issues
- `AGENTS.md`
