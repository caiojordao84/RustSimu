# Arquitetura do Dicionário e Parser Canônico Bidirecional do RustSimu

**Data:** 12/11/2026  
**Pré-requisito estratégico para:** Seção 1.3 (`Modificações (1).md` linha 68 — CSV, numeração de conectores, BOM), P2 #17 (i18n `pt`/`en`, BOM/CSV) e futuros simuladores (Eletropneumática, Hidráulica e Ladder).

---

## 1. Diagnóstico do Estado Atual e Motivação

Embora o RustSimu já conte com a fundação de catálogo canônico em `src/canonical.rs` (`CanonicalCatalog`, `CanonicalElement`, `CanonicalKind`), hoje ainda coexistem **três representações desconectadas** para um mesmo componente físico:

| Origem | Identificação Atual no `SchematicItem` | Lacunas Identificadas |
|---|---|---|
| **CADe_SIMU legado (`.cad`)** | `tipo: i32` (67 símbolos em `src/symbols.rs`), `symbol_id: None`, `qet_type: None` | Carregado por `src/io.rs` sem resolução para o catálogo canônico; apenas 45 dos 67 símbolos possuem entrada `cade_native` em `symbols_qet/catalog.json` (faltam ex.: `8023`, `8030..=8034`, `2008`, `9004`, `9005`, `9013`). |
| **QElectroTech (`.qet` / `.elmt`)** | `tipo: 20000`, `qet_type: Some("embed://...")`, `symbol_id: None` | Carregado por `src/qet.rs` preservando o caminho bruto; na exportação `.qet` de itens nativos gera fallback `type_{tipo}`; na exportação `.cad` não possui tabela reversa para `tipo`. |
| **Biblioteca Unificada D2 (`symbols_qet/catalog.json`)** | `9.273` elementos em `1.018` categorias (`electric`: 7.053, `energy`: 1.325, `pneumatic`: 343, `hydraulic`: 94, `import`: 338, `logic`: 75, `cade_native`: 45) | `4.407` itens ainda classificados como `CanonicalKind::Passive`; múltiplas variantes visuais ou em idiomas distintos (`EN`/`FR`/`DE`/`PT`) para o mesmo componente não compartilham uma identidade semântica única. |

### Por que resolver isso antes da Seção 1.3 (linha 68) e do P2 #17

1. **BOM / Lista de Materiais e CSV coerentes:** Um botão pulsador monoestável NA deve aparecer agregado sob o mesmo conceito e especificação do RustSimu, tenha ele vindo de um `.cad` (`tipo = 8000`), de um `.qet` (`push_button.elmt`, `bouton_poussoir.elmt`, `drucktaster.elmt`) ou da paleta canônica.
2. **Internacionalização (`pt` / `en`) sem duplicação:** Traduzir por chave conceitual única do RustSimu em vez de manter traduções isoladas por formato externo.
3. **Escalabilidade para novos domínios:** A entrada de **Eletropneumática/Hidráulica** (ISO 1219) e **Ladder** (IEC 61131-3) trará centenas de válvulas, cilindros, bobinas e blocos. Cada formato/coleção externa deve ser apenas um adaptador na borda, nunca lógica espalhada pelo `solver.rs`, `xref.rs` ou `export.rs`.

---

## 2. Princípio Arquitetural: Conceito Canônico vs. Variante Visual

O núcleo do projeto passa a separar claramente **o que o elemento é e como funciona** (**Conceito Canônico do RustSimu**) de **como ele é desenhado ou de qual arquivo externo veio** (**Variante Visual / Representação**):

```text
  [.cad legado]         [.qet / .elmt]         [Futuros: Pneum./Hidr./Ladder]
   tipo = 8000       "bouton_poussoir_na"         ISO 1219 / IEC 61131-3
        \                     |                             /
         v                    v                            v
   +-------------------------------------------------------------------+
   |      PARSER & DICIONÁRIO BIDIRECIONAL (Adaptadores de Borda)      |
   |  - Normalização léxica multi-idioma (PT / EN / FR / DE / ES)      |
   |  - Tabela determinística (.cad tipo <-> concept_id <-> .elmt)     |
   |  - Remapeamento de bornes externos -> pinagem canônica RustSimu   |
   +-------------------------------------------------------------------+
                              |               ^
                    Importação|               |Exportação
                              v               |
   +-------------------------------------------------------------------+
   |              NÚCLEO CANÔNICO DO RUSTSIMU (Fonte Única)            |
   |                                                                   |
   |  concept_id: "el.act.pb.no" (Botão Monoestável NA)                |
   |  symbol_id:  "00021" (padrão) ou variante visual alternativa      |
   |  pins:       ["13", "14"] (IEC 60947-5-1 normalizado)             |
   |                                                                   |
   |  Consumidores: Canvas | Simulador | Inspetor | XRef | BOM | i18n  |
   +-------------------------------------------------------------------+
```

