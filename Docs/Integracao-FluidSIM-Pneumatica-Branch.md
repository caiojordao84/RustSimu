# Integração FluidSIM Pneumática — Anotação da Branch `pneumatica`

Documento de registro do trabalho feito nesta branch até o momento (abrir/exportar `.ct`, motor de ar, importação visual com grade/bornes/tubos/texto). Complementa `integracao-rustsimu/TODO.md` e `integracao-rustsimu/README.md` (laboratório) e a especificação de grade em `Docs/Padronizacao-Grid-Bornes-SVG.md`.

**Branch:** `pneumatica`  
**Laboratório:** `integracao-rustsimu/testes/ct-parser`  
**Destino:** `cade-simu-rs` (`src/ct/`, `src/pneu/`, `src/sim/pneumatic.rs`, …)

---

## 1. Objetivo

Trazer circuitos FluidSIM Pneumatic (`.ct`) para o RustSimu com:

- parse fiel do binário (Str2 + LZ77 COMPRESS);
- símbolos na grade rígida 48 px / 6 u (`data-generator="cade2svg"`, `data-standard="48px"`);
- tubos Manhattan a partir da geometria real do FluidSIM;
- bornes alinhados aos ports e à grade;
- texto nativo (`tipo == 8`);
- motor de ar + elo eletropneumático;
- Abrir / Exportar `.ct` na UI, no mesmo espírito do `.cad`.

---

## 2. Linha do tempo (P0 → P9 + polish visual)

| Etapa | Entrega | Onde |
| :--- | :--- | :--- |
| **P0** | Contrato local: `NoAr` ≠ `Potential`; válvula = tabela de vias; esquema alinhado a `SchematicItem` | `ct-parser/contrato.rs` |
| **P1** | Dicionário `ep.kb` → `concept_id`, bornes ISO, `simular` | `ct-parser/dicionario.rs` + `cade-simu-rs/dictionary.rs` / `fluidsim_kb.rs` |
| **P2** | Escala 10240 un → 48 px; SVG repouso/acionado; bornes ops 10–15 | `escala.rs`, `pneu/symbols.rs` |
| **P3** | `MotorAr` (5/2, cilindro, PVS1/PVS3, fim de curso) | `motor.rs` → `sim/pneumatic.rs` |
| **P4** | Solenóide A1–A2, impulso, pressostato | mesmo motor + solver elétrico |
| **P5** | Adaptador `Pagina`+`Rede` → esquema | `adaptador.rs` |
| **P6** | Circuitos de prova (critério de merge) | `tests/prova.rs` |
| **Merge** | `CanonicalKind`, paleta, Abrir `.ct`, `MotorAr`↔`SimState` | branch `pneumatica` |
| **P7** | Curso contínuo; P7.2; PIC completo; QET map; `lang_map` 0x0200 | `pneu/*`, `ct/lang_map.rs` |
| **P8** | Corpus `TP*.ct` 132/132 (`CT_CORPUS_STRICT=1`) | `tests/tp_corpus.rs` |
| **P9** | UI Abrir/Exportar `.ct` (Str2 + LZ77 writer) | `ct/export.rs`, `app.rs`, `export.rs` |
| **Polish** | Tubos `L`, âncora por Connections, bornes na grade, textos, só-visuais | `adaptador.rs`, `ct/import.rs`, `pneu/symbols.rs` |

---

## 3. Arquitetura no RustSimu

```
.ct binário
  → ct::decifra (Str2 / lang_map + LZ77)
  → ct_parser::Pagina (+ Rede union-find)
  → ct_parser::adaptador::adaptar  → Esquema + textos + ligações
  → ct::import::schematic_from_ct_*  → SchematicDocument
       • itens tipo 20000 + symbol_id (classe FluidSIM)
       • cabos 4000 (P) / 4003 (E)
       • textos tipo 8
  → canvas (SVG cade2svg) + sim/pneumatic + solver elétrico
```

