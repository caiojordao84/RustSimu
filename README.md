# RustSimu

> **Editor CAD Eletrotécnico e Simulador de Comandos Elétricos nativo em Rust.**

O **RustSimu** é uma reescrita moderna, performática e de código aberto inspirada no clássico CADe_SIMU. Desenvolvido em **Rust** utilizando o toolkit gráfico **Iced**, o software é multiplataforma (Linux, Windows e macOS) e oferece renderização vetorial suave com aceleração de hardware.

---

## ⚡ Principais Recursos

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

Para compilar e iniciar o **RustSimu**:

```bash
cargo run --release
```

Para executar o modo de desenvolvimento:

```bash
cargo run
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
├── Cargo.lock          # Versões fixadas das dependências
├── src/                # Código-fonte Rust
│   ├── main.rs         # Ponto de entrada, janelas, menus e atalhos
│   ├── canvas_view.rs  # Motor de desenho gráfico da folha e componentes
│   ├── sim.rs          # Motor de simulação elétrica e lógica
│   ├── model.rs        # Estruturas de dados dos esquemas e elementos
│   ├── io.rs           # Serialização e parser (.cad, .json, .toml, .qet)
│   ├── symbols.rs      # Catálogo e primitivas de símbolos nativos
│   ├── svg_symbol.rs   # Renderizador de símbolos vetoriais SVG
│   ├── icons.rs        # Ícones vetoriais da barra de ferramentas
│   ├── layout.rs       # Gerenciador de painéis e docas
│   └── theme.rs        # Paleta de cores e temas (Light/Dark/System)
├── symbols_qet_all/    # Biblioteca completa de símbolos elétricos e catálogo JSON
└── Projetos/           # Esquemas elétricos de exemplo para testes e simulação
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
