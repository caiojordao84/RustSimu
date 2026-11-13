use cade_simu_rs::ct::schematic_from_ct_path;
use cade_simu_rs::model::GRID_MODULE_UNITS;
use cade_simu_rs::svg_symbol::{parse_svg_content, SvgShape};

#[test]
fn import_tp101_bornes_na_grade_e_tubos_nos_pins() {
    let path = "/mnt/Dados/Downloads/FluidSim Pneumatic/FluidSim Pneumatic/ct/TP101/TP101_17.ct";
    if !std::path::Path::new(path).exists() {
        return;
    }
    cade_simu_rs::pneu::palette::force_reregister_svgs();
    let doc = schematic_from_ct_path(path).expect("import");
    let g = GRID_MODULE_UNITS as i32;

    let mut pin_pts = std::collections::BTreeSet::new();
    for it in doc.items.iter().filter(|i| i.symbol_id.is_some()) {
        let classe = it.symbol_id_or_slug().unwrap();
        let (svg, terms) = cade_simu_rs::pneu::svg_v1_classe(classe, false)
            .unwrap_or_else(|| panic!("SVG {classe}"));
        assert!(svg.contains("data-standard=\"48px\""), "{classe}");
        assert!(
            terms.iter().all(|t| t.x % g == 0 && t.y % g == 0),
            "{classe} TerminalSimbolo off-grid {terms:?}"
        );
        let parsed = parse_svg_content(&svg).expect("parse");
        let s = parsed.scale_factor();
        for shape in &parsed.shapes {
            if let SvgShape::Circle {
                cx,
                cy,
                is_terminal: true,
                ..
            } = shape
            {
                let (lx, ly) = (cx * s, cy * s);
                // Após escala, deve cair em múltiplo do módulo (tol. 0.05).
                assert!(
                    (lx / g as f32).round().mul_add(g as f32, -lx).abs() < 0.05
                        && (ly / g as f32).round().mul_add(g as f32, -ly).abs() < 0.05,
                    "{classe} qet-terminal off-grid svg=({cx},{cy}) world=({lx},{ly})"
                );
                let (rx, ry) = cade_simu_rs::model::transform_offset_f32(lx, ly, it.orientation);
                let wx = (it.x1 as f32 + rx).round() as i32;
                let wy = (it.y1 as f32 + ry).round() as i32;
                assert_eq!(wx % g, 0, "pin x off-grid");
                assert_eq!(wy % g, 0, "pin y off-grid");
                pin_pts.insert((wx, wy));
            }
        }
    }
    assert!(!pin_pts.is_empty(), "sem pins");

    let wires: Vec<_> = doc.items.iter().filter(|i| i.is_wire()).collect();
    assert!(wires.len() > 19);
    for w in &wires {
        assert!(w.x1 == w.x2 || w.y1 == w.y2, "diagonal");
    }
    let hits = wires
        .iter()
        .filter(|w| pin_pts.contains(&(w.x1, w.y1)) || pin_pts.contains(&(w.x2, w.y2)))
        .count();
    assert!(
        hits >= 10,
        "poucos tubos nos pins: hits={hits} pins={}",
        pin_pts.len()
    );

    let textos = doc
        .items
        .iter()
        .filter(|i| i.tipo == 8 && !i.trailing_str.is_empty())
        .count();
    assert!(textos >= 5, "textos={textos}");

    // Potência / só-visual: R_SCHALT deve aparecer (não página em branco).
    assert!(
        doc.items.iter().any(|i| {
            i.symbol_id.as_deref() == Some("R_SCHALT")
                || i.concept_id
                    .as_deref()
                    .is_some_and(|c| c.contains("position"))
        }),
        "sensor R_SCHALT ausente — circuito incompleto"
    );
}

