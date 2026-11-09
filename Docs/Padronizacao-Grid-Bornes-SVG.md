# Padronização Rígida de Grade, Espaçamento de Bornes e Dimensões SVG

## 1. Visão Geral e Motivação

Nos esquemas elétricos industriais e diagramas de comando e potência (normas IEC 60947 / IEC 60617 / DIN), a interoperabilidade entre ferramentas gráficas vetoriais (como Inkscape, Illustrator, AutoCAD Electrical, QElectroTech e CADe_SIMU) exige proporcionalidade matemática exata.

Anteriormente, os arquivos SVG da biblioteca possuíam escalas heterogêneas dependendo de sua origem:
* Símbolos nativos gerados do CADe utilizavam pitch de 30 px entre bornes e raio de 2.5 px.
* Símbolos importados do QElectroTech (QET) utilizavam pitches de 20 px, 40 px ou 80 px e raio de borne de 1.8 px.
* Diagramas exportados em SVG usavam um fator de 3.2 px por unidade interna (19.2 px entre fases).

Como resultado, ao abrir arquivos SVG individuais ou esquemas exportados diretamente no Inkscape ou visualizadores externos, as dimensões não coincidiam com a grade métrica de impressão a 300 DPI, causando desalinhamentos e discrepâncias de escala.

Esta especificação define e documenta a padronização definitiva e uniforme aplicada a **todos os arquivos SVG em disco**, **motores de conversão**, **canvas interativo** e **esteira de exportação gráfica**.

---

## 2. Parâmetros da Grade e Geometria dos Bornes

A plataforma adota um sistema rígido baseado na resolução padrão de impressão técnica de 300 DPI, relacionando unidades físicas em milímetros, unidades de grade lógica do CADe_SIMU e coordenadas de pixels vetoriais:

| Parâmetro | Milímetros (mm) | Pixels a 300 DPI (px) | Unidades CADe Internas | Função no Sistema |
| :--- | :---: | :---: | :---: | :--- |
| **Módulo Principal do Grid** | **4.0 mm** | **48.0 px** | **6.0 unidades** | Espaço rígido entre bornes de fases adjacentes ($L_1 \to L_2 \to L_3$, $1 \to 3 \to 5$, $U_1 \to V_1 \to W_1$) |
| **Meio Módulo (Sub-grid)** | **2.0 mm** | **24.0 px** | **3.0 unidades** | Centralização de corpos de bobinas, contatos auxiliares e eixos simétricos |
| **Diâmetro dos Bornes** | **0.67 mm** | **8.0 px** | **1.0 unidade** | Diâmetro visual dos círculos de conexão elétrica dos componentes |
| **Raio dos Bornes** | **0.33 mm** | **4.0 px** | **0.5 unidade** | Raio dos círculos de bornes (`r="4.0"`) |

### Constantes em Código (`src/model.rs`)

```rust
// Grade métrica e proporcionalidade rígida da plataforma (300 DPI)
pub const GRID_MODULE_MM: f32 = 4.0;
pub const GRID_MODULE_PX_300DPI: f32 = 48.0;
pub const GRID_MODULE_UNITS: f32 = 6.0;

pub const SUB_GRID_MM: f32 = 2.0;
pub const SUB_GRID_PX_300DPI: f32 = 24.0;
pub const SUB_GRID_UNITS: f32 = 3.0;

pub const TERMINAL_DIAMETER_PX_300DPI: f32 = 8.0;
pub const TERMINAL_RADIUS_PX_300DPI: f32 = 4.0;
pub const TERMINAL_RADIUS_UNITS: f32 = 0.5;
```

---

## 3. Relação de Conversão e Escala

### 3.1 Símbolos Nativos do CADe_SIMU
No CADe_SIMU original (C++ / MFC), as coordenadas internas de fiação operavam em passos de 6 unidades, enquanto os comandos vetoriais brutos continham 30 unidades de desenho por módulo.
Para obter 48.0 pixels no módulo principal:
$$\text{Fator de Escala Vetorial} = \frac{48.0\text{ px}}{30.0\text{ unidades brutas}} = 1.6$$
$$\text{Fator de Conversão para Unidades CADe} = \frac{6.0\text{ unidades}}{48.0\text{ px}} = 0.125 = \frac{1}{8.0}$$