### Módulos novos / relevantes

| Caminho | Papel |
| :--- | :--- |
| `src/ct/` | Decifra, import, export, LZ77, `lang_map` |
| `src/pneu/` | PIC, SVG, motor espelhado, paleta, QET map, contrato |
| `src/sim/pneumatic.rs` | Ponte `MotorAr` ↔ estado do canvas |
| `src/fluidsim_kb.rs` | Sementes / classes FluidSIM no dicionário |
| `testes/ct-parser/` | Parser, adaptador, dicionário, motor (fonte da verdade do laboratório) |

Dependência de path: `ct-parser = { path = ".../integracao-rustsimu/testes/ct-parser" }`.

---

## 4. Formato `.ct` (o que importa para o import)

### 4.1 Empacotamento

- Cabeçalho / corpo cifrado (Str2; modo hash `0x0200` via `lang_map.bin`).
- Camada `COMPRESS`: LZ77 janela 2048, seed `"PREVIEW"`, máscara XOR `0x9a`.
- Exportação RustSimu: `ct::export_doc_to_ct` (Str2 + literais LZ77) — folha única.

### 4.2 Corpo textual

- Objetos: `<Tipo> O<id> x1 y1 x2 y2 flag ang cor`.
- Propriedades LISP: ` S nome T valor`.
- `gateN # comp conn` — porta N do componente → Connection.
- `CON PLine|DLine|… a b` — topologia entre Connections (union-find).
- `L x1 y1 x2 y2` **nas Connections** — geometria visual do tubo (AABB relativos ao **centro** do bbox).
- `text` — anotações (`prop text`, `height`, …).
- `SYM` — desenho por estado (usado no laboratório; o canvas pneumático usa PIC/`cade2svg`).

### 4.3 PIC.BIN

- 347 records; comandos 33 B; ops 1/3/4/5/6/7/8 = tinta; ops 10–15 = terminais (`ps[4],ps[5]` relativos à âncora).
- Classe → pic: `nome_da_classe + ".PIC"` (ex.: `PVWM8.PIC`).
- Aliases CT → classe: `WV_5`→`PVWM8`, `WV_3`→`PVWM1`, `CylDGPPAB`→`PAL2`, etc.

---

## 5. Grade e SVG (regra RustSimu)

Alinhado a `Padronizacao-Grid-Bornes-SVG.md`:

| Grandeza | Valor |
| :--- | ---: |
| Módulo | 4 mm / **48 px** / **6 u** |
| Submódulo | 2 mm / 24 px / 3 u |
| Raio do borne | **4.0 px** |
| Fator canvas cade2svg | **0.125** (48→6) |

### Geração `cade2svg` (`pneu/symbols.rs`)

- ViewBox relativo à âncora PIC `(X0,Y0)`.
- Atributos: `data-generator="cade2svg"` e `data-standard="48px"`.
- Terminais `class="qet-terminal"` em **múltiplos de 48 px**.
- Extremidades de `<line>` a ≤ ½ módulo de um borne são **atraídas** para o borne (tinta do port e conector coincidem na grade).
- `PAL1` e instrumentos P8 (`pol1`/`pol2`/Generator/…) são procedurais.

---

## 6. Adaptador e import visual (estado atual)

### 6.1 Itens

- Qualquer classe do dicionário mapeável entra (simulável **e** só-visual / potência catalogada).
- `*Connection` / `*Triconnection` nunca viram item.
- Posição do item = **âncora encaixada nas Connections**:
  \[
  \text{âncora} \approx \mathrm{média}\big(\mathrm{conn}_{\text{porta}} - R(\mathrm{offset}_{\text{PIC}})\big)
  \]
  Assim os `qet-terminal` caem onde o FluidSIM amarrou os tubos. Fallback: `bbox_tl + (X0,Y0)`.