### Exemplo Concreto: Botão Monoestável Normalmente Aberto (NA)

- **Identidade Semântica Única (`RustSimuConcept`):**
  - `concept_id`: `"el.act.pb.no"`
  - `domain`: `SimulationDomain::ElectricCommand`
  - `kind`: `CanonicalKind::MomentaryButton`
  - `contact_form`: `Some(ContactForm::No)` (`NA`, fecha enquanto acionado, retorna por mola)
  - `pole_count`: `1`
  - `canonical_pins`: `["13", "14"]`
  - `default_tag_prefix`: `"-S"`
  - `default_symbol_id`: `"00021"`
  - `i18n_key`: `"concept.el.act.pb.no"` (`pt`: `"Botão Pulsador Monoestável NA"`, `en`: `"Momentary Pushbutton NO"`)
- **Mapeamentos Externos Apontando para o Mesmo Conceito:**
  - **CADe_SIMU (`cade_tipo`):** `Some(8000)` (mapeamento exato 1:1 na importação e na exportação `.cad`).
  - **QElectroTech (`qet_aliases`):** lista exata de caminhos/slugs conhecidos + regras léxicas para variações (`pushbutton_no`, `push_button`, `bouton_poussoir_no`, `drucktaster_schliesser`, etc.), registrando inclusive se os pinos locais no `.elmt` vierem como `["1", "2"]` ou `["", ""]` para alinhar ao modelo `["13", "14"]`.
  - **Variante Visual Alternativa:** Um usuário que abra um `.qet` estilizado pode escolher entre manter a geometria SVG original do `.qet` preservada ou padronizar visualmente para o símbolo `"00021"` do RustSimu — **em ambos os casos, a simulação, BOM, XRef e exportação sabem exatamente que se trata de `"el.act.pb.no"`**.

---

## 3. Esquema de Dados Proposto

### 3.1. Domínios e Sub-semântica (`src/dictionary.rs`)

