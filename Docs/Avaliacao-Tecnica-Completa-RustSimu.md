# RustSimu — Avaliação Técnica Completa

> **Documento de referência de engenharia** — avaliação independente do código, arquitetura, motor de simulação, qualidade, ferramental multi-agente e roadmap.
>
> | Metadado | Valor |
> |---|---|
> | Data da avaliação | 05/11/2026 |
> | Commit-base | `37e06c2` (feat(ui): redimensionamento de painéis + detecção de tema nativa) |
> | Versão | 0.4.0 (edition 2021) |
> | Escopo | Todo o repositório: `src/` (14 arquivos Rust), testes, CI, assets, docs e ferramental de agentes |
> | Método | Leitura integral dos módulos, medições com `cargo check/test`, greps de inventário, revisão de histórico Git (13 commits) e cotejo do código contra as especificações (`AGENTS.md`, `Layout RustSimu HTML/`) |
> | Status do build | `cargo check --all-targets` limpo · `cargo test` **28/28 verde (~4,4 s)** |

---

## 1. Sumário executivo

O **RustSimu** é uma reescrita nativa em Rust (iced 0.13) do CADe_SIMU 4.0 — editor de esquemas eletrotécnicos + simulador de comandos elétricos — com compatibilidade real (roundtrip byte-exato) dos arquivos `.cad` legados. O projeto está em **estágio de qualidade surpreendentemente sólido para sua idade**: compila limpo, possui suíte de testes funcionais de circuitos elétricos reais (rara em projetos desse porte), engenharia reversa minuciosamente documentada e arquitetura Elm (Model/Update/Message) bem delimitada fora do `main.rs`.

**Veredicto global:** base arquitetural e técnica **boa** (7.5/10) com dois gargalos concentrados: o god-file `main.rs` (6.812 linhas) e a ausência de CI de qualidade. Nenhum bloqueio arquitetural foi identificado.

| Dimensão | Nota (1–5) | Comentário |
|---|---|---|
| Arquitetura & design | 4,0 | Separação clara model/sim/io/symbols/canvas; penalizado pelo `main.rs` monolítico |
| Motor de simulação | 4,5 | BFS de potenciais + cascata até ponto fixo + trip magnético novo; física de potência simplificada por opção de modelagem |
| Compatibilidade legada | 5,0 | `.cad` byte-exato com comentários RE preservados; export QET real; JSON/TOML simétricos |
| Qualidade de código | 3,5 | 0 `unwrap` de produção em `main.rs` e nenhum TODO; penalizado por heurísticas duplicadas e falta de clippy |
| Testes | 4,0 | 28 testes funcionais com 100% verde; todos inline no `main.rs`, sem cobertura medida |
| UI/UX | 3,5 | Temas light/dark/system completos, docks responsivos; exportar/imprimir ainda examinam a spec |
| CI/CD | 2,5 | Release Windows/Linux automático; **não há** CI de build/test/lint |
| Ferramental multi-agente | 4,5 | `AGENTS.md` refinado + 5 agentes customizados com permissões; protocolo ativo com hot-reload verificado |
| Performance | 3,5 | Tick de 80 ms reconstroi a rede por completo a cada passo; `find_symbol` reconstrói o catálogo por chamada (98 call-sites) |

---

## 2. Métricas principais do código

| Métrica | Valor |
|---|---|
| Linhas Rust (`src/*.rs` + bins) | **17.110** |
| Maior arquivo | `main.rs` — 6.812 linhas (mod `tests` nas linhas ~4087–6812 ≈ **2.700 linhas de teste**) |
| Módulos | `model`, `sim`, `io`, `qet`, `symbols`, `svg_symbol`, `canvas_view`, `icons`, `layout`, `theme`, `lib` + bins `cade2svg`, `elmt2svg` |
| Catálogo de símbolos nativos | **69 `SymbolDef`** (1000–9013) definidos matematicamente por comandos `Dibujo` herdados |
| Biblioteca QElectroTech | **9.327 SVGs** (`symbols_qet_all/`) + `catalog.json` com ~8.890 entradas indexadas |
| Testes | **28**, todos passando; helpers `step_secs`; fixtures reais de `Projetos/*.cad` |
| `unwrap()/expect()` em produção | `main.rs`: **0** (os 225 matches estão no `mod tests`); total de produção ≈ **3** (`qet.rs` 2 — extremidades de condutor na exportação, `svg_symbol.rs` 1 — `terminal_ys.last()`) |
| TODOs/FIXMEs | **0** |
| Chamadas a `find_symbol()` | 98 call-sites (hot path de sim/render/hit-test) |
| Memos/Lazy | catálogo **não memoizado** (reconstruído a cada chamada) |

