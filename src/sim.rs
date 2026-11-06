use std::collections::{HashMap, HashSet, VecDeque};

use crate::model::{MotorMode, Potential, SchematicDocument, SchematicItem, SimState};
use crate::symbols::find_symbol_ref;

/// Verifica se dois potenciais formam uma diferença de tensão válida para acionar bobinas, lâmpadas ou motores 1~.
pub fn has_voltage_diff(p1: Potential, p2: Potential) -> bool {
    if p1 == p2
        || !p1.is_energized()
        || !p2.is_energized()
        || p1 == Potential::ShortCircuit
        || p2 == Potential::ShortCircuit
        || p1 == Potential::PE
        || p2 == Potential::PE
    {
        return false;
    }
    matches!(
        (p1, p2),
        (Potential::L1 | Potential::L2 | Potential::L3, Potential::N)
            | (Potential::N, Potential::L1 | Potential::L2 | Potential::L3)
            | (
                Potential::L1 | Potential::L2 | Potential::L3,
                Potential::L1 | Potential::L2 | Potential::L3
            )
            | (Potential::DcPos, Potential::DcNeg)
            | (Potential::DcNeg, Potential::DcPos)
    )
}

/// Verifica se um ponto `(px, py)` está exatamente sobre o segmento de cabo `(x1, y1) - (x2, y2)`.
fn point_on_segment(px: i32, py: i32, x1: i32, y1: i32, x2: i32, y2: i32) -> bool {
    if x1 == x2 && px == x1 {
        return py >= y1.min(y2) && py <= y1.max(y2);
    }
    if y1 == y2 && py == y1 {
        return px >= x1.min(x2) && px <= x1.max(x2);
    }
    // Caso diagonal geral (colinear e dentro da caixa delimitadora)
    let min_x = x1.min(x2);
    let max_x = x1.max(x2);
    let min_y = y1.min(y2);
    let max_y = y1.max(y2);
    if px < min_x || px > max_x || py < min_y || py > max_y {
        return false;
    }
    let cross = (px - x1) as i64 * (y2 - y1) as i64 - (py - y1) as i64 * (x2 - x1) as i64;
    cross == 0
}

/// Verifica se `label` contém qualquer uma das palavras/tokens fornecidos como palavra inteira (delimitada por pontuação, barra ou underline).
pub fn has_any_token(label: &str, tokens: &[&str]) -> bool {
    let mut words = label.split(|c: char| !c.is_alphanumeric());
    words.any(|w| tokens.iter().any(|t| w.eq_ignore_ascii_case(t)))
}

/// Casador híbrido: verifica se algum radical (stem) está contido em qualquer parte de `label` (para compostos como
/// "Endschalter", "limitswitch", "microswitch", "Drucktaster", "pushbutton") OU se casa exatamente como palavra inteira (exact).
pub fn matches_label(label: &str, stems: &[&str], exact: &[&str]) -> bool {
    let l = label.to_ascii_lowercase();
    stems.iter().any(|s| l.contains(s)) || has_any_token(label, exact)
}

/// Heurística para identificar interruptores, seccionadores ou botões manuais da biblioteca QET.
pub fn is_qet_switch_or_button(label: &str) -> bool {
    matches_label(
        label,
        &[
            "switch",
            "schalter",
            "ignition",
            "button",
            "taster",
            "bouton",
            "interrupteur",
        ],
        &[],
    )
}

/// Heurística para identificar distribuidores, divisores ou conexões passivas da biblioteca QET.
pub fn is_qet_splitter_or_passive(label: &str) -> bool {
    matches_label(
        label,
        &[
            "splitter",
            "verteiler",
            "distributor",
            "splice",
            "repartiteur",
            "combine",
            "lphd",
        ],
        &[],
    )
}

/// Heurística para identificar conectores, réguas de bornes ou terminais da biblioteca QET.
/// Quando possuem exatamente 2 pinos (caso comum de bornes de passagem), conduzem de um lado para o outro.
pub fn is_qet_connector_or_terminal(label: &str) -> bool {
    matches_label(
        label,
        &[
            "terminal",
            "bornier",
            "borne",
            "connector",
            "connecteur",
            "klemme",
            "stecker",
        ],
        &[],
    )
}

/// Heurística para identificar cargas elétricas consumidoras (motores, lâmpadas, buzinas, ventoinhas) da QET.
pub fn is_qet_load(label: &str) -> bool {
    let is_switch_or_diag = matches_label(
        label,
        &["switch", "schalter"],
        &["connector", "diagram", "plan"],
    );
    !is_switch_or_diag
        && matches_label(
            label,
            &["motor", "starter", "klaxon", "sirene", "voyant"],
            &["fan", "light", "lamp", "lampe", "bulb", "horn"],
        )
}

/// Heurística para identificar fontes de alimentação contínua positiva (bateria / terminal positivo) da QET.
pub fn is_qet_source_dc_pos(label: &str) -> bool {
    matches_label(label, &["batter"], &["plus", "dcpos"])
}

/// Heurística para identificar terra / referência negativa de corrente contínua da QET.
pub fn is_qet_source_ground(label: &str) -> bool {
    matches_label(label, &["ground", "masse"], &["gnd", "earth", "terre"])
}

/// Verifica se o item pode ser acionado/comutado manualmente pelo usuário.
pub fn is_item_manual(item: &crate::model::SchematicItem) -> bool {
    matches!(
        item.tipo,
        2004 | 5000 | 5003 | 5008 | 6000 | 6003 | 6004 | 6007 | 6008 | 6009 | 8000..=8034
    ) || item
        .qet_type
        .as_ref()
        .map(|t| is_qet_switch_or_button(t))
        .unwrap_or(false)
}

