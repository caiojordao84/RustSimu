//! `.ct` binário → [`SchematicDocument`] no padrão CADe/QET do canvas.
//!
//! O adaptador do `ct-parser` entrega coords em px (módulo 48). Aqui convertemos
//! para unidades de grade (módulo 6), registramos o SVG `cade2svg` e montamos
//! `SchematicItem` como o importador QET (`tipo=20000` + `symbol_id` + pins).
//! Textos do FluidSIM viram `tipo == 8` (ferramenta de texto).

use crate::ct::{decifra_ct_bytes, ErroCt};
use crate::model::{SchematicDocument, SchematicItem, GRID_MODULE_UNITS};
use crate::pneu::symbols::px_adaptador_para_world;
use ct_parser::adaptador::adaptar;
use ct_parser::Pagina;

/// Importa bytes de um `.ct` FluidSIM para o modelo do RustSimu.
pub fn schematic_from_ct_bytes(bruto: &[u8]) -> Result<SchematicDocument, String> {
    let dec = decifra_ct_bytes(bruto).map_err(|e| e.to_string())?;
    let pagina = Pagina::parse(&dec.corpo).map_err(|e| format!("parse corpo .ct: {e}"))?;
    let ad = adaptar(&pagina);
    crate::pneu::palette::ensure_svg_registrados();

    let mut doc = SchematicDocument::default();
    doc.settings.format_orientation = 2; // Paisagem (horizontal A4)
    doc.settings.custom_width_mm = 297;
    doc.settings.custom_height_mm = 210;
    let titulo = dec
        .cabecalho
        .first()
        .cloned()
        .unwrap_or_else(|| "FluidSIM".into());
    doc.title_block.title = titulo;
    doc.title_block.entity = "FluidSIM Pneumatic Import".to_string();

    let concept_dict = crate::dictionary::global_concept_dictionary();
    let rede = pagina.rede();

    let mut terminal_remap: std::collections::HashMap<(i32, i32), (i32, i32)> =
        std::collections::HashMap::new();

    for item in &ad.esquema.itens {
        let fs_class = ct_parser::dicionario::catalogo()
            .iter()
            .find(|c| c.concept_id == item.concept_id)
            .map(|c| c.classe.to_string())
            .unwrap_or_else(|| item.concept_id.clone());

        // Pins canônicos do conceito (ordem ISO), como no import QET.
        let mut pins: [String; 9] = Default::default();
        if let Some(concept) = concept_dict.get_by_fluidsim_class(&fs_class) {
            for (i, p) in concept.canonical_pins.iter().take(9).enumerate() {
                pins[i] = p.clone();
            }
        } else {
            for (i, b) in item.bornes.iter().take(9).enumerate() {
                pins[i] = b.nome.clone();
            }
        }

        let x = px_adaptador_para_world(item.x);
        let y = px_adaptador_para_world(item.y);

        // pol1/pol2 → tipos nativos CC para o solver injetar DcNeg/DcPos.
        // Componentes de comando elétrico recebem seus tipos nativos RustSimu (8008, 7000, 9000, 9014).
        // Demais FluidSIM: tipo 20000 + symbol_id (padrão QET externo).
        let concept = concept_dict.get_by_fluidsim_class(&fs_class);
        let tipo = match fs_class.as_str() {
            "pol1" => 3009, // DcNeg nativo
            "pol2" => 3008, // DcPos nativo
            _ => concept.and_then(|c| c.cade_tipo).unwrap_or(20000),
        };

        let is_native_electrical = matches!(
            tipo,
            7000..=7009 | 8000..=8011 | 9000..=9014 | 3008 | 3009
        );

        let has_explicit_label = pagina
            .objetos
            .get(&item.id)
            .map(|o| {
                o.props.iter().any(|p| {
                    (p.nome == "label" || p.nome == "ID" || p.nome == "identification")
                        && !p.valor.trim().is_empty()
                })
            })
            .unwrap_or(false);

        let mut si = SchematicItem::new_wire(item.id as i32, tipo, x, y, x, y);
        si.tag = item.tag.clone();
        si.show_tag = if has_explicit_label && !si.tag.is_empty() {
            1
        } else {
            0
        };
        si.pins = pins;
        si.pin_visibility = [1; 9];
        si.orientation = item.orientacao.rem_euclid(8);

        if tipo == 3008 {
            if !has_explicit_label {
                si.tag = "+24V".to_string();
                si.show_tag = 1;
            }
            si.pin_visibility = [0; 9];
        } else if tipo == 3009 {
            if !has_explicit_label {
                si.tag = "0V".to_string();
                si.show_tag = 1;
            }
            si.pin_visibility = [0; 9];
        }

        // Fonte de ar comprimido (PPE1 / PRV / PVP1 / PPE2): extrai pressão operacional (P_LIM em bar → MPa)
        // e vazão máxima (FLOW em l/min) do objeto .ct (padrão FluidSIM: 0.6 MPa e 1000 l/min).
        if matches!(fs_class.as_str(), "PPE1" | "PRV" | "PVP1" | "PPE2") {
            let mut p_bar = 6.0f32;
            let mut flow_lpm = 1000.0f32;
            if let Some(obj) = pagina.objetos.get(&item.id) {
                if let Some(v) = obj.prop("P_LIM").and_then(|s| s.trim().parse::<f32>().ok()) {
                    p_bar = v.clamp(0.0, 20.0);
                }
                if let Some(v) = obj.prop("FLOW").and_then(|s| s.trim().parse::<f32>().ok()) {
                    flow_lpm = v.clamp(0.0, 5000.0);
                }
            }
            let p_mpa = (p_bar / 10.0).clamp(0.0, 2.0);
            si.pins[1] = "MPa".to_string();
            si.pins[2] = format_pneu_num(p_mpa);
            si.pins[3] = "l/min".to_string();
            si.pins[4] = format_pneu_num(flow_lpm);
            si.show_value = 0;
        }

        // Se for componente de comando elétrico canônico, usa o catálogo RustSimu (sem sobrescrever por SVG bruto)
        if is_native_electrical {
            si.qet_type = None;
            si.symbol_id = None;

            // Mapeia bornes do macro FluidSIM para os pinos canônicos da Lei Suprema de Tamanhos (módulo 12)
            if let Some(sym) = crate::symbols::find_symbol_ref(tipo) {
                for (idx, _pin_label, (wx, wy)) in sym.world_pins(&si) {
                    if let Some(b) = item.bornes.get(idx) {
                        let old_pos = (px_adaptador_para_world(b.x), px_adaptador_para_world(b.y));
                        if old_pos != (wx, wy) {
                            terminal_remap.insert(old_pos, (wx, wy));
                        }
                    }
                }
            }
        } else {
            si.qet_type = Some(fs_class.clone());
            si.symbol_id = Some(fs_class.clone());

            // Descobre tag por texto próximo ou conexão de comando
            // 1. Piloto elétrico associado (ex: UEConnection com label "1Y" na válvula 5/2)
            for a in rede.arestas.iter().filter(|a| a.componente == item.id) {
                if let Some(conn_obj) = pagina.objetos.get(&a.conn) {
                    if let Some(p) = conn_obj.props.iter().find(|p| p.nome == "label") {
                        if !p.valor.trim().is_empty() {
                            si.value = p.valor.trim().to_string();
                        }
                    }
                }
            }

            // 2. Texto de designação próximo (ex: "1A1", "1A2", "1V")
            if si.tag.is_empty()
                || si.tag.starts_with("-V")
                || si.tag.starts_with("-A")
                || si.tag.starts_with("-P")
            {
                let mut best_text: Option<String> = None;
                let mut min_dist = 50.0f32;
                for t in &ad.textos {
                    let tx = px_adaptador_para_world(t.x) as f32;
                    let ty = px_adaptador_para_world(t.y) as f32;
                    let dx = tx - x as f32;
                    let dy = ty - y as f32;
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist < min_dist && !t.conteudo.trim().is_empty() {
                        min_dist = dist;
                        best_text = Some(t.conteudo.trim().to_string());
                    }
                }
                if let Some(t_tag) = best_text {
                    si.tag = t_tag;
                    // Esconde a tag automática para usar o texto renderizado na posição exata do FluidSIM
                    si.show_tag = 0;
                }
            }
        }

        si.symbol_state = Some(if item.inicial_acionado { '1' } else { '0' });
        si.concept_id = concept
            .map(|c| c.concept_id.clone())
            .or_else(|| Some(item.concept_id.clone()));
        doc.items.push(si);
    }

    // Condutores → cabos elétricos (tipo 4003) vs tubos pneumáticos (tipo 4005).
    let mut raw_wires_el = Vec::new();
    let mut raw_wires_pn = Vec::new();
    for c in &ad.esquema.condutores {
        let raw_p1 = (px_adaptador_para_world(c.x1), px_adaptador_para_world(c.y1));
        let raw_p2 = (px_adaptador_para_world(c.x2), px_adaptador_para_world(c.y2));

        let (x1, y1) = terminal_remap.get(&raw_p1).copied().unwrap_or(raw_p1);
        let (x2, y2) = terminal_remap.get(&raw_p2).copied().unwrap_or(raw_p2);

        let mut segs = Vec::new();
        // Garante eixos H/V após snap de grade (evita diagonais por arredondamento).
        if x1 != x2 && y1 != y2 {
            let mid = (x2, y1);
            segs.push(((x1, y1), mid));
            segs.push((mid, (x2, y2)));
        } else if (x1, y1) != (x2, y2) {
            segs.push(((x1, y1), (x2, y2)));
        }
        if c.eletrico {
            raw_wires_el.extend(segs);
        } else {
            raw_wires_pn.extend(segs);
        }
    }

    let clean_wires_el = simplify_wire_segments(raw_wires_el);
    let clean_wires_pn = simplify_wire_segments(raw_wires_pn);
    let mut next_id = doc.items.iter().map(|i| i.id).max().unwrap_or(0) + 1;
    for ((x1, y1), (x2, y2)) in clean_wires_el {
        doc.items
            .push(SchematicItem::new_wire(next_id, 4003, x1, y1, x2, y2));
        next_id += 1;
    }
    for ((x1, y1), (x2, y2)) in clean_wires_pn {
        doc.items
            .push(SchematicItem::new_wire(next_id, 4005, x1, y1, x2, y2));
        next_id += 1;
    }

    // Textos FluidSIM → ferramenta de texto (tipo 8), como o import QET.
    for t in &ad.textos {
        let tx = px_adaptador_para_world(t.x);
        let ty = px_adaptador_para_world(t.y);
        let mut text_item = SchematicItem::new_graphic(next_id, 8, tx, ty, tx, ty);
        next_id += 1;
        text_item.trailing_str = t.conteudo.clone();
        // altura_px (módulo 48) → mundo (módulo 6): 48 px → 6 u.
        let world_h = (t.altura_px as f32) * GRID_MODULE_UNITS / 48.0;
        text_item.set_text_world_height(world_h.clamp(2.0, 16.0));
        text_item.sync_text_bounds();
        doc.items.push(text_item);
    }

    // Ordenação canônica determinística do canvas (Lei Suprema do Layout):
    // 1. Elementos principais / aparatos (relés, válvulas, cilindros, chaves)
    // 2. Objetos de apoio (fontes CC/ar comprimido, caixas de texto)
    // 3. Conexões conectando-se aos bornes (cabos e tubos Manhattan a 90º)
    doc.items.sort_by_key(|item| {
        if item.is_wire() {
            3
        } else if item.tipo == 8
            || item.tipo == 3008
            || item.tipo == 3009
            || item.symbol_id.as_deref() == Some("PPE1")
        {
            2
        } else {
            1
        }
    });

    Ok(doc)
}