---

## 3. Arquitetura e fluxo de dados

### 3.1 Padrão geral

Aplicativo iced 0.13 no padrão Elm: `CadeSimuApp` (≈47 campos de estado) + enum `Message` (≈70 variantes) + singleton `SchematicDocument` ativo mutável, com `Vec<ProjectPage>` para multipágina e pilha de undo/redo (limite 50, snapshot integral do documento). O canvas é um `canvas::Program` próprio (`SchematicCanvas`) que converte tela↔mundo (zoom/pan/snap de 3 pontos), faz hit-test e devolve `CanvasMessage`s que o `update` central converte em mutações documentadas.

```
        [ model.rs ] ◄─────────────► [ sim.rs ]  (tick 80 ms via Subscription)
             ▲                             ▲
             ·                             └──► symbols/svg_symbol (bornes mundo)
          io.rs (cad/json/toml/qet)
             ▲
     canvas_view.rs ◄──── theme.rs  (tokens Dark/Light/System)
             ▲
        main.rs (coordenador + mod tests)
```

### 3.2 Avaliação por módulo

| Módulo | Linhas | Avaliação |
|---|---|---|
| `model.rs` | 439 | Excelente como contrato de dados; documentação RE densa (offsets `0x…`). Ponto de atenção: 7 campos `sim_*` transitórios embutidos em `SchematicItem` acoplam simulação ao modelo persistido |
| `sim.rs` | 1.035 | Motor coeso; `step_simulation` fica ~700 linhas com closure `resolve_cascade` aninhada. Novo `detect_and_trip_protective_devices` (trip magnético + queima de fusíveis) integra-se bem |
| `io.rs` | 490 | Roundtrip byte-exato do `.cad`; parser tolerante silencia dados inválidos (sem diagnóstico) |
| `qet.rs` | 1.121 | Import/export QElectroTech bidirecional com preservação da coleção XML; únicos unwraps de produção (~2) |
| `symbols.rs` | 3.447 | Catálogo + numeração IEC 60947 (sequência de bornes, tags `ApparatusRole`); heavyweight: ~2.400 linhas de definições reconstruídas a cada `find_symbol` |
| `svg_symbol.rs` | 507 | Parser SVG → `SvgShape` com cache global `OnceLock`; 1 unwrap de produção |
| `canvas_view.rs` | 1.516 | Canvas rico (draw primitivos, preview fantasma, sim-click); cores paralelas aos tokens de tema |
| `theme.rs` | 649 | Design system 1:1 com o protótipo CSS; `detect_system_dark()` nativo (Windows registry, KDE `kreadconfig6`, GNOME `gsettings`, portal freedesktop, `GTK_THEME`) |
| `layout.rs` | 155 | 3 colunas; novo resize por drag (splitters 6px, clamp 160–480px). Mensagens `Msg::Resized`/`ToggleFocus` mortas |
| `main.rs` | 6.812 | Coordenador completo + menubar/toolbar/docks em overlays (`Stack`). Riscos: `update` ≈1.400 linhas, `view_properties_panel` ≈690, `view_collections_panel` ≈412 |

### 3.3 Pontos fortes e fracos arquiteturais

**Fortes:**
- Contrato único de dados (`SchematicItem`) compartilhado por todos os domínios, com compat 1:1 aos arrays paralelos do CADe original.
- Simulação determinística e separada da UI (mesma função usada nos testes e no tick).
- Pipeline SVG com cache e mapeamento coordenado QET↔mundo.
- Undo de arraste em transação única (não polui o histórico por pixel).

