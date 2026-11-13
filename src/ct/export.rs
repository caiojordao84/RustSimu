//! Exportação `SchematicDocument` → bytes `.ct` FluidSIM (folha única).
//!
//! Formato: cabeçalho Str2 + linha `COMPRESS` + LZ77 (XOR `0x9a`).
//! Corpo didático mínimo: PREVIEW vazio + objetos + conexões + `ENDCT`.

use crate::ct::{enc_str2_line, lz77};
use crate::dictionary::global_concept_dictionary;
use crate::model::SchematicDocument;
use crate::pneu::escala::px_para_unidades;
use crate::pneu::pic::UN_POR_GRADE;
use crate::sim::pneumatic::fluidsim_class_of;

/// Classe FluidSIM / tipo CT a gravar no corpo.
fn tipo_ct_de_classe(classe: &str) -> &str {
    match classe {
        "PVWM8" | "PVWP3" | "PVWP4" | "PVWE3" | "PVWE4" => "WV_5",
        "PVWM1" | "PVWM3" | "PVWP1" | "PVWE1" => "WV_3",
        "PAL2" => "CylDGPPAB",
        "PAL1" => "CylSGPPAB",
        other => other,
    }
}

fn world_para_unidades(world: i32) -> i64 {
    let px = f64::from(world) * 8.0; // 6 world ≡ 48 px
    px_para_unidades(px).round() as i64
}

fn bbox_centrado(cx: i64, cy: i64, half: i64) -> [i64; 4] {
    [cx - half, cy - half, cx + half, cy + half]
}

fn emit_objeto(out: &mut String, tipo: &str, id: u32, bbox: [i64; 4], angulo: i64) {
    // `<Tipo> O<id> x1 y1 x2 y2 flag ang cor`
    out.push_str(&format!(
        "{tipo} O{id} {} {} {} {} 0 {angulo} 0\n",
        bbox[0], bbox[1], bbox[2], bbox[3]
    ));
    out.push_str(" S description_FEST B T\n");
    out.push_str(&format!(" S description S {tipo}\n"));
}

fn emit_conexao(out: &mut String, id: u32, x: i64, y: i64, eletrico: bool) {
    let half = UN_POR_GRADE as i64 / 4;
    let tipo = if eletrico {
        "EConnection"
    } else {
        "PConnection"
    };
    let bbox = bbox_centrado(x, y, half);
    emit_objeto(out, tipo, id, bbox, 0);
    out.push_str(" S ex_type_FEST B T\n");
    out.push_str(" S ex_type S EX0\n");
}

/// Monta o corpo textual descomprimido (PREVIEW + objetos + CON + ENDCT).
pub fn corpo_ct_from_doc(doc: &SchematicDocument) -> Vec<u8> {
    let half = UN_POR_GRADE as i64 / 2;
    let mut corpo = String::new();

    // PREVIEW 0 + DWORD rodapé + ENDPREVIEW
    corpo.push_str("PREVIEW 0\n");
    corpo.push_str(&String::from_utf8_lossy(&[0, 0, 0, 0]));
    corpo.push_str("ENDPREVIEW\n");

    let mut next_id = 1u32;
    let mut wire_pairs: Vec<(u32, u32)> = Vec::new();
    let mut tem_eletrico = false;

    for item in &doc.items {
        if item.is_wire() {
            continue;
        }
        let Some(classe) = fluidsim_class_of(item) else {
            // Itens elétricos nativos (3008/3009) sem fluidsim_class no symbol.
            if matches!(item.tipo, 3008 | 3009) {
                let classe = if item.tipo == 3008 { "pol2" } else { "pol1" };
                let tipo = tipo_ct_de_classe(classe);
                let cx = world_para_unidades(item.x1);
                let cy = world_para_unidades(item.y1);
                emit_objeto(&mut corpo, tipo, next_id, bbox_centrado(cx, cy, half), 0);
                next_id += 1;
                tem_eletrico = true;
            }
            continue;
        };
        if matches!(
            classe.as_str(),
            "pol1"
                | "pol2"
                | "Generator"
                | "VMeter"
                | "LoopController"
                | "LoopControllerPID"
                | "E_Y001"
                | "E_S011"
                | "E_S031"
        ) {
            tem_eletrico = true;
        }
        let tipo = tipo_ct_de_classe(&classe);
        let cx = world_para_unidades(item.x1);
        let cy = world_para_unidades(item.y1);
        let ang = i64::from(item.orientation) * 45 * 1024; // aproximação didática
        emit_objeto(&mut corpo, tipo, next_id, bbox_centrado(cx, cy, half), ang);
        next_id += 1;
    }

    for item in &doc.items {
        if !item.is_wire() {
            continue;
        }
        let x1 = world_para_unidades(item.x1);
        let y1 = world_para_unidades(item.y1);
        let x2 = world_para_unidades(item.x2);
        let y2 = world_para_unidades(item.y2);
        let id_a = next_id;
        emit_conexao(&mut corpo, id_a, x1, y1, tem_eletrico || item.tipo == 4003);
        next_id += 1;
        let id_b = next_id;
        emit_conexao(&mut corpo, id_b, x2, y2, tem_eletrico || item.tipo == 4003);
        next_id += 1;
        wire_pairs.push((id_a, id_b));
    }

    for (a, b) in wire_pairs {
        corpo.push_str(&format!("CON PLine {a} {b}\n"));
        corpo.push_str(" S COLOR I 0\n");
        corpo.push_str(" S Style I 2\n");
    }

    corpo.push_str("ENDCT\n");
    corpo.into_bytes()
}