/// Simplifica condutores Manhattan:
/// 1. Separa linhas horizontais e verticais.
/// 2. Mescla segmentos colineares adjacentes ou sobrepostos em um único traço contínuo.
/// 3. Remove duplicatas exatas e segmentos degenerados de comprimento zero.
pub fn simplify_wire_segments(raw: Vec<((i32, i32), (i32, i32))>) -> Vec<((i32, i32), (i32, i32))> {
    use std::collections::BTreeMap;

    let mut h_lines: BTreeMap<i32, Vec<(i32, i32)>> = BTreeMap::new();
    let mut v_lines: BTreeMap<i32, Vec<(i32, i32)>> = BTreeMap::new();
    let mut others = Vec::new();

    for ((x1, y1), (x2, y2)) in raw {
        if (x1, y1) == (x2, y2) {
            continue;
        }
        if y1 == y2 {
            let min_x = x1.min(x2);
            let max_x = x1.max(x2);
            h_lines.entry(y1).or_default().push((min_x, max_x));
        } else if x1 == x2 {
            let min_y = y1.min(y2);
            let max_y = y1.max(y2);
            v_lines.entry(x1).or_default().push((min_y, max_y));
        } else {
            let p1 = (x1, y1);
            let p2 = (x2, y2);
            if p1 <= p2 {
                others.push((p1, p2));
            } else {
                others.push((p2, p1));
            }
        }
    }

    let mut out = Vec::new();

    for (y, mut intervals) in h_lines {
        intervals.sort_unstable();
        let mut cur: Option<(i32, i32)> = None;
        for (x0, x1) in intervals {
            match cur {
                None => cur = Some((x0, x1)),
                Some((cx0, cx1)) => {
                    if x0 <= cx1 {
                        cur = Some((cx0, cx1.max(x1)));
                    } else {
                        out.push(((cx0, y), (cx1, y)));
                        cur = Some((x0, x1));
                    }
                }
            }
        }
        if let Some((cx0, cx1)) = cur {
            out.push(((cx0, y), (cx1, y)));
        }
    }

    for (x, mut intervals) in v_lines {
        intervals.sort_unstable();
        let mut cur: Option<(i32, i32)> = None;
        for (y0, y1) in intervals {
            match cur {
                None => cur = Some((y0, y1)),
                Some((cy0, cy1)) => {
                    if y0 <= cy1 {
                        cur = Some((cy0, cy1.max(y1)));
                    } else {
                        out.push(((x, cy0), (x, cy1)));
                        cur = Some((y0, y1));
                    }
                }
            }
        }
        if let Some((cy0, cy1)) = cur {
            out.push(((x, cy0), (x, cy1)));
        }
    }

    out.extend(others);
    out.sort_unstable();
    out.dedup();
    out
}