**Fracos / dívidas estruturais:**
- `main.rs` redeclara os `pub mod` — os módulos compilam duas vezes (lib + bin).
- `update()` de ~1.400 linhas dificulta mudanças seguras e contexto de agentes.
- Autoconexão (*smart wiring*) dividida entre `canvas_view` (emissão) e `main.rs` (fusão de junções 4001) — funcional, porém pouco óbvia para manutenção.

---

## 4. Motor de simulação — análise profunda

### 4.1 Algoritmo (por tick de 80 ms)

1. **Rede**: pontas de cabos + junções 4001 viram nós únicos; pinos de símbolos nativos via `world_pins` e de símbolos QET via círculos `is_terminal` do SVG parseado.
2. **Adjacência**: cada cabo é subdividido por pontos colineares (`point_on_segment`) — O(N²) em nós, refeita a cada tick.
3. **Estado de controles**: fusíveis conduzem por padrão; disjuntores/chaves/botões seguem `sim_manual_latched` (sincronizados por TAG); contatores 2000/2001/2006/2008 e contatos 7000–7003 seguem a bobina; contatos térmicos 8016–8019 (95-96/97-98) seguem o térmico pai.
4. **Cascata `resolve_cascade`**: até 32 passes de ponto fixo — conecta pares de bornes por morfologia de contato (NA/NF 1P, duplo 4T, SPDT, seletoras 2/3 posições, inversora/crossover, 2P/3P/4P, térmico), injeta fontes (3000–3005 AC, 3008/3009 CC, heurísticas QET battery/ground/arrows) e propaga potenciais por **BFS**, marcando `Potential::ShortCircuit` em conflito de polaridades.
5. **Trip magnético (novo)**: `detect_and_trip_protective_devices` varre disjuntores fechados e fusíveis intactos cujos polos receberam `ShortCircuit` e os desarma (`sim_manual_latched = false`) — o curto é extinto a jusante; coberto por teste dedicado.
6. **Temporizadores**: TON acumula `dt_ms`; TOF pré-carrega e decrementa; disparo re-resolve a cascata dentro do mesmo tick.
7. **Motores**: 1000 trifásico (sequência L1→L2→L3 define giro CW/CCW, falta de fase ⇒ `sim_fault`), 1001 Y/Δ 6T (estrela = ponte W2-U2-V2 no mesmo componente conexo; delta = 3 fases nos bornes inferiores), 1002 monofásico (diferença de potencial).
8. **Visual**: cabos e junções herdam `sim_potential` (cores por potencial; curto pinta em vermelho espesso).

### 4.2 Resultados e limitações por desenho

- **Lógica booleana rigorosa**: série = AND, paralelo = OR, validada por testes dedicados de comando e potência (`test_strict_and_series_conduction_command_and_power` / `test_strict_or_parallel_conduction_command_and_power`).
- **Acoplamento por TAG**: bobina `-KM1` comuta todos os contatos `-KM1` no mesmo instante de resolução; relé térmico `-F1` sobre contatos `-F1`.
- **Limitações (por desenho, não por erro)**: não há quantidades físicas (correntes em A, kW, torque) — o modelo é de potenciais discretos; não há queda de tensão nem acoplamento magnético real; o curto é um estado de potencial (marcação visual + trip), não um cálculo de corrente.

### 4.3 Riscos e candidatos a evolução

| Risco | Impacto | Mitigação recomendada |
|---|---|---|
| Rebuild da rede a cada tick | Custo O(N²) por passo e flicker crescente em esquemas grandes | Cache de rede invalidado por mutação do documento; re-resolve apenas gatilhos/timers |
| Tipos fantasma 7003/7007 | Braços de match sem `SymbolDef` — código morto potencialmente confuso | Catalogar os SPDT temporizados ou remover os braços |
| Heurísticas QET por substring repetidas 5× | Edição única exige 5 ajustes (bug à vista) | Extrair helpers `is_qet_manual_switch` / `is_qet_load` / `is_qet_source` |
| Constante mágica `y1 >= 600` (fonte QET por posição) | Comportamento implícito | Regra nomeada + comentário RE |

