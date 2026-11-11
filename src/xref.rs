use std::collections::{HashMap, HashSet, VecDeque};

use crate::model::{ProjectPage, SchematicDocument, SchematicItem};
use crate::sim::graph::build_network_graph;
use crate::symbols::{classify_item_role, ApparatusRole};

/// Classificação funcional de Réguas de Bornes segundo a norma IEC 81346.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum TerminalStripKind {
    /// XP ou X0: Bornes destinados à alimentação geral e motores (Potência / Força)
    Power,
    /// XC: Bornes para interligação de botoeiras, sensores e lógicas (Comando / Auxiliar)
    Control,
    /// XT: Bornes dedicados a transformadores e medição (Auxiliar)
    Measurement,
    /// X ou X1..Xn: Réguas gerais de bornes
    General,
}

impl TerminalStripKind {
    pub fn from_tag(tag: &str) -> Self {
        let clean = tag.trim().trim_start_matches('-').to_uppercase();
        if clean.starts_with("XP") || clean.starts_with("X0") {
            Self::Power
        } else if clean.starts_with("XC") {
            Self::Control
        } else if clean.starts_with("XT") {
            Self::Measurement
        } else {
            Self::General
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Power => "Potência / Força (XP)",
            Self::Control => "Comando / Auxiliar (XC)",
            Self::Measurement => "Medição / Transformadores (XT)",
            Self::General => "Geral (X)",
        }
    }

    pub fn badge_label(&self) -> &'static str {
        match self {
            Self::Power => "Potência (XP)",
            Self::Control => "Comando (XC)",
            Self::Measurement => "Medição (XT)",
            Self::General => "Geral (X)",
        }
    }
}

/// Dimensões da folha de trabalho para cálculo de coordenadas do Cajetín e grade de referência.
pub fn sheet_dimensions(doc: &SchematicDocument) -> (f32, f32) {
    let sheet_w = if doc.settings.custom_height_mm > 420 {
        doc.settings.custom_height_mm as f32 + 30.0
    } else {
        (doc.settings.custom_height_mm.clamp(279, 420) as f32) + 45.0
    };
    let sheet_h = if doc.settings.custom_width_mm > 320 {
        doc.settings.custom_width_mm as f32 + 30.0
    } else {
        (doc.settings.custom_width_mm.clamp(210, 320) as f32) + 30.0
    };
    (sheet_w, sheet_h)
}

/// Calcula a coordenada de grade industrial da folha considerando o número de colunas e linhas configuradas.
pub fn calculate_grid_cell_with_grid(
    x: f32,
    y: f32,
    sheet_w: f32,
    sheet_h: f32,
    cols: i32,
    rows: i32,
) -> String {
    let eff_cols = cols.clamp(4, 24);
    let eff_rows = rows.clamp(4, 16);
    let w = (sheet_w - 12.0).max(10.0);
    let h = (sheet_h - 12.0).max(10.0);
    let col = (((x - 6.0) / (w / eff_cols as f32)).floor() as i32).clamp(0, eff_cols - 1) + 1;
    let row_idx = (((y - 6.0) / (h / eff_rows as f32)).floor() as i32).clamp(0, eff_rows - 1);
    let row_char = char::from(b'A' + row_idx as u8);
    format!("{row_char}{col}")
}

/// Calcula a coordenada de grade industrial da folha (padrão 8x6: "C6", "B2").
/// Colunas: 1..8 (esquerda para direita); Linhas: A..F (superior para inferior).
pub fn calculate_grid_cell(x: f32, y: f32, sheet_w: f32, sheet_h: f32) -> String {
    calculate_grid_cell_with_grid(x, y, sheet_w, sheet_h, 8, 6)
}

/// Formata a localização de referência cruzada completa (ex: "1.C6").
pub fn format_grid_ref(page_index: usize, x: f32, y: f32, sheet_w: f32, sheet_h: f32) -> String {
    let cell = calculate_grid_cell(x, y, sheet_w, sheet_h);
    format!("{}.{}", page_index + 1, cell)
}

/// Formata a localização de referência cruzada respeitando a grade dinâmica do documento.
pub fn format_grid_ref_doc(page_index: usize, x: f32, y: f32, doc: &SchematicDocument) -> String {
    let (sheet_w, sheet_h) = sheet_dimensions(doc);
    let cell = calculate_grid_cell_with_grid(
        x,
        y,
        sheet_w,
        sheet_h,
        doc.settings.effective_grid_cols(),
        doc.settings.effective_grid_rows(),
    );
    format!("{}.{}", page_index + 1, cell)
}

