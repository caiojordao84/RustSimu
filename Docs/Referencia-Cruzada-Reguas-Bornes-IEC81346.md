# Sistema Global de Referência Cruzada (XRef) e Réguas de Bornes IEC 81346

## 1. Visão Geral e Normas de Referência

Em projetos eletrotécnicos industriais de média e grande escala, diagramas elétricos são distribuídos em múltiplas folhas estruturadas (potência, comando, intertravamentos, bornes de campo e interfaces com controladores lógicos programáveis). 

A correlação entre bobinas acionadoras e seus respectivos contatos auxiliares distribuídos, bem como a organização de réguas de bornes físicas para conexão a painéis e motores, constitui o núcleo da rastreabilidade técnica dos esquemas.

Esta especificação documenta o motor de **Referência Cruzada (XRef)** e de **Gestão de Réguas de Bornes (Terminal Strips)** implementado em `src/xref.rs`, integrado à arquitetura Elm da aplicação (`src/app.rs`) e ao painel de propriedades (`src/ui/properties.rs`), em conformidade com as normas:

- **IEC 81346-1 / IEC 81346-2**: Sistemas industriais, instalações e equipamentos — Princípios de estruturação e designações de referência (classificação de réguas de bornes com a letra `X`).
- **IEC 60947-1 / IEC 60947-4-1 / IEC 60947-5-1**: Aparelhagem de baixa tensão, contatores, disjuntores-motores e contatos de comando.
- **IEC 60204-1**: Segurança de máquinas — Equipamento elétrico de máquinas (regras de marcação de condutores e identificação de componentes nos esquemas).
- **EN 50005 / EN 50011 / EN 50012**: Regras gerais de identificação de terminais em aparelhos de manobra e relés auxiliares.

---

## 2. Coordenadas de Grade Normalizadas (Grid Cells)

Para permitir a localização rápida de qualquer elemento na folha impressa ou na visualização em tela, cada folha de desenho (formato A4 horizontal com moldura técnica) é particionada em uma malha de quadrantes identificados por colunas numéricas e linhas alfabéticas:

- **Colunas horizontais**: 8 zonas numeradas de `1` a `8`.
- **Linhas verticais**: 6 faixas alfabéticas de `A` a `F` (do topo para a base).
- **Dimensões nominais de referência (A4 Landscape)**: 330,0 unidades CADe de largura por 240,0 unidades CADe de altura, com margens técnicas de borda de 6,0 unidades.
- **Tamanho médio das células**: ~39,75 unidades na largura ($X$) e ~38,0 unidades na altura ($Y$).

### Formatação das Coordenadas
O sistema gera referências compactas e expandidas:
- **Referência compacta**: `<Folha>.<Linha><Coluna>` (exemplo: `1.C6`, `2.A1`, `3.D4`).
- **Referência textual estendida**: `Folha <N> · <Linha><Coluna>` (exemplo: `Folha 1 · C6`).

### Funções do Módulo (`src/xref.rs`)
```rust
pub fn calculate_grid_cell(x: f32, y: f32, sheet_w: f32, sheet_h: f32) -> String;
pub fn format_grid_ref(page_number: usize, grid_cell: &str) -> String;
pub fn sheet_dimensions() -> (f32, f32);
```

---

## 3. Arquitetura da Referência Cruzada Global

O módulo `src/xref.rs` consolida todas as páginas do projeto elétrico (`Vec<DiagramPage>`) em uma estrutura indexada em memória, construindo um catálogo cruzado completo:

### 3.1 Classificação de Papéis de Aparelhos (`ApparatusRole`)
Cada item no diagrama é classificado conforme sua função elétrica:
- `ApparatusRole::Primary`: bobinas de contatores (`-KM1`), bobinas de relés de tempo (`-KT1`), relés térmicos (`-FR1`), atuadores ou aparelhos de comando mestres.
- `ApparatusRole::Subordinate`: contatos auxiliares normalmente abertos (NA, 13-14), normalmente fechados (NF, 11-12), contatos de potência (1-2, 3-4, 5-6), contatos comutadores ou botões com tag associada.
- `ApparatusRole::Terminal`: bornes de conexão em régua (tipo nativo 6 ou símbolos QET com prefixos `-X` ou tags de régua).
- `ApparatusRole::Other`: sinalizadores, cargas, fontes e demais elementos passivos.

### 3.2 Estrutura do Diretório de Referências Cruzadas
```rust
pub struct ProjectCrossReferences {
    pub apparatus_groups: Vec<ApparatusGroup>,
    pub terminal_strips: Vec<TerminalStripInfo>,
    pub orphan_contacts: Vec<XrefElementRef>,
    pub duplicate_coils: Vec<XrefElementRef>,
}
```

