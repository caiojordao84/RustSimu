use std::collections::{HashSet, VecDeque};

use crate::model::{MotorMode, Potential, SchematicDocument, SchematicItem, SimState};
use crate::sim::cache::NetworkCache;
use crate::sim::graph::NetworkGraph;
use crate::sim::rules::{
    detect_and_trip_protective_devices, is_fuse, is_qet_connector_or_terminal, is_qet_load,
    is_qet_source_dc_pos, is_qet_source_ground, is_qet_splitter_or_passive,
    is_qet_switch_or_button,
};
use crate::sim::timers::{advance_timers, collect_initial_timers};

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

/// Resolve a cascata instantânea de relés, contatores e chaves até atingir o ponto fixo de estabilidade elétrica (máx 32 iterações).
pub fn resolve_cascade(
    items: &[SchematicItem],
    sim: &mut SimState,
    network: &NetworkGraph,
    coils: &mut HashSet<String>,
    ton_trig: &HashSet<String>,
    tof_trig: &HashSet<String>,
) -> (Vec<Potential>, Vec<Vec<usize>>) {
    let num_nodes = network.num_nodes();
    let mut last_potentials = vec![Potential::None; num_nodes];
    let mut last_adj = network.base_adj.clone();

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

        let mut adj = network.base_adj.clone();
        for (idx, item) in items.iter().enumerate() {
            let Some(p) = network.item_pin_nodes.get(&idx) else {
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
            let Some(p) = network.item_pin_nodes.get(&idx) else {
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
            let Some(p) = network.item_pin_nodes.get(&idx) else {
                continue;
            };
            match item.tipo {
                9000 if p.len() >= 2 => {
                    let energized = has_voltage_diff(node_potentials[p[0]], node_potentials[p[1]]);
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
                    let powered = has_voltage_diff(node_potentials[p[0]], node_potentials[p[1]]);
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
}

/// Executa um passo da simulação elétrica e de comando (`dt_ms` milissegundos) utilizando o `SimState` da folha.
pub fn step_simulation(doc: &mut SchematicDocument, dt_ms: u64) {
    step_simulation_state(&doc.items, &mut doc.sim, dt_ms);
}

/// Executa um passo da simulação elétrica e de comando (`dt_ms` milissegundos) de forma totalmente desacoplada,
/// mantendo a lista de elementos `items` estritamente imutável (somente leitura) e aplicando todas as mutações no `SimState`.
/// Utiliza cache topológico da rede para evitar reconstruções $O(N^2)$ a cada tick.
pub fn step_simulation_state(items: &[SchematicItem], sim: &mut SimState, dt_ms: u64) {
    let network = NetworkCache::get_or_build(items, sim);
    let num_nodes = network.num_nodes();

    if network.is_empty() {
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

    // Inicializa estado de bobinas e temporizadores ativos no início do passo
    let mut energized_coils: HashSet<String> = HashSet::new();
    let mut active_timers = collect_initial_timers(items, sim);

    for item in items {
        let tag = item.tag.trim().to_uppercase();
        if tag.is_empty() {
            continue;
        }
        let s = sim.get(item.id);
        if item.tipo == 9000 && s.active {
            energized_coils.insert(tag);
        }
    }

    // Passo A: Resolve a cascata de relés para o estado atual
    let (mut final_node_potentials, mut final_adj) = resolve_cascade(
        items,
        sim,
        &network,
        &mut energized_coils,
        &active_timers.ton_triggered,
        &active_timers.tof_triggered,
    );

    // Passo A.1: Detecção de Curto-Circuito e Desarme (Trip) de Disjuntores e Fusíveis
    if detect_and_trip_protective_devices(
        items,
        sim,
        &network.item_pin_nodes,
        &final_node_potentials,
    ) {
        let (pots2, adj2) = resolve_cascade(
            items,
            sim,
            &network,
            &mut energized_coils,
            &active_timers.ton_triggered,
            &active_timers.tof_triggered,
        );
        final_node_potentials = pots2;
        final_adj = adj2;
    }

    // Passo B: Avança os acumuladores dos temporizadores TON (9004) e TOF (9005)
    if advance_timers(items, sim, dt_ms, &mut active_timers) {
        let (pots2, adj2) = resolve_cascade(
            items,
            sim,
            &network,
            &mut energized_coils,
            &active_timers.ton_triggered,
            &active_timers.tof_triggered,
        );
        final_node_potentials = pots2;
        final_adj = adj2;

        if detect_and_trip_protective_devices(
            items,
            sim,
            &network.item_pin_nodes,
            &final_node_potentials,
        ) {
            let (pots3, adj3) = resolve_cascade(
                items,
                sim,
                &network,
                &mut energized_coils,
                &active_timers.ton_triggered,
                &active_timers.tof_triggered,
            );
            final_node_potentials = pots3;
            final_adj = adj3;
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
        let Some(p) = network.item_pin_nodes.get(&idx) else {
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
    for &(w_idx, n1, n2) in &network.wire_nodes {
        let p1 = final_node_potentials[n1];
        let p2 = final_node_potentials[n2];
        let s = sim.get_mut(items[w_idx].id);
        s.potential = if p1 != Potential::None { p1 } else { p2 };
        s.active = s.potential.is_energized();
    }
    for item in items {
        if item.is_junction() {
            if let Some(&node_idx) = network.pt_map.get(&(item.x1, item.y1)) {
                let s = sim.get_mut(item.id);
                s.potential = final_node_potentials[node_idx];
                s.active = s.potential.is_energized();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_voltage_diff_rules() {
        // Fase x Neutro
        assert!(has_voltage_diff(Potential::L1, Potential::N));
        assert!(has_voltage_diff(Potential::N, Potential::L3));

        // Fase x Fase
        assert!(has_voltage_diff(Potential::L1, Potential::L2));
        assert!(has_voltage_diff(Potential::L2, Potential::L3));

        // DC Pos x DC Neg
        assert!(has_voltage_diff(Potential::DcPos, Potential::DcNeg));
        assert!(has_voltage_diff(Potential::DcNeg, Potential::DcPos));

        // Mesma fase / potencial
        assert!(!has_voltage_diff(Potential::L1, Potential::L1));
        assert!(!has_voltage_diff(Potential::DcPos, Potential::DcPos));

        // Terra PE ou sem potencial
        assert!(!has_voltage_diff(Potential::L1, Potential::PE));
        assert!(!has_voltage_diff(Potential::L1, Potential::None));
        assert!(!has_voltage_diff(Potential::ShortCircuit, Potential::L1));
    }
}
