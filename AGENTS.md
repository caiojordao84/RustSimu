# RustSimu — Instruções de Projeto, Regras Operacionais e Arquitetura

Este documento estabelece as regras e convenções obrigatórias para agentes de IA (**Antigravity**, **OpenCode** e outros) que desenvolvem no repositório **RustSimu**.
Toda e qualquer alteração deve respeitar rigorosamente as diretrizes abaixo descritas para garantir estabilidade, conformidade eletrotécnica e regressão zero.

---

## 1. Comandos Operacionais

* **Checagem estática de tipos e compilação:**
  ```bash
  cargo check --all-targets
  ```
* **Suíte de testes (Obrigatório antes de qualquer commit):**
  ```bash
  cargo test
  ```
  * Os testes (simulação de circuitos reais, roundtrip de formatos, UI) vivem inline no `mod tests` do `main.rs`, e usam como fixtures os esquemas originais do diretório `Projetos/*.cad`. É a rede de proteção de toda simulação — qualquer mudança em `sim.rs`, `symbols.rs`, `io.rs` ou `model.rs` deve acrescentar ou atualizar pelo menos um teste nele.
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
  * *Nota:* O `elmt2svg` depende de uma coleção **QElectroTech instalada localmente** (caminho padrão `/usr/share/qelectrotech`; aceita caminho customizado por argumento). Se o comando falhar por ausência dessa coleção, não é defeito do código nem do ambiente do agente — regere-o apenas quando a coleção estiver disponível.

---

## 2. Arquitetura de Módulos e Fluxo de Dados

```text
       [ model.rs ] ◄─────────────► [ sim.rs ] (Motor Eletrotécnico)
            ▲                             ▲
            │                             │
       [  io.rs  ] ◄──────────────► [ symbols.rs / svg_symbol.rs ]
            ▲                             ▲
            │                             │
    [ canvas_view.rs ] ◄──────────► [ layout.rs / theme.rs ]
            ▲
            └─────────────── [ main.rs ] (Coordenador Iced 0.13)
```

* **`model.rs`**: Representação pura de dados do esquema (`SchematicDocument`, `SchematicItem`, condutores, bornes, nós de conexão, páginas e potenciais).
* **`sim.rs`**: Motor de propagação física e lógica de potenciais (L1–L3, N, PE, CC+/CC-; curto-circuito com desarme de disjuntores e queima de fusíveis; intertravamentos e contatos auxiliares). O acoplamento entre bobinas e contatos é **exclusivamente por TAG** (não há modelo de acoplamento magnético no motor).
* **`canvas_view.rs`**: Renderização acelerada via GPU no Iced Canvas (`iced::widget::canvas`), emissão das mensagens de auto-conexão (*smart wiring* — a lógica de fusão de condutores e junções é aplicada no `main.rs`), *hit-test* com priorização de símbolos sobre fios, e manipulação na grade (*grid snap*).
* **`symbols.rs` / `svg_symbol.rs`**: Catálogo universal com primitivas geométricas, mapeamento de bornes e adaptação de contraste SVG por tema (`adapt_svg_colors`).
* **`icons.rs`**: Renderização de ícones vetoriais modernos da interface em SVG.
* **`layout.rs` / `theme.rs`**: Sistema de colunas físicas recolhíveis (288px ↔ 28px) e design system com paletas semânticas Dark, Light e System.
* **`io.rs`**: Serialização e desserialização multiprotocolo (`.cad`, `.json`, `.toml`, `.qet`).
* **`main.rs`**: Ponto de entrada, coordenador de estado, toolbar responsiva, menubar em overlays e loop de mensagens principal.
* *Dependências adicionais fora do diagrama:* `sim.rs` depende de `symbols.rs` (`find_symbol`, bornes no mundo a cada passo) e de `svg_symbol.rs` (terminais de símbolos SVG QElectroTech); `io.rs` depende de `symbols.rs` (instanciação do preset de partida direta); `main.rs` orquestra todos os módulos acima.

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
   * **Estado de Curto-Circuito:** `Potential::ShortCircuit` (já implementado em `model.rs`, rótulo visual `"CC!"`; condutores sob curto são pintados em vermelho espesso em `canvas_view.rs`). Conexão direta entre potenciais de polaridades opostas (ex.: L1 com N, ou L1 com L2) marca este estado, dispara a falta (*trip*) e **desarma disjuntores magnéticos** associados, além de **queimar fusíveis** no circuito afetado.
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
   * Quando um menu ou modal estiver ativo (`open_menu.is_some() || show_about_modal`), o Canvas opera em estado bloqueado (`is_blocked: true`), retornando `(event::Status::Ignored, None)`.
   * Eventos de movimento espacial (`CanvasMessage::CursorMoved`) **nunca** devem fechar menus abertos no `update`.