Além do `CanonicalKind` existente em `src/canonical.rs`, cada conceito canônico do RustSimu carrega atributos tipados necessários à simulação multimodo, BOM e XRef:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulationDomain {
    ElectricPower,
    ElectricCommand,
    Pneumatic,
    Electropneumatic,
    Hydraulic,
    LadderLogic,
    PanelBlock,
    Annotation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactForm {
    No,        // Normalmente Aberto (13-14, 1-2 potência)
    Nc,        // Normalmente Fechado (11-12, 95-96)
    NoNc,      // Duplo isolado NA + NF
    Changeover,// Comutador SPDT (11-12-14)
    ThreeWay,  // 3 posições / inversora
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoilBehavior {
    Instantaneous,
    OnDelay,
    OffDelay,
    Cyclic,
    Latching,
}
```

### 3.2. Entrada do Dicionário Canônico (`CanonicalConcept`)

Cada entrada em `concepts.json` (versionado em repositório e validado em testes de CI) descreve um **único conceito do RustSimu**:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalConcept {
    pub concept_id: String,               // ex.: "el.act.pb.no", "pneu.valve.3_2_no"
    pub domain: SimulationDomain,
    pub kind: CanonicalKind,
    pub contact_form: Option<ContactForm>,
    pub coil_behavior: Option<CoilBehavior>,
    pub poles: u8,
    pub default_symbol_id: String,        // símbolo RustSimu padrão no catalog.json ("00021")
    pub variant_symbol_ids: Vec<String>,  // outros symbol_id do catálogo equivalentes em função
    pub default_tag_prefix: String,       // ex.: "-S", "-KM", "-Y", "-V"
    pub canonical_pins: Vec<String>,      // ex.: ["13", "14"], ["A1", "A2"], ["1", "2", "3"]
    pub name_pt: String,
    pub name_en: String,
    pub cade_tipo: Option<i32>,           // mapeamento bidirecional CADe_SIMU (ex.: Some(8000))
    pub qet_primary_path: Option<String>, // caminho QET preferido ao exportar .qet
    pub qet_aliases: Vec<String>,         // paths/uuids/slugs exatos do QET que convergem aqui
    pub lexical_rules: Vec<LexicalRule>,  // regras semânticas de parser multi-idioma
}
```

### 3.3. Evolução Retrocompatível no `SchematicItem` (`src/model.rs`)

Para não quebrar projetos salvos (`.json`, `.toml`, `.cad`, `.qet`), adicionamos apenas metadado opcional:

```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub concept_id: Option<String>,
```

- Projetos antigos ao serem abertos têm `concept_id` resolvido automaticamente em tempo de carga pelo **Parser/Resolvedor** (sem alterar `tipo` ou coordenadas originais).
- Itens novos inseridos da barra lateral ou paleta já nascem com `(concept_id, symbol_id, cade_tipo equivalente)`.

---

## 4. Algoritmo do Parser & Resolvedor Bidirecional

### 4.1. Caminho de Importação (Externo -> RustSimu)

A função central `resolve_to_concept(...)` segue ordem estrita de precedência (do mais determinístico para o heurístico):

1. **Vínculo explícito já presente (`concept_id`):** Lookup O(1) direto na tabela de conceitos.
2. **Símbolo canônico explícito (`symbol_id`):** Lookup O(1) pelo índice `symbol_id_to_concept` (cobre `default_symbol_id` e `variant_symbol_ids`).
3. **Código nativo CADe_SIMU (`tipo != 20000` e não-fio/não-gráfico):** Lookup O(1) na tabela exata `cade_tipo_to_concept` cobrindo **100% dos 67 símbolos** de `src/symbols.rs`.
4. **Assinatura QET exata (`qet_type` / `qet_uuid`):** Normalização de prefixo (`embed://`, barras `\`, caixa, `.elmt`) e lookup em `qet_alias_to_concept`.
5. **Parser Léxico-Semântico Estruturado (Fallback Inteligente):**
   - Tokeniza caminho + categoria + nome (`PT`, `EN`, `FR`, `DE`, `ES`), removendo ruído numérico/sufixos (`_001`, `_v2`).
   - Infere **domínio** (`pneumatic`/`hydraulic`/`ladder`/`electric`), **família funcional** (ex.: botão, fim de curso, disjuntor, bobina, sinaleiro, motor, válvula), **forma de contato** (`NO`, `NC`, `NO+NC`, `CO`) e **contagem de terminais** (`.elmt` `terminals.len()`).
   - Se a combinação `(domínio, família, forma_contato, polos)` apontar inequivocamente para um `CanonicalConcept`, vincula `concept_id` com nível de confiança (`ResolvedMatch::HeuristicSemantic`); caso contrário, mantém como genérico auditável (`Unresolved`), registrável em relatório de diagnóstico.

### 4.2. Remapeamento de Bornes (`PinRemap`)

Quando um `.qet` traz bornes genéricos (ex.: `["1", "2"]` em vez de `["13", "14"]` ou `["A1", "A2"]`):
- O `SchematicItem` preserva os índices geométricos dos pinos do SVG para conexão de condutores no Canvas, enquanto o dicionário expõe os nomes normalizados IEC/ISO para **BOM, Réguas de Bornes e XRef** (ou preenche bornes vazios segundo `canonical_pins`).

### 4.3. Caminho de Exportação (RustSimu -> Externo)

- **Exportar para `.cad` (`src/io.rs`):**
  1. Se `item.tipo` já for código legado nativo CADe_SIMU válido, usa `item.tipo`.
  2. Senão, consulta `concept.cade_tipo`: um botão ou contator vindo do QET ou da biblioteca canônica exporta com o `tipo` numérico exato do CADe_SIMU (`8000`, `2001`, etc.).
  3. Itens sem equivalente no CADe_SIMU 4.0 (ex.: válvula pneumática 5/2 ou bloco Ladder) geram diagnóstico estruturado no resumo de exportação em vez de falharem silenciosamente.
- **Exportar para `.qet` (`src/qet.rs`):**
  1. Se houver `qet_primary_path` ou `qet_type` original válido, exporta para o `.elmt` correspondente no QET.
  2. Para itens originados do CADe_SIMU ou nativos RustSimu sem `qet_type` prévio, mapeia via `concept.qet_primary_path` (e gera `<definition>` compatível quando necessário), eliminando placeholders opacos do tipo `type_8000`.
- **Exportar BOM / CSV / Relatórios (Seção 1.3 linha 68 & P2 #17):**
  - Agrupa todos os itens por `(concept_id, valor/especificação)` com descrição traduzida pelo `concept.name_pt` / `concept.name_en` e lista de TAGs (`-S1, -S2`), garantindo consistência total independente da origem do arquivo.

---

## 5. Plano de Implementação Incremental (4 Fases)

| Fase | Escopo Técnico | Arquivos Principais | Critério de Pronto |
|---|---|---|---|
| **Fase 1: Núcleo do Dicionário + 67 Símbolos CADe_SIMU** | Criar `src/dictionary.rs` (`SimulationDomain`, `ContactForm`, `CanonicalConcept`, `ConceptDictionary`, `ResolutionConfidence`) e popular o mapeamento 100% determinístico de todos os 67 `SymbolDef` de `src/symbols.rs` + famílias base de comandos (botões NA/NF/duplo/comutador/cogumelo/emergência, contatos, bobinas, sinaleiros, motores, fusíveis, disjuntores) com aliases multi-idioma QET. | `src/dictionary.rs`, `src/model.rs`, `src/lib.rs`, `symbols_qet/catalog.json` | 67/67 símbolos CADe_SIMU resolvidos em `O(1)`; exemplos reais QET (EN/FR/DE/PT) convergindo para o mesmo `concept_id`; testes unitários verdes. |
| **Fase 2: Integração Bidirecional `.cad` e `.qet`** | Conectar `resolve_to_concept` na leitura em `src/io.rs` e `src/qet.rs` (preenchendo `concept_id` e resolvendo `cade_tipo` / `symbol_id`), e conectar tradução reversa na gravação/exportação `.cad` e `.qet`. | `src/io.rs`, `src/qet.rs`, `src/app.rs` | Roundtrip `.cad -> RustSimu -> .qet -> RustSimu -> .cad` preservando equivalência de botão monoestável NA, bobinas, contatos e motores sem `type_20000`. |
| **Fase 3: Unificação de Simulação, Inspetor e Auditoria dos 9.273 Símbolos** | Fazer `sim/solver.rs`, `sim/rules.rs`, `xref.rs` e `ui/properties.rs` consultarem `concept_id` em primeira instância (deixando heurísticas textuais apenas como último fallback); criar relatório CLI/teste de cobertura para classificar os `4.407` itens hoje em `passive` e preparar os subconjuntos `pneumatic` (343), `hydraulic` (94) e `logic` (75). | `src/sim/solver.rs`, `src/sim/rules.rs`, `src/xref.rs`, `src/ui/properties.rs` | Simulação idêntica para componentes equivalentes vindos de `.cad`, `.qet` ou biblioteca canônica; painel mostrando o nome conceitual normalizado. |
| **Fase 4: Desbloqueio da Seção 1.3 (linha 68) e P2 #17** | Usar o `ConceptDictionary` como base para numeração de conectores, exportação BOM/CSV unificada e tabelas `i18n` (`pt`/`en`). | `src/export.rs`, `src/xref.rs`, `src/ui/` | BOM/CSV agrupa símbolos equivalentes por `concept_id` e idioma selecionado (`pt`/`en`). |

---

## 6. Decisões Definidas para Implementação

1. **Preservação dos SVGs em disco e da geometria importada:**
   - Todos os arquivos SVG existentes no repositório permanecem inalterados neste momento.
   - Ao importar um `.qet`, mantém-se a aparência visual/geometria original do `.elmt` no Canvas enquanto o `concept_id` canônico é vinculado em paralelo (garantindo fidelidade de fios/bornes originais e preparando futura opção explícita de padronização visual).
2. **Armazenamento híbrido das tabelas de equivalência:**
   - Os 67 símbolos nativos CADe_SIMU + matriz base multi-idioma ficam em Rust (`src/dictionary.rs`), extensíveis via `concepts.json` carregado junto ao `catalog.json` para acomodar centenas de mapeamentos QET, Pneumática, Hidráulica e Ladder.
3. **Critério de validação visual ao final das fases:**
   - Ao concluir a integração bidirecional, os esquemas de [Projetos](file:///home/caiojordao84/Downloads/CADE%20SIMU%204.0,%20PC%20SIMU%203.0/CADe_SIMU/CADe_SIMU/cade-simu-rs/Projetos) serão exportados em formato `.qet` nessa pasta para validação visual direta no QElectroTech (verificando presença e posicionamento de todos os elementos e condutores).
