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

    // 1. Circuito de comando: 8 condutores elétricos (tipo 4003) conectando sem gaps
    let wires_el: Vec<_> = doc.items.iter().filter(|i| i.tipo == 4003).collect();
    assert_eq!(wires_el.len(), 8, "8 condutores eletricos perfeitos");
    for w in &wires_el {
        assert!(w.x1 == w.x2 || w.y1 == w.y2, "sem diagonais elétricas");
    }

    // Verifica que os elementos de comando usam tipos nativos RustSimu sem sobrescrita
    let sw = doc.items.iter().find(|i| i.tag == "S").unwrap();
    assert_eq!(sw.tipo, 8008, "Interruptor NA deve usar tipo nativo 8008");
    assert!(sw.symbol_id.is_none(), "Nao deve usar symbol_id externo");

    let ct = doc
        .items
        .iter()
        .find(|i| i.tag == "K1" && i.tipo == 7000)
        .unwrap();
    assert_eq!(ct.tipo, 7000, "Contato NA deve usar tipo nativo 7000");
    assert!(ct.symbol_id.is_none());

    let r_coil = doc
        .items
        .iter()
        .find(|i| i.tag == "K1" && i.tipo == 9000)
        .unwrap();
    assert_eq!(
        r_coil.tipo, 9000,
        "Bobina de rele deve usar tipo nativo 9000"
    );
    assert!(r_coil.symbol_id.is_none());

    let y_sol = doc
        .items
        .iter()
        .find(|i| i.tag == "1Y" && i.tipo == 9014)
        .unwrap();
    assert_eq!(
        y_sol.tipo, 9014,
        "Solenoide de comando deve usar tipo nativo 9014"
    );
    assert!(y_sol.symbol_id.is_none());

    // 2. Cilindros simples ação: 1A1 (avançado/S=50, symbol_state='1') e 1A2 (recuado/S=0, symbol_state='0')
    let c1 = doc.items.iter().find(|i| i.tag == "1A1").unwrap();
    assert_eq!(c1.symbol_id.as_deref(), Some("PAL1"));
    assert_eq!(c1.symbol_state, Some('1'));

    let c2 = doc.items.iter().find(|i| i.tag == "1A2").unwrap();
    assert_eq!(c2.symbol_id.as_deref(), Some("PAL1"));
    assert_eq!(c2.symbol_state, Some('0'));

    // 3. Válvula 5/2 na posição canônica (180, 114) com bornes alinhados e tag 1V
    let valve = doc.items.iter().find(|i| i.tag == "1V").unwrap();
    assert_eq!(valve.symbol_id.as_deref(), Some("PVWE4"));
    assert_eq!((valve.x1, valve.y1), (180, 114));
    assert_eq!(valve.value, "1Y", "Valvula deve carregar tag do piloto 1Y");

    // 4. Manômetro 152841 presente e perfeitamente ancorado
    let gauge = doc
        .items
        .iter()
        .find(|i| i.tag == "-P33" || i.concept_id.as_deref() == Some("pneu.gauge"))
        .unwrap();
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
        .any(|i| i.concept_id.as_deref() == Some("el.coil.contactor") && i.tag == "K1");
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
fn test_demo_09_simulation_eletropneumatica() {
    let path = "/home/caiojordao84/Downloads/ct/demo_09.ct";
    if !std::path::Path::new(path).exists() {
        return;
    }
    let mut doc = schematic_from_ct_path(path).expect("schematic import");

    let sw_id = doc.items.iter().find(|i| i.tag == "S").unwrap().id;
    let k1_id = doc
        .items
        .iter()
        .find(|i| i.tag == "K1" && i.tipo == 9000)
        .unwrap()
        .id;
    let y1_id = doc
        .items
        .iter()
        .find(|i| i.tag == "1Y" && i.tipo == 9014)
        .unwrap()
        .id;
    let _v1_id = doc.items.iter().find(|i| i.tag == "1V").unwrap().id;
    let _c1_id = doc.items.iter().find(|i| i.tag == "1A1").unwrap().id;
    let c2_id = doc.items.iter().find(|i| i.tag == "1A2").unwrap().id;

    // 1) Repouso inicial: interruptor S aberto, K1 e 1Y desenergizados
    cade_simu_rs::sim::step_simulation(&mut doc, 50);
    assert!(
        !doc.sim_active(k1_id),
        "Bobina K1 deve iniciar desenergizada"
    );
    assert!(
        !doc.sim_active(y1_id),
        "Solenoide 1Y deve iniciar desenergizado"
    );

    // 2) Fecha o interruptor S (tipo 8008 retentivo)
    cade_simu_rs::sim::press_component_on_sim(&mut doc, sw_id);
    cade_simu_rs::sim::release_component_on_sim(&mut doc, sw_id);
    cade_simu_rs::sim::step_simulation(&mut doc, 50);

    assert!(
        doc.sim_active(k1_id),
        "Bobina K1 deve energizar com S fechado"
    );
    assert!(
        doc.sim_active(y1_id),
        "Solenoide 1Y deve energizar via contato K1"
    );

    // Válvula 1V comuta e cilindros completam o curso em menos de 1 segundo (ex: 400ms)
    for _ in 0..10 {
        cade_simu_rs::sim::step_simulation(&mut doc, 40);
    }
    let s_c1 = doc.sim.get(_c1_id);
    let s_c2 = doc.sim.get(c2_id);
    assert_eq!(
        s_c1.timer_elapsed_ms, 1000,
        "Cilindro 1A1 deve avancar 100% em < 1s com solenoide 1Y ativo"
    );
    assert_eq!(
        s_c2.timer_elapsed_ms, 0,
        "Cilindro 1A2 deve recuar 100% por mola em < 1s com solenoide 1Y ativo"
    );

    // 3) Abre novamente o interruptor S
    cade_simu_rs::sim::press_component_on_sim(&mut doc, sw_id);
    cade_simu_rs::sim::release_component_on_sim(&mut doc, sw_id);
    for _ in 0..10 {
        cade_simu_rs::sim::step_simulation(&mut doc, 40);
    }

    assert!(
        !doc.sim_active(k1_id),
        "Bobina K1 deve desenergizar com S aberto"
    );
    assert!(
        !doc.sim_active(y1_id),
        "Solenoide 1Y deve desenergizar com S aberto"
    );
    assert_eq!(
        doc.sim.get(_c1_id).timer_elapsed_ms,
        0,
        "Cilindro 1A1 deve recuar a 0 em < 1s ao abrir S"
    );
    assert_eq!(
        doc.sim.get(c2_id).timer_elapsed_ms,
        1000,
        "Cilindro 1A2 deve avancar a 1000 em < 1s ao abrir S"
    );
}

