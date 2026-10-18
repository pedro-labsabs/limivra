# Segurança, privacidade, licenciamento e pesquisa

## Modelo de ameaça
ADB com depuração autorizada concede acesso amplo à sessão Android do usuário; capture/log de tela pode revelar informações pessoais; Wi-Fi mal configurado pode ampliar superfície. O Limivra não deve prometer isolamento que ADB não fornece.

## Política
- Sem root ou desbloqueio no fluxo base; acesso local explícito e revogável.
- Não coletar credenciais/serial integral/IMEI/logcat/capturas privados em diagnóstico remoto.
- Logs mínimos sanitizados, opt-in para exportação, sem telemetry remota por padrão.
- Não aceitar automaticamente novos hosts; usuário aprova no dispositivo.
- Wi-Fi: pareamento autorizado, não publicar listener sem proteção na rede aberta.
- Nunca fazer bypass de DRM, FLAG_SECURE, apps bancários ou controles de ROM; falha de captura deve ser respeitada.
- Dependências fixadas por release/checksum e auditadas; não incorporar APK/SDK binário de fabricante sem permissão.
- REA apenas para pesquisa legal, interoperabilidade, documentação de comportamento, evidências redigidas e implementação independente.

## Licenças
Repositório **público não significa licença open source**. Escolha explícita da licença do Limivra deve ser discutida e registrada; não criar um LICENSE arbitrário. Antes de distribuir, verificar licenças de scrcpy, Qt, Rust crates, ADB e packaging, obrigações de notices e linking.

## Divulgação de vulnerabilidade
Ver `SECURITY.md`. Não publicar tokens, vulnerabilidades exploráveis nem registros pessoais em issues públicos.
