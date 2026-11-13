//! Acoplamento `MotorAr` ↔ solver elétrico / `SimState`.
//!
//! Por tick: bobinas/`A1`–`A2` → carretel; `MotorAr::passo`; curso/pressostato
//! → `ItemSimState.active`; `NoAr` nos tubos para coloração.

use crate::canonical::CanonicalKind;
use crate::dictionary::global_concept_dictionary;
use crate::model::{Potential, SchematicItem, SimState};
use crate::pneu::contrato::{Acionamento, CursoCilindro, EstadoCarretel, EstadoPistao, NoAr};
use crate::pneu::motor::{
    valvula_3_2_nc, valvula_5_2, ComandoEletrico, Dispositivo, MotorAr, Porta,
};
use crate::sim::graph::NetworkGraph;
use crate::sim::solver::has_voltage_diff;
use std::collections::{HashMap, HashSet};

/// Resolve a classe FluidSIM (`PPE1`, `PVWE4`, …) de um item do canvas.
pub fn fluidsim_class_of(item: &SchematicItem) -> Option<String> {
    let dict = global_concept_dictionary();
    if let Some(ref cid) = item.concept_id {
        if let Some(c) = dict.get_by_id(cid) {
            if let Some(ref fc) = c.fluidsim_class {
                return Some(fc.clone());
            }
        }
    }
    if let Some(r) = dict.resolve_item(item) {
        if let Some(ref fc) = r.concept.fluidsim_class {
            return Some(fc.clone());
        }
    }
    if let Some(sym) = item.symbol_id_or_slug() {
        let key = sym.trim().to_ascii_uppercase();
        if dict.get_by_fluidsim_class(&key).is_some() {
            return Some(key);
        }
    }
    None
}

fn pin_nodes_named(item: &SchematicItem, pin_nodes: &[usize]) -> HashMap<String, u32> {
    let mut map = HashMap::new();
    let class_terms = fluidsim_class_of(item)
        .and_then(|fc| crate::pneu::svg_v1_classe(&fc, false))
        .map(|(_, terms)| terms);

    for (i, &n) in pin_nodes.iter().enumerate() {
        let nome = class_terms
            .as_ref()
            .and_then(|terms| terms.get(i).map(|t| t.nome.clone()))
            .or_else(|| {
                item.pins
                    .get(i)
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
            })
            .or_else(|| {
                fluidsim_class_of(item).and_then(|fc| {
                    global_concept_dictionary()
                        .get_by_fluidsim_class(&fc)
                        .and_then(|c| c.canonical_pins.get(i).cloned())
                })
            })
            .unwrap_or_else(|| (i + 1).to_string());
        map.insert(nome, n as u32);
    }
    map
}

fn portas_de(mapa: &HashMap<String, u32>, nomes: &[&str]) -> Vec<Porta> {
    nomes
        .iter()
        .filter_map(|n| mapa.get(*n).map(|&no| Porta::nova(*n, no)))
        .collect()
}

fn tag_upper(item: &SchematicItem) -> String {
    item.tag.trim().to_uppercase()
}

#[derive(Debug, PartialEq, Eq)]
struct DeviceTag<'a> {
    prefix: &'a str,
    letter: &'a str,
    index: &'a str,
}

fn parse_tag(tag: &str) -> Option<DeviceTag<'_>> {
    let s = tag.trim().trim_start_matches('-');
    if s.is_empty() {
        return None;
    }
    let letter_start = s.find(|c: char| c.is_ascii_alphabetic())?;
    let prefix = &s[..letter_start];
    let rest = &s[letter_start..];
    let letter_end = rest
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(rest.len());
    let letter = &rest[..letter_end];
    let index = &rest[letter_end..];
    Some(DeviceTag {
        prefix,
        letter,
        index,
    })
}