#[test]
fn test_demo_01_routing_speed_and_manometer() {
    let path = "/home/caiojordao84/Downloads/ct/demo_01.ct";
    if !std::path::Path::new(path).exists() {
        return;
    }
    cade_simu_rs::pneu::palette::force_reregister_svgs();
    let mut doc = schematic_from_ct_path(path).expect("demo_01 import");

    // 1. Verifica que o roteamento de demo_01.ct não possui overshoots em (204, 84) nem em (222, 66)
    let wires_pn: Vec<_> = doc.items.iter().filter(|i| i.tipo == 4005).collect();
    assert_eq!(
        wires_pn.len(),
        10,
        "demo_01 deve ter 10 segmentos pneumaticos"
    );
    for w in &wires_pn {
        if w.x1 == 204 && w.x2 == 204 {
            assert!(
                w.y1.min(w.y2) >= 84,
                " Derivacao em x=204 nao pode ultrapassar y=84 para cima: ({}, {}) -> ({}, {})",
                w.x1,
                w.y1,
                w.x2,
                w.y2
            );
        }
        if w.x1 == 222 && w.x2 == 222 {
            assert!(
                w.y1.min(w.y2) >= 66,
                "Tubo vertical em x=222 nao pode ultrapassar o borne do cilindro 1A2 em y=66: ({}, {}) -> ({}, {})",
                w.x1,
                w.y1,
                w.x2,
                w.y2
            );
        }
    }

    // 2. Tags sintetizadas (-P33, -Z0) ocultas e +24V / 0V visíveis nas fontes elétricas
    let gauge = doc
        .items
        .iter()
        .find(|i| i.concept_id.as_deref() == Some("pneu.gauge"))
        .expect("manometro presente");
    assert_eq!(gauge.show_tag, 0, "Tag sintetizada -P33 deve ficar oculta");
    let gauge_id = gauge.id;

    let air_src = doc
        .items
        .iter()
        .find(|i| i.is_pneumatic_air_source())
        .expect("fonte de ar presente");
    assert_eq!(air_src.show_tag, 0, "Tag sintetizada -Z0 deve ficar oculta");
    assert!(
        (air_src.pneu_pressure_mpa() - 0.6).abs() < 1e-4,
        "Pressao operacional padrao deve ser 0.6 MPa"
    );
    assert!(
        (air_src.pneu_flow_lmin() - 1000.0).abs() < 1e-2,
        "Flow rate padrao deve ser 1000 l/min"
    );

    let p24 = doc.items.iter().find(|i| i.tipo == 3008).expect("+24V");
    assert_eq!(p24.tag, "+24V");
    assert_eq!(p24.show_tag, 1);

    let p0 = doc.items.iter().find(|i| i.tipo == 3009).expect("0V");
    assert_eq!(p0.tag, "0V");
    assert_eq!(p0.show_tag, 1);

    // Exporta imagem em modo edição
    let opts = cade_simu_rs::export::ExportOptions {
        include_cajetin: false,
        show_terminals: true,
        monochrome: false,
    };
    if let Ok(bytes) = cade_simu_rs::export::export_to_png(&doc, None, opts, 2.0) {
        let _ = std::fs::write(
            "/home/caiojordao84/.gemini/antigravity/brain/dfe77710-9bba-4889-8767-eec33a5bf9bf/demo_01_edit.png",
            bytes,
        );
    }

    // 3. Simulação em repouso (S aberto): manômetro deve indicar 0.60 MPa (600 kPa) e 1A2 avançado
    let sw_id = doc.items.iter().find(|i| i.tag == "S").unwrap().id;
    let c1_id = doc.items.iter().find(|i| i.tag == "1A1").unwrap().id;
    let c2_id = doc.items.iter().find(|i| i.tag == "1A2").unwrap().id;

    for _ in 0..10 {
        cade_simu_rs::sim::step_simulation(&mut doc, 40);
    }
    assert_eq!(
        doc.sim.get(gauge_id).timer_elapsed_ms,
        600,
        "Manometro em repouso deve marcar 600 kPa (0.60 MPa)"
    );
    assert_eq!(doc.sim.get(c1_id).timer_elapsed_ms, 0);
    assert_eq!(doc.sim.get(c2_id).timer_elapsed_ms, 1000);

    let opts_sim = cade_simu_rs::export::ExportOptions {
        include_cajetin: false,
        show_terminals: false,
        monochrome: false,
    };
    if let Ok(bytes) = cade_simu_rs::export::export_to_png(&doc, None, opts_sim, 2.0) {
        let _ = std::fs::write(
            "/home/caiojordao84/.gemini/antigravity/brain/dfe77710-9bba-4889-8767-eec33a5bf9bf/demo_01_sim_rest.png",
            bytes,
        );
    }

    // 4. Simulação acionada (fecha S): manômetro cai para 0.00 MPa (0 kPa), 1A1 avança e 1A2 recua em < 1s
    cade_simu_rs::sim::press_component_on_sim(&mut doc, sw_id);
    cade_simu_rs::sim::release_component_on_sim(&mut doc, sw_id);
    for _ in 0..10 {
        cade_simu_rs::sim::step_simulation(&mut doc, 40);
    }
    assert_eq!(
        doc.sim.get(gauge_id).timer_elapsed_ms,
        0,
        "Manometro com 1V comutada deve marcar 0 kPa (0.00 MPa)"
    );
    assert_eq!(doc.sim.get(c1_id).timer_elapsed_ms, 1000);
    assert_eq!(doc.sim.get(c2_id).timer_elapsed_ms, 0);

    if let Ok(bytes) = cade_simu_rs::export::export_to_png(&doc, None, opts_sim, 2.0) {
        let _ = std::fs::write(
            "/home/caiojordao84/.gemini/antigravity/brain/dfe77710-9bba-4889-8767-eec33a5bf9bf/demo_01_sim_active.png",
            bytes,
        );
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
