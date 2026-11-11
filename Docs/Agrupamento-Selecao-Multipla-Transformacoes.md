# Sistema de Agrupamento, Seleção Múltipla e Transformações Atômicas (Etapa 1)

> **Documento de Especificação Técnica e Arquitetura**
> Data: 11/11/2026
> Escopo: `src/model.rs`, `src/app.rs`, `src/canvas_view.rs`, `src/ui/toolbar.rs`, `tests/integration_tests.rs`
> Conformidade: `Docs/Modificações (1).md` (Seção 1.2 "Editar", Seção 2 "Barra de Ferramentas")
> Status da suíte: 98 testes automatizados aprovados (100% verde em ~3,5 s)

---

## 1. Visão Geral

A Etapa 1 das especificações de edição do RustSimu implementou o suporte de primeira classe para manipulação múltipla de elementos esquemáticos e lógica de grupos nativos no canvas de projeto. O trabalho foi decomposto e executado em três fases complementares:

1. **Trabalho 1.1**: Modelo de dados de grupos e seleção múltipla no canvas (caixa marquee, selecão com Shift, atalhos de seleção e propagação automática de grupo).
2. **Trabalho 1.2**: Comandos estruturados de agrupamento (`Ctrl+G`) e desagrupamento (`Ctrl+U`), integração de botões e ícones na barra de ferramentas e histórico transacional de desfazer/refazer.
3. **Trabalho 1.3**: Transformações atômicas de grupo e seleção múltipla (arrasto síncrono com estiramento topológico de fios, rotação por centroide geométrico, espelhamento, duplicação determinística com novos IDs de grupo e clipboard para múltiplos elementos).

---

## 2. Arquitetura de Dados (Trabalho 1.1)

