# RustSimu — Instruções de Projeto, Regras Operacionais e Arquitetura

Este documento estabelece as regras e convenções obrigatórias para agentes de IA (**Antigravity**, **OpenCode** e outros) que desenvolvem no repositório **RustSimu**.
Toda e qualquer alteração deve respeitar rigorosamente as diretrizes abaixo descritas para garantir estabilidade, conformidade eletrotécnica e regressão zero.

---

## 1. Comandos Operacionais

* **Checagem estática de tipos, linter e formatação (Obrigatório sem warnings):**
  ```bash
  cargo check --all-targets
  cargo clippy --all-targets -- -D warnings
  cargo fmt --all -- --check
  ```
* **Suíte de testes (Obrigatório antes de qualquer commit):**
  ```bash
  cargo test --all-targets
  ```
  * A suíte conta com **78 testes automatizados** (100% verde):
    - `tests/integration_tests.rs` (50 testes de integração cobrindo simulação de circuitos reais, intertravamentos, térmicos, botoeiras, multicanais, multipágina, D1, D2, D3, D5, D6/P3-#17, esteira de exportação real e padrão rígido de grade/bornes).
    - `src/lib.rs` (23 unittests cobrindo `cp1252`, `layout`, `sim::cache`, `sim::graph`, `sim::rules`, `sim::solver`, `sim::timers`, `symbols` e `export` incluindo geometria exata de arcos/elipses e pipeline vetorial).
    - `src/bin/elmt2svg.rs` (5 unittests cobrindo indexação e normalização de SVGs).
    - As fixtures de teste utilizam esquemas reais em `Projetos/` (`.cad`, `.json`, `.toml`, `.qet`). Qualquer alteração em `sim/`, `symbols.rs`, `canonical.rs`, `io.rs`, `model.rs` ou `app.rs` deve manter essa suíte verde e, quando aplicável, acrescentar novos testes de regressão.
* **Execução do aplicativo em modo desenvolvimento ou release:**
  ```bash
  cargo run
  cargo run --release
  ```
* **Utilitários de Assets e Conversão:**
  ```bash
  cargo run --bin cade2svg   # Regenera SVGs nativos a partir dos assets legados
  cargo run --bin elmt2svg   # Processa a biblioteca QElectroTech e regenera catalog.json
  ```
  * *Nota:* O modo `elmt2svg --check` e `elmt2svg --index-only` valida a integridade do catálogo canônico D2 (`symbols_qet/catalog.json`) sem exigir a coleção externa instalada.

---

## 2. Arquitetura de Módulos e Fluxo de Dados

```text
       [ model.rs ] ◄─────────────► [ sim/ (cache/graph/rules/solver/timers) ]
            ▲                                              ▲
            ·                                              ├──► canonical.rs (CanonicalKind O(1))
       [  io.rs   ] ◄──────────────► [ symbols.rs / svg_symbol.rs ]
            ▲
    [ canvas_view.rs ] ◄──────────► [ layout.rs / theme.rs ]
            ▲
    [    src/ui/     ] (CanvasArea, Library, Menu, Modals, Properties, Statusbar, Toolbar, Widgets)
            ▲
    [    app.rs      ] (CadeSimuApp: Coordenador Elm e Handlers de Update por Domínio)
            ▲
    [    main.rs     ] (Ponto de entrada enxuto de 9 linhas)
```

* **`model.rs`**: Representação pura de dados do esquema (`SchematicDocument`, `SchematicItem`, condutores, bornes, nós de conexão, páginas e potenciais), além das constantes rígidas do grid métrico e bornes a 300 DPI (`GRID_MODULE_MM`, `GRID_MODULE_PX_300DPI`, `SUB_GRID_PX_300DPI`, `TERMINAL_DIAMETER_PX_300DPI`). Campos de runtime elétrico residem em `SimState`.
* **`sim/`**: Motor de simulação física e lógica desacoplado em 5 submódulos:
  - `graph.rs`: Grafo topológico (`build_network_graph`), conectando cabos e bornes por coordenadas geométricas `(x, y)` e ignorando blocos visuais (`!is_wireable()`).
  - `cache.rs`: Hash topológico e reaproveitamento de adjacência de rede entre passos de simulação.
  - `rules.rs`: Classificação funcional de dispositivos via enum tipado `CanonicalKind` com fallback legado; detecção de curto-circuito.
  - `solver.rs`: Cascata de ponto fixo, injeção de fontes e propagação BFS de potenciais.
  - `timers.rs`: Gestão determinística de temporizadores On-Delay (TON) e Off-Delay (TOF).
  - *Nota:* O acoplamento entre bobinas e contatos auxiliares é **exclusivamente por TAG**.
* **`canonical.rs`**: Dicionário global de símbolos D2/D3 com indexação $O(1)$, enum `CanonicalKind`, resolução de caminhos SVG e categorização dinâmica.
* **`canvas_view.rs`**: Renderização acelerada via GPU no Iced Canvas (`iced::widget::canvas`), emissão de mensagens de auto-conexão, *hit-test* com priorização de símbolos sobre fios, visualização fantasma (escala 0.30) e manipulação na grade (*grid snap*).
* **`symbols.rs` / `svg_symbol.rs`**: Catálogo nativo IEC 60947 com memoização global `LazyLock`, gerador e parser SVG padronizados na grade rígida de 48 px e bornes $r=4.0$ px, com escala isotrópica 0.125 (1:8) e normalização de bibliotecas externas.
* **`export.rs`**: Exportação gráfica real: SVG vetorial limpo padronizado a 8.0 px/unidade (48 px de pitch entre fases e $r=4.0$ nos bornes) com cajetín estruturado em células, PNG rasterizado a 300 DPI (via `resvg`), PDF multipágina vetorial nativo (via `svg2pdf` e `pdf-writer`) com fonte Inter embutida, geometria exata de arcos e elipses GDI/CADe, modos monocromático e com/sem bornes, e CAD folha única.
* **`io.rs`**: Leitura/escrita simétrica de formatos multipágina (`.json`, `.toml`, `.qet`) e escritor byte-exato do formato `.cad` legado (restrito à exportação).
* **`cp1252.rs`**: Conversão bidirecional CP-1252 ↔ UTF-8 para compatibilidade perfeita com strings de esquemas do CADe original.
* **`icons.rs`**: Renderização de ícones vetoriais modernos da interface em SVG.
* **`layout.rs` / `theme.rs`**: Sistema de colunas físicas com clamps de redimensionamento e design system com paletas semânticas Dark, Light e System nativo.
* **`src/ui/`**: Submódulo modularizado de interface gráfica:
  - `canvas_area.rs`: Viewport do esquemático, barra de folhas/páginas e cabeçalho do documento.
  - `library.rs`: Painel unificado da biblioteca canônica (9.3k) e coleções nativas.
  - `menu.rs`: Barra de menubar e menus flutuantes suspensos.
  - `modals.rs`: Modal de informações do RustSimu e modal universal de exportação (Decisão D1).
  - `properties.rs`: Dock de propriedades, conformidade IEC 60947, referências cruzadas e cajetín.
  - `statusbar.rs`: Rodapé de status, coordenadas, grelha, formato ativo e zoom.
  - `toolbar.rs`: Barra de ferramentas responsiva com controles de simulação e ferramentas.
  - `widgets.rs`: Utilitários compartilhados de interface (`app_tooltip`).
* **`app.rs`**: Estado do aplicativo (`CadeSimuApp`), enum `Message` e ciclo de vida Elm (`update`, `view`, `subscription`) com despacho particionado em 7 manipuladores por domínio (`update_file`, `update_edit`, `update_pages`, `update_library`, `update_properties`, `update_simulation`, `update_view_layout`).
* **`main.rs`**: Ponto de entrada minimalista de 9 linhas (`fn main() -> iced::Result { CadeSimuApp::run(...) }`).

---

## 3. Regras Eletrotécnicas Inegociáveis

1. **Lógica Booleana Estrita:**
   * **Ligação em Série = AND (`&&`):** A continuidade do potencial elétrico só se propaga se **todos** os elementos consecutivos do caminho estiverem conduzindo.
   * **Ligação em Paralelo = OR (`||`):** A corrente flui se **qualquer** um dos ramos paralelos estiver fechado.
   * *Nota:* Esta matriz booleana simplificada é a base unificada para comandos, potência trifásica e futuras expansões (Pneumática, Hidráulica e Ladder).
2. **Matriz de Potenciais Suportados:**
   * **Fases AC:** `Potential::L1`, `Potential::L2`, `Potential::L3`
   * **Neutro & Terra:** `Potential::N`, `Potential::PE`
   * **Corrente Contínua:** `Potential::DcPos` (CC+), `Potential::DcNeg` (CC-)
   * **Desenergizado:** `Potential::None`
   * **Estado de Curto-Circuito:** `Potential::ShortCircuit` (rótulo visual `"CC!"`; condutores sob curto são pintados em vermelho espesso). Conexão direta entre potenciais de polaridades opostas (ex.: L1 com N, ou L1 com L2) marca este estado, dispara a falta (*trip*) e **desarma disjuntores magnéticos** associados, além de **queimar fusíveis** no circuito afetado.
3. **Numeração de Bornes e Terminais (IEC 60947-1):**
   * **Contatos NA (Normalmente Abertos):** Terminações com dígitos `3` e `4` (ex.: `13-14`, `23-24`, `33-34`).
   * **Contatos NF (Normalmente Fechados):** Terminações com dígitos `1` e `2` (ex.: `11-12`, `21-22`, `31-32`).
   * **Contatos Comutadores / Reversores:** Terminações triplas padronizadas: `11` (Comum), `12` (NF) e `14` (NA).
   * **Bobinas de Contactores e Relés:** Estritamente identificadas como `A1` e `A2` (nunca herdam numeração de contatos).
   * **Relés Térmicos de Proteção:** Contatos auxiliares de disparo `95-96` (NF) e `97-98` (NA).
4. **Comportamento Dinâmico de Atuadores:**
   * **Pulsadores / Momentâneos:** Botoeiras simples, botões cogumelo de impulso e sensores de fim de curso (*limit switches*). Conduzem apenas enquanto o botão do mouse estiver pressionado sobre o elemento.
   * **Retentivos / Bi-estáveis:** Interruptores, chaves comutadoras rotativas e botões de emergência com trava. Alternam e retêm o estado a cada clique.
5. **Vínculo Unificado por TAG Identificadora:**
   * O acoplamento entre bobinas e seus contatos auxiliares ocorre pela TAG (ex.: a energização da bobina `-KM1` comuta simultaneamente todos os contatos `-KM1`).
   * O relé térmico principal (`-F1`) atua automaticamente sobre os contatos térmicos auxiliares `95-98` identificados como `-F1`.

---

## 4. Regras de Interface Gráfica e UX (Iced 0.13)

1. **Prioridade Absoluta de Camadas (Overlays vs. Canvas):**
   * Menus suspensos e janelas modais residem em camadas superiores (`Stack` / `layers`).
   * Quando um menu ou modal estiver ativo (`open_menu.is_some() || show_about_modal || show_export_modal`), o Canvas opera em estado bloqueado (`is_blocked: true`), ignorando eventos interativos.
   * Eventos de movimento espacial (`CanvasMessage::CursorMoved`) **nunca** devem fechar menus abertos no `update`.
2. **Hit-Testing com Prioridade para Símbolos:**
   * O clique do mouse em coordenadas onde símbolos e fios se cruzam **deve sempre selecionar o símbolo** (componente) e não o fio que passa por baixo.
3. **Layout Físico Responsivo:**
   * **Docks Laterais:** Largura padrão de `288px` quando expandidos (redimensionáveis entre `160px` e `480px` via separador de 6px) e `28px` quando recolhidos (com botão chevron centralizado em `align_x: Center`). O separador é desativado quando o dock correspondente está recolhido.
   * **Área Central (Canvas):** Expande dinamicamente para consumir todo o espaço liberado (`Length::Fill`).
   * **Toolbar:** As ferramentas à esquerda usam container com rolagem suave (`scrollable`), enquanto o grupo de simulação (**Play**, **Stop**, **Step**) fica fixado à direita com botões icônicos compactos (sem textos).
   * **Efeito Play:** O botão Play exibe destaque visual quando ativo, com cor de texto adaptada via tokens do tema.
4. **Histórico Undo / Redo:**
   * Alterações de posição por arrasto no Canvas só devem ser empilhadas no `undo_stack` quando o usuário soltar o botão do mouse (`ButtonReleased` / `EndMoveItem`), jamais em movimentações intermediárias.
5. **Adaptação a Temas:**
   * Todas as miniaturas de símbolos e pré-visualizações na biblioteca devem se adaptar dinamicamente ao contraste do tema ativo (Dark, Light, System) via `adapt_svg_colors`.
   * **Detecção do Sistema:** `detect_system_dark()` detecta a preferência do SO nativamente no Windows e Linux. Sombras de menus e tooltips utilizam tokens semânticos.
6. **Zero Emojis na UI:**
   * A interface deve utilizar exclusivamente ícones vetoriais SVG (`crate::icons`) para representar ferramentas, estados e indicadores. Caracteres emoji são terminantemente proibidos na interface para evitar caracteres não renderizados ("▯") em distribuições Linux.

---

## 5. Diretrizes de Engenharia e Decisões de Arquitetura

1. **Sincronismo Rígido de Tipos:** Alterações em structs ou enums em `model.rs` exigem atualização imediata e atômica em `sim/`, `canonical.rs`, `symbols.rs` e `io.rs`.
2. **Preservação de Dados de Engenharia Reversa:** **Nunca** remova nem altere comentários contendo offsets e ponteiros hexadecimais (ex.: `// RE: 0x00431xxx`), pois mapeiam o binário original do CADe_SIMU.
3. **Windows Long Paths no Git:** Os repositórios QElectroTech possuem caminhos de arquivo extensos. No Windows ou CI, garanta `git config core.longpaths true`.
4. **Decisão D1 (Implementada e Validada): Salvamento Multipágina e Exportação:**
   * O formato legado `.cad` foi **completamente removido da rota de salvamento** — ele é estritamente monofolha e perderia páginas adicionais.
   * O salvamento nativo do projeto é multipágina em `.json` (padrão), `.toml` ou `.qet`.
   * O formato `.cad` é restrito à **Importação** e **Exportação** (com seletor de folha quando o projeto possuir múltiplas páginas), implementado no modal de exportação (`src/ui/modals.rs` e `src/export.rs`) reutilizando o escritor byte-exato de `io.rs`.
5. **Decisão D2 (Implementada e Validada): Catálogo Canônico Unificado (9.3k itens):**
   * A biblioteca de símbolos utiliza nomes fixos de 15 caracteres no formato `<familia>/<subcategoria:3>-<id:5hex>-<estado:1hex>.svg` (ex.: `electric/012-00020-0.svg`).
   * Catálogo em `symbols_qet/catalog.json`: 9.273 elementos base, 9.327 arquivos SVG em disco, 0 órfãos e 0 colisões case-insensitive.
   * Busca em $O(1)$ provida por `CanonicalDictionary::global()` em `src/canonical.rs`.
   * Campo `kind: CanonicalKind` obrigatório e tipado em todos os elementos, eliminando parsing frágil de strings.
6. **Decisão D3 (Implementada e Validada): Isolamento de Blocos Visuais de Montagem:**
   * Elementos gráficos de montagem visual (painéis 2D, trilhos DIN, canaletas, bornes mecânicos e módulos de CLP) são classificados como `kind: CanonicalKind::Block`.
   * Não possuem terminais elétricos funcionais (`!is_wireable()`) e são **completamente ignorados pelo motor de simulação** em `src/sim/graph.rs`, preservando alta performance de simulação.
7. **P2-#15 e P3-#17 (Bornes e Referência Cruzada):**
   * A simulação elétrica gráfica conecta condutores via **coordenadas geométricas espaciais `(x, y)` e índices ordinais de pinos**, operando 100% mesmo na ausência de tags nos bornes.
   * **P3-#17 (Concluído):** 100% dos 5.700 elementos ligáveis da biblioteca canônica possuem bornes não-vazios e estritamente únicos normalizados segundo IEC 60947-1, EN 50005 e IEC 60204-1 (ex.: bobinas `A1`/`A2`, lâmpadas `X1`/`X2`, contatos térmicos `95`/`96` e `97`/`98`, disjuntores `1`/`2`, `3`/`4`, motores `U1`/`V1`/`W1` etc.), desbloqueando integralmente as réguas de bornes e a netlist de P2-#15.

---

## 6. Procedimentos de Entrega e Convivência Multi-Agente (CI/CD)

1. **Regra de Validação Local (Não Suba sem Sinal Verde):**
   * **Nenhum agente deve executar `git push`** ou criar tags no GitHub sem a autorização explícita do usuário no chat. Mantenha os commits locais enquanto as validações e ajustes estiverem em andamento.
2. **Atomicidade e Regressão Zero:**
   * Antes de encerrar qualquer turno de trabalho, execute obrigatoriamente:
     ```bash
     cargo fmt --all -- --check
     cargo clippy --all-targets -- -D warnings
     cargo test --all-targets
     ```
   * Todos os 78 testes devem passar com 100% de sucesso e zero advertências do compilador.
3. **Padrão de Commits:**
   * Use Conventional Commits em Português do Brasil (ex.: `feat(ui): ...`, `fix(sim): ...`, `refactor(core): ...`, `test(integration): ...`).
   * É estritamente **proibido** o uso de `git push --force`.
4. **Integração Contínua (CI):**
   * O repositório conta com esteira automatizada (`.github/workflows/ci.yml`) que valida formatação (`cargo fmt`), análise estática (`cargo clippy --all-targets -- -D warnings`) e suíte de testes (`cargo test --all-targets`) em Linux e Windows a cada push ou PR para a branch `main`.
5. **Geração de Releases:**
   * A esteira de release no GitHub Actions constrói executáveis portáteis de Windows e Linux automaticamente ao receber tags no padrão `v*` (ex.: `v0.4.1-demo`; o workflow também aceita disparo manual via `workflow_dispatch`).
