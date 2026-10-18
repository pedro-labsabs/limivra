# ADR-0002 — detecção de capacidades, não de fabricante

**Status:** PROPOSED (2026-10-18)

## Contexto
Um display virtual Android pode abrir sem permitir launcher completo, decorações do sistema ou freeform. Fabricante/API level não determina suporte.

## Decisão proposta
`SUPPORTED | UNSUPPORTED | UNKNOWN | BLOCKED` com origem da prova, versão, razões. Separar `MIRROR`, `VIRTUAL_DISPLAY` e `DESKTOP_WINDOWING`. UNKNOWN por padrão. Detectar por comportamento observado no dispositivo e não só por `getprop`.

## Consequências
Fallback explícito evita UX enganosa; testes de suporte têm limites, evitam manipulação persistente e preservam privacidade.