### 2.1 Identificação de Grupos no Modelo
No arquivo `src/model.rs`, cada elemento `SchematicItem` recebeu o campo opcional:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchematicItem {
    pub id: u32,
    pub tipo: u8,
    // ...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group_id: Option<u32>,
}
```
- **Persistência transparente**: Grupos são preservados com fidelidade bidirecional nos formatos JSON e TOML do projeto.
- **Isolamento de compatibilidade**: O campo é omitido quando nulo (`None`), garantindo total compatibilidade retroativa com arquivos legados e exportação CADe_SIMU (.cad).

### 2.2 Conjunto de Seleção no Estado da Aplicação
No arquivo `src/app.rs`, o estado de seleção foi unificado em:
```rust
pub struct CadeSimuApp {
    // ...
    pub selected_ids: std::collections::HashSet<u32>,
    pub marquee_box: Option<(iced::Point, iced::Point)>,
    // ...
}
```
A seleção singular `selected_id: Option<u32>` permanece sincronizada como conveniência de interface (apontando para o elemento primário da seleção), enquanto `selected_ids` governa todas as operações em lote.

### 2.3 Modos de Seleção no Canvas
- **Seleção Marquee (Caixa Retangular)**: Ao clicar e arrastar com o botão esquerdo em uma área vazia da folha de trabalho, uma caixa pontilhada de seleção é desenhada. Ao soltar o mouse, todos os símbolos e fios cujos retângulos envolventes interceptam ou estão contidos na caixa são adicionados à seleção.
- **Seleção Incremental (Shift + Clique)**: Clicar sobre um elemento com a tecla `Shift` pressionada alterna o seu estado na seleção (`toggle`).
- **Atalhos Globais**:
  - `Ctrl + A`: Selecionar todos os elementos e fios da página ativa.
  - `Ctrl + Shift + A`: Desmarcar todos os elementos ativos.
  - `Ctrl + I`: Inverter a seleção atual (seleciona o complemento dos itens presentes).
- **Propagação Inteligente de Grupo**: Ao clicar em qualquer elemento que pertença a um grupo (`group_id.is_some()`), todos os demais elementos pertencentes ao mesmo `group_id` são automaticamente selecionados em conjunto.

---

## 3. Comandos de Agrupamento e Desagrupamento (Trabalho 1.2)

### 3.1 Agrupamento (`Message::GroupSelected` / `Ctrl+G`)
- **Regra de cardinalidade mínima**: Exige ao menos dois elementos selecionados (`selected_ids.len() >= 2`). Caso contrário, emite notificação explicativa na barra de status.
- **Atribuição determinística**: Calcula `max(group_id) + 1` no documento ativo e associa esse novo identificador a todos os elementos selecionados.
- **Transacionalidade**: Empilha o estado anterior no histórico de `Undo` antes da mutação, permitindo reversão imediata com `Ctrl+Z`.

### 3.2 Desagrupamento (`Message::UngroupSelected` / `Ctrl+U`)
- Itera sobre os elementos selecionados e redefine `group_id = None`.
- Se o grupo for parcialmente selecionado, os itens selecionados são destacados do grupo; os restantes mantêm seu identificador caso ainda atendam à regra do grupo.
- Suporte completo a `Undo` e `Redo`.

### 3.3 Barra de Ferramentas e Ícones
- Adicionados os ícones vetoriais `Icon::Group` e `Icon::Ungroup` em `src/icons.rs`.
- Inseridos botões dedicados de Agrupar e Desagrupar na seção de transformações da barra de ferramentas superior (`src/ui/toolbar.rs`), com dicas de atalho no hover (`Ctrl+G` e `Ctrl+U`).

---

## 4. Transformações Atômicas de Grupo (Trabalho 1.3)

### 4.1 Arrasto Síncrono no Canvas
- **Manutenção de distâncias relativas**: Ao arrastar qualquer item da seleção, o deslocamento delta `(dx, dy)` é aplicado simultaneamente a todos os nós e coordenadas `(x1, y1, x2, y2)` dos elementos e fios selecionados.
- **Estiramento de Fios Conectados**: Fios elétricos que não fazem parte da seleção, mas que possuem extremidades conectadas aos bornes dos aparelhos movidos, têm essas extremidades estiradas de forma síncrona, preservando a continuidade elétrica do circuito. Caso a opção `auto_disconnect` esteja ativada no menu Projeto, os fios são mantidos estáticos e desconectados.
- **Undo Transacional Único**: Toda a operação de arrasto, independentemente da quantidade de itens manipulados, é consolidada em um único registro de `Undo` ao soltar o mouse (`commit_drag_undo`).

### 4.2 Rotação Atômica por Centroide
- **Cálculo de Centroide Eletrotécnico**:
  Diferente de sistemas de desenho vetorial genérico que utilizam caixas delimitadoras visuais (as quais geram assimetria em símbolos com textos e terminais laterais), o centroide do circuito é calculado rigorosamente pelos limites extremos das coordenadas elétricas de inserção `(x1, y1)` dos aparelhos e vértices dos fios:
  $$cx = \frac{\min(x) + \max(x)}{2}, \quad cy = \frac{\min(y) + \max(y)}{2}$$
- **Rotação Horária (`R`) e Anti-Horária (`Shift+R`)**:
  Cada elemento tem sua orientação individual incrementada/decrementada em 90 graus e sua coordenada transladada segundo a matriz ortogonal de rotação em torno de $(cx, cy)$:
  $$\begin{bmatrix} x_{novo} \\ y_{novo} \end{bmatrix} = \begin{bmatrix} cx \mp (y - cy) \\ cy \pm (x - cx) \end{bmatrix}$$
  Fios conectores entre elementos do grupo têm seus dois vértices rotacionados solidariamente, mantendo a geometria interna intacta.

### 4.3 Espelhamento Atômico
- **Espelhar Horizontalmente (`M`)**:
  Inverte cada elemento pelo eixo vertical central da seleção:
  $$x_{novo} = 2 \cdot cx - x$$
  Fios invertem seus terminais horizontais e componentes alternam suas flags de espelhamento.
- **Espelhar Verticalmente (`F`)**:
  Inverte cada elemento pelo eixo horizontal central da seleção:
  $$y_{novo} = 2 \cdot cy - y$$

### 4.4 Duplicação Atômica (`Ctrl+D`)
- Clona simultaneamente todos os elementos da seleção com deslocamento padrão de grade (+24 px, +24 px).
- **Mapeamento de Grupos**: Os `group_id`s antigos presentes nos itens duplicados são mapeados para novos `group_id`s distintos e contíguos, preservando a identidade interna do grupo clonado sem colidir com o grupo original.
- **Renumeração Sequencial de Tags**: Aparelhos independentes (contatores, relés térmicos, botões) recebem automaticamente o próximo sufixo numérico disponível (ex.: `-K1` -> `-K2`, `-S1` -> `-S2`).
- **Foco da Seleção**: A seleção no canvas é transferida imediatamente para os elementos duplicados para permitir reposicionamento contínuo.

### 4.5 Área de Transferência para Múltiplos Elementos
A estrutura do clipboard da aplicação foi evoluída de um único item para um vetor completo:
```rust
pub clipboard: Option<Vec<SchematicItem>>
```
- **Cortar (`Ctrl+X`)**: Copia todos os itens selecionados para a memória, remove-os do documento e limpa a seleção em uma única transação de `Undo`.
- **Copiar (`Ctrl+C`)**: Copia todos os itens selecionados preservando suas propriedades, conexões relativas e identificadores de grupo.
- **Colar (`Ctrl+V`)**: Instancia a cópia com mapeamento de IDs, renumeração de tags e posicionamento relativo ao ponteiro do mouse.
- **Colar na Posição Original (`Ctrl+Shift+V`)**: Cola os itens exatamente nas suas coordenadas originais com renumeração e mapeamento de grupos, ideal para duplicação entre folhas de um projeto multipágina.

### 4.6 Alinhamento em Lote à Grade (`Ctrl+Shift+G`)
- Ao acionar o comando de alinhamento com múltiplos elementos selecionados, o primeiro elemento serve de referência angular para alinhar à grade mais próxima (`GridDensity::step`), e todo o conjunto é transladado pelo mesmo delta rígido, mantendo inalteradas as distâncias e conexões internas entre os elementos.

---

## 5. Validação Automatizada e Cobertura de Testes

Foram desenvolvidos cinco testes de integração dedicados em `tests/integration_tests.rs`:

1. `test_group_atomic_move_and_wire_tracking`: Valida o arrasto síncrono de grupos de aparelhos e fios internos, além do estiramento correto de fios externos conectados.
2. `test_group_atomic_rotation_around_centroid`: Valida a conservação do baricentro geométrico e das distâncias relativas após rotações de 90° e 360°.
3. `test_group_atomic_mirror_horizontal_and_vertical`: Valida a inversão simétrica de elementos e fios em torno dos eixos vertical e horizontal.
4. `test_group_atomic_duplicate_creates_new_group`: Valida a criação de novos IDs de grupo e renumeração automática de tags para aparelhos clonados via `Ctrl+D`.
5. `test_group_atomic_cut_copy_paste_roundtrip`: Valida o ciclo completo de clipboard com múltiplos elementos e reversão via Undo/Redo.

### Resultado da Suíte:
```
running 27 tests (lib unit tests) -> 27 passed
running 5 tests (elmt2svg tests)  -> 5 passed
running 66 tests (integration)    -> 66 passed
Total: 98 testes aprovados, 0 falhas, 0 warnings de clippy.
```