#[test]
fn diag_demo_09() {
    let path = "/home/caiojordao84/Downloads/ct/demo_09.ct";
    if !std::path::Path::new(path).exists() {
        return;
    }
    let bruto = std::fs::read(path).expect("read");
    let dec = cade_simu_rs::ct::decifra_ct_bytes(&bruto).expect("decifra");
    let pagina = ct_parser::Pagina::parse(&dec.corpo).expect("parse");
    let ad = ct_parser::adaptador::adaptar(&pagina);

    let doc = schematic_from_ct_path(path).expect("schematic import");

    println!("\n=== ITENS DE DEMO_09 ===");
    for it in &doc.items {
        println!(
            "id={} tag='{}' tipo={} sym={:?} concept={:?} at ({}, {}) to ({}, {})",
            it.id, it.tag, it.tipo, it.symbol_id, it.concept_id, it.x1, it.y1, it.x2, it.y2
        );
    }

    // 1. Circuito de comando: 8 condutores elétricos (tipo 4003) conectando sem gaps
    let wires_el: Vec<_> = doc.items.iter().filter(|i| i.tipo == 4003).collect();
    assert_eq!(wires_el.len(), 8, "8 condutores eletricos perfeitos");
    for w in &wires_el {
        assert!(w.x1 == w.x2 || w.y1 == w.y2, "sem diagonais elétricas");
    }

    // 2. Cilindros simples ação: 1A1 (avançado/S=50, symbol_state='1') e 1A2 (recuado/S=0, symbol_state='0')
    let c1 = doc.items.iter().find(|i| i.tag == "-A20").unwrap();
    assert_eq!(c1.symbol_id.as_deref(), Some("PAL1"));
    assert_eq!(c1.symbol_state, Some('1'));

    let c2 = doc.items.iter().find(|i| i.tag == "-A18").unwrap();
    assert_eq!(c2.symbol_id.as_deref(), Some("PAL1"));
    assert_eq!(c2.symbol_state, Some('0'));

    // 3. Válvula 5/2 na posição canônica (180, 114) com bornes alinhados
    let valve = doc.items.iter().find(|i| i.tag == "-V39").unwrap();
    assert_eq!(valve.symbol_id.as_deref(), Some("PVWE4"));
    assert_eq!((valve.x1, valve.y1), (180, 114));

    // 4. Manômetro 152841 presente e perfeitamente ancorado
    let gauge = doc.items.iter().find(|i| i.tag == "-P33").unwrap();
    println!("GAUGE ORIENTATION = {}", gauge.orientation);
    let (g_svg, g_terms) = cade_simu_rs::pneu::svg_v1_classe("152841", false).unwrap();
    println!("GAUGE SVG:\n{}", g_svg);
    println!("GAUGE TERMS: {:?}", g_terms);
    assert_eq!(gauge.symbol_id.as_deref(), Some("152841"));
    assert_eq!((gauge.x1, gauge.y1), (246, 114));

    // 5. Tubos pneumáticos (tipo 4005): 9 segmentos sem nenhum cruzamento fantasma
    let wires_pn: Vec<_> = doc.items.iter().filter(|i| i.tipo == 4005).collect();
    assert_eq!(wires_pn.len(), 9, "9 tubos pneumaticos perfeitos");
    for w in &wires_pn {
        assert!(w.x1 == w.x2 || w.y1 == w.y2, "sem diagonais pneumáticas");
    }
    let has_gauge = doc
        .items
        .iter()
        .any(|i| i.concept_id.as_deref() == Some("pneu.gauge"));
    assert!(has_gauge, "manometro 152841 deve ser importado");

    let single_cylinders = doc
        .items
        .iter()
        .filter(|i| i.concept_id.as_deref() == Some("pneu.cylinder.single"))
        .count();
    assert_eq!(
        single_cylinders, 2,
        "ambos os cilindros 1A1 e 1A2 sao de simples acao"
    );

    let has_k1 = doc
        .items
        .iter()
        .any(|i| i.concept_id.as_deref() == Some("el.coil.relay") && i.tag == "K1");
    assert!(has_k1, "rele K1 deve ter tag K1");

    let has_1y = doc
        .items
        .iter()
        .any(|i| i.concept_id.as_deref() == Some("el.coil.solenoid") && i.tag == "1Y");
    assert!(has_1y, "solenoide 1Y deve ter tag 1Y");

    // Fios elétricos no x de K1 e 1Y não devem ter desvios em U
    let k1_item = ad.esquema.itens.iter().find(|i| i.tag == "K1").unwrap();
    assert_eq!(
        k1_item.bornes[0].x, k1_item.bornes[1].x,
        "bornes A1 e A2 de K1 na vertical"
    );

    let y1_item = ad.esquema.itens.iter().find(|i| i.tag == "1Y").unwrap();
    assert_eq!(
        y1_item.bornes[0].x, y1_item.bornes[1].x,
        "bornes A1 e A2 de 1Y na vertical"
    );

    // Renderiza PNG de demonstração em alta resolução para inspeção visual
    let artifact_png = std::path::Path::new("/home/caiojordao84/.gemini/antigravity/brain/dfe77710-9bba-4889-8767-eec33a5bf9bf/demo_09_rustsimu.png");
    let opts = cade_simu_rs::export::ExportOptions {
        include_cajetin: false,
        show_terminals: false,
        monochrome: false,
    };
    if let Ok(bytes) = cade_simu_rs::export::export_to_png(&doc, None, opts, 2.0) {
        let _ = std::fs::write(artifact_png, bytes);
    }
}

