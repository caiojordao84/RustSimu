# RustSimu

> **Editor CAD Eletrotécnico, Simulador de Comandos Elétricos e PLC Ladder/FBD nativo em Rust.**

O **RustSimu** é uma reescrita moderna, performática e de código aberto inspirada no clássico CADe_SIMU, agora também com um **simulador PLC Ladder Logic + FBD** (iced 0.13). Multiplataforma (Linux, Windows e macOS) com renderização vetorial e aceleração de hardware.

---

## PLC Ladder / FBD Simulator

Native rewrite of the web PLC trainer (contacts, coils, seal-in, RS latch, NOT, I/O panel, scan engine).

```bash
# Requires rustc ≥ 1.89 (see rust-toolchain.toml) and iced 0.13
cargo run --bin plc-simu
```

What works in the MVP:
- **Layout**: instruction palette, canvas, toolbar, I/O + status, properties
- **Ladder**: rungs, NO/NC/edge contacts, coils (out/set/reset/neg), TON, place/select/address edit, zoom
- **FBD**: Input/Output Var, NOT, AND/OR, RS/SR, TON, port wiring
- **Scan**: RUN/STOP, Step, cycle count, scan time; I0.x (Switch/NO/NC) → Q0.x / M bits
- **Demo**: motor start/stop with seal-in (Ladder + equivalent FBD), Save/Load JSON

Stubbed / not yet ported from the web app: full timer/counter/compare/math catalogs, traffic-light and other presets, share/export pack, fullscreen canvas chrome.

Toolchain note: the assignment asked for rustc **1.86.0**, but this repository’s existing CAD sources and Cargo.lock already require **≥ 1.88/1.89** (`is_multiple_of`, `slice::as_chunks`, ICU crates). The PLC app targets **iced 0.13** as specified; `rust-toolchain.toml` pins **1.89.0**.

---

## ⚡ Principais Recursos (CAD Eletrotécnico)

- **Simulação Eletrotécnica em Tempo Real**:
  - Circuitos de potência trifásicos e monofásicos (L1, L2, L3, N, PE) e corrente contínua (DC+, DC-).
  - Motores elétricos com indicação de sentido de rotação (horário/anti-horário) e proteção contra falta de fase.
  - Intertravamento de contatores, relés térmicos, botões de comando pulsadores e retentivos.
  - Temporizadores eletrônicos (retardo na energização e pulsos com contagem regressiva visual).
  - Sinalizadores luminosos configuráveis (Verde, Vermelho, Amarelo/Âmbar, Azul, Branco) e lâmpadas pulsantes.

- **Compatibilidade e Formatos de Arquivo**:
  - Compatibilidade com arquivos `.cad` originais do CADe_SIMU.
  - Formatos nativos estruturados em `.json` e `.toml`.
  - Importação de esquemas do **QElectroTech** (`.qet`).
  - Exportação de diagramas em **PDF**, **SVG** e **CAD**.

- **Interface Moderna e Dinâmica**:
  - **Suporte a Temas (Light, Dark e System)**: A folha de desenho, símbolos e condutores adaptam-se automaticamente ao contraste ideal, respeitando cores personalizadas pelo usuário.
  - **Estrutura Multi-Páginas**: Criação, reordenação e renomeação de múltiplos fólios em um mesmo projeto.
  - **Barra de Ferramentas Vetorial**: Ícones puros com tooltips contextuais.
  - **Padronização IEC**: Normas IEC 60947-1 e IEC 81346 para identificação e numeração automática de bornes.
  - **Autoconexão Inteligente**: Traçado automático de condutores ao alinhar componentes.

---

## 🚀 Como Executar

### Pré-requisitos

1. Tenha o ambiente **Rust** e o **Cargo** instalados (versão estável recente):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. No **Linux**, certifique-se de que as bibliotecas gráficas e de fontes estão instaladas:
   - **Ubuntu/Debian**:
     ```bash
     sudo apt update && sudo apt install -y pkg-config libxkbcommon-dev libfontconfig1-dev
     ```
   - **Fedora**:
     ```bash
     sudo dnf install libxkbcommon-devel fontconfig-devel
     ```
   - **Arch Linux**:
     ```bash
     sudo pacman -S libxkbcommon fontconfig
     ```

### Executando a Aplicação

**PLC Ladder/FBD:**

```bash
cargo run --bin plc-simu
# or release:
cargo run --release --bin plc-simu
```

**CAD eletrotécnico (padrão):**

```bash
cargo run --release
# or:
cargo run --bin cade-simu-rs
```

### Executando os Testes

O projeto conta com uma suíte abrangente de testes unitários e de integração:

```bash
cargo test
```

---

## 📁 Estrutura do Repositório

```text
├── Cargo.toml          # Configuração do projeto e dependências
├── rust-toolchain.toml # Pin rustc 1.89.0
├── src/
│   ├── main.rs         # CAD eletrotécnico (cade-simu-rs)
│   ├── bin/plc-simu.rs # PLC Ladder/FBD entry point
│   ├── plc/            # PLC model / scan / demo / ui
│   ├── canvas_view.rs  # CAD canvas
│   ├── sim/            # Motor de simulação elétrica
│   ├── model.rs        # Esquemas CAD
│   └── ...
├── symbols_qet_all/    # Biblioteca QET
└── Projetos/           # Exemplos CAD
```

---

## 📋 Teclas de Atalho Principais

| Ação | Atalho |
|---|---|
| **Novo Projeto** | `Ctrl + N` |
| **Abrir Arquivo** | `Ctrl + O` |
| **Salvar** | `Ctrl + S` |
| **Desfazer / Refazer** | `Ctrl + Z` / `Ctrl + Shift + Z` |
| **Cortar / Copiar / Colar** | `Ctrl + X` / `Ctrl + C` / `Ctrl + V` |
| **Rotacionar Símbolo** | `R` (sentido horário) / `Shift + R` (grupo) |
| **Espelhar Horizontal / Vertical** | `M` (Horizontal) / `F` (Vertical) |
| **Alternar Grade / Snap** | `Ctrl + Shift + G` |
| **Iniciar / Parar Simulação** | Botões de transporte na barra de ferramentas |
| **Cancelar Ação** | `Esc` ou Botão Direito do Mouse |

---

## 📄 Licença

Este projeto é desenvolvido para estudo, projeto e ensino de comandos elétricos e automação industrial.