---

## 5. Formatos e compatibilidade

| Formato | Escrita | Leitura | Observações |
|---|---|---|---|
| `.cad` | ✅ byte-exato (`OnFileSave 0x00431600`) | ✅ tolerante (`OnFileOpen 0x0042c290`) | 8/9 pinos (1052, 1073, 10500..=10550); cajetín com truncamentos fixos; trailer `***$$$&&&***$$$&&&`; decoder UTF-8 → Latin-1 por byte (CP-1252 não mapeado 1:1) |
| `.json`/`.toml` | ✅ pretty | ✅ | Espelhos exatos; campos `sim_*` nunca persistidos (`#[serde(skip)]`) |
| `.qet` | ✅ (folha única) | ✅ coleção embutida + multi-diagrama (abre o 1º não vazio) | Condutores sintéticos `node_{id}` na exportação |
| Multipágina | `.json`/`.toml`/`.qet` | `.cad` **perde páginas silenciosamente** | Definir formato "projeto" ou avisar no save |

---

## 6. UI/UX — estado versus especificação

O gap analysis abaixo usa a spec `Layout RustSimu HTML/Modificações (1).md` como fonte e o enum `Message` + views como evidência (estimativa aproximada, classificada por ocorrência no código; revisar item a item ao planejar):

| Seção da spec | Implementado | Parcial | Pendente |
|---|---|---|---|
| Arquivo | Novo, Abrir, Salvar, Salvar como, Fechar | Exportar (dialog ✅, formatos PDF/SVG/PNG **ainda não geram arquivo real** — `save_document` cai em JSON para extensões desconhecidas) · Imprimir (só status) | Salvar cópia |
| Editar | Undo/Redo, Cortar/Copiar/Colar (na posição do mouse e na origem), Duplicar, Inserir último, Selecionar tudo/inverter/anular, Copiar/colar infos, Cor do elemento (lamp/elementos com preset) | — | Agrupar Ctrl+G/U, linhas/colunas, adicionar (arco/texto/imagem), propriedades página |
| Projeto | Página antes/depois/deletar/mover/rename, auto-conexão, desconexão | — | Propriedades do projeto, CSV/BOM |
| Display | Grade + snap + densidades, zoom (in/out/reset/content/fit), tema System nativo | — | Mira em todo fólio, grelha de texto, guias |
| Docks | Recolher 28px ↔ 288px, **redimensionar por drag (160–480px, separador de 6px)**, abas Elementos/Projetos, recentes com delete, drag-and-drop do símbolo para o canvas | — | — |
| Simulação | Play/Pausa com brilho, Stop, **passo manual (Step → `SimTick`)** | Campo `sim_speed` existe no documento mas não é exposto na UI | — |

**Conclusão de UX:** o núcleo de edição/simulação do CADe_SIMU original está coberto; as pendências concentram-se em saída de documentos (export/impressão) e features avançadas da spec (agrupamento, ferramentas de desenho adicionais, referência cruzada de projeto inteiro).

---

## 7. Qualidade de código e testes

### 7.1 Higiene

- **0 `unwrap/expect` de produção em `main.rs`** (todos os 225 matches residem no `mod tests`); produção total ≈ 3 (`qet.rs` 2 — em resgate de elementosroteados, `svg_symbol.rs` 1 — `terminal_ys.last()`).
- **0 TODO/FIXME** — as dívidas não estão documentadas em código; vivem nesta avaliação.
- Sem `.clippy.toml`/`rustfmt.toml`; `cargo check` limpo; **clippy não executado em CI**.
- Duplicação de `pub mod` entre `lib.rs` e `main.rs` (os módulos compilam duas vezes).

### 7.2 Testes (28/28 verde, ~4,4 s)