fn cabecalho_padrao(doc: &SchematicDocument, tipos_obj: &[String]) -> Vec<String> {
    let titulo = if doc.title_block.title.trim().is_empty() {
        "RustSimu Schaltkreis".to_string()
    } else {
        doc.title_block.title.clone()
    };
    let mut lines = vec![
        titulo,
        "%%".into(),
        "POS 0300 0501 1.000000 1".into(),
        "DISPLAY 0 1 1".into(),
    ];
    for t in tipos_obj {
        lines.push(format!("OBJ {t}"));
    }
    lines.push("PAPER_SIZE 2 0 210 297 1".into());
    lines.push("EXT 0 0 300000 400000".into());
    lines.push("CREATOR RUSTSIMU_CT".into());
    lines
}

fn tipos_obj_from_doc(doc: &SchematicDocument) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    for item in &doc.items {
        if item.is_wire() {
            continue;
        }
        if let Some(classe) = fluidsim_class_of(item) {
            seen.insert(tipo_ct_de_classe(&classe).to_string());
        } else if item.tipo == 3008 {
            seen.insert("pol2".into());
        } else if item.tipo == 3009 {
            seen.insert("pol1".into());
        }
    }
    if doc.items.iter().any(|i| i.is_wire()) {
        seen.insert("PConnection".into());
    }
    seen.into_iter().collect()
}

/// Serializa a folha para bytes `.ct` (Str2 + COMPRESS).
pub fn export_doc_to_ct(doc: &SchematicDocument) -> Result<Vec<u8>, String> {
    let _ = global_concept_dictionary(); // garante índices FluidSIM
    let tipos = tipos_obj_from_doc(doc);
    let cab = cabecalho_padrao(doc, &tipos);
    let corpo = corpo_ct_from_doc(doc);
    let mut out = Vec::new();
    for line in &cab {
        out.extend_from_slice(&enc_str2_line(line.as_bytes()));
        out.push(b'\n');
    }
    out.extend_from_slice(&enc_str2_line(b"COMPRESS"));
    out.push(b'\n');
    out.extend_from_slice(&lz77::codifica_bloco_compress(&corpo)?);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ct::{decifra_ct_bytes, schematic_from_ct_bytes};
    use crate::model::SchematicItem;

    #[test]
    fn export_ct_roundtrip_minimo() {
        let mut doc = SchematicDocument::default();
        doc.title_block.title = "Teste CT".into();
        let mut a = SchematicItem::new_wire(1, 20000, 60, 60, 60, 60);
        a.symbol_id = Some("PPE1".into());
        a.qet_type = Some("PPE1".into());
        a.concept_id = Some("pneu.source.air".into());
        a.pins[0] = "1".into();
        doc.items.push(a);
        let mut b = SchematicItem::new_wire(2, 20000, 120, 60, 120, 60);
        b.symbol_id = Some("PAL2".into());
        b.qet_type = Some("PAL2".into());
        b.concept_id = Some("pneu.cylinder.double".into());
        b.pins[0] = "1".into();
        b.pins[1] = "2".into();
        doc.items.push(b);
        doc.items
            .push(SchematicItem::new_wire(3, 4000, 60, 60, 120, 60));

        let bytes = export_doc_to_ct(&doc).expect("export");
        let dec = decifra_ct_bytes(&bytes).expect("decifra");
        assert!(String::from_utf8_lossy(&dec.corpo).contains("ENDCT"));
        let loaded = schematic_from_ct_bytes(&bytes).expect("reimport");
        assert!(
            loaded
                .items
                .iter()
                .any(|i| i.symbol_id.as_deref() == Some("PPE1")),
            "PPE1 ausente após round-trip"
        );
        assert!(
            loaded
                .items
                .iter()
                .any(|i| i.symbol_id.as_deref() == Some("PAL2")
                    || i.symbol_id.as_deref() == Some("CylDGPPAB")),
            "cilindro ausente: {:?}",
            loaded
                .items
                .iter()
                .filter_map(|i| i.symbol_id.clone())
                .collect::<Vec<_>>()
        );
    }
}