/// Lê um arquivo `.ct` do disco.
pub fn schematic_from_ct_path(
    path: impl AsRef<std::path::Path>,
) -> Result<SchematicDocument, String> {
    let bruto = std::fs::read(path.as_ref()).map_err(|e| e.to_string())?;
    schematic_from_ct_bytes(&bruto)
}

impl From<ErroCt> for String {
    fn from(e: ErroCt) -> Self {
        e.to_string()
    }
}

fn format_pneu_num(v: f32) -> String {
    let s = format!("{v:.2}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" {
        "0".to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn importa_tp101_com_escala_de_grade() {
        let path =
            "/mnt/Dados/Downloads/FluidSim Pneumatic/FluidSim Pneumatic/ct/TP101/TP101_17.ct";
        if !std::path::Path::new(path).exists() {
            return;
        }
        let doc = schematic_from_ct_path(path).expect("import TP101_17");
        assert!(
            doc.items
                .iter()
                .any(|i| i.tipo == 20000 && i.symbol_id.is_some()),
            "deve haver símbolos pneumáticos"
        );
        assert!(
            doc.items.iter().any(|i| i.is_wire()),
            "deve haver tubos/condutores"
        );
        assert!(
            doc.items
                .iter()
                .any(|i| i.tipo == 8 && !i.trailing_str.is_empty()),
            "deve importar textos tipo 8"
        );
        // Posições em unidades de grade (módulo 6), não em px (módulo 48).
        for it in doc.items.iter().filter(|i| i.tipo == 20000) {
            assert_eq!(it.x1 % 3, 0, "x1 deve alinhar à sub-grade: {}", it.x1);
            assert_eq!(it.y1 % 3, 0, "y1 deve alinhar à sub-grade: {}", it.y1);
            assert!(it.x1.abs() < 5000, "x1 fora da folha razoável: {}", it.x1);
            assert!(it.y1.abs() < 5000, "y1 fora da folha razoável: {}", it.y1);
            let classe = it.symbol_id_or_slug().unwrap();
            let (svg, terms) = crate::pneu::svg_v1_classe(classe, false)
                .unwrap_or_else(|| panic!("SVG ausente para {classe}"));
            assert!(
                svg.contains("data-generator=\"cade2svg\""),
                "{classe} sem cade2svg"
            );
            assert!(
                svg.contains("data-standard=\"48px\""),
                "{classe} sem data-standard=48px"
            );
            let g = GRID_MODULE_UNITS as i32;
            assert!(
                terms.iter().all(|t| t.x % g == 0 && t.y % g == 0),
                "{classe} bornes fora do módulo {g}: {terms:?}"
            );
            let parsed = crate::svg_symbol::parse_svg_content(&svg).expect("parse svg");
            let s = parsed.scale_factor();
            let w = parsed.view_box.2 * s;
            assert!(w < 160.0, "{classe} largo demais no mundo: {w}");
        }
        // Tubos só H/V.
        for w in doc.items.iter().filter(|i| i.is_wire()) {
            assert!(
                w.x1 == w.x2 || w.y1 == w.y2,
                "tubo diagonal ({},{})→({},{})",
                w.x1,
                w.y1,
                w.x2,
                w.y2
            );
        }
    }
}