- **Agrupamento por Tag**: elementos com a mesma identificação de tag (ex.: `-KM1`) são agrupados. A bobina principal torna-se o nó pai (`primary`), e os contatos auxiliares distribuídos nas diversas páginas são vinculados como subordinados (`subordinates`).
- **Detecção de Contatos Órfãos (`orphan_contacts`)**: contatos que referenciam uma tag de comando para a qual não existe nenhuma bobina ou elemento mestre instanciado no projeto são destacados com alertas visuais e sinalizados no sumário de consistência.
- **Detecção de Bobinas Duplicadas (`duplicate_coils`)**: duas ou mais bobinas com a mesma tag funcional em páginas distintas geram colisão e são imediatamente alertadas para evitar ambiguidades operacionais.

---

## 4. Análise Topológica de Ramais Elétricos por Borne

Quando um elemento está selecionado no canvas, o motor executa uma análise contextual profunda de seus bornes e dos condutores a eles conectados. Utilizando o grafo de adjacência da simulação (`base_adj`) e algoritmos de busca em componentes conexos (`compute_connected_components`), o sistema classifica as conexões elétricas de cada borne:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElectricalConnectionType {
    Parallel,
    Series,
    Direct,
}
```

### 4.1 Conexões em Paralelo (`Parallel`)
Ocorre quando o elemento selecionado compartilha simultaneamente dois nós equipotenciais distintos com outro componente em dois pinos diferentes.
- **Exemplo clássico**: o contato de selo `-KM1` (pinos 13 e 14) ligado em paralelo com a botoeira de partida `-S1` (pinos 13 e 14).
- O sistema detecta que o nó do pino 13 de `-KM1` conecta ao pino 13 de `-S1`, e o nó do pino 14 de `-KM1` conecta ao pino 14 de `-S1`, rotulando o ramal como `[Paralelo]`.

### 4.2 Conexões em Série (`Series`)
Ocorre quando o elemento selecionado compartilha exatamente um único nó equipotencial em um pino de chaveamento com outro componente ativo de controle ou proteção.
- **Exemplo clássico**: a botoeira de desliga `-S0` (pinos 11 e 12) ligada em série com a botoeira de liga `-S1` (pinos 13 e 14), compartilhando o nó elétrico entre 12 e 13.
- O sistema rotula esse vínculo de controle como `[Série]`.

### 4.3 Conexões Diretas (`Direct`)
Ocorre quando há interligação simples ao mesmo nó equipotencial (distribuição de fases, barra de neutro, alimentação de bobina a partir de um disjuntor de comando), rotulada como `[Direto]`.

---

## 5. Réguas de Bornes (IEC 81346)

Conforme a norma **IEC 81346**, todas as réguas de bornes de interface e campo são designadas pela letra base **X**, com subdivisões funcionais para separação rigorosa de circuitos de potência e comando em painéis elétricos:

| Identificação da Régua | Tipo de Circuito | Função Principal na Instalação |
|---|---|---|
| **XP** (ou **X0**) | Potência / Força | Alimentação geral trifásica, barramentos de distribuição e saídas para motores ($L_1, L_2, L_3, PE, U, V, W$) |
| **XC** | Comando / Auxiliar | Interligação de botoeiras de campo, sensores de fim de curso, lógicas de segurança e sinalização externa |
| **XT** | Medição / Auxiliar | Bornes dedicados a transformadores de corrente (TC), transformadores de potencial (TP) e instrumentação |
| **X1 .. Xn** | Geral / Específico | Réguas numéricas genéricas e subdivisões por borneiras específicas de painel |

### 5.1 Detecção de Conflitos e Inconsistências
O sistema rastreia todos os bornes pertencentes à mesma régua (ex.: régua `XP`). Se dois bornes físicos possuírem o mesmo número/identificador (ex.: dois bornes com número `1` na mesma régua `XP`), o sistema emite alerta imediato de borne duplicado (`has_duplicates = true`), apontando as folhas e coordenadas onde ocorrem.

### 5.2 Algoritmo de Renumeração Sequencial Determinística (`1..N`)
Para eliminar duplicidades e garantir numeração limpa em esquemas executivos, o sistema implementa renumeração automática espacial determinística:
1. Os bornes da régua são coletados em todas as folhas do projeto.
2. É aplicada ordenação estável pela tupla:
   $$\text{Ordem} = (\text{Índice da Página}, Y_{\text{espacial}}, X_{\text{espacial}})$$
3. Os bornes são renumerados sequencialmente de `1` a $N$.
4. A renumeração suporta tanto símbolos nativos (atualizando a etiqueta do borne no item) quanto símbolos canônicos (atualizando o nome do terminal de passagem).
5. O comando gera snapshot de histórico no sistema de Undo/Redo (`push_undo`), permitindo desfazer a ação com um único clique (`Ctrl + Z`).

---

## 6. Interface do Usuário e Navegação Inter-Folhas

A interface em `src/ui/properties.rs` disponibiliza três abas integradas na barra lateral direita:

### 6.1 Aba 0 — [Propriedades]
- Cabeçalho contextual exibindo ID do elemento, nome funcional e **coordenada de grade exata** (ex.: `#25 · Bobina · 1.E5`).
- Se o elemento for um contato auxiliar subordinado, exibe atalho clicável `[ Ir ]` com a folha e célula da bobina pai correspondente.
- Se o elemento for um borne de régua, exibe card dedicado da **Régua de Bornes (IEC 81346)** com classificação, régua associada, número do borne e atalho para a aba de réguas.