/// Obtém o nome amigável do símbolo a partir de seu tipo CADe ou slug canônico QET.
pub fn resolve_symbol_name(item: &SchematicItem) -> String {
    if let Some(sym) = crate::symbols::find_symbol(item.tipo) {
        sym.name.to_string()
    } else if let Some(qet_id) = item.symbol_id_or_slug() {
        if let Some(dict) = crate::canonical::global_dictionary() {
            if let Some(el) = dict.get(qet_id) {
                return el.name_pt.clone();
            }
        }
        "Elemento QET".to_string()
    } else if item.is_wire() {
        "Cabo Condutor".to_string()
    } else if item.is_junction() {
        "Junção".to_string()
    } else {
        "Dispositivo".to_string()
    }
}

/// Resumo de um elemento em uma folha para o diretório de referências cruzadas.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XrefItemSummary {
    pub id: i32,
    pub page_index: usize,
    pub page_name: String,
    pub grid_ref: String,
    pub pos: (i32, i32),
    pub tipo: i32,
    pub symbol_name: String,
    pub tag: String,
    pub role: ApparatusRole,
    pub pins: Vec<String>,
    pub is_active: bool,
}

/// Agrupamento de instâncias de aparelhos por etiqueta no projeto completo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApparatusXrefGroup {
    pub tag: String,
    pub primary: Option<XrefItemSummary>,
    pub contacts: Vec<XrefItemSummary>,
    pub other_instances: Vec<XrefItemSummary>,
    pub has_duplicate_primary: bool,
    pub is_orphan_contact: bool,
}

/// Tipo de ligação elétrica entre bornes de componentes na mesma folha.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElectricalConnectionType {
    /// Ligação em paralelo (ambos os polos/terminais compartilham os mesmos nós equipotenciais)
    Parallel,
    /// Ligação em série (terminais em sequência na mesma cadeia)
    Series,
    /// Ligação direta / ramificação no mesmo nó condutor
    Direct,
}

impl ElectricalConnectionType {
    pub fn badge_label(&self) -> &'static str {
        match self {
            Self::Parallel => "Paralelo",
            Self::Series => "Série",
            Self::Direct => "Direto",
        }
    }
}

/// Alvo conectado a um borne específico.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectedTarget {
    pub item_id: i32,
    pub tag: String,
    pub pin_label: String,
    pub symbol_name: String,
    pub grid_ref: String,
    pub conn_type: ElectricalConnectionType,
}

/// Resumo de conexões elétricas de um borne do elemento selecionado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PinConnectionSummary {
    pub pin_index: usize,
    pub pin_label: String,
    pub is_connected: bool,
    pub targets: Vec<ConnectedTarget>,
}

/// Entrada de borne em uma régua de bornes (IEC 81346).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalEntry {
    pub item_id: i32,
    pub page_index: usize,
    pub page_name: String,
    pub grid_ref: String,
    pub pos: (i32, i32),
    pub terminal_number: String,
    pub numeric_seq: Option<u32>,
    pub is_duplicate: bool,
    pub connected_targets: Vec<String>,
}

/// Régua de bornes completa no projeto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalStrip {
    pub strip_tag: String,
    pub kind: TerminalStripKind,
    pub terminals: Vec<TerminalEntry>,
    pub has_duplicates: bool,
}

/// Diretório global de referências cruzadas e réguas de bornes do projeto.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectXrefDirectory {
    pub groups: Vec<ApparatusXrefGroup>,
    pub terminal_strips: Vec<TerminalStrip>,
    pub total_pages: usize,
    pub total_tags: usize,
    pub total_terminals: usize,
}

/// Contexto de referência cruzada de um elemento selecionado.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementXrefContext {
    pub selected_item: XrefItemSummary,
    pub parent_apparatus: Option<XrefItemSummary>,
    pub sibling_contacts: Vec<XrefItemSummary>,
    pub pin_connections: Vec<PinConnectionSummary>,
    pub terminal_strip: Option<TerminalStrip>,
}