fn coil_tag_matches(coils: &HashSet<String>, valve_tag: &str) -> bool {
    if valve_tag.is_empty() {
        return false;
    }
    let vt_clean = valve_tag.trim().trim_start_matches('-').to_uppercase();
    if coils.iter().any(|c| {
        c.trim()
            .trim_start_matches('-')
            .eq_ignore_ascii_case(&vt_clean)
    }) {
        return true;
    }
    let Some(vt) = parse_tag(&vt_clean) else {
        return false;
    };
    for coil in coils {
        let ct_clean = coil.trim().trim_start_matches('-').to_uppercase();
        if ct_clean == vt_clean {
            return true;
        }
        let Some(ct) = parse_tag(&ct_clean) else {
            continue;
        };
        // Bobinas geralmente usam letra Y, K ou mesma letra da válvula
        if !ct.letter.contains('Y') && !ct.letter.contains('K') && ct.letter != vt.letter {
            continue;
        }
        // Prefixo numérico do grupo (ex: 1V1 vs 1Y1) deve ser idêntico se ambos existirem
        if !vt.prefix.is_empty() && !ct.prefix.is_empty() && vt.prefix != ct.prefix {
            continue;
        }
        if !vt.index.is_empty() && !ct.index.is_empty() {
            if vt.index == ct.index || ct.index.starts_with(vt.index) {
                return true;
            }
        } else if (!vt.prefix.is_empty() && vt.prefix == ct.prefix)
            || (vt.prefix.is_empty()
                && ct.prefix.is_empty()
                && !vt.index.is_empty()
                && vt.index == ct.index)
        {
            return true;
        }
    }
    false
}

fn a1_a2_energizado(mapa: &HashMap<String, u32>, potentials: &[Potential]) -> bool {
    let (Some(&a1), Some(&a2)) = (mapa.get("A1"), mapa.get("A2")) else {
        return false;
    };
    let ia = a1 as usize;
    let ib = a2 as usize;
    if ia >= potentials.len() || ib >= potentials.len() {
        return false;
    }
    has_voltage_diff(potentials[ia], potentials[ib])
}

