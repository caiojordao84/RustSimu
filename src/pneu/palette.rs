//! Paleta v1 (23 classes) + registro de SVG no cache do canvas.

use crate::dictionary::global_concept_dictionary;
use crate::pneu::symbols::svg_v1_classe;
use crate::svg_symbol::{parse_svg_content, register_parsed_svg};
use std::sync::atomic::{AtomicBool, Ordering};

static REGISTROU: AtomicBool = AtomicBool::new(false);

/// Classes do conjunto v1 (`simular: true` no catálogo FluidSIM).
pub const V1_CLASSES: &[&str] = &[
    "PPE1", "PRV", "PVP1", "PAL1", "PAL2", "PVWM1", "PVWM3", "PVWM8", "PSG1", "PVWP1", "PVWP3",
    "PVWP4", "PVWE1", "PVWE3", "PVWE4", "PVS1", "PVS3", "PVS2", "E_Y001", "E_S011", "E_S031",
    "PSW1", "PEW",
];

/// Classes P7.2 com comportamento mínimo no motor + SVG na paleta.
pub const P72_CLASSES: &[&str] = &["PVG2", "PVG3", "PVS4", "PAV1", "PAR2"];

/// Pólos DC + instrumentos elétricos dos TP111 analógicos (P8).
pub const P8_ELETRICO: &[&str] = &[
    "pol1",
    "pol2",
    "Generator",
    "VMeter",
    "LoopController",
    "LoopControllerPID",
];

#[derive(Debug, Clone)]
pub struct EntradaPaleta {
    pub classe: &'static str,
    pub concept_id: String,
    pub name_pt: String,
    pub domain: String,
    pub pins: Vec<String>,
    pub tag_prefix: String,
}

/// Entradas da paleta Pneumática / Eletropneumática (v1 + P7.2).
pub fn entradas_v1() -> Vec<EntradaPaleta> {
    let dict = global_concept_dictionary();
    V1_CLASSES
        .iter()
        .copied()
        .chain(P72_CLASSES.iter().copied())
        .chain(P8_ELETRICO.iter().copied())
        .filter_map(|c| {
            let concept = dict.get_by_fluidsim_class(c)?;
            Some(EntradaPaleta {
                classe: c,
                concept_id: concept.concept_id.clone(),
                name_pt: concept.name_pt.clone(),
                domain: match concept.domain {
                    crate::dictionary::SimulationDomain::Pneumatic => "pneumatic".into(),
                    crate::dictionary::SimulationDomain::Electropneumatic
                    | crate::dictionary::SimulationDomain::ElectricCommand => {
                        "electropneumatic".into()
                    }
                    _ => "pneumatic".into(),
                },
                pins: concept.canonical_pins.clone(),
                tag_prefix: concept.default_tag_prefix.clone(),
            })
        })
        .collect()
}

/// Garante SVG idle/ativo no cache (`get_parsed_svg("PVWM8")` etc.).
///
/// Registra o conjunto v1/P7/P8 e ainda qualquer classe FluidSIM do dicionário
/// que tenha PIC/`svg_v1_classe` (circuitos de potência / só-visuais).
pub fn ensure_svg_registrados() {
    if REGISTROU.swap(true, Ordering::SeqCst) {
        return;
    }
    let mut classes: Vec<String> = V1_CLASSES
        .iter()
        .chain(P72_CLASSES.iter())
        .chain(P8_ELETRICO.iter())
        .map(|s| (*s).to_string())
        .collect();
    for c in ct_parser::dicionario::catalogo() {
        if !classes.iter().any(|x| x == c.classe) {
            classes.push(c.classe.to_string());
        }
    }
    for classe in &classes {
        for acionado in [false, true] {
            let Some((svg, _)) = svg_v1_classe(classe, acionado) else {
                continue;
            };
            if let Ok(parsed) = parse_svg_content(&svg) {
                let key = if acionado {
                    format!("{classe}_active")
                } else {
                    classe.clone()
                };
                register_parsed_svg(&key, parsed);
            }
        }
    }
}

/// Força re-registro (útil após hot-reload de símbolos em testes).
pub fn force_reregister_svgs() {
    REGISTROU.store(false, Ordering::SeqCst);
    ensure_svg_registrados();
}