/// Calcula os componentes conexos (redes equipotenciais) do grafo base de condutores.
pub fn compute_connected_components(graph: &crate::sim::graph::NetworkGraph) -> Vec<usize> {
    let num_nodes = graph.num_nodes();
    let mut comp_ids = vec![usize::MAX; num_nodes];
    let mut current_comp = 0;

    for i in 0..num_nodes {
        if comp_ids[i] == usize::MAX {
            let mut queue = VecDeque::new();
            queue.push_back(i);
            comp_ids[i] = current_comp;
            while let Some(u) = queue.pop_front() {
                for &v in &graph.base_adj[u] {
                    if comp_ids[v] == usize::MAX {
                        comp_ids[v] = current_comp;
                        queue.push_back(v);
                    }
                }
            }
            current_comp += 1;
        }
    }
    comp_ids
}

/// Identifica conexões imediatas ligadas a um elemento de borne em uma folha.
pub fn find_immediate_terminal_connections(
    item: &SchematicItem,
    doc: &SchematicDocument,
) -> Vec<String> {
    let graph = build_network_graph(&doc.items, 0);
    let item_idx = match doc.items.iter().position(|i| i.id == item.id) {
        Some(idx) => idx,
        None => return Vec::new(),
    };
    let pin_nodes = match graph.item_pin_nodes.get(&item_idx) {
        Some(pn) if !pn.is_empty() => pn,
        _ => return Vec::new(),
    };

    let comp_ids = compute_connected_components(&graph);
    let mut targets = Vec::new();

    for &node_u in pin_nodes {
        if node_u >= comp_ids.len() {
            continue;
        }
        let net_id = comp_ids[node_u];

        for (other_idx, other_pins) in &graph.item_pin_nodes {
            if *other_idx == item_idx {
                continue;
            }
            let other_item = &doc.items[*other_idx];
            if other_item.is_wire() || other_item.is_junction() {
                continue;
            }

            for (p_sub_idx, &other_node) in other_pins.iter().enumerate() {
                if other_node < comp_ids.len() && comp_ids[other_node] == net_id {
                    let pin_lbl = other_item
                        .pins
                        .get(p_sub_idx)
                        .filter(|s| !s.is_empty())
                        .cloned()
                        .unwrap_or_else(|| format!("{}", p_sub_idx + 1));
                    let sym_name = resolve_symbol_name(other_item);
                    targets.push(format!("{} : {} ({})", other_item.tag, pin_lbl, sym_name));
                }
            }
        }
    }
    targets.sort();
    targets.dedup();
    targets
}

