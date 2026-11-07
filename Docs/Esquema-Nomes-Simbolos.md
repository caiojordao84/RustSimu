# Esquema de Nomes de Símbolos — RustSimu

> **Especificação de nomenclatura da biblioteca de símbolos** (decisão **D2**, 06/11/2026).
> Substitui os nomes-slug herdados do QElectroTech (`ac_motor_3-phases.svg`) por um identificador
> numérico curto, estável e ordenável. O dicionário JSON é a **única fonte de verdade**; o nome do
> arquivo é derivado dele.

---

## 1. Formato canônico

```
<familia>/<subcategoria:3>-<id:5hex>-<estado:1hex>.svg
```

**Exemplo:**

```
electric/012-00020-0.svg
electric/012-00020-1.svg
energy/007-0014A-0.svg
```

**Comprimento:** 15 caracteres incluindo a extensão (`012-00020-0.svg`).

| Campo | Largura | Domínio | Escopo | Exemplo |
|---|---|---|---|---|
| `familia` | — | pasta primária | global | `electric` |
| `subcategoria` | 3 decimal | `000`–`999` | **único dentro da família** | `012` |
| `id` | 5 hex maiúsculo | `00000`–`FFFFF` | **único globalmente** | `00020` |
| `estado` | 1 hex maiúsculo | `0`–`F` | enum do elemento | `0` |

Separador `-` (hífen). Não usar `x` — ambíguo com o sinal de multiplicação e hostil a glob/CLI.

### 1.1 Layout em disco

Somente as **pastas primárias** são preservadas:

```
symbols_qet/
├── electric/      8.5k arquivos
├── energy/
├── pneumatic/
├── hydraulic/
├── logic/
├── import/
└── cade_native/
```

A árvore interna do QElectroTech (até 9 níveis) é **achatada**: o caminho original sobrevive
apenas como metadado `path` no dicionário.

---

## 2. Por que 3 dígitos cabem

O código de subcategoria é escopado **por família**, então o teto é o maior ramo, não o total.
Medição sobre `catalog.json` (8.890 elementos):

| Família | Subcategorias-folha | Livres | Uso |
|---|---:|---:|---:|
| `electric` | **855** | **144** | 85,6% |
| `energy` | 81 | 918 | 8,1% |
| `pneumatic` | 45 | 954 | 4,5% |
| `hydraulic` | 16 | 983 | 1,6% |
| `logic` | 6 | 993 | 0,6% |
| **Total** | **1.003** | — | — |

O total (1.003) excede 999, mas **nunca precisa caber num único namespace** — cada família tem o seu.

### 2.1 Ponto de atenção: `electric` está em 85,6%

Restam **144 códigos** em `electric`. Política de esgotamento (v. §4):

1. Códigos são **permanentes e nunca reutilizados** — uma categoria removida deixa o código vago.
2. Ao esgotar, promover para **4 dígitos** (`0123-00020-0.svg`, 16 chars) sob `schema_version` novo,
   com migração automática pelo dicionário.

---

## 3. Regras invariantes

| # | Regra | Motivo |
|---|---|---|
| **R1** | `id` é global, imutável e nunca reutilizado | Chave primária estável; permite achatar a árvore sem colisão |
| **R2** | `subcategoria` é **atribuída e persistida** no dicionário | Recalcular por ordem alfabética quebraria todas as referências ao renomear uma pasta QET |
| **R3** | Hex sempre **maiúsculo** | Windows/macOS são case-insensitive; `0002a` e `0002A` colidiriam |
| **R4** | Estado `0` = repouso, sempre existe | Garante fallback determinístico |
| **R5** | O dicionário é a fonte de verdade; o nome é derivado | Impede divergência nome ↔ metadado |
| **R6** | Renomear categoria QET muda só o `path`, nunca o código | Estabilidade histórica |
| **R7** | Arquivo de estado só existe se a **geometria** muda | Cor, brilho, falha e curto são overlays de runtime (`canvas_view.rs`) |
| **R8** | `slug` original preservado no JSON | Mapa de migração e busca textual na UI |

### 3.1 Estados são enum por elemento, não bit flags

`0`–`F` = **16 códigos**, interpretados conforme o `kind` do elemento. Nunca flags ortogonais.

| `kind` | Estados |
|---|---|
| `motor` | `0` parado · `1` direta CW · `2` direta CCW · `3` estrela CW · `4` estrela CCW · `5` triângulo CW · `6` triângulo CCW |
| `switch` / `contact` | `0` aberto · `1` fechado · `2` desarmado |
| `lamp` | `0` apagada · `1` acesa |
| `source` | `0` inativo · `1` ativo |

