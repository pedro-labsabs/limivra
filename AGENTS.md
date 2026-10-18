# Instruções obrigatórias para agentes — Limivra

**Leia antes de alterar qualquer arquivo.** Objetivo: entregar software Android→Linux com evidência reproduzível, sem root no fluxo base.

## Fonte de verdade e ordem de leitura

1. `README.md` e `docs/product/PRD.md` (objetivos, não objetivos, gates).
2. `docs/architecture/system.md` e ADRs (contratos e decisões propostas).
3. `docs/planning/roadmap.md` e `docs/planning/backlog.md` (épicos, milestones e dependências).
4. Issue atribuída (escopo autorizado). Consulte `docs/testing/strategy.md`.
5. Para reverse engineering: `docs/research/rea-plan.md` e limites legais.

## Mandato e restrições

- **Não começar implementação somente porque há backlog.** A especificação de arquitetura continua *proposta para revisão humana*. Implementação requer plano aprovado e issue pronta.
- Um agente = uma issue/slice por vez. Trabalhar em branch `agent/<numero>-<slug>`; PR para `main`; sem push direto em `main` para desenvolvimento.
- Não criar, alterar ou apagar recursos externos, redefinir branches, fazer merge ou instalar software no dispositivo sem autorização aplicável.
- Evitar root, bootloader unlock, desabilitar controles de segurança, bypass de DRM, acesso a credenciais, captura de informações pessoais ou modificação persistente da ROM.
- Não copiar classes, binários, assets nem trechos decompilados de software proprietário para o produto. REA somente para pesquisa de interoperabilidade, com proveniência.
- Nunca afirmar que a ROM suporta freeform/desktop sem teste observável.
- Não gravar dumps de dados pessoais, `adb logcat` bruto ou capturas privadas em issues/commits. Redigir identificadores e caminhos pessoais.
- Dependências com licenças compatíveis, pinadas e auditadas; licença do projeto deve ser resolvida antes de distribuição.
- Sem afirmações fictícias de teste. `BLOCKED` e `UNPROVEN` são resultados válidos.

## Contrato de execução de toda issue

1. Verificar HEAD, issue, milestone, bloqueios, ADRs e estado do workspace.
2. Declarar objetivo, arquivos esperados, critérios de aceite, riscos e testes.
3. Executar somente o escopo autorizado; preferência por testes antes de implementação.
4. Registrar comandos e resultados reais, versões e evidências reproduzíveis.
5. Revisar efeitos, segurança, permissões e regressões; evitar refatoração não relacionada.
6. Abrir PR com: `Closes #N` quando apropriado, mudanças, prova de teste, risco, rollback, pendências.
7. Marcar conclusão apenas com critérios atendidos. CI verde não prova experiência real num aparelho.

## Estados e evidência

- `COMPLETE`: critérios técnicos comprovados e evidência anexada.
- `BLOCKED`: dependência externa ou limite comprovado impede o próximo passo.
- `NEEDS_APPROVAL`: requer decisão de escopo/risco/licença.
- `FAILED`: tentativa e reparo limitado falharam.
- Etiquetas `status:needs-evidence`, `status:blocked` e `status:ready` descrevem prontidão, não sucesso automático.

## Política de issue

Uma tarefa deve ter: contexto, objetivo, passos, artefato de saída, critérios verificáveis, dependências e comportamento em caso de falha. Não criar issues duplicadas; atualizar issue existente. Conferir o progresso pelo estado *real* das sub-issues, não por descrições otimistas.

## Primeiro trabalho executável

`SPIKE-001` (docs/research/SPIKE-001.md). O experimento requer consentimento para ativar a depuração USB no telefone e observação local. Não presumir que já ocorreu.