/// Coleta o diretório completo de referências cruzadas e réguas de bornes do projeto inter-folhas.
pub fn collect_project_xrefs(
    pages: &[ProjectPage],
    active_page_index: usize,
    active_doc: &SchematicDocument,
) -> ProjectXrefDirectory {
    let total_pages = pages.len().max(1);
    let mut all_items: Vec<(usize, String, &SchematicItem, &SchematicDocument)> = Vec::new();

    for p_idx in 0..total_pages {
        let (p_name, doc) = if p_idx == active_page_index {
            let name = pages
                .get(p_idx)
                .map(|p| p.name.clone())
                .unwrap_or_else(|| format!("Folha {}", p_idx + 1));
            (name, active_doc)
        } else if let Some(p) = pages.get(p_idx) {
            (p.name.clone(), &p.doc)
        } else {
            (format!("Folha {}", p_idx + 1), active_doc)
        };

        for item in &doc.items {
            if !item.is_wire() && !item.is_junction() {
                all_items.push((p_idx, p_name.clone(), item, doc));
            }
        }
    }

    // 1. Agrupa itens por tag de aparelho
    let mut by_tag: HashMap<String, Vec<XrefItemSummary>> = HashMap::new();
    let mut terminal_items: Vec<(usize, String, &SchematicItem, &SchematicDocument)> = Vec::new();

    for (p_idx, p_name, item, doc) in all_items {
        let clean_tag = item.tag.trim().to_string();
        let role = classify_item_role(item);
        let grid_ref = format_grid_ref_doc(p_idx, item.x1 as f32, item.y1 as f32, doc);
        let symbol_name = resolve_symbol_name(item);

        let pins: Vec<String> = item
            .pins
            .iter()
            .filter(|p| !p.trim().is_empty())
            .cloned()
            .collect();

        let is_active = doc.sim_active(item.id);

        let summary = XrefItemSummary {
            id: item.id,
            page_index: p_idx,
            page_name: p_name.clone(),
            grid_ref,
            pos: (item.x1, item.y1),
            tipo: item.tipo,
            symbol_name,
            tag: clean_tag.clone(),
            role,
            pins,
            is_active,
        };

        if !clean_tag.is_empty() {
            by_tag.entry(clean_tag.clone()).or_default().push(summary);
        }

        if role == ApparatusRole::Terminal
            || ((clean_tag.to_uppercase().starts_with("-X")
                || clean_tag.to_uppercase().starts_with('X'))
                && !(3000..=3009).contains(&item.tipo))
        {
            terminal_items.push((p_idx, p_name, item, doc));
        }
    }

    // 2. Constrói ApparatusXrefGroup
    let mut groups = Vec::new();
    let mut sorted_tags: Vec<String> = by_tag.keys().cloned().collect();
    sorted_tags.sort();

    for tag in sorted_tags {
        let items = by_tag.remove(&tag).unwrap_or_default();
        let mut primaries = Vec::new();
        let mut contacts = Vec::new();
        let mut others = Vec::new();

        for it in items {
            if it.role.is_independent_apparatus() && it.role != ApparatusRole::Terminal {
                primaries.push(it);
            } else if it.role.is_subordinate_contact() {
                contacts.push(it);
            } else {
                others.push(it);
            }
        }

        let has_duplicate_primary = primaries.len() > 1;
        let is_orphan_contact = primaries.is_empty() && !contacts.is_empty();
        let primary = primaries.into_iter().next();

        groups.push(ApparatusXrefGroup {
            tag,
            primary,
            contacts,
            other_instances: others,
            has_duplicate_primary,
            is_orphan_contact,
        });
    }

    // 3. Constrói Réguas de Bornes (IEC 81346)
    let mut strips_map: HashMap<String, Vec<TerminalEntry>> = HashMap::new();
    let mut total_terminals = 0;

    for (p_idx, p_name, item, doc) in terminal_items {
        total_terminals += 1;
        let strip_tag = if item.tag.trim().is_empty() {
            "-X1".to_string()
        } else {
            item.tag.trim().to_string()
        };

        let term_num = item
            .pins
            .first()
            .filter(|s| !s.trim().is_empty())
            .cloned()
            .unwrap_or_else(|| {
                if !item.value.trim().is_empty() {
                    item.value.trim().to_string()
                } else {
                    String::new()
                }
            });

        let numeric_seq = term_num.parse::<u32>().ok();
        let grid_ref = format_grid_ref_doc(p_idx, item.x1 as f32, item.y1 as f32, doc);

        let connected_targets = find_immediate_terminal_connections(item, doc);

        strips_map
            .entry(strip_tag)
            .or_default()
            .push(TerminalEntry {
                item_id: item.id,
                page_index: p_idx,
                page_name: p_name,
                grid_ref,
                pos: (item.x1, item.y1),
                terminal_number: term_num,
                numeric_seq,
                is_duplicate: false,
                connected_targets,
            });
    }

    let mut terminal_strips = Vec::new();
    let mut strip_tags: Vec<String> = strips_map.keys().cloned().collect();
    strip_tags.sort();

    for s_tag in strip_tags {
        let mut terminals = strips_map.remove(&s_tag).unwrap_or_default();
        // Ordena por número sequencial ou folha/posição espacial
        terminals.sort_by(|a, b| match (a.numeric_seq, b.numeric_seq) {
            (Some(n1), Some(n2)) => n1.cmp(&n2),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => (a.page_index, a.pos.1, a.pos.0).cmp(&(b.page_index, b.pos.1, b.pos.0)),
        });

        // Detecção de duplicatas de número de borne na mesma régua
        let mut seen_numbers: HashSet<String> = HashSet::new();
        let mut has_duplicates = false;
        for t in &mut terminals {
            if !t.terminal_number.is_empty() && !seen_numbers.insert(t.terminal_number.clone()) {
                t.is_duplicate = true;
                has_duplicates = true;
            }
        }

        let kind = TerminalStripKind::from_tag(&s_tag);
        terminal_strips.push(TerminalStrip {
            strip_tag: s_tag,
            kind,
            terminals,
            has_duplicates,
        });
    }

    let total_tags = groups.len();
    ProjectXrefDirectory {
        groups,
        terminal_strips,
        total_pages,
        total_tags,
        total_terminals,
    }
}

