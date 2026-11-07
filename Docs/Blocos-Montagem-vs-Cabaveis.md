# Classificação de Elementos Gráficos e Blocos de Montagem vs Elementos Cabáveis

> **Documento de Especificação e Arquitetura** (Decisão **D3**, 07/11/2026 — Tarefa **P3-#18**).  
> Estabelece a separação clara entre **símbolos cabáveis funcionais** (esquema elétrico/lógico/fluídico) e **blocos visuais de montagem** (layout físico 2D de painéis, trilhos DIN, acessórios mecânicos e carcaças de CLP).

---

## 1. Contexto e Motivação

O acervo canônico unificado do RustSimu conta com **9.273 elementos base** (9.327 arquivos SVG em disco). Historicamente, cerca de um terço desses elementos (2.820 itens) não possuía nenhum terminal elétrico, compreendendo:
- Trilhos DIN (TS15, TS35 Wago, Siemens, Weidmüller).
- Gabaritos e vistas frontais 2D de placas de montagem de painéis (*thumbnails mounting plate*).
- Molduras, carimbos, batentes finais e tampas de réguas de bornes.
- Elementos gráficos decorativos ou qualificadores IEC sem condutividade (setas, blindagens, chassis mecânicos).

Adicionalmente, os blocos de Controladores Lógicos Programáveis (CLPs) herdados do QElectroTech (Schneider M221/M241/M340, Siemens S7-300/S7-1500/Logo, Allen-Bradley, Omron, Arduino, Sofrel, etc.) continham dezenas de pequenos círculos de terminais geométricos que **não são úteis para a simulação funcional de esquemas** e poluíam visualmente o canvas com pontos vermelhos desnecessários.

A **Decisão D3** estabelece que tais elementos são **blocos de montagem visual não cabáveis**, destinados ao planejamento físico e montagem mecânica de painéis elétricos, devendo ser estritamente isolados do grafo elétrico do motor de simulação.

---

## 2. Especificação Canônica (Decisão D3)

### 2.1 Tipo Canônico `kind: "block"`
No dicionário de símbolos (`symbols_qet/catalog.json` e `symbols_qet_all/catalog.json`), o tipo canônico `kind: "block"` é atribuído a:
1. **Elementos com 0 terminais** (2.819 elementos base).
2. **Blocos e módulos de CLP / controladores** (752 elementos base).

Total de blocos no catálogo: **3.571 elementos** (38,5% da biblioteca).  
Total de elementos cabáveis de esquema: **5.702 elementos** (61,5% da biblioteca).

### 2.2 Desativação dos Terminais de CLP
Conforme determinação de projeto, os terminais dos blocos de CLP **não são importados e são zerados** (`terminals: []`):
- O módulo de CLP passa a ser tratado como um bloco íntegro de layout 2D.
- Não são renderizados círculos vermelhos de bornes sobre o corpo do CLP no canvas.
- No arquivo `src/bin/elmt2svg.rs`, a rotina de conversão e indexação detecta controladores (`is_plc_element`) e ignora tags `<terminal>` durante o parsing de arquivos `.elmt`.

### 2.3 Predicado de Cababilidade `is_wireable()`
Na estrutura [`CanonicalElement`](file:///home/caiojordao84/Downloads/CADE%20SIMU%204.0,%20PC%20SIMU%203.0/CADe_SIMU/CADe_SIMU/cade-simu-rs/src/canonical.rs#L83):

```rust
impl CanonicalElement {
    /// Indica se o elemento é cabável (pode ser conectado eletricamente/ligado por fios).
    /// Retorna `false` para blocos de montagem visual (painel, trilhos DIN, módulos CLP e acessórios de layout).
    #[inline]
    pub fn is_wireable(&self) -> bool {
        self.kind != "block" && !self.terminals.is_empty()
    }
}
```

---

## 3. Isolamento do Motor de Simulação

Elementos do tipo bloco **não participam do cálculo elétrico**:
1. **Construção do Grafo Topológico ([`src/sim/graph.rs`](file:///home/caiojordao84/Downloads/CADE%20SIMU%204.0,%20PC%20SIMU%203.0/CADe_SIMU/CADe_SIMU/cade-simu-rs/src/sim/graph.rs#L95-L105))**:
   Na função `build_network_graph`, ao iterar pelos elementos do esquema, itens com `symbol_id` cujo elemento canônico retorne `!el.is_wireable()` são imediatamente ignorados via `continue`.
2. **Zero Nós Elétricos**:
   Não geram entradas em `all_points` nem em `item_pin_nodes`. Fios desenhados sobre ou perto de um bloco de montagem não são subdivididos nem conectados ao bloco.
3. **Regras de Comutação e Dispositivos ([`src/sim/rules.rs`](file:///home/caiojordao84/Downloads/CADE%20SIMU%204.0,%20PC%20SIMU%203.0/CADe_SIMU/CADe_SIMU/cade-simu-rs/src/sim/rules.rs))**:
   `kind == "block"` não casa com nenhuma regra de disjuntor, botão, chave, bobina ou motor.

---

## 4. Experiência do Usuário (UI/UX)

### 4.1 Seletor de Escopo na Biblioteca Unificada
No dock esquerdo da biblioteca unificada ([`src/app.rs`](file:///home/caiojordao84/Downloads/CADE%20SIMU%204.0,%20PC%20SIMU%203.0/CADe_SIMU/CADe_SIMU/cade-simu-rs/src/app.rs#L4240-L4275)), foi introduzido um seletor de escopo horizontal em chips:

| Chip de Escopo | Descrição | Quantidade |
|---|---|---|
| **`[ Todos ]`** | Exibe o catálogo completo sem restrição de tipo | 9.273 |
| **`[ ⚡ Esquema (Cabáveis) ]`** | Filtra apenas itens com bornes funcionais para desenho de circuitos de comando e potência | 5.702 |
| **`[ 📦 Painel (Blocos) ]`** | Filtra apenas gabaritos 2D, trilhos DIN, carcaças de CLP e acessórios mecânicos para layout físico | 3.571 |

### 4.2 Indicadores Visuais
- **Listagem de Itens**: Elementos sem terminais exibem o badge estilizado `[Bloco]` em vez do antigo e ambíguo `0b`.
- **Preview Box**: Na parte inferior do painel, exibe a mensagem de esclarecimento: *"Bloco de montagem visual (não simulável)"*.
- **Prefixo de Tag Padrão**: Ao posicionar um bloco no canvas, o aplicativo gera automaticamente tags sequenciais com prefixo `-BK` (ex: `-BK1`, `-BK2`).
- **Canvas View ([`src/canvas_view.rs`](file:///home/caiojordao84/Downloads/CADE%20SIMU%204.0,%20PC%20SIMU%203.0/CADe_SIMU/CADe_SIMU/cade-simu-rs/src/canvas_view.rs#L898-L915))**: A rotina de renderização de bornes só desenha círculos vermelhos se `el.is_wireable() == true`.

---

## 5. Vinculação Inter-Folhas e Referência Cruzada

Em projetos industriais reais, um projeto elétrico é tipicamente organizado em múltiplas folhas:
- **Folhas Funcionais (1, 2, ...)**: Diagramas unifilares e multifilares de potência e comando (motores, contatores `-KM1`, relés térmicos `-F1`, disjuntores `-Q1`, botões `-S1`).
- **Folhas de Painel / Layout Físico**: Desenho 2D da disposição dos aparelhos no trilho DIN dentro do quadro de comando.

### Fluxo de Trabalho com Referência Cruzada:
1. O desenhista insere o bloco visual 2D do CLP ou contator na folha de layout de painel.
2. No painel de propriedades (dock direito), o usuário altera a tag do bloco para coincidir com a peça funcional (ex: `-PLC1` ou `-KM1`).
3. O painel exibe um banner informativo:  
   *`ℹ Bloco de Montagem Visual (Painel/Layout). Não simulável.`*
4. Na aba **Referência Cruzada**, o sistema lista automaticamente todas as ocorrências daquela tag no projeto, relacionando o bloco físico às peças funcionais distribuídas nas outras folhas.

---

## 6. Verificação e Testes Automatizados

A integridade da decisão D3 é permanentemente auditada pela suíte de testes:
1. **Validador D2/D3 (`cargo run --bin elmt2svg -- --check`)**:
   Valida todas as invariantes canônicas sem erros e sem avisos (9.273 elementos, 1.018 categorias, 9.327 arquivos em disco).
2. **Teste de Integração Dedicado**:
   [`test_canonical_block_classification_and_simulation_isolation`](file:///home/caiojordao84/Downloads/CADE%20SIMU%204.0,%20PC%20SIMU%203.0/CADe_SIMU/CADe_SIMU/cade-simu-rs/tests/integration_tests.rs#L5135):
   - Valida que blocos de CLP (`007B2`) e trilhos DIN (`00C55`) possuem `kind: "block"`, `terminals.is_empty()` e `is_wireable() == false`.
   - Valida a contagem exata dos filtros de escopo (`Wireable`: 5.702, `Block`: 3.571, soma: 9.273).
   - Valida que a inserção na folha gera tag `-BK`.
   - Valida que `build_network_graph` não aloca nós nem conexões no grafo de simulação para o bloco inserido.
