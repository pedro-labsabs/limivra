# ADR-0002 — detecção de capacidades, não de fabricante

**Status:** ACCEPTED (2026-10-22), limitado à separação dos modos, aos estados de capacidade, ao padrão UNKNOWN, à evidência observável e ao fallback consentido.

## Contexto
Um display virtual Android pode abrir sem permitir launcher completo, decorações do sistema ou freeform. Fabricante/API level não determina suporte.

## Decisão aceita no escopo limitado
Representar capacidades com SUPPORTED, UNSUPPORTED, UNKNOWN e BLOCKED, registrando origem da prova, versão e razão. Separar MIRROR, VIRTUAL_DISPLAY e DESKTOP_WINDOWING. UNKNOWN é o padrão; suporte de ROM só pode ser classificado com evidência observável no dispositivo e ambiente identificados, não por fabricante, nível de API ou presença de flag no host. Oferecer fallback de VIRTUAL_DISPLAY para MIRROR com explicação e iniciar mirror somente após aceite explícito do usuário. Esta ratificação não define probes, critérios concretos de detector ou qualquer capacidade do Redmi Note 13 4G/HyperOS.

## Consequências
Fallback nunca é silencioso: a oferta de MIRROR exige explicação e aceite afirmativo antes de iniciar. Ausência de prova mantém UNKNOWN; BLOCKED não equivale a UNSUPPORTED.