/// Analisa o contexto de referência cruzada e conexões elétricas de um elemento selecionado.
pub fn analyze_element_xref(
    item_id: i32,
    pages: &[ProjectPage],
    active_page_index: usize,
    active_doc: &SchematicDocument,
) -> Option<ElementXrefContext> {
    let directory = collect_project_xrefs(pages, active_page_index, active_doc);
    let sel_item = active_doc.items.iter().find(|i| i.id == item_id)?;

    let grid_ref = format_grid_ref_doc(
        active_page_index,
        sel_item.x1 as f32,
        sel_item.y1 as f32,
        active_doc,
    );
    let symbol_name = resolve_symbol_name(sel_item);
    let role = classify_item_role(sel_item);
    let pins: Vec<String> = sel_item
        .pins
        .iter()
        .filter(|s| !s.trim().is_empty())
        .cloned()
        .collect();

    let page_name = pages
        .get(active_page_index)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| format!("Folha {}", active_page_index + 1));

    let selected_summary = XrefItemSummary {
        id: sel_item.id,
        page_index: active_page_index,
        page_name,
        grid_ref,
        pos: (sel_item.x1, sel_item.y1),
        tipo: sel_item.tipo,
        symbol_name,
        tag: sel_item.tag.trim().to_string(),
        role,
        pins,
        is_active: active_doc.sim_active(sel_item.id),
    };

    // 1. Procura grupo correspondente à tag do item selecionado
    let mut parent_apparatus = None;
    let mut sibling_contacts = Vec::new();

    if let Some(group) = directory
        .groups
        .iter()
        .find(|g| g.tag.eq_ignore_ascii_case(sel_item.tag.trim()))
    {
        if role.is_subordinate_contact() {
            parent_apparatus = group.primary.clone();
            for c in &group.contacts {
                if c.id != item_id || c.page_index != active_page_index {
                    sibling_contacts.push(c.clone());
                }
            }
        } else {
            for c in &group.contacts {
                sibling_contacts.push(c.clone());
            }
        }
    }

    // 2. Analisa ligações elétricas dos bornes (Série e Paralelo) na folha ativa
    let pin_connections = analyze_pin_connections(item_id, active_doc, active_page_index);

    // 3. Se for borne de régua, localiza a régua correspondente
    let terminal_strip = directory
        .terminal_strips
        .iter()
        .find(|s| s.strip_tag.eq_ignore_ascii_case(sel_item.tag.trim()))
        .cloned();

    Some(ElementXrefContext {
        selected_item: selected_summary,
        parent_apparatus,
        sibling_contacts,
        pin_connections,
        terminal_strip,
    })
}

/// Analisa as ligações elétricas (Série e Paralelo) de cada borne do elemento especificado.
pub fn analyze_pin_connections(
    item_id: i32,
    doc: &SchematicDocument,
    page_index: usize,
) -> Vec<PinConnectionSummary> {
    let graph = build_network_graph(&doc.items, 0);
    let sel_idx = match doc.items.iter().position(|i| i.id == item_id) {
        Some(idx) => idx,
        None => return Vec::new(),
    };
    let sel_item = &doc.items[sel_idx];
    let sel_pins = match graph.item_pin_nodes.get(&sel_idx) {
        Some(p) => p.clone(),
        None => return Vec::new(),
    };

    let comp_ids = compute_connected_components(&graph);

    let mut summaries = Vec::new();

    for (p_idx, &node_u) in sel_pins.iter().enumerate() {
        let pin_label = sel_item
            .pins
            .get(p_idx)
            .filter(|s| !s.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("{}", p_idx + 1));
        if node_u >= comp_ids.len() {
            summaries.push(PinConnectionSummary {
                pin_index: p_idx,
                pin_label,
                is_connected: false,
                targets: Vec::new(),
            });
            continue;
        }

        let net_u = comp_ids[node_u];
        let has_wire = !graph.base_adj[node_u].is_empty();
        let mut targets = Vec::new();

        // Encontra outros pinos conectados a net_u
        for (&other_idx, other_pins) in &graph.item_pin_nodes {
            if other_idx == sel_idx {
                continue;
            }
            let other_item = &doc.items[other_idx];
            if other_item.is_wire() || other_item.is_junction() {
                continue;
            }

            for (q_idx, &other_node) in other_pins.iter().enumerate() {
                if other_node < comp_ids.len() && comp_ids[other_node] == net_u {
                    // Verifica se other_item compartilha outro par de pinos com sel_item (Paralelo)
                    let is_parallel = sel_pins.iter().enumerate().any(|(p_alt, &node_alt)| {
                        if p_alt == p_idx || node_alt >= comp_ids.len() {
                            return false;
                        }
                        let net_alt = comp_ids[node_alt];
                        other_pins
                            .iter()
                            .enumerate()
                            .any(|(q_alt, &node_other_alt)| {
                                q_alt != q_idx
                                    && node_other_alt < comp_ids.len()
                                    && comp_ids[node_other_alt] == net_alt
                            })
                    });

                    let conn_type = if is_parallel {
                        ElectricalConnectionType::Parallel
                    } else if (sel_item.tipo >= 1000 || other_item.tipo >= 1000)
                        && (has_wire || node_u == other_node)
                    {
                        ElectricalConnectionType::Series
                    } else {
                        ElectricalConnectionType::Direct
                    };

                    let target_pin_label = other_item
                        .pins
                        .get(q_idx)
                        .filter(|s| !s.is_empty())
                        .cloned()
                        .unwrap_or_else(|| format!("{}", q_idx + 1));
                    let target_grid = format_grid_ref_doc(
                        page_index,
                        other_item.x1 as f32,
                        other_item.y1 as f32,
                        doc,
                    );
                    let sym_name = resolve_symbol_name(other_item);

                    targets.push(ConnectedTarget {
                        item_id: other_item.id,
                        tag: other_item.tag.clone(),
                        pin_label: target_pin_label,
                        symbol_name: sym_name,
                        grid_ref: target_grid,
                        conn_type,
                    });
                }
            }
        }

        let is_connected = has_wire || !targets.is_empty();
        summaries.push(PinConnectionSummary {
            pin_index: p_idx,
            pin_label,
            is_connected,
            targets,
        });
    }

    summaries
}

