# Propriedades do Elemento e Referência Cruzada — Documentação Técnica (Seção 4)

Este documento descreve a implementação completa da **Seção 4** de `Docs/Modificações (1).md` no projeto **RustSimu**, compreendendo a reestruturação da barra lateral direita com dois seletores principais ("Propriedades" e "Referência Cruzada"), a edição padronizada de Nome e Função, padrões de bobinas especiais e temporizadores, sinalizadores luminosos com 6 cores industriais e modo pulsante, numeração estruturada de bornes segundo IEC 60947-1 e IEC 81346, e os modos global e contextual de referência cruzada com ligações em série/paralelo e gestão de réguas de bornes.

---

## 1. Estrutura de Seletores Principais (§4.0)

A barra lateral direita mantém o título **Propriedades do Elemento** e é estruturada em dois seletores principais via abas responsivas:

1. **Aba "Propriedades"** (índice 0): Exibe os parâmetros técnicos e normativos do elemento atualmente selecionado no canvas de trabalho ou as propriedades da folha/legenda (cajetín) quando nenhum elemento estiver selecionado.
2. **Aba "Referência Cruzada"** (índice 1): Apresenta a matriz de referências cruzadas inter-folhas e equipotenciais elétricas (série/paralelo/direta) no modo contextual (com elemento selecionado) ou o diretório geral de tags do projeto com resumo de réguas de bornes no modo global (sem elemento selecionado).

Ambas as abas oferecem suporte completo ao recolhimento e restauração lateral com botão chevron, garantindo espaço máximo para o canvas de trabalho quando recolhidas (mantendo barra mínima de 28 px).

---

## 2. Seletor: Propriedades (§4.1)

### 2.1 Nome do Elemento e Função Descritiva
- **Nome (`item.tag`)**:
  - Campo de entrada de texto direto com placeholder informativo (`Tag / Nome`).
  - Caixa de seleção (checkbox) "Exibir no esquema", vinculada a `item.show_tag == 1`.
  - Validação inteligente com alertas dinâmicos:
    - Contatos auxiliares: indicação de vínculo com o aparelho pai (ex.: bobina `-KM1`) com botão "Ir" para navegação direta no canvas.
    - Contatos órfãos: alerta visual indicando ausência de bobina correspondente e botões de vínculo rápido para bobinas disponíveis no projeto.
    - Aparelhos primários: detecção de tags duplicadas com botão de renumeração sequencial automática.
    - Contagem de contatos associados no esquema.
- **Função (`item.function`)**:
  - Novo campo de texto descritivo persistido na estrutura `SchematicItem` com serde (compatibilidade reversa transparente via `#[serde(default, skip_serializing_if = "Option::is_none")]`).
  - Método utilitário `item.function_or_value()` para visualização com fallback gracioso para o valor técnico caso a função não seja explicitamente informada.
  - Caixa de seleção (checkbox) "Exibir no esquema", vinculada a `item.show_value == 1`.

### 2.2 Padrões de Bobinas Especiais e Controle de Tempo (IEC / EN 50012)
Para bobinas de comando e temporizadores (`tipo: 9000`, `9004`, `9005` ou papéis funcionais `ApparatusRole::Coil` e `ApparatusRole::Timer`), o sistema apresenta um seletor dropdown (`pick_list`) com o enum `CoilPattern`:
- **Padrão Instantânea (Contator)**: `tipo: 9000`, `subvariant: 0`.
- **Ao Trabalho (TON / On-Delay)**: `tipo: 9004`, `subvariant: 0` (valor inicial "3" s).
- **Ao Repouso (TOF / Off-Delay)**: `tipo: 9005`, `subvariant: 0` (valor inicial "3" s).
- **Cíclico / Pulso Intermitente**: `tipo: 9004`, `subvariant: 2` (T_ON e T_OFF independentes, iniciais de "1" s cada).
- **Retenção / Biestável (Latching)**: `tipo: 9000`, `subvariant: 1`.