/// (Re)constrói o `MotorAr` a partir dos itens pneumáticos da folha.
pub fn rebuild_motor(
    items: &[SchematicItem],
    network: &NetworkGraph,
    previous: &MotorAr,
) -> MotorAr {
    let mut m = MotorAr::novo();
    let mut cilindros: Vec<(i32, String)> = Vec::new();

    for (idx, item) in items.iter().enumerate() {
        let Some(classe) = fluidsim_class_of(item) else {
            continue;
        };
        let Some(concept) = global_concept_dictionary().get_by_fluidsim_class(&classe) else {
            continue;
        };
        if !concept.simular && classe != "152841" {
            continue;
        }
        let pin_nodes = network
            .item_pin_nodes
            .get(&idx)
            .cloned()
            .unwrap_or_default();
        if pin_nodes.is_empty() && !matches!(classe.as_str(), "PAL1" | "PAL2") {
            // Sem bornes no grafo: ainda assim registra se houver posição (âncora).
        }
        let mapa = pin_nodes_named(item, &pin_nodes);
        for &no in mapa.values() {
            m.garante_no(no);
        }
        let id = item.id as u32;
        match classe.as_str() {
            "PPE1" | "PRV" | "PVP1" | "PPE2" => {
                if let Some(&no) = mapa.get("1").or_else(|| mapa.values().next()) {
                    m.set_fonte_config(id, item.pneu_pressure_kpa(), item.pneu_flow_lpm_u32());
                    m.dispositivos.push(Dispositivo::Fonte { id, no });
                }
            }
            "PAV1" => {
                if let Some(&no) = mapa.get("1").or_else(|| mapa.values().next()) {
                    m.dispositivos.push(Dispositivo::BombaVacuo { id, no });
                }
            }
            "PVS4" => {
                let entrada = *mapa.get("1").unwrap_or(&0);
                let saida = *mapa.get("2").unwrap_or(&0);
                m.dispositivos
                    .push(Dispositivo::Retencao { id, entrada, saida });
            }
            "PAR2" => {
                let a = *mapa.get("1").unwrap_or(&0);
                let b = *mapa.get("2").unwrap_or(&0);
                let girando = previous.motor_girando(id).unwrap_or(false);
                m.dispositivos
                    .push(Dispositivo::MotorPneumatico { id, a, b, girando });
            }
            "PVG2" | "PVG3" => {
                let portas = portas_de(&mapa, &["1", "2", "3"]);
                for p in &portas {
                    if p.nome == "3" {
                        m.dispositivos.push(Dispositivo::Escape {
                            id: id.wrapping_add(30_000),
                            no: p.no,
                        });
                    }
                }
                let (atraso_ms, acumulado_ms, estado) = previous
                    .dispositivos
                    .iter()
                    .find_map(|d| match d {
                        Dispositivo::Temporizador {
                            id: i,
                            atraso_ms,
                            acumulado_ms,
                            valvula,
                            ..
                        } if *i == id => Some((*atraso_ms, *acumulado_ms, valvula.estado)),
                        _ => None,
                    })
                    .unwrap_or((500, 0, EstadoCarretel::Repouso));
                let sinal = *mapa.get("1").unwrap_or(&0);
                m.dispositivos.push(Dispositivo::Temporizador {
                    id,
                    valvula: valvula_3_2_nc(estado, vec![Acionamento::Piloto]),
                    portas,
                    normalmente_aberto: classe == "PVG3",
                    atraso_ms,
                    acumulado_ms,
                    sinal,
                });
            }
            "PVS2" => {
                if let Some(&no) = mapa.get("2").or_else(|| mapa.get("1")) {
                    m.dispositivos.push(Dispositivo::Escape { id, no });
                }
            }
            "PAL1" => {
                let camara = *mapa.get("1").unwrap_or(&0);
                m.garante_no(camara);
                let pistao = previous.pistao(id).unwrap_or_else(|| {
                    if item.symbol_state == Some('1') {
                        EstadoPistao::avancado()
                    } else {
                        EstadoPistao::recolhido()
                    }
                });
                m.dispositivos
                    .push(Dispositivo::CilindroSimples { id, camara, pistao });
                cilindros.push((item.id, tag_upper(item)));
            }
            "PAL2" => {
                let avanco = *mapa.get("1").unwrap_or(&0);
                let retorno = *mapa.get("2").unwrap_or(&0);
                m.garante_no(avanco);
                m.garante_no(retorno);
                let pistao = previous.pistao(id).unwrap_or_else(|| {
                    if item.symbol_state == Some('1') {
                        EstadoPistao::avancado()
                    } else {
                        EstadoPistao::recolhido()
                    }
                });
                m.dispositivos.push(Dispositivo::CilindroDuplo {
                    id,
                    avanco,
                    retorno,
                    pistao,
                });
                cilindros.push((item.id, tag_upper(item)));
            }
            "PVS1" => {
                let a = *mapa.get("1").unwrap_or(&0);
                let b = *mapa.get("2").unwrap_or(&0);
                let saida = *mapa.get("3").unwrap_or(&0);
                m.dispositivos
                    .push(Dispositivo::Alternancia { id, a, b, saida });
            }
            "PVS3" => {
                let a = *mapa.get("1").unwrap_or(&0);
                let b = *mapa.get("2").unwrap_or(&0);
                let saida = *mapa.get("3").unwrap_or(&0);
                m.dispositivos
                    .push(Dispositivo::DuasPressoes { id, a, b, saida });
            }
            "PSW1" | "PEW" => {
                let no_ar = *mapa.get("1").unwrap_or(&0);
                m.dispositivos.push(Dispositivo::Pressostato {
                    id,
                    no_ar,
                    contato_fechado: previous.contato_fechado(id).unwrap_or(false),
                });
            }
            "PSG1" => {
                let portas = portas_de(&mapa, &["1", "2", "3"]);
                for p in &portas {
                    if p.nome == "3" {
                        m.dispositivos.push(Dispositivo::Escape {
                            id: id.wrapping_add(10_000),
                            no: p.no,
                        });
                    }
                }
                let cilindro = match_cylinder(&cilindros, &tag_upper(item))
                    .or_else(|| cilindros.first().map(|(i, _)| *i as u32))
                    .unwrap_or(0);
                m.dispositivos.push(Dispositivo::FimDeCurso {
                    id,
                    cilindro,
                    ativo_em: CursoCilindro::Avancado,
                    ativo: false,
                    valvula: valvula_3_2_nc(EstadoCarretel::Repouso, vec![Acionamento::Mecanico]),
                    portas,
                });
            }
            c if c.starts_with("PVW") => {
                let is_5_2 = matches!(c, "PVWM8" | "PVWP3" | "PVWP4" | "PVWE3" | "PVWE4" | "PVWE5");
                let eletrico = matches!(c, "PVWE1" | "PVWE3" | "PVWE4" | "PVWE2" | "PVWE5");
                let impulso = matches!(c, "PVWE3" | "PVWP4");
                let manual = matches!(c, "PVWM1" | "PVWM3" | "PVWM8");
                let acion = if eletrico {
                    vec![Acionamento::Eletrico]
                } else if manual {
                    vec![Acionamento::Manual]
                } else {
                    vec![Acionamento::Piloto]
                };
                let estado = previous
                    .dispositivos
                    .iter()
                    .find_map(|d| match d {
                        Dispositivo::Direcional { id: i, valvula, .. } if *i == id => {
                            Some(valvula.estado)
                        }
                        _ => None,
                    })
                    .unwrap_or(EstadoCarretel::Repouso);
                let valvula = if is_5_2 {
                    valvula_5_2(estado, acion)
                } else {
                    valvula_3_2_nc(estado, acion)
                };
                let nomes: &[&str] = if is_5_2 {
                    &["1", "2", "3", "4", "5"]
                } else {
                    &["1", "2", "3"]
                };
                let portas = portas_de(&mapa, nomes);
                for p in &portas {
                    if p.nome == "3" || p.nome == "5" {
                        m.dispositivos.push(Dispositivo::Escape {
                            id: id.wrapping_add(20_000 + p.nome.as_bytes()[0] as u32),
                            no: p.no,
                        });
                    }
                }
                let comando = if impulso {
                    let (energizado, anterior) = previous
                        .dispositivos
                        .iter()
                        .find_map(|d| match d {
                            Dispositivo::Direcional {
                                id: i,
                                comando:
                                    ComandoEletrico::Impulso {
                                        energizado,
                                        anterior,
                                    },
                                ..
                            } if *i == id => Some((*energizado, *anterior)),
                            _ => None,
                        })
                        .unwrap_or((false, false));
                    ComandoEletrico::Impulso {
                        energizado,
                        anterior,
                    }
                } else if eletrico {
                    ComandoEletrico::Monoestavel { energizado: false }
                } else {
                    ComandoEletrico::Nenhum
                };
                m.dispositivos.push(Dispositivo::Direcional {
                    id,
                    valvula,
                    portas,
                    comando,
                });
            }
            _ => {}
        }
    }

    // Segunda passagem: fins de curso criados antes dos cilindros na 1ª ordem.
    let cyl_ids: Vec<(i32, String)> = items
        .iter()
        .filter_map(|it| {
            let c = fluidsim_class_of(it)?;
            if c == "PAL1" || c == "PAL2" {
                Some((it.id, tag_upper(it)))
            } else {
                None
            }
        })
        .collect();
    for d in &mut m.dispositivos {
        if let Dispositivo::FimDeCurso { id, cilindro, .. } = d {
            if *cilindro == 0 {
                if let Some(item) = items.iter().find(|it| it.id as u32 == *id) {
                    *cilindro = match_cylinder(&cyl_ids, &tag_upper(item))
                        .or_else(|| cyl_ids.first().map(|(i, _)| *i as u32))
                        .unwrap_or(0);
                }
            }
        }
    }

    for &(widx, n1, n2) in &network.wire_nodes {
        if items.get(widx).map(|i| i.tipo == 4005).unwrap_or(false) {
            let (x1, y1) = network.all_points[n1];
            let (x2, y2) = network.all_points[n2];
            let mut on_seg: Vec<usize> = (0..network.num_nodes())
                .filter(|&u| {
                    let (px, py) = network.all_points[u];
                    crate::sim::graph::point_on_segment(px, py, x1, y1, x2, y2)
                })
                .collect();
            on_seg.sort_by_key(|&u| network.all_points[u]);
            for pair in on_seg.windows(2) {
                let u = pair[0] as u32;
                let v = pair[1] as u32;
                if u != v {
                    m.garante_no(u);
                    m.garante_no(v);
                    m.tubos.push((u, v));
                }
            }
        }
    }

    m
}