2. **Hit-Testing com Prioridade para Símbolos:**
   * O clique do mouse em coordenadas onde símbolos e fios se cruzam **deve sempre selecionar o símbolo** (componente) e não o fio que passa por baixo.
3. **Layout Físico Responsivo:**
   * **Docks Laterais:** Largura padrão de `288px` quando expandidos (redimensionáveis entre `160px` e `480px` via separador de 6px) e `28px` quando recolhidos (com botão chevron centralizado em `align_x: Center`). O separador é desativado quando o dock correspondente está recolhido.
   * **Área Central (Canvas):** Expande dinamicamente para consumir todo o espaço liberado (`Length::Fill`).
   * **Toolbar:** As ferramentas à esquerda usam container com rolagem suave (`scrollable`), enquanto o grupo de simulação (**Play**, **Stop**, **Step**) fica fixado à direita com botões icônicos compactos (sem textos).
   * **Efeito Play:** O botão Play exibe brilho radial (*neon glow* emitido via `Shadow`) quando ativo, com cor de texto adaptada via `tokens.on_primary`.
4. **Histórico Undo / Redo:**
   * Alterações de posição por arrasto no Canvas só devem ser empilhadas no `undo_stack` quando o usuário soltar o botão do mouse (`ButtonReleased` / `EndMoveItem`), jamais em movimentações intermediárias.
5. **Adaptação a Temas:**
   * Todas as miniaturas de símbolos e pré-visualizações na biblioteca devem se adaptar dinamicamente ao contraste do tema ativo (Dark, Light, System) via `adapt_svg_colors`.
   * **Detecção do Sistema:** `detect_system_dark()` detecta a preferência do SO nativamente no Windows (via registro `AppsUseLightTheme`), Linux GNOME e KDE Plasma 6 (via portal freedesktop `Settings.Read`, `kreadconfig6`, `GTK_THEME` e `gsettings`). Sombras de menus e tooltips utilizam `theme.extended_palette().is_dark`.

---

## 5. Diretrizes de Engenharia e Compatibilidade Legada

1. **Sincronismo Rígido de Tipos:** Alterações em structs ou enums em `model.rs` exigem atualização imediata e atômica em `sim.rs`, `symbols.rs` e `io.rs`.
2. **Preservação de Dados de Engenharia Reversa:** **Nunca** remova nem altere comentários contendo offsets e ponteiros hexadecimais (ex.: `// RE: 0x00431xxx`), pois mapeiam o binário original do CADe_SIMU.
3. **Windows Long Paths no Git:** Os repositórios QElectroTech possuem caminhos de arquivo extensos. No Windows ou CI, garanta `git config core.longpaths true`.

---

## 6. Procedimentos de Entrega e Convivência Multi-Agente (CI/CD)

1. **Regra de Validação Local (Não Suba sem Sinal Verde):**
   * Nenhum agente deve executar `git push` ou criar tags no GitHub sem a confirmação explícita do usuário no chat. Mantenha os commits locais enquanto as validações e ajustes estiverem em andamento.
2. **Atomicidade e Regressão Zero:**
   * Antes de encerrar qualquer turno de trabalho, execute `cargo check` e `cargo test`. Todos os testes devem passar com 100% de sucesso.
3. **Padrão de Commits:**
   * Use Conventional Commits em Português do Brasil (ex.: `feat(ui): ...`, `fix(sim): ...`, `test(core): ...`).
   * É estritamente **proibido** o uso de `git push --force` para evitar corrupção de commits feitos por outros agentes ou pelo usuário.
4. **Geração de Releases:**
   * A esteira de CI no GitHub Actions constrói executáveis portáteis de Windows e Linux automaticamente ao receber tags no padrão `v*` (ex.: `v0.4.1-demo`; o workflow também aceita disparo manual via `workflow_dispatch`).