Quando um padrão temporizado (`OnDelay`, `OffDelay`, `Cyclic`) está ativo, o painel exibe o módulo **Controle de Tempo**:
- **Tempo Base + Steppers**: Campo numérico editável acompanhado dos botões de ajuste incremental `[-]`, `[+]` (+/- 1 s) e chips de ajuste fino/amplo (`-5s`, `-0.5s`, `+0.5s`, `+5s`).
- **Escala Multiplicadora (`TimerScaleMultiplier`)**: Dropdown compatível com a codificação original `.cad` em `pins[2]` (e `pins[4]` para `T_OFF`):
  - `x 0.1 Seg (Décimos)` — código `"3"`, fator `0.1`.
  - `x 1 Seg (Segundos)` — código `"0"`, fator `1.0`.
  - `x 10 Seg (Dezenas)` — código `"1"`, fator `10.0`.
  - `x 60 Seg (Minutos)` — código `"2"`, fator `60.0`.
- **Duração Efetiva Calculada**: Badge de leitura direta exibindo o tempo efetivo em segundos e milissegundos (`Tempo Efetivo: X s (Y ms)`), mantendo `pins[3]` (e `item.value`) sincronizados com precisão adaptativa (`format_timer_seconds`).
- **Atalhos Rápidos (Quick Presets)**: Botões de um clique (`0.5s`, `1s`, `2s`, `3s`, `5s`, `10s`, `30s`, `60s`) que configuram automaticamente o multiplicador ideal e o valor efetivo.
- **Temporizador Cíclico Independente (`T_ON` / `T_OFF`)**: Controles dedicados para `Tempo Ligado (T_ON)` (`pins[2]`/`pins[3]`) e `Tempo Desligado (T_OFF)` (`pins[4]`/`pins[5]`), com cartão resumo exibindo período `T`, frequência `f` (Hz) e ciclo de trabalho `D` (%).
- **Monitoramento ao Vivo na Simulação**: Durante `sim_running`, exibe estado em tempo real (`Em Repouso`, `Contando`, `Disparado / Concluído` ou `T_ON (Pulso)` / `T_OFF (Pausa)`), barra de progresso proporcional com porcentagem e botão **Reiniciar Temporizador** (`Message::ResetTimerSimulation`) além de overlay dinâmico no canvas.
- **Preset de Bornes**: Botão rápido `A1-A2 (IEC)`.

### 2.3 Sinalizadores Luminosos e Pulsantes (IEC / EN 60073)
Identificados automaticamente por `item.is_lamp_or_signaling()`:
- **Cores Industriais Padronizadas**:
  - Código 1: Verde
  - Código 2: Vermelho
  - Código 3: Amarelo
  - Código 4: Azul
  - Código 0: Branco
  - Código 5: Laranja (novo token `lamp_orange: rgb(0xEA580C)`)
  - Amostras visuais (`swatches`) com realce de seleção ativa (`theme::tool_active`).
- **Modo Pulsante (Intermitente)**:
  - Checkbox "Sinalizador Pulsante (Intermitente)", armazenado no campo `item.extra_int1 == 1`.
  - Campo numérico para tempo de pulso em segundos (`Message::SetPulseDuration`), inicializado em `0.5` s por padrão.
- **Preset de Bornes**: Botão de preset rápido para bornes de sinalizador `X1-X2`.

### 2.4 Numeração Estruturada de Bornes (IEC 60947-1 e IEC 81346)
- **Circuitos de Potência / Força (1 dígito)**:
  - Regra normativa: Números ímpares (`1, 3, 5` ou `L1, L2, L3`) para entrada de alimentação das fases; números pares (`2, 4, 6` ou `T1, T2, T3`) para saída de carga/motor.
  - Presets de 1 clique:
    - `1-2, 3-4, 5-6` (Tripolar numérico padrão)
    - `L1-T1, L2-T2, L3-T3` (Tripolar de fases industriais)
    - `1-2` (Unipolar numérico)
    - `L1-T1` (Unipolar de fase)
- **Circuitos de Comando (Contatos Auxiliares – 2 dígitos)**:
  - Regra normativa:
    - 1º Dígito: Sequência do contato no aparelho (`1..9`).
    - 2º Dígito: Função elétrica (`1-2`: NF, `3-4`: NA, `5-6`: NF especial temporizado/sobrecarga, `7-8`: NA especial temporizado/falha térmica `97-98`).
  - Dropdown sequencial inteligente IEC 60947 (`1..9`) com pré-visualização dos bornes, sugestão automática de dígito livre e alerta em caso de colisões com outros contatos da mesma tag.
  - Presets de 1 clique:
    - `13-14 (NA)`
    - `21-22 (NF)`
    - `95-98 (Térmico)`
