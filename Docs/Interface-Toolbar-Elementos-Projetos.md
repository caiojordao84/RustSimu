# Barra de Ferramentas, Elementos e Projetos (Opções A e B)

> **Documento de Especificação Técnica e Arquitetura**  
> Data: 11/11/2026  
> Escopo: `src/app.rs`, `src/theme.rs`, `src/canvas_view.rs`, `src/export.rs`, `src/ui/toolbar.rs`, `src/ui/library.rs`, `src/ui/modals.rs`, `src/ui/canvas_area.rs`, `src/ui/menu.rs`, `tests/integration_tests.rs`  
> Conformidade: `Docs/Modificações (1).md` (Seção 2 "Barra de Ferramentas", Seção 3 "Elementos e Projetos")  
> Status da suíte: 106 testes automatizados aprovados (100% verde em ~3,0 s)

---

## 1. Visão Geral

Este documento consolida a especificação técnica e implementação das Opções A e B de modernização da interface do RustSimu:

1. **Opção A (Seção 2 - Barra de Ferramentas / Toolbar)**:
   - Exclusividade mútua entre as ferramentas Selecionar (`EditorTool::Select`) e Mover Pan (`EditorTool::Pan`).
   - Pan livre no canvas mantendo o botão direito do mouse pressionado, sem interromper a ferramenta ativa.
   - Cursor visual reativo: mão aberta (`Grab`) e fechada (`Grabbing`) no modo Pan, e cruz (`Crosshair`) no modo Marquee.
   - Dropdown de controle de visibilidade de textos e tags no canvas (`Textos: Ligado / Desligado`).
   - Janela modal de seleção de cores para elementos (`view_color_modal`) com paleta industrial de 12 cores, entrada de código hexadecimal livre e cartão de preview em tempo real.
   - Suporte a cores customizadas na renderização do canvas e na exportação vetorial SVG.

2. **Opção B (Seção 3 - Elementos e Projetos)**:
   - Painel lateral esquerdo renomeado para "Elementos e Projetos" com seletores de aba inferiores ("Elementos" e "Projetos").
   - Aba "Elementos" selecionada por padrão (`left_dock_tab = 0`), com inserção por duplo clique, botão "Inserir no Esquema" ou arrasto direto ao canvas, com cancelamento por botão direito ou ESC.
   - Aba "Projetos" com menu dropdown (`pick_list`) de navegação multipágina no formato `Nome do Projeto -> (Nº da Página / Total de Páginas) Nome da Página`.
   - Lista de projetos recentes com clique simples para seleção, duplo clique ou botão dedicado "Abrir" para carregamento.
   - Exclusão segura com janela modal de confirmação (`view_confirm_delete_project_modal`) disparada pelo botão "Apagar" ou pela tecla `Delete` do teclado.
   - Recolhimento e expansão da barra lateral com botão chevron, mantendo uma barra fina de 28px visível com botão de restauração.

---

## 2. Opção A: Barra de Ferramentas (Toolbar)

### 2.1 Exclusividade Selecionar vs Mover
As ferramentas de seleção e pan agora são mutuamente exclusivas na interface:
- No arquivo `src/app.rs`:
  - Selecionar `EditorTool::Pan` desativa a ferramenta de seleção e ativa a navegação panorâmica com cursor de mão aberta (`Grab`) e mão fechada durante arrasto (`Grabbing`).
  - Durante o modo de simulação, a ativação do Pan é não-intrusiva: não interrompe a execução física dos circuitos.
  - O botão direito do mouse no canvas permite deslocar a área de visualização em qualquer ferramenta de edição ativa sem perder a seleção corrente.

### 2.2 Controle de Visibilidade de Textos
Implementado o tipo `TextVisibility` e dropdown na toolbar:
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextVisibility {
    #[default]
    On,
    Off,
}
```
- No canvas (`src/canvas_view.rs`), quando `show_texts` está desligado (`Off`), tags, designações de bornes e textos dos componentes são ocultados na renderização sem alterar a integridade geométrica dos condutores.
- No menu `Exibição`, adicionada a marcação `[X]` de estado para refletir a visibilidade dos textos.

### 2.3 Modal de Cores de Elementos
Implementado o modal `view_color_modal` em `src/ui/modals.rs`:
- Paleta predefinida com 12 cores industriais:
  - Preto industrial (`#212121`), Cinza grafite (`#424242`), Vermelho comando (`#E53935`), Azul controle (`#1E88E5`);
  - Verde terra/PE (`#43A047`), Amarelo sinal (`#FDD835`), Laranja neutro (`#FB8C00`), Roxo especial (`#8E24AA`);
  - Marrom fase L1 (`#6D4C41`), Preto fase L2 (`#1A237E`), Cinza fase L3 (`#546E7A`), Branco neutro (`#FFFFFF`).
