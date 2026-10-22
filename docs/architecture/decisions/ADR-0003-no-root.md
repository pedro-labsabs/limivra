# ADR-0003 — sem root no caminho principal

**Status:** ACCEPTED (2026-10-22), limitado às restrições de segurança descritas nesta decisão.

## Contexto
Root, bootloader desbloqueado e hacks podem comprometer integridade Android, apps bancários e adoção.

## Decisão aceita no escopo limitado
Nenhum fluxo base exigirá root, desbloqueio de bootloader, bypass de controles de segurança ou modificação persistente de mecanismos/configurações de segurança. Esta aceitação não define a interface ou o protocolo de autorização ADB e não autoriza acesso, comandos ou testes no aparelho.

## Consequências
Suporte de ROM permanece condicionado a evidência observável. Se VIRTUAL_DISPLAY não estiver disponível, MIRROR só pode ser oferecido com explicação e iniciado após aceite explícito do usuário; não contornar bloqueios.

## Reabrir quando
Mudança explícita de escopo com análise jurídica e de segurança e autorização específica do usuário.