- **Réguas de Bornes em Painéis (IEC 81346 – Letra X)**:
  - Tabela de subdivisão funcional:
    - `XP` (ou `X0`): Potência / Força (Alimentação geral e motores).
    - `XC`: Comando / Auxiliar (Botoeiras, sensores e lógicas).
    - `XT`: Medição / Auxiliar (Transformadores e instrumentos).
    - `X1`: Geral.
  - Badges de identificação de tipo (`badge_label()`) e botões rápidos para atribuir a tag correspondente (`-XP`, `-XC`, `-XT`, `-X1`) ao borne selecionado.
  - Botão para renumeração sequencial ordenada por posição física no esquema.

### 2.5 Propriedades da Folha (Cajetín)
Quando nenhum elemento está selecionado, o painel exibe:
- Título do Projeto (`app.doc.title_block.title`).
- Subtítulo (`app.doc.title_block.subtitle`).
- Desenhista / Autor (`app.doc.title_block.drawn_by`).
- Dicas de navegação e contexto do esquema.

---

## 3. Seletor: Referência Cruzada (§4.2)

### 3.1 Modo Contextual (Elemento Selecionado)
- Identificação do elemento selecionado (tag, tipo de símbolo, folha e quadrante de grade).
- Vínculo de aparelho pai com localização inter-folhas e botão de navegação direta "Ir".
- Relação de contatos associados no projeto com identificação de bornes, folha, grade e botões "Ir".
- Mapeamento elétrico detalhado de terminais:
  - Conexões em **Série** (badge verde).
  - Conexões em **Paralelo** (badge primário).
  - Conexões **Diretas** (badge neutro).
  - Navegação com um clique para qualquer elemento conectado em outro quadrante ou folha.
- Propriedades da régua de bornes e botão de atalho "XRef Global" para alternar para a visão ampla.

### 3.2 Modo Global (Nenhum Elemento Selecionado)
- Cabeçalho com total de tags ativas e folhas do projeto.
- Campo de busca em tempo real com filtro por tag.
- Cartões por tag agrupando:
  - Aparelho primário (com localização e bornes).
  - Contatos auxiliares subordinados.
  - Outras instâncias.
  - Alertas visuais para "Sem Bobina" (órfão) e "Bobina Duplicada".
- Resumo integrado de todas as Réguas de Bornes (IEC 81346) presentes no projeto, com contagem de bornes, alertas de numeração duplicada e botão para "Renumerar Todas as Réguas" em sequência determinística.

---

## 4. Testes de Integração e Garantia de Qualidade

Foram desenvolvidos 8 testes de integração dedicados cobrindo 100% dos cenários normativos da Seção 4 em `tests/integration_tests.rs`:

1. `test_properties_element_name_and_function_editing`: Edição de tag, exibição de nome no esquema, persistência de função, método `function_or_value()` e fallback seguro.
2. `test_properties_special_coil_patterns_and_timers`: Ciclo de vida de bobinas especiais (Instantânea, On-Delay, Off-Delay, Cíclico, Retenção), ajuste de temporização e presets A1-A2.
3. `test_properties_signaling_colors_and_pulsing_duration`: Cores industriais padronizadas (incluindo laranja), validação dos tokens de cor, modo pulsante com temporização e preset X1-X2.
4. `test_properties_iec60947_power_and_command_terminal_editing`: Presets de força (3P e 1P numérico e fases industriais), contatos de comando com dropdown inteligente IEC 60947 e presets `aux_no`, `aux_nc` e `thermal`.
5. `test_properties_terminal_strips_iec81346_classification`: Atribuição rápida de réguas XP (força), XC (comando), XT (medição) e X1 (geral).
6. `test_properties_cross_reference_selected_vs_global_modes`: Renderização sem panics nos estados selecionado e desmarcado, alternância de abas 0 e 1, e navegação direta inter-folhas via XRef.
7. `test_properties_timer_time_control_and_cyclic_parameters`: Controle de tempo para temporizadores (valores base, botões +/- e chips de passo, multiplicadores de escala x0.1/x1/x10/x60, presets rápidos de tempo e configuração cíclica independente T_ON / T_OFF).
8. `test_properties_timer_live_simulation_monitoring_and_reset`: Monitoramento ao vivo do tempo decorrido, barra de progresso proporcional e comando de reinício manual durante simulação.

A suíte completa de testes passou com **117 testes verdes em ~3,7 segundos**, compilação estrita com zero advertências (`cargo clippy --all-targets -- -D warnings`) e formatação integral aprovada (`cargo fmt --all -- --check`).