**Overlays** (desenhados em runtime, nunca viram arquivo): `fault`, `short_circuit`, `rotation`,
`selected`, `manual_forced`. O curto-circuito é propriedade do **potencial do condutor**, não do
símbolo — já é pintado em vermelho espesso pelo canvas.

---

## 4. Política de capacidade

| Versão | Subcategoria | ID | Estado | Comprimento | Capacidade/família |
|---|---|---|---|---|---|
| `schema_version: 2` | 3 decimal | 5 hex | 1 hex | 15 | 1.000 × 978.670 × 16 |
| `schema_version: 3` (reserva) | 4 decimal | 5 hex | 1 hex | 16 | 10.000 × 978.670 × 16 |

`EEEEE` é um teto autoimposto; 5 hex vão até `FFFFF` (1.048.576). Mantido `EEEEE` por legibilidade
e por reservar a faixa `Fxxxx` para símbolos sintéticos/gerados.

---

## 5. Dicionário JSON

### 5.1 Estrutura

```json
{
  "schema_version": 2,
  "generated_at": "2026-11-06T12:00:00Z",
  "source": { "kind": "qelectrotech", "version": "0.9", "path": "/usr/share/qelectrotech" },

  "categories": [
    {
      "code": "012",
      "family": "electric",
      "path": "electric/10_allpole/391_consumers_actuators",
      "name_pt": "Consumidores e atuadores",
      "name_en": "Consumers and actuators"
    }
  ],

  "elements": [
    {
      "id": "00020",
      "slug": "ac_motor_3-phases",
      "category": "012",
      "family": "electric",
      "kind": "motor",
      "name_pt": "Motor trifásico",
      "name_en": "AC motor 3-phases",
      "source": {
        "qet_uuid": "{8f2c...}",
        "qet_path": "electric/10_allpole/391_consumers_actuators/10_engines/ac_motor_3-phases.elmt"
      },
      "svg_base": "012-00020-0.svg",
      "bbox": { "w": 40.0, "h": 40.0 },
      "hotspot": { "x": 20.0, "y": 20.0 },
      "states": [
        { "code": "0", "name": "parado" },
        { "code": "1", "name": "direta CW",     "svg": "012-00020-1.svg" },
        { "code": "2", "name": "direta CCW",    "svg": "012-00020-2.svg" },
        { "code": "3", "name": "estrela CW",    "svg": "012-00020-3.svg" },
        { "code": "4", "name": "estrela CCW",   "svg": "012-00020-4.svg" },
        { "code": "5", "name": "triângulo CW",  "svg": "012-00020-5.svg" },
        { "code": "6", "name": "triângulo CCW", "svg": "012-00020-6.svg" }
      ],
      "overlays": ["fault", "short_circuit", "rotation", "selected"],
      "terminals": [
        { "name": "U1", "x":  0.0, "y": 10.0, "orientation": "w" },
        { "name": "V1", "x":  0.0, "y": 20.0, "orientation": "w" },
        { "name": "W1", "x":  0.0, "y": 30.0, "orientation": "w" }
      ],
      "tags": ["motor", "trifasico", "carga"]
    }
  ]
}
```

### 5.2 Campo `kind` — obrigatório

Hoje o motor de simulação **infere o comportamento elétrico a partir do texto do slug**
(`sim/rules.rs`: `contains("push_button")`, `contains("breaker")`, `contains("fuse")`...). Com IDs
numéricos essa inferência desaparece; o `kind` a substitui e elimina as heurísticas por substring.

Valores canônicos:

```
momentary_button · latching_switch · selector_switch · emergency_stop · limit_switch
circuit_breaker · fuse · disconnector · relay_coil · contactor_contact · thermal_contact
motor · lamp · source · terminal · connector · load · passive · block
```

- **`block`** (Decisão **D3**): Blocos de montagem visual não cabáveis (gabaritos 2D de painel, trilhos DIN, molduras, batentes e módulos de CLP sem terminais de circuito). Elementos com `kind == "block"` são automaticamente ignorados na geração do grafo de nós elétricos (`build_network_graph`) e marcados com `is_wireable() == false`. Podem ser linkados por tag às peças funcionais em outras folhas pela aba de Referência Cruzada.

### 5.3 Regras de `states`

