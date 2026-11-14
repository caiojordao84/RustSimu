//! P7.4 — Reconciliação de slugs QET `pneumatic/*` → `concept_id` FluidSIM/RustSimu.

/// Resolve um path/slug QET pneumático para `concept_id` canônico.
pub fn concept_id_from_qet_pneumatic(path_or_slug: &str) -> Option<&'static str> {
    let j = path_or_slug.to_ascii_lowercase();
    if !(j.contains("pneumatic") || j.contains("compressed_air") || j.contains("pneumatik")) {
        return None;
    }
    let has = |stems: &[&str]| stems.iter().any(|s| j.contains(s));

    if has(&["cylinder", "actuator", "zylinder", "cilindro", "verin"]) {
        return Some(
            if has(&["double", "2_way", "double_acting", "double-acting"]) {
                "pneu.cylinder.double"
            } else {
                "pneu.cylinder.single"
            },
        );
    }
    if has(&["5_2", "5-2", "5/2"]) {
        return Some("pneu.valve.5_2");
    }
    if has(&["3_2", "3-2", "3/2"]) {
        return Some("pneu.valve.3_2");
    }
    if has(&[
        "check",
        "non-return",
        "non_return",
        "retention",
        "retencao",
        "rueckschlag",
        "ruckschlag",
    ]) {
        return Some("pneu.valve.check");
    }
    if has(&["quick_exhaust", "schnell"]) {
        return Some("pneu.valve.quick_exhaust");
    }
    if has(&["compressor", "air_generation", "verdichter"]) {
        return Some("pneu.source.air");
    }
    if has(&[
        "service",
        "treatment",
        "filter",
        "lubricat",
        "regulator",
        "wartung",
        "frl",
    ]) {
        return Some("pneu.service.unit");
    }
    if has(&["pressure_relief", "relief", "safety_valve", "druckbegrenz"]) {
        return Some("pneu.valve.pressure_relief");
    }
    if has(&["vacuum", "vakuum", "suction", "saug"]) {
        return Some(if has(&["pump", "pumpe", "generator"]) {
            "pneu.vacuum.pump"
        } else {
            "pneu.vacuum.cup"
        });
    }
    if has(&["motor", "rotary", "schwenk"]) {
        return Some("pneu.motor");
    }
    if has(&["gauge", "manometer", "pressure_indicator"]) {
        return Some("pneu.gauge");
    }
    if has(&["throttle", "flow_control", "drossel"]) {
        return Some("pneu.valve.pressure_relief");
    }
    if has(&["and_valve", "two_pressure", "zweidruck"]) {
        return Some("pneu.logic.and");
    }
    if has(&["or_valve", "shuttle", "wechsel"]) {
        return Some("pneu.logic.or");
    }
    if has(&["timer", "zeit", "delay"]) {
        return Some("pneu.valve.timer_nc");
    }
    if has(&["tank", "accumulator", "reservoir"]) {
        return Some("pneu.source.air");
    }
    if has(&["pressure_controller", "controller", "setpoint", "sensor"]) {
        return Some("epneu.switch.pressure");
    }
    if has(&[
        "pipe",
        "connection",
        "distributor",
        "junction",
        "plug",
        "drain",
        "vent",
        "hose",
        "cable",
        "silencer",
        "muffler",
    ]) {
        return Some("pneu.valve.quick_exhaust"); // conduíte/acessório passivo didático
    }
    if has(&["valve", "ventil", "distributeur"]) {
        return Some("pneu.valve.3_2");
    }
    if has(&["source", "air_supply"]) {
        return Some("pneu.source.air");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dictionary::global_concept_dictionary;

    #[test]
    fn amostra_qet_pneumatic_resolve() {
        let samples = [
            (
                "pneumatic/5010_compressed_air/valves/5_2_monostable_valve.elmt",
                "pneu.valve.5_2",
            ),
            (
                "pneumatic/5010_compressed_air/501010_air_generation/air_compressor_in_general",
                "pneu.source.air",
            ),
            (
                "pneumatic/5010_compressed_air/actuators/double_acting_cylinder",
                "pneu.cylinder.double",
            ),
        ];
        let dict = global_concept_dictionary();
        for (slug, expect) in samples {
            let cid = concept_id_from_qet_pneumatic(slug).expect(slug);
            assert_eq!(cid, expect, "{slug}");
            assert!(dict.get_by_id(cid).is_some(), "conceito {cid}");
        }
    }

    #[test]
    fn catalogo_qet_pneumatic_taxa_de_match() {
        let s = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/symbols_qet/catalog.json"
        ));
        let catalog: crate::canonical::CanonicalCatalog =
            serde_json::from_str(s).expect("parse canonical catalog");
        let cat_paths: std::collections::HashMap<(String, String), String> = catalog
            .categories
            .iter()
            .map(|c| ((c.family.clone(), c.code.clone()), c.path.clone()))
            .collect();

        let mut total = 0usize;
        let mut matched = 0usize;
        for el in &catalog.elements {
            if el.family == "pneumatic" {
                total += 1;
                let cat_path = cat_paths
                    .get(&("pneumatic".to_string(), el.category.clone()))
                    .map(|s| s.as_str())
                    .unwrap_or("pneumatic");
                let slug = format!("{cat_path}/{}", el.slug);
                if concept_id_from_qet_pneumatic(&slug).is_some() {
                    matched += 1;
                }
            }
        }
        assert_eq!(total, 343, "catálogo QET pneumatic");
        assert!(
            matched >= 240,
            "esperava ≥240 matches de 343, got {matched}"
        );
    }
}
