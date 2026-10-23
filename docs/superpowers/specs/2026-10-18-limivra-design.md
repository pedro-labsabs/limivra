# Especificação de design — Limivra

**Status atual:** APROVADA em 23/10/2026, conforme confirmação humana do mantenedor na [Issue #8](https://github.com/pedro-labsabs/limivra/issues/8#issuecomment-6367240150).
**Histórico:** publicada como proposta para revisão humana em 18/10/2026.
**Limite:** a aprovação documental permanece subordinada à segurança e aos escopos das ADRs aceitas; não autoriza implementação nem conclui os gates de M1.

## Objetivo/usuário
Um usuário Linux conecta telefone Android via USB, autoriza depuração e opera apps do telefone dentro de uma janela Linux; posteriormente usa Wi-Fi. Idealmente numa tela virtual independente, sem root; multijanela depende da ROM e não é requisito obrigatório do MVP.

## Premissas e possibilidades
Separar três modos (mirror, virtual display, desktop windowing) e detectar capacidades por prova. Android executa os apps; Linux só apresenta imagem e encaminha controles. Primeiro alvo de teste é Redmi Note 13 4G com HyperOS, versão exata a medir. Não validar toda uma classe de aparelhos com um único teste.

## Abordagens comparadas
- **A. Wrapper scrcpy + ADB:** recomendada para v0.1; aproveita transporte, encurta risco e facilita prova.
- **B. Protocolo próprio:** controle fino, porém custo alto e desnecessário antes de benchmarks.
- **C. Modificação da ROM / root:** potencial mais acesso, mas incompatível com exigência de segurança e adoção.

## Divisão do sistema
UI Qt6/QML (hipótese de stack) separada do Session Core Rust (hipótese) e backends ADB/scrcpy. CapabilityProbe retorna status, evidência e causa; DisplayProvider escolhe modo; processo scrcpy é controlado com estados, timeouts e cleanup idempotente. APK companion não necessário a priori.

## Fluxo principal e exceções
Descobrir dispositivo → pedir/aguardar autorização ADB → verificar executáveis/opções → detectar suporte → iniciar modo selecionado → acompanhar subprocesso/input → encerrar sem afetar apps/sessões não pertencentes ao Limivra.
`VIRTUAL_DISPLAY` indisponível: informar causa e oferecer `MIRROR`; nunca declarar sucesso ou alternar sem consentimento. Wi-Fi é separado e depende de pareamento permitido.

## Evolução desacoplada
Programa A: provas/capabilities (M0).
Programa B: sessão USB Linux (M1).
Programa C: display virtual/lançamento app/fallback (M2).
Programa D: interface desktop, entrada, Wi-Fi (M3).
Programa E: qualidade, licenças e empacotamento (M4).
Pesquisa REA paralela e não bloqueadora, focada apenas em dúvidas evidenciadas.

## Teste e segurança
Mocks de parse ADB, state machine, modos e reconexão; integração com fake subprocesso; E2E em hardware real com versions e logs sanitizados; smoke Wayland/X11; sem root, bypass, coleta silenciosa ou binários proprietários reaproveitados. CI não substitui prova em aparelho.

## Critérios de design e gate
Detalhes verificáveis em `docs/product/PRD.md`, `docs/research/SPIKE-001.md` e `docs/testing/strategy.md`. A confirmação humana de aprovação desta especificação e do plano mínimo da #15 foi registrada na [Issue #8](https://github.com/pedro-labsabs/limivra/issues/8#issuecomment-6367240150); ela não libera M1 enquanto #9, #10 e a conclusão de viabilidade da #11 permanecerem pendentes.
