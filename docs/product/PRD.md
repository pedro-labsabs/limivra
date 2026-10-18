# Limivra — PRD (proposta v0.1)

**Status:** PROPOSTA, requer revisão humana. **Data:** 2026-10-18.

## Visão
Permitir usar aplicativos executados no próprio celular Android, controlados dentro do desktop Linux por USB ou Wi-Fi, sem depender de Samsung ou root. O Limivra não executa APKs no PC nem garante recursos que a ROM não oferece.

## Público e casos de uso
- Usuários Linux querendo usar Android no monitor com mouse/teclado.
- Usuários de celulares de outras marcas que não têm DeX.
- Desenvolvedores que precisam diagnosticar a viabilidade por aparelho/versão.

## Modos distintos
- MIRROR: replica e controla o display físico (baseline).
- VIRTUAL_DISPLAY: cria tela adicional e abre aplicativo Android nela (condicional).
- DESKTOP_WINDOWING: vários apps em janelas Android livres (condicional e *não* implícito no display virtual).

## Requisitos
| ID | Requisito | Evidência de aceite |
|---|---|---|
| FR-01 | Enumerar aparelhos | Lista 0/1/N, offline, unauthorized com seleção manual |
| FR-02 | Sessão USB mirror | Vídeo e input com desligamento limpo sem root |
| FR-03 | Detector de capacidade | SUPPORTED, UNSUPPORTED, UNKNOWN, BLOCKED, com causa/evidência |
| FR-04 | Display virtual | App abre em tela secundária, tela física continua independente |
| FR-05 | Fallback | Quando falha virtual, oferecer mirror explicando limitação |
| FR-06 | Entrada e foco | Mouse, teclado, atalhos e clipboard quando permitido |
| FR-07 | Recuperação | Timeouts, conexão perdida e encerramento sem processos órfãos |
| FR-08 | Diagnósticos privados | Logs opt-in sem tokens, seriais completos ou conteúdo pessoal |
| FR-09 | Wi-Fi | Pareamento local autorizado, conexão/reconexão com aviso |
| FR-10 | Freeform | Só expor como suportado após observação de múltiplas janelas |

## NFRs e qualidade
Sem root/bootloader unlock, ferramentas executadas como usuário; Linux (Wayland/X11 testados separadamente); consentimento de depuração ADB; medição de latência/FPS antes de fixar metas; comportamento estável na desconexão; sem telemetria remota por padrão.

## Produto por fases
M0 prova e segurança; M1 conexão USB e sessão mirror; M2 tela virtual e fallback; M3 experiência desktop e Wi-Fi; M4 testes, empacotamento e release candidate.

## Fora de escopo inicial
Execução de Android no Linux, suporte Windows/macOS, DeX copiado, root e bypass de DRM, modo celular ligado diretamente a monitor, controle não autorizado pela Internet, contornar bloqueios de aplicativos protegidos.

## Questões abertas
HyperOS pode restringir displays; integração Qt/Rust requer validação; licenciamento do Limivra ainda não escolhido; versão exata scrcpy e distribuição dependem de descoberta e revisão de licenças. Não criar release antes das decisões.