/// Extrai o tempo programado (em milissegundos) de um temporizador `9004` (TON) ou `9005` (TOF).
/// No formato `.cad` original, o tempo em segundos fica em `pins[3]` (ou `value` no inspetor).
pub fn timer_preset_ms(pins: &[String; 9], value: &str) -> u64 {
    let sec = pins[3]
        .trim()
        .parse::<f64>()
        .ok()
        .or_else(|| value.trim().parse::<f64>().ok())
        .unwrap_or(3.0)
        .max(0.1);
    (sec * 1000.0).round() as u64
}

/// Verifica se o `tipo` corresponde a um botão pulsador momentâneo
/// (`8000..=8003` Botões Pulsadores, `8004..=8007` Botões Cogumelo Simples, `8012..=8015` Fins de Curso, `8023` Botão 2D),
/// ou componente QET pulsador (ex: horn_switch, taster, limitswitch).
pub fn is_momentary_button(tipo: i32) -> bool {
    matches!(tipo, 8000..=8007 | 8012..=8015 | 8023)
}

pub fn is_item_momentary(item: &crate::model::SchematicItem) -> bool {
    if is_momentary_button(item.tipo) {
        return true;
    }
    if let Some(ref qet_type) = item.qet_type {
        let l = qet_type.to_ascii_lowercase();
        if l.contains("emergency") || l.contains("latch") {
            return false;
        }
        return l.contains("horn_switch")
            || l.contains("push_button")
            || l.contains("taster")
            || l.contains("limitswitch");
    }
    false
}

/// Verifica se o item representa um disjuntor ou chave seccionadora de proteção
/// que conduz quando acionado manualmente (`sim_manual_latched == true`).
pub fn is_circuit_breaker(item: &crate::model::SchematicItem) -> bool {
    matches!(item.tipo, 5008 | 6000 | 6003 | 6004 | 6008 | 6009)
        || item
            .qet_type
            .as_ref()
            .map(|t| {
                let l = t.to_ascii_lowercase();
                l.contains("breaker")
                    || l.contains("disjoncteur")
                    || l.contains("leistungs_schalter")
            })
            .unwrap_or(false)
}

/// Verifica se o item representa um cartucho fusível permanente
/// que conduz em repouso (`sim_manual_latched == false`) e abre ao queimar.
pub fn is_fuse(item: &crate::model::SchematicItem) -> bool {
    matches!(item.tipo, 5000 | 5003)
        || item
            .qet_type
            .as_ref()
            .map(|t| {
                let l = t.to_ascii_lowercase();
                l.contains("fuse") || l.contains("sicherung")
            })
            .unwrap_or(false)
}

/// Retorna os pares de índices de pinos correspondentes aos polos de condução de disjuntores e fusíveis.
pub fn protective_device_poles(tipo: i32) -> &'static [(usize, usize)] {
    match tipo {
        // Unipolar 1P: pino 0 (entrada) - pino 1 (saída)
        5000 | 6000 => &[(0, 1)],
        // Bipolar 2P: polos 1-2 e 3-4 (pinos 0-2 e 1-3)
        6008 => &[(0, 2), (1, 3)],
        // Tripolar 3P: polos 1-2, 3-4, 5-6 (pinos 0-3, 1-4, 2-5)
        5003 | 5008 | 6003 | 6009 => &[(0, 3), (1, 4), (2, 5)],
        // Tetrapolar 4P: polo Neutro N-N e polos 1-2, 3-4, 5-6 (pinos 0-4, 1-5, 2-6, 3-7)
        6004 => &[(0, 4), (1, 5), (2, 6), (3, 7)],
        _ => &[],
    }
}

/// Detecta se algum disjuntor fechado ou fusível intacto está conduzindo corrente de curto-circuito
/// (`Potential::ShortCircuit`). Se houver, desarma (trip) os dispositivos e comuta os contatos para aberto.
pub fn detect_and_trip_protective_devices(
    items: &[SchematicItem],
    sim: &mut SimState,
    item_pin_nodes: &HashMap<usize, Vec<usize>>,
    node_potentials: &[Potential],
) -> bool {
    let mut tripped_tags = HashSet::new();
    let mut tripped_indices = HashSet::new();

    for (idx, item) in items.iter().enumerate() {
        let Some(p) = item_pin_nodes.get(&idx) else {
            continue;
        };

        let is_cb = is_circuit_breaker(item);
        let is_fu = is_fuse(item);
        let s = sim.get(item.id);

        if is_cb && s.manual_latched {
            let poles = protective_device_poles(item.tipo);
            let has_sc = if !poles.is_empty() {
                poles.iter().any(|&(a, b)| {
                    (a < p.len() && node_potentials[p[a]] == Potential::ShortCircuit)
                        || (b < p.len() && node_potentials[p[b]] == Potential::ShortCircuit)
                })
            } else {
                p.iter()
                    .any(|&node| node_potentials[node] == Potential::ShortCircuit)
            };

            if has_sc {
                tripped_indices.insert(idx);
                let tag = item.tag.trim().to_uppercase();
                if !tag.is_empty() {
                    tripped_tags.insert(tag);
                }
            }
        } else if is_fu && !s.manual_latched {
            let poles = protective_device_poles(item.tipo);
            let has_sc = if !poles.is_empty() {
                poles.iter().any(|&(a, b)| {
                    (a < p.len() && node_potentials[p[a]] == Potential::ShortCircuit)
                        || (b < p.len() && node_potentials[p[b]] == Potential::ShortCircuit)
                })
            } else {
                p.iter()
                    .any(|&node| node_potentials[node] == Potential::ShortCircuit)
            };

            if has_sc {
                tripped_indices.insert(idx);
                let tag = item.tag.trim().to_uppercase();
                if !tag.is_empty() {
                    tripped_tags.insert(tag);
                }
            }
        }
    }

    if tripped_indices.is_empty() {
        return false;
    }

    for (idx, item) in items.iter().enumerate() {
        let tag = item.tag.trim().to_uppercase();
        let is_match =
            tripped_indices.contains(&idx) || (!tag.is_empty() && tripped_tags.contains(&tag));

        if is_match {
            let s = sim.get_mut(item.id);
            if is_circuit_breaker(item) {
                s.manual_latched = false; // Desarma o disjuntor (abre os contatos principais)
                s.fault = true; // Sinaliza visualmente TRIP / FALHA
                s.active = false;
            } else if is_fuse(item) {
                s.manual_latched = true; // Fusível queima (abre circuito)
                s.fault = true;
                s.active = false;
            } else {
                s.fault = true;
                s.active = false;
            }
        }
    }

    true
}