- Campo de entrada textual de código hexadecimal com normalização e sanitização automática de caracteres (`#RRGGBB`).
- Cartão de visualização em tempo real (preview) com nome, tag e amostra da cor selecionada.
- As cores dos elementos selecionados são aplicadas ao campo `item.color: i32` e respeitadas na exportação vetorial SVG (`src/export.rs`), exceto no modo monocromático compulsório.

---

## 3. Opção B: Elementos e Projetos

### 3.1 Aba "Elementos"
- Exibição padrão ao abrir a aplicação (`left_dock_tab: 0`).
- Duplo clique em qualquer item da biblioteca unificada ou clique em "Inserir no Esquema" coloca o símbolo no ponteiro do mouse (`EditorTool::PlaceCanonicalSymbol`). No próximo clique na folha, o símbolo é inserido.
- Arrasto (`mouse_area(..).on_press(Message::StartCanonicalSymbolDrag(..))`) posiciona o elemento diretamente nas coordenadas do cursor no canvas ao soltar o botão do mouse.
- Cancelamento com botão direito ou tecla `ESC` retorna o ponteiro imediatamente à ferramenta de seleção (`EditorTool::Select`).

### 3.2 Aba "Projetos"
- **Navegação de Páginas em Dropdown**:
  Substituiu a lista estática anterior por um componente `pick_list`:
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct ProjectPageOption {
      pub index: usize,
      pub label: String,
  }
  ```
  Formato da etiqueta:
  `Nome do Projeto -> (Nº da Página / Total de Páginas) Nome da Página`
  *(Exemplo: `Projeto Principal -> (1/3) Diagrama de Potência`)*.
  A seleção no dropdown dispara `Message::SelectPage(opt.index)`, trocando a página ativa no editor de forma síncrona.
- **Seleção e Abertura de Projetos Recentes**:
  - Clique simples seleciona a linha na lista, destacando-a visualmente (`theme::tool_active`).
  - Duplo clique na linha (intervalo < 400ms verificado via `std::time::Instant`) ou clique no botão dedicado "Abrir" carrega o arquivo no editor.
- **Exclusão Segura com Confirmação Modal**:
  - Selecionar uma linha e pressionar a tecla `Delete` do teclado ou clicar no botão "Apagar" abre a janela modal de confirmação `view_confirm_delete_project_modal`.
  - O modal exibe o nome do arquivo, caminho completo, nota esclarecendo que o arquivo em disco é preservado, botão "Cancelar" e botão destrutivo "Apagar da Lista" estilizado com `theme::danger`.
  - Cancelar pelo botão, clique no fundo ou tecla `ESC` fecha o modal sem remover o item.
  - Confirmar a exclusão remove o projeto de `app.recent_files` e limpa a seleção.

### 3.3 Recolhimento da Barra Lateral
- Botão chevron no cabeçalho permite recolher e expandir o painel lateral esquerdo.
- Quando recolhido, permanece visível uma barra de 28px de largura com ícone `ChevronRight` para reabertura a qualquer momento.

---

## 4. Isolamento Modal do Canvas
Garantido que quando qualquer janela modal (`show_about_modal`, `show_export_modal`, `show_color_modal`, `show_confirm_delete_modal`) estiver aberta, o canvas de trabalho bloqueia 100% dos eventos do mouse (`is_blocked: true` em `src/ui/canvas_area.rs`), evitando cliques acidentais na folha de desenho.

---

## 5. Suíte de Testes Automatizados

Foram adicionados 8 novos testes de integração em `tests/integration_tests.rs`:
1. `test_toolbar_select_and_pan_tools_exclusivity`
2. `test_toolbar_text_visibility_dropdown`
3. `test_color_element_modal_and_application`
4. `test_colored_element_export_to_svg`
5. `test_projects_tab_page_dropdown_navigation`
6. `test_recent_projects_selection_and_double_click_open`
7. `test_recent_projects_delete_confirmation_modal_flow`
8. `test_left_dock_elements_default_and_collapse_expand`

Total de testes no repositório: **106 testes** (27 unitários em lib/xref, 5 binários de validação, 74 de integração) com 100% de aprovação e zero advertências no Clippy.