- `svg` é **opcional**: ausente ⇒ usa `svg_base` + overlay de runtime.
- `code` único por elemento e contido em `0`–`F`.
- `0` sempre presente.

---

## 6. Pipeline e validação

`cargo run --bin elmt2svg -- --check` valida as invariantes e falha com código de saída ≠ 0.
Um `#[test]` na suíte chama o mesmo validador.

| Invariante | Violações hoje | Gravidade |
|---|---:|---|
| Nomes de bornes únicos por elemento | **4.699** | 🔴 bloqueia a simulação (P3-#17) |
| Elementos sem terminal e blocos CLP | **3.571** | ✅ classificados como `kind: "block"` (P3-#18 / D3) |
| `name_pt` ≠ `name_en` | **4.345** | 🟡 metade não traduzida (P3-#19) |
| IDs com sufixo `_2`/`_3` | **2.533** | 🟡 resolvido pelo ID numérico (D2) |
| `name_pt` nulo/vazio | **349** | 🟡 (P3-#19) |
| `id` divergente do nome do arquivo | 37 | 🟡 transitório (some com D2) |
| IDs duplicados | **0** | ✅ (normalizado por `--index-only`) |
| SVG ausente no disco | **0** | ✅ |
| SVG no disco fora do catálogo | **0** | ✅ (9.327/9.327) |
| Hotspot fora da bbox | **0** | ✅ |
| Colisões case-insensitive | **10** | 🟠 `import/` — portabilidade Windows (P3-#20) |
| `subcategoria` única por família | — | ✅ (855/999) |

### 6.1 SVGs órfãos — resolvido em 06/11/2026

O modo `elmt2svg --index-only` varre a árvore e indexa qualquer SVG ausente do `catalog.json`,
sem exigir a coleção QElectroTech instalada. Resultado da execução:

| Origem | SVGs indexados |
|---|---:|
| `cade_native/` | 90 |
| `import/` | 338 |
| `electric/` (`_active`) | 9 |
| **Total** | **437** |

O catálogo passou de 8.890 para **9.327 entradas = 9.327 SVGs em disco (0 órfãos)**. O comando é
idempotente (segunda execução indexa 0) e o conversor normal o executa ao final, de modo que a
invariante "nenhum arquivo fora do catálogo" é estrutural, não manual.

Cada variante `_active` foi indexada com `state: "active"` e `variant_of` apontando para o
elemento-base — a ponte para o modelo de `states[]` da D2. Os `cade_native` recebem
`source: "cade_native"`; o restante, `source: "qet"`.

**Pendência remanescente:** 10 pares de arquivos em `import/` diferem apenas por caixa
(`Chassis_ground_001.svg` × `chassis_ground_001.svg`). No Linux coexistem; no Windows/macOS
apenas um é materializado. Requer decisão de remoção/renomeação (P3-#20).

---

## 7. Migração

| Antes | Depois |
|---|---|
| `symbols_qet_all/electric/.../ac_motor_3-phases.svg` | `symbols_qet/electric/012-00020-0.svg` |
| `SchematicItem.qet_type = "ac_motor_3-phases"` | `SchematicItem.symbol_id = "00020"` + `symbol_state = '0'` |
| `get_parsed_svg("ac_motor_3-phases")` | `get_parsed_svg("00020", '0')` |
| `sim/rules.rs` heurísticas `contains(...)` | `dict.kind("00020")` |

- `qet_uuid` **permanece** (rastreio de remoção em merge multipágina).
- `slug` **permanece** no JSON como mapa de migração.
- `.json`/`.qet` antigos: aplicar tabela `slug → id` na leitura.
- `.cad` **intocado** — usa `tipo: i32` numérico e mantém roundtrip byte-exato.

---

## 8. Resumo das decisões

| ID | Decisão |
|---|---|
| **D2.1** | Somente pastas primárias; árvore interna achatada (caminho vira metadado) |
| **D2.2** | `subcategoria` de 3 dígitos decimais, escopada por família |
| **D2.3** | `id` de 5 hex maiúsculos, global e imutável |
| **D2.4** | `estado` de 1 dígito hex, enum por `kind` (não bit flags) |
| **D2.5** | Separador `-`; 15 caracteres no total |
| **D2.6** | Arquivo de estado só quando a geometria muda; resto é overlay |
| **D2.7** | `kind` obrigatório no dicionário — substitui as heurísticas por substring |
| **D2.8** | Códigos de subcategoria persistidos, nunca recalculados |
