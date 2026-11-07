use std::collections::{HashMap, HashSet};

use crate::model::{Potential, SchematicDocument, SchematicItem, SimState};

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
pub fn is_item_manual(item: &SchematicItem) -> bool {
    if matches!(
        item.tipo,
        2004 | 5000 | 5003 | 5008 | 6000 | 6003 | 6004 | 6007 | 6008 | 6009 | 8000..=8034
    ) {
        return true;
    }
    if let Some(sym_ref) = item.symbol_id_or_slug() {
        if let Some(dict) = crate::canonical::global_dictionary() {
            if let Some(k) = dict.kind(sym_ref) {
                if matches!(
                    k,
                    "momentary_button"
                        | "latching_switch"
                        | "selector_switch"
                        | "emergency_stop"
                        | "limit_switch"
                        | "circuit_breaker"
                        | "disconnector"
                        | "fuse"
                ) {
                    return true;
                }
            }
        }
        return is_qet_switch_or_button(sym_ref);
    }
    false
}

/// Verifica se o `tipo` corresponde a um botão pulsador momentâneo
/// (`8000..=8003` Botões Pulsadores, `8004..=8007` Botões Cogumelo Simples, `8012..=8015` Fins de Curso, `8023` Botão 2D),
/// ou componente QET pulsador (ex: horn_switch, taster, limitswitch).
pub fn is_momentary_button(tipo: i32) -> bool {
    matches!(tipo, 8000..=8007 | 8012..=8015 | 8023)
}

pub fn is_item_momentary(item: &SchematicItem) -> bool {
    if is_momentary_button(item.tipo) {
        return true;
    }
    if let Some(sym_ref) = item.symbol_id_or_slug() {
        if let Some(dict) = crate::canonical::global_dictionary() {
            if let Some(k) = dict.kind(sym_ref) {
                return matches!(k, "momentary_button" | "limit_switch");
            }
        }
        let l = sym_ref.to_ascii_lowercase();
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
pub fn is_circuit_breaker(item: &SchematicItem) -> bool {
    if matches!(item.tipo, 5008 | 6000 | 6003 | 6004 | 6008 | 6009) {
        return true;
    }
    if let Some(sym_ref) = item.symbol_id_or_slug() {
        if let Some(dict) = crate::canonical::global_dictionary() {
            if let Some(k) = dict.kind(sym_ref) {
                return matches!(k, "circuit_breaker" | "disconnector");
            }
        }
        let l = sym_ref.to_ascii_lowercase();
        return l.contains("breaker")
            || l.contains("disjoncteur")
            || l.contains("leistungs_schalter");
    }
    false
}

/// Verifica se o item representa um cartucho fusível permanente
/// que conduz em repouso (`sim_manual_latched == false`) e abre ao queimar.
pub fn is_fuse(item: &SchematicItem) -> bool {
    if matches!(item.tipo, 5000 | 5003) {
        return true;
    }
    if let Some(sym_ref) = item.symbol_id_or_slug() {
        if let Some(dict) = crate::canonical::global_dictionary() {
            if let Some(k) = dict.kind(sym_ref) {
                return k == "fuse";
            }
        }
        let l = sym_ref.to_ascii_lowercase();
        return l.contains("fuse") || l.contains("sicherung");
    }
    false
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protective_device_poles_mapping() {
        assert_eq!(protective_device_poles(5000), &[(0, 1)]);
        assert_eq!(protective_device_poles(6008), &[(0, 2), (1, 3)]);
        assert_eq!(protective_device_poles(5003), &[(0, 3), (1, 4), (2, 5)]);
        assert_eq!(
            protective_device_poles(6004),
            &[(0, 4), (1, 5), (2, 6), (3, 7)]
        );
        assert_eq!(protective_device_poles(9999), &[]);
    }

    #[test]
    fn test_qet_classification_rules() {
        assert!(is_qet_switch_or_button("main_switch"));
        assert!(is_qet_switch_or_button("push_button_no"));
        assert!(!is_qet_switch_or_button("power_supply_dc"));

        assert!(is_qet_splitter_or_passive("distributor_block"));
        assert!(is_qet_connector_or_terminal("terminal_block_2p"));
        assert!(is_qet_load("ac_motor_3phase"));
        assert!(is_qet_source_dc_pos("battery_12v"));
        assert!(is_qet_source_ground("earth_ground"));
    }
}
