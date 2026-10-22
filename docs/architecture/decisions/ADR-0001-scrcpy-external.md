# ADR-0001 — scrcpy externo como backend inicial

**Status:** ACCEPTED (2026-10-22), limitado ao uso de scrcpy como ferramenta externa no backend inicial e à supervisão de subprocessos próprios.

## Contexto
scrcpy oferece comunicação e interação maduras. Desenvolver streaming próprio antes de provar displays virtuais aumentaria custo.

## Decisão aceita no escopo limitado
Invocar scrcpy como ferramenta externa supervisionada em subprocesso no backend inicial; verificar versão e opções reais, tratar saída e falhas, supervisionar timeout e cancelamento e fazer teardown/encerramento seguro exclusivamente do processo filho iniciado pelo Limivra. ADB também será ferramenta externa em subprocesso supervisionado para funções futuras autorizadas. Esta aceitação não fixa a comunicação da UI com o núcleo, não aprova embedding de vídeo e não autoriza implementação.

## Consequências
+ Implementação inicial simples, reproduzível e reversível.
- UI não incorpora vídeo diretamente; opção de embedding requer nova ADR, medição e revisão de licenças.
- A instalação/distribuição precisa observar licenças de cada dependência.

## Reabrir quando
Houver prova de que a UX necessita stream/controle diretamente embutido e teste de custo/benefício.
