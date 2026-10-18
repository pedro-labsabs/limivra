# ADR-0001 — scrcpy externo como backend inicial

**Status:** PROPOSED (2026-10-18)

## Contexto
scrcpy oferece comunicação e interação maduras. Desenvolver streaming próprio antes de provar displays virtuais aumentaria custo.

## Decisão proposta
Invocar scrcpy como subprocesso controlado na primeira fase; testar `--version` e opções reais, tratar saída/falhas e encerrar apenas subprocessos próprios. ADB usado para descoberta e autorização.

## Consequências
+ Implementação inicial simples, reproduzível e reversível.
- UI não incorpora vídeo diretamente; opção de embedding requer nova ADR, medição e revisão de licenças.
- A instalação/distribuição precisa observar licenças de cada dependência.

## Reabrir quando
Houver prova de que a UX necessita stream/controle diretamente embutido e teste de custo/benefício.
