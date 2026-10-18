# Roadmap — evidência primeiro

**Status:** PLANEJADO. Milestones são gates de entrega, **sem prazos inventados**. Épicos são issues-pai com sub-issues reais no GitHub.

| Milestone | Resultado auditável | Gate de saída |
|---|---|---|
| M0 — Foundation & Feasibility | governança/licença decidida, arquitetura validada, SPIKE-001 e matriz de capacidades | relatório real de hardware + plano aprovado do primeiro slice |
| M1 — Linux USB Session MVP | descoberta + seleção, sessão mirror, supervisão e interface mínima | E2E USB, reconexão e teardown sem root |
| M2 — Virtual Display & Fallback | provider virtual, launch app, detecção, fallback explícito | evidência de virtual OU relatório de limite ROM + mirror correto |
| M3 — Desktop UX & Wi-Fi | UX de sessão, foco/input, conexão Wi-Fi autorizada, experimentos desktop | smoke Wayland/X11 e provas de compatibilidade reais |
| M4 — Hardening & Release | testes, privacidade, distribuição e documentação | check-list release, licenças e matriz coerente |

## Dependências e caminho crítico
M0 (licença/design/teste) → M1 (núcleo + USB/mirror) → M2 (virtual + fallback) → M3 (UX e Wi-Fi) → M4 (release).
A pesquisa REA é uma trilha exploratória paralela e só vira bloqueante se uma lacuna concreta for demonstrada. Multijanela completa é `stretch`, não deve bloquear MVP.
Cada sub-issue descreve seus próprios bloqueios; reordenar por evidências, não por ambição.

## Milestone versus épico
- Milestone: fase/entrega com gate verificável, uma por issue executora.
- Épico: issue de coordenação contendo escopo, critérios e sub-issues; não representa código pronto.
- Sub-issue: slice implementável/provável com owner, evidências e PR.
- Risco: etiqueta e issue própria, nunca escondido num épico concluído.

## Controle de mudanças
Se um spike negar hipótese, atualizar capability matrix + ADR + gate da issue afetada; manter mirror útil. Sem alteração silenciosa de escopo ou data fictícia.

## Progresso
Consultar milestones e issues do repositório; nenhum código de produto implementado na data desta documentação.
