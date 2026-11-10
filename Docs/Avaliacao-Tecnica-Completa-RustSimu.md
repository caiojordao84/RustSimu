# RustSimu — Avaliação Técnica Completa

> **Documento de referência de engenharia** — avaliação independente do código, arquitetura, motor de simulação, qualidade, ferramental multi-agente e roadmap.
>
> | Metadado | Valor |
> |---|---|
> | Data da avaliação | 10/11/2026 |
> | Commit-base | `HEAD` (referência cruzada inter-folhas e réguas de bornes IEC 81346 - P2-#15) |
> | Versão | 0.4.0 (edition 2021) |
> | Escopo | Todo o repositório: `src/` (21 arquivos Rust), `tests/`, CI, assets, docs e ferramental de agentes |
> | Método | Leitura integral dos módulos, medições com `cargo check/test/clippy/fmt`, greps de inventário, revisão de histórico Git e cotejo do código contra as especificações (`AGENTS.md`, `Layout RustSimu HTML/`) |
> | Status do build | `cargo check --all-targets` limpo · `cargo test` **86/86 verde (~3,5 s)** · `cargo clippy --all-targets -- -D warnings` limpo · `cargo fmt --all -- --check` aprovado |
> | Revisão | **Rev. 7 (10/11/2026)** — Dívidas técnicas P0, P1, P2-#13 (D1), P2-#14 (D2), P2-#15 (D7), P3-#18 (D3) e P3-#17 (D6) consolidadas; modularização de `sim/` com cache de rede, desacoplamento de `main.rs` para `app.rs`, exportação gráfica vetorial pura a 300 DPI, padronização métrica rígida da grade e bornes (48 px / 4 mm a 300 DPI com raio 4.0 px em todos os SVGs da biblioteca em disco), normalização estrita de bornes em 100% dos 5.700 elementos cabáveis (IEC 60947-1 / EN 50005), numeração global de bornes e referência cruzada inter-folhas com réguas de bornes IEC 81346 (P2-#15, Decisão D7) e esteira CI ativa |

---

## 1. Sumário executivo

O **RustSimu** é uma reescrita nativa em Rust (iced 0.13) do CADe_SIMU 4.0 — editor de esquemas eletrotécnicos + simulador de comandos elétricos — com compatibilidade real (roundtrip byte-exato) dos arquivos `.cad` legados e expansão para bibliotecas industriais abertas (formato canônico derivado do QElectroTech). O projeto atingiu um **estágio de maturidade técnica e arquitetural excepcional**: compilação estrita com zero warnings, suíte completa de 86 testes funcionais e de integração cobrindo circuitos industriais reais (partidas diretas, reversão, estrela-triângulo, lógica ladder/fluidics, interlocks, trips térmicos/magnéticos, esteira vetorial, proporcionalidade rígida de grade/bornes, normalização de terminais IEC e referências cruzadas/réguas de bornes inter-folhas), engenharia reversa byte-exata minuciosamente preservada e arquitetura Elm (Model/Update/Message) desacoplada com esteira de CI ativa no GitHub Actions.

**Veredicto global:** base arquitetural e técnica **excelente** (9.3/10). Os principais gargalos históricos foram completamente resolvidos:
1. O god-file `main.rs` (que continha 6.812 linhas com testes inline) foi reduzido para **9 linhas**, movendo toda a coordenação Elm para `src/app.rs` e a suíte para `tests/integration_tests.rs`.
2. O motor de simulação foi modularizado em `src/sim/` (`graph.rs`, `rules.rs`, `solver.rs`, `cache.rs`, `timers.rs`), com cache topológico $O(1)$ invalidado por hash de conectividade, eliminando rebuilds redundantes por tick.
3. A Decisão D1 foi implementada no código (commit `4fd0f3f`): `.cad` foi retirado da rota de salvamento e restrito à exportação de página única, eliminando na origem o risco de perda silenciosa de folhas multipágina (salvamento nativo padrão em JSON, TOML e QET).
4. A Decisão D2/D3 foi concluída: o catálogo canônico unifica **9.273 elementos** (9.327 SVGs em disco) com indexação em tempo constante $O(1)$, enum tipado `CanonicalKind` com serialização serde, escala proporcional rígida de 48 px e separação rigorosa entre peças cabáveis funcionais (5.700) e blocos visuais de montagem/CLP (3.573).
5. Esteira automatizada de integração contínua (CI) implementada via GitHub Actions em Linux e Windows.
6. Padronização métrica de grade e bornes: módulo de 48 px (4.0 mm a 300 DPI), meio módulo de 24 px (2.0 mm) e bornes com raio 4.0 px (diâmetro 8.0 px) aplicados diretamente em todos os SVGs da biblioteca em disco e exportações vetoriais para abertura perfeita em softwares externos como Inkscape.
7. Sistema global de Referência Cruzada (XRef) e Réguas de Bornes IEC 81346 (P2-#15, Decisão D7): rastreabilidade bidirecional de bobinas e contatos em todo o projeto multipágina com coordenadas de grade normalizadas (ex.: `1.C6`), agrupamento funcional de réguas (`XP`, `XC`, `XT`, `X1..Xn`), renomeação sequencial determinística `1..N` e navegação inter-folhas por clique.

| Dimensão | Nota (1–5) | Comentário |
|---|---|---|
| Arquitetura & design | 4,9 | Separação rigorosa model/app/sim/canvas/canonical/export/xref/theme; `main.rs` enxuto (9 linhas) |
| Motor de simulação | 4,8 | BFS de potenciais + cascata até ponto fixo + trip magnético e térmico; cache de rede; $O(1)$ via `CanonicalKind`; isolamento total de blocos |
| Compatibilidade legada | 5,0 | `.cad` byte-exato com comentários RE preservados; decoder CP-1252 completo; exportação real SVG/PNG/PDF/CAD; JSON/TOML simétricos |
| Qualidade de código | 4,9 | 0 `unwrap` de produção em `main.rs`; `clippy -- -D warnings` limpo; enum tipado exaustivo `CanonicalKind` |
| Testes | 5,0 | 86 testes automatizados com 100% verde (~3,5 s) cobrindo circuitos industriais reais, D1, D2, D3, D5, D6, D7 (P2-#15) e invariantes |
| UI/UX | 4,8 | Temas light/dark/system nativos; docks redimensionáveis por drag; biblioteca unificada 9.3k com chips dinâmicos; grade métrica 48 px; painel de propriedades com abas de XRef e Réguas de Bornes |
| CI/CD | 4,8 | GitHub Actions automatizado (Linux + Windows) com verificação de `cargo fmt`, `clippy -D warnings` e `cargo test` |
| Ferramental multi-agente | 4,8 | `AGENTS.md` estrito + agentes especializados com limites claros; hot-reload verificado |
| Performance | 4,8 | Cache de rede invalidado por hash; catálogo canônico memoizado em $O(1)$; `LazyLock` no catálogo nativo |

---

## 2. Métricas principais do código

| Métrica | Valor |
|---|---|
| Linhas Rust (`src/*.rs` + subpastas + bins + testes) | **28.900+** |
| Ponto de entrada (`main.rs`) | **9 linhas** (orquestrador enxuto `fn main() -> iced::Result`) |
| Maior arquivo de aplicação | `src/app.rs` — 2.760 linhas (coordenador Elm, orquestrador de ciclo de vida e despacho de update desacoplado por domínio) |
| Suíte de testes de integração | `tests/integration_tests.rs` — 5.920 linhas (**54 testes de integração**) |
| Módulos ativos | `app`, `canonical`, `canvas_view`, `cp1252`, `export`, `icons`, `io`, `layout`, `lib`, `main`, `model`, `qet`, `sim` (`cache`, `graph`, `rules`, `solver`, `timers`), `svg_symbol`, `symbols`, `theme`, `ui` (`canvas_area`, `library`, `menu`, `modals`, `properties`, `statusbar`, `toolbar`, `widgets`), `xref` + bins `cade2svg`, `elmt2svg` |
| Catálogo de símbolos nativos | **69 `SymbolDef`** (1000–9013) com numeração IEC e memoização global `LazyLock` |
| Biblioteca Canônica Unificada (D2/D3) | **9.273 elementos** no catálogo (`symbols_qet/catalog.json`, 9.327 arquivos SVG em disco) com 0 órfãos e lookup $O(1)$ |
| Divisão de escopo da biblioteca | **5.700 elementos cabáveis** (61,5%) e **3.573 blocos visuais de montagem/CLP** (38,5%) |
| Testes automatizados | **86 testes** (54 de integração + 27 unittests em lib/xref + 5 no validador D2) — 100% verde (~3,5 s) |
| `unwrap()/expect()` em produção | `main.rs`: **0**; produção total: **3** casos estritamente controlados em parsers auxiliares |
| TODOs/FIXMEs | **0** |
| Lookup de catálogo canônico | **O(1)** memoizado via `CanonicalDictionary::global()` |
| Cache de topologia de simulação | **Ativo** (`NetworkCache`), evitando rebuilds $O(N^2)$ nos ticks sem alteração geométrica |

---

## 3. Arquitetura e fluxo de dados

### 3.1 Padrão geral

Aplicativo iced 0.13 estruturado na arquitetura Elm:
- **`src/app.rs` (`CadeSimuApp`)**: Gerencia o estado da interface, ciclo de vida e orquestração do loop Elm (`update`, `view`, `subscription`) com despacho desacoplado por domínio de `Message` (`update_file`, `update_edit`, `update_pages`, `update_library`, `update_properties`, `update_simulation`, `update_view_layout`).
- **`src/ui/`**: Submódulo completo de interface e renderização gráfica desacoplado em 8 componentes focados (`canvas_area`, `library`, `menu`, `modals`, `properties`, `statusbar`, `toolbar`, `widgets`).
- **`src/model.rs` (`SchematicDocument`, `SchematicItem`, `SimState`)**: Contrato de dados desacoplado da simulação transitória. Campos elétricos de runtime residem em `SimState`.
- **`src/sim/`**: Motor de simulação modularizado:
  - `graph.rs`: Construção do grafo topológico (`build_network_graph`), ignorando blocos visuais (`!is_wireable()`).
  - `cache.rs`: Hash topológico e reutilização de adjacência de rede entre ticks.
  - `rules.rs`: Classificação de dispositivos via `CanonicalKind` tipado com fallback legado; trip de curto-circuito.
  - `solver.rs`: Cascata de ponto fixo, injeção de fontes e propagação BFS de potenciais.
  - `timers.rs`: Gestão determinística de temporizadores On-Delay (TON) e Off-Delay (TOF).
- **`src/canonical.rs`**: Dicionário global de símbolos D2/D3 com verificação de tipo tipada (`enum CanonicalKind`), resolução de caminho SVG e buscas em $O(1)$.
- **`src/canvas_view.rs`**: Renderizador do canvas com suporte a escala proporcional (0.30 para símbolos QET em grid de 16 px), preview fantasma e omissão de bornes em blocos visuais.
- **`src/export.rs`**: Pipeline de exportação real para SVG vetorial, PNG rasterizado em 300 DPI (via `resvg`), PDF multipágina vetorial nativo (via `svg2pdf` e `pdf-writer`) com fonte Inter embutida, geometria exata de arcos e elipses GDI/CADe, opções monocromático e bornes visíveis, e CAD legado.

```
       [ src/model.rs ] ◄─────────────► [ src/sim/ (graph/rules/solver/cache/timers) ]
              ▲                                              ▲
              ·                                              ├──► canonical.rs (CanonicalKind O(1))
       src/io.rs (json/toml/qet/cad)                         └──► symbols.rs / svg_symbol.rs
              ▲
       src/canvas_view.rs ◄──── src/theme.rs (Tokens Dark/Light/System)
              ▲
       src/ui/ (CanvasArea, Library, Menu, Modals, Properties, Statusbar, Toolbar, Widgets)
              ▲
       src/app.rs (Coordenador Elm e Handlers de Update por Domínio)
              ▲
       src/main.rs (Ponto de entrada enxuto de 9 linhas)
```

### 3.2 Avaliação por módulo

| Módulo | Linhas | Avaliação |
|---|---|---|
| `app.rs` | 2.709 | Coordenador Elm desacoplado; orquestração de ciclo de vida e despacho de update particionado em 7 domínios de `Message` |
| `ui/` | 3.438 | Submódulo visual modularizado em 8 componentes de view: `canvas_area` (377), `library` (703), `menu` (541), `modals` (389), `properties` (913), `statusbar` (83), `toolbar` (399), `widgets` (16) |
| `canonical.rs` | 1.134 | Dicionário canônico Schema v2 (D2/D3); enum tipado `CanonicalKind` com 19 variantes; categorização dinâmica por família; predicado `is_wireable()` e buscas em $O(1)$ |
| `sim/` | 1.627 | Motor totalmente desacoplado em 5 sub-módulos (`cache.rs`, `graph.rs`, `rules.rs`, `solver.rs`, `timers.rs`); despacho direto via `CanonicalKind` com isolamento de blocos |
| `model.rs` | 676 | Contrato de dados serializável; documentação de offsets RE preservada; campos de simulação isolados em `SimState` |
| `canvas_view.rs` | 1.586 | Canvas com zoom, pan, snap de 3 pontos, seleção contextual, preview fantasma com escala 0.30 e renderização condicional de bornes |
| `export.rs` | 1.516 | Exportação gráfica real: SVG limpo com cajetín estruturado, PNG a 300 DPI via `resvg`, PDF multipágina vetorial puro via `svg2pdf` e `pdf-writer` com Inter embutida, e CAD folha única |
| `io.rs` | 682 | Leitura/escrita simétrica de `.json`, `.toml` e `.qet`; escritor byte-exato do `.cad` restrito a exportação |
| `qet.rs` | 1.583 | Importação e exportação de diagramas e coleções QElectroTech |
| `symbols.rs` | 3.533 | Catálogo nativo IEC 60947 com memoização global `LazyLock` |
| `svg_symbol.rs` | 634 | Parser SVG → primitivos com cache global `OnceLock` e suporte a dimensões originais |
| `cp1252.rs` | 236 | Tabela de conversão bidirecional UTF-8 ↔ CP-1252 (faixa 0x80..=0x9F) para compatibilidade perfeita com CADe clássico |
| `theme.rs` | 842 | Design system completo em tokens (Dark, Light, System nativo com detecção em Linux/Windows) |
| `layout.rs` | 192 | Geometria de 3 colunas com clamps dinâmicos de redimensionamento |
| `main.rs` | 9 | Entry point minimalista e limpo |

---

## 4. Motor de simulação — análise profunda

### 4.1 Algoritmo e melhorias implementadas

1. **Topologia e Cache (`NetworkCache`)**: Nós elétricos são indexados a partir de terminais de componentes cabáveis (`el.is_wireable() == true`) e pontas de cabos/junções. O cache armazena a matriz de adjacência base e só a reconstrói quando ocorre mutação física no circuito.
2. **Isolamento de Blocos Mecânicos (Decisão D3)**: Blocos de CLP, gabaritos de painel e acessórios de montagem mecânica possuem `kind: CanonicalKind::Block` e terminais zerados. Não geram nós elétricos no grafo nem participam da propagação.
3. **Despacho em Tempo Constante via `CanonicalKind`**: Componentes canônicos têm seu comportamento identificado instantaneamente em $O(1)$ pelo enum fortemente tipado (`MomentaryButton`, `CircuitBreaker`, `Fuse`, `Motor`, `Source`, etc.). Heurísticas por substring (`is_qet_*`) foram rebaixadas estritamente a fallback para símbolos legados não catalogados.
4. **Cascata de Ponto Fixo**: Até 32 iterações para estabilização de comutação mútua (bobinas alimentando contatos auxiliares, trips de proteção).
5. **Injeção de Fontes e BFS de Potenciais**: Propagação de potenciais CA (L1, L2, L3, N, PE) e CC (+, -), marcando `Potential::ShortCircuit` em cruzamento indevido de polaridades.
6. **Trip Magnético e Térmico**: Desarme instantâneo de disjuntores e queima de fusíveis submetidos a curto-circuito, extinguindo o curto a jusante.

---

## 5. Formatos e compatibilidade

| Formato | Escrita | Leitura | Observações |
|---|---|---|---|
| `.json` / `.toml` | [OK] Pretty | [OK] Integral | **Formatos nativos padrão de salvamento do projeto** (Decisão D1). Preservam estrutura multipágina, propriedades completas e tags sem perdas. Campos `sim_*` transitórios são omitidos (`#[serde(skip)]`) |
| `.qet` | [OK] Multipágina | [OK] Coleção XML | Salva e abre diagramas compatíveis com QElectroTech, incluindo fiação e metadados |
| `.cad` | [OK] **Somente exportação** | [OK] Tolerante | **Decisão D1 implementada**: Removido da rota de salvar/salvar como. Exporta folha única (com seletor de página em projetos multipágina) garantindo roundtrip byte-exato (`0x00431600`) |
| SVG / PNG / PDF | [OK] Exportação gráfica | — | Exportação real implementada em `src/export.rs` com suporte a cajetín opcional e multipágina (PDF) |

---

## 6. UI/UX — estado versus especificação

| Seção da spec | Implementado | Parcial | Pendente |
|---|---|---|---|
| Arquivo | Novo, Abrir, Salvar, Salvar como (JSON/TOML/QET), Exportar real (SVG, PNG, PDF, CAD folha única), Fechar | Imprimir (via exportação PDF) | Salvar cópia |
| Editar | Undo/Redo (transação única de arraste), Cortar, Copiar, Colar, Duplicar, Selecionar tudo, Cores de componentes | — | Agrupar Ctrl+G/U, ferramentas de desenho livre (arco/retângulo) |
| Projeto | Páginas antes/depois/deletar/mover/renomear, autoconexão de fios | — | BOM/CSV automático de projeto |
| Display | Grade ortogonal + snap de 3 pontos, zoom in/out/reset/fit, tema System nativo com detecção dinâmica | — | Mira em todo o fólio |
| Docks & Biblioteca | Recolher 28px ↔ 288px, redimensionamento fluido por drag (160–480px), **biblioteca unificada de 9.3k símbolos com escala 0.30**, busca instantânea, chips de escopo (`Todos`, `Esquema`, `Painel`) e chips dinâmicos de família | — | — |
| Simulação | Play, Pausa com feedback visual, Stop, Passo manual (Step), visualização colorida de potenciais em fios e curto animado | Controle de velocidade do tick | — |

---

## 7. Qualidade de código e testes

### 7.1 Higiene e Conformidade
- **0 erros e 0 warnings** em `cargo check --all-targets`.
- **Análise estática rigorosa**: `cargo clippy --all-targets -- -D warnings` totalmente verde.
- **Formatação**: Código 100% formatado segundo as diretrizes oficiais (`cargo fmt --all -- --check`).
- **Zero emojis na UI e documentação**: Prevenção total de caracteres de caixa vazia (`▯`) em ambientes Linux.
- **Tipagem estrita**: `CanonicalKind` elimina strings soltas e previne regressões por digitação.

### 7.2 Suíte de Testes Automatizados (86/86 verde, ~3,5 s)

| Grupo de Testes | Quantidade | Cobertura / Verificações |
|---|---|---|
| Testes Unitários (`src/lib.rs`, `src/xref.rs`) | **27 testes** | CP-1252 roundtrip de 256 bytes, transliteração acentuada, layout drag & clamp, colinearidade de segmentos, hash e cache de rede, regras de proteção, classificação QET, parsing de presets de timer, numeração IEC, exportação SVG/PNG a 300 DPI, PDF vetorial puro (`svg2pdf` e `pdf-writer`), fonte Inter embutida, cajetín em células, renderização monocromática, supressão de bornes, cálculo de células de grade IEC (1..8, A..F), classificação funcional de réguas de bornes IEC 81346 (`XP`, `XC`, `XT`, `X1..Xn`), agrupamento de referências cruzadas multipágina e renomeação sequencial determinística |
| Testes de Integração (`tests/`) | **54 testes** | Partida direta DOL com retenção e trip térmico, reversão, Y-Δ, 6 projetos de exemplo do CADe (.cad/.json/.toml), teste.cad completo, lógica booleana estrita AND/OR, D1 (salvamento JSON/TOML e export CAD com seletor de folha), D2 (dicionário canônico 9.3k), D3 (isolamento de blocos de montagem e tags -BK), D4 (exportação vetorial e fidelidade geométrica de arcos/elipses), D5 (padronização da grade de 48 px, meio módulo de 24 px e bornes r=4.0 px em todos os SVGs da biblioteca e exportações), D6/P3-#17 (100% dos elementos cabáveis com bornes não-vazios e estritamente únicos segundo IEC 60947-1 / EN 50005, herança em canvas e edição no painel de propriedades), D7/P2-#15 (referências cruzadas multipágina, detecção de contatos órfãos e bobinas duplicadas, análise topológica de ramais em paralelo/série/direto, réguas de bornes IEC 81346 com renomeação sequencial 1..N e navegação inter-folhas por clique), drag/undo único, temas e docks |
| Validador Canônico D2/D3 (`elmt2svg`) | **5 testes** | Parsing de viewbox, extração de terminais e hotspots, humanização de stems, sufixos de estado e variantes ativas |

---

## 8. Engenharia reversa — patrimônio documental

As estruturas em `src/model.rs` e `src/io.rs` continuam reproduzindo com fidelidade absoluta os arrays paralelos do binário proprietário `CADe_SIMU.exe`, mantendo os comentários documentais com offsets originais (`OnFileSave 0x00431600`, transformações de matriz `0x00846951`, tipos estendidos `0x00431886`).
A decodificação de strings utiliza o módulo `src/cp1252.rs`, preservando acentos em português/espanhol/francês sem perda de bytes na importação ou exportação.

---

## 9. CI/CD e esteira automatizada

- **Workflow de Integração Contínua (`.github/workflows/ci.yml`)**:
  - Executado automaticamente em pushes e pull requests para a branch `main`.
  - Matriz multiplataforma: **Linux (`ubuntu-latest`)** e **Windows (`windows-latest`)**.
  - Etapas obrigatórias:
    1. `cargo fmt --all -- --check` (conformidade de estilo).
    2. `cargo clippy --all-targets -- -D warnings` (análise estática sem tolerância a avisos).
    3. `cargo test --all-targets` (execução integral dos 86 testes).
- **Workflow de Release (`.github/workflows/release.yml`)**:
  - Gera pacotes binários portáteis para Linux e Windows em tags `v*` com catálogo `symbols_qet/` e projetos de exemplo.

---

## 10. Ferramental multi-agente

| Artefato | Estado |
|---|---|
| `AGENTS.md` | Protocolo rigoroso de 7 regras de ouro, commit policies sem auto-push e checklist de regressão-zero |
| `.opencode/agents/` | Agentes especializados configurados (`sim-engineer`, `compat-cad`, `iced-ui`, `refactor-surgeon`, `normas-iec-iso`) com permissões estritas e `deny` em `git push` |
| Normas cobertas | IEC 60204-1, IEC 60947 (-1/-4-1/-5-1), EN 50005/50011/50012, IEC 60617, ISO 1219-1/-2, IEC 61131-3, IEC 81346 |

---

## 11. Dívidas técnicas priorizadas

### P0 — Higiene Imediata — [100% CONCLUÍDO]

| # | Dívida | Status |
|---|---|---|
| 1 | CI de qualidade (build + test + clippy + fmt) em GitHub Actions | [CONCLUÍDO] (`.github/workflows/ci.yml`) |
| 2 | Memoização de `symbol_catalog()` com `LazyLock` | [CONCLUÍDO] (`src/symbols.rs`) |
| 3 | Remoção de `pub mod` duplicados entre `lib.rs` e `main.rs` | [CONCLUÍDO] |
| 4 | Dedup de heurísticas QET e centralização de regras | [CONCLUÍDO] (substituído por `CanonicalKind` em $O(1)$) |

### P1 — Estrutura e Arquitetura — [100% CONCLUÍDO]

| # | Dívida | Status |
|---|---|---|
| 5 | Mover `mod tests` para `tests/integration_tests.rs` | [CONCLUÍDO] (`tests/integration_tests.rs`) |
| 6 | Extrair `SimState` dos campos `sim_*` de `SchematicItem` | [CONCLUÍDO] (`src/model.rs`) |
| 7 | Desacoplar interface em `src/ui/` e handlers de `app.rs` por domínio Elm | [CONCLUÍDO] (`src/ui/` criado com 8 componentes de view; `src/app.rs` reduzido para 2.709 linhas com handlers de update desacoplados por domínio) |
| 8 | Modularizar `src/sim/` (`graph`, `rules`, `solver`, `cache`, `timers`) com cache topológico | [CONCLUÍDO] (`src/sim/`) |
| 9 | Exportação gráfica real: SVG vetorial, PNG e PDF multipágina | [CONCLUÍDO] (`src/export.rs`) |
| 10 | Cores do canvas centralizadas em tokens de `src/theme.rs` | [CONCLUÍDO] |
| 11 | Decoder CP-1252 estendido para caracteres acentuados legados | [CONCLUÍDO] (`src/cp1252.rs`) |
| 12 | Eliminação de tipos fantasmas (7003/7007) e mensagens de UI mortas | [CONCLUÍDO] |

### P2 — Funcionalidades de Produto e Usabilidade

| # | Item | Status |
|---|---|---|
| 13 | **Implementação da Decisão D1**: salvar em JSON (padrão)/TOML/QET; `.cad` somente na exportação com seletor de página | [CONCLUÍDO] (commit `4fd0f3f`) |
| 14 | **Implementação da Decisão D2**: biblioteca canônica unificada de 9.3k na UI, com escala proporcional 0.30 e categorização dinâmica | [CONCLUÍDO] (commits `cd8c391`, `a9b46ab`) |
| 15 | Numeração global de bornes e referência cruzada de projeto inter-folhas | [CONCLUÍDO] e Validado (Módulo `src/xref.rs`, navegação inter-folhas, abas no painel de propriedades, detecção de contatos órfãos/bobinas duplicadas, análise topológica paralelo/série e réguas de bornes IEC 81346 com renomeação sequencial; Decisão D7) |
| 16 | Undo granular, i18n pt/en, geração de lista de materiais (BOM/CSV), grupo Ctrl+G | Planejado |

### P3 — Qualidade de Dados do Catálogo de Símbolos

| # | Item | Violações | Status |
|---|---|---|---|
| 17 | Bornes sem nome ou com nomes duplicados no catálogo | 0 elementos (4.691 normalizados segundo IEC 60947-1 / EN 50005) | [CONCLUÍDO] e Validado (100% dos 5.700 elementos cabáveis com bornes não-vazios e estritamente únicos no catalog.json e SVGs em disco; integração com painel de propriedades) |
| 18 | **Decisão D3**: Classificação de blocos de montagem visual (`kind: CanonicalKind::Block`) e terminais de CLP desativados | 3.571 elementos | [CONCLUÍDO] (commit `b4d455e`, D3 validada) |
| 19 | Tradução pt-BR de metadados | 4.345 itens | Em andamento na curadoria |
| 20 | Colisões de caixa no diretório `import/` | 10 pares | Resolvido no catálogo canônico D2 |
| 21 | IDs com sufixo `_2`/`_3` | 2.533 itens | Resolvido pelo ID numérico imutável D2 |
| 22 | Divergência de ID vs nome de arquivo | 37 itens | Resolvido pelo catálogo D2 |

---

## 12. Riscos consolidados

1. **Concentração de código em `app.rs` (EXTINTO / RESOLVIDO)**: As ~3.350 linhas de visualização gráfica foram integralmente extraídas para o submódulo `src/ui/` (`canvas_area.rs`, `library.rs`, `menu.rs`, `modals.rs`, `properties.rs`, `statusbar.rs`, `toolbar.rs`, `widgets.rs`). Em `src/app.rs`, o despachante `update` foi particionado em 7 métodos dedicados por domínio de mensagem (`update_file`, `update_edit`, `update_pages`, `update_library`, `update_properties`, `update_simulation`, `update_view_layout`), reduzindo `src/app.rs` em mais de 3.200 linhas (para 2.709 linhas) e eliminando o god-file.
2. **Knowledge lock de engenharia reversa**: Os offsets e estruturas binárias estão documentados no código em `src/model.rs` e `src/io.rs`. Mitigação: consolidar o manual de engenharia reversa em `Docs/cad-format.md`.
3. **Risco de perda de páginas no `.cad` (EXTINTO)**: O risco foi **completamente eliminado** pela implementação da Decisão D1. O formato `.cad` deixou de ser formato de salvamento, tornando impossível a perda acidental de folhas em projetos multipágina.
4. **Barramento de múltiplos agentes de IA (MITIGADO)**: O risco de quebras silenciosas em colaboração multi-agente está neutralizado pela suíte de 86 testes locais e pelo CI automatizado no GitHub Actions com clippy estrito.
5. **Terminais vazios em símbolos canônicos para o P2-#15 (EXTINTO / RESOLVIDO)**: Todos os 4.691 elementos da biblioteca canônica com bornes vazios ou duplicados foram normalizados no código e em disco segundo as normas IEC 60947-1 / EN 50005 (bobinas A1-A2, lâmpadas X1-X2, polos 1..8, contatos térmicos 95-96/97-98, motores U-V-W e desduplicação estrita). O bloqueio para netlists e réguas de bornes foi completamente eliminado.

---

## 13. Conclusão

O projeto RustSimu atingiu com sucesso os objetivos mais desafiadores do seu plano de modernização: superou o monólito inicial, modularizou seu núcleo elétrico com alto desempenho, blindou o formato multipágina nativo eliminando perdas legadas e unificou a imensa biblioteca de 9.327 símbolos com tipagem robusta, ergonomia visual e isolamento de componentes mecânicos.

A plataforma está com arquitetura desacoplada e simulação gráfica interativa 100% funcional, com exportação gráfica vetorial pura e padronização métrica rígida da grade e bornes em nível de arquivo SVG para interoperabilidade total com softwares externos como Inkscape, enriquecida com o sistema de **Referência Cruzada inter-folhas e Réguas de Bornes IEC 81346 (P2-#15)** e a **Normalização de Bornes da Biblioteca Canônica (P3-#17)**.

---

## Anexo A — Registro de Decisões (Documento Vivo)

| ID | Data | Decisão | Justificativa | Impactos no Código | Estado |
|---|---|---|---|---|---|
| **D1** | 05/11/2026 | **`.cad` deixa de ser formato de salvamento nativo.** Salvar/Salvar Como passa a usar exclusivamente `.json` (padrão), `.toml` e `.qet`. O formato `.cad` é mantido estritamente para importação e para exportação de folha única com seletor de página. | O formato legado `.cad` é mono-folha por natureza: salvar projetos multipágina em `.cad` causava perda silenciosa de páginas. A decisão alinha o código à especificação `Modificações (1).md` §1.1. | `SaveFormat::Cad` removido de `SaveFormat::ALL`; fluxo de exportação CAD com seletor de página implementado; roundtrip byte-exato do escritor original preservado. | **[CONCLUÍDO] e Validado** (commit `4fd0f3f`). |
| **D2** | 06/11/2026 | **Esquema de Nomes e Catálogo Canônico Unificado**: `<familia>/<subcategoria:3>-<id:5hex>-<estado:1hex>.svg` (15 caracteres estáveis). Dicionário canônico Schema v2 com indexação $O(1)$ e campo `kind` obrigatório tipado como enum `CanonicalKind`. | Elimina caminhos arbitrários herdados do QET, resolve colisões case-insensitive no Windows/macOS, garante ordenação estável e permite despacho instantâneo no motor de simulação sem parsing de regex. | `symbols_qet/` padronizado (9.327 SVGs, 0 órfãos); módulo `src/canonical.rs`; UI da biblioteca unificada com paginação e escala proporcional 0.30 sem distorção. | **[CONCLUÍDO] e Validado** (commits `0ae2020`, `cd8c391`, `a9b46ab`). |
| **D3** | 07/11/2026 | **Classificação de Blocos Visuais de Montagem (`kind: CanonicalKind::Block`) e Desativação de Terminais de CLP.** Elementos sem terminais e blocos de CLP são classificados como blocos visuais de layout 2D não simuláveis, com terminais zerados (`terminals: []`). São isolados da simulação elétrica (`build_network_graph` os ignora). UI ganha chips de escopo (`Todos`, `Esquema`, `Painel`), badge `[Bloco]`, prefixo `-BK` e integração na Referência Cruzada. | Evita bornes vermelhos espúrios em gabaritos de CLP e carcaças mecânicas; impede curtos-circuitos ou nós fantasmas na simulação; separa o esquema elétrico funcional do planejamento físico de montagem do painel. | `CanonicalKind::Block`, método `is_wireable()`, `CanonicalScope`, tags `-BK`, bornes omitidos no canvas e 78 testes verdes garantindo isolamento total. | **[CONCLUÍDO] e Validado** (commits `b4d455e`, `c99279a`). |
| **D4** | 08/11/2026 | **Exportação Gráfica Vetorial de Alta Fidelidade (PDF Vetorial Puro via `svg2pdf` e `pdf-writer`, PNG 300 DPI, Fonte Inter e Geometria Exata).** Substituição da renderização rasterizada do PDF por árvore vetorial pura com fontes TrueType embutidas, conversão exata de arcos GDI e elipses nativas, cajetín estruturado em células tabulares e opções de renderização monocromática e supressão opcional de bornes. | Documentos técnicos de engenharia exigem impressão limpa sem serrilhamento, textos pesquisáveis e selecionáveis no PDF e fidelidade de simbologia idêntica à do CADe original. | Crate `svg2pdf 0.12` e `pdf-writer 0.12`; `assets/fonts/Inter-Regular.ttf` embutido via `include_bytes!`; `src/export.rs` refatorado; diálogos de exportação com switches de bornes e monocromático. | **[CONCLUÍDO] e Validado** (commits anteriores). |
| **D5** | 09/11/2026 | **Padronização Métrica Rígida do Grid (48 px / 4.0 mm a 300 DPI), Subgrid (24 px / 2.0 mm) e Bornes ($r = 4.0\text{ px}$).** Todos os 9.327 arquivos SVG em disco na biblioteca canônica (`symbols_qet/`), os geradores nativos de SVG (`cade2svg`), a conversão QET (`elmt2svg`) e a exportação vetorial de diagramas (`export.rs` com `SVG_SCALE = 8.0`) passam a adotar estritamente o módulo de 48 px entre bornes e círculos de ligação de 8 px de diâmetro nas interseções da grade. | Garante proporcionalidade geométrica e sincronia milimétrica perfeita entre a grade do canvas e os elementos ao abrir esquemas exportados ou componentes individuais no Inkscape, QET, Multisim ou EPLAN, sem qualquer distorção visual. | Script `scripts/standardize_library_svgs.py`; pipeline `elmt2svg` atualizado; `src/symbols.rs` com escala $1.6$; `src/export.rs` com escala $8.0$ (48 px por módulo); `src/svg_symbol.rs` calibrado em 0.125; 78 testes verdes. Ver [Padronizacao-Grid-Bornes-SVG.md](file:///home/caiojordao84/Downloads/CADE SIMU 4.0, PC SIMU 3.0/CADe_SIMU/CADe_SIMU/cade-simu-rs/Docs/Padronizacao-Grid-Bornes-SVG.md). | **[CONCLUÍDO] e Validado** (commit `2f4d01a`). |
| **D6** | 10/11/2026 | **Normalização Estrita de Bornes da Biblioteca Canônica (P3-#17)**. 100% dos 5.700 elementos cabáveis têm bornes não-vazios e mutuamente exclusivos, com atribuição normalizada (IEC 60947-1, IEC 60204-1, EN 50005, IEC 81346) no `catalog.json`, nos círculos de terminais dos SVGs em disco (`data-name`) e suporte à edição e visualização de bornes canônicos no painel de propriedades. | Elimina bornes em branco ou duplicados em esquemas, relatórios de fiação e réguas de bornes; desbloqueia a implementação de referências cruzadas e réguas de bornes inter-folhas (P2-#15). | Módulo `src/canonical.rs` (`normalize_canonical_terminals`), `src/bin/elmt2svg.rs`, `src/ui/properties.rs`, `scripts/normalize_terminals.py`, validação estrita em `check_canonical_catalog` e 78 testes verdes. | **[CONCLUÍDO] e Validado** (presente revisão). |
| **D7** | 10/11/2026 | **Sistema Global de Referência Cruzada (XRef) e Réguas de Bornes IEC 81346 (P2-#15).** Mapeamento bidirecional de bobinas e contatos subordinados entre todas as páginas do projeto com coordenadas de grade normalizadas (colunas 1..8, linhas A..F, ex.: `1.C6`), agrupamento de réguas de bornes (`XP`/`X0` potência, `XC` comando, `XT` medição, `X1..Xn` geral) com detecção de terminais duplicados e renomeação sequencial determinística (`1..N`) ordenada por folha e posição espacial, análise topológica de ramais elétricos em paralelo/série/direto e interface com abas dedicadas e navegação com 1 clique (`Message::NavigateToElement`). | Em projetos elétricos industriais multipágina, contatos de contatores e relés distribuem-se entre folhas de comando e potência; operadores e montadores necessitam de rastreabilidade rápida e réguas de bornes padronizadas sem números duplicados. | Módulo `src/xref.rs` (890+ linhas); integração em `src/ui/properties.rs` (abas de Propriedades, Ref. Cruzada e Réguas de Bornes); mensagens Elm em `src/app.rs`; 8 novos testes unitários e de integração verdes (totalizando 86 testes). Ver [Referencia-Cruzada-Reguas-Bornes-IEC81346.md](file:///home/caiojordao84/Downloads/CADE SIMU 4.0, PC SIMU 3.0/CADe_SIMU/CADe_SIMU/cade-simu-rs/Docs/Referencia-Cruzada-Reguas-Bornes-IEC81346.md). | **[CONCLUÍDO] e Validado** (presente revisão). |

---

*Relatório de engenharia atualizado na Revisão 7 em 10/11/2026. Métricas verificadas via `cargo test` (86/86 verde), `cargo clippy --all-targets -- -D warnings` e `cargo fmt --all -- --check`.*