| Grupo | Testes (exemplos) |
|---|---|
| Simulação eletrotécnica | DOL com selo e trip térmico; Y-Δ c/ e s/ inversão; 3 motores; escalonada; desconexão; enchimento de tanque; **AND/estricto série e OR paralelo**; equivalência ladder/fluido; **trip por curto de disjuntores e fusíveis (novo)** |
| Comportamento de aparelhos | Pulsadores 4 tipos (press/release), retentivos × momentâneos, emergências, cogumelo, fins de curso, crossover, seletoras, térmicos auxiliares |
| Compatibilidade | Roundtrip `.cad`/`.json`/`.toml` dos 6+1 projetos; `teste.cad` completo; import QET Kaefer |
| UI | Layout/temas, docks (colapso, **redimensionamento clampado (novo)**), toolbar ícones, menus em overlay sobre canvas, drag/undo único, numeração IEC/sequência de bornes, ícones SVG válidos |

**Brechas de teste:** ausência de cobertura medida (`llvm-cov`/`tarpaulin`); nenhum teste de stress de rede grande (perf dos ticks); qualidade do parsing `.cad` com dados corrompidos não é testada explicitamente (o parser é tolerante "sem deixar rastro").

---

## 8. Engenharia reversa — patrimônio documental

Um diferencial grande do projeto: as estruturas reproduzem os arrays paralelos do `CADe_SIMU.exe` com comentários citando **offsets do binário** (`OnFileSave 0x00431600`, transformação de orientação `0x00846951`, tipos estendidos `0x00431886`...). Isso é uma **referência documental viva de compatibilidade** — a regra §5.2 do `AGENTS.md` (nunca remover/alterar) é correta e hoje é cumprida integralmente.

**Risco:** esse conhecimento existe *só* em comentários inline. Recomenda-se extrair para `docs/cad-format.md` (fonte citada) para sobreviver a refatorações e facilitar agentes.

---

## 9. CI/CD e operação

- **`release.yml`**: build portátil Windows (MSVC + `core.longpaths`) e Linux em tags `v*` + `workflow_dispatch`; empacota binário + `symbols_qet_all/` + `Projetos/` + README.
- **Gap crítico**: não há workflow de **build+test+clippy+fmt** em PR/push. Em um fluxo de três "engenheiros" (humano + OpenCode + Antigravity/Gemini), o CI é o contrato mais barato de regressão-zero.
- O manifest define `default-run = cade-simu-rs`; dois bins utilitários cobrem a regeneração de assets (com caveat documentado no `AGENTS.md`: `elmt2svg` exige coleção QET instalada).

---

## 10. Ferramental multi-agente (estado atual, todos commitados)

| Artefato | Estado |
|---|---|
| `AGENTS.md` | Refinado com 7 checks de precisão vs código (commit `1a11a27`); hot-reload de instruções verificado em sessão (as atualizações chegam ao agente ativo) |
| `.opencode/agents/` (commit `d8239c1`) | `sim-engineer` (subagent), `compat-cad` (subagent), `iced-ui` (subagent), `refactor-surgeon` (all), `normas-iec-iso` (all) — todos com `deny` em `git push*` e `git tag*`, cores distintas e escopos/critérios de resposta definidos |
| Normas cobertas | IEC 60204-1, IEC 60947 (-1/-4-1/-5-1), EN 50005/50011/50012, IEC 60617, ISO 1219-1/-2 (pneumática/hidráulica), IEC 61131-3 (ladder) |

---

## 11. Dívidas técnicas priorizadas

### P0 — higiene imediata (alto ganho/1 dia no total)

| # | Dívida | Esforço | Ganho |
|---|---|---|---|
| 1 | CI de qualidade (build+test+clippy+fmt) | 0,5 d | Contrato de regressão-zero para humano + 2 IAs |
| 2 | Memoizar `symbol_catalog()` (`LazyLock`) | 0,5 d | Corta trabalho por `find_symbol` × 98 call-sites × 69 defs (sim/render/hit-test a cada 80 ms) |
| 3 | Remover `pub mod` duplicados do `main.rs` | 0,25 d | Compilação mais simples e única |
| 4 | Dedup das heurísticas QET (5× substring) em `sim.rs` | 0,5 d | Corretibilidade |

