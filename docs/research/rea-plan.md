# Pesquisa com REA — interoperabilidade DeX/Android

**Status:** PLANEJADO, não iniciado. A pesquisa não é condição para começar o MVP de espelhamento.

## Pergunta de pesquisa
Quais comportamentos de transporte, display secundário, entrada, gestão de janelas e inicialização do DeX são reproduzíveis por APIs comuns do Android/ADB/scrcpy, e quais dependem de componentes Samsung/privilegiados?

## Método
1. Começar por documentação Android AOSP/scrcpy e comportamento real do SPIKE-001.
2. Criar pergunta focal, hipóteses testáveis e artefato-alvo legalmente obtido; verificar autorização e termos aplicáveis **antes** de analisar.
3. Usar REA `morluto/rea` instalado de fonte oficial e ferramentas nativas apropriadas, versão registrada; análise preferencialmente estática.
4. Manter evidência: versão/hash do artefato (privado), método, elementos observados, confiança, limitações, fonte legal.
5. Produzir *especificações de comportamento independentes*, nunca copiar trechos, estruturas internas confidenciais, ícones, marcas, binários ou código decompilado.
6. Validar cada conclusão via prova pública/Android/scrcpy e teste separado da nossa implementação.
7. Se o artefato é restrito, DRM protegido ou análise infringiria termos/lei, encerrar essa rota e pesquisar alternativas públicas.

## Linhas
- R1: displays secundários/launcher/IME/decorações.
- R2: input/mouse/teclado e eventos.
- R3: negociação/transportes entre host e Android, focando documentação disponível.
- R4: freeform e permissões; diferenciar AOSP, OEM, app privilegiado.

## Não objetivos
Clonar APK DeX, contornar root/FRP/DRM, instalar software extraído, recuperar segredos, burlar autenticação ou capturar tráfego alheio.

## Saídas
`docs/research/findings/<slug>.md` contendo questão, proveniência, prova reproduzível, comportamento observável, risco e decisão arquitetural. Criar ADR quando a conclusão mudar contrato. Não registrar material proprietário bruto.

## Referências
https://github.com/morluto/rea
https://source.android.com/docs/core/display/multi_display