fn match_cylinder(cilindros: &[(i32, String)], sensor_tag: &str) -> Option<u32> {
    let st_clean = sensor_tag.trim().trim_start_matches('-').to_uppercase();
    let st = parse_tag(&st_clean)?;

    // 1. Vínculo por grupo numérico (ex: 1S1 ↔ 1A1, 2S1 ↔ 2A1)
    if !st.prefix.is_empty() {
        for (id, ctag) in cilindros {
            let ct_clean = ctag.trim().trim_start_matches('-').to_uppercase();
            if let Some(ct) = parse_tag(&ct_clean) {
                if ct.prefix == st.prefix {
                    return Some(*id as u32);
                }
            }
        }
    }

    // 2. Vínculo por letra do atuador (ex: cilindro A, sensor A0 ou A1)
    for (id, ctag) in cilindros {
        let ct_clean = ctag.trim().trim_start_matches('-').to_uppercase();
        if let Some(ct) = parse_tag(&ct_clean) {
            if ct.letter == st.letter {
                return Some(*id as u32);
            }
        }
    }

    // 3. Vínculo por índice quando prefixos forem vazios (ex: -A1 ↔ -S1, A1 ↔ S1)
    if st.prefix.is_empty() && !st.index.is_empty() {
        for (id, ctag) in cilindros {
            let ct_clean = ctag.trim().trim_start_matches('-').to_uppercase();
            if let Some(ct) = parse_tag(&ct_clean) {
                if ct.prefix.is_empty() && (ct.index == st.index || ct.index.starts_with(st.index))
                {
                    return Some(*id as u32);
                }
            }
        }
    }

    // 4. Se só existe um cilindro na folha, associa diretamente
    if cilindros.len() == 1 {
        return Some(cilindros[0].0 as u32);
    }

    None
}