- Orientação: só passos de 45° estáveis; ângulos “de configuração” (ex. −30° em `WV_5`) → 0.
- Tags: prefixo do conceito + id (`-V3`, `-A0`, …).

### 6.2 Tubos

1. Em cada `CON`, a geometria mora nas `L` de **um** dos lados.
2. Cada `L` = AABB (cantos opostos relativos ao centro da Connection).
3. Centerline H ou V; cantos H↔V costurados (90°).
4. Stubs Manhattan até os bornes PIC do item (liga path → conector).
5. No `ct/import.rs`: px→mundo (módulo 6); garante eixos H/V após snap.

**Não** usar stubs `gate→conn` a partir do centro do símbolo (gerava o “starburst”).

### 6.3 Texto

- Objetos `tipo == "text"` → `TextoEsquema` → `SchematicItem` **`tipo == 8`** (`trailing_str`, altura tipográfica, `sync_text_bounds`), igual ao import QET / ferramenta de texto.

### 6.4 Elétrico no import

- Presença de conceitos elétricos ⇒ cabos tipo `4003` (DC).
- `pol1`/`pol2` ⇒ tipos nativos `3009`/`3008` (DcNeg/DcPos) para o solver.

---

## 7. Simulação

- **Ar:** `MotorAr` / `sim/pneumatic.rs` — nós `Suprimento` / `Escape` / `SemPressao`; vias de válvula; cilindro (binário + `EstadoPistao` contínuo); PVS1/PVS3; P7.2 (retenção, temporizador, vácuo, motor).
- **Elo EP:** energização A1–A2; impulso retentivo; pressostato exporta contato ao solver elétrico (sem propagar L1/N no motor de ar).
- **Grafo:** pinos de símbolos `20000` vêm dos `qet-terminal` do SVG; extremos de cabo precisam **coincidir** (inteiro) com o pin.

---

## 8. UI

- **Abrir:** filtro `*.ct` / `*.CT` (mesmo fluxo de `load_project`).
- **Exportar:** `ExportFormat::Ct` no modal; `save_document` / `save_project` aceitam `.ct`.
- **Paleta:** Pneumática / Eletropneumática (v1 + P7.2 + P8); `ensure_svg_registrados` também cobre classes do catálogo com PIC (só-visuais).

---

## 9. Testes de referência

| Teste | O que garante |
| :--- | :--- |
| `ct-parser` `tests/adaptador.rs` | Topologia TP101, tubos Manhattan, textos, âncora PPE1 ↔ Connection |
| `ct::import::importa_tp101_com_escala_de_grade` | Import + `data-standard` + bornes no módulo |
| `tests/ct_import_diag.rs` | Bornes on-grid, tubos nos pins, textos, `R_SCHALT` presente |
| `tests/tp_corpus.rs` | 132/132 `TP*.ct` abrir+simular (`CT_CORPUS_STRICT=1`) |
| `ct::export::export_ct_roundtrip_minimo` | Round-trip mínimo `.ct` |
| `pneu::symbols::pic_embutido_e_pal1_padrao_canvas` | SVG cade2svg + grade |

---

## 10. Problemas encontrados e decisões

