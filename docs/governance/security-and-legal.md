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
O código original do Limivra é formalmente licenciado sob a [Licença MIT](../../LICENSE) (ver [ADR-0004](../architecture/decisions/ADR-0004-project-license.md)). A licença MIT aplica-se estritamente ao código produzido pelo projeto e não se estende automaticamente a componentes de terceiros.

Antes de qualquer distribuição ou empacotamento (ex.: pacotes de sistema, Flatpak ou AppImage), é obrigatório auditar e cumprir os termos e licenças de cada dependência:
- **scrcpy:** licença Apache-2.0 (manter licença, avisos de copyright e avaliar reprodução de arquivo `NOTICE` de origem caso existente no bundle).
- **Qt 6:** licença LGPLv3 no conjunto base (garantir mecanismos de substituição/relink, fornecimento de código-fonte correspondente e avisos de licença; atentar para que módulos Qt exclusivos sob GPLv3 não sejam integrados inadvertidamente sem prévia decisão arquitetural).
- **ADB / Platform-Tools:** licença Apache-2.0 para código AOSP; binários do SDK do Google possuem restrições de redistribuição que requerem atenção no modelo de empacotamento.
- **Rust crates:** auditar licenças na árvore de dependências (`cargo-deny` / `cargo-license`) para assegurar compatibilidade permissiva (MIT/Apache-2.0/BSD).

## Divulgação de vulnerabilidade
Ver `SECURITY.md`. Não publicar tokens, vulnerabilidades exploráveis nem registros pessoais em issues públicos.