/// Um passo do domínio pneumático após o cascade elétrico.
pub fn step_pneumatic(
    items: &[SchematicItem],
    sim: &mut SimState,
    network: &NetworkGraph,
    potentials: &[Potential],
    coils: &HashSet<String>,
    dt_ms: u64,
) {
    let prev = std::mem::take(&mut sim.pneumatic);
    let mut motor = rebuild_motor(items, network, &prev);

    for (idx, item) in items.iter().enumerate() {
        let Some(classe) = fluidsim_class_of(item) else {
            continue;
        };
        let pin_nodes = network
            .item_pin_nodes
            .get(&idx)
            .cloned()
            .unwrap_or_default();
        let mapa = pin_nodes_named(item, &pin_nodes);
        let id = item.id as u32;
        let s = sim.get(item.id);

        if matches!(
            classe.as_str(),
            "PVWE1" | "PVWE2" | "PVWE3" | "PVWE4" | "PVWE5"
        ) {
            let energizado = a1_a2_energizado(&mapa, potentials)
                || coil_tag_matches(coils, &tag_upper(item))
                || (!item.value.is_empty() && coil_tag_matches(coils, &item.value.to_uppercase()));
            motor.set_energizado(id, energizado);
        }

        if matches!(
            classe.as_str(),
            "PVWM1" | "PVWM2" | "PVWM3" | "PVWM6" | "PVWM7" | "PVWM8"
        ) {
            for d in &mut motor.dispositivos {
                if let Dispositivo::Direcional {
                    id: i,
                    valvula,
                    comando: ComandoEletrico::Nenhum,
                    ..
                } = d
                {
                    if *i == id {
                        valvula.estado = if s.manual_latched {
                            EstadoCarretel::Acionado
                        } else {
                            EstadoCarretel::Repouso
                        };
                    }
                }
            }
        }
    }

    let dt_s = (dt_ms as f32 / 1000.0).clamp(0.001, 0.25);
    motor.passo_dt(dt_s);

    // Espelha curso / pressostato / fim de curso / manômetro em ItemSimState.
    for (idx, item) in items.iter().enumerate() {
        let Some(classe) = fluidsim_class_of(item) else {
            continue;
        };
        let id = item.id as u32;
        let s = sim.get_mut(item.id);
        match classe.as_str() {
            "PAL1" | "PAL2" => {
                let pos = motor.pistao(id).map(|p| p.pos()).unwrap_or(0.0);
                s.active = pos >= 0.5;
                // Reusa timer_elapsed_ms como pos_pm (0..=1000) para o canvas interpolar a haste.
                s.timer_elapsed_ms = motor.pistao(id).map(|p| u64::from(p.pos_pm)).unwrap_or(0);
            }
            "152841" => {
                let pin_nodes = network
                    .item_pin_nodes
                    .get(&idx)
                    .cloned()
                    .unwrap_or_default();
                let mapa = pin_nodes_named(item, &pin_nodes);
                let no = mapa
                    .get("1")
                    .copied()
                    .or_else(|| mapa.values().next().copied())
                    .unwrap_or(0);
                let p_kpa = motor.pressao_kpa(no);
                s.active = p_kpa > 0;
                // Reusa timer_elapsed_ms como pressão em kPa (ex.: 600 = 0.60 MPa, 0 = 0.00 MPa).
                s.timer_elapsed_ms = u64::from(p_kpa);
            }
            "PSW1" | "PEW" => {
                s.active = motor.contato_fechado(id).unwrap_or(false);
            }
            "PSG1" => {
                s.active = motor.fim_de_curso_ativo(id).unwrap_or(false);
            }
            c if c.starts_with("PVW") => {
                let acionado = motor.dispositivos.iter().any(|d| {
                    matches!(
                        d,
                        Dispositivo::Direcional { id: i, valvula, .. }
                            if *i == id && valvula.estado == EstadoCarretel::Acionado
                    )
                });
                s.active = acionado;
            }
            "PVG2" | "PVG3" => {
                s.active = motor.dispositivos.iter().any(|d| {
                    matches!(
                        d,
                        Dispositivo::Temporizador { id: i, valvula, .. }
                            if *i == id && valvula.estado == EstadoCarretel::Acionado
                    )
                });
            }
            "PAR2" => {
                s.active = motor.motor_girando(id).unwrap_or(false);
            }
            _ => {}
        }
        // Fins de curso elétricos (LimitSwitch) associados ao cilindro por TAG.
        if let Some(r) = global_concept_dictionary().resolve_item(item) {
            if r.concept.kind == CanonicalKind::LimitSwitch && r.concept.fluidsim_class.is_none() {
                if let Some(cyl) = match_cylinder(
                    &items
                        .iter()
                        .filter_map(|it| {
                            let c = fluidsim_class_of(it)?;
                            if c == "PAL1" || c == "PAL2" {
                                Some((it.id, tag_upper(it)))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>(),
                    &tag_upper(item),
                ) {
                    let avancado = motor.curso_cilindro(cyl) == Some(CursoCilindro::Avancado);
                    // Convenção: tag com 'a' / avançado vs 'r' / recolhido no final do prefixo.
                    let tag = tag_upper(item);
                    let quer_avancado = !tag.contains("R0") && !tag.ends_with('R');
                    s.manual_latched = if quer_avancado { avancado } else { !avancado };
                    s.active = s.manual_latched;
                }
            }
        }
    }

    // Coloração de tubos: NoAr do nó de cada extremidade do fio.
    sim.air_by_wire.clear();
    for (widx, n1, n2) in &network.wire_nodes {
        let item = &items[*widx];
        let a = motor.no(*n1 as u32);
        let b = motor.no(*n2 as u32);
        let ar = match (a, b) {
            (NoAr::Suprimento, _) | (_, NoAr::Suprimento) => NoAr::Suprimento,
            (NoAr::Escape, _) | (_, NoAr::Escape) => NoAr::Escape,
            _ => NoAr::SemPressao,
        };
        sim.air_by_wire.insert(item.id, ar);
    }

    sim.pneumatic = motor;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pneu::contrato::NoAr;

    #[test]
    fn motor_manual_avanca_cilindro() {
        // Circuito mínimo montado direto no MotorAr (paridade com P6).
        let mut m = MotorAr::novo();
        for n in [10u32, 20, 30, 40, 50] {
            m.garante_no(n);
        }
        m.dispositivos.push(Dispositivo::Fonte { id: 1, no: 10 });
        m.dispositivos.push(Dispositivo::Escape { id: 2, no: 30 });
        m.dispositivos.push(Dispositivo::Escape { id: 3, no: 50 });
        m.dispositivos.push(Dispositivo::Direcional {
            id: 4,
            valvula: valvula_5_2(EstadoCarretel::Repouso, vec![Acionamento::Manual]),
            portas: vec![
                Porta::nova("1", 10),
                Porta::nova("2", 20),
                Porta::nova("3", 30),
                Porta::nova("4", 40),
                Porta::nova("5", 50),
            ],
            comando: ComandoEletrico::Nenhum,
        });
        m.dispositivos.push(Dispositivo::CilindroDuplo {
            id: 5,
            avanco: 40,
            retorno: 20,
            pistao: EstadoPistao::recolhido(),
        });
        m.passo();
        assert_eq!(m.curso_cilindro(5), Some(CursoCilindro::Recolhido));
        if let Dispositivo::Direcional { valvula, .. } = &mut m.dispositivos[3] {
            valvula.estado = EstadoCarretel::Acionado;
        }
        m.passo();
        assert_eq!(m.no(40), NoAr::Suprimento);
        assert_eq!(m.curso_cilindro(5), Some(CursoCilindro::Avancado));
    }

    #[test]
    fn test_parse_tag_and_coil_tag_matches_anti_crosstalk() {
        let mut coils = HashSet::new();
        coils.insert("1Y1".to_string());
        coils.insert("2Y1".to_string());

        // 1V1 deve casar apenas com 1Y1, NUNCA com 2Y1
        assert!(coil_tag_matches(&coils, "1V1"));
        assert!(coil_tag_matches(&coils, "1V"));
        assert!(coil_tag_matches(&coils, "2V1"));

        let mut coils_only_2 = HashSet::new();
        coils_only_2.insert("2Y1".to_string());
        // Com apenas 2Y1 energizado, 1V1 NÃO pode casar (sem cross-talk)
        assert!(!coil_tag_matches(&coils_only_2, "1V1"));
        assert!(!coil_tag_matches(&coils_only_2, "1V"));
        assert!(coil_tag_matches(&coils_only_2, "2V1"));

        // Notação DIN com traço (-V1 ↔ -Y1)
        let mut coils_traco = HashSet::new();
        coils_traco.insert("-Y1".to_string());
        assert!(coil_tag_matches(&coils_traco, "-V1"));
        assert!(coil_tag_matches(&coils_traco, "V1"));
        assert!(!coil_tag_matches(&coils_traco, "-V2"));
    }

    #[test]
    fn test_match_cylinder_anti_crosstalk() {
        let cilindros = vec![(101, "1A1".to_string()), (202, "2A1".to_string())];
        // Sensor do grupo 1 deve casar com cilindro 101, e grupo 2 com 202
        assert_eq!(match_cylinder(&cilindros, "1S1"), Some(101));
        assert_eq!(match_cylinder(&cilindros, "1S2"), Some(101));
        assert_eq!(match_cylinder(&cilindros, "2S1"), Some(202));
        assert_eq!(match_cylinder(&cilindros, "2S2"), Some(202));

        // Cilindros nomeados A e B
        let cilindros_ab = vec![(1, "A".to_string()), (2, "B".to_string())];
        assert_eq!(match_cylinder(&cilindros_ab, "A0"), Some(1));
        assert_eq!(match_cylinder(&cilindros_ab, "A1"), Some(1));
        assert_eq!(match_cylinder(&cilindros_ab, "B0"), Some(2));
        assert_eq!(match_cylinder(&cilindros_ab, "B1"), Some(2));
    }
}