/// Renumera sequencialmente (1..N) todos os bornes de uma régua específica em todo o projeto.
pub fn renumber_terminal_strip(
    strip_tag: &str,
    pages: &mut [ProjectPage],
    active_page_index: usize,
    active_doc: &mut SchematicDocument,
) {
    if active_page_index < pages.len() {
        pages[active_page_index].doc = active_doc.clone();
    }
    let norm_tag = strip_tag.trim().to_uppercase();

    // Coleta todos os itens de borne da régua
    let mut entries = Vec::new();
    for (p_idx, page) in pages.iter().enumerate() {
        for item in &page.doc.items {
            if item.tag.trim().to_uppercase() == norm_tag {
                entries.push((p_idx, item.id, item.y1, item.x1));
            }
        }
    }

    // Ordenação determinística: folha, depois Y (superior->inferior), depois X (esquerda->direita)
    entries.sort_by_key(|&(p, _, y, x)| (p, y, x));

    for (idx, &(p_idx, item_id, _, _)) in entries.iter().enumerate() {
        let new_num = format!("{}", idx + 1);
        if p_idx < pages.len() {
            if let Some(item) = pages[p_idx].doc.items.iter_mut().find(|i| i.id == item_id) {
                item.pins[0] = new_num.clone();
                if item.value.trim().chars().all(|c| c.is_ascii_digit()) {
                    item.value = new_num.clone();
                }
            }
        }
        if p_idx == active_page_index {
            if let Some(item) = active_doc.items.iter_mut().find(|i| i.id == item_id) {
                item.pins[0] = new_num.clone();
                if item.value.trim().chars().all(|c| c.is_ascii_digit()) {
                    item.value = new_num;
                }
            }
        }
    }
}