/// Aciona um componente ao pressionar o botão esquerdo do mouse durante a simulação:
/// - Para botões pulsadores momentâneos, define o estado como pressionado (`true`).
/// - Para chaves seletoras/interruptores bi-estáveis, disjuntores e relés térmicos, alterna o estado (`!manual_latched`).
pub fn press_component_on_sim_state(items: &[SchematicItem], sim: &mut SimState, clicked_id: i32) {
    let Some(idx) = items.iter().position(|i| i.id == clicked_id) else {
        return;
    };
    let tag = items[idx].tag.trim().to_uppercase();

    if is_item_momentary(&items[idx]) {
        sim.get_mut(clicked_id).manual_latched = true;
        if !tag.is_empty() {
            for item in items {
                if is_item_momentary(item) && item.tag.trim().eq_ignore_ascii_case(&tag) {
                    sim.get_mut(item.id).manual_latched = true;
                }
            }
        }
        return;
    }

    toggle_component_on_click_state(items, sim, clicked_id);
}

pub fn press_component_on_sim(doc: &mut SchematicDocument, clicked_id: i32) {
    press_component_on_sim_state(&doc.items, &mut doc.sim, clicked_id);
}

/// Solta um botão pulsador momentâneo ao liberar o botão esquerdo do mouse,
/// fazendo-o retornar imediatamente ao seu estado de repouso original (`false`).
pub fn release_component_on_sim_state(
    items: &[SchematicItem],
    sim: &mut SimState,
    released_id: i32,
) {
    let Some(idx) = items.iter().position(|i| i.id == released_id) else {
        return;
    };
    if !is_item_momentary(&items[idx]) {
        return;
    }
    let tag = items[idx].tag.trim().to_uppercase();
    sim.get_mut(released_id).manual_latched = false;
    if !tag.is_empty() {
        for item in items {
            if is_item_momentary(item) && item.tag.trim().eq_ignore_ascii_case(&tag) {
                sim.get_mut(item.id).manual_latched = false;
            }
        }
    }
}

pub fn release_component_on_sim(doc: &mut SchematicDocument, released_id: i32) {
    release_component_on_sim_state(&doc.items, &mut doc.sim, released_id);
}

/// Alterna o estado manual de um componente clicado durante a simulação
/// (disjuntores, chaves, botões pulsadores, relés térmicos) e sincroniza irmãos com a mesma tag.
pub fn toggle_component_on_click_state(
    items: &[SchematicItem],
    sim: &mut SimState,
    clicked_id: i32,
) {
    let Some(idx) = items.iter().position(|i| i.id == clicked_id) else {
        return;
    };
    let tag = items[idx].tag.trim().to_uppercase();

    if !is_item_manual(&items[idx]) {
        return;
    }

    let new_state = !sim.get(clicked_id).manual_latched;
    {
        let s = sim.get_mut(clicked_id);
        s.manual_latched = new_state;
        if new_state {
            s.fault = false;
        }
    }

    if !tag.is_empty() {
        for item in items {
            if item.tag.trim().eq_ignore_ascii_case(&tag) && is_item_manual(item) {
                let s = sim.get_mut(item.id);
                s.manual_latched = new_state;
                if new_state {
                    s.fault = false;
                }
            }
        }
    }
}

pub fn toggle_component_on_click(doc: &mut SchematicDocument, clicked_id: i32) {
    toggle_component_on_click_state(&doc.items, &mut doc.sim, clicked_id);
}

/// Define explicitamente o estado manual (`pressed`) de todos os botões/chaves/disjuntores com a `tag` informada.
pub fn set_manual_by_tag_state(
    items: &[SchematicItem],
    sim: &mut SimState,
    tag: &str,
    pressed: bool,
) {
    let target = tag.trim();
    for item in items {
        if item.tag.trim().eq_ignore_ascii_case(target) && is_item_manual(item) {
            let s = sim.get_mut(item.id);
            s.manual_latched = pressed;
            if pressed {
                s.fault = false;
            }
        }
    }
}

pub fn set_manual_by_tag(doc: &mut SchematicDocument, tag: &str, pressed: bool) {
    set_manual_by_tag_state(&doc.items, &mut doc.sim, tag, pressed);
}

/// Executa um passo da simulação elétrica e de comando (`dt_ms` milissegundos) utilizando o `SimState` da folha.
pub fn step_simulation(doc: &mut SchematicDocument, dt_ms: u64) {
    step_simulation_state(&doc.items, &mut doc.sim, dt_ms);
}