#[test]
fn test_corpus_quality() {
    let dir = std::path::Path::new("/home/caiojordao84/Downloads/ct");
    if !dir.exists() {
        return;
    }
    cade_simu_rs::pneu::palette::force_reregister_svgs();
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|s| s.to_str()) == Some("ct") {
                files.push(p);
            }
        }
    }
    files.sort();

    println!("\n=== AUDITORIA DE QUALIDADE DE IMPORTACAO .CT ===");
    for path in &files {
        let fname = path.file_name().unwrap().to_string_lossy();
        match schematic_from_ct_path(path) {
            Ok(doc) => {
                let items: Vec<_> = doc
                    .items
                    .iter()
                    .filter(|i| !i.is_wire() && i.tipo != 8)
                    .collect();
                let wires: Vec<_> = doc.items.iter().filter(|i| i.is_wire()).collect();
                let mut bad_ori = 0;
                let mut diag_wires = 0;
                let mut offgrid_items = 0;
                let mut offgrid_wires = 0;
                let mut dup_wires = 0;
                let mut seen_wires = std::collections::BTreeSet::new();

                for it in &items {
                    if it.orientation > 3 {
                        bad_ori += 1;
                    }
                    if it.x1 % 6 != 0 || it.y1 % 6 != 0 {
                        offgrid_items += 1;
                    }
                }
                for w in &wires {
                    if w.x1 != w.x2 && w.y1 != w.y2 {
                        diag_wires += 1;
                    }
                    if w.x1 % 6 != 0 || w.y1 % 6 != 0 || w.x2 % 6 != 0 || w.y2 % 6 != 0 {
                        offgrid_wires += 1;
                    }
                    let p1 = (w.x1, w.y1);
                    let p2 = (w.x2, w.y2);
                    let key = if p1 <= p2 { (p1, p2) } else { (p2, p1) };
                    if !seen_wires.insert(key) {
                        dup_wires += 1;
                    }
                }
                println!("  {:<15} itens={:<3} fios={:<3} (dups={:<2} diag={:<2} off_grid_w={:<2} off_grid_i={:<2} bad_ori={:<2})",
                    fname, items.len(), wires.len(), dup_wires, diag_wires, offgrid_wires, offgrid_items, bad_ori);
                assert_eq!(bad_ori, 0, "{fname} tem orientacoes invalidas");
                assert_eq!(diag_wires, 0, "{fname} tem fios diagonais");
                assert_eq!(offgrid_items, 0, "{fname} tem itens fora da grade");
                assert_eq!(offgrid_wires, 0, "{fname} tem fios fora da grade");
                assert_eq!(dup_wires, 0, "{fname} tem fios duplicados");
            }
            Err(e) => {
                println!("  {:<15} FALHA: {}", fname, e);
            }
        }
    }
}
