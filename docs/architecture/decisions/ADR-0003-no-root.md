# ADR-0003 — sem root no caminho principal

**Status:** PROPOSED (2026-10-18)

## Contexto
Root, bootloader desbloqueado e hacks podem comprometer integridade Android, apps bancários e adoção.

## Decisão proposta
Nenhum fluxo base requer root, ROM customizada, desbloqueio, alteração de políticas, privilégios persistentes ou bypass de mecanismos de segurança. O usuário pode habilitar e revogar autorização ADB no próprio dispositivo.

## Consequências
Nem todas ROMs permitirão tela virtual ou multijanela. Retornar modo espelhamento com explicação em vez de tentar contornar bloqueios.

## Reabrir quando
Mudança explícita de escopo com análise jurídica e de segurança e autorização específica do usuário.