### P1 — estrutura (1–2 semanas)

| # | Dívida | Esforço | Ganho |
|---|---|---|---|
| 5 | Mover `mod tests` (≈2.700 linhas) para `tests/` | 1 d | `main.rs` -40%, suíte compartilhável |
| 6 | Extrair `SimState` dos campos `sim_*` de `SchematicItem` | 2 d | Model puro; abre caminho para sim em threads e snapshot barato |
| 7 | Fragmentar `main.rs` em `src/ui/` + `src/app/` por domínio de `Message` | 3–5 d | Recupera separação de responsabilidades e reduz o contexto de agentes |
| 8 | Modularizar `sim.rs` (graph/solver/rules/timers) com cache da rede | 3 d | Performance e legibilidade do motor |
| 9 | Export real: folha → SVG → PNG/PDF (stub hoje) | 2–4 d | Fecha promessa do README e da spec §1.1 |
| 10 | Cores do canvas via `theme::tokens` (+ base da folha) | 0,5 d | Coerência de tema |
| 11 | Decoder CP-1252 (bytes 0x80–0x9F) | 0,5 d | Textos legados corretos |
| 12 | Tipos fantasma 7003/7007 + `Msg::Resized`/`ToggleFocus` mortos | 0,5 d | Menos code-ruído |

### P2 — produto (roadmap)

| # | Item | justificativa |
|---|---|---|
| 13 | Formato de projeto multipágina (`.rsim` zipado?) com aviso em `.cad` | Evita perda silenciosa de folhas |
| 14 | Biblioteca QET completa na UI (index/busca/paginação sobre o `catalog.json` de ~8.890 entradas; drag-and-drop já existe para nativos) | Libera os 9.327 SVGs que hoje ficam como assets não expostos |
| 15 | Numeração global de bornes + referência cruzada de projeto | Fechar spec §4.2 |
| 16 | Undo granular (command pattern), i18n pt/en, BOM/CSV, grupo Ctrl+G, ferramentas de desenho (texto/arco/retângulo) | Alinhados à spec e prática CAD |

---

## 12. Riscos consolidados

1. **Concentração de contexto em `main.rs`** — risco de regressão em UI a cada mudança + custo de contexto alto para agentes; mitigação P1-#7.
2. **Knowledge lock de RE** — os offsets RE vivem apenas no código; mitigação `docs/cad-format.md` + bloqueio linter.
3. **Performance de sim em esquemas grandes** — rebuild O(N²)/tick; mitigação P1-#8.
4. **`.cad` multipágina** — perda silenciosa de dados (leitura do arquivo salva no `.cad` pega só a folha ativa); mitigação P2-#13.
5. **Barramento de três escritores** (humano + OpenCode + Antigravity) — furo de disciplina se CI (P0-#1) não existir; sem CI, `cargo test` local continua sendo o único selo verde.

---

## 13. Conclusão

O RustSimu realizou em poucos dias de trabalho o que normalmente toma semanas: **núcleo de simulação funcional e testado, compatibilidade byte-exata com um binário proprietário reverse-escrito e documentado, UI responsiva com temas completos e ferramentação de agentes com protocolo escrito**. Os dois próximos passos de maior valor são o **P0 (CI + memoização)** — baratos e desbloqueiam todo o restante — e o **saneamento estrutural da P1** (fragmentar `main.rs`/`tests`), que transforma o código em um terreno amigável tanto para humanos quanto para os agentes OpenCode/Antigravity agora em operação.

Nenhum bloqueio arquitetural existe. O relatório recomenda atacar na ordem: **P0 → #5/#6/#7 (estrutura) → #9 (export real) → #14 (biblioteca QET)**, mantendo o contrato de regressão-zero já estabelecido no `AGENTS.md`.

---

*Relatório gerado por avaliação de agente (OpenCode) a pedido do mantenedor. Métricas colhidas em 05/11/2026 @ `37e06c2`. Ambiente de medição: `cargo test` 28/28 verde no momento da avaliação.*