Todos os 134 símbolos em `symbols_qet_all/cade_native` (pares de repouso e acionamento ativo) foram gerados por `src/bin/cade2svg.rs` com essa proporção, posicionando os pinos em múltiplos exatos de 48.0 px com `r="4.0"`.

### 3.2 Símbolos Importados de Fontes Externas (QET, Multisim, EPLAN)
Para qualquer símbolo externo com passo horizontal ou vertical de bornes $dx$ ou $dy$:
$$\beta = \frac{48.0}{\text{passo}}$$

* Símbolos com passo base de 20 px: $\beta = 48.0 / 20.0 = 2.4$.
* Símbolos com passo de 40 px: $\beta = 48.0 / 40.0 = 1.2$.
* Símbolos com passo de 80 px: $\beta = 48.0 / 80.0 = 0.6$.
* Símbolos com passo de 30 px: $\beta = 48.0 / 30.0 = 1.6$.
* Símbolos sem bornes ou com único borne: $\beta = 2.4$ (escala padrão QET).

---

## 4. Estrutura dos Arquivos SVG Padronizados em Disco

Todos os arquivos SVG nas pastas `symbols_qet_all/` e `symbols_qet/` possuem:

1. Atributo declarativo na raiz `<svg>`:
   ```xml
   <svg xmlns="http://www.w3.org/2000/svg" width="..." height="..." viewBox="..." data-standard="48px">
   ```
2. Círculos de bornes elétricos identificados com a classe `qet-terminal`:
   ```xml
   <circle class="qet-terminal" cx="0.00" cy="0.00" r="4.0" fill="#dc2626" stroke="none" data-label="1" />
   <circle class="qet-terminal" cx="48.00" cy="0.00" r="4.0" fill="#dc2626" stroke="none" data-label="3" />
   <circle class="qet-terminal" cx="96.00" cy="0.00" r="4.0" fill="#dc2626" stroke="none" data-label="5" />
   ```
3. Geometrias de linhas, retângulos, elipses, polígonos e caminhos de arcos escalados por $\beta$ com espessuras de linha ajustadas (`stroke-width="2.4"` no grupo principal) para alta nitidez em visualizadores vetoriais.

---

## 5. Esteira de Exportação Gráfica (`src/export.rs`)

Ao exportar esquemas completos ou seleções de elementos:

* A constante `SVG_SCALE` foi definida em `8.0` pixels por unidade CADe:
  $$6.0\text{ unidades CADe} \times 8.0 = 48.0\text{ pixels}$$
  $$3.0\text{ unidades CADe (sub-grid)} \times 8.0 = 24.0\text{ pixels}$$
  $$0.5\text{ unidades CADe (raio)} \times 8.0 = 4.0\text{ pixels (diâmetro 8.0 px)}$$
* Ao abrir o SVG exportado no Inkscape:
  - As distâncias entre fases medem exatamente 48 px (4.0 mm).
  - O grid do Inkscape configurado em 4.0 mm ou 48 px alinha perfeitamente com todas as conexões e fios.
  - A opção `show_terminals` desenha os círculos de borne em cada pino com raio $r = 4.0\text{ px}$.
* Exportação PNG: renderiza o SVG a 300 DPI utilizando a árvore `usvg` e a fonte TrueType Inter embutida (`include_bytes!`).
* Exportação PDF: converte a árvore `usvg` em PDF vetorial puro sem rasterização via `svg2pdf` e `pdf-writer`.

---

## 6. Ferramentas de Manutenção e Migração

* **`src/bin/cade2svg.rs`**: Regenera todos os 134 símbolos nativos do CADe no formato SVG animado a 48 px.
* **`src/bin/elmt2svg.rs`**:
  - `cargo run --bin elmt2svg`: Converte arquivos `.elmt` diretamente para o padrão de 48 px com $r = 4.0$.
  - `cargo run --bin elmt2svg -- --migrate`: Migra todos os SVGs padronizados de `symbols_qet_all/` para `symbols_qet/` no formato canônico D2 (`<subcat:3>-<id:5hex>-<estado:1hex>.svg`).
  - `cargo run --bin elmt2svg -- --check`: Valida as invariantes D2 em disco (9.327 arquivos verificados).
* **`scripts/standardize_library_svgs.py`**: Script de auditoria e normalização em lote de bibliotecas SVG.