| Sintoma | Causa | Decisão |
| :--- | :--- | :--- |
| Starburst de tubos cinza | Condutores = stubs gate→centro | Tubos só de `CON` + `L` |
| Tip da fonte longe do triângulo | Item no centro do bbox | Âncora PIC / encaixe por Connections |
| Bornes fora da tinta e fora do grid | Snap só do terminal, tinta solta; âncora TL com bbox ≠ PIC | Snap 48 px + atração de linhas; âncora = média(conn − offset) |
| Página “vazia” em potência | Só `simular: true` virava item | Importa todo conceito do dicionário com classe mapeável |
| Rotação estranha em `WV_5` | Ângulo −30° tratado como ori 7 | Só aceitar passos ~45° (ortogonais 0..=3) |
| Manômetro ausente (`152841`) | Tokenizer `eh_nome_simb` rejeitava part numbers que iniciam por dígito | Aceitar alfanumérico e `_`/`-` no tokenizer |
| Laços em U ao redor de bobinas e relés | Bornes A1/A2 sofriam offset lateral forçado de piloto de válvula | Manter bornes A1/A2 no alinhamento vertical PIC para componentes puramente elétricos (`E_K001`, `E_Y001`) |
| Tocos horizontais espúrios em conexões verticais | Caixas de junção 10240×10240 e segmentos curtos classificados como H | Ignorar caixas de junção quadradas redundantes e usar espessura 10240 para determinar eixo Manhattan |
| Cilindros de simples ação com setas e portas fantasmas | `CylDGPPAB` mapeado estaticamente para dupla ação (`PAL2`) | Inspecionar `MoName` e `description` para mapear cilindros simples para `PAL1` e renderizar símbolo procedural ISO 1219 limpo (sem setas diagonais) com pistão, haste e mola de retorno |
| Rótulos sintéticos (-S6, -K15) no canvas | Tags geradas apenas por `tag_prefix + id` | Extrair `label`, `ID` ou `identification` de `obj.props` quando presente |
| Desordem nas portas da válvula 5/2 | Portas PIC e gates fora de ordem ISO | Mapeamento explícito ISO 1219 (portas 4 e 2 no topo; 5, 1, 3 na base com silenciadores) e símbolo canônico procedural |
| Tubos pneumáticos com cor de condutor elétrico | Mesma classe `4003` para ambos | Segregação estrita: condutores elétricos `4003` e tubos pneumáticos `4005` (renderizados com `tok.ink` respeitando temas claro/escuro e azul sob pressão) |

**Limitações conscientes**

- Válvulas configuráveis FluidSIM (`WV_5` com muitos gates/atuadores) usam PIC de classe aproximado (ex. `PVWM8` com 3 terminais vs 5 ports reais): o encaixe pela média reduz o erro, mas o desenho PIC pode não cobrir todos os atuadores.
- Tipos fora do `ep.kb` / sem `classe_de_tipo_ct` continuam em `ignorados` (ex. partes digitais puras `OrD1`, timers não catalogados).
- Export `.ct` é folha única e não reconstrói toda a riqueza LISP/SYM do FluidSIM.

---

## 11. Como validar manualmente

1. Compilar o RustSimu na branch `pneumatica`.
2. Abrir `ct/TP101/TP101_17.ct` (ou Schaltkreis / TPs em `Downloads/ct`).
3. Conferir: bornes vermelhos **sobre** os ports e **nos cruzamentos** da grade; tubos H/V até os bornes; textos `1V1`/`1A1` como ferramenta de texto; title block “FluidSIM Pneumatic Import”.
4. Opcional: `CT_CORPUS_STRICT=1 cargo test --test tp_corpus`.

---

## 12. Próximos passos naturais (ainda abertos)

- Mapear classes de potência/comando que ainda caem em `ignorados` (quando o utilizador apontar o `.ct` vazio).
- Melhorar PIC/SVG para válvulas configuráveis 5/2 com 5+ ports reais.
- Export `.ct` mais completo (props LISP, `L` nos tubos, multi-folha se necessário).
- Simulação didática ponta a ponta na UI sobre os TPs abertos (já coberta no harness P8).

---

## 13. Referências internas

- `integracao-rustsimu/TODO.md` — ordem P0–P9 e critérios de pronto  
- `integracao-rustsimu/README.md` — REA/Ghidra, scripts, status  
- `integracao-rustsimu/analise/NOTAS.md` — PIC, gates, CON, escala  
- `Docs/Padronizacao-Grid-Bornes-SVG.md` — grade 48 px  
- `Docs/Dicionario-Parser-Canonico-Bidirecional.md` — conceitos canônicos  