/// Executa um passo da simulação elétrica e de comando (`dt_ms` milissegundos) de forma totalmente desacoplada,
/// mantendo a lista de elementos `items` estritamente imutável (somente leitura) e aplicando todas as mutações no `SimState`.
pub fn step_simulation_state(items: &[SchematicItem], sim: &mut SimState, dt_ms: u64) {
    let mut all_points: Vec<(i32, i32)> = Vec::new();
    let mut pt_map: HashMap<(i32, i32), usize> = HashMap::new();
    let mut add_unique_pt = |pt: (i32, i32)| -> usize {
        if let Some(&idx) = pt_map.get(&pt) {
            idx
        } else {
            let idx = all_points.len();
            all_points.push(pt);
            pt_map.insert(pt, idx);
            idx
        }
    };

    // 1. Registra pontas de cabos e pontos de conexão (empalmes 4001)
    let mut wire_nodes: Vec<(usize, usize, usize)> = Vec::new(); // (item_idx, n1, n2)
    for (idx, item) in items.iter().enumerate() {
        if item.is_wire() {
            let n1 = add_unique_pt((item.x1, item.y1));
            let n2 = add_unique_pt((item.x2, item.y2));
            wire_nodes.push((idx, n1, n2));
        } else if item.is_junction() {
            add_unique_pt((item.x1, item.y1));
        }
    }

    // 2. Registra os pinos de todos os símbolos
    let mut item_pin_nodes: HashMap<usize, Vec<usize>> = HashMap::new();
    for (idx, item) in items.iter().enumerate() {
        if let Some(sym) = find_symbol_ref(item.tipo) {
            let mut p_nodes = Vec::new();
            for (_, _, wpos) in sym.world_pins(item) {
                p_nodes.push(add_unique_pt(wpos));
            }
            item_pin_nodes.insert(idx, p_nodes);
        } else if let Some(ref qet_type) = item.qet_type {
            if let Some(parsed) = crate::svg_symbol::get_parsed_svg(qet_type) {
                let mut p_nodes = Vec::new();
                for shape in &parsed.shapes {
                    if let crate::svg_symbol::SvgShape::Circle {
                        cx,
                        cy,
                        is_terminal: true,
                        ..
                    } = shape
                    {
                        let (rx, ry) =
                            crate::model::transform_offset_f32(*cx, *cy, item.orientation);
                        let wpos = (
                            (item.x1 as f32 + rx).round() as i32,
                            (item.y1 as f32 + ry).round() as i32,
                        );
                        p_nodes.push(add_unique_pt(wpos));
                    }
                }
                item_pin_nodes.insert(idx, p_nodes);
            }
        }
    }

    let num_nodes = all_points.len();
    if num_nodes == 0 {
        // Atualiza estado base dos componentes e zera bobinas, motores e lâmpadas sem fiação
        for item in items {
            let s = sim.get_mut(item.id);
            s.potential = Potential::None;
            match item.tipo {
                5000 | 5003 => {
                    s.active = !s.manual_latched;
                }
                2004 | 5008 | 6000 | 6003 | 6004 | 6007 | 6008 | 6009 | 8000..=8034 => {
                    s.active = s.manual_latched;
                }
                // Motores
                1000..=1002 => {
                    s.active = false;
                    s.fault = false;
                    s.motor_dir = 0;
                    s.motor_mode = MotorMode::Off;
                }
                // Bobinas, contatores, temporizadores, sinaleiros e buzinas
                2000 | 2001 | 2005 | 2006 | 2007 | 2008 | 2009 | 7000..=7009 | 9004 | 9005 => {
                    s.active = false;
                    s.timer_elapsed_ms = 0;
                }
                _ => {
                    if let Some(ref qet_type) = item.qet_type {
                        if is_qet_switch_or_button(qet_type) {
                            s.active = s.manual_latched;
                        } else if is_fuse(item) {
                            s.active = !s.manual_latched;
                        } else {
                            s.active = false;
                            s.motor_dir = 0;
                        }
                    } else {
                        s.active = false;
                    }
                }
            }
        }
        return;
    }

    // 3. Constrói adjacência estática dos cabos subdividindo cada segmento pelos pontos especiais (pinos, pontas e nós 4001)
    let mut base_adj: Vec<Vec<usize>> = vec![Vec::new(); num_nodes];
    for &(_w_idx, n1, n2) in &wire_nodes {
        let (x1, y1) = all_points[n1];
        let (x2, y2) = all_points[n2];
        let mut on_seg: Vec<usize> = (0..num_nodes)
            .filter(|&u| {
                let (px, py) = all_points[u];
                point_on_segment(px, py, x1, y1, x2, y2)
            })
            .collect();
        on_seg.sort_by_key(|&u| all_points[u]);
        for pair in on_seg.windows(2) {
            let u = pair[0];
            let v = pair[1];
            if u != v {
                base_adj[u].push(v);
                base_adj[v].push(u);
            }
        }
    }

    // Inicializa estado de bobinas e temporizadores ativos no início do passo
    let mut energized_coils: HashSet<String> = HashSet::new();
    let mut ton_triggered: HashSet<String> = HashSet::new();
    let mut tof_triggered: HashSet<String> = HashSet::new();

    for item in items {
        let tag = item.tag.trim().to_uppercase();
        if tag.is_empty() {
            continue;
        }
        let s = sim.get(item.id);
        match item.tipo {
            9000 if s.active => {
                energized_coils.insert(tag);
            }
            9004 => {
                let preset_ms = timer_preset_ms(&item.pins, &item.value);
                if s.active && s.timer_elapsed_ms >= preset_ms {
                    ton_triggered.insert(tag);
                }
            }
            9005 if (s.active || s.timer_elapsed_ms > 0) => {
                tof_triggered.insert(tag);
            }
            _ => {}
        }
    }

    // Função auxiliar que resolve a cascata instantânea de relés e contatores até atingir ponto fixo
    let resolve_cascade = |items: &[SchematicItem],
                           sim: &mut SimState,
                           coils: &mut HashSet<String>,
                           ton_trig: &HashSet<String>,
                           tof_trig: &HashSet<String>|
     -> (Vec<Potential>, Vec<Vec<usize>>) {
        let mut last_potentials = vec![Potential::None; num_nodes];
        let mut last_adj = base_adj.clone();

        for pass in 0..32 {
            let mut breakers_active: HashSet<String> = HashSet::new();
            for item in items {
                let tag = item.tag.trim().to_uppercase();
                if !tag.is_empty() {
                    let s = sim.get(item.id);
                    match item.tipo {
                        5000 | 5003 if !s.manual_latched => {
                            breakers_active.insert(tag);
                        }
                        2004 | 5008 | 6000 | 6003 | 6004 | 6007 | 6008 | 6009 | 8000..=8034
                            if s.manual_latched =>
                        {
                            breakers_active.insert(tag);
                        }
                        _ => {}
                    }
                }
            }

            for item in items {
                let tag = item.tag.trim().to_uppercase();
                let s = sim.get_mut(item.id);
                match item.tipo {
                    // Fusíveis 1P / 3P: conduzem por padrão (manual_latched == false)
                    5000 | 5003 => {
                        s.active = !s.manual_latched;
                    }
                    // Contatos auxiliares de relé térmico (8016 NA, 8017 NF, 8018 NA+NF, 8019 CON):
                    // Ativam se manuseados manualmente OU se o relé térmico pai tripou (breakers_active)
                    8016..=8019 => {
                        s.active =
                            s.manual_latched || (!tag.is_empty() && breakers_active.contains(&tag));
                    }
                    // Disjuntores, chaves seccionadoras, botões, fins de curso, emergências e chaves seletoras: seguem o acionamento manual
                    2004
                    | 5008
                    | 6000
                    | 6003
                    | 6004
                    | 6007
                    | 6008
                    | 6009
                    | 8000..=8015
                    | 8020..=8023
                    | 8030..=8034 => {
                        s.active = s.manual_latched;
                    }
                    // Contatores de potência e contatos auxiliares instantâneos
                    2000 | 2001 | 2006 | 2008 | 7000 | 7001 | 7002 | 7003 => {
                        s.active = !tag.is_empty()
                            && (coils.contains(&tag)
                                || ton_trig.contains(&tag)
                                || breakers_active.contains(&tag));
                    }
                    // Contatos temporizados On-Delay (TON: 7004 NA, 7005 NF, 7006 Comutador, 7007 SPDT)
                    7004..=7007 => {
                        s.active = !tag.is_empty() && ton_trig.contains(&tag);
                    }
                    // Contatos temporizados Off-Delay (TOF: 7008 NA, 7009 NF)
                    7008 | 7009 => {
                        s.active = !tag.is_empty() && tof_trig.contains(&tag);
                    }
                    _ => {
                        if let Some(ref qet_type) = item.qet_type {
                            if is_qet_switch_or_button(qet_type) {
                                s.active = s.manual_latched;
                            } else if is_fuse(item) {
                                s.active = !s.manual_latched;
                            }
                        }
                    }
                }
            }

            let mut adj = base_adj.clone();
            for (idx, item) in items.iter().enumerate() {
                let Some(p) = item_pin_nodes.get(&idx) else {
                    continue;
                };
                let mut connect_pair = |a: usize, b: usize| {
                    if a < p.len() && b < p.len() {
                        adj[p[a]].push(p[b]);
                        adj[p[b]].push(p[a]);
                    }
                };

                let s = sim.get(item.id);
                match item.tipo {
                    // Elementos 1P NA (fecham quando active == true)
                    5000 | 6000 | 2006 | 7000 | 7004 | 7008 | 8000 | 8004 | 8008 | 8012 | 8016
                    | 8031 => {
                        if s.active {
                            connect_pair(0, 1);
                        }
                    }
                    // Elementos 1P NF (conduzem quando active == false)
                    7001 | 7005 | 7009 | 8001 | 8005 | 8009 | 8013 | 8017 | 8030 | 8033 | 8034 => {
                        if !s.active {
                            connect_pair(0, 1);
                        }
                    }
                    // Contato duplo 4 terminais NA+NF (pino 0-2 é NF 11-12/95-96; pino 1-3 é NA 13-14/97-98)
                    7002 | 7006 | 8002 | 8006 | 8010 | 8014 | 8018 | 8032 => {
                        if !s.active {
                            connect_pair(0, 2);
                        } else {
                            connect_pair(1, 3);
                        }
                    }
                    // Botão / Contato Comutador CON 3 terminais SPDT (pino 0 comum; pino 1 NF; pino 2 NA)
                    7003 | 7007 | 8003 | 8007 | 8011 | 8015 | 8019 => {
                        if !s.active {
                            connect_pair(0, 1);
                        } else {
                            connect_pair(0, 2);
                        }
                    }
                    // Chave Seletora 3 Posições (I-0-II): em ativo fecha polo 1-2 (pinos 0 e 2)
                    8020 => {
                        if s.active {
                            connect_pair(0, 2);
                        }
                    }
                    // Chave Seletora 3 Posições (I-II-III): em ativo fecha polos 1-2 e 3-4 (pinos 0-3 e 1-4)
                    8021 => {
                        if s.active {
                            connect_pair(0, 3);
                            connect_pair(1, 4);
                        }
                    }
                    // Chave Inversora / Crossover: em repouso direto (0-2 e 1-3), em ativo cruzado (0-3 e 1-2)
                    8022 => {
                        if !s.active {
                            connect_pair(0, 2);
                            connect_pair(1, 3);
                        } else {
                            connect_pair(0, 3);
                            connect_pair(1, 2);
                        }
                    }
                    // Contator 2P / Disjuntor 2P (1-2, 3-4)
                    2000 | 6008 => {
                        if s.active {
                            connect_pair(0, 2);
                            connect_pair(1, 3);
                        }
                    }
                    // Elementos Tripolares 3P (1-2, 3-4, 5-6)
                    5003 | 5008 | 6003 | 6009 | 2001 | 2004 | 2008 => {
                        if s.active {
                            connect_pair(0, 3);
                            connect_pair(1, 4);
                            connect_pair(2, 5);
                        }
                    }
                    // Disjuntor Tetrapolar 4P (N-N, 1-2, 3-4, 5-6)
                    6004 => {
                        if s.active {
                            connect_pair(0, 4);
                            connect_pair(1, 5);
                            connect_pair(2, 6);
                            connect_pair(3, 7);
                        }
                    }
                    // Relé térmico 3P: bimetais principais conduzem nos polos 1-2, 3-4, 5-6
                    6007 => {
                        connect_pair(0, 3);
                        connect_pair(1, 4);
                        connect_pair(2, 5);
                    }
                    _ => {
                        if let Some(ref qet_type) = item.qet_type {
                            if is_qet_switch_or_button(qet_type) {
                                if s.manual_latched {
                                    for i in 0..p.len() {
                                        for j in (i + 1)..p.len() {
                                            connect_pair(i, j);
                                        }
                                    }
                                }
                            } else if is_fuse(item) {
                                if !s.manual_latched {
                                    for i in 0..p.len() {
                                        for j in (i + 1)..p.len() {
                                            connect_pair(i, j);
                                        }
                                    }
                                }
                            } else if is_qet_splitter_or_passive(qet_type) {
                                for i in 0..p.len() {
                                    for j in (i + 1)..p.len() {
                                        connect_pair(i, j);
                                    }
                                }
                            } else if is_qet_connector_or_terminal(qet_type) && p.len() == 2 {
                                connect_pair(0, 1);
                            }
                        }
                    }
                }
            }

            // Injeção de fontes de alimentação e propagação BFS
            let mut node_potentials = vec![Potential::None; num_nodes];
            let mut queue = VecDeque::new();

            let mut inject_source = |node: usize, pot: Potential| {
                if node_potentials[node] == Potential::None {
                    node_potentials[node] = pot;
                    queue.push_back(node);
                } else if node_potentials[node] != pot
                    && pot != Potential::PE
                    && node_potentials[node] != Potential::PE
                {
                    node_potentials[node] = Potential::ShortCircuit;
                    queue.push_back(node);
                }
            };

            for (idx, item) in items.iter().enumerate() {
                let Some(p) = item_pin_nodes.get(&idx) else {
                    continue;
                };
                match item.tipo {
                    3000 if !p.is_empty() => inject_source(p[0], Potential::L1),
                    3001 if !p.is_empty() => inject_source(p[0], Potential::N),
                    3002 if !p.is_empty() => inject_source(p[0], Potential::PE),
                    3003 if p.len() >= 2 => {
                        inject_source(p[0], Potential::L1);
                        inject_source(p[1], Potential::N);
                    }
                    3004 if p.len() >= 3 => {
                        inject_source(p[0], Potential::L1);
                        inject_source(p[1], Potential::L2);
                        inject_source(p[2], Potential::L3);
                    }
                    3005 if p.len() >= 4 => {
                        inject_source(p[0], Potential::L1);
                        inject_source(p[1], Potential::L2);
                        inject_source(p[2], Potential::L3);
                        inject_source(p[3], Potential::PE);
                    }
                    3008 if !p.is_empty() => inject_source(p[0], Potential::DcPos),
                    3009 if !p.is_empty() => inject_source(p[0], Potential::DcNeg),
                    _ => {
                        if let Some(ref qet_type) = item.qet_type {
                            let l = qet_type.to_ascii_lowercase();
                            if is_qet_source_dc_pos(qet_type) {
                                if p.len() >= 2 {
                                    inject_source(p[0], Potential::DcNeg);
                                    inject_source(p[1], Potential::DcPos);
                                } else if !p.is_empty() {
                                    inject_source(p[0], Potential::DcPos);
                                }
                            } else if is_qet_source_ground(qet_type) {
                                for &pin_node in p {
                                    inject_source(pin_node, Potential::DcNeg);
                                }
                            } else if l.contains("coming_arrow") {
                                let pot = if item.y1 >= 600 {
                                    Potential::DcNeg
                                } else {
                                    Potential::DcPos
                                };
                                for &pin_node in p {
                                    inject_source(pin_node, pot);
                                }
                            } else if (l.contains("going_arrow") && item.y1 >= 600)
                                || (l.contains("horn_switch") && sim.get(item.id).manual_latched)
                            {
                                for &pin_node in p {
                                    inject_source(pin_node, Potential::DcNeg);
                                }
                            }
                        }
                    }
                }
            }

            while let Some(u) = queue.pop_front() {
                let pot = node_potentials[u];
                for &v in &adj[u] {
                    if node_potentials[v] == Potential::None {
                        node_potentials[v] = pot;
                        queue.push_back(v);
                    } else if node_potentials[v] != pot
                        && node_potentials[v] != Potential::ShortCircuit
                        && pot != Potential::PE
                        && node_potentials[v] != Potential::PE
                    {
                        node_potentials[v] = Potential::ShortCircuit;
                        queue.push_back(v);
                    }
                }
            }

            let mut next_coils = HashSet::new();
            for (idx, item) in items.iter().enumerate() {
                let Some(p) = item_pin_nodes.get(&idx) else {
                    continue;
                };
                match item.tipo {
                    9000 if p.len() >= 2 => {
                        let energized =
                            has_voltage_diff(node_potentials[p[0]], node_potentials[p[1]]);
                        let s = sim.get_mut(item.id);
                        s.active = energized;
                        s.potential = if energized {
                            node_potentials[p[0]]
                        } else {
                            Potential::None
                        };
                        let tag = item.tag.trim().to_uppercase();
                        if energized && !tag.is_empty() {
                            next_coils.insert(tag);
                        }
                    }
                    9004 | 9005 | 9008 | 9013 if p.len() >= 2 => {
                        let powered =
                            has_voltage_diff(node_potentials[p[0]], node_potentials[p[1]]);
                        let s = sim.get_mut(item.id);
                        s.active = powered;
                        s.potential = if powered {
                            node_potentials[p[0]]
                        } else {
                            Potential::None
                        };
                    }
                    _ => {
                        if let Some(ref qet_type) = item.qet_type {
                            let l = qet_type.to_ascii_lowercase();
                            if is_qet_load(qet_type) {
                                let powered = if l.contains("starter") && p.len() >= 3 {
                                    let has_neg =
                                        p.iter().any(|&n| node_potentials[n] == Potential::DcNeg);
                                    let pos_count = p
                                        .iter()
                                        .filter(|&&n| node_potentials[n] == Potential::DcPos)
                                        .count();
                                    has_neg && pos_count >= 2
                                } else {
                                    let mut diff = false;
                                    for i in 0..p.len() {
                                        for j in (i + 1)..p.len() {
                                            if has_voltage_diff(
                                                node_potentials[p[i]],
                                                node_potentials[p[j]],
                                            ) {
                                                diff = true;
                                                break;
                                            }
                                        }
                                        if diff {
                                            break;
                                        }
                                    }
                                    diff
                                };
                                let s = sim.get_mut(item.id);
                                s.active = powered;
                                if powered {
                                    s.motor_dir = 1;
                                } else {
                                    s.motor_dir = 0;
                                }
                            }
                        }
                    }
                }
            }

            last_potentials = node_potentials;
            last_adj = adj;
            if next_coils == *coils && pass >= 1 {
                break;
            }
            *coils = next_coils;
        }

        (last_potentials, last_adj)
    };

    // Passo A: Resolve a cascata de relés para o estado atual
    let (mut final_node_potentials, mut final_adj) = resolve_cascade(
        items,
        sim,
        &mut energized_coils,
        &ton_triggered,
        &tof_triggered,
    );

    // Passo A.1: Detecção de Curto-Circuito e Desarme (Trip) de Disjuntores e Fusíveis
    // Se houver curto-circuito (Potential::ShortCircuit), qualquer disjuntor fechado ou fusível
    // intacto que esteja conduzindo a corrente de falta desarma (abre) imediatamente,
    // comutando manual_latched para false e fault para true, extinguindo o curto.
    if detect_and_trip_protective_devices(items, sim, &item_pin_nodes, &final_node_potentials) {
        let (pots2, adj2) = resolve_cascade(
            items,
            sim,
            &mut energized_coils,
            &ton_triggered,
            &tof_triggered,
        );
        final_node_potentials = pots2;
        final_adj = adj2;
    }

    // Passo B: Avança os acumuladores dos temporizadores TON (9004) e TOF (9005)
    if dt_ms > 0 {
        let mut new_ton = HashSet::new();
        let mut new_tof = HashSet::new();

        for item in items {
            let tag = item.tag.trim().to_uppercase();
            let s = sim.get_mut(item.id);
            match item.tipo {
                9004 => {
                    let preset_ms = timer_preset_ms(&item.pins, &item.value);
                    if s.active {
                        s.timer_elapsed_ms = s.timer_elapsed_ms.saturating_add(dt_ms);
                    } else {
                        s.timer_elapsed_ms = 0;
                    }
                    if s.active && s.timer_elapsed_ms >= preset_ms && !tag.is_empty() {
                        new_ton.insert(tag);
                    }
                }
                9005 => {
                    let preset_ms = timer_preset_ms(&item.pins, &item.value);
                    if s.active {
                        s.timer_elapsed_ms = preset_ms;
                    } else {
                        s.timer_elapsed_ms = s.timer_elapsed_ms.saturating_sub(dt_ms);
                    }
                    if (s.active || s.timer_elapsed_ms > 0) && !tag.is_empty() {
                        new_tof.insert(tag);
                    }
                }
                _ => {}
            }
        }

        // Passo C: Se algum temporizador disparou ou desarmou neste passo, re-resolve a cascata imediatamente
        if new_ton != ton_triggered || new_tof != tof_triggered {
            ton_triggered = new_ton;
            tof_triggered = new_tof;
            let (pots2, adj2) = resolve_cascade(
                items,
                sim,
                &mut energized_coils,
                &ton_triggered,
                &tof_triggered,
            );
            final_node_potentials = pots2;
            final_adj = adj2;

            if detect_and_trip_protective_devices(
                items,
                sim,
                &item_pin_nodes,
                &final_node_potentials,
            ) {
                let (pots3, adj3) = resolve_cascade(
                    items,
                    sim,
                    &mut energized_coils,
                    &ton_triggered,
                    &tof_triggered,
                );
                final_node_potentials = pots3;
                final_adj = adj3;
            }
        }
    }

    // Calcula componentes conexos do grafo `final_adj` para verificar fechamento em Estrela (W2-U2-V2 em ponte)
    let mut comp_id = vec![usize::MAX; num_nodes];
    let mut current_cid = 0;
    for start in 0..num_nodes {
        if comp_id[start] == usize::MAX {
            let mut stack = vec![start];
            comp_id[start] = current_cid;
            while let Some(u) = stack.pop() {
                for &v in &final_adj[u] {
                    if comp_id[v] == usize::MAX {
                        comp_id[v] = current_cid;
                        stack.push(v);
                    }
                }
            }
            current_cid += 1;
        }
    }

    // Avalia estado final de todos os motores (1000, 1001, 1002)
    for (idx, item) in items.iter().enumerate() {
        let Some(p) = item_pin_nodes.get(&idx) else {
            continue;
        };
        match item.tipo {
            // Motor Trifásico 3 terminais + PE (U1, V1, W1, PE)
            1000 if p.len() >= 3 => {
                let u = final_node_potentials[p[0]];
                let v = final_node_potentials[p[1]];
                let w = final_node_potentials[p[2]];
                let phases = [u, v, w];
                let has_l1 = phases.contains(&Potential::L1);
                let has_l2 = phases.contains(&Potential::L2);
                let has_l3 = phases.contains(&Potential::L3);
                let energized_count = phases
                    .iter()
                    .filter(|&&pt| matches!(pt, Potential::L1 | Potential::L2 | Potential::L3))
                    .count();

                let s = sim.get_mut(item.id);
                if has_l1 && has_l2 && has_l3 {
                    s.active = true;
                    s.fault = false;
                    let cw = matches!(
                        (u, v, w),
                        (Potential::L1, Potential::L2, Potential::L3)
                            | (Potential::L2, Potential::L3, Potential::L1)
                            | (Potential::L3, Potential::L1, Potential::L2)
                    );
                    s.motor_dir = if cw { 1 } else { -1 };
                    s.motor_mode = MotorMode::Direct;
                } else if energized_count >= 1 {
                    s.active = false;
                    s.fault = true;
                    s.motor_dir = 0;
                    s.motor_mode = MotorMode::Off;
                } else {
                    s.active = false;
                    s.fault = false;
                    s.motor_dir = 0;
                    s.motor_mode = MotorMode::Off;
                }
            }
            // Motor Trifásico 6 terminais Estrela-Triângulo (U1, V1, W1, PE, W2, U2, V2)
            1001 if p.len() >= 7 => {
                let u1 = final_node_potentials[p[0]];
                let v1 = final_node_potentials[p[1]];
                let w1 = final_node_potentials[p[2]];
                let w2 = final_node_potentials[p[4]];
                let u2 = final_node_potentials[p[5]];
                let v2 = final_node_potentials[p[6]];

                let top_phases = [u1, v1, w1];
                let top_all_3 = top_phases.contains(&Potential::L1)
                    && top_phases.contains(&Potential::L2)
                    && top_phases.contains(&Potential::L3);

                let s = sim.get_mut(item.id);
                if top_all_3 {
                    let cw = matches!(
                        (u1, v1, w1),
                        (Potential::L1, Potential::L2, Potential::L3)
                            | (Potential::L2, Potential::L3, Potential::L1)
                            | (Potential::L3, Potential::L1, Potential::L2)
                    );
                    let dir = if cw { 1 } else { -1 };

                    // Verifica se W2, U2, V2 estão interligados em Estrela (Y)
                    let star_bridged = comp_id[p[4]] == comp_id[p[5]]
                        && comp_id[p[5]] == comp_id[p[6]]
                        && w2 != Potential::ShortCircuit;

                    // Verifica se W2, U2, V2 estão alimentados em Triângulo (Delta)
                    let bot_phases = [w2, u2, v2];
                    let bot_all_3 = bot_phases.contains(&Potential::L1)
                        && bot_phases.contains(&Potential::L2)
                        && bot_phases.contains(&Potential::L3)
                        && u1 != u2
                        && v1 != v2
                        && w1 != w2;

                    if star_bridged {
                        s.active = true;
                        s.fault = false;
                        s.motor_dir = dir;
                        s.motor_mode = MotorMode::Star;
                    } else if bot_all_3 {
                        s.active = true;
                        s.fault = false;
                        s.motor_dir = dir;
                        s.motor_mode = MotorMode::Delta;
                    } else {
                        s.active = false;
                        s.fault = false;
                        s.motor_dir = 0;
                        s.motor_mode = MotorMode::Off;
                    }
                } else {
                    s.active = false;
                    s.fault = false;
                    s.motor_dir = 0;
                    s.motor_mode = MotorMode::Off;
                }
            }
            // Motor Monofásico (U1, V1, PE)
            1002 if p.len() >= 2 => {
                let on = has_voltage_diff(final_node_potentials[p[0]], final_node_potentials[p[1]]);
                let s = sim.get_mut(item.id);
                s.active = on;
                s.motor_dir = if on { 1 } else { 0 };
                s.motor_mode = if on {
                    MotorMode::Direct
                } else {
                    MotorMode::Off
                };
            }
            _ => {}
        }
    }

    // Atualiza o potencial visual de cada cabo e junção
    for &(w_idx, n1, n2) in &wire_nodes {
        let p1 = final_node_potentials[n1];
        let p2 = final_node_potentials[n2];
        let s = sim.get_mut(items[w_idx].id);
        s.potential = if p1 != Potential::None { p1 } else { p2 };
        s.active = s.potential.is_energized();
    }
    for item in items {
        if item.is_junction() {
            if let Some(&node_idx) = pt_map.get(&(item.x1, item.y1)) {
                let s = sim.get_mut(item.id);
                s.potential = final_node_potentials[node_idx];
                s.active = s.potential.is_energized();
            }
        }
    }
}
