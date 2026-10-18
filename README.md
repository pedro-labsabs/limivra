# Limivra

> O Android ganha espaço no Linux, sem depender de um fabricante.

**Estado:** planejamento e pesquisa; nenhum aplicativo Limivra funcional foi implementado. **Idioma principal:** português brasileiro. **Repositório:** https://github.com/pedro-labsabs/limivra

Limivra é uma proposta de aplicativo livre para acessar e controlar um telefone Android a partir de um computador Linux, por USB ou Wi-Fi, com uma experiência progressivamente próxima de um desktop. O produto deve funcionar sem root para seu conjunto básico de recursos e *detectar* capacidades reais de cada dispositivo, sem prometer que todo Android oferece janelas livres.

## O que pretendemos entregar

1. **Controle confiável:** descoberta via ADB, conexão autorizada, vídeo e entrada de mouse/teclado.
2. **Tela independente:** criar uma tela virtual Android, quando suportada, e abrir aplicativos nela.
3. **Experiência desktop:** gerenciador de sessão, launcher/atalhos e integração à interface Linux; múltiplas janelas nativas só quando comprovadas.
4. **Interoperabilidade aberta:** investigação documentada de comportamentos de fabricantes com REA, sem copiar código proprietário.

**Não confundir:** espelhamento, display virtual e desktop multijanela são três capacidades diferentes.

## Comece por aqui

- [Visão do produto e escopo](docs/product/PRD.md)
- [Arquitetura proposta](docs/architecture/system.md)
- [Prova SPIKE-001 no Redmi](docs/research/SPIKE-001.md)
- [Plano de investigação REA](docs/research/rea-plan.md)
- [Roadmap e gates](docs/planning/roadmap.md)
- [Backlog, milestones e épicos](docs/planning/backlog.md)
- [Estratégia de teste](docs/testing/strategy.md)
- [Guia dos agentes](AGENTS.md)
- [Contribuição](CONTRIBUTING.md) e [segurança](SECURITY.md)

## Ambiente e limites

- Destino primário: desktop Linux (distribuições e servidores gráficos a validar).
- Dispositivo de referência inicial: Redmi Note 13 4G com HyperOS, sem root.
- Transporte inicial: scrcpy executado como subprocesso; ADB controla descoberta e autorização.
- Stack **proposta, sujeita a ADR e prova:** Qt 6/QML (UI), Rust (núcleo de sessão), scrcpy e ADB (backends).
- **Sem código de produto, sem CI funcional e sem release neste momento.** Não marcar tarefas concluídas com base apenas em documentação.

## Referências oficiais

- [scrcpy](https://github.com/Genymobile/scrcpy)
- [Multi-Display Android AOSP](https://source.android.com/docs/core/display/multi_display)
- [REA](https://github.com/morluto/rea)

**Licença:** escolha e arquivo LICENSE pendentes de decisão registrada no backlog. Repositório público não equivale, por si só, a uma licença open source.