### 6.2 Aba 1 — [Ref. Cruzada]
- **Com elemento selecionado**:
  - Dados da bobina mestre (tag, folha, célula, botão `[ Ir ]`).
  - Lista de contatos subordinados em todas as folhas, com identificação de tipo (NA/NF), coordenadas e botões de salto instantâneo.
  - Tabela de conexões por pino, detalhando os nós elétricos e os badges `[Paralelo]`, `[Série]`, `[Direto]`.
- **Sem elemento selecionado (Visão Global do Projeto)**:
  - Campo de busca e filtro em tempo real (`app.xref_search`).
  - Sumário de consistência: número total de aparelhos rastreados, total de contatos subordinados, total de contatos órfãos e total de réguas de bornes.
  - Alertas em destaque para contatos órfãos e bobinas duplicadas.
  - Diretório completo de aparelhos agrupados com botões `[ Ir ]` para cada item.

### 6.3 Aba 2 — [Réguas de Bornes]
- Botão superior de ação global: `[ Renumerar Todas as Réguas (1..N) ]`.
- Cards estruturados por régua com badge de classificação (`XP: Potência / Força`, `XC: Comando / Auxiliar`, `XT: Medição`, `Xn: Geral`).
- Alerta visual em vermelho caso a régua contenha identificadores de bornes duplicados.
- Botão local `[ Renumerar Régua (1..N) ]`.
- Tabela de bornes com identificador, folha, coordenada de grade e botão de navegação `[ Ir ]`.

### 6.4 Mensagens do Ciclo de Vida Elm (`src/app.rs`)
```rust
Message::NavigateToElement { page_index, item_id }
Message::RenumberTerminalStrip(String)
Message::RenumberAllTerminalStrips
Message::XrefSearchChanged(String)
```
Ao acionar o botão `[ Ir ]`, a mensagem `NavigateToElement` alterna a página ativa do projeto para a folha de destino (`app.active_page = page_index`), atualiza a seleção do canvas para o item correspondente (`diagram.selected_id = Some(item_id)`) e emite mensagem de status com a localização acessada.

---

## 7. Cobertura de Testes Automatizados

A funcionalidade é 100% coberta por testes automatizados sem regressões, integrados ao comando `cargo test --all-targets` (total de 86 testes verdes):

1. **Testes Unitários (`src/xref.rs`)**:
   - `test_calculate_grid_cell_bounds`: verifica os limites e cálculos das células de grade $A..F$ e $1..8$.
   - `test_terminal_strip_kind_classification`: verifica a classificação funcional IEC 81346 de réguas `XP`, `XC`, `XT` e `X1`.
   - `test_multipage_xref_grouping_and_orphan_detection`: valida o agrupamento mestre-subordinado em projetos de múltiplas folhas, detecção de contatos órfãos e bobinas duplicadas.
   - `test_terminal_strip_renumbering_spatial_order`: valida a renumeração determinística ordenada por página, $Y$ e $X$.

2. **Testes de Integração (`tests/integration_tests.rs`)**:
   - `test_p2_15_multipage_cross_reference_and_orphan_detection`: projeto multipágina com contator `-KM1`, contatos auxiliares e bobina auxiliar, verificando coordenadas de grade e detecção de inconsistências.
   - `test_p2_15_electrical_connections_parallel_and_series_analysis`: verificação de análise de circuito de partida direta (DOL), confirmando detecção topológica de ramal em paralelo (`-KM1` selo e `-S1`) e em série (`-S0` e `-S1`).
   - `test_p2_15_terminal_strips_iec_81346_classification_and_renumbering`: verificação de réguas `XP` e `XC`, detecção de números duplicados e validação da renumeração determinística `1..N`.
   - `test_p2_15_xref_navigation_message_switches_page_and_selects_item`: despacho de `Message::NavigateToElement`, confirmando troca automática de página e seleção de item no canvas.
