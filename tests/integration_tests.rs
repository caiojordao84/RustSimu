use iced::Color;

use cade_simu_rs::app::*;
use cade_simu_rs::canonical::*;
use cade_simu_rs::canvas_view::*;
use cade_simu_rs::export::*;
use cade_simu_rs::io::*;
use cade_simu_rs::model::*;
use cade_simu_rs::sim::*;
use cade_simu_rs::symbols::*;
use cade_simu_rs::theme::{self, *};
use cade_simu_rs::xref::TerminalStripKind;

#[test]
fn test_cad_roundtrip_dol_starter() {
    let original = build_dol_starter_preset();
    let cad_text = serialize_cad_str(&original);
    assert!(cad_text.starts_with("CADe_SIMU*"));
    assert!(cad_text.contains("$$$"));
    assert!(cad_text.ends_with("***$$$&&&***$$$&&&"));

    let parsed = parse_cad_str(&cad_text).expect("Falha ao fazer parse do .cad gerado");
    assert_eq!(parsed.items.len(), original.items.len());
    assert_eq!(parsed.items[0].tipo, 3005);
    assert_eq!(parsed.items[1].tag, "-Q1");
    assert_eq!(parsed.items[4].tipo, 1000);
    assert_eq!(parsed.items[4].pins[0], "U1");
    assert_eq!(parsed.title_block.title, "Partida Direta Trifasica");
}

#[test]
fn test_dol_starter_simulation_self_holding_and_thermal_trip() {
    let mut doc = build_dol_starter_preset();
    step_simulation(&mut doc, 50);

    // Inicialmente o motor -M1 está parado
    let m1 = doc.items.iter().find(|i| i.tipo == 1000).unwrap();
    assert!(!doc.sim_active(m1.id));
    assert_eq!(doc.sim_motor_dir(m1.id), 0);

    // 1) Fecha o disjuntor tripolar -Q1 (bi-estável)
    let q1_id = doc.items.iter().find(|i| i.tag == "-Q1").unwrap().id;
    press_component_on_sim(&mut doc, q1_id);
    release_component_on_sim(&mut doc, q1_id);
    step_simulation(&mut doc, 50);

    // Motor ainda parado porque -KM1 está aberto
    let m1 = doc.items.iter().find(|i| i.tipo == 1000).unwrap();
    assert!(!doc.sim_active(m1.id));

    // 2) Pressiona o botão de partida -S1 (NA, 8000)
    let s1_id = doc.items.iter().find(|i| i.tag == "-S1").unwrap().id;
    press_component_on_sim(&mut doc, s1_id);
    step_simulation(&mut doc, 50);

    // Bobina -KM1 energiza, fecha contato de selo -KM1 (13-14), fecha contator -KM1 e gira -M1 no sentido horário!
    let km1_coil = doc
        .items
        .iter()
        .find(|i| i.tipo == 9000 && i.tag == "-KM1")
        .unwrap();
    assert!(
        doc.sim_active(km1_coil.id),
        "Bobina -KM1 deveria estar energizada"
    );

    let m1 = doc.items.iter().find(|i| i.tipo == 1000).unwrap();
    assert!(doc.sim_active(m1.id), "Motor -M1 deveria estar girando");
    assert_eq!(doc.sim_motor_dir(m1.id), 1);

    // 3) Solta o botão pulsador -S1 -> -S1 retorna automaticamente ao estado original (aberto),
    //    e o motor DEVE permanecer ligado pelo contato de selo 13-14 de -KM1!
    release_component_on_sim(&mut doc, s1_id);
    step_simulation(&mut doc, 50);
    let s1 = doc.items.iter().find(|i| i.id == s1_id).unwrap();
    assert!(
        !doc.sim_latched(s1.id) && !doc.sim_active(s1.id),
        "Botão -S1 deve retornar ao estado original quando solto"
    );

    let m1 = doc.items.iter().find(|i| i.tipo == 1000).unwrap();
    assert!(
        doc.sim_active(m1.id),
        "Motor -M1 deve permanecer ligado via contato de selo -KM1 (13-14)"
    );

    // 4) Atua o relé térmico -F1 -> Abre o contato 95-96 e desliga a bobina -KM1 e o motor -M1!
    let f1_id = doc
        .items
        .iter()
        .find(|i| i.tipo == 6007 && i.tag == "-F1")
        .unwrap()
        .id;
    press_component_on_sim(&mut doc, f1_id);
    release_component_on_sim(&mut doc, f1_id);
    step_simulation(&mut doc, 50);

    let m1 = doc.items.iter().find(|i| i.tipo == 1000).unwrap();
    assert!(
        !doc.sim_active(m1.id),
        "Motor -M1 deve desligar imediatamente após atuação do relé térmico -F1"
    );
}

#[test]
fn test_momentary_buttons_press_and_release_all_4_types() {
    let mut doc = SchematicDocument::default();
    // Instancia os 4 tipos de botões pulsadores: 8000 (NO), 8001 (NC), 8002 (NO-NC), 8003 (CON)
    // mais uma chave seletora retentiva 8004 (Switch) para comparar.
    for (idx, tipo) in [8000, 8001, 8002, 8003, 8008].into_iter().enumerate() {
        let sym = find_symbol(tipo).unwrap_or_else(|| panic!("Símbolo {tipo} deve existir"));
        let mut item = sym.instantiate((idx + 1) as i32, (idx as i32) * 30, 30, 0);
        item.tag = format!("-S{}", idx + 1);
        doc.items.push(item);
    }

    // Ao pressionar e soltar os 4 botões (8000, 8001, 8002, 8003), todos voltam para false!
    for id in 1..=4 {
        press_component_on_sim(&mut doc, id);
        step_simulation(&mut doc, 10);
        let it = doc.items.iter().find(|i| i.id == id).unwrap();
        assert!(
            doc.sim_latched(it.id) && doc.sim_active(it.id),
            "Botão tipo {} deve ativar enquanto pressionado",
            it.tipo
        );

        release_component_on_sim(&mut doc, id);
        step_simulation(&mut doc, 10);
        let it = doc.items.iter().find(|i| i.id == id).unwrap();
        assert!(
            !doc.sim_latched(it.id) && !doc.sim_active(it.id),
            "Botão tipo {} deve voltar ao estado original quando solto",
            it.tipo
        );
    }

    // Já a chave interruptor 8008 (id=5, DJ) é bi-estável: permanece ligada ao soltar o mouse,
    // e só desliga no próximo clique.
    press_component_on_sim(&mut doc, 5);
    release_component_on_sim(&mut doc, 5);
    step_simulation(&mut doc, 10);
    let sw = doc.items.iter().find(|i| i.id == 5).unwrap();
    assert!(doc.sim_latched(sw.id) && doc.sim_active(sw.id));

    press_component_on_sim(&mut doc, 5);
    release_component_on_sim(&mut doc, 5);
    step_simulation(&mut doc, 10);
    let sw = doc.items.iter().find(|i| i.id == 5).unwrap();
    assert!(!doc.sim_latched(sw.id) && !doc.sim_active(sw.id));
}

fn verify_doc_equivalence(a: &SchematicDocument, b: &SchematicDocument, ctx: &str) {
    assert_eq!(a.items.len(), b.items.len(), "{ctx}: items.len mismatch");
    assert_eq!(
        a.title_block.title, b.title_block.title,
        "{ctx}: title mismatch"
    );
    for (ia, ib) in a.items.iter().zip(b.items.iter()) {
        assert_eq!(ia.tipo, ib.tipo, "{ctx}: tipo mismatch");
        assert_eq!(ia.tag, ib.tag, "{ctx}: tag mismatch");
        assert_eq!(ia.value, ib.value, "{ctx}: value mismatch");
        assert_eq!(ia.pins, ib.pins, "{ctx}: pins mismatch");
        assert_eq!(
            (ia.x1, ia.y1, ia.x2, ia.y2),
            (ib.x1, ib.y1, ib.x2, ib.y2),
            "{ctx}: coords mismatch"
        );
        assert_eq!(
            ia.orientation, ib.orientation,
            "{ctx}: orientation mismatch"
        );
        assert_eq!(
            ia.trailing_str, ib.trailing_str,
            "{ctx}: trailing_str mismatch"
        );
    }
}

fn simulate_enchimento_de_tanque_fluido(mut doc: SchematicDocument, label: &str) {
    use cade_simu_rs::sim::set_manual_by_tag;

    step_simulation(&mut doc, 50);
    let k1 = doc
        .items
        .iter()
        .find(|i| i.tipo == 9000 && i.tag == "K1")
        .unwrap();
    assert!(!doc.sim_active(k1.id), "{label}: K1 deve iniciar desligado");

    // Pulsa E1 -> liga K1 (selo) e solenoide S1
    set_manual_by_tag(&mut doc, "E1", true);
    step_simulation(&mut doc, 50);
    set_manual_by_tag(&mut doc, "E1", false);
    step_simulation(&mut doc, 50);

    let k1 = doc
        .items
        .iter()
        .find(|i| i.tipo == 9000 && i.tag == "K1")
        .unwrap();
    let s1 = doc
        .items
        .iter()
        .find(|i| i.tipo == 9000 && i.tag == "S1")
        .unwrap();
    assert!(
        doc.sim_active(k1.id),
        "{label}: K1 deve ficar retido após pulsar E1"
    );
    assert!(
        doc.sim_active(s1.id),
        "{label}: S1 deve estar energizada durante enchimento"
    );

    // Nível alto E2 atua (tanque cheio mantém E2 fechado) -> desliga K1/S1, liga FS e temporizador T1 (2s)
    set_manual_by_tag(&mut doc, "E2", true);
    step_simulation(&mut doc, 50);

    let k1 = doc
        .items
        .iter()
        .find(|i| i.tipo == 9000 && i.tag == "K1")
        .unwrap();
    let fs = doc
        .items
        .iter()
        .find(|i| i.tipo == 9000 && i.tag == "FS")
        .unwrap();
    let t1 = doc
        .items
        .iter()
        .find(|i| i.tipo == 9004 && i.tag == "T1")
        .unwrap();
    assert!(
        !doc.sim_active(k1.id),
        "{label}: K1 deve desligar ao atingir E2"
    );
    assert!(doc.sim_active(fs.id), "{label}: FS deve ligar após E2");
    assert!(
        doc.sim_active(t1.id),
        "{label}: T1 deve iniciar contagem de 2s"
    );

    // Avança 2.1s -> T1 dispara, liga K2, S2 e T2 (3s)
    step_simulation(&mut doc, 2100);
    let k2 = doc
        .items
        .iter()
        .find(|i| i.tipo == 9000 && i.tag == "K2")
        .unwrap();
    let s2 = doc
        .items
        .iter()
        .find(|i| i.tipo == 9000 && i.tag == "S2")
        .unwrap();
    let t2 = doc
        .items
        .iter()
        .find(|i| i.tipo == 9004 && i.tag == "T2")
        .unwrap();
    assert!(
        doc.sim_active(k2.id),
        "{label}: K2 deve ligar após 2s de T1"
    );
    assert!(
        doc.sim_active(s2.id),
        "{label}: S2 deve abrir esvaziamento após T1"
    );
    assert!(
        doc.sim_active(t2.id),
        "{label}: T2 deve iniciar contagem de 3s"
    );

    // Avança 3.1s -> T2 dispara e abre contato NF T2 desligando S2
    step_simulation(&mut doc, 3100);
    let s2 = doc
        .items
        .iter()
        .find(|i| i.tipo == 9000 && i.tag == "S2")
        .unwrap();
    assert!(
        !doc.sim_active(s2.id),
        "{label}: S2 deve desligar após 3s de T2"
    );
}

fn simulate_inversao_rotacao_motor_trifasico(mut doc: SchematicDocument, label: &str) {
    use cade_simu_rs::model::MotorMode;
    use cade_simu_rs::sim::set_manual_by_tag;

    step_simulation(&mut doc, 50);

    // Pulsa B1 (sentido horário via K1)
    set_manual_by_tag(&mut doc, "B1", true);
    step_simulation(&mut doc, 50);
    set_manual_by_tag(&mut doc, "B1", false);
    step_simulation(&mut doc, 50);

    let m = doc
        .items
        .iter()
        .find(|i| i.tipo == 1000 && i.tag == "-M")
        .unwrap();
    assert!(doc.sim_active(m.id), "{label}: Motor -M deve ligar com B1");
    assert_eq!(
        doc.sim_motor_dir(m.id),
        1,
        "{label}: Motor -M deve girar CW (1) com K1"
    );
    assert_eq!(doc.sim_motor_mode(m.id), MotorMode::Direct);

    // Pulsa B0 (parada)
    set_manual_by_tag(&mut doc, "B0", true);
    step_simulation(&mut doc, 50);
    set_manual_by_tag(&mut doc, "B0", false);
    step_simulation(&mut doc, 50);

    let m = doc
        .items
        .iter()
        .find(|i| i.tipo == 1000 && i.tag == "-M")
        .unwrap();
    assert!(!doc.sim_active(m.id), "{label}: Motor -M deve parar com B0");

    // Pulsa B2 (sentido anti-horário via K2)
    set_manual_by_tag(&mut doc, "B2", true);
    step_simulation(&mut doc, 50);
    set_manual_by_tag(&mut doc, "B2", false);
    step_simulation(&mut doc, 50);

    let m = doc
        .items
        .iter()
        .find(|i| i.tipo == 1000 && i.tag == "-M")
        .unwrap();
    assert!(doc.sim_active(m.id), "{label}: Motor -M deve ligar com B2");
    assert_eq!(
        doc.sim_motor_dir(m.id),
        -1,
        "{label}: Motor -M deve girar CCW (-1) com K2"
    );
}

fn simulate_estrela_triangulo_sem_inversao(mut doc: SchematicDocument, label: &str) {
    use cade_simu_rs::model::MotorMode;
    use cade_simu_rs::sim::set_manual_by_tag;

    step_simulation(&mut doc, 50);
    // Pulsa S1 -> Partida em Estrela (K1 + K3 + K6=5s)
    set_manual_by_tag(&mut doc, "S1", true);
    step_simulation(&mut doc, 50);
    set_manual_by_tag(&mut doc, "S1", false);
    step_simulation(&mut doc, 50);

    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert!(
        doc.sim_active(m.id),
        "{label}: Motor deve partir ao pulsar S1"
    );
    assert_eq!(doc.sim_motor_dir(m.id), 1, "{label}: Motor deve girar CW");
    assert_eq!(
        doc.sim_motor_mode(m.id),
        MotorMode::Star,
        "{label}: Motor deve iniciar em Estrela (Y)"
    );

    // Avança 5.1s -> Comuta para Triângulo (K1 + K2)
    step_simulation(&mut doc, 5100);
    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert!(
        doc.sim_active(m.id),
        "{label}: Motor deve continuar ligado após 5s"
    );
    assert_eq!(
        doc.sim_motor_dir(m.id),
        1,
        "{label}: Motor deve manter sentido CW"
    );
    assert_eq!(
        doc.sim_motor_mode(m.id),
        MotorMode::Delta,
        "{label}: Motor deve comutar para Triângulo (Δ) após 5s"
    );
}

fn simulate_estrela_triangulo_com_inversao(mut doc: SchematicDocument, label: &str) {
    use cade_simu_rs::model::MotorMode;
    use cade_simu_rs::sim::set_manual_by_tag;

    step_simulation(&mut doc, 50);

    // 1) Sentido horário (B1): Estrela -> Triângulo
    set_manual_by_tag(&mut doc, "B1", true);
    step_simulation(&mut doc, 50);
    set_manual_by_tag(&mut doc, "B1", false);
    step_simulation(&mut doc, 50);

    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert_eq!(
        (
            doc.sim_active(m.id),
            doc.sim_motor_dir(m.id),
            doc.sim_motor_mode(m.id)
        ),
        (true, 1, MotorMode::Star)
    );

    step_simulation(&mut doc, 5100);
    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert_eq!(
        (
            doc.sim_active(m.id),
            doc.sim_motor_dir(m.id),
            doc.sim_motor_mode(m.id)
        ),
        (true, 1, MotorMode::Delta)
    );

    // 2) Parada (B0)
    set_manual_by_tag(&mut doc, "B0", true);
    step_simulation(&mut doc, 50);
    set_manual_by_tag(&mut doc, "B0", false);
    step_simulation(&mut doc, 50);
    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert!(!doc.sim_active(m.id), "{label}: Motor deve parar com B0");

    // 3) Sentido anti-horário (B2): Estrela -> Triângulo
    set_manual_by_tag(&mut doc, "B2", true);
    step_simulation(&mut doc, 50);
    set_manual_by_tag(&mut doc, "B2", false);
    step_simulation(&mut doc, 50);

    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert_eq!(
        (
            doc.sim_active(m.id),
            doc.sim_motor_dir(m.id),
            doc.sim_motor_mode(m.id)
        ),
        (true, -1, MotorMode::Star)
    );

    step_simulation(&mut doc, 5100);
    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert_eq!(
        (
            doc.sim_active(m.id),
            doc.sim_motor_dir(m.id),
            doc.sim_motor_mode(m.id)
        ),
        (true, -1, MotorMode::Delta)
    );
}

fn step_secs(doc: &mut SchematicDocument, secs: f64) {
    let steps = ((secs * 10.0).round() as usize).max(1);
    for _ in 0..steps {
        step_simulation(doc, 100);
    }
}

fn simulate_estrela_triangulo_com_inversao_automatico(mut doc: SchematicDocument, label: &str) {
    use cade_simu_rs::model::MotorMode;
    use cade_simu_rs::sim::set_manual_by_tag;

    step_simulation(&mut doc, 50);

    // Pulsa B1: inicia ciclo automático
    set_manual_by_tag(&mut doc, "B1", true);
    step_simulation(&mut doc, 100);
    set_manual_by_tag(&mut doc, "B1", false);
    step_simulation(&mut doc, 100);

    // t = 0..4s: Horário em Estrela (K1 + K4)
    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert_eq!(
        (
            doc.sim_active(m.id),
            doc.sim_motor_dir(m.id),
            doc.sim_motor_mode(m.id)
        ),
        (true, 1, MotorMode::Star),
        "{label}: t=0s deve estar CW Star"
    );

    // Avança 5.5s -> Horário em Triângulo (K1 + K3)
    step_secs(&mut doc, 5.5);
    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert_eq!(
        (
            doc.sim_active(m.id),
            doc.sim_motor_dir(m.id),
            doc.sim_motor_mode(m.id)
        ),
        (true, 1, MotorMode::Delta),
        "{label}: t=5.5s deve estar CW Delta"
    );

    // Avança +5.5s (t=11s) -> Pausa de frenagem/inércia antes de inverter (K5 TOF contando 5s)
    step_secs(&mut doc, 5.5);
    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert_eq!(
        (
            doc.sim_active(m.id),
            doc.sim_motor_dir(m.id),
            doc.sim_motor_mode(m.id)
        ),
        (false, 0, MotorMode::Off),
        "{label}: t=11s deve estar em pausa antes de inverter"
    );

    // Avança +5.5s (t=16.5s) -> Inversão automática para Anti-horário em Estrela (K2 + K4)
    step_secs(&mut doc, 5.5);
    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert_eq!(
        (
            doc.sim_active(m.id),
            doc.sim_motor_dir(m.id),
            doc.sim_motor_mode(m.id)
        ),
        (true, -1, MotorMode::Star),
        "{label}: t=16.5s deve estar CCW Star"
    );

    // Avança +5.5s (t=22s) -> Anti-horário em Triângulo (K2 + K3)
    step_secs(&mut doc, 5.5);
    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert_eq!(
        (
            doc.sim_active(m.id),
            doc.sim_motor_dir(m.id),
            doc.sim_motor_mode(m.id)
        ),
        (true, -1, MotorMode::Delta),
        "{label}: t=22s deve estar CCW Delta"
    );
}

fn simulate_ligacao_tres_motores(mut doc: SchematicDocument, label: &str) {
    use cade_simu_rs::model::MotorMode;
    use cade_simu_rs::sim::set_manual_by_tag;

    step_simulation(&mut doc, 50);
    // Pulsa S1: cascata sequencial de 3 motores estrela-triângulo (3s por estágio)
    set_manual_by_tag(&mut doc, "S1", true);
    step_simulation(&mut doc, 50);
    set_manual_by_tag(&mut doc, "S1", false);
    step_simulation(&mut doc, 50);

    let motor_modes = |d: &SchematicDocument| -> Vec<(String, MotorMode)> {
        d.items
            .iter()
            .filter(|i| i.tipo == 1001)
            .map(|i| (i.tag.clone(), d.sim_motor_mode(i.id)))
            .collect()
    };

    // t = 0s: M1 em Estrela, M2 Off, M3 Off
    assert_eq!(
        motor_modes(&doc),
        vec![
            ("M1".to_string(), MotorMode::Star),
            ("M2".to_string(), MotorMode::Off),
            ("M3".to_string(), MotorMode::Off),
        ],
        "{label}: estágio 1 (t=0s)"
    );

    // t = 3.1s: M1 em Triângulo, M2 Off, M3 Off
    step_simulation(&mut doc, 3100);
    assert_eq!(
        motor_modes(&doc),
        vec![
            ("M1".to_string(), MotorMode::Delta),
            ("M2".to_string(), MotorMode::Off),
            ("M3".to_string(), MotorMode::Off),
        ],
        "{label}: estágio 2 (t=3.1s)"
    );

    // t = 6.2s: M1 em Triângulo, M2 em Estrela, M3 Off
    step_simulation(&mut doc, 3100);
    assert_eq!(
        motor_modes(&doc),
        vec![
            ("M1".to_string(), MotorMode::Delta),
            ("M2".to_string(), MotorMode::Star),
            ("M3".to_string(), MotorMode::Off),
        ],
        "{label}: estágio 3 (t=6.2s)"
    );

    // t = 9.3s: M1 em Triângulo, M2 em Triângulo, M3 Off
    step_simulation(&mut doc, 3100);
    assert_eq!(
        motor_modes(&doc),
        vec![
            ("M1".to_string(), MotorMode::Delta),
            ("M2".to_string(), MotorMode::Delta),
            ("M3".to_string(), MotorMode::Off),
        ],
        "{label}: estágio 4 (t=9.3s)"
    );

    // t = 12.4s: M1 em Triângulo, M2 em Triângulo, M3 em Estrela
    step_simulation(&mut doc, 3100);
    assert_eq!(
        motor_modes(&doc),
        vec![
            ("M1".to_string(), MotorMode::Delta),
            ("M2".to_string(), MotorMode::Delta),
            ("M3".to_string(), MotorMode::Star),
        ],
        "{label}: estágio 5 (t=12.4s)"
    );

    // t = 15.5s: Todos os 3 motores M1, M2, M3 em Triângulo!
    step_simulation(&mut doc, 3100);
    assert_eq!(
        motor_modes(&doc),
        vec![
            ("M1".to_string(), MotorMode::Delta),
            ("M2".to_string(), MotorMode::Delta),
            ("M3".to_string(), MotorMode::Delta),
        ],
        "{label}: estágio 6 final (t=15.5s)"
    );
}

fn simulate_partida_direta(mut doc: SchematicDocument, label: &str) {
    use cade_simu_rs::sim::set_manual_by_tag;

    step_simulation(&mut doc, 50);
    set_manual_by_tag(&mut doc, "DJ1", true);
    set_manual_by_tag(&mut doc, "DJ2", true);
    step_simulation(&mut doc, 50);

    let mo = doc
        .items
        .iter()
        .find(|i| i.tipo == 1000 && i.tag == "MO")
        .unwrap();
    assert!(
        !doc.sim_active(mo.id),
        "{label}: Motor MO deve iniciar desligado"
    );

    // Pulsa B1 -> liga -KM1 (selo) e gira motor MO
    set_manual_by_tag(&mut doc, "B1", true);
    step_simulation(&mut doc, 50);
    set_manual_by_tag(&mut doc, "B1", false);
    step_simulation(&mut doc, 50);

    let mo = doc
        .items
        .iter()
        .find(|i| i.tipo == 1000 && i.tag == "MO")
        .unwrap();
    assert!(
        doc.sim_active(mo.id),
        "{label}: Motor MO deve girar após pulsar B1"
    );
    assert_eq!(doc.sim_motor_dir(mo.id), 1);

    // Pulsa B0 -> desliga -KM1 e para motor MO
    set_manual_by_tag(&mut doc, "B0", true);
    step_simulation(&mut doc, 50);
    set_manual_by_tag(&mut doc, "B0", false);
    step_simulation(&mut doc, 50);

    let mo = doc
        .items
        .iter()
        .find(|i| i.tipo == 1000 && i.tag == "MO")
        .unwrap();
    assert!(
        !doc.sim_active(mo.id),
        "{label}: Motor MO deve parar após pulsar B0"
    );
}

fn simulate_exemplo_desconexao(mut doc: SchematicDocument, label: &str) {
    use cade_simu_rs::sim::set_manual_by_tag;

    step_simulation(&mut doc, 50);
    set_manual_by_tag(&mut doc, "DJ1", true);
    set_manual_by_tag(&mut doc, "DJ2", true);
    step_simulation(&mut doc, 50);

    // Pulsa B1 -> liga K1 (motor 1 id=6)
    set_manual_by_tag(&mut doc, "B1", true);
    step_simulation(&mut doc, 50);
    set_manual_by_tag(&mut doc, "B1", false);
    step_simulation(&mut doc, 50);

    let m1 = doc.items.iter().find(|i| i.id == 6).unwrap();
    let m2 = doc.items.iter().find(|i| i.id == 7).unwrap();
    assert!(
        doc.sim_active(m1.id),
        "{label}: Motor 1 (K1) deve ligar com B1"
    );
    assert!(
        !doc.sim_active(m2.id),
        "{label}: Motor 2 (K2) ainda desligado"
    );

    // Pulsa B0 -> desliga K1 (motor 1) e dispara temporizador Off-Delay TP (30s) ligando K2 (motor 2)
    set_manual_by_tag(&mut doc, "B0", true);
    step_simulation(&mut doc, 100);
    set_manual_by_tag(&mut doc, "B0", false);
    step_simulation(&mut doc, 100);

    let m1 = doc.items.iter().find(|i| i.id == 6).unwrap();
    let m2 = doc.items.iter().find(|i| i.id == 7).unwrap();
    assert!(
        !doc.sim_active(m1.id),
        "{label}: Motor 1 deve desligar ao pulsar B0"
    );
    assert!(
        doc.sim_active(m2.id),
        "{label}: Motor 2 (K2) deve ficar ligado durante os 30s do Off-Delay TP"
    );

    // Avança 30.5s -> TP expira e desliga K2 (motor 2)
    step_secs(&mut doc, 30.5);
    let m2 = doc.items.iter().find(|i| i.id == 7).unwrap();
    assert!(
        !doc.sim_active(m2.id),
        "{label}: Motor 2 (K2) deve desligar após os 30s do Off-Delay TP"
    );
}

fn simulate_partida_motores_escalonada(mut doc: SchematicDocument, label: &str) {
    use cade_simu_rs::sim::set_manual_by_tag;

    step_simulation(&mut doc, 50);
    set_manual_by_tag(&mut doc, "DJ", true);
    step_simulation(&mut doc, 50);

    // Pulsa B1 -> Liga K1/M1 imediatamente e inicia T1 (7s)
    set_manual_by_tag(&mut doc, "B1", true);
    step_simulation(&mut doc, 100);
    set_manual_by_tag(&mut doc, "B1", false);
    step_simulation(&mut doc, 100);

    let m_states = |d: &SchematicDocument| -> (bool, bool, bool, bool) {
        let m1 = d.sim_active(
            d.items
                .iter()
                .find(|i| i.tipo == 1000 && i.tag == "M1")
                .unwrap()
                .id,
        );
        let m2 = d.sim_active(
            d.items
                .iter()
                .find(|i| i.tipo == 1000 && i.tag == "M2")
                .unwrap()
                .id,
        );
        let m3 = d.sim_active(
            d.items
                .iter()
                .find(|i| i.tipo == 1000 && i.tag == "M3")
                .unwrap()
                .id,
        );
        let ok = d.sim_active(
            d.items
                .iter()
                .find(|i| i.tipo == 9008 && i.tag == "OK")
                .unwrap()
                .id,
        );
        (m1, m2, m3, ok)
    };

    assert_eq!(
        m_states(&doc),
        (true, false, false, false),
        "{label}: t=0s (M1 ligado)"
    );

    // Avança 7.2s -> T1 (7s) dispara e liga K2/M2 + inicia T2 (7s)
    step_secs(&mut doc, 7.2);
    assert_eq!(
        m_states(&doc),
        (true, true, false, false),
        "{label}: t=7.2s (M1 e M2 ligados)"
    );

    // Avança +7.2s (t=14.4s) -> T2 (7s) dispara e liga K3/M3 + inicia T3 (3s)
    step_secs(&mut doc, 7.2);
    assert_eq!(
        m_states(&doc),
        (true, true, true, false),
        "{label}: t=14.4s (M1, M2, M3 ligados)"
    );

    // Avança +3.2s (t=17.6s) -> T3 (3s) dispara e acende sinaleiro OK
    step_secs(&mut doc, 3.2);
    assert_eq!(
        m_states(&doc),
        (true, true, true, true),
        "{label}: t=17.6s (M1, M2, M3 e sinaleiro OK ligados)"
    );
}

#[test]
fn test_all_6_projetos_open_simulate_and_roundtrip_json_toml() {
    use std::path::Path;

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let projetos_dir = if manifest.join("Projetos").is_dir() {
        manifest.join("Projetos")
    } else {
        manifest.parent().unwrap().join("Projetos")
    };

    #[allow(clippy::type_complexity)]
    let test_cases: Vec<(&str, fn(SchematicDocument, &str))> = vec![
        (
            "EnchimentodeTanqueFluido",
            simulate_enchimento_de_tanque_fluido,
        ),
        (
            "Inversão de rotação motor trifásico",
            simulate_inversao_rotacao_motor_trifasico,
        ),
        (
            "Estrela triângulo sem inversão de rotação",
            simulate_estrela_triangulo_sem_inversao,
        ),
        (
            "Estrela triângulo com inversão de rotação",
            simulate_estrela_triangulo_com_inversao,
        ),
        (
            "Estrela triângulo com inversão de rotação automático",
            simulate_estrela_triangulo_com_inversao_automatico,
        ),
        ("Ligação três motores", simulate_ligacao_tres_motores),
        ("Partida Direta", simulate_partida_direta),
        ("Exemplo Desconexão", simulate_exemplo_desconexao),
        (
            "Partida de Motores Escalonada",
            simulate_partida_motores_escalonada,
        ),
    ];

    for (stem, sim_fn) in test_cases {
        let cad_path = projetos_dir.join(format!("{stem}.cad"));
        let json_path = projetos_dir.join(format!("{stem}.json"));
        let toml_path = projetos_dir.join(format!("{stem}.toml"));

        // 1) Abre o projeto .cad original
        let doc_cad = load_document(&cad_path)
            .unwrap_or_else(|e| panic!("Erro ao abrir {:?}: {}", cad_path, e));
        assert!(
            !doc_cad.items.is_empty(),
            "Projeto {stem}.cad não deve estar vazio"
        );

        // 2) Salva como .json e .toml na pasta Projetos/
        save_document(&json_path, &doc_cad)
            .unwrap_or_else(|e| panic!("Erro ao salvar {:?}: {}", json_path, e));
        save_document(&toml_path, &doc_cad)
            .unwrap_or_else(|e| panic!("Erro ao salvar {:?}: {}", toml_path, e));

        // 3) Reabre os arquivos .json e .toml (e testa roundtrip .cad em memória)
        let doc_json = load_document(&json_path)
            .unwrap_or_else(|e| panic!("Erro ao reabrir {:?}: {}", json_path, e));
        let doc_toml = load_document(&toml_path)
            .unwrap_or_else(|e| panic!("Erro ao reabrir {:?}: {}", toml_path, e));
        let cad_roundtrip = parse_cad_str(&serialize_cad_str(&doc_cad))
            .unwrap_or_else(|e| panic!("Erro no roundtrip .cad de {stem}: {}", e));

        // 4) Verifica equivalência estrutural completa
        verify_doc_equivalence(&doc_cad, &doc_json, &format!("{stem} (.cad vs .json)"));
        verify_doc_equivalence(&doc_cad, &doc_toml, &format!("{stem} (.cad vs .toml)"));
        verify_doc_equivalence(
            &doc_cad,
            &cad_roundtrip,
            &format!("{stem} (.cad vs .cad roundtrip)"),
        );

        // 5) Executa a simulação elétrica completa no .cad, no .json reaberto e no .toml reaberto
        sim_fn(doc_cad, &format!("{stem}.cad"));
        sim_fn(doc_json, &format!("{stem}.json"));
        sim_fn(doc_toml, &format!("{stem}.toml"));

        println!(
            "[OK] Projeto '{}' aberto (.cad), salvo e reaberto (.json e .toml) e simulado nas 3 versões com sucesso!",
            stem
        );
    }
}

#[test]
fn test_svg_push_button_rendering_and_animation() {
    use cade_simu_rs::svg_symbol::{get_parsed_svg, SvgShape};

    let sym_no = find_symbol(8000).expect("Botão NA 8000 deve existir no catálogo");
    let (no_idle_path, no_active_path) = sym_no
        .svg_paths()
        .expect("Botão NA 8000 deve ter caminhos SVG definidos");

    let no_idle = get_parsed_svg(no_idle_path)
        .unwrap_or_else(|| panic!("Falha ao carregar SVG NO idle: {no_idle_path}"));
    let no_active = get_parsed_svg(no_active_path)
        .unwrap_or_else(|| panic!("Falha ao carregar SVG NO active: {no_active_path}"));

    assert_eq!(no_idle.term_top_y, -25.2);
    assert_eq!(no_idle.term_bottom_y, 25.2);

    // Mapeamento de terminais para escala de grade do CADe (distância 12 unidades)
    let (t0_x, t0_y) = no_idle.map_point(0.0, -25.2, 12.0);
    let (t1_x, t1_y) = no_idle.map_point(0.0, 25.2, 12.0);
    assert!((t0_x - 0.0).abs() < 1e-4 && (t0_y - 0.0).abs() < 1e-4);
    assert!((t1_x - 0.0).abs() < 1e-4 && (t1_y - 12.0).abs() < 1e-4);

    // Verifica que no idle a lâmina está aberta e no ativo fecha em x=0
    let has_open_blade = no_idle.shapes.iter().any(|s| match s {
        SvgShape::Polyline { points } => points.iter().any(|(x, _)| *x < -1.0),
        _ => false,
    });
    assert!(
        has_open_blade,
        "Lâmina do botão NA deve estar aberta em repouso"
    );

    let has_closed_blade = no_active.shapes.iter().any(|s| match s {
        SvgShape::Polyline { points } => points.iter().all(|(x, _)| x.abs() < 0.1),
        _ => false,
    });
    assert!(
        has_closed_blade,
        "Lâmina do botão NA deve fechar (x=0) quando ativo/pressionado"
    );

    // Verifica botão NF (8001)
    let sym_nc = find_symbol(8001).expect("Botão NF 8001 deve existir no catálogo");
    let (nc_idle_path, nc_active_path) = sym_nc
        .svg_paths()
        .expect("Botão NF 8001 deve ter caminhos SVG definidos");

    let nc_idle = get_parsed_svg(nc_idle_path)
        .unwrap_or_else(|| panic!("Falha ao carregar SVG NC idle: {nc_idle_path}"));
    let nc_active = get_parsed_svg(nc_active_path)
        .unwrap_or_else(|| panic!("Falha ao carregar SVG NC active: {nc_active_path}"));

    assert_eq!(nc_idle.term_top_y, -25.2);
    assert_eq!(nc_idle.term_bottom_y, 25.2);

    // No ativo do NF, a lâmina é empurrada para a direita (x >= 8) abrindo o circuito
    let has_pushed_blade = nc_active.shapes.iter().any(|s| match s {
        SvgShape::Polyline { points } => points.iter().any(|(x, _)| *x >= 8.0),
        _ => false,
    });
    assert!(
        has_pushed_blade,
        "Lâmina do botão NF deve ser empurrada para abrir quando pressionada"
    );

    // Verifica que o Motor 1000 possui corpo (Ellipse) e texto descritivo
    let sym_motor = find_symbol(1000).expect("Motor 1000 deve existir");
    let (m_idle_path, m_act_path) = sym_motor.svg_paths().expect("Motor deve ter SVGs");
    let m_idle = get_parsed_svg(m_idle_path).expect("Motor idle deve carregar");
    let m_act = get_parsed_svg(m_act_path).expect("Motor active deve carregar");
    assert!(
        m_idle
            .shapes
            .iter()
            .any(|s| matches!(s, SvgShape::Ellipse { .. })),
        "Motor deve conter elipse/círculo principal de carcaça"
    );
    assert!(
        m_idle
            .shapes
            .iter()
            .any(|s| matches!(s, SvgShape::Text { .. })),
        "Motor deve conter textos informativos (ex: M, 3~)"
    );
    assert!(m_act
        .shapes
        .iter()
        .any(|s| matches!(s, SvgShape::Ellipse { .. })));

    // Verifica que o Relé Térmico Tripolar 6007 (QElectroTech thermal_relay_4) mapeia perfeitamente os 3 polos (0, 6, 12) e tem linha pontilhada
    let sym_thermal = find_symbol(6007).expect("Relé térmico 6007 deve existir");
    let (th_idle_path, th_act_path) = sym_thermal.svg_paths().expect("Relé térmico deve ter SVGs");
    assert!(th_idle_path.contains("thermal_relay_4.svg"));
    assert!(th_act_path.contains("thermal_relay_4_active.svg"));
    let th_idle = get_parsed_svg(th_idle_path).expect("Relé térmico QET idle deve carregar");
    let _th_act = get_parsed_svg(th_act_path).expect("Relé térmico QET active deve carregar");
    assert_eq!(th_idle.max_term_x, 96.0);
    assert!(
        th_idle
            .shapes
            .iter()
            .any(|s| matches!(s, SvgShape::Rect { dashed: true, .. })),
        "Relé térmico 4 deve conter linha pontilhada em volta (rect dashed)"
    );
    let (p1x, _) = th_idle.map_point(0.0, -21.0, 12.0);
    let (p2x, _) = th_idle.map_point(48.0, -21.0, 12.0);
    let (p3x, _) = th_idle.map_point(96.0, -21.0, 12.0);
    assert!(
        (p1x - 0.0).abs() < 1e-4,
        "Pólo 1 do relé térmico deve estar em x=0"
    );
    assert!(
        (p2x - 6.0).abs() < 1e-4,
        "Pólo 2 do relé térmico deve estar em x=6"
    );
    assert!(
        (p3x - 12.0).abs() < 1e-4,
        "Pólo 3 do relé térmico deve estar em x=12"
    );

    // Verifica que os contatos auxiliares 8016 e 8017 do relé térmico usam os SVGs QET
    let sym_th_nc = find_symbol(8017).expect("Contato NF relé térmico deve existir");
    let (nc_path, nc_act) = sym_th_nc.svg_paths().expect("8017 deve ter SVGs");
    assert!(nc_path.contains("contact_thermal_relay__nc.svg"));
    assert!(nc_act.contains("contact_thermal_relay__nc_active.svg"));

    // Verifica os Disjuntores Termomagnéticos (2P, 3P e 4P) com os SVGs oficiais solicitados
    // 1. Disjuntor 2P (com 2 pontos)
    let sym_brk_2p = find_symbol(6008).expect("Disjuntor 2P deve existir");
    let (b2_idle_path, b2_act_path) = sym_brk_2p.svg_paths().expect("6008 deve ter SVGs");
    assert!(b2_idle_path.contains("magneto-thermal_circuit_breaker_gv_2p.svg"));
    assert!(b2_act_path.contains("magneto-thermal_circuit_breaker_gv_2p_active.svg"));
    let b2_idle = get_parsed_svg(b2_idle_path).expect("SVG 2P idle deve carregar");
    let _b2_act = get_parsed_svg(b2_act_path).expect("SVG 2P active deve carregar");
    assert_eq!(b2_idle.min_term_x, -48.0);
    assert_eq!(b2_idle.max_term_x, 0.0);
    let (p1, p1_y) = b2_idle.map_point(-48.0, b2_idle.term_top_y, 21.0);
    let (p2, p2_y) = b2_idle.map_point(0.0, b2_idle.term_bottom_y, 21.0);
    assert!(
        (p1 - 0.0).abs() < 1e-4,
        "Terminal 1 do disjuntor 2P deve estar em x=0"
    );
    assert!(
        (p1_y - 0.0).abs() < 1e-4,
        "Terminal 1 do disjuntor 2P deve estar em y=0"
    );
    assert!(
        (p2 - 6.0).abs() < 1e-4,
        "Terminal 4 do disjuntor 2P deve estar em x=6"
    );
    assert!(
        (p2_y - 21.0).abs() < 1e-4,
        "Terminal 4 do disjuntor 2P deve estar em y=21"
    );

    // 2. Disjuntor 3P (com 3 pontos) - mesma altura do 2P (target_span=21.0)
    let sym_brk_3p = find_symbol(6003).expect("Disjuntor 3P deve existir");
    let (b3_idle_path, b3_act_path) = sym_brk_3p.svg_paths().expect("6003 deve ter SVGs");
    assert!(b3_idle_path.contains("magneto-thermal_circuit_breaker_gv_4.svg"));
    assert!(b3_act_path.contains("magneto-thermal_circuit_breaker_gv_4_active.svg"));
    let b3_idle = get_parsed_svg(b3_idle_path).expect("SVG 3P idle deve carregar");
    let _b3_act = get_parsed_svg(b3_act_path).expect("SVG 3P active deve carregar");
    assert_eq!(b3_idle.min_term_x, -48.0);
    assert_eq!(b3_idle.max_term_x, 48.0);
    let (b3_p1, _) = b3_idle.map_point(-48.0, b3_idle.term_top_y, 21.0);
    let (b3_p2, _) = b3_idle.map_point(0.0, b3_idle.term_top_y, 21.0);
    let (b3_p3, b3_p3_y) = b3_idle.map_point(48.0, b3_idle.term_bottom_y, 21.0);
    assert!(
        (b3_p1 - 0.0).abs() < 1e-4,
        "Pólo 1 do disjuntor 3P deve estar em x=0"
    );
    assert!(
        (b3_p2 - 6.0).abs() < 1e-4,
        "Pólo 2 do disjuntor 3P deve estar em x=6"
    );
    assert!(
        (b3_p3 - 12.0).abs() < 1e-4,
        "Pólo 3 do disjuntor 3P deve estar em x=12"
    );
    assert!(
        (b3_p3_y - 21.0).abs() < 1e-4,
        "Bornes inferiores do disjuntor 3P devem estar em y=21 (altura igual ao 2P)"
    );

    // 3. Disjuntor 4P (com 4 pontos) - mesma altura do 2P (target_span=21.0)
    let sym_brk_4p = find_symbol(6004).expect("Disjuntor 4P deve existir");
    let (b4_idle_path, b4_act_path) = sym_brk_4p.svg_paths().expect("6004 deve ter SVGs");
    assert!(b4_idle_path.contains("magneto-thermal_circuit_breaker.svg"));
    assert!(b4_act_path.contains("magneto-thermal_circuit_breaker_active.svg"));
    let b4_idle = get_parsed_svg(b4_idle_path).expect("SVG 4P idle deve carregar");
    let _b4_act = get_parsed_svg(b4_act_path).expect("SVG 4P active deve carregar");
    assert_eq!(b4_idle.min_term_x, -48.0);
    assert_eq!(b4_idle.max_term_x, 96.0);
    let (b4_pn, _) = b4_idle.map_point(-48.0, b4_idle.term_top_y, 21.0);
    let (b4_p1, _) = b4_idle.map_point(0.0, b4_idle.term_top_y, 21.0);
    let (b4_p2, _) = b4_idle.map_point(48.0, b4_idle.term_top_y, 21.0);
    let (b4_p3, b4_p3_y) = b4_idle.map_point(96.0, b4_idle.term_bottom_y, 21.0);
    assert!(
        (b4_pn - 0.0).abs() < 1e-4,
        "Pólo N do disjuntor 4P deve estar em x=0"
    );
    assert!(
        (b4_p1 - 6.0).abs() < 1e-4,
        "Pólo 1 do disjuntor 4P deve estar em x=6"
    );
    assert!(
        (b4_p2 - 12.0).abs() < 1e-4,
        "Pólo 2 do disjuntor 4P deve estar em x=12"
    );
    assert!(
        (b4_p3 - 18.0).abs() < 1e-4,
        "Pólo 3 do disjuntor 4P deve estar em x=18"
    );
    assert!(
        (b4_p3_y - 21.0).abs() < 1e-4,
        "Bornes inferiores do disjuntor 4P devem estar em y=21 (altura igual ao 2P)"
    );
}

#[test]
fn test_qet_kaefer_open_simulate_and_roundtrip() {
    use std::path::Path;

    let qet_path = Path::new("/home/caiojordao84/Downloads/Kaefer_1303_20260204-2.qet");
    if !qet_path.exists() {
        println!("Arquivo QET de teste não encontrado no caminho padrão, ignorando teste.");
        return;
    }

    // 1. Carrega o projeto completo com 23 folhas e 300+ definições de biblioteca
    let project = cade_simu_rs::qet::load_qet_file(qet_path)
        .expect("Deve carregar o arquivo Kaefer_1303_20260204-2.qet com sucesso");

    assert_eq!(project.title, "Kaefer_1303_20251206");
    assert_eq!(
        project.diagrams.len(),
        23,
        "Projeto do Fusca deve conter 23 diagramas"
    );
    assert!(
        project.element_definitions.len() >= 100,
        "Coleção de símbolos embutidos deve ter mais de 100 elementos definidos"
    );

    // 2. Converte o Diagrama 2 (Bateria, chave de ignição e motor de partida) em SchematicDocument
    let d2_idx = project
        .diagrams
        .iter()
        .position(|d| d.order == 2)
        .expect("Diagrama de ordem 2 deve existir");

    let mut doc = cade_simu_rs::qet::diagram_to_document(&project, d2_idx);
    assert!(
        doc.items.len() >= 30,
        "Diagrama 2 deve ter elementos, condutores e textos importados (total: {})",
        doc.items.len()
    );

    // Verifica a presença dos componentes principais do circuito de partida
    let has_battery = doc.items.iter().any(|i| {
        i.qet_type
            .as_deref()
            .map(|t| t.to_ascii_lowercase().contains("battery"))
            .unwrap_or(false)
    });
    assert!(
        has_battery,
        "Diagrama 2 deve conter o componente da Bateria 12V"
    );

    let starter_idx = doc
        .items
        .iter()
        .position(|i| {
            i.qet_type
                .as_deref()
                .map(|t| t.to_ascii_lowercase().ends_with("starter_001.elmt"))
                .unwrap_or(false)
        })
        .expect("Diagrama 2 deve conter o motor de partida (starter_001.elmt)");

    // Verifica que os símbolos QET possuem textos dinâmicos e labels de bornes compilados
    let starter_qet_type = doc.items[starter_idx].qet_type.as_ref().unwrap();
    let parsed_starter = cade_simu_rs::svg_symbol::get_parsed_svg(starter_qet_type)
        .expect("Símbolo do motor de partida deve estar no cache de SVGs compilados");
    let has_text_shapes = parsed_starter
        .shapes
        .iter()
        .any(|s| matches!(s, cade_simu_rs::svg_symbol::SvgShape::Text { .. }));
    assert!(
        has_text_shapes,
        "Símbolo do motor de partida deve conter textos/labels dinâmicos compilados com rotação!"
    );

    let switch_idx = doc
        .items
        .iter()
        .position(|i| {
            i.qet_type
                .as_deref()
                .map(|t| {
                    t.to_ascii_lowercase()
                        .contains("ignition_starter_switch_with_park")
                })
                .unwrap_or(false)
        })
        .expect("Diagrama 2 deve conter o comutador de ignição");

    // 3. Testa a simulação elétrica do circuito de partida do Fusca
    // Estado inicial de repouso: motor de partida desligado
    step_simulation(&mut doc, 0);
    let starter_id = doc.items[starter_idx].id;
    let switch_id = doc.items[switch_idx].id;
    assert!(
        !doc.sim_active(starter_id),
        "Motor de partida deve estar desativado com a chave de ignição aberta"
    );

    // Aciona a chave de ignição (partida)
    doc.set_sim_latched(switch_id, true);
    step_simulation(&mut doc, 0);

    // O motor de partida deve ser energizado pela bateria via comutador
    assert!(
        doc.sim_active(starter_id),
        "Motor de partida deve estar ativo quando a chave de ignição for ligada!"
    );
    assert_eq!(
        doc.sim_motor_dir(starter_id),
        1,
        "Motor de partida deve indicar rotação ativa (dir = 1)"
    );

    // Ao desligar a chave de ignição, o motor de partida desliga
    doc.set_sim_latched(switch_id, false);
    step_simulation(&mut doc, 0);
    assert!(
        !doc.sim_active(starter_id),
        "Motor de partida deve desligar ao soltar a chave de ignição"
    );

    // 3b. Testa a simulação da Folha 12 ("Horn, fuel gauge, fan", índice 11)
    let mut doc_horn = cade_simu_rs::qet::diagram_to_document(&project, 11);
    let horn_idx = doc_horn
        .items
        .iter()
        .position(|i| {
            i.qet_type
                .as_deref()
                .map(|t| t.to_ascii_lowercase().ends_with("horn_001.elmt"))
                .unwrap_or(false)
        })
        .expect("Diagrama 12 deve conter a buzina (horn_001.elmt)");
    let horn_sw_idx = doc_horn
        .items
        .iter()
        .position(|i| {
            i.qet_type
                .as_deref()
                .map(|t| t.to_ascii_lowercase().contains("horn_switch"))
                .unwrap_or(false)
        })
        .expect("Diagrama 12 deve conter o botão da buzina (horn_switch_001.elmt)");

    let horn_id = doc_horn.items[horn_idx].id;
    let horn_sw_id = doc_horn.items[horn_sw_idx].id;

    step_simulation(&mut doc_horn, 0);
    assert!(!doc_horn.sim_active(horn_id), "Buzina desligada em repouso");

    doc_horn.set_sim_latched(horn_sw_id, true);
    step_simulation(&mut doc_horn, 0);
    assert!(
        doc_horn.sim_active(horn_id),
        "Buzina deve tocar ao acionar o botão horn_switch!"
    );

    // 3c. Simulação em todas as 23 folhas do projeto Kaefer
    let mut total_energized = 0;
    for idx in 0..project.diagrams.len() {
        let mut d = cade_simu_rs::qet::diagram_to_document(&project, idx);
        let source_ids: Vec<i32> = d
            .items
            .iter()
            .filter(|i| {
                i.qet_type
                    .as_deref()
                    .map(cade_simu_rs::sim::is_qet_source_dc_pos)
                    .unwrap_or(false)
            })
            .map(|i| i.id)
            .collect();
        for id in source_ids {
            d.set_sim_latched(id, true);
        }
        step_simulation(&mut d, 0);
        let count = d
            .items
            .iter()
            .filter(|i| d.sim_potential(i.id) != Potential::None || d.sim_active(i.id))
            .count();
        total_energized += count;
    }
    println!(
        "Total de itens energizados nas 23 folhas do Kaefer: {}",
        total_energized
    );
    assert!(
        total_energized >= 401,
        "Total de elementos energizados ({}) não deve regredir!",
        total_energized
    );

    // 4. Testa roundtrip de gravação e recarregamento em XML .qet
    let tmp_out = Path::new("/tmp/test_kaefer_roundtrip.qet");
    cade_simu_rs::qet::save_qet_file(tmp_out, &project)
        .expect("Gravação de projeto .qet deve ser concluída sem erros");

    let reloaded = cade_simu_rs::qet::load_qet_file(tmp_out)
        .expect("Deve recarregar com sucesso o arquivo .qet salvo");

    assert_eq!(reloaded.title, project.title);
    assert_eq!(reloaded.diagrams.len(), 23);
    assert!(reloaded.element_definitions.len() >= 100);

    let _ = std::fs::remove_file(tmp_out);
}

#[test]
fn test_new_ui_layout_and_theme_collections() {
    let (mut app, _) = CadeSimuApp::new();

    // Verifica estado inicial do novo layout e tema
    assert_eq!(app.theme_mode, ThemeMode::Dark);
    assert_eq!(app.prop_tab, 0);
    assert_eq!(app.selected_collection_sym, Some(6003));
    assert!(app.collection_search.is_empty());

    // Verifica ciclo de tema: Dark -> System -> Light -> Dark
    assert_eq!(app.theme_mode.next(), ThemeMode::System);
    assert_eq!(app.theme_mode.next().next(), ThemeMode::Light);
    assert_eq!(app.theme_mode.next().next().next(), ThemeMode::Dark);

    // Testa comportamento da aba Elementos e Projetos:
    // 1) Clique simples apenas seleciona o símbolo para pré-visualização, mantendo a ferramenta Select
    let _ = app.update(Message::SelectCollectionSymbol(6008));
    assert_eq!(app.selected_collection_sym, Some(6008));
    assert_eq!(app.tool, EditorTool::Select);

    // 2) Duplo clique arma o símbolo no ponteiro do mouse
    let _ = app.update(Message::SelectCollectionSymbol(6008));
    assert_eq!(app.tool, EditorTool::PlaceSymbol(6008));

    // 3) Cancelamento com tecla ESC retorna imediatamente ao modo de seleção
    let _ = app.update(Message::CancelAction);
    assert_eq!(app.tool, EditorTool::Select);

    // 4) Botão "Inserir no Esquema" arma o símbolo
    let _ = app.update(Message::InsertSelectedCollectionSymbol);
    assert_eq!(app.tool, EditorTool::PlaceSymbol(6008));

    // 5) Cancelamento com botão direito no Canvas retorna ao modo de seleção
    let _ = app.update(Message::Canvas(CanvasMessage::CancelAction));
    assert_eq!(app.tool, EditorTool::Select);

    // 6) Arraste do símbolo solto no Canvas coloca o elemento diretamente na posição
    let items_before = app.doc.items.len();
    let _ = app.update(Message::StartSymbolDrag(6008));
    assert_eq!(app.dragging_symbol, Some(6008));
    let _ = app.update(Message::Canvas(CanvasMessage::PlaceComponent {
        tipo: 6008,
        x: 100,
        y: 150,
    }));
    assert_eq!(app.doc.items.len(), items_before + 1);
    assert_eq!(app.dragging_symbol, None);
    assert_eq!(app.tool, EditorTool::Select);

    // Testa troca de abas de propriedades
    let _ = app.update(Message::SelectPropTab(1));
    assert_eq!(app.prop_tab, 1);
    let _ = app.update(Message::SelectPropTab(2));
    assert_eq!(app.prop_tab, 2);

    // Testa busca de coleções
    let _ = app.update(Message::CollectionSearchChanged("motor".to_string()));
    assert_eq!(app.collection_search, "motor");

    // Testa renderização completa da UI
    let _view_element = app.view();
}

#[test]
fn test_multipage_menus_iec_undo_redo_toolbar() {
    let (mut app, _) = CadeSimuApp::new();

    // 1) Multi-páginas: inicial tem 1 página
    assert_eq!(app.pages.len(), 1);
    assert_eq!(app.active_page_index, 0);

    // Adiciona página depois
    let _ = app.update(Message::AddPageAfter(0));
    assert_eq!(app.pages.len(), 2);
    assert_eq!(app.active_page_index, 1);
    assert!(app.is_modified);

    // Adiciona página antes
    let _ = app.update(Message::AddPageBefore(1));
    assert_eq!(app.pages.len(), 3);
    assert_eq!(app.active_page_index, 1);

    // Renomeia folha ativa
    let _ = app.update(Message::StartEditingPageName(1));
    assert!(app.editing_page_name.is_some());
    let _ = app.update(Message::UpdateEditingPageName(
        "Diagrama de Comando".to_string(),
    ));
    let _ = app.update(Message::CommitEditingPageName);
    assert_eq!(app.pages[1].name, "Diagrama de Comando");

    // Reordena páginas
    let _ = app.update(Message::MovePage(1, 0));
    assert_eq!(app.pages[0].name, "Diagrama de Comando");
    assert_eq!(app.active_page_index, 0);

    // Exclui página
    let _ = app.update(Message::DeletePage(2));
    assert_eq!(app.pages.len(), 2);

    // 2) Menus Dropdown Flutuantes em Camada Superior (Stack / Overlay)
    assert_eq!(app.open_menu, None);
    let _ = app.update(Message::ToggleMenu(Some(ActiveMenu::Arquivo)));
    assert_eq!(app.open_menu, Some(ActiveMenu::Arquivo));
    {
        let _ = app.view();
    }
    // Alterna diretamente para outro menu (Exibição)
    let _ = app.update(Message::ToggleMenu(Some(ActiveMenu::Exibicao)));
    assert_eq!(app.open_menu, Some(ActiveMenu::Exibicao));
    {
        let _ = app.view();
    }
    // Uma ação como ZoomIn executa e fecha o menu dropdown automaticamente
    let _ = app.update(Message::ZoomIn);
    assert_eq!(app.open_menu, None);

    // Fecha explicitamente com CloseMenu
    let _ = app.update(Message::ToggleMenu(Some(ActiveMenu::Arquivo)));
    assert_eq!(app.open_menu, Some(ActiveMenu::Arquivo));
    let _ = app.update(Message::CloseMenu);
    assert_eq!(app.open_menu, None);

    // 3) Configurações de Grade, Textos e Autoconexão
    assert_eq!(app.grid_density.step(), 6);
    let _ = app.update(Message::SetGridDensity(
        cade_simu_rs::model::GridDensity::Fine,
    ));
    assert_eq!(app.grid_density.step(), 3);
    let _ = app.update(Message::SetGridDensity(
        cade_simu_rs::model::GridDensity::Coarse,
    ));
    assert_eq!(app.grid_density.step(), 12);

    assert!(app.show_texts);
    let _ = app.update(Message::ToggleShowTexts);
    assert!(!app.show_texts);

    assert!(app.auto_connect);
    let _ = app.update(Message::ToggleAutoConnect);
    assert!(!app.auto_connect);

    assert!(!app.auto_disconnect);
    let _ = app.update(Message::ToggleAutoDisconnect);
    assert!(app.auto_disconnect);

    // 4) IEC Presets e Edição de Propriedades
    let original_page_idx = app
        .pages
        .iter()
        .position(|p| !p.doc.items.is_empty())
        .unwrap_or(0);
    let _ = app.update(Message::SelectPage(original_page_idx));
    assert!(!app.doc.items.is_empty());

    let initial_items_count = app.doc.items.len();
    let first_id = app.doc.items[0].id;
    app.selected_id = Some(first_id);

    let _ = app.update(Message::ApplyIecNumberingPreset("power_3p"));
    let item = app.doc.items.iter().find(|i| i.id == first_id).unwrap();
    assert_eq!(item.pins[0], "1");
    assert_eq!(item.pins[1], "3");
    assert_eq!(item.pins[2], "5");
    assert_eq!(item.pins[3], "2");
    assert_eq!(item.pins[4], "4");
    assert_eq!(item.pins[5], "6");

    let _ = app.update(Message::ApplyIecNumberingPreset("aux_no"));
    let item = app.doc.items.iter().find(|i| i.id == first_id).unwrap();
    assert_eq!(item.pins[0], "13");
    assert_eq!(item.pins[1], "14");

    let _ = app.update(Message::ApplyIecNumberingPreset("aux_nc"));
    let item = app.doc.items.iter().find(|i| i.id == first_id).unwrap();
    assert_eq!(item.pins[0], "21");
    assert_eq!(item.pins[1], "22");

    let _ = app.update(Message::ApplyIecNumberingPreset("coil"));
    let item = app.doc.items.iter().find(|i| i.id == first_id).unwrap();
    assert_eq!(item.pins[0], "A1");
    assert_eq!(item.pins[1], "A2");

    // 5) Cores de lâmpada e pulso
    let _ = app.update(Message::SetLampColor(2)); // Vermelho
    let item = app.doc.items.iter().find(|i| i.id == first_id).unwrap();
    assert_eq!(item.color, 2);

    let _ = app.update(Message::SetPulseDuration("0.5".to_string()));
    let item = app.doc.items.iter().find(|i| i.id == first_id).unwrap();
    assert_eq!(item.value, "0.5");

    // 6) Área de transferência: Copiar, Colar, Duplicar
    let _ = app.update(Message::Copy);
    assert!(app.clipboard.is_some());

    let _ = app.update(Message::Paste);
    assert_eq!(app.doc.items.len(), initial_items_count + 1);

    let _ = app.update(Message::Duplicate);
    assert_eq!(app.doc.items.len(), initial_items_count + 2);

    // 7) Undo e Redo
    let _ = app.update(Message::Undo);
    assert_eq!(app.doc.items.len(), initial_items_count + 1);
    let _ = app.update(Message::Undo);
    assert_eq!(app.doc.items.len(), initial_items_count);

    let _ = app.update(Message::Redo);
    assert_eq!(app.doc.items.len(), initial_items_count + 1);

    // 8) Dock Esquerdo Recolhível e Abas
    assert!(!app.left_dock_collapsed);
    let _ = app.update(Message::ToggleLeftDockCollapse);
    assert!(app.left_dock_collapsed);
    {
        let _ = app.view();
    }
    let _ = app.update(Message::ToggleLeftDockCollapse);
    assert!(!app.left_dock_collapsed);

    let _ = app.update(Message::SelectLeftDockTab(1)); // Projetos
    assert_eq!(app.left_dock_tab, 1);
    {
        let _ = app.view();
    }

    // 8b) Dock Direito Recolhível
    assert!(!app.right_dock_collapsed);
    let _ = app.update(Message::ToggleRightDockCollapse);
    assert!(app.right_dock_collapsed);
    {
        let _ = app.view();
    }
    let _ = app.update(Message::ToggleRightDockCollapse);
    assert!(!app.right_dock_collapsed);
    {
        let _ = app.view();
    }

    // 9) Modal Sobre
    let _ = app.update(Message::ShowAbout);
    assert!(app.show_about_modal);
    {
        let _ = app.view();
    }
    let _ = app.update(Message::CloseAbout);
    assert!(!app.show_about_modal);

    // 10) Seleção, Cópia de Info e Snap à Grelha
    let _ = app.update(Message::SelectAll);
    assert!(app.selected_id.is_some());

    let _ = app.update(Message::CopyElementInfo);
    assert!(app.copied_info.is_some());

    let _ = app.update(Message::DeselectAll);
    assert!(app.selected_id.is_none());

    assert!(app.snap_to_grid);
    let _ = app.update(Message::ToggleSnap);
    assert!(!app.snap_to_grid);

    let _ = app.update(Message::PasteAtOriginalPos);
    assert_eq!(app.doc.items.len(), initial_items_count + 2);

    // 11) Tooltip seguindo o tema com fundo e estilo
    let tt_style_dark = theme::tooltip(&iced::Theme::Dark);
    assert!(tt_style_dark.background.is_some());
    let tt_style_light = theme::tooltip(&iced::Theme::Light);
    assert!(tt_style_light.background.is_some());
}

#[test]
fn test_sinaleiro_color_and_aux_contacts_distinction() {
    let (mut app, _) = CadeSimuApp::new();

    // 1) Contatos Auxiliares NA (7000) e NF (7001) não devem ser sinalizadores/lâmpadas
    let sym_na = find_symbol(7000).expect("Contato NA deve existir");
    let item_na = sym_na.instantiate(101, 10, 10, 0);
    assert_eq!(item_na.tipo, 7000);
    assert!(
        !item_na.is_lamp_or_signaling(),
        "Contato NA (7000) NÃO deve ser considerado lâmpada"
    );

    let sym_nf = find_symbol(7001).expect("Contato NF deve existir");
    let item_nf = sym_nf.instantiate(102, 20, 20, 0);
    assert_eq!(item_nf.tipo, 7001);
    assert!(
        !item_nf.is_lamp_or_signaling(),
        "Contato NF (7001) NÃO deve ser considerado lâmpada"
    );

    // 2) Sinaleiro Luminoso (Piloto, 9008) DEVE ser sinaleiro/lâmpada
    let sym_lamp = find_symbol(9008).expect("Sinaleiro 9008 deve existir");
    let item_lamp = sym_lamp.instantiate(103, 30, 30, 0);
    assert_eq!(item_lamp.tipo, 9008);
    assert!(
        item_lamp.is_lamp_or_signaling(),
        "Sinaleiro (9008) DEVE ser considerado lâmpada"
    );

    // 3) Ao inserir o sinaleiro, ele deve ter cor verde por padrão (1) e permitir alterar
    assert_eq!(item_lamp.color, 1); // Verde padrão
    app.doc.items.push(item_lamp);
    app.selected_id = Some(103);

    // Altera cor para Vermelho (2)
    let _ = app.update(Message::SetLampColor(2));
    let item = app.doc.items.iter().find(|i| i.id == 103).unwrap();
    assert_eq!(item.color, 2);

    // Altera cor para Amarelo (3)
    let _ = app.update(Message::SetLampColor(3));
    let item = app.doc.items.iter().find(|i| i.id == 103).unwrap();
    assert_eq!(item.color, 3);

    // Altera cor para Azul (4)
    let _ = app.update(Message::SetLampColor(4));
    let item = app.doc.items.iter().find(|i| i.id == 103).unwrap();
    assert_eq!(item.color, 4);

    // Aplica preset de bornes Piloto X1-X2
    let _ = app.update(Message::ApplyIecNumberingPreset("lamp"));
    let item = app.doc.items.iter().find(|i| i.id == 103).unwrap();
    assert_eq!(item.pins[0], "X1");
    assert_eq!(item.pins[1], "X2");

    // Renderiza a view garantindo que o painel de propriedades renderiza sem pânico
    let _ = app.view();
}

#[test]
fn test_all_icons_render_valid_svg() {
    use cade_simu_rs::icons::{view_icon, Icon};
    let all_icons = [
        Icon::New,
        Icon::Open,
        Icon::Save,
        Icon::SaveAs,
        Icon::Close,
        Icon::Print,
        Icon::ExportPdf,
        Icon::Undo,
        Icon::Redo,
        Icon::Cut,
        Icon::Copy,
        Icon::Paste,
        Icon::Duplicate,
        Icon::Delete,
        Icon::RotateCw,
        Icon::RotateCcw,
        Icon::MirrorH,
        Icon::MirrorV,
        Icon::Palette,
        Icon::Select,
        Icon::Pan,
        Icon::WireL,
        Icon::WireN,
        Icon::WirePe,
        Icon::WireDc,
        Icon::Junction,
        Icon::DeleteWire,
        Icon::ZoomIn,
        Icon::ZoomOut,
        Icon::ZoomReset,
        Icon::FitToPage,
        Icon::Grid,
        Icon::Magnet,
        Icon::Text,
        Icon::Theme,
        Icon::Play,
        Icon::Pause,
        Icon::Stop,
        Icon::Step,
        Icon::Reset,
        Icon::ChevronLeft,
        Icon::ChevronRight,
        Icon::AddBefore,
        Icon::AddAfter,
        Icon::Search,
        Icon::Pencil,
        Icon::Folder,
        Icon::Bolt,
        Icon::Info,
        Icon::Warning,
        Icon::Check,
        Icon::Group,
        Icon::Ungroup,
    ];
    for icon in all_icons {
        let bytes = icon.bytes();
        assert!(!bytes.is_empty());
        let _widget = view_icon(icon, 16, iced::Color::BLACK);
    }
}

#[test]
fn test_canvas_theme_adaptation_light_dark_system() {
    let (mut app, _) = CadeSimuApp::new();

    // 1. Testa resolução de temas (LIGHT, DARK, SYSTEM)
    app.theme_mode = ThemeMode::Light;
    assert!(!app.is_dark());

    app.theme_mode = ThemeMode::Dark;
    assert!(app.is_dark());

    app.theme_mode = ThemeMode::System;
    app.system_is_dark = true;
    assert!(app.is_dark());

    app.system_is_dark = false;
    assert!(!app.is_dark());

    // 2. Testa comportamento do wire_color no Canvas
    let mut wire_phase = SchematicItem::new_wire(1, 4000, 10, 10, 20, 10);
    let wire_neutral = SchematicItem::new_wire(2, 4009, 10, 20, 20, 20);
    let wire_dneg = SchematicItem::new_wire(3, 4004, 10, 30, 20, 30);
    let wire_generic = SchematicItem::new_wire(4, 4001, 10, 40, 20, 40);

    let dummy_selected_ids = std::collections::HashSet::new();

    let canvas_light = SchematicCanvas {
        doc: &app.doc,
        tool: app.tool.clone(),
        selected_id: None,
        selected_ids: &dummy_selected_ids,
        is_shift_pressed: false,
        is_ctrl_pressed: false,
        is_alt_pressed: false,
        sim_running: false,
        show_grid: true,
        snap_to_grid: true,
        grid_density: app.grid_density,
        show_texts: true,
        show_terminals: true,
        show_cajetin: true,
        show_full_crosshair: false,
        show_text_grid: false,
        show_alignment_guides: false,
        keep_headers_visible: false,
        show_cell_bounds: false,
        is_dark: false,
        auto_connect: true,
        auto_disconnect: true,
        zoom: 1.0,
        pan: iced::Vector::default(),
        placement_orientation: 0,
        dragging_symbol: None,
        dragging_canonical_symbol: None,
        is_blocked: false,
    };

    let canvas_dark = SchematicCanvas {
        doc: &app.doc,
        tool: app.tool.clone(),
        selected_id: None,
        selected_ids: &dummy_selected_ids,
        is_shift_pressed: false,
        is_ctrl_pressed: false,
        is_alt_pressed: false,
        sim_running: false,
        show_grid: true,
        snap_to_grid: true,
        grid_density: app.grid_density,
        show_texts: true,
        show_terminals: true,
        show_cajetin: true,
        show_full_crosshair: false,
        show_text_grid: false,
        show_alignment_guides: false,
        keep_headers_visible: false,
        show_cell_bounds: false,
        is_dark: true,
        auto_connect: true,
        auto_disconnect: true,
        zoom: 1.0,
        pan: iced::Vector::default(),
        placement_orientation: 0,
        dragging_symbol: None,
        dragging_canonical_symbol: None,
        is_blocked: false,
    };

    // No tema LIGHT, itens e conectores são escuros (inalterados)
    let (col_l_phase, _) = canvas_light.wire_color(&wire_phase);
    assert_eq!(col_l_phase, Color::from_rgb(0.55, 0.18, 0.12));

    let (col_l_neutral, _) = canvas_light.wire_color(&wire_neutral);
    assert_eq!(col_l_neutral, Color::from_rgb(0.08, 0.38, 0.82));

    let (col_l_dneg, _) = canvas_light.wire_color(&wire_dneg);
    assert_eq!(col_l_dneg, Color::from_rgb(0.15, 0.15, 0.20));

    let (col_l_generic, _) = canvas_light.wire_color(&wire_generic);
    assert_eq!(col_l_generic, Color::from_rgb(0.15, 0.18, 0.24));

    // No tema DARK, conectores sem cor personalizada tornam-se claros
    let (col_d_phase, _) = canvas_dark.wire_color(&wire_phase);
    assert_eq!(col_d_phase, Color::from_rgb(0.94, 0.58, 0.50));

    let (col_d_neutral, _) = canvas_dark.wire_color(&wire_neutral);
    assert_eq!(col_d_neutral, Color::from_rgb(0.45, 0.75, 1.0));

    let (col_d_dneg, _) = canvas_dark.wire_color(&wire_dneg);
    assert_eq!(col_d_dneg, Color::from_rgb(0.85, 0.88, 0.95));

    let (col_d_generic, _) = canvas_dark.wire_color(&wire_generic);
    assert_eq!(col_d_generic, Color::from_rgb(0.88, 0.92, 0.98));

    // 3. Conectores com cor personalizada definida pelo usuário (item.color != 0)
    // NUNCA são sobrescritos pelo tema
    let custom_color_rgb = 0x00FF88; // verde-água vibrante
    wire_phase.color = custom_color_rgb;
    let (col_cust_light, _) = canvas_light.wire_color(&wire_phase);
    let (col_cust_dark, _) = canvas_dark.wire_color(&wire_phase);
    assert_eq!(col_cust_light, col_cust_dark);
    assert_eq!(
        col_cust_light,
        Color::from_rgb(0.0, 1.0, 0x88 as f32 / 255.0)
    );

    // 4. Renderiza view em modo claro e escuro sem erros
    app.theme_mode = ThemeMode::Light;
    let _ = app.view();
    app.theme_mode = ThemeMode::Dark;
    let _ = app.view();

    // 5. Testa adaptação da miniatura de preview (preview_dock) ao tema claro e escuro
    app.selected_collection_sym = Some(8008); // Chave interruptor DJ
    let sym_dj = find_symbol(8008).unwrap();
    let svg_dj_light = cade_simu_rs::symbols::get_themed_symbol_svg_string(&sym_dj, false);
    let svg_dj_dark = cade_simu_rs::symbols::get_themed_symbol_svg_string(&sym_dj, true);

    assert!(
        svg_dj_light.contains("stroke=\"#111827\"") || svg_dj_light.contains("stroke=\"black\""),
        "No tema claro, o traço da miniatura deve ser escuro"
    );
    assert!(
        svg_dj_dark.contains("stroke=\"#E2E8F0\""),
        "No tema escuro, o traço da miniatura deve ser claro (#E2E8F0)"
    );
    assert!(
        !svg_dj_dark.contains("stroke=\"#111827\"") && !svg_dj_dark.contains("stroke=\"black\""),
        "No tema escuro, traços pretos devem ter sido substituídos por claros"
    );

    // Testa também para botão de emergência com trava (8030)
    let sym_emg = find_symbol(8030).unwrap();
    let svg_emg_dark = cade_simu_rs::symbols::get_themed_symbol_svg_string(&sym_emg, true);
    assert!(
        svg_emg_dark.contains("stroke=\"#E2E8F0\""),
        "Emergência 8030 deve ter traços claros em dark mode"
    );
    assert!(
        !svg_emg_dark.contains("stroke=\"black\""),
        "Emergência 8030 não pode ter stroke='black' em dark mode"
    );

    // Testa para componente gerado dinamicamente (ex: motor 1000)
    let sym_motor = find_symbol(1000).unwrap();
    let svg_motor_dark = cade_simu_rs::symbols::get_themed_symbol_svg_string(&sym_motor, true);
    assert!(
        svg_motor_dark.contains("stroke=\"#E2E8F0\""),
        "Motor 1000 deve ter traços claros em dark mode"
    );

    // Renderiza app.view() com o símbolo selecionado em ambos os temas para garantir integração total
    app.theme_mode = ThemeMode::Light;
    let _ = app.view();
    app.theme_mode = ThemeMode::Dark;
    let _ = app.view();
}

#[test]
fn test_strict_and_series_conduction_command_and_power() {
    use cade_simu_rs::sim::set_manual_by_tag;

    // ==================== 1. CIRCUITO DE COMANDO EM SÉRIE (AND / &&) ====================
    // Monta: L1 (3000) -> S1 (NA, 8000) -> S2 (NA, 8000) -> Bobina KM1 (9000) -> Neutro N (3001)
    let mut doc = SchematicDocument::default();
    let mut id_counter = 1;
    let mut next_id = || {
        let cur = id_counter;
        id_counter += 1;
        cur
    };

    // Alimentação Fase L em (72, 10)
    let sym_l = find_symbol(3000).unwrap();
    let mut it_l = sym_l.instantiate(next_id(), 72, 10, 0);
    it_l.tag = "-L1".to_string();
    doc.items.push(it_l);

    // Chave S1 (NA, 8000) em (72, 20): pin 0 em (72, 20), pin 1 em (72, 32)
    let sym_s1 = find_symbol(8000).unwrap();
    let mut it_s1 = sym_s1.instantiate(next_id(), 72, 20, 0);
    it_s1.tag = "-S1".to_string();
    doc.items.push(it_s1);

    // Chave S2 (NA, 8000) em (72, 40): pin 0 em (72, 40), pin 1 em (72, 52)
    let sym_s2 = find_symbol(8000).unwrap();
    let mut it_s2 = sym_s2.instantiate(next_id(), 72, 40, 0);
    it_s2.tag = "-S2".to_string();
    doc.items.push(it_s2);

    // Bobina KM1 (9000) em (72, 60): pin 0 (A1) em (72, 60), pin 1 (A2) em (72, 72)
    let sym_km = find_symbol(9000).unwrap();
    let mut it_km = sym_km.instantiate(next_id(), 72, 60, 0);
    it_km.tag = "-KM1".to_string();
    doc.items.push(it_km);

    // Alimentação Neutro N (3001) em (72, 80, rot 2)
    let sym_n = find_symbol(3001).unwrap();
    let mut it_n = sym_n.instantiate(next_id(), 72, 80, 2);
    it_n.tag = "-N".to_string();
    doc.items.push(it_n);

    // Cabos ligando em série:
    // Cabo 1: L -> S1 (72, 10) -> (72, 20)
    let w_l_s1 = next_id();
    doc.items
        .push(SchematicItem::new_wire(w_l_s1, 4000, 72, 10, 72, 20));
    // Cabo 2: S1 -> S2 (72, 32) -> (72, 40)
    let w_s1_s2 = next_id();
    doc.items
        .push(SchematicItem::new_wire(w_s1_s2, 4000, 72, 32, 72, 40));
    // Cabo 3: S2 -> KM1 (72, 52) -> (72, 60)
    let w_s2_km = next_id();
    doc.items
        .push(SchematicItem::new_wire(w_s2_km, 4000, 72, 52, 72, 60));
    // Cabo 4: KM1 -> N (72, 72) -> (72, 80)
    let w_km_n = next_id();
    doc.items
        .push(SchematicItem::new_wire(w_km_n, 4009, 72, 72, 72, 80));

    // Caso 1: (S1=0, S2=0) -> Ambos abertos
    step_simulation(&mut doc, 50);
    let km = doc
        .items
        .iter()
        .find(|i| i.tag == "-KM1" && i.tipo == 9000)
        .unwrap();
    assert!(
        !doc.sim_active(km.id),
        "AND (0,0): Bobina deve estar DESLIGADA"
    );
    assert_eq!(doc.sim_potential(w_s1_s2), Potential::None);
    assert_eq!(doc.sim_potential(w_s2_km), Potential::None);

    // Caso 2: (S1=1, S2=0) -> S1 fechado, S2 aberto
    // A eletricidade passa por S1 (cabo w_s1_s2 recebe L1), mas NÃO passa por S2 (w_s2_km é None)!
    set_manual_by_tag(&mut doc, "-S1", true);
    step_simulation(&mut doc, 50);
    let km = doc
        .items
        .iter()
        .find(|i| i.tag == "-KM1" && i.tipo == 9000)
        .unwrap();
    assert!(
        !doc.sim_active(km.id),
        "AND (1,0): Bobina deve estar DESLIGADA se S2 estiver aberto"
    );
    assert_eq!(doc.sim_potential(w_s1_s2), Potential::L1);
    assert_eq!(doc.sim_potential(w_s2_km), Potential::None);

    // Caso 3: (S1=0, S2=1) -> S1 aberto, S2 fechado
    // A eletricidade é barrada em S1!
    set_manual_by_tag(&mut doc, "-S1", false);
    set_manual_by_tag(&mut doc, "-S2", true);
    step_simulation(&mut doc, 50);
    let km = doc
        .items
        .iter()
        .find(|i| i.tag == "-KM1" && i.tipo == 9000)
        .unwrap();
    assert!(
        !doc.sim_active(km.id),
        "AND (0,1): Bobina deve estar DESLIGADA se S1 estiver aberto"
    );
    assert_eq!(doc.sim_potential(w_s1_s2), Potential::None);
    assert_eq!(doc.sim_potential(w_s2_km), Potential::None);

    // Caso 4: (S1=1, S2=1) -> Ambos fechados!
    // Condição AND satisfeita: eletricidade passa por S1 E por S2, energizando KM1!
    set_manual_by_tag(&mut doc, "-S1", true);
    set_manual_by_tag(&mut doc, "-S2", true);
    step_simulation(&mut doc, 50);
    let km = doc
        .items
        .iter()
        .find(|i| i.tag == "-KM1" && i.tipo == 9000)
        .unwrap();
    assert!(
        doc.sim_active(km.id),
        "AND (1,1): Bobina DEVE LIGAR quando ambos estiverem fechados"
    );
    assert_eq!(doc.sim_potential(w_s1_s2), Potential::L1);
    assert_eq!(doc.sim_potential(w_s2_km), Potential::L1);

    // ==================== 2. CIRCUITO DE POTÊNCIA EM SÉRIE (AND / &&) ====================
    // Monta: Alimentação 3F (3004) -> Disjuntor 3P -Q1 (6003) -> Contator 3P -KM2 (2001) -> Motor 3~ -M1 (1000)
    let mut doc_pwr = SchematicDocument::default();
    let sym_3f = find_symbol(3004).unwrap();
    doc_pwr.items.push(sym_3f.instantiate(next_id(), 24, 18, 0));

    let sym_q1 = find_symbol(6003).unwrap();
    let mut it_q1 = sym_q1.instantiate(next_id(), 24, 24, 0);
    it_q1.tag = "-Q1".to_string();
    doc_pwr.items.push(it_q1);

    let sym_km2 = find_symbol(2001).unwrap();
    let mut it_km2 = sym_km2.instantiate(next_id(), 24, 51, 0);
    it_km2.tag = "-KM2".to_string();
    doc_pwr.items.push(it_km2);

    // Circuito de comando para acionar a bobina KM2:
    // L -> S_KM2 (8000) -> Bobina KM2 (9000) -> N
    let sym_l_cmd = find_symbol(3000).unwrap();
    doc_pwr
        .items
        .push(sym_l_cmd.instantiate(next_id(), 72, 45, 0));

    let sym_s_km2 = find_symbol(8000).unwrap();
    let mut it_s_km2 = sym_s_km2.instantiate(next_id(), 72, 50, 0);
    it_s_km2.tag = "-SKM2".to_string();
    doc_pwr.items.push(it_s_km2);

    let mut it_km2_coil = sym_km.instantiate(next_id(), 72, 68, 0);
    it_km2_coil.tag = "-KM2".to_string();
    doc_pwr.items.push(it_km2_coil);

    let sym_n_cmd = find_symbol(3001).unwrap();
    doc_pwr
        .items
        .push(sym_n_cmd.instantiate(next_id(), 72, 85, 2));

    // Cabos de comando da bobina KM2:
    doc_pwr
        .items
        .push(SchematicItem::new_wire(next_id(), 4000, 72, 45, 72, 50));
    doc_pwr
        .items
        .push(SchematicItem::new_wire(next_id(), 4000, 72, 62, 72, 68));
    doc_pwr
        .items
        .push(SchematicItem::new_wire(next_id(), 4009, 72, 80, 72, 85));

    let sym_m1 = find_symbol(1000).unwrap();
    let mut it_m1 = sym_m1.instantiate(next_id(), 24, 69, 0);
    it_m1.tag = "-M1".to_string();
    doc_pwr.items.push(it_m1);

    // Cabos das 3 fases L1, L2, L3 interligando os elementos em série
    let mut wires_mid = Vec::new();
    let mut wires_bot = Vec::new();
    for dx in [0, 6, 12] {
        let x = 24 + dx;
        doc_pwr
            .items
            .push(SchematicItem::new_wire(next_id(), 4000, x, 18, x, 24));
        let w_m = next_id();
        doc_pwr
            .items
            .push(SchematicItem::new_wire(w_m, 4000, x, 45, x, 51));
        wires_mid.push(w_m);
        let w_b = next_id();
        doc_pwr
            .items
            .push(SchematicItem::new_wire(w_b, 4000, x, 63, x, 69));
        wires_bot.push(w_b);
    }

    // Q1 aberto, KM2 aberto -> Motor Parado
    step_simulation(&mut doc_pwr, 50);
    let m1 = doc_pwr.items.iter().find(|i| i.tipo == 1000).unwrap();
    assert!(
        !doc_pwr.sim_active(m1.id),
        "Potência: Motor desligado quando Q1 e KM2 abertos"
    );

    // Q1 fechado, KM2 aberto -> Eletricidade chega até a entrada de KM2, mas NÃO passa para o motor!
    set_manual_by_tag(&mut doc_pwr, "-Q1", true);
    step_simulation(&mut doc_pwr, 50);
    let m1 = doc_pwr.items.iter().find(|i| i.tipo == 1000).unwrap();
    assert!(
        !doc_pwr.sim_active(m1.id),
        "Potência: Motor desligado se contator KM2 estiver aberto"
    );
    for &w in &wires_mid {
        assert!(doc_pwr.sim_potential(w).is_energized());
    }
    for &w in &wires_bot {
        assert_eq!(doc_pwr.sim_potential(w), Potential::None);
    }

    // Q1 aberto, KM2 fechado -> Eletricidade NÃO passa por Q1, motor parado!
    set_manual_by_tag(&mut doc_pwr, "-Q1", false);
    set_manual_by_tag(&mut doc_pwr, "-SKM2", true);
    step_simulation(&mut doc_pwr, 50);
    let m1 = doc_pwr.items.iter().find(|i| i.tipo == 1000).unwrap();
    assert!(
        !doc_pwr.sim_active(m1.id),
        "Potência: Motor desligado se disjuntor Q1 estiver aberto"
    );

    // Q1 fechado E KM2 fechado -> Eletricidade passa por ambos e motor gira!
    set_manual_by_tag(&mut doc_pwr, "-Q1", true);
    set_manual_by_tag(&mut doc_pwr, "-SKM2", true);
    step_simulation(&mut doc_pwr, 50);
    let m1 = doc_pwr.items.iter().find(|i| i.tipo == 1000).unwrap();
    assert!(
        doc_pwr.sim_active(m1.id),
        "Potência: Motor DEVE LIGAR quando Q1 E KM2 estiverem fechados"
    );
    assert_eq!(doc_pwr.sim_motor_dir(m1.id), 1);
    for &w in &wires_bot {
        assert!(doc_pwr.sim_potential(w).is_energized());
    }
}

#[test]
fn test_strict_or_parallel_conduction_command_and_power() {
    use cade_simu_rs::sim::set_manual_by_tag;

    // ==================== 1. CIRCUITO DE COMANDO EM PARALELO (OR / ||) ====================
    // Monta: L1 -> Nó J1
    //               |--> Ramo A: S1 (8000) -->|
    //               |--> Ramo B: S2 (8000) -->|--> Nó J2 -> Bobina KM1 -> Neutro N
    let mut doc = SchematicDocument::default();
    let mut id_counter = 1;
    let mut next_id = || {
        let cur = id_counter;
        id_counter += 1;
        cur
    };

    // Alimentação Fase L em (72, 10)
    let sym_l = find_symbol(3000).unwrap();
    let mut it_l = sym_l.instantiate(next_id(), 72, 10, 0);
    it_l.tag = "-L1".to_string();
    doc.items.push(it_l);

    // Chave S1 (NA) no ramo A em (72, 25): pin 0 em (72, 25), pin 1 em (72, 37)
    let sym_s1 = find_symbol(8000).unwrap();
    let mut it_s1 = sym_s1.instantiate(next_id(), 72, 25, 0);
    it_s1.tag = "-S1".to_string();
    doc.items.push(it_s1);

    // Chave S2 (NA) no ramo B em (84, 25): pin 0 em (84, 25), pin 1 em (84, 37)
    let sym_s2 = find_symbol(8000).unwrap();
    let mut it_s2 = sym_s2.instantiate(next_id(), 84, 25, 0);
    it_s2.tag = "-S2".to_string();
    doc.items.push(it_s2);

    // Bobina KM1 em (72, 50): pin 0 em (72, 50), pin 1 em (72, 62)
    let sym_km = find_symbol(9000).unwrap();
    let mut it_km = sym_km.instantiate(next_id(), 72, 50, 0);
    it_km.tag = "-KM1".to_string();
    doc.items.push(it_km);

    // Alimentação Neutro N em (72, 70, rot 2)
    let sym_n = find_symbol(3001).unwrap();
    let mut it_n = sym_n.instantiate(next_id(), 72, 70, 2);
    it_n.tag = "-N".to_string();
    doc.items.push(it_n);

    // Cabo Fase até o Nó J1 (72, 20)
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 72, 10, 72, 20));
    doc.items
        .push(SchematicItem::new_junction(next_id(), 72, 20));

    // Ramo A: (72, 20) -> (72, 25)
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 72, 20, 72, 25));
    // Saída Ramo A: (72, 37) -> (72, 45)
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 72, 37, 72, 45));

    // Ramo B: (72, 20) -> (84, 20) -> (84, 25)
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 72, 20, 84, 20));
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 84, 20, 84, 25));
    // Saída Ramo B: (84, 37) -> (84, 45) -> (72, 45)
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 84, 37, 84, 45));
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 84, 45, 72, 45));
    doc.items
        .push(SchematicItem::new_junction(next_id(), 72, 45));

    // Cabo de J2 (72, 45) até Bobina A1 (72, 50)
    let w_out = next_id();
    doc.items
        .push(SchematicItem::new_wire(w_out, 4000, 72, 45, 72, 50));
    // Cabo Bobina A2 (72, 62) até Neutro (72, 70)
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4009, 72, 62, 72, 70));

    // Caso 1: (S1=0, S2=0) -> Ambos abertos
    step_simulation(&mut doc, 50);
    let km = doc
        .items
        .iter()
        .find(|i| i.tag == "-KM1" && i.tipo == 9000)
        .unwrap();
    assert!(
        !doc.sim_active(km.id),
        "OR (0,0): Bobina deve estar DESLIGADA"
    );
    assert_eq!(doc.sim_potential(w_out), Potential::None);

    // Caso 2: (S1=1, S2=0) -> S1 fechado, S2 aberto
    // A corrente flui pelo ramo A, alcança o nó J2 e liga a bobina!
    set_manual_by_tag(&mut doc, "-S1", true);
    step_simulation(&mut doc, 50);
    let km = doc
        .items
        .iter()
        .find(|i| i.tag == "-KM1" && i.tipo == 9000)
        .unwrap();
    assert!(
        doc.sim_active(km.id),
        "OR (1,0): Bobina DEVE LIGAR pelo ramo A fechado"
    );
    assert_eq!(doc.sim_potential(w_out), Potential::L1);

    // Caso 3: (S1=0, S2=1) -> S1 aberto, S2 fechado
    // A corrente flui pelo ramo B, alcança o nó J2 e liga a bobina!
    set_manual_by_tag(&mut doc, "-S1", false);
    set_manual_by_tag(&mut doc, "-S2", true);
    step_simulation(&mut doc, 50);
    let km = doc
        .items
        .iter()
        .find(|i| i.tag == "-KM1" && i.tipo == 9000)
        .unwrap();
    assert!(
        doc.sim_active(km.id),
        "OR (0,1): Bobina DEVE LIGAR pelo ramo B fechado"
    );
    assert_eq!(doc.sim_potential(w_out), Potential::L1);

    // Caso 4: (S1=1, S2=1) -> Ambos fechados
    set_manual_by_tag(&mut doc, "-S1", true);
    set_manual_by_tag(&mut doc, "-S2", true);
    step_simulation(&mut doc, 50);
    let km = doc
        .items
        .iter()
        .find(|i| i.tag == "-KM1" && i.tipo == 9000)
        .unwrap();
    assert!(doc.sim_active(km.id), "OR (1,1): Bobina DEVE LIGAR");
    assert_eq!(doc.sim_potential(w_out), Potential::L1);
}

#[test]
fn test_ladder_and_fluid_power_boolean_equivalence() {
    use cade_simu_rs::sim::set_manual_by_tag;

    // Monta a lógica clássica: Saída Y = (A || B) && !C
    // Representa:
    // - Ladder: Rung com dois contatos NA em paralelo (A || B) em série com um contato NF (!C) acionando bobina Y
    // - Eletropneumática: Válvula alternadora (OR) alimentando válvula direcional normalmente aberta/fechada e atuando solenoide Y
    // - Eletrohidráulica: Bloqueio de emergência (C) com duplo comando de partida (A ou B)
    let mut doc = SchematicDocument::default();
    let mut id_counter = 1;
    let mut next_id = || {
        let cur = id_counter;
        id_counter += 1;
        cur
    };

    let sym_l = find_symbol(3000).unwrap();
    doc.items.push(sym_l.instantiate(next_id(), 72, 10, 0));

    // Botão de Parada de Emergência C (NF, 8001) em (72, 15): pin 0 em (72, 15), pin 1 em (72, 27)
    let sym_c = find_symbol(8001).unwrap();
    let mut it_c = sym_c.instantiate(next_id(), 72, 15, 0);
    it_c.tag = "-C".to_string();
    doc.items.push(it_c);

    // Bloco paralelo (A || B) abaixo de C:
    // Botão A (NA, 8000) em (72, 35): pin 0 em (72, 35), pin 1 em (72, 47)
    let sym_a = find_symbol(8000).unwrap();
    let mut it_a = sym_a.instantiate(next_id(), 72, 35, 0);
    it_a.tag = "-A".to_string();
    doc.items.push(it_a);

    // Botão B (NA, 8000) em (84, 35): pin 0 em (84, 35), pin 1 em (84, 47)
    let sym_b = find_symbol(8000).unwrap();
    let mut it_b = sym_b.instantiate(next_id(), 84, 35, 0);
    it_b.tag = "-B".to_string();
    doc.items.push(it_b);

    // Bobina Solenoide / Saída Y (9000) em (72, 60): pin 0 em (72, 60), pin 1 em (72, 72)
    let sym_y = find_symbol(9000).unwrap();
    let mut it_y = sym_y.instantiate(next_id(), 72, 60, 0);
    it_y.tag = "-Y".to_string();
    doc.items.push(it_y);

    let sym_n = find_symbol(3001).unwrap();
    doc.items.push(sym_n.instantiate(next_id(), 72, 80, 2));

    // Fiação:
    // L -> C (72, 10) -> (72, 15)
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 72, 10, 72, 15));
    // C saída (72, 27) -> Nó J1 (72, 30)
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 72, 27, 72, 30));
    doc.items
        .push(SchematicItem::new_junction(next_id(), 72, 30));

    // Ramo A: (72, 30) -> (72, 35) e saída (72, 47) -> (72, 55)
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 72, 30, 72, 35));
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 72, 47, 72, 55));

    // Ramo B: (72, 30) -> (84, 30) -> (84, 35) e saída (84, 47) -> (84, 55) -> (72, 55)
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 72, 30, 84, 30));
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 84, 30, 84, 35));
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 84, 47, 84, 55));
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 84, 55, 72, 55));
    doc.items
        .push(SchematicItem::new_junction(next_id(), 72, 55));

    // J2 (72, 55) -> Y (72, 60) e Y (72, 72) -> N (72, 80)
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 72, 55, 72, 60));
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4009, 72, 72, 72, 80));

    // Valida todas as 8 combinações da tabela verdade: Y = (A || B) && !C_pressed
    for (a_val, b_val, c_pressed) in [
        (false, false, false), // 0, 0, !0 -> Y=0
        (false, true, false),  // 0, 1, !0 -> Y=1
        (true, false, false),  // 1, 0, !0 -> Y=1
        (true, true, false),   // 1, 1, !0 -> Y=1
        (false, false, true),  // 0, 0, !1 -> Y=0
        (false, true, true),   // 0, 1, !1 -> Y=0 (Emergência bloqueia mesmo com B ligado!)
        (true, false, true),   // 1, 0, !1 -> Y=0 (Emergência bloqueia mesmo com A ligado!)
        (true, true, true),    // 1, 1, !1 -> Y=0 (Emergência bloqueia mesmo com A e B ligados!)
    ] {
        set_manual_by_tag(&mut doc, "-A", a_val);
        set_manual_by_tag(&mut doc, "-B", b_val);
        set_manual_by_tag(&mut doc, "-C", c_pressed);
        step_simulation(&mut doc, 50);

        let expected = (a_val || b_val) && !c_pressed;
        let y_id = doc
            .items
            .iter()
            .find(|i| i.tag == "-Y" && i.tipo == 9000)
            .unwrap()
            .id;
        let y_active = doc.sim_active(y_id);
        assert_eq!(
            y_active, expected,
            "Falha na tabela verdade para A={}, B={}, C_pressed={}: esperado {}, obtido {}",
            a_val, b_val, c_pressed, expected, y_active
        );
    }
}

#[test]
fn test_smart_sequential_pin_dropdown_and_context_reading() {
    use cade_simu_rs::symbols::ContactKind;
    let (mut app, _) = CadeSimuApp::new();
    app.doc.items.clear();

    // Valida ContactKind e geração dos bornes conforme a norma
    assert_eq!(ContactKind::NO.generate_pins(1), vec!["13", "14"]);
    assert_eq!(ContactKind::NO.generate_pins(2), vec!["23", "24"]);
    assert_eq!(ContactKind::NC.generate_pins(1), vec!["11", "12"]);
    assert_eq!(ContactKind::NC.generate_pins(2), vec!["21", "22"]);

    // 1. Testa seleção sequencial em Botão NA (tipo 8000)
    let sym_btn_na = find_symbol(8000).expect("Botão NA deve existir no catálogo");
    let id_btn1 = 1001;
    let mut btn1 = sym_btn_na.instantiate(id_btn1, 100, 100, 0);
    btn1.tag = "-S1".to_string();
    app.doc.items.push(btn1);
    app.selected_id = Some(id_btn1);

    // Por padrão: 13 e 14
    let item = app.doc.items.iter().find(|i| i.id == id_btn1).unwrap();
    assert_eq!(item.pins[0], "13");
    assert_eq!(item.pins[1], "14");

    // Seleciona Sequência 2 -> 23 e 24
    let _ = app.update(Message::SelectPinSequence(2));
    let item = app.doc.items.iter().find(|i| i.id == id_btn1).unwrap();
    assert_eq!(
        item.pins[0], "23",
        "Borne 1 deve atualizar para 23 na sequência 2"
    );
    assert_eq!(
        item.pins[1], "24",
        "Borne 2 deve atualizar para 24 na sequência 2"
    );

    // Seleciona Sequência 3 -> 33 e 34
    let _ = app.update(Message::SelectPinSequence(3));
    let item = app.doc.items.iter().find(|i| i.id == id_btn1).unwrap();
    assert_eq!(item.pins[0], "33");
    assert_eq!(item.pins[1], "34");

    // Seleciona Sequência 1 -> volta para 13 e 14
    let _ = app.update(Message::SelectPinSequence(1));
    let item = app.doc.items.iter().find(|i| i.id == id_btn1).unwrap();
    assert_eq!(item.pins[0], "13");
    assert_eq!(item.pins[1], "14");

    // 2. Testa seleção sequencial em Botão NF (tipo 8001)
    let sym_btn_nf = find_symbol(8001).expect("Botão NF deve existir no catálogo");
    let id_btn2 = 1002;
    let mut btn2 = sym_btn_nf.instantiate(id_btn2, 120, 100, 0);
    btn2.tag = "-S1".to_string();
    app.doc.items.push(btn2);
    app.selected_id = Some(id_btn2);

    // Seleciona Sequência 2 em NF -> 21 e 22
    let _ = app.update(Message::SelectPinSequence(2));
    let item = app.doc.items.iter().find(|i| i.id == id_btn2).unwrap();
    assert_eq!(
        item.pins[0], "21",
        "Borne 1 de NF na sequência 2 deve ser 21"
    );
    assert_eq!(
        item.pins[1], "22",
        "Borne 2 de NF na sequência 2 deve ser 22"
    );

    // 3. Testa edição livre de texto nos bornes (inputs manuais continuam livres)
    let _ = app.update(Message::EditPin(0, "COM".to_string()));
    let _ = app.update(Message::EditPin(1, "NO".to_string()));
    let item = app.doc.items.iter().find(|i| i.id == id_btn2).unwrap();
    assert_eq!(item.pins[0], "COM");
    assert_eq!(item.pins[1], "NO");

    // 4. Testa Leitura Inteligente do Contexto para Contatos Auxiliares (-KM1)
    let sym_cont_na = find_symbol(7000).expect("Contato NA deve existir");
    let sym_cont_nf = find_symbol(7001).expect("Contato NF deve existir");

    let id_km_na1 = 2001;
    let mut cont_na1 = sym_cont_na.instantiate(id_km_na1, 200, 100, 0);
    cont_na1.tag = "-KM1".to_string();
    cont_na1.pins[0] = "13".to_string();
    cont_na1.pins[1] = "14".to_string(); // Sequência 1 ocupada
    app.doc.items.push(cont_na1);

    let id_km_nf1 = 2002;
    let mut cont_nf1 = sym_cont_nf.instantiate(id_km_nf1, 250, 100, 0);
    cont_nf1.tag = "-KM1".to_string();
    // Inicialmente também na sequência 1 (conflito)
    cont_nf1.pins[0] = "11".to_string();
    cont_nf1.pins[1] = "12".to_string();
    app.doc.items.push(cont_nf1);

    // Contexto: quais sequências de -KM1 estão ocupadas excluindo o próprio cont_nf1?
    let occupied = find_occupied_sequences_for_tag(&app.doc.items, "-KM1", Some(id_km_nf1));
    assert!(
        occupied.contains(&1),
        "Sequência 1 deve ser detectada como ocupada pelo cont_na1"
    );
    assert!(!occupied.contains(&2), "Sequência 2 deve estar livre");

    // Próxima sequência sugerida deve ser 2
    let suggested = (1..=9).find(|d| !occupied.contains(d)).unwrap();
    assert_eq!(
        suggested, 2,
        "A próxima sequência sugerida para -KM1 deve ser 2"
    );

    // Seleciona o contato NF com conflito e aplica a sequência sugerida 2
    app.selected_id = Some(id_km_nf1);
    let _ = app.update(Message::SelectPinSequence(suggested));
    let item = app.doc.items.iter().find(|i| i.id == id_km_nf1).unwrap();
    assert_eq!(item.pins[0], "21");
    assert_eq!(item.pins[1], "22");

    // Agora tanto 1 quanto 2 estão ocupados em -KM1
    let occupied_all = find_occupied_sequences_for_tag(&app.doc.items, "-KM1", None);
    assert!(occupied_all.contains(&1));
    assert!(occupied_all.contains(&2));
    let next_suggested = (1..=9).find(|d| !occupied_all.contains(d)).unwrap();
    assert_eq!(
        next_suggested, 3,
        "Novo contato de -KM1 deve sugerir sequência 3"
    );

    // 5. Testa Bobinas: bornes SEMPRE A1-A2 e sequenciamento de tags (-KM1, -KM2)
    let sym_coil = find_symbol(9000).expect("Bobina deve existir");
    // Bobina NUNCA deve ser classificada como contato com sequência aberta/fechada
    assert!(
        detect_contact_kind(9000, &sym_coil.pins).is_none(),
        "Bobina não deve ter ContactKind"
    );

    // Insere primeira bobina no Canvas -> deve ser -KM1 e bornes A1-A2
    let _ = app.update(Message::Canvas(CanvasMessage::PlaceComponent {
        tipo: 9000,
        x: 400,
        y: 100,
    }));
    let coil1 = app.doc.items.last().unwrap();
    assert_eq!(coil1.tag, "-KM1", "Primeira bobina deve receber tag -KM1");
    assert_eq!(coil1.pins[0], "A1", "Borne 1 da bobina DEVE ser A1");
    assert_eq!(coil1.pins[1], "A2", "Borne 2 da bobina DEVE ser A2");

    // Insere segunda bobina no Canvas -> deve ser -KM2 e bornes A1-A2
    let _ = app.update(Message::Canvas(CanvasMessage::PlaceComponent {
        tipo: 9000,
        x: 450,
        y: 100,
    }));
    let (coil2_id, coil2_tag, coil2_p0, coil2_p1) = {
        let c2 = app.doc.items.last().unwrap();
        (
            c2.id,
            c2.tag.clone(),
            c2.pins[0].clone(),
            c2.pins[1].clone(),
        )
    };
    assert_eq!(
        coil2_tag, "-KM2",
        "Segunda bobina deve receber tag sequencial -KM2"
    );
    assert_eq!(coil2_p0, "A1", "Borne 1 da segunda bobina DEVE ser A1");
    assert_eq!(coil2_p1, "A2", "Borne 2 da segunda bobina DEVE ser A2");

    // Insere botões/interruptores no Canvas -> -S1, -S2...
    let _ = app.update(Message::Canvas(CanvasMessage::PlaceComponent {
        tipo: 8000,
        x: 500,
        y: 100,
    }));
    let btn_s = app.doc.items.last().unwrap();
    assert!(
        btn_s.tag.starts_with("-S"),
        "Botão deve iniciar com prefixo -S"
    );

    // 6. Testa verificação de contato com e sem bobina
    let mut orphan_contact = sym_cont_na.instantiate(3001, 600, 100, 0);
    orphan_contact.tag = "-KM99".to_string(); // Bobina inexistente no esquema
    assert!(
        find_parent_apparatus(&app.doc.items, &orphan_contact).is_none(),
        "Contato -KM99 deve ser identificado como sem bobina"
    );

    let mut linked_contact = sym_cont_na.instantiate(3002, 650, 100, 0);
    linked_contact.tag = "-KM1".to_string(); // Bobina existente
    assert!(
        find_parent_apparatus(&app.doc.items, &linked_contact).is_some(),
        "Contato -KM1 deve encontrar a bobina -KM1"
    );

    // 7. Testa duplicação de aparelhos (Ctrl+D)
    app.selected_id = Some(coil2_id); // seleciona -KM2
    let _ = app.update(Message::Duplicate);
    let duplicated_coil = app.doc.items.last().unwrap();
    assert_eq!(
        duplicated_coil.tag, "-KM3",
        "Duplicar bobina -KM2 deve gerar -KM3"
    );
    assert_eq!(duplicated_coil.pins[0], "A1");
    assert_eq!(duplicated_coil.pins[1], "A2");

    // 8. Renderiza a view garantindo que o pick_list e os avisos são gerados sem pânico
    let _ = app.view();
}

#[test]
fn test_drag_move_single_undo_step() {
    let (mut app, _) = CadeSimuApp::new();
    app.doc.items.clear();

    // 1. Posiciona componente inicial em (100, 100)
    let _ = app.update(Message::Canvas(CanvasMessage::PlaceComponent {
        tipo: 9000,
        x: 100,
        y: 100,
    }));
    let item_id = app.doc.items[0].id;
    assert_eq!(app.doc.items[0].x1, 100);
    assert_eq!(app.doc.items[0].y1, 100);
    let undo_len_after_place = app.undo_stack.len();

    // 2. Simula o arraste suave do componente passando por múltiplos pontos da grade até (180, 100)
    let intermediate_steps = [110, 120, 130, 140, 150, 160, 170, 180];
    for &step_x in &intermediate_steps {
        let _ = app.update(Message::Canvas(CanvasMessage::MoveItem {
            id: item_id,
            x1: step_x,
            y1: 100,
        }));
        // Durante o arraste contínuo, a pilha de undo NÃO deve acumular passos intermediários!
        assert_eq!(
            app.undo_stack.len(),
            undo_len_after_place,
            "Passos intermediários de arraste não podem poluir a pilha de Ctrl+Z"
        );
        assert_eq!(app.doc.items[0].x1, step_x);
    }

    // 3. Usuário solta o botão do mouse -> EndMoveItem
    let _ = app.update(Message::Canvas(CanvasMessage::EndMoveItem { id: item_id }));

    // Apenas UM snapshot único deve ser adicionado ao histórico
    assert_eq!(
        app.undo_stack.len(),
        undo_len_after_place + 1,
        "Apenas uma transação de movimento deve ser registrada ao soltar o mouse"
    );
    assert_eq!(app.doc.items[0].x1, 180);

    // 4. Um ÚNICO Ctrl+Z (Undo) deve retornar o elemento diretamente da posição final (180, 100) para a inicial (100, 100)
    let _ = app.update(Message::Undo);
    assert_eq!(
        app.doc.items[0].x1, 100,
        "Um único Ctrl+Z deve retornar o elemento diretamente para o ponto inicial (100, 100)"
    );
    assert_eq!(app.doc.items[0].y1, 100);

    // 5. Um ÚNICO Ctrl+Y (Redo) deve mover de volta para a posição final (180, 100)
    let _ = app.update(Message::Redo);
    assert_eq!(
        app.doc.items[0].x1, 180,
        "Um único Ctrl+Y deve avançar o elemento para a posição final (180, 100)"
    );
    assert_eq!(app.doc.items[0].y1, 100);
}

#[test]
fn test_drag_move_cancel_and_wire_tracking() {
    let (mut app, _) = CadeSimuApp::new();
    app.doc.items.clear();

    // Insere componente e fio conectado
    let _ = app.update(Message::Canvas(CanvasMessage::PlaceComponent {
        tipo: 9000,
        x: 100,
        y: 100,
    }));
    let comp_id = app.doc.items[0].id;
    let _ = app.update(Message::Canvas(CanvasMessage::AddWire {
        tipo: 4000,
        x1: 100,
        y1: 100,
        x2: 100,
        y2: 50,
    }));
    let wire_id = app.doc.items.iter().find(|i| i.is_wire()).unwrap().id;
    let initial_undo_len = app.undo_stack.len();

    // 1. Arraste cancelado via ESC / Botão Direito (CancelAction)
    let _ = app.update(Message::Canvas(CanvasMessage::MoveItem {
        id: comp_id,
        x1: 140,
        y1: 100,
    }));
    assert_eq!(
        app.doc.items.iter().find(|i| i.id == comp_id).unwrap().x1,
        140
    );
    let _ = app.update(Message::Canvas(CanvasMessage::CancelAction));
    assert_eq!(
        app.doc.items.iter().find(|i| i.id == comp_id).unwrap().x1,
        100,
        "Cancelar deve reverter o elemento à posição de origem imediatamente"
    );
    assert_eq!(
        app.undo_stack.len(),
        initial_undo_len,
        "Cancelar arraste não deve criar entrada na pilha de undo"
    );

    // 2. Arraste concluído com fio conectado
    let _ = app.update(Message::Canvas(CanvasMessage::MoveItem {
        id: comp_id,
        x1: 130,
        y1: 100,
    }));
    let _ = app.update(Message::Canvas(CanvasMessage::EndMoveItem { id: comp_id }));
    assert_eq!(app.undo_stack.len(), initial_undo_len + 1);

    // Fio deve ter acompanhado o componente para x1 = 130
    let wire_moved = app.doc.items.iter().find(|i| i.id == wire_id).unwrap();
    assert_eq!(wire_moved.x1, 130);

    // Desfazer (Ctrl+Z) restaura ambos para x1 = 100 em um passo
    let _ = app.update(Message::Undo);
    let comp_undone = app.doc.items.iter().find(|i| i.id == comp_id).unwrap();
    let wire_undone = app.doc.items.iter().find(|i| i.id == wire_id).unwrap();
    assert_eq!(comp_undone.x1, 100);
    assert_eq!(wire_undone.x1, 100);

    // 3. Clique simples sem deslocamento não deve criar histórico de undo
    let pre_click_undo = app.undo_stack.len();
    let _ = app.update(Message::Canvas(CanvasMessage::EndMoveItem { id: comp_id }));
    assert_eq!(
        app.undo_stack.len(),
        pre_click_undo,
        "Clique sem deslocamento não deve adicionar entrada na pilha de undo"
    );
}

#[test]
fn test_teste_cad_loading_and_mushroom_button_dj() {
    let cad_path = "Projetos/teste.cad";
    let content =
        std::fs::read_to_string(cad_path).expect("teste.cad deve existir na pasta Projetos");
    let doc =
        cade_simu_rs::io::parse_cad_str(&content).expect("teste.cad deve carregar com sucesso");

    // Verifica que o elemento 139 existe no arquivo e foi decodificado como 8008 (Interruptor NA / DJ)
    let item_dj = doc
        .items
        .iter()
        .find(|i| i.id == 139)
        .expect("Item #139 (DJ) deve estar presente no teste.cad");
    assert_eq!(
        item_dj.tipo, 8008,
        "Item #139 deve ser tipo 8008 (Contacto interruptor NA - DJ)"
    );
    assert_eq!(item_dj.tag.trim(), "DJ", "Tag do elemento #139 deve ser DJ");
    assert_eq!(item_dj.pins[0].trim(), "13", "Pino 0 deve ser 13");
    assert_eq!(item_dj.pins[1].trim(), "14", "Pino 1 deve ser 14");

    // Verifica que o símbolo existe no catálogo do programa com primitivas vetoriais completas
    let sym = find_symbol(8008).expect("Símbolo 8008 deve existir no catálogo de símbolos");
    assert_eq!(
        sym.category,
        cade_simu_rs::symbols::SymbolCategory::PushbuttonsActuators
    );
    assert!(
        sym.name.contains("Interruptor") || sym.name.contains("Chave"),
        "8008 deve ser Interruptor / Chave"
    );
    assert!(
        !sym.idle_cmds.is_empty(),
        "Comandos idle não podem ser vazios"
    );
    assert!(
        !sym.active_cmds.is_empty(),
        "Comandos active não podem ser vazios"
    );
    // Verifica que contém o comando do traço degrau de retenção do interruptor (LINE,-030,+020,-030,+040)
    assert!(
        sym.idle_cmds
            .iter()
            .any(|cmd| cmd.contains("-030,+020,-030,+040")),
        "Deve conter degrau retentivo do interruptor"
    );
    assert_eq!(sym.pins.len(), 2, "Interruptor NA tem 2 pinos");
}

#[test]
fn test_actuators_and_switches_catalog_completeness() {
    let expected_types = [
        8000, 8001, 8002, 8003, // Botões pulsadores
        8004, 8005, 8006, 8007, // Cogumelos pulsadores
        8008, 8009, 8010, 8011, // Chaves / Interruptores retentivos (8008 = DJ)
        8012, 8013, 8014, 8015, // Fim de Curso pulsadores
        8016, 8017, 8018, 8019, // Relé Térmico auxiliares
        8020, 8021, 8022, // Chaves Seletoras I-0-II, I-II-III, Inversora
        8023, // Botão Pulsador 2D
        8030, 8031, 8032, 8033,
        8034, // Botões de Parada de Emergência Retentivos com travas
    ];

    for &tipo in &expected_types {
        let sym =
            find_symbol(tipo).unwrap_or_else(|| panic!("Tipo {} deve existir no catálogo", tipo));
        assert!(
            !sym.idle_cmds.is_empty(),
            "Tipo {} deve ter comandos idle",
            tipo
        );
        assert!(
            !sym.active_cmds.is_empty(),
            "Tipo {} deve ter comandos active",
            tipo
        );
        if tipo != 8023 {
            assert!(
                !sym.pins.is_empty(),
                "Tipo {} deve ter pinos declarados",
                tipo
            );
        }
    }
}

#[test]
fn test_actuator_switches_latching_and_momentary_distinction() {
    use cade_simu_rs::sim::{
        is_momentary_button, press_component_on_sim, release_component_on_sim,
    };

    // 1. Verifica momentâneos (pulsadores simples, cogumelos simples 8004..=8007, fins de curso 8012..=8015)
    for &m in &[
        8000, 8001, 8002, 8003, 8004, 8005, 8006, 8007, 8012, 8013, 8014, 8015, 8023,
    ] {
        assert!(
            is_momentary_button(m),
            "Tipo {} deve ser momentâneo (diferencial/pulsador)",
            m
        );
    }
    // 2. Verifica retentivos / bi-estáveis (interruptores 8008..=8011, chaves 8020..=8022, emergências 8030..=8034)
    for &b in &[
        8008, 8009, 8010, 8011, 8020, 8021, 8022, 8030, 8031, 8032, 8033, 8034,
    ] {
        assert!(
            !is_momentary_button(b),
            "Tipo {} deve ser bi-estável / retentivo",
            b
        );
    }

    let mut doc = cade_simu_rs::model::SchematicDocument::default();
    doc.items.clear();

    // 3. Insere um botão pulsador momentâneo (8000), um interruptor retentivo (8008) e uma emergência NF (8030)
    let mut btn_mom = cade_simu_rs::model::SchematicItem::new_symbol(1, 8000, 50, 50, "-S1");
    btn_mom.pins[0] = "13".to_string();
    btn_mom.pins[1] = "14".to_string();
    let mut sw_ret = cade_simu_rs::model::SchematicItem::new_symbol(2, 8008, 100, 50, "-DJ");
    sw_ret.pins[0] = "13".to_string();
    sw_ret.pins[1] = "14".to_string();
    let mut emg_nc = cade_simu_rs::model::SchematicItem::new_symbol(3, 8030, 150, 50, "-S0");
    emg_nc.pins[0] = "11".to_string();
    emg_nc.pins[1] = "12".to_string();

    doc.items.push(btn_mom);
    doc.items.push(sw_ret);
    doc.items.push(emg_nc);

    // Pressiona o botão momentâneo -> fica ativo
    press_component_on_sim(&mut doc, 1);
    assert!(
        doc.sim_latched(1),
        "Botão momentâneo deve ficar ativo ao pressionar"
    );

    // Solta o botão momentâneo -> volta a repouso
    release_component_on_sim(&mut doc, 1);
    assert!(
        !doc.sim_latched(1),
        "Botão momentâneo deve soltar ao liberar o mouse"
    );

    // Pressiona a chave interruptor (8008) -> retém ativo
    press_component_on_sim(&mut doc, 2);
    assert!(doc.sim_latched(2), "Chave interruptor deve reter ativo");

    // Solta a chave interruptor -> DEVE CONTINUAR ATIVO (retentivo/bi-estável)
    release_component_on_sim(&mut doc, 2);
    assert!(
        doc.sim_latched(2),
        "Chave interruptor deve permanecer retida após soltar mouse"
    );

    // Segundo clique na chave interruptor -> desativa (alterna)
    press_component_on_sim(&mut doc, 2);
    assert!(
        !doc.sim_latched(2),
        "Segundo clique desativa chave interruptor"
    );

    // Pressiona o botão de emergência retentivo (8030)
    press_component_on_sim(&mut doc, 3);
    assert!(
        doc.sim_latched(3),
        "Botão de emergência deve travar ao pressionar"
    );
    release_component_on_sim(&mut doc, 3);
    assert!(
        doc.sim_latched(3),
        "Botão de emergência retém travado ao soltar mouse"
    );
    press_component_on_sim(&mut doc, 3);
    assert!(
        !doc.sim_latched(3),
        "Segundo clique destrava o botão de emergência"
    );
}

#[test]
fn test_canvas_selection_prioritizes_symbols_over_wires() {
    use cade_simu_rs::canvas_view::SchematicCanvas;

    let mut doc = cade_simu_rs::model::SchematicDocument::default();
    doc.items.clear();

    // Cria símbolo DJ (8008) em (50, 50)
    let sym_dj = find_symbol(8008).unwrap();
    let item_dj = sym_dj.instantiate(100, 50, 50, 0);
    let wp = sym_dj.world_pins(&item_dj);

    // Cria fios conectados exatamente nos pinos do símbolo
    let wire1 = cade_simu_rs::model::SchematicItem::new_wire(
        201,
        4000,
        wp[0].2 .0,
        wp[0].2 .1 - 20,
        wp[0].2 .0,
        wp[0].2 .1,
    );
    let wire2 = cade_simu_rs::model::SchematicItem::new_wire(
        202,
        4000,
        wp[1].2 .0,
        wp[1].2 .1,
        wp[1].2 .0,
        wp[1].2 .1 + 20,
    );

    // Fio horizontal cruzando por trás do símbolo
    let wire3 = cade_simu_rs::model::SchematicItem::new_wire(203, 4000, 30, 55, 70, 55);

    doc.items.push(wire1);
    doc.items.push(wire2);
    doc.items.push(wire3);
    doc.items.push(item_dj);

    let dummy_selected_ids = std::collections::HashSet::new();
    let canvas = SchematicCanvas {
        doc: &doc,
        tool: cade_simu_rs::EditorTool::Select,
        selected_id: None,
        selected_ids: &dummy_selected_ids,
        is_shift_pressed: false,
        is_ctrl_pressed: false,
        is_alt_pressed: false,
        sim_running: false,
        show_grid: true,
        snap_to_grid: true,
        grid_density: cade_simu_rs::model::GridDensity::Medium,
        show_texts: true,
        show_terminals: true,
        show_cajetin: true,
        show_full_crosshair: false,
        show_text_grid: false,
        show_alignment_guides: false,
        keep_headers_visible: false,
        show_cell_bounds: false,
        is_dark: false,
        auto_connect: true,
        auto_disconnect: false,
        zoom: 1.0,
        pan: iced::Vector::ZERO,
        placement_orientation: 0,
        dragging_symbol: None,
        dragging_canonical_symbol: None,
        is_blocked: false,
    };

    // Clicar no corpo do símbolo deve retornar o ID do símbolo (100) e JAMAIS o ID de um fio
    let hit = canvas.hit_test(50, 55);
    assert_eq!(
        hit,
        Some(100),
        "Hit test no centro do símbolo deve selecionar o símbolo e não fios adjacentes"
    );

    let hit_pin = canvas.hit_test(wp[0].2 .0, wp[0].2 .1);
    assert_eq!(
        hit_pin,
        Some(100),
        "Hit test no pino do símbolo deve priorizar o símbolo sobre o fio conectado"
    );

    // Clicar longe do símbolo, em ponto exclusivo do fio
    let hit_wire = canvas.hit_test(wp[0].2 .0, wp[0].2 .1 - 15);
    assert_eq!(
        hit_wire,
        Some(201),
        "Hit test longe do símbolo deve selecionar o fio"
    );
}

#[test]
fn test_crossover_switch_and_limit_switches_conduction() {
    use cade_simu_rs::model::Potential;
    use cade_simu_rs::sim::step_simulation;

    let mut doc = cade_simu_rs::model::SchematicDocument::default();
    doc.items.clear();

    let sym_l1 = find_symbol(3000).unwrap();
    let sym_n = find_symbol(3001).unwrap();
    let sym_cross = find_symbol(8022).unwrap();

    let src_l1 = sym_l1.instantiate(1, 50, 40, 0);
    let src_n = sym_n.instantiate(2, 56, 40, 0);
    let cross = sym_cross.instantiate(3, 50, 50, 0);

    let wp_l1 = sym_l1.world_pins(&src_l1);
    let wp_n = sym_n.world_pins(&src_n);
    let wp_cross = sym_cross.world_pins(&cross);

    let pt_l1 = wp_l1[0].2;
    let pt_n = wp_n[0].2;
    let pt_c0 = wp_cross[0].2; // pino 0 ("1")
    let pt_c1 = wp_cross[1].2; // pino 1 ("3")
    let pt_c2 = wp_cross[2].2; // pino 2 ("2")
    let pt_c3 = wp_cross[3].2; // pino 3 ("4")

    doc.items.push(src_l1);
    doc.items.push(src_n);
    doc.items.push(cross);

    // Fios conectando L1 ao pino 0 e N ao pino 1
    doc.items.push(cade_simu_rs::model::SchematicItem::new_wire(
        4, 4000, pt_l1.0, pt_l1.1, pt_c0.0, pt_c0.1,
    ));
    doc.items.push(cade_simu_rs::model::SchematicItem::new_wire(
        5, 4002, pt_n.0, pt_n.1, pt_c1.0, pt_c1.1,
    ));

    // Cabos de saída a partir dos pinos 2 e 3
    doc.items.push(cade_simu_rs::model::SchematicItem::new_wire(
        6,
        4000,
        pt_c2.0,
        pt_c2.1,
        pt_c2.0,
        pt_c2.1 + 20,
    ));
    doc.items.push(cade_simu_rs::model::SchematicItem::new_wire(
        7,
        4002,
        pt_c3.0,
        pt_c3.1,
        pt_c3.0,
        pt_c3.1 + 20,
    ));

    // Executa simulação em repouso (cross.sim_manual_latched = false)
    // No repouso: conexão direta -> pino 0 (L1) liga ao pino 2 (fio 6), pino 1 (N) liga ao pino 3 (fio 7)
    step_simulation(&mut doc, 50);

    let pot_out_left = doc.sim_potential(6);
    let pot_out_right = doc.sim_potential(7);
    assert_eq!(
        pot_out_left,
        Potential::L1,
        "Em repouso, saída esquerda deve ser L1"
    );
    assert_eq!(
        pot_out_right,
        Potential::N,
        "Em repouso, saída direita deve ser N"
    );

    // Ativa a chave inversora (cross.sim_manual_latched = true)
    // No ativo: conexão cruzada -> pino 0 (L1) liga ao pino 3 (fio 7), pino 1 (N) liga ao pino 2 (fio 6)
    doc.set_sim_latched(3, true);
    step_simulation(&mut doc, 50);

    let pot_out_left_inv = doc.sim_potential(6);
    let pot_out_right_inv = doc.sim_potential(7);
    assert_eq!(
        pot_out_left_inv,
        Potential::N,
        "Ao inverter, saída esquerda deve trocar para N"
    );
    assert_eq!(
        pot_out_right_inv,
        Potential::L1,
        "Ao inverter, saída direita deve trocar para L1"
    );
}

#[test]
fn test_limit_switches_and_thermal_aux_contacts() {
    use cade_simu_rs::model::Potential;
    use cade_simu_rs::sim::step_simulation;

    let mut doc = cade_simu_rs::model::SchematicDocument::default();
    doc.items.clear();

    // 1. Fonte L1 em (50, 40)
    let sym_l1 = find_symbol(3000).unwrap();
    let src_l1 = sym_l1.instantiate(1, 50, 40, 0);
    let pt_l1 = sym_l1.world_pins(&src_l1)[0].2;
    doc.items.push(src_l1);

    // 2. Fim de curso NA (8012) e NF (8013)
    let sym_no = find_symbol(8012).unwrap();
    let sym_nc = find_symbol(8013).unwrap();
    let lim_no = sym_no.instantiate(2, 50, 50, 0);
    let lim_nc = sym_nc.instantiate(3, 80, 50, 0);

    let wp_no = sym_no.world_pins(&lim_no);
    let wp_nc = sym_nc.world_pins(&lim_nc);

    doc.items.push(lim_no);
    doc.items.push(lim_nc);

    // Fios de entrada: L1 -> pino 0 de lim_no e lim_nc
    doc.items.push(cade_simu_rs::model::SchematicItem::new_wire(
        4,
        4000,
        pt_l1.0,
        pt_l1.1,
        wp_no[0].2 .0,
        wp_no[0].2 .1,
    ));
    doc.items.push(cade_simu_rs::model::SchematicItem::new_wire(
        5,
        4000,
        pt_l1.0,
        pt_l1.1,
        wp_nc[0].2 .0,
        wp_nc[0].2 .1,
    ));

    // Fios de saída: pino 1 -> carga
    doc.items.push(cade_simu_rs::model::SchematicItem::new_wire(
        6,
        4000,
        wp_no[1].2 .0,
        wp_no[1].2 .1,
        wp_no[1].2 .0,
        wp_no[1].2 .1 + 20,
    ));
    doc.items.push(cade_simu_rs::model::SchematicItem::new_wire(
        7,
        4000,
        wp_nc[1].2 .0,
        wp_nc[1].2 .1,
        wp_nc[1].2 .0,
        wp_nc[1].2 .1 + 20,
    ));

    // Em repouso: NO aberto (saída None), NC fechado (saída L1)
    step_simulation(&mut doc, 50);
    assert_eq!(doc.sim_potential(6), Potential::None);
    assert_eq!(doc.sim_potential(7), Potential::L1);

    // Ativa ambos os fins de curso: NO fecha (saída L1), NC abre (saída None)
    doc.set_sim_latched(2, true);
    doc.set_sim_latched(3, true);
    step_simulation(&mut doc, 50);
    assert_eq!(doc.sim_potential(6), Potential::L1);
    assert_eq!(doc.sim_potential(7), Potential::None);

    // 3. Testa atuação do relé térmico pai (6007) sobre contato auxiliar térmico duplo (8018)
    let sym_f = find_symbol(6007).unwrap();
    let sym_f_aux = find_symbol(8018).unwrap();
    let mut r_term = sym_f.instantiate(10, 120, 50, 0);
    r_term.tag = "-F1".to_string();
    let mut aux_term = sym_f_aux.instantiate(11, 150, 50, 0);
    aux_term.tag = "-F1".to_string();

    let wp_aux = sym_f_aux.world_pins(&aux_term);
    // pino 0 ("95") e pino 2 ("96") formam o contato NF
    // pino 1 ("97") e pino 3 ("98") formam o contato NA
    doc.items.push(r_term);
    doc.items.push(aux_term);

    // Alimenta 95 (NF) e 97 (NA) com L1
    doc.items.push(cade_simu_rs::model::SchematicItem::new_wire(
        12,
        4000,
        pt_l1.0,
        pt_l1.1,
        wp_aux[0].2 .0,
        wp_aux[0].2 .1,
    ));
    doc.items.push(cade_simu_rs::model::SchematicItem::new_wire(
        13,
        4000,
        pt_l1.0,
        pt_l1.1,
        wp_aux[1].2 .0,
        wp_aux[1].2 .1,
    ));
    // Saídas de 96 (NF) e 98 (NA)
    doc.items.push(cade_simu_rs::model::SchematicItem::new_wire(
        14,
        4000,
        wp_aux[2].2 .0,
        wp_aux[2].2 .1,
        wp_aux[2].2 .0,
        wp_aux[2].2 .1 + 20,
    ));
    doc.items.push(cade_simu_rs::model::SchematicItem::new_wire(
        15,
        4000,
        wp_aux[3].2 .0,
        wp_aux[3].2 .1,
        wp_aux[3].2 .0,
        wp_aux[3].2 .1 + 20,
    ));

    // Em repouso do relé térmico: 95-96 conduz (fio 14 = L1), 97-98 aberto (fio 15 = None)
    step_simulation(&mut doc, 50);
    assert_eq!(doc.sim_potential(14), Potential::L1);
    assert_eq!(doc.sim_potential(15), Potential::None);

    // Dispara o relé térmico principal -F1 -> o contato auxiliar 8018 comuta automaticamente via tag!
    doc.set_sim_latched(10, true);
    step_simulation(&mut doc, 50);
    assert_eq!(
        doc.sim_potential(14),
        Potential::None,
        "95-96 abre quando -F1 desarma"
    );
    assert_eq!(
        doc.sim_potential(15),
        Potential::L1,
        "97-98 fecha quando -F1 desarma"
    );
}

#[test]
fn test_dock_panels_physical_collapse_and_expand() {
    let (mut app, _) = CadeSimuApp::new();

    // Inicialmente ambos os painéis estão expandidos (288px)
    assert!(!app.left_dock_collapsed);
    assert!(!app.right_dock_collapsed);
    assert_eq!(app.layout.left_width, 288.0);
    assert_eq!(app.layout.right_width, 288.0);

    // Renderiza a interface normalmente
    let _ = app.view();

    // 1. Recolhe o painel esquerdo (Elementos e Projetos)
    let _ = app.update(Message::ToggleLeftDockCollapse);
    assert!(app.left_dock_collapsed);
    let _ = app.view();

    // 2. Recolhe o painel direito (Propriedades do Elemento)
    let _ = app.update(Message::ToggleRightDockCollapse);
    assert!(app.right_dock_collapsed);
    let _ = app.view();

    // 3. Expande o painel esquerdo novamente
    let _ = app.update(Message::ToggleLeftDockCollapse);
    assert!(!app.left_dock_collapsed);
    let _ = app.view();

    // 4. Expande o painel direito novamente
    let _ = app.update(Message::ToggleRightDockCollapse);
    assert!(!app.right_dock_collapsed);
    let _ = app.view();
}

#[test]
fn test_dock_panels_resize_drag_and_clamp() {
    let (mut app, _) = CadeSimuApp::new();

    // 1. Testa redimensionamento do painel esquerdo
    let _ = app.update(Message::LayoutMsg(cade_simu_rs::layout::Msg::DragStart(
        cade_simu_rs::layout::Side::Left,
    )));
    assert_eq!(app.layout.drag, Some(cade_simu_rs::layout::Side::Left));

    // Move para 350px (dentro do intervalo 160..=480)
    let _ = app.update(Message::LayoutMsg(cade_simu_rs::layout::Msg::DragMove(
        350.0,
    )));
    assert_eq!(app.layout.left_width, 350.0);

    // Clamping inferior: tenta mover para 50px -> limita a 160px
    let _ = app.update(Message::LayoutMsg(cade_simu_rs::layout::Msg::DragMove(
        50.0,
    )));
    assert_eq!(app.layout.left_width, 160.0);

    // Clamping superior: tenta mover para 600px -> limita a 480px
    let _ = app.update(Message::LayoutMsg(cade_simu_rs::layout::Msg::DragMove(
        600.0,
    )));
    assert_eq!(app.layout.left_width, 480.0);

    // Solta o botão do mouse
    let _ = app.update(Message::LayoutMsg(cade_simu_rs::layout::Msg::DragEnd));
    assert_eq!(app.layout.drag, None);

    // 2. Testa redimensionamento do painel direito (largura da janela = 1420px)
    let _ = app.update(Message::LayoutMsg(cade_simu_rs::layout::Msg::DragStart(
        cade_simu_rs::layout::Side::Right,
    )));
    assert_eq!(app.layout.drag, Some(cade_simu_rs::layout::Side::Right));

    // Posiciona o cursor em x = 1100 -> right_width = 1420 - 1100 = 320px
    let _ = app.update(Message::LayoutMsg(cade_simu_rs::layout::Msg::DragMove(
        1100.0,
    )));
    assert_eq!(app.layout.right_width, 320.0);

    // Clamping inferior do painel direito: x = 1380 -> 1420 - 1380 = 40px -> limita a 160px
    let _ = app.update(Message::LayoutMsg(cade_simu_rs::layout::Msg::DragMove(
        1380.0,
    )));
    assert_eq!(app.layout.right_width, 160.0);

    // Clamping superior do painel direito: x = 800 -> 1420 - 800 = 620px -> limita a 480px
    let _ = app.update(Message::LayoutMsg(cade_simu_rs::layout::Msg::DragMove(
        800.0,
    )));
    assert_eq!(app.layout.right_width, 480.0);

    let _ = app.update(Message::LayoutMsg(cade_simu_rs::layout::Msg::DragEnd));
    assert_eq!(app.layout.drag, None);

    // 3. Testa redimensionamento da janela
    let _ = app.update(Message::LayoutMsg(
        cade_simu_rs::layout::Msg::WindowResized(1600.0),
    ));
    assert_eq!(app.layout.window_w, 1600.0);

    // 4. Renderiza view com painéis redimensionados
    let _ = app.view();
}

#[test]
fn test_toolbar_responsive_and_icon_buttons() {
    let (mut app, _) = CadeSimuApp::new();

    // Renderiza toolbar com tema padrão
    let _ = app.view_toolbar();

    // Alterna tema via CycleTheme (o botão de tema agora é ícone puro)
    assert_eq!(app.theme_mode, ThemeMode::Dark);
    let _ = app.update(Message::CycleTheme);
    assert_eq!(app.theme_mode, ThemeMode::System);
    let _ = app.view_toolbar();

    // Testa controles de simulação no toolbar
    assert!(!app.sim_running);
    let _ = app.update(Message::ToggleSimulation);
    assert!(app.sim_running);
    let _ = app.view_toolbar();

    // Stop zera a simulação (botão stop sem texto)
    let _ = app.update(Message::StopSimulation);
    assert!(!app.sim_running);
    let _ = app.view_toolbar();
}

#[test]
fn test_menu_overlay_stays_open_over_canvas() {
    use iced::widget::canvas::Program;

    let (mut app, _) = CadeSimuApp::new();

    // 1. Abre o menu Arquivo
    let _ = app.update(Message::ToggleMenu(Some(ActiveMenu::Arquivo)));
    assert_eq!(app.open_menu, Some(ActiveMenu::Arquivo));

    // 2. Simula o cursor do mouse se movendo sobre a área de altura do canvas
    let _ = app.update(Message::Canvas(CanvasMessage::CursorMoved(120, 240)));
    assert_eq!(
        app.open_menu,
        Some(ActiveMenu::Arquivo),
        "Cursor se movendo sobre a altura do canvas NÃO pode fechar o menu flutuante!"
    );

    // 3. Renderiza a interface com o menu aberto e verifica que o canvas fica bloqueado
    let _ = app.view();

    let dummy_selected_ids = std::collections::HashSet::new();
    let canvas_blocked = cade_simu_rs::canvas_view::SchematicCanvas {
        doc: &app.doc,
        tool: app.tool.clone(),
        selected_id: None,
        selected_ids: &dummy_selected_ids,
        is_shift_pressed: false,
        is_ctrl_pressed: false,
        is_alt_pressed: false,
        sim_running: false,
        show_grid: true,
        snap_to_grid: true,
        grid_density: app.grid_density,
        show_texts: true,
        show_terminals: true,
        show_cajetin: true,
        show_full_crosshair: false,
        show_text_grid: false,
        show_alignment_guides: false,
        keep_headers_visible: false,
        show_cell_bounds: false,
        is_dark: true,
        auto_connect: true,
        auto_disconnect: true,
        zoom: 1.0,
        pan: iced::Vector::ZERO,
        placement_orientation: 0,
        dragging_symbol: None,
        dragging_canonical_symbol: None,
        is_blocked: app.open_menu.is_some(),
    };

    let mut c_state = cade_simu_rs::canvas_view::CanvasInteraction::default();
    let (status, msg) = canvas_blocked.update(
        &mut c_state,
        iced::widget::canvas::Event::Mouse(iced::mouse::Event::CursorMoved {
            position: iced::Point::new(100.0, 300.0),
        }),
        iced::Rectangle::new(iced::Point::ORIGIN, iced::Size::new(800.0, 600.0)),
        iced::mouse::Cursor::Available(iced::Point::new(100.0, 300.0)),
    );

    assert_eq!(status, iced::widget::canvas::event::Status::Ignored);
    assert!(msg.is_none(), "Canvas bloqueado não gera mensagens");

    // 4. Clicar fora ou fechar menu encerra o overlay normalmente
    let _ = app.update(Message::ToggleMenu(None));
    assert_eq!(app.open_menu, None);
}

#[test]
fn test_circuit_breakers_and_fuses_trip_on_short_circuit() {
    use cade_simu_rs::model::{Potential, SchematicDocument, SchematicItem};
    use cade_simu_rs::sim::{step_simulation, toggle_component_on_click};

    let mut doc = SchematicDocument::default();
    doc.items.clear();

    // 1. Testa Disjuntor Tripolar 6003 (-Q1) com curto-circuito entre fases L1 e L2
    let sym_src3 = find_symbol(3004).unwrap();
    let sym_q3p = find_symbol(6003).unwrap();

    let src = sym_src3.instantiate(1, 20, 20, 0);
    let wp_src = sym_src3.world_pins(&src);

    let mut q1 = sym_q3p.instantiate(2, 20, 40, 0);
    q1.tag = "-Q1".to_string();
    let wp_q1 = sym_q3p.world_pins(&q1);

    doc.items.push(src);
    doc.items.push(q1);

    // Fios de alimentação: L1, L2, L3 para entradas do disjuntor (pinos 0, 1, 2)
    doc.items.push(SchematicItem::new_wire(
        3,
        4000,
        wp_src[0].2 .0,
        wp_src[0].2 .1,
        wp_q1[0].2 .0,
        wp_q1[0].2 .1,
    ));
    doc.items.push(SchematicItem::new_wire(
        4,
        4000,
        wp_src[1].2 .0,
        wp_src[1].2 .1,
        wp_q1[1].2 .0,
        wp_q1[1].2 .1,
    ));
    doc.items.push(SchematicItem::new_wire(
        5,
        4000,
        wp_src[2].2 .0,
        wp_src[2].2 .1,
        wp_q1[2].2 .0,
        wp_q1[2].2 .1,
    ));

    // Fio de curto-circuito franco entre saída L1 (pino 3) e saída L2 (pino 4)
    let sc_wire_id = 6;
    doc.items.push(SchematicItem::new_wire(
        sc_wire_id,
        4000,
        wp_q1[3].2 .0,
        wp_q1[3].2 .1,
        wp_q1[4].2 .0,
        wp_q1[4].2 .1,
    ));

    // Em repouso (disjuntor aberto), nada conduz a jusante e o disjuntor está sem falha
    step_simulation(&mut doc, 50);
    assert!(!doc.sim_latched(2));
    assert!(!doc.sim_fault(2));

    // Fecha o disjuntor -Q1 -> a corrente de curto entre L1 e L2 faz o disjuntor DESARMAR (trip magnético)!
    doc.set_sim_latched(2, true);
    step_simulation(&mut doc, 50);

    assert!(
        !doc.sim_latched(2),
        "O disjuntor -Q1 deve desarmar (abrir) por curto-circuito!"
    );
    assert!(
        doc.sim_fault(2),
        "O disjuntor -Q1 deve registrar estado de trip/falha!"
    );
    assert!(
        !doc.sim_active(2),
        "O disjuntor desarmado não deve conduzir!"
    );

    // Como o disjuntor desarmou, as saídas são desenergizadas, extinguindo o curto
    let sc_wire_pot = doc.sim_potential(sc_wire_id);
    assert_ne!(
        sc_wire_pot,
        Potential::ShortCircuit,
        "A abertura do disjuntor deve extinguir o curto a jusante"
    );

    // 2. Testa rearme do disjuntor após remoção do curto
    doc.items.retain(|i| i.id != sc_wire_id);
    toggle_component_on_click(&mut doc, 2);
    assert!(doc.sim_latched(2));
    assert!(
        !doc.sim_fault(2),
        "O rearme do disjuntor deve limpar o flag de trip"
    );

    step_simulation(&mut doc, 50);
    assert!(
        doc.sim_latched(2),
        "Sem curto-circuito, o disjuntor deve permanecer fechado"
    );
    assert!(!doc.sim_fault(2));

    // 3. Testa Disjuntor Unipolar 6000 (-Q2) com curto fase-neutro
    let mut doc2 = SchematicDocument::default();
    doc2.items.clear();
    let sym_l1 = find_symbol(3000).unwrap();
    let sym_n = find_symbol(3001).unwrap();
    let sym_q1p = find_symbol(6000).unwrap();

    let l1 = sym_l1.instantiate(10, 10, 10, 0);
    let n = sym_n.instantiate(11, 30, 10, 0);
    let mut q2 = sym_q1p.instantiate(12, 10, 30, 0);
    q2.tag = "-Q2".to_string();

    let wp_l1 = sym_l1.world_pins(&l1);
    let wp_n = sym_n.world_pins(&n);
    let wp_q2 = sym_q1p.world_pins(&q2);

    doc2.items.push(l1);
    doc2.items.push(n);
    doc2.items.push(q2);
    doc2.set_sim_latched(12, true); // Fecha o disjuntor

    // Alimentação L1 -> entrada pino 0 de -Q2
    doc2.items.push(SchematicItem::new_wire(
        13,
        4000,
        wp_l1[0].2 .0,
        wp_l1[0].2 .1,
        wp_q2[0].2 .0,
        wp_q2[0].2 .1,
    ));
    // Curto direto: saída pino 1 de -Q2 -> Neutro N
    doc2.items.push(SchematicItem::new_wire(
        14,
        4009,
        wp_q2[1].2 .0,
        wp_q2[1].2 .1,
        wp_n[0].2 .0,
        wp_n[0].2 .1,
    ));

    step_simulation(&mut doc2, 50);
    assert!(
        !doc2.sim_latched(12),
        "Disjuntor 1P deve desarmar por curto fase-neutro!"
    );
    assert!(doc2.sim_fault(12));

    // 4. Testa Fusível Seccionador 5008 (-F1) desarmando em curto-circuito
    let mut doc3 = SchematicDocument::default();
    doc3.items.clear();
    let sym_f3p = find_symbol(5008).unwrap();
    let src3 = sym_src3.instantiate(20, 10, 10, 0);
    let wp_src3 = sym_src3.world_pins(&src3);
    let mut f1 = sym_f3p.instantiate(21, 10, 30, 0);
    f1.tag = "-F1".to_string();
    let wp_f1 = sym_f3p.world_pins(&f1);

    doc3.items.push(src3);
    doc3.items.push(f1);
    doc3.set_sim_latched(21, true); // Fecha a seccionadora fusível

    // Alimenta L1 e L2 no fusível
    doc3.items.push(SchematicItem::new_wire(
        22,
        4000,
        wp_src3[0].2 .0,
        wp_src3[0].2 .1,
        wp_f1[0].2 .0,
        wp_f1[0].2 .1,
    ));
    doc3.items.push(SchematicItem::new_wire(
        23,
        4000,
        wp_src3[1].2 .0,
        wp_src3[1].2 .1,
        wp_f1[1].2 .0,
        wp_f1[1].2 .1,
    ));
    // Curto entre saídas L1 (pino 3) e L2 (pino 4) do fusível
    doc3.items.push(SchematicItem::new_wire(
        24,
        4000,
        wp_f1[3].2 .0,
        wp_f1[3].2 .1,
        wp_f1[4].2 .0,
        wp_f1[4].2 .1,
    ));

    step_simulation(&mut doc3, 50);
    assert!(
        !doc3.sim_latched(21),
        "Fusível seccionador deve desarmar (abrir) por curto-circuito!"
    );
    assert!(
        doc3.sim_fault(21),
        "Fusível queimado deve registrar estado de falha"
    );
    assert!(!doc3.sim_active(21));

    // 5. Testa Fusível Permanente 5003 (-F2) queimando em curto-circuito
    let mut doc4 = SchematicDocument::default();
    doc4.items.clear();
    let sym_f3_fixed = find_symbol(5003).unwrap();
    let src4 = sym_src3.instantiate(30, 10, 10, 0);
    let wp_src4 = sym_src3.world_pins(&src4);
    let mut f2 = sym_f3_fixed.instantiate(31, 10, 30, 0);
    f2.tag = "-F2".to_string();
    let wp_f2 = sym_f3_fixed.world_pins(&f2);

    doc4.items.push(src4);
    doc4.items.push(f2);
    doc4.items.push(SchematicItem::new_wire(
        32,
        4000,
        wp_src4[0].2 .0,
        wp_src4[0].2 .1,
        wp_f2[0].2 .0,
        wp_f2[0].2 .1,
    ));
    doc4.items.push(SchematicItem::new_wire(
        33,
        4000,
        wp_src4[1].2 .0,
        wp_src4[1].2 .1,
        wp_f2[1].2 .0,
        wp_f2[1].2 .1,
    ));
    doc4.items.push(SchematicItem::new_wire(
        34,
        4000,
        wp_f2[3].2 .0,
        wp_f2[3].2 .1,
        wp_f2[4].2 .0,
        wp_f2[4].2 .1,
    ));

    step_simulation(&mut doc4, 50);
    assert!(
        doc4.sim_latched(31),
        "Fusível fixo deve queimar (sim_manual_latched: true)!"
    );
    assert!(doc4.sim_fault(31));
    assert!(!doc4.sim_active(31));
}

#[test]
fn test_decision_d1_native_save_and_cad_export_with_sheet_selector() {
    // 1. Validação de Formatos de Salvamento Nativo (Decisão D1 / P2-#13)
    // .cad foi removido dos formatos de salvamento nativo do projeto (evita perda de multipáginas)
    assert_eq!(SaveFormat::ALL.len(), 3);
    assert!(SaveFormat::ALL.contains(&SaveFormat::Json));
    assert!(SaveFormat::ALL.contains(&SaveFormat::Toml));
    assert!(SaveFormat::ALL.contains(&SaveFormat::Qet));
    assert_eq!(
        SaveFormat::from_path(std::path::Path::new("esquema.cad")),
        None
    );
    assert_eq!(
        SaveFormat::from_path(std::path::Path::new("esquema.json")),
        Some(SaveFormat::Json)
    );
    assert_eq!(
        SaveFormat::from_path(std::path::Path::new("esquema.toml")),
        Some(SaveFormat::Toml)
    );
    assert_eq!(
        SaveFormat::from_path(std::path::Path::new("esquema.qet")),
        Some(SaveFormat::Qet)
    );

    // 2. Validação de Formatos de Exportação
    // Todos os 8 formatos da Seção 1.1: PNG, JPG, BMP, SVG, DXF, CAD, CT, PDF
    assert_eq!(ExportFormat::ALL.len(), 8);
    assert_eq!(
        ExportFormat::ALL,
        [
            ExportFormat::Png,
            ExportFormat::Jpg,
            ExportFormat::Bmp,
            ExportFormat::Svg,
            ExportFormat::Dxf,
            ExportFormat::Cad,
            ExportFormat::Ct,
            ExportFormat::Pdf,
        ]
    );

    // 3. Validação do Estado da Aplicação e UX
    let (mut app, _) = CadeSimuApp::new();
    assert_eq!(app.save_format, SaveFormat::Json, "Padrão deve ser JSON");
    assert!(!app.show_export_modal);

    // Rota legada SaveAsCad redireciona automaticamente para OpenExportModal
    let _ = app.update(Message::SaveAsCad);
    assert!(app.show_export_modal);
    assert_eq!(app.export_format, ExportFormat::Cad);

    // Tecla ESC (CancelAction) fecha o modal de exportação
    let _ = app.update(Message::CancelAction);
    assert!(!app.show_export_modal);

    // Adiciona nova folha para testar comportamento multipáginas
    let _ = app.update(Message::AddPageAfter(0));
    assert_eq!(app.pages.len(), 2);

    // Abrir modal de exportação preserva índice da folha ativa
    let _ = app.update(Message::OpenExportModal);
    assert!(app.show_export_modal);
    assert_eq!(app.export_page_index, app.active_page_index);

    // Selecionar folha específica para exportação
    let _ = app.update(Message::SelectExportPage(1));
    assert_eq!(app.export_page_index, 1);

    // Fechar explicitamente
    let _ = app.update(Message::CloseExportModal);
    assert!(!app.show_export_modal);

    // 4. Teste de exportação byte-exata para .cad via save_document
    let mut test_doc = SchematicDocument::default();
    test_doc.title_block.title = "Motor Trifásico Folha 1".to_string();
    test_doc
        .items
        .push(SchematicItem::new_symbol(1, 1000, 100, 100, "")); // Fonte 3P
    test_doc
        .items
        .push(SchematicItem::new_symbol(2, 6003, 100, 150, "-Q1")); // Disjuntor 3P

    let temp_dir = std::env::temp_dir();
    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let export_cad_path = temp_dir.join(format!(
        "test_export_sheet1_{}_{}.cad",
        std::process::id(),
        unique_id
    ));

    let export_res = cade_simu_rs::io::save_document(&export_cad_path, &test_doc);
    assert!(
        export_res.is_ok(),
        "Exportação para .cad deve ser bem sucedida"
    );

    // Valida que o arquivo gerado em CP-1252 pode ser lido de volta perfeitamente
    let raw_bytes = std::fs::read(&export_cad_path).expect("Leitura dos bytes do .cad exportado");
    let parsed_doc = parse_cad_bytes(&raw_bytes).expect("Parser do .cad exportado");
    assert_eq!(parsed_doc.items.len(), 2);
    assert_eq!(parsed_doc.title_block.title, "Motor Trifásico Folha 1");

    // Limpeza
    let _ = std::fs::remove_file(export_cad_path);
}

#[test]
fn test_multipage_json_save_and_reload() {
    let (mut app, _) = CadeSimuApp::new();

    // 1. Configura folha 1 (Potência)
    app.doc.title_block.title = "Potência Trifásica".to_string();
    app.doc.items.clear();
    app.doc
        .items
        .push(SchematicItem::new_symbol(1, 1000, 100, 100, "")); // Fonte 3P
    app.doc
        .items
        .push(SchematicItem::new_symbol(2, 6003, 100, 150, "-Q1")); // Disjuntor 3P
    if !app.pages.is_empty() {
        app.pages[0].name = "Potência".to_string();
        app.pages[0].doc = app.doc.clone();
    }

    // 2. Adiciona folha 2 (Comando)
    let _ = app.update(Message::AddPageAfter(0));
    assert_eq!(app.pages.len(), 2);
    assert_eq!(app.active_page_index, 1);

    // Configura folha 2
    app.doc.title_block.title = "Comando 24V".to_string();
    app.doc.items.clear();
    app.doc
        .items
        .push(SchematicItem::new_symbol(3, 7000, 200, 200, "-KM1")); // Bobina
    app.doc
        .items
        .push(SchematicItem::new_symbol(4, 2000, 200, 250, "-S1")); // Botão NA
    app.pages[1].name = "Comando".to_string();
    app.pages[1].doc = app.doc.clone();

    // 3. Salva projeto em JSON com nome único
    let temp_dir = std::env::temp_dir();
    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let project_path = temp_dir.join(format!(
        "test_project_multipage_{}_{}.json",
        std::process::id(),
        unique_id
    ));

    let _ = app.update(Message::FileSaved(Some(project_path.clone())));
    assert!(!app.is_modified);

    // 4. Cria nova instância em branco da aplicação e recarrega o projeto
    let (mut new_app, _) = CadeSimuApp::new();
    assert_eq!(new_app.pages.len(), 1);

    let _ = new_app.update(Message::FileOpened(Some(project_path.clone())));
    assert_eq!(
        new_app.pages.len(),
        2,
        "Projeto multipáginas deve conter exatamente 2 folhas após recarregamento"
    );
    assert_eq!(new_app.pages[0].name, "Potência");
    assert_eq!(new_app.pages[0].doc.items.len(), 2);
    assert_eq!(new_app.pages[0].doc.title_block.title, "Potência Trifásica");

    assert_eq!(new_app.pages[1].name, "Comando");
    assert_eq!(new_app.pages[1].doc.items.len(), 2);
    assert_eq!(new_app.pages[1].doc.title_block.title, "Comando 24V");

    // Limpeza
    let _ = std::fs::remove_file(project_path);
}

#[test]
fn test_qet_fuse_opens_circuit_when_blown() {
    use cade_simu_rs::sim::{is_fuse, is_qet_splitter_or_passive, step_simulation};

    // 1. Valida que o nome "fuse" NÃO é mais considerado splitter/passivo contínuo
    assert!(
        !is_qet_splitter_or_passive("qet:protection/fuse.elmt"),
        "Fusível QET não pode ser tratado como splitter passivo (circuito fechado incondicional)"
    );
    assert!(!is_qet_splitter_or_passive("fuse_10a.elmt"));
    assert!(!is_qet_splitter_or_passive("fusible_nh00.elmt"));

    // 2. Cria item representando fusível QET
    let mut qet_fuse = SchematicItem::new_symbol(10, 0, 100, 100, "-F1");
    qet_fuse.qet_type = Some("qet:protection/fuse.elmt".to_string());
    assert!(is_fuse(&qet_fuse), "Deve ser reconhecido como fusível");

    // 3. Em estado intacto (sim_manual_latched: false), sim_active deve ser true após step
    let mut doc = SchematicDocument::default();
    doc.items.push(qet_fuse.clone());
    step_simulation(&mut doc, 50);

    let fuse_intact = &doc.items[0];
    assert!(
        doc.sim_active(fuse_intact.id),
        "Fusível QET intacto deve estar ativo e condutivo"
    );

    // 4. Em estado queimado / desarmado (sim_manual_latched: true), sim_active deve ser false
    let mut doc_blown = SchematicDocument::default();
    doc_blown.items.push(qet_fuse);
    doc_blown.set_sim_latched(10, true);
    step_simulation(&mut doc_blown, 50);

    let fuse_blown = &doc_blown.items[0];
    assert!(
        !doc_blown.sim_active(fuse_blown.id),
        "Fusível QET queimado (latched) deve abrir o circuito e ter sim_active: false"
    );
}

#[test]
fn test_multipage_qet_save_and_reload() {
    let (mut app, _) = CadeSimuApp::new();

    // 1. Folha 1 (Potência)
    app.doc.title_block.title = "Potência QET".to_string();
    app.doc.items.clear();
    app.doc
        .items
        .push(SchematicItem::new_symbol(1, 1000, 100, 100, ""));
    app.doc
        .items
        .push(SchematicItem::new_symbol(2, 6003, 100, 150, "-Q1"));
    if !app.pages.is_empty() {
        app.pages[0].name = "Potência".to_string();
        app.pages[0].doc = app.doc.clone();
    }

    // 2. Folha 2 (Comando)
    let _ = app.update(Message::AddPageAfter(0));
    assert_eq!(app.pages.len(), 2);
    assert_eq!(app.active_page_index, 1);

    app.doc.title_block.title = "Comando QET".to_string();
    app.doc.items.clear();
    app.doc
        .items
        .push(SchematicItem::new_symbol(3, 7000, 200, 200, "-KM1"));
    app.doc
        .items
        .push(SchematicItem::new_symbol(4, 2000, 200, 250, "-S1"));
    app.pages[1].name = "Comando".to_string();
    app.pages[1].doc = app.doc.clone();

    // 3. Salva em arquivo .qet temporário
    let temp_dir = std::env::temp_dir();
    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let qet_path = temp_dir.join(format!(
        "test_project_multipage_{}_{}.qet",
        std::process::id(),
        unique_id
    ));

    let save_res = cade_simu_rs::io::save_project(&qet_path, &app.pages, app.active_page_index);
    assert!(
        save_res.is_ok(),
        "save_project em .qet multipáginas deve ser bem-sucedido"
    );

    // 4. Recarrega o projeto via load_project
    let loaded_proj =
        cade_simu_rs::io::load_project(&qet_path).expect("Leitura do .qet multipáginas");
    assert_eq!(
        loaded_proj.pages.len(),
        2,
        "Projeto QET multipáginas deve recarregar ambas as folhas"
    );
    assert_eq!(loaded_proj.pages[0].name, "Potência");
    assert_eq!(loaded_proj.pages[0].doc.items.len(), 2);
    assert_eq!(loaded_proj.pages[1].name, "Comando");
    assert_eq!(loaded_proj.pages[1].doc.items.len(), 2);

    let _ = std::fs::remove_file(qet_path);
}

#[test]
fn test_project_version_and_error_reporting() {
    use cade_simu_rs::model::{ProjectDocument, ProjectPage, SchematicDocument};

    // 1. Testa que ProjectDocument inclui versão padrão 1
    let proj = ProjectDocument::new(
        vec![ProjectPage::new(1, "Folha 1", SchematicDocument::default())],
        0,
    );
    assert_eq!(proj.version, 1);

    let json = serde_json::to_string(&proj).unwrap();
    assert!(
        json.contains("\"version\":1"),
        "JSON serializado deve conter o campo 'version': 1"
    );

    // 2. Testa desserialização de JSON antigo sem campo version (deve assumir version = 1 por padrão)
    let legacy_json = r#"{"pages":[{"id":1,"name":"Teste","doc":{"title_block":{"title":"","subtitle":"","drawn_by":"","drawn_date":"","scale":"","folio":"","organization":""},"settings":{"width":1040,"height":720,"grid_density":"Standard"},"items":[]}}],"active_page_index":0}"#;
    let parsed: ProjectDocument = serde_json::from_str(legacy_json).unwrap();
    assert_eq!(parsed.version, 1);

    // 3. Testa relatório de erro no load_project para JSON com campo de projeto inválido
    let temp_dir = std::env::temp_dir();
    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let bad_json_path = temp_dir.join(format!(
        "test_bad_project_{}_{}.json",
        std::process::id(),
        unique_id
    ));
    // pages como string em vez de array
    std::fs::write(&bad_json_path, r#"{"pages": "invalido"}"#).unwrap();

    let err = cade_simu_rs::io::load_project(&bad_json_path).unwrap_err();
    assert!(
        err.contains("Erro ao ler projeto JSON"),
        "Deve acusar erro específico de projeto JSON em vez de fallback silencioso: {err}"
    );

    let _ = std::fs::remove_file(bad_json_path);

    // 4. Testa rejeição de projeto com versão futura (version > CURRENT_PROJECT_VERSION)
    let future_json_path = temp_dir.join(format!(
        "test_future_project_{}_{}.json",
        std::process::id(),
        unique_id
    ));
    std::fs::write(
        &future_json_path,
        r#"{"version": 99, "pages": [], "active_page_index": 0}"#,
    )
    .unwrap();
    let err_future = cade_simu_rs::io::load_project(&future_json_path).unwrap_err();
    assert!(
        err_future.contains("não suportada por esta versão do RustSimu"),
        "Deve rejeitar versão futura de projeto: {err_future}"
    );
    let _ = std::fs::remove_file(future_json_path);
}

#[test]
fn test_num_nodes_zero_resets_energized_loads() {
    use cade_simu_rs::model::MotorMode;
    use cade_simu_rs::sim::step_simulation;

    let mut doc = SchematicDocument::default();
    doc.items.clear();

    // Motor 1000 ativo
    let motor = SchematicItem::new_symbol(1, 1000, 100, 100, "-M1");
    doc.items.push(motor);
    let s_motor = doc.sim.get_mut(1);
    s_motor.active = true;
    s_motor.motor_dir = 1;
    s_motor.motor_mode = MotorMode::Direct;

    // Bobina 9000 ativa
    let coil = SchematicItem::new_symbol(2, 9000, 200, 200, "-KM1");
    doc.items.push(coil);
    doc.sim.get_mut(2).active = true;

    // Lâmpada sinaleira 9008 ativa
    let lamp = SchematicItem::new_symbol(3, 9008, 300, 300, "-H1");
    doc.items.push(lamp);
    doc.sim.get_mut(3).active = true;

    // Sem nenhum fio (sem fontes/diferença de potencial)
    step_simulation(&mut doc, 50);

    // Motor, bobina e lâmpada devem ser zerados
    assert!(!doc.sim_active(1), "Motor deve ser desligado sem fios");
    assert_eq!(doc.sim_motor_dir(1), 0);
    assert_eq!(doc.sim_motor_mode(1), MotorMode::Off);

    assert!(!doc.sim_active(2), "Bobina deve ser desenergizada sem fios");
    assert!(!doc.sim_active(3), "Lâmpada deve ser desligada sem fios");

    // Também valida o ramo num_nodes == 0 estrito (itens sem pinos catalogados)
    let mut doc_empty_nodes = SchematicDocument::default();
    let custom_load = SchematicItem::new_symbol(1, 0, 100, 100, "-H2");
    doc_empty_nodes.items.push(custom_load);
    doc_empty_nodes.sim.get_mut(1).active = true;
    step_simulation(&mut doc_empty_nodes, 50);
    assert!(
        !doc_empty_nodes.sim_active(1),
        "Item sem nós deve ser desativado no ramo num_nodes == 0"
    );
}

#[test]
fn test_qet_heuristic_token_matching_no_false_positives() {
    use cade_simu_rs::sim::{
        is_qet_load, is_qet_source_dc_pos, is_qet_source_ground, is_qet_splitter_or_passive,
        is_qet_switch_or_button,
    };

    // 1. Interruptores e botões com palavras compostas (alemão, inglês, francês)
    assert!(is_qet_switch_or_button("sensors/limitswitch_no.elmt"));
    assert!(is_qet_switch_or_button("relays/microswitch.elmt"));
    assert!(is_qet_switch_or_button("buttons/pushbutton_green.elmt"));
    assert!(is_qet_switch_or_button("automatisme/endschalter_nc.elmt"));
    assert!(is_qet_switch_or_button("bedienung/drucktaster_rot.elmt"));
    assert!(is_qet_switch_or_button("commandes/bouton_poussoir.elmt"));
    assert!(is_qet_switch_or_button(
        "contacteurs/interrupteur_sectionneur.elmt"
    ));
    assert!(is_qet_switch_or_button("auto/ignition_starter_switch.elmt"));

    assert!(!is_qet_switch_or_button("transformateurs/transfo_1p.elmt"));
    assert!(!is_qet_switch_or_button("borniers/terminal_block_3p.elmt"));

    // 2. Cargas elétricas: falsos positivos comuns de substring não devem casar
    assert!(
        !is_qet_load("flight_control_unit.elmt"),
        "flight não deve ser reconhecido como light"
    );
    assert!(
        !is_qet_load("logic/fanout_gate.elmt"),
        "fanout não deve ser reconhecido como fan"
    );
    assert!(
        !is_qet_load("terminal_clamping.elmt"),
        "clamping não deve ser reconhecido como lamp"
    );

    // Cargas compostas e legítimas
    assert!(is_qet_load("cooling/cooling_fan_24v.elmt"));
    assert!(is_qet_load("signal/warning_light_red.elmt"));
    assert!(is_qet_load("eclairage/lampe_temoin.elmt"));
    assert!(is_qet_load("actuators/horn_12v.elmt"));
    assert!(is_qet_load("engines/starter_motor.elmt"));
    assert!(is_qet_load("motors/servomotor_ac.elmt"));
    assert!(is_qet_load("alarmes/sirene_electronique.elmt"));
    assert!(is_qet_load("signalisation/voyant_led.elmt"));

    // 3. Fontes DC+ e Terra
    assert!(is_qet_source_dc_pos("auto/autobatterie_12v.elmt"));
    assert!(is_qet_source_dc_pos("power/battery_pack.elmt"));
    assert!(is_qet_source_dc_pos("dc_power/terminal_plus.elmt"));
    assert!(!is_qet_source_dc_pos("terminals/chassis_ground.elmt"));

    assert!(is_qet_source_ground("power/chassis_ground.elmt"));
    assert!(is_qet_source_ground("auto/masse_carrosserie.elmt"));
    assert!(is_qet_source_ground("protection/earth_ground.elmt"));
    assert!(is_qet_source_ground("logic/gnd_plane.elmt"));

    // 4. Conexões passivas e distribuidores
    assert!(is_qet_splitter_or_passive("distribution/verteiler_3p.elmt"));
    assert!(is_qet_splitter_or_passive("power/distributor_block.elmt"));
    assert!(is_qet_splitter_or_passive("cables/splice_point.elmt"));
    assert!(is_qet_splitter_or_passive(
        "cables/bornier_repartiteur.elmt"
    ));
    // Conectores e bornes NÃO devem ser splitters automáticos para não curto-circuitar todos os pinos
    assert!(!is_qet_splitter_or_passive(
        "borniers/terminal_strip_24p.elmt"
    ));
    assert!(!is_qet_splitter_or_passive(
        "connectors/connector_db25.elmt"
    ));
}

#[test]
fn test_qet_multipage_merge_preserves_existing_diagram_properties() {
    use cade_simu_rs::model::ProjectPage;
    use cade_simu_rs::qet::{
        export_pages_to_qet, load_qet_file, QetConductor, QetConductorSegment, QetDiagram,
        QetElementInstance, QetProject,
    };

    // 1. Cria um QetProject simulado com propriedades e dimensões customizadas
    let mut custom_diag1 = QetDiagram {
        order: 1,
        title: "Folha Original 1".to_string(),
        author: "Engenharia".to_string(),
        date: "2026-11-05".to_string(),
        rows: 14,
        cols: 10,
        rowsize: 65.5,
        colsize: 145.0,
        height: 920.0,
        width: 1450.0,
        ..Default::default()
    };
    custom_diag1.elements.push(QetElementInstance {
        uuid: "uuid-orig-1".to_string(),
        element_type: "motor_custom".to_string(),
        x: 120.0,
        y: 180.0,
        orientation: 0,
        labels: vec!["-M1".to_string()],
        ..Default::default()
    });
    custom_diag1.conductors.push(QetConductor {
        element1: "uuid-orig-1".to_string(),
        terminal1: "1".to_string(),
        element2: "uuid-orig-2".to_string(),
        terminal2: "2".to_string(),
        segments: vec![QetConductorSegment {
            orientation: "horizontal".to_string(),
            length: 50.0,
        }],
        ..Default::default()
    });

    let custom_diag2 = QetDiagram {
        order: 2,
        title: "Folha Original 2".to_string(),
        author: "Engenharia".to_string(),
        date: "2026-11-05".to_string(),
        rows: 10,
        cols: 8,
        rowsize: 70.0,
        colsize: 160.0,
        height: 700.0,
        width: 1280.0,
        ..Default::default()
    };

    let original_project = QetProject {
        title: "Projeto Customizado QET".to_string(),
        version: "0.100.0".to_string(),
        diagrams: vec![custom_diag1, custom_diag2],
        ..Default::default()
    };

    // 2. Converte para páginas editadas:
    // Na folha 1, alteramos a posição do elemento uuid-orig-1 e adicionamos um novo elemento
    let mut doc1 = cade_simu_rs::qet::diagram_to_document(&original_project, 0);
    // Move o elemento
    if let Some(item) = doc1
        .items
        .iter_mut()
        .find(|i| i.qet_uuid.as_deref() == Some("uuid-orig-1"))
    {
        item.x1 = 200;
        item.y1 = 250;
    }
    // Adiciona novo elemento
    doc1.items
        .push(SchematicItem::new_symbol(99, 1000, 300, 300, "-M2"));

    let doc2 = cade_simu_rs::qet::diagram_to_document(&original_project, 1);

    // E adicionamos uma nova Folha 3 (criada do zero no RustSimu)
    let doc3 = SchematicDocument::default();

    let pages = vec![
        ProjectPage::with_origin(1, "Folha Original 1".to_string(), doc1, Some(0)),
        ProjectPage::with_origin(2, "Folha Original 2".to_string(), doc2, Some(1)),
        ProjectPage::new(3, "Folha Nova 3".to_string(), doc3),
    ];

    let temp_dir = std::env::temp_dir();
    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let out_path = temp_dir.join(format!(
        "test_merge_qet_{}_{}.qet",
        std::process::id(),
        unique_id
    ));

    // Exporta utilizando o existing_proj
    export_pages_to_qet(&pages, 0, &out_path, Some(&original_project))
        .expect("export_pages_to_qet com merge deve salvar com sucesso");

    // 3. Recarrega o arquivo gerado
    let reloaded = load_qet_file(&out_path).expect("load_qet_file do arquivo mesclado");
    assert_eq!(
        reloaded.diagrams.len(),
        3,
        "Deve conter 3 diagramas no total"
    );

    // Folha 1: dimensões originais preservadas intactas!
    let d1 = &reloaded.diagrams[0];
    assert_eq!(d1.rows, 14, "rows da folha 1 deve ser preservado");
    assert_eq!(d1.cols, 10, "cols da folha 1 deve ser preservado");
    assert_eq!(d1.rowsize, 65.5, "rowsize da folha 1 deve ser preservado");
    assert_eq!(d1.colsize, 145.0, "colsize da folha 1 deve ser preservado");
    assert_eq!(d1.height, 920.0, "height da folha 1 deve ser preservado");
    assert_eq!(d1.width, 1450.0, "width da folha 1 deve ser preservado");
    assert_eq!(
        d1.conductors.len(),
        1,
        "Condutor original deve ser preservado"
    );
    let moved_el = d1
        .elements
        .iter()
        .find(|e| e.uuid == "uuid-orig-1")
        .expect("Elemento original existente");
    assert_eq!(
        moved_el.x, 200.0,
        "Posição x do elemento editado deve ser atualizada"
    );
    assert_eq!(
        moved_el.y, 250.0,
        "Posição y do elemento editado deve ser atualizada"
    );
    let added_el = d1
        .elements
        .iter()
        .find(|e| e.uuid == "item_99")
        .expect("Novo elemento adicionado");
    assert_eq!(added_el.x, 300.0);
    assert_eq!(added_el.labels, vec!["-M2"]);

    // Folha 2: dimensões originais preservadas intactas!
    let d2 = &reloaded.diagrams[1];
    assert_eq!(d2.rows, 10, "rows da folha 2 deve ser preservado");
    assert_eq!(d2.cols, 8, "cols da folha 2 deve ser preservado");
    assert_eq!(d2.rowsize, 70.0);
    assert_eq!(d2.colsize, 160.0);

    // Folha 3: folha nova gerada via document_to_diagram
    let d3 = &reloaded.diagrams[2];
    assert_eq!(d3.order, 3);
    assert_eq!(d3.title, "Folha Nova 3");

    let _ = std::fs::remove_file(out_path);
}

#[test]
fn test_qet_multipage_reorder_and_delete_preserves_elements() {
    use cade_simu_rs::model::ProjectPage;
    use cade_simu_rs::qet::{
        export_pages_to_qet, load_qet_file, QetDiagram, QetElementInstance, QetProject,
    };

    // 1. Cria projeto original com 2 folhas distintas
    let mut diag1 = QetDiagram {
        order: 1,
        title: "Folha Original 1".to_string(),
        rows: 12,
        ..Default::default()
    };
    diag1.elements.push(QetElementInstance {
        uuid: "uuid-elem-A".to_string(),
        element_type: "motor_a".to_string(),
        x: 100.0,
        y: 100.0,
        labels: vec!["-MA".to_string()],
        ..Default::default()
    });

    let mut diag2 = QetDiagram {
        order: 2,
        title: "Folha Original 2".to_string(),
        rows: 16,
        ..Default::default()
    };
    diag2.elements.push(QetElementInstance {
        uuid: "uuid-elem-B".to_string(),
        element_type: "contactor_b".to_string(),
        x: 200.0,
        y: 200.0,
        labels: vec!["-KM1".to_string()],
        ..Default::default()
    });

    let original_project = QetProject {
        title: "Projeto Multipage Teste".to_string(),
        version: "0.100.0".to_string(),
        diagrams: vec![diag1, diag2],
        ..Default::default()
    };

    let doc1 = cade_simu_rs::qet::diagram_to_document(&original_project, 0);
    let doc2 = cade_simu_rs::qet::diagram_to_document(&original_project, 1);

    let temp_dir = std::env::temp_dir();
    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    // Caso A: Reordenação de folhas (Folha 2 movida para a 1ª posição, Folha 1 para a 2ª posição)
    let reordered_pages = vec![
        ProjectPage::with_origin(1, "Folha B (Reordenada)".to_string(), doc2.clone(), Some(1)),
        ProjectPage::with_origin(2, "Folha A (Reordenada)".to_string(), doc1, Some(0)),
    ];
    let path_reordered = temp_dir.join(format!("test_reorder_{}.qet", unique_id));
    export_pages_to_qet(
        &reordered_pages,
        0,
        &path_reordered,
        Some(&original_project),
    )
    .expect("export com reordenação deve salvar");

    let reloaded_reordered = load_qet_file(&path_reordered).expect("recarrega reordenado");
    assert_eq!(reloaded_reordered.diagrams.len(), 2);
    // Diagrama 0 agora é o original 1 (Folha B)
    assert_eq!(reloaded_reordered.diagrams[0].rows, 16);
    assert_eq!(reloaded_reordered.diagrams[0].title, "Folha B (Reordenada)");
    assert_eq!(
        reloaded_reordered.diagrams[0].elements[0].uuid,
        "uuid-elem-B"
    );
    // Diagrama 1 agora é o original 0 (Folha A)
    assert_eq!(reloaded_reordered.diagrams[1].rows, 12);
    assert_eq!(reloaded_reordered.diagrams[1].title, "Folha A (Reordenada)");
    assert_eq!(
        reloaded_reordered.diagrams[1].elements[0].uuid,
        "uuid-elem-A"
    );
    let _ = std::fs::remove_file(path_reordered);

    // Caso B: Exclusão de folha (mantém apenas a Folha 2)
    let single_page = vec![ProjectPage::with_origin(
        1,
        "Folha B Única".to_string(),
        doc2,
        Some(1),
    )];
    let path_deleted = temp_dir.join(format!("test_delete_{}.qet", unique_id));
    export_pages_to_qet(&single_page, 0, &path_deleted, Some(&original_project))
        .expect("export com folha apagada deve salvar");

    let reloaded_deleted = load_qet_file(&path_deleted).expect("recarrega após exclusão");
    assert_eq!(reloaded_deleted.diagrams.len(), 1);
    assert_eq!(reloaded_deleted.diagrams[0].order, 1);
    assert_eq!(reloaded_deleted.diagrams[0].rows, 16);
    assert_eq!(reloaded_deleted.diagrams[0].elements[0].uuid, "uuid-elem-B");
    let _ = std::fs::remove_file(path_deleted);
}

#[test]
fn test_qet_delete_element_and_wire_propagates_to_save() {
    use cade_simu_rs::model::ProjectPage;
    use cade_simu_rs::qet::{
        export_pages_to_qet, load_qet_file, QetConductor, QetDiagram, QetElementInstance,
        QetProject,
    };

    let mut diag = QetDiagram {
        order: 1,
        title: "Folha Teste Remoções".to_string(),
        ..Default::default()
    };
    diag.elements.push(QetElementInstance {
        uuid: "uuid-orig-1".to_string(),
        element_type: "borne_1".to_string(),
        x: 100.0,
        y: 100.0,
        ..Default::default()
    });
    diag.elements.push(QetElementInstance {
        uuid: "uuid-orig-2".to_string(),
        element_type: "borne_2".to_string(),
        x: 200.0,
        y: 100.0,
        ..Default::default()
    });
    diag.conductors.push(QetConductor {
        element1: "uuid-orig-1".to_string(),
        terminal1: "1".to_string(),
        element2: "uuid-orig-2".to_string(),
        terminal2: "2".to_string(),
        ..Default::default()
    });

    let project = QetProject {
        title: "Projeto Remocao Teste".to_string(),
        version: "0.100.0".to_string(),
        diagrams: vec![diag],
        ..Default::default()
    };

    // 1. Importa para SchematicDocument
    let mut doc = cade_simu_rs::qet::diagram_to_document(&project, 0);
    assert_eq!(doc.items.len(), 3); // 2 elementos + 1 fio importado
    assert!(doc.imported_uuids.contains("uuid-orig-1"));
    assert!(doc.imported_uuids.contains("uuid-orig-2"));
    assert!(doc.imported_uuids.contains("cond_0"));

    // 2. Apaga o fio importado (condutor)
    doc.items.retain(|i| !i.is_wire());
    assert_eq!(doc.items.len(), 2);

    let temp_dir = std::env::temp_dir();
    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path1 = temp_dir.join(format!("test_del_wire_{}.qet", unique_id));

    let pages1 = vec![ProjectPage::with_origin(1, "Folha 1", doc.clone(), Some(0))];
    export_pages_to_qet(&pages1, 0, &path1, Some(&project))
        .expect("deve salvar após remoção de fio");

    let reloaded1 = load_qet_file(&path1).expect("recarrega após remoção de fio");
    assert_eq!(
        reloaded1.diagrams[0].conductors.len(),
        0,
        "Condutor apagado não deve reaparecer!"
    );
    assert_eq!(
        reloaded1.diagrams[0].elements.len(),
        2,
        "Elementos devem permanecer"
    );
    let _ = std::fs::remove_file(path1);

    // 3. Apaga o elemento uuid-orig-2 e adiciona um novo fio desenhado pelo usuário
    doc.items
        .retain(|i| i.qet_uuid.as_deref() != Some("uuid-orig-2"));
    assert_eq!(doc.items.len(), 1); // sobrou apenas uuid-orig-1

    // Adiciona um novo fio desenhado pelo usuário (qet_uuid == None)
    doc.items
        .push(SchematicItem::new_wire(999, 4000, 100, 100, 150, 100));

    let path2 = temp_dir.join(format!("test_del_elem_add_wire_{}.qet", unique_id));
    let pages2 = vec![ProjectPage::with_origin(1, "Folha 1", doc, Some(0))];
    export_pages_to_qet(&pages2, 0, &path2, Some(&project))
        .expect("deve salvar após remoção de elemento e adição de novo fio");

    let reloaded2 = load_qet_file(&path2).expect("recarrega após remoção e adição");
    // O elemento uuid-orig-2 não deve reaparecer
    assert!(
        !reloaded2.diagrams[0]
            .elements
            .iter()
            .any(|e| e.uuid == "uuid-orig-2"),
        "Elemento apagado não deve reaparecer!"
    );
    // O novo fio desenhado deve ser gravado como condutor
    assert_eq!(
        reloaded2.diagrams[0].conductors.len(),
        1,
        "Novo fio desenhado deve persistir no QET!"
    );
    let _ = std::fs::remove_file(path2);
}

#[test]
fn test_qet_rename_sheet_persists_over_title_block() {
    use cade_simu_rs::model::ProjectPage;
    use cade_simu_rs::qet::{export_pages_to_qet, load_qet_file, QetDiagram, QetProject};

    let diag = QetDiagram {
        order: 1,
        title: "Nome Antigo Folha".to_string(),
        ..Default::default()
    };
    let project = QetProject {
        title: "Projeto Rename Teste".to_string(),
        version: "0.100.0".to_string(),
        diagrams: vec![diag],
        ..Default::default()
    };

    let mut doc = cade_simu_rs::qet::diagram_to_document(&project, 0);
    // Suponha que o carimbo tenha um título preenchido diferente do novo nome da aba
    doc.title_block.title = "Título Antigo do Carimbo".to_string();

    let new_sheet_name = "Folha Renomeada Pelo Usuário".to_string();
    let pages = vec![ProjectPage::with_origin(
        1,
        new_sheet_name.clone(),
        doc,
        Some(0),
    )];

    let temp_dir = std::env::temp_dir();
    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = temp_dir.join(format!("test_rename_sheet_{}.qet", unique_id));

    export_pages_to_qet(&pages, 0, &path, Some(&project)).expect("deve salvar com nome renomeado");

    let reloaded = load_qet_file(&path).expect("recarrega projeto renomeado");
    assert_eq!(
        reloaded.diagrams[0].title, new_sheet_name,
        "O nome da folha editado pelo usuário deve prevalecer sobre o carimbo!"
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn test_qet_connector_two_pin_conduction_and_multi_pin_isolation() {
    use cade_simu_rs::sim::{
        is_qet_connector_or_terminal, is_qet_splitter_or_passive, step_simulation,
    };
    use cade_simu_rs::svg_symbol::{register_parsed_svg, ParsedSvgSymbol, SvgFill, SvgShape};

    // 1. Testa heurísticas de classificação
    assert!(is_qet_connector_or_terminal("terminal/terminal_001.elmt"));
    assert!(is_qet_connector_or_terminal("bornier/bornier_1.elmt"));
    assert!(is_qet_connector_or_terminal(
        "connectors/connector_001.elmt"
    ));
    assert!(is_qet_connector_or_terminal(
        "connectors/kfz_klemme_2pol_001.elmt"
    ));
    assert!(is_qet_splitter_or_passive("connections/combine.elmt"));
    assert!(is_qet_splitter_or_passive("connections/lphd.elmt"));

    // Registra símbolos SVG no cache com 2 terminais e 4 terminais
    let term_circle = |cx: f32, cy: f32| SvgShape::Circle {
        cx,
        cy,
        r: 2.0,
        is_terminal: true,
        fill: SvgFill::None,
    };

    let sym_2pin = ParsedSvgSymbol {
        view_box: (0.0, 0.0, 100.0, 100.0),
        shapes: vec![term_circle(0.0, 0.0), term_circle(20.0, 0.0)],
        term_top_y: 0.0,
        term_bottom_y: 0.0,
        min_term_x: 0.0,
        max_term_x: 20.0,
        is_cade_svg: false,
        is_qet_native: true,
        normalized_scale: 0.30,
    };
    register_parsed_svg("terminal/test_2pol.elmt", sym_2pin);

    let sym_4pin = ParsedSvgSymbol {
        view_box: (0.0, 0.0, 100.0, 100.0),
        shapes: vec![
            term_circle(0.0, 0.0),
            term_circle(20.0, 0.0),
            term_circle(40.0, 0.0),
            term_circle(60.0, 0.0),
        ],
        term_top_y: 0.0,
        term_bottom_y: 0.0,
        min_term_x: 0.0,
        max_term_x: 60.0,
        is_cade_svg: false,
        is_qet_native: true,
        normalized_scale: 0.30,
    };
    register_parsed_svg("connectors/test_4pol.elmt", sym_4pin);

    // 2. Conector/borne de 2 pinos conduz entre os pinos 0 e 1
    let mut doc = SchematicDocument::default();
    // Fonte DC+ (3000 = fonte L1/DC+) na posição (100, 100)
    let p_dc = SchematicItem::new_symbol(1, 3000, 100, 100, "+24V");
    // Borne/Conector de 2 pinos: pino 0 em (100, 100), pino 1 em (120, 100)
    let mut terminal = SchematicItem::new_symbol(2, 20000, 100, 100, "-X1");
    terminal.qet_type = Some("terminal/test_2pol.elmt".to_string());
    // Fio de saída conectado ao pino 1 em (120, 100) até (150, 100)
    let wire_out = SchematicItem::new_wire(3, 4000, 120, 100, 150, 100);

    doc.items = vec![p_dc, terminal, wire_out];
    step_simulation(&mut doc, 0);

    // O borne de 2 pinos conduziu de (100, 100) para (120, 100), energizando o fio de saída
    assert!(
        doc.sim_active(3),
        "Conector/borne de 2 pinos deve conduzir a corrente e energizar o circuito a jusante!"
    );
    assert_ne!(doc.sim_potential(3), Potential::None);

    // 3. Conector multipinos (4 pinos) NÃO curto-circuita pinos independentes
    let mut doc_multi = SchematicDocument::default();
    let p_dc2 = SchematicItem::new_symbol(1, 3000, 100, 100, "+24V");
    // Conector de 4 pinos em (100, 100): pino 0 em (100, 100), pino 1 em (120, 100), etc.
    let mut multi_conn = SchematicItem::new_symbol(2, 20000, 100, 100, "-X2");
    multi_conn.qet_type = Some("connectors/test_4pol.elmt".to_string());
    // Fio conectado ao pino 1 em (120, 100)
    let wire_out2 = SchematicItem::new_wire(3, 4000, 120, 100, 150, 100);

    doc_multi.items = vec![p_dc2, multi_conn, wire_out2];
    step_simulation(&mut doc_multi, 0);

    // Como tem 4 pinos (> 2), não conduz cegamente entre pinos distintos
    assert!(
        !doc_multi.sim_active(3),
        "Conector multipinos (> 2) não deve curto-circuitar todos os circuitos!"
    );
    assert_eq!(doc_multi.sim_potential(3), Potential::None);
}

#[test]
fn test_cp1252_tipo_8_text_items_and_legacy_projects_reading() {
    use std::path::Path;

    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let projetos_dir = if manifest.join("Projetos").is_dir() {
        manifest.join("Projetos")
    } else {
        manifest.parent().unwrap().join("Projetos")
    };

    // 1. Carrega 'Partida Direta.cad' legado e valida texto tipo == 8 com acento CP-1252 (É = 0xC9)
    let doc_partida = load_document(&projetos_dir.join("Partida Direta.cad"))
        .expect("Deve carregar 'Partida Direta.cad'");
    let text_item = doc_partida
        .items
        .iter()
        .find(|i| i.tipo == 8 && i.trailing_str.contains("BLOG ENSINANDO"))
        .expect("Elemento de texto tipo == 8 com 'BLOG ENSINANDO ELÉTRICA' deve existir");
    assert_eq!(
        text_item.trailing_str, "BLOG ENSINANDO ELÉTRICA",
        "Texto CP-1252 deve ser decodificado com acento correto (ELÉTRICA com 'É', não C1 control code)"
    );

    // 2. Carrega 'Inversão de rotação motor trifásico.cad' legado e valida acentuação no cajetín
    let doc_inversao = load_document(&projetos_dir.join("Inversão de rotação motor trifásico.cad"))
        .expect("Deve carregar 'Inversão de rotação motor trifásico.cad'");
    assert_eq!(
        doc_inversao.title_block.signature, "Inversão de rotação motor trifásico",
        "Assinatura do cajetín deve preservar ã, ç, á corretamente decodificados de CP-1252: '{}'",
        doc_inversao.title_block.signature
    );

    // 3. Roundtrip completo com caracteres da faixa 0x80..=0x9F em itens tipo == 8
    let mut doc_custom = SchematicDocument::default();
    let mut custom_text = SchematicItem::new_symbol(1, 8, 50, 50, "");
    custom_text.trailing_str = "Comando – Painel “A” • 380V ± 5% • 10µF • €150 • TM™".to_string();
    doc_custom.items.push(custom_text);
    doc_custom.title_block.title = "Esquema Especial – Teste".to_string();

    let cad_bytes = serialize_cad_bytes(&doc_custom);

    // Valida que os bytes foram gravados como bytes únicos CP-1252:
    // '–' (en dash) deve ser byte 0x96
    assert!(
        cad_bytes.contains(&0x96),
        "Byte 0x96 (en dash) deve estar presente no arquivo .cad gerado"
    );
    // '“' (left quote) deve ser byte 0x93
    assert!(
        cad_bytes.contains(&0x93),
        "Byte 0x93 (left quote) deve estar presente no arquivo .cad gerado"
    );
    // '”' (right quote) deve ser byte 0x94
    assert!(
        cad_bytes.contains(&0x94),
        "Byte 0x94 (right quote) deve estar presente no arquivo .cad gerado"
    );
    // '•' (bullet) deve ser byte 0x95
    assert!(
        cad_bytes.contains(&0x95),
        "Byte 0x95 (bullet) deve estar presente no arquivo .cad gerado"
    );
    // '€' (euro) deve ser byte 0x80
    assert!(
        cad_bytes.contains(&0x80),
        "Byte 0x80 (euro) deve estar presente no arquivo .cad gerado"
    );
    // '™' (trademark) deve ser byte 0x99
    assert!(
        cad_bytes.contains(&0x99),
        "Byte 0x99 (trademark) deve estar presente no arquivo .cad gerado"
    );

    // Reabre através de parse_cad_bytes
    let reloaded = parse_cad_bytes(&cad_bytes).expect("Deve reabrir bytes .cad sem erro");
    assert_eq!(reloaded.items.len(), 1);
    assert_eq!(reloaded.items[0].tipo, 8);
    assert_eq!(
        reloaded.items[0].trailing_str,
        "Comando – Painel “A” • 380V ± 5% • 10µF • €150 • TM™"
    );
    assert!(reloaded
        .title_block
        .title
        .contains("Esquema Especial – Teste"));
}

#[test]
fn test_export_real_svg_png_pdf_cad_scopes_and_cajetin_toggle() {
    // 1. Cria aplicação multipáginas com elementos realistas
    let (mut app, _) = CadeSimuApp::new();

    // Folha 1: Potência Trifásica
    app.doc = build_dol_starter_preset();
    app.doc.title_block.title = "Potência Trifásica".to_string();
    app.pages[0].name = "Potência".to_string();
    app.pages[0].doc = app.doc.clone();

    // Folha 2: Comando Auxiliar
    let _ = app.update(Message::AddPageAfter(0));
    assert_eq!(app.pages.len(), 2);
    app.active_page_index = 1;
    app.doc.items.clear();
    app.doc.title_block.title = "Circuito de Comando 24V".to_string();
    app.doc
        .items
        .push(SchematicItem::new_symbol(10, 8000, 200, 200, "-S1"));
    app.doc
        .items
        .push(SchematicItem::new_symbol(11, 7001, 200, 300, "-H1"));
    app.pages[1].name = "Comando".to_string();
    app.pages[1].doc = app.doc.clone();

    // Retorna para a Folha 1
    app.active_page_index = 0;
    app.doc = app.pages[0].doc.clone();

    // 2. Exportação SVG
    // 2.1 Folha completa com Cajetín ON
    let svg_cajetin_on = export_to_svg(&app.doc, None, true);
    assert!(svg_cajetin_on.starts_with("<?xml"));
    assert!(svg_cajetin_on.contains("<svg"));
    assert!(svg_cajetin_on.contains("Potência Trifásica"));
    assert!(svg_cajetin_on.contains("Entidade:"));
    assert!(svg_cajetin_on.ends_with("</svg>\n"));

    // 2.2 Folha completa com Cajetín OFF
    let svg_cajetin_off = export_to_svg(&app.doc, None, false);
    assert!(svg_cajetin_off.starts_with("<?xml"));
    assert!(!svg_cajetin_off.contains("Entidade:"));
    assert!(!svg_cajetin_off.contains("Potência Trifásica"));
    // Contém os símbolos do esquema mesmo sem cajetín
    assert!(svg_cajetin_off.contains("-Q1"));

    // 2.3 Seleção de Elemento
    let q1_id = app.doc.items.iter().find(|i| i.tag == "-Q1").unwrap().id;
    let svg_sel = export_to_svg(&app.doc, Some(&[q1_id]), false);
    assert!(svg_sel.contains("-Q1"));
    assert!(!svg_sel.contains("-KM1"));
    assert!(!svg_sel.contains("Entidade:"));

    // 3. Exportação PNG
    let png_page = export_to_png(&app.doc, None, true, 1.5).expect("Render PNG de folha completa");
    assert!(
        png_page.starts_with(b"\x89PNG\r\n\x1a\n"),
        "Deve conter cabeçalho mágico PNG válido"
    );
    assert!(png_page.len() > 1000, "PNG deve ter tamanho substancial");

    let png_sel =
        export_to_png(&app.doc, Some(&[q1_id]), false, 2.0).expect("Render PNG de seleção");
    assert!(
        png_sel.starts_with(b"\x89PNG\r\n\x1a\n"),
        "Seleção deve produzir PNG válido"
    );

    // 4. Exportação PDF
    // 4.1 PDF de Folha Única
    let pdf_page = export_doc_to_pdf(&app.doc, None, true).expect("PDF folha única");
    assert!(pdf_page.starts_with(b"%PDF-1.4"));
    assert!(pdf_page.windows(8).any(|w| w == b"/Count 1"));
    assert!(pdf_page.windows(5).any(|w| w == b"%%EOF"));

    // 4.2 PDF de Seleção
    let pdf_sel = export_doc_to_pdf(&app.doc, Some(&[q1_id]), false).expect("PDF seleção");
    assert!(pdf_sel.starts_with(b"%PDF-1.4"));
    assert!(pdf_sel.windows(8).any(|w| w == b"/Count 1"));

    // 4.3 PDF de Projeto Completo (Multi-páginas)
    let pdf_project = export_project_to_pdf(&app.pages, true).expect("PDF multipáginas");
    assert!(pdf_project.starts_with(b"%PDF-1.4"));
    assert!(
        pdf_project.windows(8).any(|w| w == b"/Count 2"),
        "PDF de projeto com 2 páginas deve ter /Count 2"
    );
    assert!(pdf_project.windows(5).any(|w| w == b"%%EOF"));

    // 5. Exportação CAD (.cad)
    let cad_cajetin_on = export_to_cad(&app.doc, true);
    assert!(cad_cajetin_on.starts_with(b"CADe_SIMU*"));
    assert!(cad_cajetin_on.ends_with(b"***$$$&&&***$$$&&&"));
    let parsed_on = parse_cad_bytes(&cad_cajetin_on).expect("Parser CAD com cajetin");
    assert_eq!(parsed_on.title_block.title, "Potência Trifásica");

    let cad_cajetin_off = export_to_cad(&app.doc, false);
    let parsed_off = parse_cad_bytes(&cad_cajetin_off).expect("Parser CAD sem cajetin");
    assert!(
        parsed_off.title_block.title.is_empty(),
        "Título do bloco de carimbo deve ser vazio quando cajetín está desativado"
    );

    // 6. Ciclo Completo de Mensagens e UX no CadeSimuApp
    assert!(!app.show_export_modal);
    let _ = app.update(Message::OpenExportModal);
    assert!(app.show_export_modal);

    // Seleciona PDF e escopo Projeto
    let _ = app.update(Message::SelectExportFormat(ExportFormat::Pdf));
    assert_eq!(app.export_format, ExportFormat::Pdf);
    let _ = app.update(Message::SelectExportScope(ExportScope::Project));
    assert_eq!(app.export_scope, ExportScope::Project);

    // Desativa cajetín
    let _ = app.update(Message::ToggleExportCajetin(false));
    assert!(!app.export_include_cajetin);

    // Altera para SVG: escopo Projeto não é válido para SVG em multipáginas,
    // então deve reverter automaticamente para Page
    let _ = app.update(Message::SelectExportFormat(ExportFormat::Svg));
    assert_eq!(app.export_format, ExportFormat::Svg);
    assert_eq!(app.export_scope, ExportScope::Page);

    // Testa gravação em disco via FileExported
    let temp_dir = std::env::temp_dir();
    let unique_id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();

    // 6.1 Gravação real de arquivo SVG via FileExported
    let temp_svg = temp_dir.join(format!(
        "test_app_export_{}_{}.svg",
        std::process::id(),
        unique_id
    ));
    let _ = app.update(Message::FileExported(
        Some(temp_svg.clone()),
        Box::new(app.doc.clone()),
        app.pages[0].name.clone(),
        ExportFormat::Svg,
        ExportScope::Page,
        true.into(),
        None,
    ));
    assert!(
        temp_svg.exists(),
        "Arquivo SVG deve ter sido gravado em disco"
    );
    let svg_content = std::fs::read_to_string(&temp_svg).expect("Leitura do SVG gravado");
    assert!(svg_content.contains("<svg"));
    assert!(app.status_msg.contains("concluída com sucesso"));
    let _ = std::fs::remove_file(temp_svg);

    // 6.2 Gravação real de arquivo PDF multipáginas via FileExported
    let temp_pdf = temp_dir.join(format!(
        "test_app_export_{}_{}.pdf",
        std::process::id(),
        unique_id
    ));
    let _ = app.update(Message::FileExported(
        Some(temp_pdf.clone()),
        Box::new(app.doc.clone()),
        "Projeto Completo".to_string(),
        ExportFormat::Pdf,
        ExportScope::Project,
        true.into(),
        None,
    ));
    assert!(
        temp_pdf.exists(),
        "Arquivo PDF multipáginas deve ter sido gravado em disco"
    );
    let pdf_bytes = std::fs::read(&temp_pdf).expect("Leitura do PDF gravado");
    assert!(pdf_bytes.starts_with(b"%PDF-1.4"));
    assert!(app.status_msg.contains("Projeto Completo"));
    let _ = std::fs::remove_file(temp_pdf);
}

#[test]
fn test_canonical_catalog_validation_d2() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let symbols_dir = manifest_dir.join("symbols_qet");
    assert!(symbols_dir.exists(), "symbols_qet deve existir");

    let summary = check_canonical_catalog(&symbols_dir)
        .expect("check_canonical_catalog deve validar todas as invariantes D2 com sucesso");

    assert_eq!(summary.schema_version, 2, "Schema version deve ser 2");
    assert_eq!(
        summary.categories_count, 1018,
        "Total de categorias deve ser 1018"
    );
    assert_eq!(
        summary.elements_count, 9273,
        "Total de elementos base deve ser 9273"
    );
    assert_eq!(
        summary.svgs_checked, 9327,
        "Total de SVGs no disco deve ser 9327"
    );
    assert!(
        summary.warnings.is_empty(),
        "Não deve haver avisos na verificação D2"
    );
    assert!(
        summary.errors.is_empty(),
        "Não deve haver erros na verificação D2"
    );
}

#[test]
fn test_canonical_dictionary_lookup_and_svg_resolution() {
    let dict = global_dictionary().expect("Dicionário canônico global deve ser carregado");

    // 1. Lookup por ID 5-hex
    let motor = dict
        .get_by_id("00001")
        .expect("Elemento 00001 deve existir");
    assert_eq!(motor.id, "00001");
    assert_eq!(motor.kind, cade_simu_rs::canonical::CanonicalKind::Motor);
    assert_eq!(motor.family, "cade_native");
    assert_eq!(motor.svg_base, "000-00001-0.svg");
    assert_eq!(motor.states.len(), 2);
    assert_eq!(motor.states[0].code, "0");
    assert_eq!(motor.states[1].code, "1");

    // 2. Lookup por slug legado
    let by_slug = dict
        .get_by_slug("1000_motor_assíncrono_trifásico")
        .expect("Lookup por slug legado deve funcionar");
    assert_eq!(by_slug.id, "00001");

    // 3. Resolução de caminhos SVG para os estados
    let path_repouso = dict
        .resolve_svg_path("00001", Some('0'))
        .expect("SVG de repouso deve existir");
    assert!(path_repouso.exists());
    assert!(path_repouso.to_string_lossy().ends_with("000-00001-0.svg"));

    let path_ativo = dict
        .resolve_svg_path("00001", Some('1'))
        .expect("SVG de estado ativo deve existir");
    assert!(path_ativo.exists());
    assert!(path_ativo.to_string_lossy().ends_with("000-00001-1.svg"));

    // 4. Integração com get_parsed_svg e get_parsed_canonical_svg
    let parsed_base = cade_simu_rs::svg_symbol::get_parsed_svg("00001")
        .expect("get_parsed_svg deve carregar elemento por ID canônico");
    assert!(!parsed_base.shapes.is_empty());

    let parsed_active = cade_simu_rs::svg_symbol::get_parsed_canonical_svg("00001", '1')
        .expect("get_parsed_canonical_svg deve carregar variante de estado");
    assert!(!parsed_active.shapes.is_empty());

    // 5. Verificação em SchematicItem
    let mut item = SchematicItem::new_symbol(10, 1000, 10, 20, "-M1");
    assert_eq!(item.symbol_id, None);
    assert_eq!(item.symbol_state_code(), '0');

    item.set_symbol("00001", Some('1'));
    assert_eq!(item.symbol_id.as_deref(), Some("00001"));
    assert_eq!(item.symbol_state, Some('1'));
    assert_eq!(item.symbol_id_or_slug(), Some("00001"));
    assert_eq!(item.symbol_state_code(), '1');

    // 6. Busca textual
    let results = dict.search("trifásico");
    assert!(
        !results.is_empty(),
        "Busca textual por 'trifásico' deve retornar elementos"
    );
    assert!(results.iter().any(|e| e.id == "00001"));
}

#[test]
fn test_canonical_library_ui_and_canvas_placement() {
    let (mut app, _) = CadeSimuApp::new();

    // 1. Biblioteca unificada: seleção de família reseta a subdivisão
    let _ = app.update(Message::SelectCanonicalFamily(Some("electric".to_string())));
    assert_eq!(app.canonical_family_filter.as_deref(), Some("electric"));
    let _ = app.update(Message::SelectCanonicalKind(Some("motor".to_string())));
    assert_eq!(app.canonical_kind_filter.as_deref(), Some("motor"));

    // Ao mudar para família pneumatic, a subdivisão é resetada para None
    let _ = app.update(Message::SelectCanonicalFamily(Some(
        "pneumatic".to_string(),
    )));
    assert_eq!(app.canonical_family_filter.as_deref(), Some("pneumatic"));
    assert_eq!(app.canonical_kind_filter, None);

    // Seleciona subdivisão de pneumática
    let _ = app.update(Message::SelectCanonicalKind(Some(
        "pneu_valves".to_string(),
    )));
    assert_eq!(app.canonical_kind_filter.as_deref(), Some("pneu_valves"));

    // 2. Filtragem e busca de elementos
    let _ = app.update(Message::SelectCanonicalFamily(Some("electric".to_string())));
    let _ = app.update(Message::CanonicalSearchChanged("motor".to_string()));
    assert_eq!(app.canonical_search, "motor");
    assert_eq!(app.canonical_page, 0);

    let _ = app.update(Message::SelectCanonicalKind(Some("motor".to_string())));
    assert_eq!(app.canonical_kind_filter.as_deref(), Some("motor"));

    // 3. Paginação
    let _ = app.update(Message::NextCanonicalPage);
    let _ = app.update(Message::PrevCanonicalPage);
    assert_eq!(app.canonical_page, 0);

    // 4. Seleção simples de símbolo (atualiza preview, não arma cursor)
    let _ = app.update(Message::SelectCanonicalSymbol("00001".to_string()));
    assert_eq!(app.selected_canonical_id.as_deref(), Some("00001"));
    assert_eq!(app.tool, EditorTool::Select);

    // 5. Botão "Inserir no Esquema" arma o cursor
    let _ = app.update(Message::InsertSelectedCanonicalSymbol);
    assert_eq!(
        app.tool,
        EditorTool::PlaceCanonicalSymbol("00001".to_string())
    );

    // 6. Cancelamento via CancelAction desfaz o cursor armado
    let _ = app.update(Message::CancelAction);
    assert_eq!(app.tool, EditorTool::Select);
    assert_eq!(app.dragging_canonical_id, None);

    // 7. Duplo clique no símbolo arma o cursor diretamente
    let _ = app.update(Message::SelectCanonicalSymbol("00001".to_string()));
    let _ = app.update(Message::SelectCanonicalSymbol("00001".to_string()));
    assert_eq!(
        app.tool,
        EditorTool::PlaceCanonicalSymbol("00001".to_string())
    );

    // 8. Inserção no canvas via CanvasMessage::PlaceCanonicalComponent
    let initial_items = app.doc.items.len();
    let _ = app.update(Message::Canvas(
        cade_simu_rs::canvas_view::CanvasMessage::PlaceCanonicalComponent {
            symbol_id: "00001".to_string(),
            x: 50,
            y: 80,
        },
    ));

    assert_eq!(app.doc.items.len(), initial_items + 1);
    assert_eq!(app.tool, EditorTool::Select);
    assert!(app.is_modified);

    let placed = app.doc.items.last().expect("Item inserido");
    assert_eq!(placed.symbol_id.as_deref(), Some("00001"));
    assert_eq!(placed.x1, 50);
    assert_eq!(placed.y1, 80);
    assert!(placed.tag.starts_with("-M"));
    assert_eq!(placed.pins[0], "U1");
    assert_eq!(placed.pins[1], "V1");
    assert_eq!(placed.pins[2], "W1");
    assert_eq!(placed.pins[3], "PE");

    // 9. Desfazer (Undo) remove o elemento inserido
    let _ = app.update(Message::Undo);
    assert_eq!(app.doc.items.len(), initial_items);

    // 10. Refazer (Redo) restaura o elemento inserido
    let _ = app.update(Message::Redo);
    assert_eq!(app.doc.items.len(), initial_items + 1);

    // 11. Renderização da UI da biblioteca unificada não causa panic
    let _ = app.view();

    // 12. Verificação de escala proporcional 0.30 para elementos QET inseridos no CADe
    let parsed_motor = cade_simu_rs::svg_symbol::get_parsed_svg(
        "symbols_qet_all/electric/10_allpole/391_consumers_actuators/10_engines/induction_motor__three-phase_6_terminals.svg",
    ).expect("Motor QET 6 terminais deve existir");
    assert!(!parsed_motor.is_cade_svg);
    assert!(!parsed_motor.is_qet_native);

    // No SVG padronizado, os bornes U1, V1, W1 estão em x = -48.0, 0.0, 48.0 (distância 48 px a 300 DPI / 4 mm)
    let (u1_x, _) = parsed_motor.map_point(-48.0, -100.0, 1.0);
    let (v1_x, _) = parsed_motor.map_point(0.0, -100.0, 1.0);
    let (w1_x, _) = parsed_motor.map_point(48.0, -100.0, 1.0);
    assert!(
        (u1_x - (-6.0)).abs() < 1e-4,
        "Terminal U1 deve estar em x = -6.0"
    );
    assert!(
        (v1_x - 0.0).abs() < 1e-4,
        "Terminal V1 deve estar em x = 0.0"
    );
    assert!(
        (w1_x - 6.0).abs() < 1e-4,
        "Terminal W1 deve estar em x = 6.0"
    );
    // Passo exato de 6.0 unidades entre pólos (alinhado perfeitamente com a fiação trifásica do CADe_SIMU)
    assert!(((w1_x - v1_x) - 6.0).abs() < 1e-4);
    assert!(((v1_x - u1_x) - 6.0).abs() < 1e-4);
}

#[test]
fn test_canonical_block_classification_and_simulation_isolation() {
    let dict = global_dictionary().expect("Dicionário canônico global deve existir");

    use cade_simu_rs::canonical::CanonicalKind;

    // 1. Elementos com 0 terminais e blocos de PLC são classificados como kind: Block
    let plc_el = dict
        .get_by_id("007B2")
        .expect("Elemento PLC 007B2 deve existir");
    assert_eq!(plc_el.kind, CanonicalKind::Block);
    assert_eq!(plc_el.terminals.len(), 0);
    assert!(!plc_el.is_wireable());
    assert!(plc_el.tags.contains(&"block".to_string()));

    let rail_el = dict
        .get_by_id("00C55")
        .expect("Trilho de montagem 00C55 deve existir");
    assert_eq!(rail_el.kind, CanonicalKind::Block);
    assert_eq!(rail_el.terminals.len(), 0);
    assert!(!rail_el.is_wireable());

    // 2. Elementos funcionais de esquema são wireable
    let motor_el = dict.get_by_id("00001").expect("Motor 00001 deve existir");
    assert_eq!(motor_el.kind, CanonicalKind::Motor);
    assert!(!motor_el.terminals.is_empty());
    assert!(motor_el.is_wireable());

    // 3. Filtragem por escopo (Scope): All vs Wireable vs Block
    let all_elements = dict.filter_elements("", None, None, CanonicalScope::All);
    let wireable_elements = dict.filter_elements("", None, None, CanonicalScope::Wireable);
    let block_elements = dict.filter_elements("", None, None, CanonicalScope::Block);

    assert_eq!(all_elements.len(), 9273);
    assert_eq!(wireable_elements.len(), 5700);
    assert_eq!(block_elements.len(), 3573);
    assert_eq!(
        wireable_elements.len() + block_elements.len(),
        all_elements.len()
    );

    assert!(wireable_elements.iter().all(|e| e.is_wireable()));
    assert!(block_elements
        .iter()
        .all(|e| !e.is_wireable() && e.kind == CanonicalKind::Block));

    // 4. Inserção na folha via PlaceCanonicalComponent gera tag com prefixo "-BK" para blocos
    let (mut app, _) = CadeSimuApp::new();
    let initial_count = app.doc.items.len();
    let _ = app.update(Message::Canvas(CanvasMessage::PlaceCanonicalComponent {
        symbol_id: "007B2".to_string(),
        x: 150,
        y: 100,
    }));
    assert_eq!(app.doc.items.len(), initial_count + 1);
    let placed_block = app.doc.items.last().unwrap();
    assert!(
        placed_block.tag.starts_with("-BK"),
        "Bloco deve receber prefixo de tag -BK"
    );

    // 5. Isolamento de simulação: build_network_graph ignora itens kind: "block" (0 nós elétricos gerados)
    let hash = 42;
    let net = build_network_graph(&app.doc.items, hash);
    // O bloco colocado em (150, 100) não deve registrar nenhum pino nem ponto elétrico no grafo
    let placed_idx = app.doc.items.len() - 1;
    assert!(
        !net.item_pin_nodes.contains_key(&placed_idx),
        "Bloco visual não deve gerar nós de pino no grafo"
    );

    // 6. UI: Alternar escopo na biblioteca via SelectCanonicalScope
    let _ = app.update(Message::SelectCanonicalScope(CanonicalScope::Block));
    assert_eq!(app.canonical_scope, CanonicalScope::Block);
    let _ = app.view();

    let _ = app.update(Message::SelectCanonicalScope(CanonicalScope::Wireable));
    assert_eq!(app.canonical_scope, CanonicalScope::Wireable);
    let _ = app.view();
}

#[test]
fn test_rigid_grid_module_and_terminal_spacing_standard() {
    use cade_simu_rs::export::{export_to_svg, ExportOptions};
    use cade_simu_rs::model::*;
    use cade_simu_rs::svg_symbol::{calculate_normalized_scale, parse_svg_content};

    // 1. Constantes padronizadas do Grid rígido e dimensões métricas / 300 DPI
    assert_eq!(GRID_MODULE_MM, 4.0);
    assert_eq!(GRID_MODULE_PX_300DPI, 48.0);
    assert_eq!(GRID_MODULE_UNITS, 6.0);

    assert_eq!(SUB_GRID_MM, 2.0);
    assert_eq!(SUB_GRID_PX_300DPI, 24.0);
    assert_eq!(SUB_GRID_UNITS, 3.0);

    assert_eq!(TERMINAL_DIAMETER_PX_300DPI, 8.0);
    assert_eq!(TERMINAL_RADIUS_PX_300DPI, 4.0);
    assert_eq!(TERMINAL_DIAMETER_MM, 0.67);
    assert_eq!(TERMINAL_RADIUS_UNITS, 0.5);

    // GridDensity reflete os passos padronizados
    assert_eq!(GridDensity::Fine.step(), 3);
    assert_eq!(GridDensity::Medium.step(), 6);
    assert_eq!(GridDensity::Coarse.step(), 12);

    // 2. Normalização de escala isotrópica para bornes (Regra Rígida de 4 mm / 6 unidades)
    // Símbolo nativo CADe_SIMU (30 px entre bornes) -> escala 1:5 = 0.20
    let native_terms = vec![(0.0, 0.0), (30.0, 0.0), (60.0, 0.0)];
    let scale_native = calculate_normalized_scale(&native_terms, true);
    assert_eq!(scale_native, 0.20);
    assert_eq!(30.0 * scale_native, 6.0); // 6.0 unidades = 4 mm

    // Símbolo padrão QET (20 px entre bornes) -> escala 6.0 / 20.0 = 0.30
    let qet_20px_terms = vec![(-20.0, 0.0), (0.0, 0.0), (20.0, 0.0)];
    let scale_20px = calculate_normalized_scale(&qet_20px_terms, false);
    assert_eq!(scale_20px, 0.30);
    assert_eq!(20.0 * scale_20px, 6.0); // 6.0 unidades = 4 mm

    // Símbolo externo largo (40 px entre bornes) -> escala 6.0 / 40.0 = 0.15
    let ext_40px_terms = vec![(0.0, 0.0), (40.0, 0.0), (80.0, 0.0)];
    let scale_40px = calculate_normalized_scale(&ext_40px_terms, false);
    assert_eq!(scale_40px, 0.15);
    assert_eq!(40.0 * scale_40px, 6.0); // 6.0 unidades = 4 mm

    // Símbolo externo de grande porte (80 px entre bornes) -> escala 6.0 / 80.0 = 0.075
    let ext_80px_terms = vec![(0.0, 0.0), (80.0, 0.0)];
    let scale_80px = calculate_normalized_scale(&ext_80px_terms, false);
    assert_eq!(scale_80px, 0.075);
    assert_eq!(80.0 * scale_80px, 6.0); // 6.0 unidades = 4 mm

    // Símbolo vertical (bobina com 40 px entre bornes) -> escala 12.0 / 40.0 = 0.30
    let coil_terms = vec![(0.0, -20.0), (0.0, 20.0)];
    let scale_coil = calculate_normalized_scale(&coil_terms, false);
    assert_eq!(scale_coil, 0.30);
    assert_eq!(40.0 * scale_coil, 12.0); // 12.0 unidades = 2 módulos

    // Bloco sem terminais (visual / PLC / montagem) -> escala padrão 0.30
    let block_no_terms = vec![];
    let scale_block = calculate_normalized_scale(&block_no_terms, false);
    assert_eq!(scale_block, 0.30);

    // 3. Teste em SVG dinâmico: parse_svg_content computa e aplica a escala
    let svg_xml = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100">
        <circle class="qet-terminal" cx="10" cy="50" r="1.8" />
        <circle class="qet-terminal" cx="50" cy="50" r="1.8" />
        <circle class="qet-terminal" cx="90" cy="50" r="1.8" />
    </svg>"#;
    let parsed = parse_svg_content(svg_xml).expect("SVG com 40px pitch deve compilar");
    assert_eq!(parsed.normalized_scale, 0.15);
    let (p1, _) = parsed.map_point(10.0, 50.0, 1.0);
    let (p2, _) = parsed.map_point(50.0, 50.0, 1.0);
    assert!(
        ((p2 - p1) - 6.0).abs() < 1e-4,
        "Pitch deve ser exatamente 6.0 unidades (4 mm)"
    );

    // 4. Exportação com show_terminals inclui bornes de raio padronizado (0.5 unidades * 8.0 = 4.0 px)
    let mut doc = SchematicDocument::default();
    doc.items
        .push(SchematicItem::new_symbol(1, 1000, 50, 50, "-M1"));
    let opts = ExportOptions {
        include_cajetin: false,
        show_terminals: true,
        monochrome: false,
    };
    let svg_out = export_to_svg(&doc, None, opts);
    assert!(
        svg_out.contains("r=\"4.0\""),
        "Exportação SVG deve desenhar bornes com raio 4.0 px (diâmetro 8.0 px a 300 DPI)"
    );
}

#[test]
fn test_canonical_catalog_all_wireable_terminals_non_empty_and_unique() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let catalog_path = manifest_dir.join("symbols_qet").join("catalog.json");
    let content =
        std::fs::read_to_string(&catalog_path).expect("symbols_qet/catalog.json deve ser legível");
    let catalog: CanonicalCatalog =
        serde_json::from_str(&content).expect("catalog.json deve ter schema válido");

    let mut total_wireable = 0;
    let mut empty_term_count = 0;
    let mut dup_term_count = 0;

    for el in &catalog.elements {
        if !el.kind.is_wireable() {
            continue;
        }
        total_wireable += 1;
        let mut seen = std::collections::HashSet::new();
        for t in &el.terminals {
            if t.name.trim().is_empty() {
                empty_term_count += 1;
            }
            if !seen.insert(t.name.trim().to_string()) {
                dup_term_count += 1;
            }
        }
    }

    assert_eq!(
        total_wireable, 5700,
        "Devem existir 5.700 elementos cabáveis"
    );
    assert_eq!(
        empty_term_count, 0,
        "Zero elementos cabáveis devem possuir bornes vazios após P3-#17"
    );
    assert_eq!(
        dup_term_count, 0,
        "Zero elementos cabáveis devem possuir bornes duplicados após P3-#17"
    );
}

#[test]
fn test_canonical_terminal_normalization_rules() {
    // 1. Bobina (relay_coil): A1 no topo, A2 na base
    let mut coil_terms = vec![
        TerminalMetadata {
            name: String::new(),
            x: 0.0,
            y: 24.0,
            orientation: "s".to_string(),
        },
        TerminalMetadata {
            name: String::new(),
            x: 0.0,
            y: -24.0,
            orientation: "n".to_string(),
        },
    ];
    normalize_canonical_terminals(
        CanonicalKind::RelayCoil,
        "contactor_coil",
        "Bobina",
        "Coil",
        &mut coil_terms,
    );
    assert_eq!(coil_terms[0].name, "A2"); // y = 24.0 (base)
    assert_eq!(coil_terms[1].name, "A1"); // y = -24.0 (topo)

    // 2. Sinalizador luminoso (lamp): X1 no topo, X2 na base
    let mut lamp_terms = vec![
        TerminalMetadata {
            name: String::new(),
            x: 0.0,
            y: -20.0,
            orientation: "n".to_string(),
        },
        TerminalMetadata {
            name: String::new(),
            x: 0.0,
            y: 20.0,
            orientation: "s".to_string(),
        },
    ];
    normalize_canonical_terminals(
        CanonicalKind::Lamp,
        "pilot_lamp",
        "Lâmpada piloto",
        "Pilot lamp",
        &mut lamp_terms,
    );
    assert_eq!(lamp_terms[0].name, "X1");
    assert_eq!(lamp_terms[1].name, "X2");

    // 3. Contato térmico NA (97-98) e NF (95-96)
    let mut th_no = vec![
        TerminalMetadata {
            name: String::new(),
            x: 0.0,
            y: -20.0,
            orientation: "n".to_string(),
        },
        TerminalMetadata {
            name: String::new(),
            x: 0.0,
            y: 20.0,
            orientation: "s".to_string(),
        },
    ];
    normalize_canonical_terminals(
        CanonicalKind::ThermalContact,
        "thermal_no",
        "Contato NA térmico",
        "Thermal contact NO",
        &mut th_no,
    );
    assert_eq!(th_no[0].name, "97");
    assert_eq!(th_no[1].name, "98");

    let mut th_nc = vec![
        TerminalMetadata {
            name: String::new(),
            x: 0.0,
            y: -20.0,
            orientation: "n".to_string(),
        },
        TerminalMetadata {
            name: String::new(),
            x: 0.0,
            y: 20.0,
            orientation: "s".to_string(),
        },
    ];
    normalize_canonical_terminals(
        CanonicalKind::ThermalContact,
        "thermal_nc",
        "Contato NF térmico",
        "Thermal contact NC",
        &mut th_nc,
    );
    assert_eq!(th_nc[0].name, "95");
    assert_eq!(th_nc[1].name, "96");

    // 4. Disjuntor tripolar (circuit_breaker): 3 polos -> 1-2, 3-4, 5-6
    let mut cb_terms = vec![
        TerminalMetadata {
            name: String::new(),
            x: 48.0,
            y: -48.0,
            orientation: "n".to_string(),
        },
        TerminalMetadata {
            name: String::new(),
            x: 48.0,
            y: 48.0,
            orientation: "s".to_string(),
        },
        TerminalMetadata {
            name: String::new(),
            x: 0.0,
            y: -48.0,
            orientation: "n".to_string(),
        },
        TerminalMetadata {
            name: String::new(),
            x: 0.0,
            y: 48.0,
            orientation: "s".to_string(),
        },
        TerminalMetadata {
            name: String::new(),
            x: 24.0,
            y: -48.0,
            orientation: "n".to_string(),
        },
        TerminalMetadata {
            name: String::new(),
            x: 24.0,
            y: 48.0,
            orientation: "s".to_string(),
        },
    ];
    normalize_canonical_terminals(
        CanonicalKind::CircuitBreaker,
        "circuit_breaker_3p",
        "Disjuntor 3P",
        "Circuit breaker 3P",
        &mut cb_terms,
    );
    // Polo 1 (x=0): in 1, out 2
    assert_eq!(cb_terms[2].name, "1");
    assert_eq!(cb_terms[3].name, "2");
    // Polo 2 (x=24): in 3, out 4
    assert_eq!(cb_terms[4].name, "3");
    assert_eq!(cb_terms[5].name, "4");
    // Polo 3 (x=48): in 5, out 6
    assert_eq!(cb_terms[0].name, "5");
    assert_eq!(cb_terms[1].name, "6");

    // 5. Motor trifásico (motor): U, V, W
    let mut motor_terms = vec![
        TerminalMetadata {
            name: String::new(),
            x: 0.0,
            y: -30.0,
            orientation: "n".to_string(),
        },
        TerminalMetadata {
            name: String::new(),
            x: 24.0,
            y: -30.0,
            orientation: "n".to_string(),
        },
        TerminalMetadata {
            name: String::new(),
            x: 48.0,
            y: -30.0,
            orientation: "n".to_string(),
        },
    ];
    normalize_canonical_terminals(
        CanonicalKind::Motor,
        "motor_3p",
        "Motor trifásico",
        "Three-phase motor",
        &mut motor_terms,
    );
    assert_eq!(motor_terms[0].name, "U");
    assert_eq!(motor_terms[1].name, "V");
    assert_eq!(motor_terms[2].name, "W");

    // 6. Desduplicação estrita: múltiplos GND -> GND e GND_2
    let mut dup_terms = vec![
        TerminalMetadata {
            name: "GND".to_string(),
            x: 0.0,
            y: 0.0,
            orientation: "n".to_string(),
        },
        TerminalMetadata {
            name: "GND".to_string(),
            x: 10.0,
            y: 0.0,
            orientation: "n".to_string(),
        },
    ];
    normalize_canonical_terminals(
        CanonicalKind::Passive,
        "module",
        "Módulo",
        "Module",
        &mut dup_terms,
    );
    assert_eq!(dup_terms[0].name, "GND");
    assert_eq!(dup_terms[1].name, "GND_2");
}

#[test]
fn test_canvas_placement_canonical_pins_populated_and_editable() {
    let (mut app, _) = CadeSimuApp::new();
    let dict = global_dictionary().expect("Dicionário global deve estar carregado");

    // Encontra um elemento canônico cabável com terminais normalizados (bobina)
    let coil_el = dict
        .catalog
        .elements
        .iter()
        .find(|e| e.kind == CanonicalKind::RelayCoil && e.terminals.len() == 2)
        .expect("Deve haver uma bobina canônica com 2 terminais");

    // Coloca o componente canônico no canvas
    let initial_count = app.doc.items.len();
    let _ = app.update(Message::Canvas(CanvasMessage::PlaceCanonicalComponent {
        symbol_id: coil_el.id.clone(),
        x: 150,
        y: 100,
    }));
    assert_eq!(app.doc.items.len(), initial_count + 1);

    // Verifica que o item foi inserido no documento com os bornes preenchidos
    let placed_item = app.doc.items.last().unwrap();
    assert_eq!(
        placed_item.pins[0], coil_el.terminals[0].name,
        "Borne 1 do item deve herdar nome normalizado da bobina"
    );
    assert_eq!(
        placed_item.pins[1], coil_el.terminals[1].name,
        "Borne 2 do item deve herdar nome normalizado da bobina"
    );
    assert_eq!(placed_item.pins[0], "A1");
    assert_eq!(placed_item.pins[1], "A2");

    // Edita o Borne 0 via mensagem do painel de propriedades
    let sel_id = placed_item.id;
    app.selected_id = Some(sel_id);
    let _ = app.update(Message::EditPin(0, "A1_CUSTOM".to_string()));

    let updated_item = app.doc.items.iter().find(|i| i.id == sel_id).unwrap();
    assert_eq!(
        updated_item.pins[0], "A1_CUSTOM",
        "Edição de borne deve ser persistida no item selecionado"
    );
}

#[test]
fn test_p2_15_multipage_cross_reference_and_orphan_detection() {
    use cade_simu_rs::xref::collect_project_xrefs;

    // Folha 1: Circuito de Potência
    let mut doc_power = SchematicDocument::default();
    let mut breaker = SchematicItem::new_symbol(1, 6000, 100, 100, "-Q1");
    breaker.pins[0] = "1".to_string();
    breaker.pins[1] = "2".to_string();
    breaker.pins[2] = "3".to_string();
    breaker.pins[3] = "4".to_string();
    doc_power.items.push(breaker);

    let mut contactor_poles = SchematicItem::new_symbol(2, 2000, 100, 200, "-KM1");
    contactor_poles.pins[0] = "1".to_string();
    contactor_poles.pins[1] = "2".to_string();
    contactor_poles.pins[2] = "3".to_string();
    contactor_poles.pins[3] = "4".to_string();
    contactor_poles.pins[4] = "5".to_string();
    contactor_poles.pins[5] = "6".to_string();
    doc_power.items.push(contactor_poles);

    let mut motor = SchematicItem::new_symbol(3, 1000, 100, 300, "-M1");
    motor.pins[0] = "U".to_string();
    motor.pins[1] = "V".to_string();
    motor.pins[2] = "W".to_string();
    doc_power.items.push(motor);

    // Folha 2: Circuito de Comando
    let mut doc_control = SchematicDocument::default();
    let mut coil_km1 = SchematicItem::new_symbol(4, 9000, 150, 100, "-KM1");
    coil_km1.pins[0] = "A1".to_string();
    coil_km1.pins[1] = "A2".to_string();
    doc_control.items.push(coil_km1);

    let mut aux_no_km1 = SchematicItem::new_symbol(5, 7000, 150, 200, "-KM1");
    aux_no_km1.pins[0] = "13".to_string();
    aux_no_km1.pins[1] = "14".to_string();
    doc_control.items.push(aux_no_km1);

    let mut orphan_contact = SchematicItem::new_symbol(6, 7000, 150, 300, "-KM2");
    orphan_contact.pins[0] = "13".to_string();
    orphan_contact.pins[1] = "14".to_string();
    doc_control.items.push(orphan_contact);

    let pages = vec![
        ProjectPage::new(1, "Circuito de Força".to_string(), doc_power.clone()),
        ProjectPage::new(2, "Circuito de Comando".to_string(), doc_control.clone()),
    ];

    let dir = collect_project_xrefs(&pages, 0, &doc_power);
    assert_eq!(
        dir.total_pages, 2,
        "Projeto deve conter exatamente 2 folhas"
    );

    // Grupo -KM1: Bobina na Folha 2 (índice 1) e polos/contatos em ambas as folhas
    let km1_group = dir
        .groups
        .iter()
        .find(|g| g.tag == "-KM1")
        .expect("Grupo -KM1 deve existir");
    assert!(
        km1_group.primary.is_some(),
        "-KM1 deve ter uma bobina primária"
    );
    assert_eq!(
        km1_group.primary.as_ref().unwrap().page_index,
        1,
        "Bobina de -KM1 deve estar na Folha 2"
    );
    assert!(
        km1_group
            .primary
            .as_ref()
            .unwrap()
            .grid_ref
            .starts_with("2."),
        "Coordenada da bobina deve ter prefixo de página 2."
    );
    assert_eq!(
        km1_group.contacts.len(),
        2,
        "-KM1 deve ter 2 contatos subordinados (polos de força + contato aux NA)"
    );

    let has_page_1_contact = km1_group.contacts.iter().any(|c| c.page_index == 0);
    let has_page_2_contact = km1_group.contacts.iter().any(|c| c.page_index == 1);
    assert!(
        has_page_1_contact,
        "Deve haver polo de força de -KM1 na Folha 1"
    );
    assert!(
        has_page_2_contact,
        "Deve haver contato auxiliar de -KM1 na Folha 2"
    );
    assert!(
        !km1_group.is_orphan_contact,
        "-KM1 possui bobina, não é órfão"
    );
    assert!(
        !km1_group.has_duplicate_primary,
        "-KM1 não possui bobina duplicada"
    );

    // Grupo -KM2: Contato órfão (sem bobina no projeto)
    let km2_group = dir
        .groups
        .iter()
        .find(|g| g.tag == "-KM2")
        .expect("Grupo -KM2 deve existir");
    assert!(km2_group.primary.is_none(), "-KM2 não possui bobina");
    assert!(
        km2_group.is_orphan_contact,
        "-KM2 deve ser identificado como contato órfão"
    );
}

#[test]
fn test_p2_15_electrical_connections_parallel_and_series_analysis() {
    use cade_simu_rs::xref::{analyze_pin_connections, ElectricalConnectionType};

    let doc = cade_simu_rs::io::build_dol_starter_preset();

    // Na partida direta padrão:
    // - Item id 23: Botoeira Liga -S1 (NA 13-14)
    // - Item id 24: Contato de Selo -KM1 (NA 13-14)
    // - Item id 25: Bobina -KM1 (A1-A2)
    // O contato de selo e o botão liga estão em PARALELO entre si.
    // O botão liga e o selo estão em SÉRIE com a bobina.
    let conn_seal = analyze_pin_connections(24, &doc, 0);
    assert!(
        !conn_seal.is_empty(),
        "Contato de selo deve ter conexões mapeadas"
    );

    // Borne 13 do selo deve estar em paralelo com o botão liga -S1
    let pin_13_summary = conn_seal
        .iter()
        .find(|p| p.pin_label == "13")
        .expect("Borne 13");
    let parallel_to_s1 = pin_13_summary
        .targets
        .iter()
        .any(|t| t.tag.contains("S1") && t.conn_type == ElectricalConnectionType::Parallel);
    assert!(
        parallel_to_s1,
        "Borne 13 do selo -KM1 deve estar em paralelo com a botoeira -S1"
    );

    // Borne 14 do selo deve estar conectado à bobina -KM1:A1 em série
    let pin_14_summary = conn_seal
        .iter()
        .find(|p| p.pin_label == "14")
        .expect("Borne 14");
    let series_to_coil = pin_14_summary.targets.iter().any(|t| {
        t.tag.contains("KM1")
            && t.pin_label == "A1"
            && t.conn_type == ElectricalConnectionType::Series
    });
    assert!(
        series_to_coil,
        "Borne 14 do selo deve estar em série com o terminal A1 da bobina -KM1"
    );
}

#[test]
fn test_p2_15_terminal_strips_iec_81346_classification_and_renumbering() {
    use cade_simu_rs::xref::{collect_project_xrefs, renumber_terminal_strip, TerminalStripKind};

    let mut doc = SchematicDocument::default();

    // Cria bornes de potência (-XP) com numeração desordenada e repetida
    let mut t_p1 = SchematicItem::new_symbol(1, 7001, 100, 300, "-XP");
    t_p1.pins[0] = "9".to_string();
    doc.items.push(t_p1);

    let mut t_p2 = SchematicItem::new_symbol(2, 7001, 100, 100, "-XP");
    t_p2.pins[0] = "9".to_string(); // duplicado
    doc.items.push(t_p2);

    let mut t_p3 = SchematicItem::new_symbol(3, 7001, 100, 200, "-XP");
    t_p3.pins[0] = "42".to_string();
    doc.items.push(t_p3);

    // Cria borne de comando (-XC)
    let mut t_c1 = SchematicItem::new_symbol(4, 7001, 200, 100, "-XC");
    t_c1.pins[0] = "1".to_string();
    doc.items.push(t_c1);

    // Cria borne de medição (-XT)
    let mut t_m1 = SchematicItem::new_symbol(5, 7001, 300, 100, "-XT");
    t_m1.pins[0] = "1".to_string();
    doc.items.push(t_m1);

    let mut pages = vec![ProjectPage::new(1, "Painel Geral".to_string(), doc.clone())];

    let dir_before = collect_project_xrefs(&pages, 0, &doc);
    assert_eq!(
        dir_before.terminal_strips.len(),
        3,
        "Devem existir 3 réguas (-XC, -XP, -XT)"
    );

    let strip_xp = dir_before
        .terminal_strips
        .iter()
        .find(|s| s.strip_tag == "-XP")
        .unwrap();
    assert_eq!(
        strip_xp.kind,
        TerminalStripKind::Power,
        "-XP deve ser classificada como Potência (IEC 81346)"
    );
    assert!(
        strip_xp.has_duplicates,
        "Régua -XP deve reportar bornes com numeração repetida"
    );

    let strip_xc = dir_before
        .terminal_strips
        .iter()
        .find(|s| s.strip_tag == "-XC")
        .unwrap();
    assert_eq!(
        strip_xc.kind,
        TerminalStripKind::Control,
        "-XC deve ser classificada como Comando (IEC 81346)"
    );

    let strip_xt = dir_before
        .terminal_strips
        .iter()
        .find(|s| s.strip_tag == "-XT")
        .unwrap();
    assert_eq!(
        strip_xt.kind,
        TerminalStripKind::Measurement,
        "-XT deve ser classificada como Medição (IEC 81346)"
    );

    // Executa renumbração sequencial automática da régua -XP
    renumber_terminal_strip("-XP", &mut pages, 0, &mut doc);

    let dir_after = collect_project_xrefs(&pages, 0, &doc);
    let strip_xp_after = dir_after
        .terminal_strips
        .iter()
        .find(|s| s.strip_tag == "-XP")
        .unwrap();
    assert!(
        !strip_xp_after.has_duplicates,
        "Após renumbração, -XP não deve ter duplicatas"
    );
    assert_eq!(strip_xp_after.terminals.len(), 3);
    assert_eq!(strip_xp_after.terminals[0].terminal_number, "1");
    assert_eq!(strip_xp_after.terminals[1].terminal_number, "2");
    assert_eq!(strip_xp_after.terminals[2].terminal_number, "3");
}

#[test]
fn test_p2_15_xref_navigation_message_switches_page_and_selects_item() {
    let (mut app, _) = CadeSimuApp::new();

    // Cria folha 2 com um elemento alvo
    let mut doc2 = SchematicDocument::default();
    let target_item = SchematicItem::new_symbol(42, 9000, 120, 180, "-KM1");
    doc2.items.push(target_item);

    app.pages
        .push(ProjectPage::new(2, "Folha 2".to_string(), doc2));
    assert_eq!(app.pages.len(), 2);
    assert_eq!(app.active_page_index, 0);

    // Emite mensagem de navegação inter-folhas para o elemento #42 na Folha 2 (índice 1)
    let _ = app.update(Message::NavigateToElement {
        page_index: 1,
        item_id: 42,
    });

    assert_eq!(
        app.active_page_index, 1,
        "Página ativa deve mudar para a Folha 2"
    );
    assert_eq!(
        app.selected_id,
        Some(42),
        "Elemento #42 deve estar selecionado"
    );
    assert!(
        app.status_msg.contains("Navegado para elemento #42"),
        "Mensagem de status deve confirmar navegação"
    );
}

#[test]
fn test_multiple_selection_model_and_app_actions() {
    let (mut app, _) = CadeSimuApp::new();
    app.doc.items.clear();

    // Adiciona 3 itens no documento
    let it1 = SchematicItem::new_symbol(101, 9000, 30, 30, "-KM1");
    let it2 = SchematicItem::new_symbol(102, 9000, 60, 30, "-KM2");
    let it3 = SchematicItem::new_symbol(103, 9000, 90, 30, "-KM3");
    app.doc.items.push(it1);
    app.doc.items.push(it2);
    app.doc.items.push(it3);

    // 1. Seleção individual inicial
    app.select_single(Some(101));
    assert_eq!(app.selected_id, Some(101));
    assert_eq!(app.selected_ids.len(), 1);
    assert!(app.selected_ids.contains(&101));

    // 2. Toggle selection (Shift-click simulado no item 102)
    app.toggle_selection(102);
    assert_eq!(app.selected_ids.len(), 2);
    assert!(app.selected_ids.contains(&101));
    assert!(app.selected_ids.contains(&102));

    // 3. Toggle selection no item 101 (remove 101)
    app.toggle_selection(101);
    assert_eq!(app.selected_ids.len(), 1);
    assert!(!app.selected_ids.contains(&101));
    assert!(app.selected_ids.contains(&102));
    assert_eq!(app.selected_id, Some(102));

    // 4. SelectAll via Message::SelectAll
    let _ = app.update(Message::SelectAll);
    assert_eq!(app.selected_ids.len(), 3);
    assert!(app.selected_ids.contains(&101));
    assert!(app.selected_ids.contains(&102));
    assert!(app.selected_ids.contains(&103));

    // 5. InvertSelection via Message::InvertSelection (deve ficar vazio se todos estavam selecionados)
    let _ = app.update(Message::InvertSelection);
    assert_eq!(app.selected_ids.len(), 0);
    assert_eq!(app.selected_id, None);

    // 6. Seleciona 101 e 103 e inverte (deve selecionar 102)
    app.select_multiple(vec![101, 103], false);
    assert_eq!(app.selected_ids.len(), 2);
    let _ = app.update(Message::InvertSelection);
    assert_eq!(app.selected_ids.len(), 1);
    assert!(app.selected_ids.contains(&102));

    // 7. DeleteSelected com múltiplos itens
    app.select_multiple(vec![101, 102], false);
    assert_eq!(app.selected_ids.len(), 2);
    let _ = app.update(Message::DeleteSelected);
    assert_eq!(app.selected_ids.len(), 0);
    assert_eq!(app.selected_id, None);
    assert_eq!(app.doc.items.len(), 1);
    assert_eq!(app.doc.items[0].id, 103);
}

#[test]
fn test_group_id_persistence_in_json_and_toml() {
    let mut doc = SchematicDocument::default();
    let mut it1 = SchematicItem::new_symbol(1, 9000, 30, 30, "-KM1");
    let mut it2 = SchematicItem::new_symbol(2, 9000, 60, 30, "-KM2");
    let it3 = SchematicItem::new_symbol(3, 9000, 90, 30, "-KM3");

    // it1 e it2 pertencem ao grupo 42; it3 não tem grupo
    it1.group_id = Some(42);
    it2.group_id = Some(42);

    doc.items.push(it1);
    doc.items.push(it2);
    doc.items.push(it3);

    // Teste Roundtrip JSON
    let json_str = serde_json::to_string_pretty(&doc).expect("Serialização JSON falhou");
    assert!(json_str.contains("\"group_id\": 42"));

    let doc_json: SchematicDocument =
        serde_json::from_str(&json_str).expect("Deserialização JSON falhou");
    assert_eq!(doc_json.items[0].group_id, Some(42));
    assert_eq!(doc_json.items[1].group_id, Some(42));
    assert_eq!(doc_json.items[2].group_id, None);

    // Teste Roundtrip TOML
    let toml_str = toml::to_string_pretty(&doc).expect("Serialização TOML falhou");
    assert!(toml_str.contains("group_id = 42"));

    let doc_toml: SchematicDocument =
        toml::from_str(&toml_str).expect("Deserialização TOML falhou");
    assert_eq!(doc_toml.items[0].group_id, Some(42));
    assert_eq!(doc_toml.items[1].group_id, Some(42));
    assert_eq!(doc_toml.items[2].group_id, None);
}

#[test]
fn test_marquee_box_selection_bounds_and_canvas_messages() {
    let (mut app, _) = CadeSimuApp::new();
    app.doc.items.clear();

    let it1 = SchematicItem::new_symbol(1, 9000, 30, 30, "-KM1");
    let it2 = SchematicItem::new_symbol(2, 9000, 150, 150, "-KM2");
    let wire = SchematicItem::new_wire(3, 4000, 20, 20, 40, 20);

    app.doc.items.push(it1);
    app.doc.items.push(it2);
    app.doc.items.push(wire);

    // Verifica limites dos elementos
    let (min_x, min_y, max_x, max_y) = SchematicCanvas::item_bounds(&app.doc.items[0]);
    assert!(min_x <= 30 && max_x >= 30);
    assert!(min_y <= 30 && max_y >= 30);

    // Emite mensagem de seleção por caixa de arraste (marquee) cobrindo a região (10, 10) até (50, 50)
    let _ = app.update(Message::Canvas(CanvasMessage::SelectItems {
        ids: vec![1, 3],
        additive: false,
    }));

    assert_eq!(app.selected_ids.len(), 2);
    assert!(app.selected_ids.contains(&1));
    assert!(app.selected_ids.contains(&3));
    assert!(!app.selected_ids.contains(&2));

    // Agora adiciona it2 de forma cumulativa (Shift-marquee)
    let _ = app.update(Message::Canvas(CanvasMessage::SelectItems {
        ids: vec![2],
        additive: true,
    }));

    assert_eq!(app.selected_ids.len(), 3);
    assert!(app.selected_ids.contains(&1));
    assert!(app.selected_ids.contains(&2));
    assert!(app.selected_ids.contains(&3));
}

#[test]
fn test_group_and_ungroup_commands_with_undo_redo() {
    let (mut app, _) = CadeSimuApp::new();
    app.doc.items.clear();

    let it1 = SchematicItem::new_symbol(1, 9000, 30, 30, "-KM1");
    let it2 = SchematicItem::new_symbol(2, 9000, 50, 30, "-KM2");
    let it3 = SchematicItem::new_symbol(3, 9000, 70, 30, "-KM3");

    app.doc.items.push(it1);
    app.doc.items.push(it2);
    app.doc.items.push(it3);

    // Seleciona it1 e it2
    app.select_multiple([1, 2], false);
    assert_eq!(app.selected_ids.len(), 2);

    // Executa comando Agrupar (Ctrl+G)
    let _ = app.update(Message::GroupSelected);

    let g1 = app.doc.items.iter().find(|i| i.id == 1).unwrap().group_id;
    let g2 = app.doc.items.iter().find(|i| i.id == 2).unwrap().group_id;
    let g3 = app.doc.items.iter().find(|i| i.id == 3).unwrap().group_id;

    assert!(g1.is_some(), "it1 deve possuir group_id atribuído");
    assert_eq!(g1, g2, "it1 e it2 devem pertencer ao mesmo grupo");
    assert_eq!(g3, None, "it3 não deve pertencer a nenhum grupo");
    assert!(
        app.is_modified,
        "Documento deve ser marcado como modificado"
    );

    // Desfaz o agrupamento (Undo / Ctrl+Z)
    let _ = app.update(Message::Undo);
    assert_eq!(
        app.doc.items.iter().find(|i| i.id == 1).unwrap().group_id,
        None
    );
    assert_eq!(
        app.doc.items.iter().find(|i| i.id == 2).unwrap().group_id,
        None
    );

    // Refaz o agrupamento (Redo / Ctrl+Shift+Z)
    let _ = app.update(Message::Redo);
    let g1_after_redo = app.doc.items.iter().find(|i| i.id == 1).unwrap().group_id;
    let g2_after_redo = app.doc.items.iter().find(|i| i.id == 2).unwrap().group_id;
    assert_eq!(g1_after_redo, g1);
    assert_eq!(g2_after_redo, g2);

    // Agora seleciona o grupo e executa Desagrupar (Ctrl+U)
    app.select_single(Some(1));
    assert_eq!(app.selected_ids.len(), 2);
    let _ = app.update(Message::UngroupSelected);

    assert_eq!(
        app.doc.items.iter().find(|i| i.id == 1).unwrap().group_id,
        None
    );
    assert_eq!(
        app.doc.items.iter().find(|i| i.id == 2).unwrap().group_id,
        None
    );

    // Desfaz o desagrupamento -> volta a ser grupo
    let _ = app.update(Message::Undo);
    assert!(app
        .doc
        .items
        .iter()
        .find(|i| i.id == 1)
        .unwrap()
        .group_id
        .is_some());
    assert!(app
        .doc
        .items
        .iter()
        .find(|i| i.id == 2)
        .unwrap()
        .group_id
        .is_some());
}

#[test]
fn test_smart_group_selection_propagation() {
    let (mut app, _) = CadeSimuApp::new();
    app.doc.items.clear();

    let mut it1 = SchematicItem::new_symbol(1, 9000, 30, 30, "-KM1");
    it1.group_id = Some(10);
    let mut it2 = SchematicItem::new_symbol(2, 9000, 50, 30, "-KM2");
    it2.group_id = Some(10);
    let it3 = SchematicItem::new_symbol(3, 9000, 70, 30, "-KM3");

    app.doc.items.push(it1);
    app.doc.items.push(it2);
    app.doc.items.push(it3);

    // 1. Clicar em it1 deve selecionar automaticamente todos os membros do grupo 10 (it1 e it2)
    app.select_single(Some(1));
    assert_eq!(app.selected_ids.len(), 2);
    assert!(app.selected_ids.contains(&1));
    assert!(app.selected_ids.contains(&2));
    assert!(!app.selected_ids.contains(&3));
    assert_eq!(app.selected_id, Some(1));

    // 2. Clicar em it3 (independente) limpa a seleção do grupo e seleciona apenas it3
    app.select_single(Some(3));
    assert_eq!(app.selected_ids.len(), 1);
    assert!(app.selected_ids.contains(&3));
    assert_eq!(app.selected_id, Some(3));

    // 3. Shift-clique (toggle_selection) em it2 com it3 selecionado:
    // Deve adicionar o grupo 10 inteiro (it1 e it2) à seleção, resultando em 3 itens selecionados
    app.toggle_selection(2);
    assert_eq!(app.selected_ids.len(), 3);
    assert!(app.selected_ids.contains(&1));
    assert!(app.selected_ids.contains(&2));
    assert!(app.selected_ids.contains(&3));

    // 4. Shift-clique novamente em it1:
    // Deve remover o grupo 10 inteiro da seleção, mantendo apenas it3
    app.toggle_selection(1);
    assert_eq!(app.selected_ids.len(), 1);
    assert!(app.selected_ids.contains(&3));
    assert!(!app.selected_ids.contains(&1));
    assert!(!app.selected_ids.contains(&2));
}

#[test]
fn test_marquee_group_expansion() {
    let (mut app, _) = CadeSimuApp::new();
    app.doc.items.clear();

    let mut it1 = SchematicItem::new_symbol(1, 9000, 30, 30, "-KM1");
    it1.group_id = Some(7);
    let mut it2 = SchematicItem::new_symbol(2, 9000, 60, 30, "-KM2");
    it2.group_id = Some(7);
    let it3 = SchematicItem::new_symbol(3, 9000, 90, 30, "-KM3");

    app.doc.items.push(it1);
    app.doc.items.push(it2);
    app.doc.items.push(it3);

    // Marquee seleciona apenas o id 1, mas como id 1 pertence ao grupo 7,
    // a seleção deve expandir automaticamente para conter id 1 e id 2
    app.select_multiple([1], false);
    assert_eq!(app.selected_ids.len(), 2);
    assert!(app.selected_ids.contains(&1));
    assert!(app.selected_ids.contains(&2));
    assert!(!app.selected_ids.contains(&3));

    // Marquee cumulativo com id 3
    app.select_multiple([3], true);
    assert_eq!(app.selected_ids.len(), 3);
    assert!(app.selected_ids.contains(&1));
    assert!(app.selected_ids.contains(&2));
    assert!(app.selected_ids.contains(&3));
}

#[test]
fn test_group_minimum_elements_requirement() {
    let (mut app, _) = CadeSimuApp::new();
    app.doc.items.clear();

    let it1 = SchematicItem::new_symbol(1, 9000, 30, 30, "-KM1");
    app.doc.items.push(it1);

    // Tentativa de agrupar com 0 elementos selecionados
    app.deselect_all();
    let _ = app.update(Message::GroupSelected);
    assert!(app.status_msg.contains("ao menos 2 elementos"));
    assert_eq!(app.doc.items[0].group_id, None);

    // Tentativa de agrupar com 1 único elemento selecionado
    app.select_single(Some(1));
    let _ = app.update(Message::GroupSelected);
    assert!(app.status_msg.contains("ao menos 2 elementos"));
    assert_eq!(app.doc.items[0].group_id, None);

    // Tentativa de desagrupar quando nenhum grupo existe na seleção
    let _ = app.update(Message::UngroupSelected);
    assert!(app.status_msg.contains("Nenhum grupo encontrado"));
}

#[test]
fn test_group_atomic_move_and_wire_tracking() {
    let (mut app, _) = CadeSimuApp::new();
    app.doc.items.clear();

    let mut it1 = SchematicItem::new_symbol(1, 9000, 30, 30, "-KM1");
    it1.group_id = Some(1);
    let mut it2 = SchematicItem::new_symbol(2, 9000, 50, 30, "-KM2");
    it2.group_id = Some(1);
    let mut wire_int = SchematicItem::new_wire(3, 4000, 30, 30, 50, 30);
    wire_int.group_id = Some(1);
    let wire_ext = SchematicItem::new_wire(4, 4000, 10, 30, 30, 30); // Fio externo conectado em (30, 30)

    app.doc.items.push(it1);
    app.doc.items.push(it2);
    app.doc.items.push(wire_int);
    app.doc.items.push(wire_ext);

    // Clicar em it1 seleciona todo o grupo (itens 1, 2 e 3)
    app.select_single(Some(1));
    assert_eq!(app.selected_ids.len(), 3);
    assert!(app.selected_ids.contains(&1));
    assert!(app.selected_ids.contains(&2));
    assert!(app.selected_ids.contains(&3));
    assert!(!app.selected_ids.contains(&4));

    // Move o grupo arrastando it1 de (30, 30) para (40, 40) (dx = +10, dy = +10)
    let _ = app.update(Message::Canvas(CanvasMessage::MoveItem {
        id: 1,
        x1: 40,
        y1: 40,
    }));
    let _ = app.update(Message::Canvas(CanvasMessage::EndMoveItem { id: 1 }));

    // Verifica que it1 foi para (40, 40)
    let item1 = app.doc.items.iter().find(|i| i.id == 1).unwrap();
    assert_eq!(item1.x1, 40);
    assert_eq!(item1.y1, 40);

    // Verifica que it2 acompanhou para (60, 40)
    let item2 = app.doc.items.iter().find(|i| i.id == 2).unwrap();
    assert_eq!(item2.x1, 60);
    assert_eq!(item2.y1, 40);

    // Verifica que o fio interno acompanhou o grupo para (40, 40) -> (60, 40)
    let w_int = app.doc.items.iter().find(|i| i.id == 3).unwrap();
    assert_eq!(w_int.x1, 40);
    assert_eq!(w_int.y1, 40);
    assert_eq!(w_int.x2, 60);
    assert_eq!(w_int.y2, 40);

    // Verifica que o fio externo esticou mantendo conexão com it1 em (40, 40)
    let w_ext = app.doc.items.iter().find(|i| i.id == 4).unwrap();
    assert_eq!(w_ext.x1, 10);
    assert_eq!(w_ext.y1, 30);
    assert_eq!(w_ext.x2, 40);
    assert_eq!(w_ext.y2, 40);

    // Desfaz o movimento via Undo
    let _ = app.update(Message::Undo);
    let item1_undo = app.doc.items.iter().find(|i| i.id == 1).unwrap();
    assert_eq!(item1_undo.x1, 30);
    assert_eq!(item1_undo.y1, 30);
    let item2_undo = app.doc.items.iter().find(|i| i.id == 2).unwrap();
    assert_eq!(item2_undo.x1, 50);
    assert_eq!(item2_undo.y1, 30);
}

#[test]
fn test_group_atomic_rotation_around_centroid() {
    let (mut app, _) = CadeSimuApp::new();
    app.doc.items.clear();

    let mut it1 = SchematicItem::new_symbol(1, 9000, 100, 100, "-KM1");
    it1.group_id = Some(1);
    let mut it2 = SchematicItem::new_symbol(2, 9000, 120, 100, "-KM2");
    it2.group_id = Some(1);

    app.doc.items.push(it1);
    app.doc.items.push(it2);

    app.select_single(Some(1));
    assert_eq!(app.selected_ids.len(), 2);

    // Rotaciona 90 graus no sentido horário em torno do baricentro (110, 100)
    let _ = app.update(Message::RotateClockwise);

    let item1 = app.doc.items.iter().find(|i| i.id == 1).unwrap();
    let item2 = app.doc.items.iter().find(|i| i.id == 2).unwrap();

    assert_eq!(item1.x1, 110);
    assert_eq!(item1.y1, 90);
    assert_eq!(item1.orientation, 1);

    assert_eq!(item2.x1, 110);
    assert_eq!(item2.y1, 110);
    assert_eq!(item2.orientation, 1);

    // Rotaciona 90 graus no sentido anti-horário -> retorna à posição inicial
    let _ = app.update(Message::RotateCounterClockwise);

    let item1_rev = app.doc.items.iter().find(|i| i.id == 1).unwrap();
    let item2_rev = app.doc.items.iter().find(|i| i.id == 2).unwrap();

    assert_eq!(item1_rev.x1, 100);
    assert_eq!(item1_rev.y1, 100);
    assert_eq!(item1_rev.orientation, 0);

    assert_eq!(item2_rev.x1, 120);
    assert_eq!(item2_rev.y1, 100);
    assert_eq!(item2_rev.orientation, 0);
}

#[test]
fn test_group_atomic_mirror_horizontal_and_vertical() {
    let (mut app, _) = CadeSimuApp::new();
    app.doc.items.clear();

    let mut it1 = SchematicItem::new_symbol(1, 9000, 100, 100, "-KM1");
    it1.group_id = Some(2);
    let mut it2 = SchematicItem::new_symbol(2, 9000, 120, 100, "-KM2");
    it2.group_id = Some(2);

    app.doc.items.push(it1);
    app.doc.items.push(it2);

    app.select_single(Some(1));

    // Espelha horizontalmente em torno de cx = 110
    let _ = app.update(Message::MirrorHorizontal);

    let item1 = app.doc.items.iter().find(|i| i.id == 1).unwrap();
    let item2 = app.doc.items.iter().find(|i| i.id == 2).unwrap();

    assert_eq!(item1.x1, 120);
    assert_eq!(item1.orientation, 4);

    assert_eq!(item2.x1, 100);
    assert_eq!(item2.orientation, 4);

    // Espelha horizontalmente novamente -> volta para 100 e 120 com orientação 0
    let _ = app.update(Message::MirrorHorizontal);
    let item1_orig = app.doc.items.iter().find(|i| i.id == 1).unwrap();
    let item2_orig = app.doc.items.iter().find(|i| i.id == 2).unwrap();

    assert_eq!(item1_orig.x1, 100);
    assert_eq!(item1_orig.orientation, 0);

    assert_eq!(item2_orig.x1, 120);
    assert_eq!(item2_orig.orientation, 0);
}

#[test]
fn test_group_atomic_duplicate_creates_new_group() {
    let (mut app, _) = CadeSimuApp::new();
    app.doc.items.clear();

    let mut it1 = SchematicItem::new_symbol(1, 9000, 30, 30, "-KM1");
    it1.group_id = Some(5);
    let mut it2 = SchematicItem::new_symbol(2, 9000, 50, 30, "-KM2");
    it2.group_id = Some(5);

    app.doc.items.push(it1);
    app.doc.items.push(it2);

    app.select_single(Some(1));
    assert_eq!(app.selected_ids.len(), 2);

    // Executa Duplicar (Ctrl+D)
    let _ = app.update(Message::Duplicate);

    assert_eq!(app.doc.items.len(), 4);
    assert_eq!(app.selected_ids.len(), 2);

    // Os itens selecionados agora devem ser as duplicatas
    let orig_g1 = app.doc.items.iter().find(|i| i.id == 1).unwrap().group_id;
    let orig_g2 = app.doc.items.iter().find(|i| i.id == 2).unwrap().group_id;
    assert_eq!(orig_g1, Some(5));
    assert_eq!(orig_g2, Some(5));

    let dup_ids: Vec<i32> = app.selected_ids.iter().copied().collect();
    assert!(!dup_ids.contains(&1));
    assert!(!dup_ids.contains(&2));

    let dup_g1 = app
        .doc
        .items
        .iter()
        .find(|i| i.id == dup_ids[0])
        .unwrap()
        .group_id;
    let dup_g2 = app
        .doc
        .items
        .iter()
        .find(|i| i.id == dup_ids[1])
        .unwrap()
        .group_id;

    assert!(dup_g1.is_some());
    assert_eq!(
        dup_g1, dup_g2,
        "As duplicatas devem compartilhar o mesmo novo grupo"
    );
    assert_ne!(
        dup_g1,
        Some(5),
        "O novo grupo deve ser diferente do grupo original"
    );

    // Desfaz a duplicação
    let _ = app.update(Message::Undo);
    assert_eq!(app.doc.items.len(), 2);
}

#[test]
fn test_group_atomic_cut_copy_paste_roundtrip() {
    let (mut app, _) = CadeSimuApp::new();
    app.doc.items.clear();

    let mut it1 = SchematicItem::new_symbol(1, 9000, 30, 30, "-KM1");
    it1.group_id = Some(3);
    let mut it2 = SchematicItem::new_symbol(2, 9000, 50, 30, "-KM2");
    it2.group_id = Some(3);

    app.doc.items.push(it1);
    app.doc.items.push(it2);

    app.select_single(Some(1));
    assert_eq!(app.selected_ids.len(), 2);

    // Copiar
    let _ = app.update(Message::Copy);
    assert!(app.clipboard.is_some());
    assert_eq!(app.clipboard.as_ref().unwrap().len(), 2);

    // Colar
    let _ = app.update(Message::Paste);
    assert_eq!(app.doc.items.len(), 4);
    assert_eq!(app.selected_ids.len(), 2);

    // Recortar as cópias recém-coladas
    let _ = app.update(Message::Cut);
    assert_eq!(app.doc.items.len(), 2);
    assert_eq!(app.clipboard.as_ref().unwrap().len(), 2);

    // Desfaz o recorte
    let _ = app.update(Message::Undo);
    assert_eq!(app.doc.items.len(), 4);
}

#[test]
fn test_toolbar_select_and_pan_tools_exclusivity() {
    let (mut app, _) = CadeSimuApp::new();
    assert_eq!(app.tool, EditorTool::Select);

    // 1. Mudar para o modo Mover (Pan)
    let _ = app.update(Message::SelectTool(EditorTool::Pan));
    assert_eq!(app.tool, EditorTool::Pan);
    assert!(app.status_msg.contains("Modo Mover"));

    // 2. Mudar de volta para o modo Seleção
    let _ = app.update(Message::SelectTool(EditorTool::Select));
    assert_eq!(app.tool, EditorTool::Select);
    assert!(app.status_msg.contains("Modo Seleção"));

    // 3. Em modo simulação, selecionar Pan não deve parar a simulação
    let _ = app.update(Message::ToggleSimulation);
    assert!(app.sim_running);
    let _ = app.update(Message::SelectTool(EditorTool::Pan));
    assert!(
        app.sim_running,
        "Pan não deve cancelar simulação em execução"
    );

    // 4. Mudar para ferramenta de desenho deve encerrar simulação
    let _ = app.update(Message::SelectTool(EditorTool::DrawWire(4000)));
    assert!(!app.sim_running);
}

#[test]
fn test_toolbar_text_visibility_dropdown() {
    let (mut app, _) = CadeSimuApp::new();
    assert!(app.show_texts);

    // 1. Desativar exibição de textos via dropdown
    let _ = app.update(Message::SetTextVisibility(TextVisibility::Off));
    assert!(!app.show_texts);
    assert!(app.status_msg.contains("desativada"));

    // 2. Reativar via dropdown
    let _ = app.update(Message::SetTextVisibility(TextVisibility::On));
    assert!(app.show_texts);
    assert!(app.status_msg.contains("ativada"));

    // 3. Alternar via atalho/toggle
    let _ = app.update(Message::ToggleShowTexts);
    assert!(!app.show_texts);
}

#[test]
fn test_color_element_modal_and_application() {
    let (mut app, _) = CadeSimuApp::new();
    app.doc.items.clear();

    let it1 = SchematicItem::new_symbol(1, 9000, 30, 30, "-KM1");
    let it2 = SchematicItem::new_symbol(2, 9000, 50, 30, "-KM2");
    app.doc.items.push(it1);
    app.doc.items.push(it2);

    // 1. Abrir sem seleção avisa para selecionar
    let _ = app.update(Message::ColorElement);
    assert!(!app.show_color_modal);
    assert!(app.status_msg.contains("Selecione"));

    // 2. Selecionar it1 e abrir modal de cor
    app.select_single(Some(1));
    let _ = app.update(Message::ColorElement);
    assert!(app.show_color_modal);

    // 3. Escolher Vermelho (0x00E53935)
    let _ = app.update(Message::SelectColor(0x00E53935));
    assert_eq!(app.pending_color_int, 0x00E53935);
    assert_eq!(app.pending_color_hex, "#E53935");

    // 4. Confirmar aplicação
    let _ = app.update(Message::ConfirmColorModal);
    assert!(!app.show_color_modal);
    assert_eq!(app.doc.items[0].color, 0x00E53935);
    assert_eq!(app.doc.items[1].color, 0);

    // 5. Desfazer restaura a cor anterior (0)
    let _ = app.update(Message::Undo);
    assert_eq!(app.doc.items[0].color, 0);

    // 6. Refazer reaplica a cor
    let _ = app.update(Message::Redo);
    assert_eq!(app.doc.items[0].color, 0x00E53935);

    // 7. Selecionar múltiplos elementos e aplicar cor customizada via hex
    let _ = app.update(Message::SelectAll);
    let _ = app.update(Message::ColorElement);
    assert!(app.show_color_modal);
    let _ = app.update(Message::InputColorHex("#1E88E5".to_string()));
    assert_eq!(app.pending_color_int, 0x001E88E5);
    let _ = app.update(Message::ConfirmColorModal);
    assert_eq!(app.doc.items[0].color, 0x001E88E5);
    assert_eq!(app.doc.items[1].color, 0x001E88E5);

    // 8. Restaurar cor padrão (0)
    let _ = app.update(Message::ApplyColor(0));
    assert_eq!(app.doc.items[0].color, 0);
    assert_eq!(app.doc.items[1].color, 0);
}

#[test]
fn test_colored_element_export_to_svg() {
    let mut doc = SchematicDocument::default();
    doc.items.clear();

    let mut wire = SchematicItem::new_wire(1, 4000, 10, 10, 20, 10);
    wire.color = 0x00E53935; // Vermelho
    doc.items.push(wire);

    let mut sym = SchematicItem::new_symbol(2, 9000, 30, 30, "-KM1");
    sym.color = 0x001E88E5; // Azul
    doc.items.push(sym);

    let opts = ExportOptions {
        monochrome: false,
        show_terminals: false,
        include_cajetin: false,
    };
    let svg = export_to_svg(&doc, None, opts);
    assert!(svg.contains(r##"stroke="#e53935""##));
    assert!(svg.contains(r##"stroke="#1e88e5""##));

    // Modo monocromático deve forçar preto #000000
    let mono_opts = ExportOptions {
        monochrome: true,
        show_terminals: false,
        include_cajetin: false,
    };
    let mono_svg = export_to_svg(&doc, None, mono_opts);
    assert!(!mono_svg.contains(r##"stroke="#e53935""##));
    assert!(!mono_svg.contains(r##"stroke="#1e88e5""##));
}

#[test]
fn test_projects_tab_page_dropdown_navigation() {
    let (mut app, _) = CadeSimuApp::new();
    app.doc.title_block.title = "Projeto Principal".to_string();
    app.pages = vec![
        ProjectPage::new(
            1,
            "Diagrama de Potência".to_string(),
            SchematicDocument::default(),
        ),
        ProjectPage::new(2, "Comando 24V".to_string(), SchematicDocument::default()),
        ProjectPage::new(
            3,
            "Painel Frontal".to_string(),
            SchematicDocument::default(),
        ),
    ];

    let project_title = if !app.doc.title_block.title.trim().is_empty() {
        app.doc.title_block.title.trim().to_string()
    } else {
        "Projeto Principal".to_string()
    };
    let total_pages = app.pages.len().max(1);
    let page_options: Vec<ProjectPageOption> = app
        .pages
        .iter()
        .enumerate()
        .map(|(idx, p)| ProjectPageOption {
            index: idx,
            label: format!(
                "{} -> ({}/{}) {}",
                project_title,
                idx + 1,
                total_pages,
                p.name
            ),
        })
        .collect();

    assert_eq!(page_options.len(), 3);
    assert_eq!(
        page_options[0].label,
        "Projeto Principal -> (1/3) Diagrama de Potência"
    );
    assert_eq!(
        page_options[1].label,
        "Projeto Principal -> (2/3) Comando 24V"
    );
    assert_eq!(
        page_options[2].label,
        "Projeto Principal -> (3/3) Painel Frontal"
    );
    assert_eq!(
        format!("{}", page_options[0]),
        "Projeto Principal -> (1/3) Diagrama de Potência"
    );

    // Navegação via seleção na lista suspensa
    assert_eq!(app.active_page_index, 0);
    let _ = app.update(Message::SelectPage(1));
    assert_eq!(app.active_page_index, 1);
    assert!(app.status_msg.contains("Folha 2 ativa: Comando 24V"));

    let _ = app.update(Message::SelectPage(2));
    assert_eq!(app.active_page_index, 2);
    assert!(app.status_msg.contains("Folha 3 ativa: Painel Frontal"));
}

#[test]
fn test_recent_projects_selection_and_double_click_open() {
    let (mut app, _) = CadeSimuApp::new();
    app.recent_files = vec![
        "/tmp/projeto_teste1.cad".to_string(),
        "/tmp/projeto_teste2.qet".to_string(),
    ];

    // Clique único seleciona o projeto
    let _ = app.update(Message::SelectRecentProject(
        "/tmp/projeto_teste1.cad".to_string(),
    ));
    assert_eq!(
        app.selected_recent_project.as_deref(),
        Some("/tmp/projeto_teste1.cad")
    );
    assert!(app.last_recent_project_click.is_some());
    assert!(app.status_msg.contains("Projeto recente selecionado"));

    // Duplo clique (menos de 400ms) no mesmo projeto aciona abertura
    let _ = app.update(Message::SelectRecentProject(
        "/tmp/projeto_teste1.cad".to_string(),
    ));
    assert_eq!(
        app.selected_recent_project.as_deref(),
        Some("/tmp/projeto_teste1.cad")
    );

    // Botão explícito Abrir
    let _ = app.update(Message::RecentProjectSelected(
        "/tmp/projeto_teste2.qet".to_string(),
    ));
}

#[test]
fn test_recent_projects_delete_confirmation_modal_flow() {
    let (mut app, _) = CadeSimuApp::new();
    app.recent_files = vec!["/tmp/proj1.cad".to_string(), "/tmp/proj2.qet".to_string()];

    // 1. Seleciona proj1 e aciona exclusão via Delete key na aba Projetos (left_dock_tab = 1)
    app.left_dock_tab = 1;
    let _ = app.update(Message::SelectRecentProject("/tmp/proj1.cad".to_string()));
    assert_eq!(
        app.selected_recent_project.as_deref(),
        Some("/tmp/proj1.cad")
    );

    // Tecla Delete quando na aba Projetos deve abrir modal de confirmação
    let _ = app.update(Message::DeleteSelected);
    assert!(app.show_confirm_delete_modal);
    assert_eq!(app.project_to_delete.as_deref(), Some("/tmp/proj1.cad"));
    assert!(app.status_msg.contains("Confirmar remoção"));

    // 2. Cancelar modal via CancelDeleteRecentProject (botão Cancelar)
    let _ = app.update(Message::CancelDeleteRecentProject);
    assert!(!app.show_confirm_delete_modal);
    assert!(app.project_to_delete.is_none());
    assert_eq!(app.recent_files.len(), 2);
    assert!(app.recent_files.contains(&"/tmp/proj1.cad".to_string()));

    // 3. Requisitar exclusão via botão "Apagar" e cancelar via ESC (CancelAction)
    let _ = app.update(Message::RequestDeleteRecentProject(
        "/tmp/proj2.qet".to_string(),
    ));
    assert!(app.show_confirm_delete_modal);
    assert_eq!(app.project_to_delete.as_deref(), Some("/tmp/proj2.qet"));

    let _ = app.update(Message::CancelAction);
    assert!(!app.show_confirm_delete_modal);
    assert!(app.project_to_delete.is_none());
    assert_eq!(app.recent_files.len(), 2);

    // 4. Confirmar exclusão no modal
    let _ = app.update(Message::RequestDeleteRecentProject(
        "/tmp/proj1.cad".to_string(),
    ));
    assert!(app.show_confirm_delete_modal);

    let _ = app.update(Message::ConfirmDeleteRecentProject);
    assert!(!app.show_confirm_delete_modal);
    assert!(app.project_to_delete.is_none());
    assert_eq!(app.recent_files.len(), 1);
    assert!(!app.recent_files.contains(&"/tmp/proj1.cad".to_string()));
    assert_eq!(app.selected_recent_project, None);
    assert_eq!(app.status_msg, "Projeto removido da lista recente.");
}

#[test]
fn test_left_dock_elements_default_and_collapse_expand() {
    let (mut app, _) = CadeSimuApp::new();
    // 3.1: Aba Elementos deve vir selecionada por padrão
    assert_eq!(app.left_dock_tab, 0);
    assert!(!app.left_dock_collapsed);

    // 3.3: Recolhimento e restauração da barra com chevron
    let _ = app.update(Message::ToggleLeftDockCollapse);
    assert!(app.left_dock_collapsed);

    let _ = app.update(Message::ToggleLeftDockCollapse);
    assert!(!app.left_dock_collapsed);
}

#[test]
fn test_properties_element_name_and_function_editing() {
    let (mut app, _) = CadeSimuApp::new();
    let item = SchematicItem::new_symbol(101, 1000, 100, 100, "-M1");
    app.doc.items.push(item);
    app.selected_id = Some(101);

    // 1. Edição do Nome (Tag) e visibilidade no esquema
    let _ = app.update(Message::EditTag("-M_BOMBA".to_string()));
    assert_eq!(
        app.doc.items.iter().find(|i| i.id == 101).unwrap().tag,
        "-M_BOMBA"
    );

    let _ = app.update(Message::ToggleShowTag(false));
    assert_eq!(
        app.doc.items.iter().find(|i| i.id == 101).unwrap().show_tag,
        0
    );

    let _ = app.update(Message::ToggleShowTag(true));
    assert_eq!(
        app.doc.items.iter().find(|i| i.id == 101).unwrap().show_tag,
        1
    );

    // 2. Edição da Função e visibilidade no esquema
    let _ = app.update(Message::EditFunction("Bomba de Recalque 5CV".to_string()));
    let item_after = app.doc.items.iter().find(|i| i.id == 101).unwrap();
    assert_eq!(
        item_after.function.as_deref(),
        Some("Bomba de Recalque 5CV")
    );
    assert_eq!(item_after.function_or_value(), "Bomba de Recalque 5CV");

    let _ = app.update(Message::ToggleShowValue(true));
    assert_eq!(
        app.doc
            .items
            .iter()
            .find(|i| i.id == 101)
            .unwrap()
            .show_value,
        1
    );

    // Fallback para valor se função for esvaziada
    let _ = app.update(Message::EditValue("5kW".to_string()));
    let _ = app.update(Message::EditFunction("".to_string()));
    let item_fallback = app.doc.items.iter().find(|i| i.id == 101).unwrap();
    assert_eq!(item_fallback.function, None);
    assert_eq!(item_fallback.function_or_value(), "5kW");
}

#[test]
fn test_properties_special_coil_patterns_and_timers() {
    let (mut app, _) = CadeSimuApp::new();
    let mut item = SchematicItem::new_symbol(201, 9000, 200, 200, "-KM1");
    item.pins[0] = "A1".to_string();
    item.pins[1] = "A2".to_string();
    app.doc.items.push(item);
    app.selected_id = Some(201);

    let item_init = app.doc.items.iter().find(|i| i.id == 201).unwrap();
    assert!(is_coil_or_timer_apparatus(item_init));
    assert_eq!(CoilPattern::from_item(item_init), CoilPattern::Standard);

    // 1. Mudança para On-Delay (TON)
    let _ = app.update(Message::SetCoilPattern(CoilPattern::OnDelay));
    let item_ton = app.doc.items.iter().find(|i| i.id == 201).unwrap();
    assert_eq!(item_ton.tipo, 9004);
    assert_eq!(item_ton.subvariant, 0);
    assert_eq!(item_ton.value, "3");
    assert_eq!(CoilPattern::from_item(item_ton), CoilPattern::OnDelay);

    // Edição do tempo de temporização
    let _ = app.update(Message::EditValue("7.5".to_string()));
    assert_eq!(
        app.doc.items.iter().find(|i| i.id == 201).unwrap().value,
        "7.5"
    );

    // 2. Mudança para Off-Delay (TOF)
    let _ = app.update(Message::SetCoilPattern(CoilPattern::OffDelay));
    let item_tof = app.doc.items.iter().find(|i| i.id == 201).unwrap();
    assert_eq!(item_tof.tipo, 9005);
    assert_eq!(item_tof.subvariant, 0);
    assert_eq!(CoilPattern::from_item(item_tof), CoilPattern::OffDelay);

    // 3. Mudança para Cíclico / Intermitente
    let _ = app.update(Message::SetCoilPattern(CoilPattern::Cyclic));
    let item_cyc = app.doc.items.iter().find(|i| i.id == 201).unwrap();
    assert_eq!(item_cyc.subvariant, 2);
    assert_eq!(CoilPattern::from_item(item_cyc), CoilPattern::Cyclic);

    // 4. Mudança para Retenção / Biestável (Latching)
    let _ = app.update(Message::SetCoilPattern(CoilPattern::Latching));
    let item_latch = app.doc.items.iter().find(|i| i.id == 201).unwrap();
    assert_eq!(item_latch.tipo, 9000);
    assert_eq!(item_latch.subvariant, 1);
    assert_eq!(CoilPattern::from_item(item_latch), CoilPattern::Latching);

    // 5. Retorno para Padrão Instantânea e Preset de Bornes A1-A2
    let _ = app.update(Message::SetCoilPattern(CoilPattern::Standard));
    let _ = app.update(Message::ApplyIecNumberingPreset("coil"));
    let item_std = app.doc.items.iter().find(|i| i.id == 201).unwrap();
    assert_eq!(item_std.tipo, 9000);
    assert_eq!(item_std.subvariant, 0);
    assert_eq!(item_std.pins[0], "A1");
    assert_eq!(item_std.pins[1], "A2");
}

#[test]
fn test_properties_signaling_colors_and_pulsing_duration() {
    let (mut app, _) = CadeSimuApp::new();
    let item = SchematicItem::new_symbol(301, 9008, 300, 300, "-H1");
    app.doc.items.push(item);
    app.selected_id = Some(301);

    let item_init = app.doc.items.iter().find(|i| i.id == 301).unwrap();
    assert!(item_init.is_lamp_or_signaling());

    // 1. Cores industriais: Verde(1), Vermelho(2), Amarelo(3), Azul(4), Branco(0), Laranja(5)
    let colors = [
        (1, "Verde"),
        (2, "Vermelho"),
        (3, "Amarelo"),
        (4, "Azul"),
        (0, "Branco"),
        (5, "Laranja"),
    ];

    for (c_code, _) in colors {
        let _ = app.update(Message::SetLampColor(c_code));
        assert_eq!(
            app.doc.items.iter().find(|i| i.id == 301).unwrap().color,
            c_code
        );
    }

    // Verifica que a cor laranja (5) é suportada pelos tokens
    let tok = theme::tokens(&app.theme());
    assert_eq!(tok.lamp_color(5), tok.lamp_orange);

    // 2. Modo Pulsante e tempo de pulso
    let _ = app.update(Message::TogglePulsingSignaling(true));
    let item_pulsing = app.doc.items.iter().find(|i| i.id == 301).unwrap();
    assert_eq!(item_pulsing.extra_int1, 1);
    assert_eq!(item_pulsing.value, "0.5");

    let _ = app.update(Message::SetPulseDuration("1.25".to_string()));
    assert_eq!(
        app.doc.items.iter().find(|i| i.id == 301).unwrap().value,
        "1.25"
    );

    // 3. Preset de Bornes X1-X2
    let _ = app.update(Message::ApplyIecNumberingPreset("lamp"));
    let item_lamp_pins = app.doc.items.iter().find(|i| i.id == 301).unwrap();
    assert_eq!(item_lamp_pins.pins[0], "X1");
    assert_eq!(item_lamp_pins.pins[1], "X2");
}

#[test]
fn test_properties_iec60947_power_and_command_terminal_editing() {
    let (mut app, _) = CadeSimuApp::new();

    // 1. Circuito de Potência (Força - Contator 3P)
    let p_item = SchematicItem::new_symbol(401, 2001, 150, 150, "-KM1");
    app.doc.items.push(p_item);
    app.selected_id = Some(401);

    assert!(is_power_apparatus(
        app.doc.items.iter().find(|i| i.id == 401).unwrap()
    ));

    // Presets de Potência IEC 60947-1: 1-2, 3-4, 5-6 e L1-T1, L2-T2, L3-T3
    let _ = app.update(Message::ApplyIecNumberingPreset("power_3p"));
    let p_num = app.doc.items.iter().find(|i| i.id == 401).unwrap();
    assert_eq!(p_num.pins[0..6], ["1", "3", "5", "2", "4", "6"]);

    let _ = app.update(Message::ApplyIecNumberingPreset("power_3p_phases"));
    let p_pha = app.doc.items.iter().find(|i| i.id == 401).unwrap();
    assert_eq!(p_pha.pins[0..6], ["L1", "L2", "L3", "T1", "T2", "T3"]);

    // 2. Circuito de Comando (Contatos Auxiliares 2 dígitos - NA 7000)
    let c_item = SchematicItem::new_symbol(402, 7000, 250, 150, "-KM1");
    app.doc.items.push(c_item);
    app.selected_id = Some(402);

    // Dropdown sequencial IEC 60947: Sequência 4 -> 43-44
    let _ = app.update(Message::SelectPinSequence(4));
    let c_seq4 = app.doc.items.iter().find(|i| i.id == 402).unwrap();
    assert_eq!(c_seq4.pins[0..2], ["43", "44"]);

    // Presets rápidos: aux_no (13-14), aux_nc (21-22), thermal (95-98)
    let _ = app.update(Message::ApplyIecNumberingPreset("aux_no"));
    assert_eq!(
        app.doc.items.iter().find(|i| i.id == 402).unwrap().pins[0..2],
        ["13", "14"]
    );

    let _ = app.update(Message::ApplyIecNumberingPreset("aux_nc"));
    assert_eq!(
        app.doc.items.iter().find(|i| i.id == 402).unwrap().pins[0..2],
        ["21", "22"]
    );

    let _ = app.update(Message::ApplyIecNumberingPreset("thermal"));
    assert_eq!(
        app.doc.items.iter().find(|i| i.id == 402).unwrap().pins[0..4],
        ["95", "96", "97", "98"]
    );
}

#[test]
fn test_properties_terminal_strips_iec81346_classification() {
    let (mut app, _) = CadeSimuApp::new();
    let term = SchematicItem::new_symbol(501, 7001, 100, 200, "-X1");
    app.doc.items.push(term);
    app.selected_id = Some(501);

    assert!(is_terminal_item(
        app.doc.items.iter().find(|i| i.id == 501).unwrap()
    ));

    // XP: Potência / Força
    let _ = app.update(Message::ApplyTerminalStripTagPreset("-XP"));
    let term_xp = app.doc.items.iter().find(|i| i.id == 501).unwrap();
    assert_eq!(term_xp.tag, "-XP");
    assert_eq!(
        TerminalStripKind::from_tag(&term_xp.tag),
        TerminalStripKind::Power
    );

    // XC: Comando / Auxiliar
    let _ = app.update(Message::ApplyTerminalStripTagPreset("-XC"));
    let term_xc = app.doc.items.iter().find(|i| i.id == 501).unwrap();
    assert_eq!(term_xc.tag, "-XC");
    assert_eq!(
        TerminalStripKind::from_tag(&term_xc.tag),
        TerminalStripKind::Control
    );

    // XT: Medição / Transformadores
    let _ = app.update(Message::ApplyTerminalStripTagPreset("-XT"));
    let term_xt = app.doc.items.iter().find(|i| i.id == 501).unwrap();
    assert_eq!(term_xt.tag, "-XT");
    assert_eq!(
        TerminalStripKind::from_tag(&term_xt.tag),
        TerminalStripKind::Measurement
    );

    // X1: Geral
    let _ = app.update(Message::ApplyTerminalStripTagPreset("-X1"));
    let term_x1 = app.doc.items.iter().find(|i| i.id == 501).unwrap();
    assert_eq!(term_x1.tag, "-X1");
    assert_eq!(
        TerminalStripKind::from_tag(&term_x1.tag),
        TerminalStripKind::General
    );
}

#[test]
fn test_properties_cross_reference_selected_vs_global_modes() {
    let (mut app, _) = CadeSimuApp::new();

    // 1. Sem elemento selecionado: Aba 0 mostra Cajetín, Aba 1 mostra Diretório Global
    app.selected_id = None;
    let _ = app.update(Message::SelectPropTab(0));
    assert_eq!(app.prop_tab, 0);
    let _ = app.view();

    let _ = app.update(Message::SelectPropTab(1));
    assert_eq!(app.prop_tab, 1);
    let _ = app.view();

    // 2. Com elemento selecionado: Aba 0 mostra Propriedades do elemento, Aba 1 mostra Referência Cruzada específica
    let coil = SchematicItem::new_symbol(601, 9000, 100, 100, "-KM1");
    let contact = SchematicItem::new_symbol(602, 7000, 200, 100, "-KM1");
    app.doc.items.push(coil);
    app.doc.items.push(contact);

    app.selected_id = Some(601);
    let _ = app.update(Message::SelectPropTab(0));
    let _ = app.view();

    let _ = app.update(Message::SelectPropTab(1));
    let _ = app.view();

    // 3. Navegação para outro elemento via XRef
    let _ = app.update(Message::NavigateToElement {
        page_index: 0,
        item_id: 602,
    });
    assert_eq!(app.selected_id, Some(602));
    assert_eq!(app.active_page_index, 0);
    let _ = app.view();
}

#[test]
fn test_properties_timer_time_control_and_cyclic_parameters() {
    let (mut app, _) = CadeSimuApp::new();
    let mut item = SchematicItem::new_symbol(701, 9000, 150, 150, "-KT1");
    item.pins[0] = "A1".to_string();
    item.pins[1] = "A2".to_string();
    app.doc.items.push(item);
    app.selected_id = Some(701);

    // 1. Configura como On-Delay
    let _ = app.update(Message::SetCoilPattern(CoilPattern::OnDelay));
    let it = app.doc.items.iter().find(|i| i.id == 701).unwrap();
    assert_eq!(it.tipo, 9004);
    assert_eq!(it.timer_scale_multiplier(), TimerScaleMultiplier::X1);
    assert_eq!(it.timer_base_value(), 3.0);
    assert_eq!(it.timer_effective_seconds(), 3.0);

    // 2. Ajuste via Stepper buttons (+1s, +5s, -0.5s)
    let _ = app.update(Message::AdjustTimerPreset(1.0));
    assert_eq!(
        app.doc
            .items
            .iter()
            .find(|i| i.id == 701)
            .unwrap()
            .timer_effective_seconds(),
        4.0
    );

    let _ = app.update(Message::AdjustTimerPreset(5.0));
    assert_eq!(
        app.doc
            .items
            .iter()
            .find(|i| i.id == 701)
            .unwrap()
            .timer_effective_seconds(),
        9.0
    );

    let _ = app.update(Message::AdjustTimerPreset(-0.5));
    assert_eq!(
        app.doc
            .items
            .iter()
            .find(|i| i.id == 701)
            .unwrap()
            .timer_effective_seconds(),
        8.5
    );

    // 3. Multiplicadores de escala (x10s, x60s, x0.1s)
    let _ = app.update(Message::SetTimerScaleMultiplier(TimerScaleMultiplier::X10));
    let it_x10 = app.doc.items.iter().find(|i| i.id == 701).unwrap();
    assert_eq!(it_x10.timer_scale_multiplier(), TimerScaleMultiplier::X10);
    assert_eq!(it_x10.timer_effective_seconds(), 85.0);

    let _ = app.update(Message::SetTimerScaleMultiplier(TimerScaleMultiplier::X0_1));
    let it_x01 = app.doc.items.iter().find(|i| i.id == 701).unwrap();
    assert_eq!(it_x01.timer_scale_multiplier(), TimerScaleMultiplier::X0_1);
    assert!((it_x01.timer_effective_seconds() - 0.85).abs() < 1e-4);

    // 4. Atalhos rápidos (Quick Presets)
    let _ = app.update(Message::ApplyTimerQuickPreset(5.0));
    let it_p5 = app.doc.items.iter().find(|i| i.id == 701).unwrap();
    assert_eq!(it_p5.timer_scale_multiplier(), TimerScaleMultiplier::X1);
    assert_eq!(it_p5.timer_effective_seconds(), 5.0);

    let _ = app.update(Message::ApplyTimerQuickPreset(0.5));
    let it_p05 = app.doc.items.iter().find(|i| i.id == 701).unwrap();
    assert_eq!(it_p05.timer_scale_multiplier(), TimerScaleMultiplier::X0_1);
    assert_eq!(it_p05.timer_effective_seconds(), 0.5);

    let _ = app.update(Message::ApplyTimerQuickPreset(60.0));
    let it_p60 = app.doc.items.iter().find(|i| i.id == 701).unwrap();
    assert_eq!(it_p60.timer_scale_multiplier(), TimerScaleMultiplier::X60);
    assert_eq!(it_p60.timer_effective_seconds(), 60.0);

    // 5. Configuração Cíclica (T_ON e T_OFF independentes)
    let _ = app.update(Message::SetCoilPattern(CoilPattern::Cyclic));
    let _ = app.update(Message::SetTimerBaseValue("2".to_string()));
    let _ = app.update(Message::SetTimerScaleMultiplier(TimerScaleMultiplier::X1));

    let _ = app.update(Message::SetTimerCyclicOffBaseValue("1".to_string()));
    let _ = app.update(Message::SetTimerCyclicOffMultiplier(
        TimerScaleMultiplier::X1,
    ));
    let _ = app.update(Message::AdjustTimerCyclicOff(1.0)); // 1s + 1s = 2s

    let it_cyc = app.doc.items.iter().find(|i| i.id == 701).unwrap();
    assert_eq!(it_cyc.timer_effective_seconds(), 2.0);
    assert_eq!(it_cyc.timer_off_effective_seconds(), 2.0);

    let _ = app.view();
}

#[test]
fn test_properties_timer_live_simulation_monitoring_and_reset() {
    let (mut app, _) = CadeSimuApp::new();
    let mut item = SchematicItem::new_symbol(801, 9004, 120, 120, "-KT1");
    item.pins[0] = "A1".to_string();
    item.pins[1] = "A2".to_string();
    item.pins[2] = "0".to_string();
    item.pins[3] = "5".to_string(); // 5 segundos
    app.doc.items.push(item);
    app.selected_id = Some(801);

    // Inicia simulação e energiza bobina
    app.sim_running = true;
    app.doc.sim.get_mut(801).active = true;
    app.doc.sim.get_mut(801).timer_elapsed_ms = 2500; // 50% concluído

    // Renderiza interface com a barra de progresso em tempo real
    let _ = app.view();

    // Dispara reset manual durante a simulação
    let _ = app.update(Message::ResetTimerSimulation(801));
    assert_eq!(app.doc.sim.get(801).timer_elapsed_ms, 0);
    assert_eq!(app.doc.sim.get(801).timer_phase, 0);

    let _ = app.view();
}

#[test]
fn test_section1_arquivo_save_copy_recent_print_and_all_7_export_formats() {
    let (mut app, _) = CadeSimuApp::new();

    let temp_dir = std::env::temp_dir();
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let orig_path = temp_dir.join(format!(
        "rustsimu_orig_{}_{}.json",
        std::process::id(),
        unique
    ));
    let copy_path = temp_dir.join(format!(
        "rustsimu_copy_{}_{}.json",
        std::process::id(),
        unique
    ));

    // 1. Salvamento original define current_file
    app.doc.title_block.title = "Projeto Original".to_string();
    let _ = app.update(Message::FileSaved(Some(orig_path.clone())));
    assert_eq!(app.current_file.as_ref(), Some(&orig_path));
    assert!(orig_path.exists());

    // 2. Salvar Cópia escreve no destino sem sobrescrever current_file ou file_path
    app.doc.title_block.title = "Projeto Copiado".to_string();
    let _ = app.update(Message::FileCopySaved(Some(copy_path.clone())));
    assert!(copy_path.exists());
    assert_eq!(
        app.current_file.as_ref(),
        Some(&orig_path),
        "Salvar Cópia não pode alterar current_file ativo"
    );

    // 3. Lista dinâmica de Últimos Arquivos + Limpar
    app.push_recent_file(&copy_path);
    assert_eq!(
        app.recent_files.first().map(String::as_str),
        Some(copy_path.to_string_lossy().as_ref())
    );
    let _ = app.update(Message::ClearRecentFiles);
    assert!(app.recent_files.is_empty());

    // 4. Impressão vetorial PDF
    let _ = app.update(Message::OpenPrintModal);
    assert!(app.show_print_modal);
    let print_pdf = app
        .build_print_pdf_bytes()
        .expect("Impressão deve gerar PDF vetorial válido");
    assert!(print_pdf.starts_with(b"%PDF-"));
    let _ = app.update(Message::ClosePrintModal);
    assert!(!app.show_print_modal);

    // 5. Todos os 8 formatos de exportação (§1.1: PNG, JPG, BMP, SVG, DXF, CAD, CT, PDF)
    assert_eq!(ExportFormat::ALL.len(), 8);
    let mut doc = build_dol_starter_preset();
    let mut txt = SchematicItem::new_graphic(901, 8, 24, 24, 60, 32);
    txt.subvariant = 0;
    txt.trailing_str = "Motor 3F".to_string();
    doc.items.push(txt);
    doc.items
        .push(SchematicItem::new_graphic(902, 1, 20, 40, 80, 40));
    doc.items
        .push(SchematicItem::new_graphic(903, 2, 20, 50, 70, 70));
    doc.items
        .push(SchematicItem::new_graphic(904, 3, 90, 50, 120, 80));
    let mut arc = SchematicItem::new_graphic(905, 4, 130, 50, 160, 80);
    arc.aux_coords = [0, 180, 0, 0];
    doc.items.push(arc);

    let opts = ExportOptions::default();
    let png = export_to_png(&doc, None, opts, None).expect("Export PNG");
    assert!(png.starts_with(b"\x89PNG"));

    let jpg = export_to_jpg(&doc, None, opts, None).expect("Export JPG");
    assert!(jpg.starts_with(&[0xFF, 0xD8, 0xFF]));

    let bmp = export_to_bmp(&doc, None, opts, None).expect("Export BMP");
    assert!(bmp.starts_with(b"BM"));

    let svg = export_to_svg(&doc, None, opts);
    assert!(svg.contains("<svg") && svg.contains("Motor 3F"));

    let dxf = String::from_utf8(export_to_dxf(&doc, None, opts)).expect("DXF ASCII");
    assert!(dxf.contains("ENTITIES"));
    assert!(dxf.contains("LINE"));
    assert!(dxf.contains("CIRCLE"));
    assert!(dxf.contains("ARC"));

    let cad = export_to_cad(&doc, true);
    let parsed_cad = parse_cad_bytes(&cad).expect("CAD roundtrip");
    assert!(!parsed_cad.items.is_empty());

    let pdf = export_project_to_pdf(&[ProjectPage::new(1, "F1", doc)], opts).expect("Export PDF");
    assert!(pdf.starts_with(b"%PDF-"));

    let _ = std::fs::remove_file(orig_path);
    let _ = std::fs::remove_file(copy_path);
}

#[test]
fn test_section1_editar_graphic_primitives_grid_rows_cols_and_page_properties() {
    let (mut app, _) = CadeSimuApp::new();
    app.doc.items.clear();

    // 1. Inserção de primitivas (§1.2 Adicionar)
    let _ = app.update(Message::Canvas(CanvasMessage::AddGraphicItem {
        tipo: 8,
        subvariant: 0,
        x1: 30,
        y1: 30,
        x2: 60,
        y2: 38,
    }));
    let _ = app.update(Message::Canvas(CanvasMessage::AddGraphicItem {
        tipo: 8,
        subvariant: 1,
        x1: 30,
        y1: 48,
        x2: 80,
        y2: 62,
    }));
    let _ = app.update(Message::Canvas(CanvasMessage::AddGraphicItem {
        tipo: 1,
        subvariant: 0,
        x1: 20,
        y1: 80,
        x2: 90,
        y2: 80,
    }));
    let _ = app.update(Message::Canvas(CanvasMessage::AddGraphicItem {
        tipo: 2,
        subvariant: 0,
        x1: 100,
        y1: 30,
        x2: 140,
        y2: 60,
    }));
    let _ = app.update(Message::Canvas(CanvasMessage::AddGraphicItem {
        tipo: 3,
        subvariant: 0,
        x1: 150,
        y1: 30,
        x2: 180,
        y2: 60,
    }));
    let _ = app.update(Message::Canvas(CanvasMessage::AddGraphicItem {
        tipo: 4,
        subvariant: 0,
        x1: 190,
        y1: 30,
        x2: 220,
        y2: 60,
    }));
    assert_eq!(app.doc.items.len(), 6);

    // Edição de texto e ângulos de arco via painel de propriedades
    let txt_id = app.doc.items[0].id;
    app.selected_id = Some(txt_id);
    let _ = app.update(Message::EditItemTrailingText("Nota Técnica".to_string()));
    assert_eq!(app.doc.items[0].trailing_str, "Nota Técnica");

    let arc_id = app.doc.items[5].id;
    app.selected_id = Some(arc_id);
    let _ = app.update(Message::EditItemArcStart("45".to_string()));
    let _ = app.update(Message::EditItemArcSweep("135".to_string()));
    assert_eq!(app.doc.items[5].aux_coords[0], 45);
    assert_eq!(app.doc.items[5].aux_coords[1], 135);

    // Mover item gráfico único preserva geometria (dx, dy) sem colapsar x2/y2
    let rect_id = app.doc.items[3].id;
    let _ = app.update(Message::Canvas(CanvasMessage::MoveItem {
        id: rect_id,
        x1: 112,
        y1: 42,
    }));
    let moved_rect = app.doc.items.iter().find(|i| i.id == rect_id).unwrap();
    assert_eq!(
        (moved_rect.x1, moved_rect.y1, moved_rect.x2, moved_rect.y2),
        (112, 42, 152, 72)
    );

    // Hit-test no canvas acerta primitivas gráficas
    let dummy_sel = std::collections::HashSet::new();
    let canvas = SchematicCanvas {
        doc: &app.doc,
        tool: EditorTool::Select,
        selected_id: None,
        selected_ids: &dummy_sel,
        is_shift_pressed: false,
        is_ctrl_pressed: false,
        is_alt_pressed: false,
        sim_running: false,
        show_grid: true,
        snap_to_grid: true,
        grid_density: app.grid_density,
        show_texts: true,
        show_terminals: true,
        show_cajetin: true,
        show_full_crosshair: false,
        show_text_grid: false,
        show_alignment_guides: false,
        keep_headers_visible: false,
        show_cell_bounds: false,
        is_dark: false,
        auto_connect: true,
        auto_disconnect: false,
        zoom: 1.0,
        pan: iced::Vector::ZERO,
        placement_orientation: 0,
        dragging_symbol: None,
        dragging_canonical_symbol: None,
        is_blocked: false,
    };
    assert_eq!(canvas.hit_test(112, 55), Some(rect_id));
    assert_eq!(canvas.hit_test(165, 45), Some(app.doc.items[4].id));

    // 2. Linhas e Colunas dinâmicas da grade + sincronia com Xref e SVG
    let base_cols = app.doc.settings.effective_grid_cols();
    let base_rows = app.doc.settings.effective_grid_rows();
    let _ = app.update(Message::AddGridCol);
    let _ = app.update(Message::AddGridRow);
    assert_eq!(app.doc.settings.effective_grid_cols(), base_cols + 1);
    assert_eq!(app.doc.settings.effective_grid_rows(), base_rows + 1);
    let xref_ref = cade_simu_rs::xref::format_grid_ref_doc(0, 310.0, 220.0, &app.doc);
    assert!(xref_ref.starts_with("1."));
    let _ = app.update(Message::RemoveGridCol);
    let _ = app.update(Message::RemoveGridRow);
    assert_eq!(app.doc.settings.effective_grid_cols(), base_cols);
    assert_eq!(app.doc.settings.effective_grid_rows(), base_rows);

    // 3. Modal Propriedades da Página (Ctrl+Shift+Enter)
    let _ = app.update(Message::OpenPagePropertiesModal);
    assert!(app.show_page_properties_modal);
    let _ = app.update(Message::EditPagePropName("Painel Principal".to_string()));
    let _ = app.update(Message::EditPagePropTitle("Bomba Centrífuga".to_string()));
    let _ = app.update(Message::EditPagePropSubtitle(
        "Partida Estrela-Triângulo".to_string(),
    ));
    let _ = app.update(Message::EditPagePropEntity(
        "Engenharia Industrial".to_string(),
    ));
    let _ = app.update(Message::EditPagePropAuthor("Eng. Carlos".to_string()));
    let _ = app.update(Message::EditPagePropWidthMm("420".to_string()));
    let _ = app.update(Message::EditPagePropHeightMm("297".to_string()));
    let _ = app.view();
    let _ = app.update(Message::ApplyPageProperties);
    assert!(!app.show_page_properties_modal);
    assert_eq!(app.pages[0].name, "Painel Principal");
    assert_eq!(app.doc.title_block.title, "Bomba Centrífuga");
    assert_eq!(app.doc.settings.custom_width_mm, 420);
}

#[test]
fn test_section1_projeto_exibicao_configuracoes_janela_ajuda_and_submenus() {
    let (mut app, _) = CadeSimuApp::new();

    // 1. Propriedades do Projeto em múltiplas folhas (§1.3)
    let _ = app.update(Message::AddPageAfter(0));
    assert_eq!(app.pages.len(), 2);
    let _ = app.update(Message::OpenProjectPropertiesModal);
    assert!(app.show_project_properties_modal);
    let _ = app.update(Message::EditProjectPropTitle(
        "Subestação Norte".to_string(),
    ));
    let _ = app.update(Message::EditProjectPropEntity("Cia Elétrica".to_string()));
    let _ = app.update(Message::EditProjectPropAuthor("Ana Souza".to_string()));
    let _ = app.update(Message::EditProjectPropVersion("Rev 2.4".to_string()));
    let _ = app.update(Message::ToggleProjectAutoSave(true));
    let _ = app.view();
    let _ = app.update(Message::ApplyProjectProperties);
    assert!(!app.show_project_properties_modal);
    assert!(app.proj_prop_auto_save);
    for page in &app.pages {
        assert_eq!(page.doc.title_block.title, "Subestação Norte");
        assert_eq!(page.doc.title_block.entity, "Cia Elétrica");
        assert_eq!(page.doc.title_block.drawn_by, "Ana Souza");
        assert_eq!(page.doc.title_block.signature, "Rev 2.4");
    }

    // 2. Display / Exibição (§1.4)
    let _ = app.update(Message::ToggleLeftDockCollapse);
    assert!(app.left_dock_collapsed);
    let _ = app.update(Message::ShowProjectsDock);
    assert!(!app.left_dock_collapsed);
    assert_eq!(app.left_dock_tab, 1);

    let _ = app.update(Message::ToggleFullCrosshair);
    let _ = app.update(Message::ToggleTextGrid);
    let _ = app.update(Message::ToggleAlignmentGuides);
    let _ = app.update(Message::ToggleKeepHeadersVisible);
    let _ = app.update(Message::ToggleCellBounds);
    let _ = app.update(Message::ToggleShowTerminals);
    let _ = app.update(Message::ToggleShowCajetin);
    assert!(app.show_full_crosshair);
    assert!(app.show_text_grid);
    assert!(app.show_alignment_guides);
    assert!(app.keep_headers_visible);
    assert!(app.show_cell_bounds);
    assert!(!app.show_terminals);
    assert!(!app.show_cajetin);

    let _ = app.update(Message::SetSheetBgPreset(Some(0xFAF5E4)));
    assert_eq!(app.doc.settings.sheet_bg_color, Some(0xFAF5E4));
    let _ = app.update(Message::ZoomFit);
    let _ = app.update(Message::CenterOnCursor);

    // 3. Menus, Submenus inline, Configurações, Janela e Ajuda (§1.5)
    for menu in [
        ActiveMenu::Arquivo,
        ActiveMenu::Editar,
        ActiveMenu::Projeto,
        ActiveMenu::Exibicao,
        ActiveMenu::Configuracoes,
        ActiveMenu::Janela,
        ActiveMenu::Ajuda,
    ] {
        let _ = app.update(Message::ToggleMenu(Some(menu)));
        assert_eq!(app.open_menu, Some(menu));
        if menu == ActiveMenu::Editar {
            let _ = app.update(Message::ToggleSubmenu("editar_adicionar"));
            assert_eq!(app.open_submenu, Some("editar_adicionar"));
        } else if menu == ActiveMenu::Exibicao {
            let _ = app.update(Message::ToggleSubmenu("exibicao_bg"));
            assert_eq!(app.open_submenu, Some("exibicao_bg"));
        }
        let _ = app.view();
    }

    let _ = app.update(Message::OpenSettingsModal);
    assert!(app.show_settings_modal);
    let _ = app.update(Message::SetThemeMode(ThemeMode::Light));
    assert_eq!(app.theme_mode, ThemeMode::Light);
    let _ = app.view();
    let _ = app.update(Message::CancelAction);
    assert!(!app.show_settings_modal);

    let _ = app.update(Message::OpenShortcutsModal);
    assert!(app.show_shortcuts_modal);
    let _ = app.view();
    let _ = app.update(Message::CloseShortcutsModal);
    assert!(!app.show_shortcuts_modal);

    let _ = app.update(Message::ToggleFocusCanvas);
    assert!(app.focus_canvas);
    let _ = app.view();
    let _ = app.update(Message::CancelAction);
    assert!(!app.focus_canvas);
}

#[test]
fn test_section5_title_bar_modified_state_and_document_lifecycle() {
    use cade_simu_rs::app::{CadeSimuApp, Message};

    let (mut app, _) = CadeSimuApp::new();
    assert!(!app.is_modified);
    {
        let _ = app.view();
    }

    // Adiciona item para marcar projeto como modificado e renderiza Barra de Título com [Modificado]
    let _ = app.update(Message::Canvas(
        cade_simu_rs::canvas_view::CanvasMessage::AddGraphicItem {
            tipo: 2,
            subvariant: 0,
            x1: 30,
            y1: 30,
            x2: 60,
            y2: 50,
        },
    ));
    assert!(app.is_modified);
    {
        let _ = app.view();
    }

    // Fechar ou Novo limpa `file_path`, `current_file` e `is_modified`
    let _ = app.update(Message::CloseDocument);
    assert!(!app.is_modified);
    assert!(app.current_file.is_none());
    assert!(app.file_path.is_empty());
    assert_eq!(app.pages.len(), 1);
    {
        let _ = app.view();
    }

    let _ = app.update(Message::LoadPresetDol);
    assert!(!app.is_modified);
    assert_eq!(app.file_path, "Partida Direta Trifásica");

    let _ = app.update(Message::NewDocument);
    assert!(!app.is_modified);
    assert!(app.file_path.is_empty());
    assert!(app.current_file.is_none());
}

#[test]
fn test_section5_pages_bar_double_click_context_edit_number_and_last_page_modal() {
    use cade_simu_rs::app::{CadeSimuApp, Message};

    let (mut app, _) = CadeSimuApp::new();

    // Com apenas 1 página, StartEditingPageNumber não deve ativar edição numérica
    let _ = app.update(Message::StartEditingPageNumber(0));
    assert!(app.editing_page_number.is_none());

    // Cria páginas antes e depois (atalhos das extremidades / menu de contexto)
    let _ = app.update(Message::AddPageAfter(0));
    let _ = app.update(Message::AddPageBefore(1));
    assert_eq!(app.pages.len(), 3);
    assert_eq!(
        app.pages.iter().map(|p| p.id).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );

    // Menu de contexto em aba inativa (botão direito) + renderização com "Editar número"
    let _ = app.update(Message::TogglePageContextMenu(2));
    assert_eq!(app.page_context_menu, Some(2));
    {
        let _ = app.view();
    }

    // Clique duplo em uma aba seleciona a página e inicia edição direta do nome
    let _ = app.update(Message::SelectPage(2));
    let _ = app.update(Message::SelectPage(2));
    assert_eq!(app.active_page_index, 2);
    assert_eq!(app.editing_page_target, Some(2));
    assert!(app.editing_page_name.is_some());
    {
        let _ = app.view();
    }
    let _ = app.update(Message::UpdateEditingPageName("Comando Motor".to_string()));
    let _ = app.update(Message::CommitEditingPageName);
    assert_eq!(app.pages[2].name, "Comando Motor");
    assert!(app.editing_page_name.is_none());

    // Editar número da página (3 -> 1) reordena a página e atualiza índice ativo e IDs
    let _ = app.update(Message::StartEditingPageNumber(2));
    assert_eq!(app.editing_page_number, Some((2, "3".to_string())));
    {
        let _ = app.view();
    }
    let _ = app.update(Message::UpdateEditingPageNumber("1".to_string()));
    let _ = app.update(Message::CommitEditingPageNumber);
    assert_eq!(app.pages[0].name, "Comando Motor");
    assert_eq!(app.active_page_index, 0);
    assert_eq!(
        app.pages.iter().map(|p| p.id).collect::<Vec<_>>(),
        vec![1, 2, 3]
    );

    // Exclui até restar apenas 1 página
    let _ = app.update(Message::DeletePage(2));
    let _ = app.update(Message::DeletePage(1));
    assert_eq!(app.pages.len(), 1);
    assert!(!app.show_close_last_page_modal);

    // Tentar deletar a última página exibe modal de aviso e opção de salvamento
    let _ = app.update(Message::DeletePage(0));
    assert!(app.show_close_last_page_modal);
    assert_eq!(app.pages.len(), 1);
    {
        let _ = app.view();
    }

    // Cancelar mantém o projeto intacto
    let _ = app.update(Message::CancelCloseLastPage);
    assert!(!app.show_close_last_page_modal);
    assert_eq!(app.pages[0].name, "Comando Motor");

    // Salvar e Fechar grava em arquivo associado e fecha o projeto para folha em branco
    let tmp_path = std::env::temp_dir().join("rustsimu_section5_last_page_save.json");
    let _ = std::fs::remove_file(&tmp_path);
    app.current_file = Some(tmp_path.clone());
    app.file_path = tmp_path.to_string_lossy().to_string();

    let _ = app.update(Message::DeletePage(0));
    assert!(app.show_close_last_page_modal);
    let _ = app.update(Message::SaveAndCloseLastPage);
    assert!(tmp_path.exists());
    assert!(!app.show_close_last_page_modal);
    assert!(!app.is_modified);
    assert!(app.current_file.is_none());
    assert_eq!(app.pages[0].name, "Folha 1");
    let _ = std::fs::remove_file(&tmp_path);

    // Confirmar Fechar sem Salvar também encerra o projeto sem erro
    let _ = app.update(Message::DeletePage(0));
    assert!(app.show_close_last_page_modal);
    let _ = app.update(Message::ConfirmCloseLastPage);
    assert!(!app.show_close_last_page_modal);
    assert!(!app.is_modified);
}

#[test]
fn test_section5_canvas_themes_light_dark_system_and_connector_colors() {
    use cade_simu_rs::app::{CadeSimuApp, Message};
    use cade_simu_rs::canvas_view::{EditorTool, SchematicCanvas};
    use cade_simu_rs::model::{GridDensity, SchematicDocument, SchematicItem};
    use cade_simu_rs::theme::ThemeMode;
    use std::collections::HashSet;

    fn luminance(c: iced::Color) -> f32 {
        0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b
    }

    let (mut app, _) = CadeSimuApp::new();
    let doc = SchematicDocument::default();
    let empty_sel = HashSet::new();

    let mk_canvas = |is_dark: bool| SchematicCanvas {
        doc: &doc,
        tool: EditorTool::Select,
        selected_id: None,
        selected_ids: &empty_sel,
        is_shift_pressed: false,
        is_ctrl_pressed: false,
        is_alt_pressed: false,
        sim_running: false,
        show_grid: true,
        snap_to_grid: true,
        grid_density: GridDensity::Medium,
        show_texts: true,
        show_terminals: true,
        show_cajetin: true,
        show_full_crosshair: false,
        show_text_grid: false,
        show_alignment_guides: false,
        keep_headers_visible: false,
        show_cell_bounds: false,
        is_dark,
        auto_connect: true,
        auto_disconnect: false,
        zoom: 1.0,
        pan: iced::Vector::ZERO,
        placement_orientation: 0,
        dragging_symbol: None,
        dragging_canonical_symbol: None,
        is_blocked: false,
    };

    // Tema LIGHT: folha clara, traços/elementos escuros e conectores padrão escuros
    let _ = app.update(Message::SetThemeMode(ThemeMode::Light));
    assert!(!app.is_dark());
    let light_canvas = mk_canvas(app.is_dark());
    let light_tok = light_canvas.tokens();
    assert!(luminance(light_tok.sheet) > 0.85);
    assert!(luminance(light_tok.ink) < 0.30);
    assert!(luminance(light_tok.sheet_line) < 0.30);

    let default_wire = SchematicItem::new_wire(1, 4001, 10, 10, 40, 10);
    let dc_neg_wire = SchematicItem::new_wire(2, 4004, 10, 20, 40, 20);
    let mut custom_wire = SchematicItem::new_wire(3, 4000, 10, 30, 40, 30);
    custom_wire.color = 0x12AB34;

    let (light_def_col, _) = light_canvas.wire_color(&default_wire);
    let (light_dc_neg_col, _) = light_canvas.wire_color(&dc_neg_wire);
    let (light_custom_col, _) = light_canvas.wire_color(&custom_wire);
    assert!(luminance(light_def_col) < 0.30);
    assert!(luminance(light_dc_neg_col) < 0.30);
    assert!((light_custom_col.r - (0x12 as f32 / 255.0)).abs() < 0.01);
    assert!((light_custom_col.g - (0xAB as f32 / 255.0)).abs() < 0.01);
    assert!((light_custom_col.b - (0x34 as f32 / 255.0)).abs() < 0.01);

    // Tema DARK: folha escura, traços/elementos claros e conectores padrão claros
    let _ = app.update(Message::SetThemeMode(ThemeMode::Dark));
    assert!(app.is_dark());
    let dark_canvas = mk_canvas(app.is_dark());
    let dark_tok = dark_canvas.tokens();
    assert!(luminance(dark_tok.sheet) < 0.10);
    assert!(luminance(dark_tok.ink) > 0.75);
    assert!(luminance(dark_tok.sheet_line) > 0.70);

    let (dark_def_col, _) = dark_canvas.wire_color(&default_wire);
    let (dark_dc_neg_col, _) = dark_canvas.wire_color(&dc_neg_wire);
    let (dark_custom_col, _) = dark_canvas.wire_color(&custom_wire);
    assert!(luminance(dark_def_col) > 0.75);
    assert!(luminance(dark_dc_neg_col) > 0.75);
    assert!((dark_custom_col.r - light_custom_col.r).abs() < 0.001);
    assert!((dark_custom_col.g - light_custom_col.g).abs() < 0.001);
    assert!((dark_custom_col.b - light_custom_col.b).abs() < 0.001);

    // Tema SYSTEM acompanha mudanças do sistema operacional dinamicamente
    let _ = app.update(Message::SetThemeMode(ThemeMode::System));
    assert_eq!(app.theme_mode, ThemeMode::System);
    let _ = app.update(Message::SystemThemeDetected(false));
    assert!(!app.is_dark());
    let _ = app.update(Message::SystemThemeDetected(true));
    assert!(app.is_dark());
    let _ = app.update(Message::SystemThemeDetected(false));
    assert!(!app.is_dark());
}

#[test]
fn test_section1_2_text_tool_placement_modal_cancel_edit_inspector_and_qet_roundtrip() {
    use cade_simu_rs::qet::{
        diagram_to_document, document_to_diagram, load_qet_from_str, serialize_qet_xml, QetProject,
    };
    use iced::widget::text_editor::{Action as EditorAction, Edit as EditorEdit};

    let (mut app, _) = CadeSimuApp::new();
    let _ = app.update(Message::NewDocument);
    let initial_count = app.doc.items.len();

    // 1. Ativa ferramenta Texto em Linha via atalho/seleção e insere no canvas
    let _ = app.update(Message::SelectTool(EditorTool::PlaceText));
    assert_eq!(app.tool, EditorTool::PlaceText);

    let _ = app.update(Message::Canvas(CanvasMessage::AddGraphicItem {
        tipo: 8,
        subvariant: 0,
        x1: 50,
        y1: 60,
        x2: 80,
        y2: 68,
    }));
    assert!(app.show_text_editor_modal);
    assert!(app.text_editor_is_new);
    assert_eq!(app.doc.items.len(), initial_count + 1);

    // Cancelar inserção nova deve descartar o item temporário
    let _ = app.update(Message::CloseTextEditorModal);
    assert!(!app.show_text_editor_modal);
    assert_eq!(app.doc.items.len(), initial_count);

    // 2. Nova inserção via Campo de Texto (subvariant == 1), edição multilinha e formatação no modal
    let _ = app.update(Message::SelectTool(EditorTool::PlaceTextField));
    let _ = app.update(Message::Canvas(CanvasMessage::AddGraphicItem {
        tipo: 8,
        subvariant: 1,
        x1: 40,
        y1: 40,
        x2: 110,
        y2: 65,
    }));
    assert!(app.show_text_editor_modal);

    for ch in "Quadro Geral\n440V 60Hz".chars() {
        let action = if ch == '\n' {
            EditorAction::Edit(EditorEdit::Enter)
        } else {
            EditorAction::Edit(EditorEdit::Insert(ch))
        };
        let _ = app.update(Message::TextEditorAction(action));
    }
    let _ = app.update(Message::SetTextEditorSize(4.8));
    let _ = app.update(Message::ToggleTextEditorBold);
    let _ = app.update(Message::SetTextEditorOrientation(1));
    let _ = app.update(Message::SetTextEditorColor(0x2563EB));
    let _ = app.update(Message::ConfirmTextEditorModal);

    assert!(!app.show_text_editor_modal);
    assert_eq!(app.doc.items.len(), initial_count + 1);
    let item_id = app.selected_id.expect("Texto deve ficar selecionado");
    {
        let it = app
            .doc
            .items
            .iter()
            .find(|i| i.id == item_id)
            .expect("Item de texto");
        assert_eq!(it.tipo, 8);
        assert_eq!(it.subvariant, 1);
        assert_eq!(it.trailing_str, "Quadro Geral\n440V 60Hz");
        assert!((it.text_world_height() - 4.8).abs() < 0.05);
        assert_eq!(it.extra_int1, -24);
        assert!(it.text_is_bold());
        assert_eq!(it.show_value, 700);
        assert_eq!(it.orientation, 1);
        assert_eq!(it.color, 0x2563EB);
    }

    // 3. Duplo clique no canvas abre o editor de item existente; cancelar preserva alterações anteriores
    let _ = app.update(Message::Canvas(CanvasMessage::EditTextItem(item_id)));
    assert!(app.show_text_editor_modal);
    assert!(!app.text_editor_is_new);
    let _ = app.update(Message::ToggleTextEditorBold);
    let _ = app.update(Message::CloseTextEditorModal);
    assert!(app
        .doc
        .items
        .iter()
        .find(|i| i.id == item_id)
        .unwrap()
        .text_is_bold());

    // 4. Ajustes pelo Inspector atualizam modelo e bounding box
    let _ = app.update(Message::SetItemTextSize(6.4));
    let _ = app.update(Message::AdjustItemTextSize(0.4));
    let _ = app.update(Message::ToggleItemTextBold);
    let _ = app.update(Message::SetItemTextOrientation(3));
    let _ = app.update(Message::SetItemTextSubvariant(0));
    {
        let it = app.doc.items.iter().find(|i| i.id == item_id).unwrap();
        assert_eq!(it.subvariant, 0);
        assert!((it.text_world_height() - 6.8).abs() < 0.05);
        assert!(!it.text_is_bold());
        assert_eq!(it.orientation, 3);
    }

    // 5. Roundtrip QET preserva texto, tamanho, negrito e rotação
    let _ = app.update(Message::ToggleItemTextBold);
    let diag = document_to_diagram(&app.doc, "Teste Texto", 1, "folio1");
    let qet = QetProject {
        title: "Teste Texto".to_string(),
        version: "0.90".to_string(),
        diagrams: vec![diag],
        ..Default::default()
    };
    let xml = serialize_qet_xml(&qet);
    let parsed_qet = load_qet_from_str(&xml).expect("Parse QET gerado");
    let roundtrip_doc = diagram_to_document(&parsed_qet, 0);
    let rt_text = roundtrip_doc
        .items
        .iter()
        .find(|i| i.tipo == 8 && i.trailing_str.contains("Quadro Geral"))
        .expect("Texto preservado no roundtrip QET");
    assert!(rt_text.text_is_bold());
    assert_eq!(rt_text.orientation, 3);
    assert!(rt_text.text_world_height() > 5.0);

    // 6. Carregamento de projeto real .json com itens tipo == 8 preserva alturas/estilos
    let sample_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("Projetos")
        .join("Partida Direta.json");
    if sample_path.exists() {
        let project = load_document(&sample_path).expect("Projeto real Partida Direta.json");
        let textos: Vec<&SchematicItem> = project.items.iter().filter(|i| i.tipo == 8).collect();
        assert!(!textos.is_empty());
        for t in textos {
            let h = t.text_world_height();
            assert!((2.0..=16.0).contains(&h));
            let (bx1, by1, bx2, by2) = SchematicCanvas::item_bounds(t);
            assert!(bx2 > bx1 && by2 > by1);
        }
    }
}

#[test]
fn test_section1_2_vector_annotation_tools_handles_styles_and_rotation() {
    let (mut app, _) = CadeSimuApp::new();
    let _ = app.update(Message::NewDocument);

    // 1. Seleção contínua de ferramentas vetoriais e desenho via AddGraphicItem
    let tools = [
        (EditorTool::DrawGraphicLine, 1),
        (EditorTool::DrawGraphicRect, 2),
        (EditorTool::DrawGraphicCircle, 3),
        (EditorTool::DrawGraphicArc, 4),
    ];
    for (tool, expected_tipo) in tools {
        let _ = app.update(Message::SelectTool(tool.clone()));
        assert_eq!(app.tool, tool);
        let _ = app.update(Message::Canvas(CanvasMessage::AddGraphicItem {
            tipo: expected_tipo,
            subvariant: 0,
            x1: 100,
            y1: 100,
            x2: 180,
            y2: 160,
        }));
        // Ferramenta vetorial permanece ativa para desenho contínuo
        assert_eq!(app.tool, tool);
        let added = app.doc.items.last().expect("Item vetorial inserido");
        assert_eq!(added.tipo, expected_tipo);
    }
    let _ = app.update(Message::CancelAction);
    assert_eq!(app.tool, EditorTool::Select);

    // 2. Arraste de alça de redimensionamento + Undo
    let rect_id = app
        .doc
        .items
        .iter()
        .find(|i| i.tipo == 2)
        .expect("Retângulo criado")
        .id;
    let _ = app.update(Message::Canvas(CanvasMessage::SelectItem(Some(rect_id))));
    let _ = app.update(Message::Canvas(CanvasMessage::ResizeGraphicHandle {
        id: rect_id,
        handle: 1,
        x: 240,
        y: 200,
    }));
    let _ = app.update(Message::Canvas(CanvasMessage::EndResizeGraphicHandle));
    let resized = app.doc.items.iter().find(|i| i.id == rect_id).unwrap();
    assert_eq!((resized.x2, resized.y2), (240, 200));

    let _ = app.update(Message::Undo);
    let reverted = app.doc.items.iter().find(|i| i.id == rect_id).unwrap();
    assert_eq!((reverted.x2, reverted.y2), (180, 160));

    // 3. Estilos no Inspetor: estilo de linha, espessura, preenchimento, cantos arredondados
    let _ = app.update(Message::SetItemGraphicLineStyle(GraphicLineStyle::Dash));
    let _ = app.update(Message::SetItemGraphicStrokeWidth(2.0));
    let _ = app.update(Message::AdjustItemGraphicStrokeWidth(0.5));
    let _ = app.update(Message::ToggleItemGraphicFill);
    let _ = app.update(Message::ToggleItemRectRounded);
    let styled_rect = app.doc.items.iter().find(|i| i.id == rect_id).unwrap();
    assert_eq!(styled_rect.graphic_line_style(), GraphicLineStyle::Dash);
    assert!((styled_rect.graphic_stroke_width() - 2.5).abs() < 0.01);
    assert!(styled_rect.graphic_has_fill());
    assert!(styled_rect.graphic_rounded_corners());

    // 4. Rotação 90 graus de arco e presets de varredura
    let arc_id = app
        .doc
        .items
        .iter()
        .find(|i| i.tipo == 4)
        .expect("Arco criado")
        .id;
    let _ = app.update(Message::Canvas(CanvasMessage::SelectItem(Some(arc_id))));
    let _ = app.update(Message::SetItemArcSweepPreset(180));
    let arc_after_preset = app.doc.items.iter().find(|i| i.id == arc_id).unwrap();
    assert_eq!(arc_after_preset.aux_coords[1], 180);

    let _ = app.update(Message::RotateClockwise);
    let arc_cw = app.doc.items.iter().find(|i| i.id == arc_id).unwrap();
    assert_eq!(arc_cw.aux_coords[0], 270);
    assert_eq!(arc_cw.aux_coords[1], 180);

    let _ = app.update(Message::RotateCounterClockwise);
    let arc_ccw = app.doc.items.iter().find(|i| i.id == arc_id).unwrap();
    assert_eq!(arc_ccw.aux_coords[0], 0);
    assert_eq!(arc_ccw.aux_coords[1], 180);

    // 5. Validação em projetos reais: linha tracejada e moldura arredondada
    let proj_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Projetos");
    let inv_path = proj_dir.join("Inversão de rotação motor trifásico.json");
    if inv_path.exists() {
        let doc = load_document(&inv_path).expect("Carregar Inversão de rotação");
        assert!(
            doc.items
                .iter()
                .any(|i| i.tipo == 1 && i.graphic_line_style() == GraphicLineStyle::Dash),
            "Projeto real deve preservar linha tracejada (extra_int1 == 1)"
        );
    }
    let pd_path = proj_dir.join("Partida Direta.json");
    if pd_path.exists() {
        let doc = load_document(&pd_path).expect("Carregar Partida Direta");
        assert!(
            doc.items
                .iter()
                .any(|i| i.tipo == 2 && i.graphic_rounded_corners()),
            "Projeto real deve preservar retângulo com cantos arredondados (subvariant == 1)"
        );
    }
}

#[test]
fn test_vector_z_order_under_elements_and_alt_click_cycling() {
    use std::collections::HashSet;

    let mut doc = SchematicDocument::default();
    // 1. Símbolo elétrico (Bobina tipo 3000) em (100, 100)
    let mut coil = SchematicItem::new_symbol(1, 3000, 100, 100, "-K1");
    coil.id = 1;
    doc.items.push(coil);

    // 2. Cabo (tipo 4000) passando exatamente sobre (100, 100)
    let wire = SchematicItem::new_wire(2, 4000, 60, 100, 140, 100);
    doc.items.push(wire);

    // 3. Texto acima dos demais (tipo 8) cobrindo (100, 100)
    let mut txt = SchematicItem::new_graphic(3, 8, 92, 95, 118, 105);
    txt.trailing_str = "K1".to_string();
    txt.sync_text_bounds();
    doc.items.push(txt);

    // 4. Retângulo vetorial preenchido adicionado POR ÚLTIMO no Vec (tipo 2), cobrindo tudo
    let mut rect = SchematicItem::new_graphic(4, 2, 50, 70, 160, 130);
    rect.show_value = 1;
    doc.items.push(rect);

    let sel = HashSet::new();
    let mk_canvas = |selected_id: Option<i32>| SchematicCanvas {
        doc: &doc,
        tool: EditorTool::Select,
        selected_id,
        selected_ids: &sel,
        is_shift_pressed: false,
        is_ctrl_pressed: false,
        is_alt_pressed: false,
        sim_running: false,
        show_grid: true,
        snap_to_grid: true,
        grid_density: GridDensity::Medium,
        show_texts: true,
        show_terminals: true,
        show_cajetin: true,
        show_full_crosshair: false,
        show_text_grid: false,
        show_alignment_guides: false,
        keep_headers_visible: false,
        show_cell_bounds: false,
        is_dark: false,
        auto_connect: true,
        auto_disconnect: false,
        zoom: 1.0,
        pan: iced::Vector::ZERO,
        placement_orientation: 0,
        dragging_symbol: None,
        dragging_canonical_symbol: None,
        is_blocked: false,
    };

    // Mesmo o retângulo tendo sido inserido por último, elementos/cabos têm prioridade no clique normal
    let c0 = mk_canvas(None);
    assert_eq!(c0.hit_test(100, 100), Some(3)); // Topo: texto (id 3)
    assert_eq!(c0.hit_test(70, 100), Some(2)); // Cabo acima do retângulo (id 2)
    assert_eq!(c0.hit_test(58, 78), Some(4)); // Região apenas do retângulo (id 4)

    // Pilha de cima para baixo em (100, 100): [texto (3), símbolo (1), cabo (2), retângulo (4)]
    assert_eq!(c0.hit_test_all(100, 100), vec![3, 1, 2, 4]);

    // Alt+Clique sem nada selecionado pega o objeto logo abaixo do topo (id 1) e cicla um por vez
    assert_eq!(c0.hit_test_cycle(100, 100), Some(1));
    assert_eq!(mk_canvas(Some(3)).hit_test_cycle(100, 100), Some(1));
    assert_eq!(mk_canvas(Some(1)).hit_test_cycle(100, 100), Some(2));
    assert_eq!(mk_canvas(Some(2)).hit_test_cycle(100, 100), Some(4));
    assert_eq!(mk_canvas(Some(4)).hit_test_cycle(100, 100), Some(3));
    assert_eq!(c0.hit_test_cycle(58, 78), Some(4)); // Item único sob o cursor

    // No SVG exportado, o <rect vetorial aparece ANTES do <line do cabo e dos elementos/textos
    let svg = export_to_svg(&doc, None, false);
    let body = &svg[svg
        .find("<g stroke-linecap=\"round\"")
        .expect("Grupo principal no SVG")..];
    let rect_pos = body
        .find("fill-opacity=\"0.12\"")
        .expect("Retângulo vetorial no SVG");
    let wire_pos = body.find("<line x1=").expect("Cabo no SVG");
    let text_pos = body.find(">K1<").expect("Texto no SVG");
    assert!(rect_pos < wire_pos && wire_pos < text_pos);
}

#[test]
fn test_bidirectional_dictionary_and_all_projetos_qet_export_and_roundtrip() {
    use cade_simu_rs::dictionary::global_concept_dictionary;
    use cade_simu_rs::qet::{export_doc_to_qet, load_qet_file};
    use cade_simu_rs::sim::rules::{is_item_manual, is_item_momentary};
    use cade_simu_rs::symbols::{classify_item_role, ApparatusRole};
    use std::path::PathBuf;

    let dict = global_concept_dictionary();

    // 1) Verifica convergência semântica de um elemento externo QET (tipo 20000) para conceito canônico,
    // simulação, XRef e exportação .cad (tipo 8000)
    let mut qet_pb = SchematicItem::new_symbol(1, 20000, 10, 10, "-S1");
    qet_pb.pins[0] = "13".to_string();
    qet_pb.pins[1] = "14".to_string();
    qet_pb.trailing_str =
        "qet:10_allpole/380_signaling_operating/20_push_buttons/push-button.elmt".to_string();
    let res = dict
        .resolve_item(&qet_pb)
        .expect("Botão QET externo deve resolver no dicionário canônico");
    assert_eq!(res.concept.concept_id, "el.act.pb.no");
    assert_eq!(res.concept.cade_tipo, Some(8000));
    assert_eq!(classify_item_role(&qet_pb), ApparatusRole::SwitchOrButton);
    assert!(is_item_manual(&qet_pb));
    assert!(is_item_momentary(&qet_pb));

    let mut mini_doc = SchematicDocument::default();
    mini_doc.items.push(qet_pb);
    let cad_str = serialize_cad_str(&mini_doc);
    let parsed_cad = parse_cad_str(&cad_str).expect("Deve parsear .cad exportado de item QET");
    assert_eq!(parsed_cad.items.len(), 1);
    assert_eq!(parsed_cad.items[0].tipo, 8000);
    assert_eq!(
        parsed_cad.items[0].concept_id.as_deref(),
        Some("el.act.pb.no")
    );

    // 2) Exporta todos os projetos da pasta Projetos/ como .qet para inspeção visual no QElectroTech,
    // valida a coleção embarcada e os condutores, reabre o .qet e executa a simulação completa
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let projetos_dir = if manifest.join("Projetos").is_dir() {
        manifest.join("Projetos")
    } else {
        manifest.parent().unwrap().join("Projetos")
    };

    #[allow(clippy::type_complexity)]
    let test_cases: Vec<(&str, Option<fn(SchematicDocument, &str)>)> = vec![
        (
            "EnchimentodeTanqueFluido",
            Some(simulate_enchimento_de_tanque_fluido),
        ),
        (
            "Inversão de rotação motor trifásico",
            Some(simulate_inversao_rotacao_motor_trifasico),
        ),
        (
            "Estrela triângulo sem inversão de rotação",
            Some(simulate_estrela_triangulo_sem_inversao),
        ),
        (
            "Estrela triângulo com inversão de rotação",
            Some(simulate_estrela_triangulo_com_inversao),
        ),
        (
            "Estrela triângulo com inversão de rotação automático",
            Some(simulate_estrela_triangulo_com_inversao_automatico),
        ),
        ("Ligação três motores", Some(simulate_ligacao_tres_motores)),
        ("Partida Direta", Some(simulate_partida_direta)),
        ("Exemplo Desconexão", Some(simulate_exemplo_desconexao)),
        (
            "Partida de Motores Escalonada",
            Some(simulate_partida_motores_escalonada),
        ),
        ("teste", None),
    ];

    for (stem, maybe_sim_fn) in test_cases {
        let cad_path = projetos_dir.join(format!("{stem}.cad"));
        let qet_path = projetos_dir.join(format!("{stem}.qet"));

        let doc_cad = load_document(&cad_path)
            .unwrap_or_else(|e| panic!("Erro ao abrir {:?}: {}", cad_path, e));
        assert!(
            !doc_cad.items.is_empty(),
            "Projeto {stem}.cad não deve estar vazio"
        );

        // Todos os componentes elétricos do .cad devem ter concept_id atribuído pelo parser
        for item in &doc_cad.items {
            let is_cade_sentinel =
                item.tipo == 20000 && item.tag == "XXX" && item.trailing_str.is_empty();
            if !item.is_wire() && !item.is_junction() && !item.is_graphic() && !is_cade_sentinel {
                assert!(
                    item.concept_id.is_some(),
                    "Item id={} tipo={} em {}.cad deveria ter concept_id",
                    item.id,
                    item.tipo,
                    stem
                );
            }
        }

        // Salva como .qet diretamente em Projetos/
        export_doc_to_qet(&doc_cad, &qet_path)
            .unwrap_or_else(|e| panic!("Erro ao exportar {:?}: {}", qet_path, e));

        let qet_xml = std::fs::read_to_string(&qet_path)
            .unwrap_or_else(|e| panic!("Erro ao ler {:?}: {}", qet_path, e));
        assert!(
            qet_xml.contains("<collection>") && qet_xml.contains("<category name=\"import\">"),
            "Arquivo {:?} deve conter coleção QET embarcada",
            qet_path
        );

        // Verifica integridade estrutural do diagrama QET (todos os condutores apontam para elementos e terminais existentes)
        let qet_proj = load_qet_file(&qet_path)
            .unwrap_or_else(|e| panic!("Erro ao parsear XML QET {:?}: {}", qet_path, e));
        assert_eq!(qet_proj.diagrams.len(), 1);
        let diag = &qet_proj.diagrams[0];
        assert!(
            !diag.elements.is_empty(),
            "Diagrama QET de {stem} deve conter elementos"
        );
        assert!(
            !diag.conductors.is_empty(),
            "Diagrama QET de {stem} deve conter condutores"
        );

        for cond in &diag.conductors {
            assert!(
                cond.element1.starts_with('{') && cond.element1.ends_with('}'),
                "UUID element1 inválido em {stem}.qet: {}",
                cond.element1
            );
            assert!(
                cond.element2.starts_with('{') && cond.element2.ends_with('}'),
                "UUID element2 inválido em {stem}.qet: {}",
                cond.element2
            );
            let el1 = diag
                .elements
                .iter()
                .find(|e| e.uuid == cond.element1)
                .unwrap_or_else(|| {
                    panic!(
                        "Condutor referencia element1 inexistente {} em {}.qet",
                        cond.element1, stem
                    )
                });
            let el2 = diag
                .elements
                .iter()
                .find(|e| e.uuid == cond.element2)
                .unwrap_or_else(|| {
                    panic!(
                        "Condutor referencia element2 inexistente {} em {}.qet",
                        cond.element2, stem
                    )
                });
            let def1 = qet_proj
                .element_definitions
                .get(&el1.element_type)
                .unwrap_or_else(|| {
                    panic!(
                        "Definição embarcada ausente para {} em {}.qet",
                        el1.element_type, stem
                    )
                });
            let def2 = qet_proj
                .element_definitions
                .get(&el2.element_type)
                .unwrap_or_else(|| {
                    panic!(
                        "Definição embarcada ausente para {} em {}.qet",
                        el2.element_type, stem
                    )
                });
            assert!(
                def1.terminals.iter().any(|t| t.uuid == cond.terminal1),
                "Condutor referencia terminal1 inexistente {} na definição {} em {}.qet",
                cond.terminal1,
                el1.element_type,
                stem
            );
            assert!(
                def2.terminals.iter().any(|t| t.uuid == cond.terminal2),
                "Condutor referencia terminal2 inexistente {} na definição {} em {}.qet",
                cond.terminal2,
                el2.element_type,
                stem
            );
        }

        // Reabre o .qet como SchematicDocument e verifica equivalência topológica/semântica e simulação
        let doc_qet = load_document(&qet_path)
            .unwrap_or_else(|e| panic!("Erro ao reabrir {:?}: {}", qet_path, e));

        let comps_cad: Vec<_> = doc_cad
            .items
            .iter()
            .filter(|i| {
                !i.is_wire()
                    && !i.is_junction()
                    && !i.is_graphic()
                    && !(i.tipo == 20000 && i.tag == "XXX" && i.trailing_str.is_empty())
            })
            .collect();
        let comps_qet: Vec<_> = doc_qet
            .items
            .iter()
            .filter(|i| {
                !i.is_wire()
                    && !i.is_junction()
                    && !i.is_graphic()
                    && !(i.tipo == 20000 && i.tag == "XXX" && i.trailing_str.is_empty())
            })
            .collect();
        assert_eq!(
            comps_cad.len(),
            comps_qet.len(),
            "{stem}: quantidade de componentes elétricos diverge no .qet"
        );
        for (ca, cb) in comps_cad.iter().zip(comps_qet.iter()) {
            assert_eq!(ca.tipo, cb.tipo, "{stem}: tipo diverge");
            assert_eq!(ca.tag, cb.tag, "{stem}: tag diverge");
            assert_eq!(ca.value, cb.value, "{stem}: value diverge");
            assert_eq!(ca.pins, cb.pins, "{stem}: pins diverge");
            assert_eq!(
                (ca.x1, ca.y1),
                (cb.x1, cb.y1),
                "{stem}: coordenadas do componente {} divergem",
                ca.tag
            );
            assert_eq!(
                ca.orientation, cb.orientation,
                "{stem}: orientação de {} diverge",
                ca.tag
            );
            assert_eq!(
                ca.concept_id, cb.concept_id,
                "{stem}: concept_id de {} diverge",
                ca.tag
            );
        }

        let wires_cad_count = doc_cad
            .items
            .iter()
            .filter(|i| i.is_wire() && (i.x1 != i.x2 || i.y1 != i.y2))
            .count();
        let wires_qet_count = doc_qet
            .items
            .iter()
            .filter(|i| i.is_wire() && (i.x1 != i.x2 || i.y1 != i.y2))
            .count();
        assert_eq!(
            wires_cad_count, wires_qet_count,
            "{stem}: quantidade de condutores válidos diverge no .qet"
        );

        // Roundtrip .qet -> .cad -> SchematicDocument
        let doc_cad_from_qet = parse_cad_str(&serialize_cad_str(&doc_qet))
            .unwrap_or_else(|e| panic!("Erro no roundtrip .qet -> .cad de {stem}: {}", e));
        verify_doc_equivalence(
            &doc_qet,
            &doc_cad_from_qet,
            &format!("{stem} (.qet vs .qet -> .cad)"),
        );

        if let Some(sim_fn) = maybe_sim_fn {
            sim_fn(doc_qet, &format!("{stem}.qet"));
            sim_fn(doc_cad_from_qet, &format!("{stem}.qet->.cad"));
        }
    }
}