/// Renumera sequencialmente (1..N) todas as réguas de bornes encontradas no projeto.
pub fn renumber_all_terminal_strips(
    pages: &mut [ProjectPage],
    active_page_index: usize,
    active_doc: &mut SchematicDocument,
) {
    let directory = collect_project_xrefs(pages, active_page_index, active_doc);
    let strip_tags: Vec<String> = directory
        .terminal_strips
        .iter()
        .map(|s| s.strip_tag.clone())
        .collect();
    for tag in strip_tags {
        renumber_terminal_strip(&tag, pages, active_page_index, active_doc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SchematicItem;

    #[test]
    fn test_terminal_strip_kind_classification() {
        assert_eq!(TerminalStripKind::from_tag("-XP"), TerminalStripKind::Power);
        assert_eq!(TerminalStripKind::from_tag("XP1"), TerminalStripKind::Power);
        assert_eq!(TerminalStripKind::from_tag("-X0"), TerminalStripKind::Power);
        assert_eq!(
            TerminalStripKind::from_tag("-XC"),
            TerminalStripKind::Control
        );
        assert_eq!(
            TerminalStripKind::from_tag("XC2"),
            TerminalStripKind::Control
        );
        assert_eq!(
            TerminalStripKind::from_tag("-XT"),
            TerminalStripKind::Measurement
        );
        assert_eq!(
            TerminalStripKind::from_tag("-X1"),
            TerminalStripKind::General
        );
        assert_eq!(
            TerminalStripKind::from_tag("X2"),
            TerminalStripKind::General
        );
    }

    #[test]
    fn test_calculate_grid_cell_bounds() {
        let (sheet_w, sheet_h) = (330.0, 240.0);
        assert_eq!(calculate_grid_cell(10.0, 10.0, sheet_w, sheet_h), "A1");
        assert_eq!(calculate_grid_cell(165.0, 120.0, sheet_w, sheet_h), "D5");
        assert_eq!(calculate_grid_cell(320.0, 230.0, sheet_w, sheet_h), "F8");
    }

    #[test]
    fn test_multipage_xref_grouping_and_orphan_detection() {
        let mut doc1 = SchematicDocument::default();
        let mut coil = SchematicItem::new_symbol(1, 9000, 100, 100, "-KM1");
        coil.pins[0] = "A1".to_string();
        coil.pins[1] = "A2".to_string();
        doc1.items.push(coil);

        let mut doc2 = SchematicDocument::default();
        let mut contact = SchematicItem::new_symbol(2, 7000, 200, 200, "-KM1");
        contact.pins[0] = "13".to_string();
        contact.pins[1] = "14".to_string();
        doc2.items.push(contact);

        let mut orphan_contact = SchematicItem::new_symbol(3, 7000, 250, 250, "-KM2");
        orphan_contact.pins[0] = "13".to_string();
        orphan_contact.pins[1] = "14".to_string();
        doc2.items.push(orphan_contact);

        let pages = vec![
            ProjectPage::new(1, "Potência".to_string(), doc1.clone()),
            ProjectPage::new(2, "Comando".to_string(), doc2.clone()),
        ];

        let dir = collect_project_xrefs(&pages, 0, &doc1);
        assert_eq!(dir.total_pages, 2);

        let km1 = dir
            .groups
            .iter()
            .find(|g| g.tag == "-KM1")
            .expect("-KM1 group");
        assert!(km1.primary.is_some());
        assert_eq!(km1.primary.as_ref().unwrap().page_index, 0);
        assert_eq!(km1.contacts.len(), 1);
        assert_eq!(km1.contacts[0].page_index, 1);
        assert!(!km1.is_orphan_contact);
        assert!(!km1.has_duplicate_primary);

        let km2 = dir
            .groups
            .iter()
            .find(|g| g.tag == "-KM2")
            .expect("-KM2 group");
        assert!(km2.primary.is_none());
        assert_eq!(km2.contacts.len(), 1);
        assert!(km2.is_orphan_contact);
    }

    #[test]
    fn test_terminal_strip_renumbering_spatial_order() {
        let mut doc = SchematicDocument::default();
        let mut t3 = SchematicItem::new_symbol(1, 7001, 100, 300, "-XP");
        t3.pins[0] = "99".to_string();
        doc.items.push(t3);

        let mut t1 = SchematicItem::new_symbol(2, 7001, 100, 100, "-XP");
        t1.pins[0] = "5".to_string();
        doc.items.push(t1);

        let mut t2 = SchematicItem::new_symbol(3, 7001, 100, 200, "-XP");
        t2.pins[0] = "5".to_string(); // duplicado
        doc.items.push(t2);

        let mut pages = vec![ProjectPage::new(1, "Painel".to_string(), doc.clone())];

        let dir_before = collect_project_xrefs(&pages, 0, &doc);
        assert_eq!(dir_before.terminal_strips.len(), 1);
        assert!(dir_before.terminal_strips[0].has_duplicates);

        renumber_terminal_strip("-XP", &mut pages, 0, &mut doc);

        let dir_after = collect_project_xrefs(&pages, 0, &doc);
        let strip = &dir_after.terminal_strips[0];
        assert!(!strip.has_duplicates);
        assert_eq!(strip.terminals.len(), 3);
        assert_eq!(strip.terminals[0].terminal_number, "1");
        assert_eq!(strip.terminals[1].terminal_number, "2");
        assert_eq!(strip.terminals[2].terminal_number, "3");
    }
}
