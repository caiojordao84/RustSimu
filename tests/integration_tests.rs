use iced::Color;

use cade_simu_rs::app::*;
use cade_simu_rs::canvas_view::*;
use cade_simu_rs::io::*;
use cade_simu_rs::model::*;
use cade_simu_rs::sim::*;
use cade_simu_rs::symbols::*;
use cade_simu_rs::theme::{self, *};

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
    assert!(!m1.sim_active);
    assert_eq!(m1.sim_motor_dir, 0);

    // 1) Fecha o disjuntor tripolar -Q1 (bi-estável)
    let q1_id = doc.items.iter().find(|i| i.tag == "-Q1").unwrap().id;
    press_component_on_sim(&mut doc, q1_id);
    release_component_on_sim(&mut doc, q1_id);
    step_simulation(&mut doc, 50);

    // Motor ainda parado porque -KM1 está aberto
    let m1 = doc.items.iter().find(|i| i.tipo == 1000).unwrap();
    assert!(!m1.sim_active);

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
    assert!(km1_coil.sim_active, "Bobina -KM1 deveria estar energizada");

    let m1 = doc.items.iter().find(|i| i.tipo == 1000).unwrap();
    assert!(m1.sim_active, "Motor -M1 deveria estar girando");
    assert_eq!(m1.sim_motor_dir, 1);

    // 3) Solta o botão pulsador -S1 -> -S1 retorna automaticamente ao estado original (aberto),
    //    e o motor DEVE permanecer ligado pelo contato de selo 13-14 de -KM1!
    release_component_on_sim(&mut doc, s1_id);
    step_simulation(&mut doc, 50);
    let s1 = doc.items.iter().find(|i| i.id == s1_id).unwrap();
    assert!(
        !s1.sim_manual_latched && !s1.sim_active,
        "Botão -S1 deve retornar ao estado original quando solto"
    );

    let m1 = doc.items.iter().find(|i| i.tipo == 1000).unwrap();
    assert!(
        m1.sim_active,
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
        !m1.sim_active,
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
            it.sim_manual_latched && it.sim_active,
            "Botão tipo {} deve ativar enquanto pressionado",
            it.tipo
        );

        release_component_on_sim(&mut doc, id);
        step_simulation(&mut doc, 10);
        let it = doc.items.iter().find(|i| i.id == id).unwrap();
        assert!(
            !it.sim_manual_latched && !it.sim_active,
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
    assert!(sw.sim_manual_latched && sw.sim_active);

    press_component_on_sim(&mut doc, 5);
    release_component_on_sim(&mut doc, 5);
    step_simulation(&mut doc, 10);
    let sw = doc.items.iter().find(|i| i.id == 5).unwrap();
    assert!(!sw.sim_manual_latched && !sw.sim_active);
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
    assert!(!k1.sim_active, "{label}: K1 deve iniciar desligado");

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
        k1.sim_active,
        "{label}: K1 deve ficar retido após pulsar E1"
    );
    assert!(
        s1.sim_active,
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
    assert!(!k1.sim_active, "{label}: K1 deve desligar ao atingir E2");
    assert!(fs.sim_active, "{label}: FS deve ligar após E2");
    assert!(t1.sim_active, "{label}: T1 deve iniciar contagem de 2s");

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
    assert!(k2.sim_active, "{label}: K2 deve ligar após 2s de T1");
    assert!(s2.sim_active, "{label}: S2 deve abrir esvaziamento após T1");
    assert!(t2.sim_active, "{label}: T2 deve iniciar contagem de 3s");

    // Avança 3.1s -> T2 dispara e abre contato NF T2 desligando S2
    step_simulation(&mut doc, 3100);
    let s2 = doc
        .items
        .iter()
        .find(|i| i.tipo == 9000 && i.tag == "S2")
        .unwrap();
    assert!(!s2.sim_active, "{label}: S2 deve desligar após 3s de T2");
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
    assert!(m.sim_active, "{label}: Motor -M deve ligar com B1");
    assert_eq!(
        m.sim_motor_dir, 1,
        "{label}: Motor -M deve girar CW (1) com K1"
    );
    assert_eq!(m.sim_motor_mode, MotorMode::Direct);

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
    assert!(!m.sim_active, "{label}: Motor -M deve parar com B0");

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
    assert!(m.sim_active, "{label}: Motor -M deve ligar com B2");
    assert_eq!(
        m.sim_motor_dir, -1,
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
    assert!(m.sim_active, "{label}: Motor deve partir ao pulsar S1");
    assert_eq!(m.sim_motor_dir, 1, "{label}: Motor deve girar CW");
    assert_eq!(
        m.sim_motor_mode,
        MotorMode::Star,
        "{label}: Motor deve iniciar em Estrela (Y)"
    );

    // Avança 5.1s -> Comuta para Triângulo (K1 + K2)
    step_simulation(&mut doc, 5100);
    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert!(m.sim_active, "{label}: Motor deve continuar ligado após 5s");
    assert_eq!(m.sim_motor_dir, 1, "{label}: Motor deve manter sentido CW");
    assert_eq!(
        m.sim_motor_mode,
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
        (m.sim_active, m.sim_motor_dir, m.sim_motor_mode),
        (true, 1, MotorMode::Star)
    );

    step_simulation(&mut doc, 5100);
    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert_eq!(
        (m.sim_active, m.sim_motor_dir, m.sim_motor_mode),
        (true, 1, MotorMode::Delta)
    );

    // 2) Parada (B0)
    set_manual_by_tag(&mut doc, "B0", true);
    step_simulation(&mut doc, 50);
    set_manual_by_tag(&mut doc, "B0", false);
    step_simulation(&mut doc, 50);
    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert!(!m.sim_active, "{label}: Motor deve parar com B0");

    // 3) Sentido anti-horário (B2): Estrela -> Triângulo
    set_manual_by_tag(&mut doc, "B2", true);
    step_simulation(&mut doc, 50);
    set_manual_by_tag(&mut doc, "B2", false);
    step_simulation(&mut doc, 50);

    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert_eq!(
        (m.sim_active, m.sim_motor_dir, m.sim_motor_mode),
        (true, -1, MotorMode::Star)
    );

    step_simulation(&mut doc, 5100);
    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert_eq!(
        (m.sim_active, m.sim_motor_dir, m.sim_motor_mode),
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
        (m.sim_active, m.sim_motor_dir, m.sim_motor_mode),
        (true, 1, MotorMode::Star),
        "{label}: t=0s deve estar CW Star"
    );

    // Avança 5.5s -> Horário em Triângulo (K1 + K3)
    step_secs(&mut doc, 5.5);
    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert_eq!(
        (m.sim_active, m.sim_motor_dir, m.sim_motor_mode),
        (true, 1, MotorMode::Delta),
        "{label}: t=5.5s deve estar CW Delta"
    );

    // Avança +5.5s (t=11s) -> Pausa de frenagem/inércia antes de inverter (K5 TOF contando 5s)
    step_secs(&mut doc, 5.5);
    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert_eq!(
        (m.sim_active, m.sim_motor_dir, m.sim_motor_mode),
        (false, 0, MotorMode::Off),
        "{label}: t=11s deve estar em pausa antes de inverter"
    );

    // Avança +5.5s (t=16.5s) -> Inversão automática para Anti-horário em Estrela (K2 + K4)
    step_secs(&mut doc, 5.5);
    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert_eq!(
        (m.sim_active, m.sim_motor_dir, m.sim_motor_mode),
        (true, -1, MotorMode::Star),
        "{label}: t=16.5s deve estar CCW Star"
    );

    // Avança +5.5s (t=22s) -> Anti-horário em Triângulo (K2 + K3)
    step_secs(&mut doc, 5.5);
    let m = doc.items.iter().find(|i| i.tipo == 1001).unwrap();
    assert_eq!(
        (m.sim_active, m.sim_motor_dir, m.sim_motor_mode),
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
            .map(|i| (i.tag.clone(), i.sim_motor_mode))
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
    assert!(!mo.sim_active, "{label}: Motor MO deve iniciar desligado");

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
    assert!(mo.sim_active, "{label}: Motor MO deve girar após pulsar B1");
    assert_eq!(mo.sim_motor_dir, 1);

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
        !mo.sim_active,
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
    assert!(m1.sim_active, "{label}: Motor 1 (K1) deve ligar com B1");
    assert!(!m2.sim_active, "{label}: Motor 2 (K2) ainda desligado");

    // Pulsa B0 -> desliga K1 (motor 1) e dispara temporizador Off-Delay TP (30s) ligando K2 (motor 2)
    set_manual_by_tag(&mut doc, "B0", true);
    step_simulation(&mut doc, 100);
    set_manual_by_tag(&mut doc, "B0", false);
    step_simulation(&mut doc, 100);

    let m1 = doc.items.iter().find(|i| i.id == 6).unwrap();
    let m2 = doc.items.iter().find(|i| i.id == 7).unwrap();
    assert!(
        !m1.sim_active,
        "{label}: Motor 1 deve desligar ao pulsar B0"
    );
    assert!(
        m2.sim_active,
        "{label}: Motor 2 (K2) deve ficar ligado durante os 30s do Off-Delay TP"
    );

    // Avança 30.5s -> TP expira e desliga K2 (motor 2)
    step_secs(&mut doc, 30.5);
    let m2 = doc.items.iter().find(|i| i.id == 7).unwrap();
    assert!(
        !m2.sim_active,
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
        let m1 = d
            .items
            .iter()
            .find(|i| i.tipo == 1000 && i.tag == "M1")
            .unwrap()
            .sim_active;
        let m2 = d
            .items
            .iter()
            .find(|i| i.tipo == 1000 && i.tag == "M2")
            .unwrap()
            .sim_active;
        let m3 = d
            .items
            .iter()
            .find(|i| i.tipo == 1000 && i.tag == "M3")
            .unwrap()
            .sim_active;
        let ok = d
            .items
            .iter()
            .find(|i| i.tipo == 9008 && i.tag == "OK")
            .unwrap()
            .sim_active;
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

    assert_eq!(no_idle.term_top_y, -21.0);
    assert_eq!(no_idle.term_bottom_y, 21.0);

    // Mapeamento de terminais para escala de grade do CADe (distância 12 unidades)
    let (t0_x, t0_y) = no_idle.map_point(0.0, -21.0, 12.0);
    let (t1_x, t1_y) = no_idle.map_point(0.0, 21.0, 12.0);
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

    assert_eq!(nc_idle.term_top_y, -21.0);
    assert_eq!(nc_idle.term_bottom_y, 21.0);

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
    assert_eq!(th_idle.max_term_x, 40.0);
    assert!(
        th_idle
            .shapes
            .iter()
            .any(|s| matches!(s, SvgShape::Rect { dashed: true, .. })),
        "Relé térmico 4 deve conter linha pontilhada em volta (rect dashed)"
    );
    let (p1x, _) = th_idle.map_point(0.0, -21.0, 12.0);
    let (p2x, _) = th_idle.map_point(20.0, -21.0, 12.0);
    let (p3x, _) = th_idle.map_point(40.0, -21.0, 12.0);
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
    assert_eq!(b2_idle.min_term_x, -20.0);
    assert_eq!(b2_idle.max_term_x, 0.0);
    let (p1, p1_y) = b2_idle.map_point(-20.0, -40.0, 21.0);
    let (p2, p2_y) = b2_idle.map_point(0.0, 40.0, 21.0);
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
    assert_eq!(b3_idle.min_term_x, -20.0);
    assert_eq!(b3_idle.max_term_x, 20.0);
    let (b3_p1, _) = b3_idle.map_point(-20.0, -40.0, 21.0);
    let (b3_p2, _) = b3_idle.map_point(0.0, -40.0, 21.0);
    let (b3_p3, b3_p3_y) = b3_idle.map_point(20.0, 40.0, 21.0);
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
    assert_eq!(b4_idle.min_term_x, -20.0);
    assert_eq!(b4_idle.max_term_x, 40.0);
    let (b4_pn, _) = b4_idle.map_point(-20.0, -40.0, 21.0);
    let (b4_p1, _) = b4_idle.map_point(0.0, -40.0, 21.0);
    let (b4_p2, _) = b4_idle.map_point(20.0, -40.0, 21.0);
    let (b4_p3, b4_p3_y) = b4_idle.map_point(40.0, 40.0, 21.0);
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
    assert!(
        !doc.items[starter_idx].sim_active,
        "Motor de partida deve estar desativado com a chave de ignição aberta"
    );

    // Aciona a chave de ignição (partida)
    doc.items[switch_idx].sim_manual_latched = true;
    step_simulation(&mut doc, 0);

    // O motor de partida deve ser energizado pela bateria via comutador
    assert!(
        doc.items[starter_idx].sim_active,
        "Motor de partida deve estar ativo quando a chave de ignição for ligada!"
    );
    assert_eq!(
        doc.items[starter_idx].sim_motor_dir, 1,
        "Motor de partida deve indicar rotação ativa (dir = 1)"
    );

    // Ao desligar a chave de ignição, o motor de partida desliga
    doc.items[switch_idx].sim_manual_latched = false;
    step_simulation(&mut doc, 0);
    assert!(
        !doc.items[starter_idx].sim_active,
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

    step_simulation(&mut doc_horn, 0);
    assert!(
        !doc_horn.items[horn_idx].sim_active,
        "Buzina desligada em repouso"
    );

    doc_horn.items[horn_sw_idx].sim_manual_latched = true;
    step_simulation(&mut doc_horn, 0);
    assert!(
        doc_horn.items[horn_idx].sim_active,
        "Buzina deve tocar ao acionar o botão horn_switch!"
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

    let canvas_light = SchematicCanvas {
        doc: &app.doc,
        tool: app.tool,
        selected_id: None,
        sim_running: false,
        show_grid: true,
        grid_density: app.grid_density,
        show_texts: true,
        is_dark: false,
        auto_connect: true,
        auto_disconnect: true,
        zoom: 1.0,
        pan: iced::Vector::default(),
        placement_orientation: 0,
        dragging_symbol: None,
        is_blocked: false,
    };

    let canvas_dark = SchematicCanvas {
        doc: &app.doc,
        tool: app.tool,
        selected_id: None,
        sim_running: false,
        show_grid: true,
        grid_density: app.grid_density,
        show_texts: true,
        is_dark: true,
        auto_connect: true,
        auto_disconnect: true,
        zoom: 1.0,
        pan: iced::Vector::default(),
        placement_orientation: 0,
        dragging_symbol: None,
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
    assert!(!km.sim_active, "AND (0,0): Bobina deve estar DESLIGADA");
    assert_eq!(
        doc.items
            .iter()
            .find(|i| i.id == w_s1_s2)
            .unwrap()
            .sim_potential,
        Potential::None
    );
    assert_eq!(
        doc.items
            .iter()
            .find(|i| i.id == w_s2_km)
            .unwrap()
            .sim_potential,
        Potential::None
    );

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
        !km.sim_active,
        "AND (1,0): Bobina deve estar DESLIGADA se S2 estiver aberto"
    );
    assert_eq!(
        doc.items
            .iter()
            .find(|i| i.id == w_s1_s2)
            .unwrap()
            .sim_potential,
        Potential::L1
    );
    assert_eq!(
        doc.items
            .iter()
            .find(|i| i.id == w_s2_km)
            .unwrap()
            .sim_potential,
        Potential::None
    );

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
        !km.sim_active,
        "AND (0,1): Bobina deve estar DESLIGADA se S1 estiver aberto"
    );
    assert_eq!(
        doc.items
            .iter()
            .find(|i| i.id == w_s1_s2)
            .unwrap()
            .sim_potential,
        Potential::None
    );
    assert_eq!(
        doc.items
            .iter()
            .find(|i| i.id == w_s2_km)
            .unwrap()
            .sim_potential,
        Potential::None
    );

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
        km.sim_active,
        "AND (1,1): Bobina DEVE LIGAR quando ambos estiverem fechados"
    );
    assert_eq!(
        doc.items
            .iter()
            .find(|i| i.id == w_s1_s2)
            .unwrap()
            .sim_potential,
        Potential::L1
    );
    assert_eq!(
        doc.items
            .iter()
            .find(|i| i.id == w_s2_km)
            .unwrap()
            .sim_potential,
        Potential::L1
    );

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
        !m1.sim_active,
        "Potência: Motor desligado quando Q1 e KM2 abertos"
    );

    // Q1 fechado, KM2 aberto -> Eletricidade chega até a entrada de KM2, mas NÃO passa para o motor!
    set_manual_by_tag(&mut doc_pwr, "-Q1", true);
    step_simulation(&mut doc_pwr, 50);
    let m1 = doc_pwr.items.iter().find(|i| i.tipo == 1000).unwrap();
    assert!(
        !m1.sim_active,
        "Potência: Motor desligado se contator KM2 estiver aberto"
    );
    for &w in &wires_mid {
        assert!(doc_pwr
            .items
            .iter()
            .find(|i| i.id == w)
            .unwrap()
            .sim_potential
            .is_energized());
    }
    for &w in &wires_bot {
        assert_eq!(
            doc_pwr
                .items
                .iter()
                .find(|i| i.id == w)
                .unwrap()
                .sim_potential,
            Potential::None
        );
    }

    // Q1 aberto, KM2 fechado -> Eletricidade NÃO passa por Q1, motor parado!
    set_manual_by_tag(&mut doc_pwr, "-Q1", false);
    set_manual_by_tag(&mut doc_pwr, "-SKM2", true);
    step_simulation(&mut doc_pwr, 50);
    let m1 = doc_pwr.items.iter().find(|i| i.tipo == 1000).unwrap();
    assert!(
        !m1.sim_active,
        "Potência: Motor desligado se disjuntor Q1 estiver aberto"
    );

    // Q1 fechado E KM2 fechado -> Eletricidade passa por ambos e motor gira!
    set_manual_by_tag(&mut doc_pwr, "-Q1", true);
    set_manual_by_tag(&mut doc_pwr, "-SKM2", true);
    step_simulation(&mut doc_pwr, 50);
    let m1 = doc_pwr.items.iter().find(|i| i.tipo == 1000).unwrap();
    assert!(
        m1.sim_active,
        "Potência: Motor DEVE LIGAR quando Q1 E KM2 estiverem fechados"
    );
    assert_eq!(m1.sim_motor_dir, 1);
    for &w in &wires_bot {
        assert!(doc_pwr
            .items
            .iter()
            .find(|i| i.id == w)
            .unwrap()
            .sim_potential
            .is_energized());
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
    assert!(!km.sim_active, "OR (0,0): Bobina deve estar DESLIGADA");
    assert_eq!(
        doc.items
            .iter()
            .find(|i| i.id == w_out)
            .unwrap()
            .sim_potential,
        Potential::None
    );

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
        km.sim_active,
        "OR (1,0): Bobina DEVE LIGAR pelo ramo A fechado"
    );
    assert_eq!(
        doc.items
            .iter()
            .find(|i| i.id == w_out)
            .unwrap()
            .sim_potential,
        Potential::L1
    );

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
        km.sim_active,
        "OR (0,1): Bobina DEVE LIGAR pelo ramo B fechado"
    );
    assert_eq!(
        doc.items
            .iter()
            .find(|i| i.id == w_out)
            .unwrap()
            .sim_potential,
        Potential::L1
    );

    // Caso 4: (S1=1, S2=1) -> Ambos fechados
    set_manual_by_tag(&mut doc, "-S1", true);
    set_manual_by_tag(&mut doc, "-S2", true);
    step_simulation(&mut doc, 50);
    let km = doc
        .items
        .iter()
        .find(|i| i.tag == "-KM1" && i.tipo == 9000)
        .unwrap();
    assert!(km.sim_active, "OR (1,1): Bobina DEVE LIGAR");
    assert_eq!(
        doc.items
            .iter()
            .find(|i| i.id == w_out)
            .unwrap()
            .sim_potential,
        Potential::L1
    );
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
        let y_active = doc
            .items
            .iter()
            .find(|i| i.tag == "-Y" && i.tipo == 9000)
            .unwrap()
            .sim_active;
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
        doc.items[0].sim_manual_latched,
        "Botão momentâneo deve ficar ativo ao pressionar"
    );

    // Solta o botão momentâneo -> volta a repouso
    release_component_on_sim(&mut doc, 1);
    assert!(
        !doc.items[0].sim_manual_latched,
        "Botão momentâneo deve soltar ao liberar o mouse"
    );

    // Pressiona a chave interruptor (8008) -> retém ativo
    press_component_on_sim(&mut doc, 2);
    assert!(
        doc.items[1].sim_manual_latched,
        "Chave interruptor deve reter ativo"
    );

    // Solta a chave interruptor -> DEVE CONTINUAR ATIVO (retentivo/bi-estável)
    release_component_on_sim(&mut doc, 2);
    assert!(
        doc.items[1].sim_manual_latched,
        "Chave interruptor deve permanecer retida após soltar mouse"
    );

    // Segundo clique na chave interruptor -> desativa (alterna)
    press_component_on_sim(&mut doc, 2);
    assert!(
        !doc.items[1].sim_manual_latched,
        "Segundo clique desativa chave interruptor"
    );

    // Pressiona o botão de emergência retentivo (8030)
    press_component_on_sim(&mut doc, 3);
    assert!(
        doc.items[2].sim_manual_latched,
        "Botão de emergência deve travar ao pressionar"
    );
    release_component_on_sim(&mut doc, 3);
    assert!(
        doc.items[2].sim_manual_latched,
        "Botão de emergência retém travado ao soltar mouse"
    );
    press_component_on_sim(&mut doc, 3);
    assert!(
        !doc.items[2].sim_manual_latched,
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

    let canvas = SchematicCanvas {
        doc: &doc,
        tool: cade_simu_rs::EditorTool::Select,
        selected_id: None,
        sim_running: false,
        show_grid: true,
        grid_density: cade_simu_rs::model::GridDensity::Medium,
        show_texts: true,
        is_dark: false,
        auto_connect: true,
        auto_disconnect: false,
        zoom: 1.0,
        pan: iced::Vector::ZERO,
        placement_orientation: 0,
        dragging_symbol: None,
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

    let pot_out_left = doc.items.iter().find(|i| i.id == 6).unwrap().sim_potential;
    let pot_out_right = doc.items.iter().find(|i| i.id == 7).unwrap().sim_potential;
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
    doc.items
        .iter_mut()
        .find(|i| i.id == 3)
        .unwrap()
        .sim_manual_latched = true;
    step_simulation(&mut doc, 50);

    let pot_out_left_inv = doc.items.iter().find(|i| i.id == 6).unwrap().sim_potential;
    let pot_out_right_inv = doc.items.iter().find(|i| i.id == 7).unwrap().sim_potential;
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
    assert_eq!(
        doc.items.iter().find(|i| i.id == 6).unwrap().sim_potential,
        Potential::None
    );
    assert_eq!(
        doc.items.iter().find(|i| i.id == 7).unwrap().sim_potential,
        Potential::L1
    );

    // Ativa ambos os fins de curso: NO fecha (saída L1), NC abre (saída None)
    doc.items
        .iter_mut()
        .find(|i| i.id == 2)
        .unwrap()
        .sim_manual_latched = true;
    doc.items
        .iter_mut()
        .find(|i| i.id == 3)
        .unwrap()
        .sim_manual_latched = true;
    step_simulation(&mut doc, 50);
    assert_eq!(
        doc.items.iter().find(|i| i.id == 6).unwrap().sim_potential,
        Potential::L1
    );
    assert_eq!(
        doc.items.iter().find(|i| i.id == 7).unwrap().sim_potential,
        Potential::None
    );

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
    assert_eq!(
        doc.items.iter().find(|i| i.id == 14).unwrap().sim_potential,
        Potential::L1
    );
    assert_eq!(
        doc.items.iter().find(|i| i.id == 15).unwrap().sim_potential,
        Potential::None
    );

    // Dispara o relé térmico principal -F1 -> o contato auxiliar 8018 comuta automaticamente via tag!
    doc.items
        .iter_mut()
        .find(|i| i.id == 10)
        .unwrap()
        .sim_manual_latched = true;
    step_simulation(&mut doc, 50);
    assert_eq!(
        doc.items.iter().find(|i| i.id == 14).unwrap().sim_potential,
        Potential::None,
        "95-96 abre quando -F1 desarma"
    );
    assert_eq!(
        doc.items.iter().find(|i| i.id == 15).unwrap().sim_potential,
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

    let canvas_blocked = cade_simu_rs::canvas_view::SchematicCanvas {
        doc: &app.doc,
        tool: app.tool,
        selected_id: None,
        sim_running: false,
        show_grid: true,
        grid_density: app.grid_density,
        show_texts: true,
        is_dark: true,
        auto_connect: true,
        auto_disconnect: true,
        zoom: 1.0,
        pan: iced::Vector::ZERO,
        placement_orientation: 0,
        dragging_symbol: None,
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
    q1.sim_manual_latched = false; // Começa aberto
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
    let q1_item = doc.items.iter().find(|i| i.id == 2).unwrap();
    assert!(!q1_item.sim_manual_latched);
    assert!(!q1_item.sim_fault);

    // Fecha o disjuntor -Q1 -> a corrente de curto entre L1 e L2 faz o disjuntor DESARMAR (trip magnético)!
    doc.items
        .iter_mut()
        .find(|i| i.id == 2)
        .unwrap()
        .sim_manual_latched = true;
    step_simulation(&mut doc, 50);

    let q1_after = doc.items.iter().find(|i| i.id == 2).unwrap();
    assert!(
        !q1_after.sim_manual_latched,
        "O disjuntor -Q1 deve desarmar (abrir) por curto-circuito!"
    );
    assert!(
        q1_after.sim_fault,
        "O disjuntor -Q1 deve registrar estado de trip/falha!"
    );
    assert!(
        !q1_after.sim_active,
        "O disjuntor desarmado não deve conduzir!"
    );

    // Como o disjuntor desarmou, as saídas são desenergizadas, extinguindo o curto
    let sc_wire_pot = doc
        .items
        .iter()
        .find(|i| i.id == sc_wire_id)
        .unwrap()
        .sim_potential;
    assert_ne!(
        sc_wire_pot,
        Potential::ShortCircuit,
        "A abertura do disjuntor deve extinguir o curto a jusante"
    );

    // 2. Testa rearme do disjuntor após remoção do curto
    doc.items.retain(|i| i.id != sc_wire_id);
    toggle_component_on_click(&mut doc, 2);
    let q1_rearmed = doc.items.iter().find(|i| i.id == 2).unwrap();
    assert!(q1_rearmed.sim_manual_latched);
    assert!(
        !q1_rearmed.sim_fault,
        "O rearme do disjuntor deve limpar o flag de trip"
    );

    step_simulation(&mut doc, 50);
    let q1_holding = doc.items.iter().find(|i| i.id == 2).unwrap();
    assert!(
        q1_holding.sim_manual_latched,
        "Sem curto-circuito, o disjuntor deve permanecer fechado"
    );
    assert!(!q1_holding.sim_fault);

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
    q2.sim_manual_latched = true; // Fecha o disjuntor

    let wp_l1 = sym_l1.world_pins(&l1);
    let wp_n = sym_n.world_pins(&n);
    let wp_q2 = sym_q1p.world_pins(&q2);

    doc2.items.push(l1);
    doc2.items.push(n);
    doc2.items.push(q2);

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
    let q2_tripped = doc2.items.iter().find(|i| i.id == 12).unwrap();
    assert!(
        !q2_tripped.sim_manual_latched,
        "Disjuntor 1P deve desarmar por curto fase-neutro!"
    );
    assert!(q2_tripped.sim_fault);

    // 4. Testa Fusível Seccionador 5008 (-F1) desarmando em curto-circuito
    let mut doc3 = SchematicDocument::default();
    doc3.items.clear();
    let sym_f3p = find_symbol(5008).unwrap();
    let src3 = sym_src3.instantiate(20, 10, 10, 0);
    let wp_src3 = sym_src3.world_pins(&src3);
    let mut f1 = sym_f3p.instantiate(21, 10, 30, 0);
    f1.tag = "-F1".to_string();
    f1.sim_manual_latched = true; // Fecha a seccionadora fusível
    let wp_f1 = sym_f3p.world_pins(&f1);

    doc3.items.push(src3);
    doc3.items.push(f1);

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
    let f1_blown = doc3.items.iter().find(|i| i.id == 21).unwrap();
    assert!(
        !f1_blown.sim_manual_latched,
        "Fusível seccionador deve desarmar (abrir) por curto-circuito!"
    );
    assert!(
        f1_blown.sim_fault,
        "Fusível queimado deve registrar estado de falha"
    );
    assert!(!f1_blown.sim_active);

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
    let f2_blown = doc4.items.iter().find(|i| i.id == 31).unwrap();
    assert!(
        f2_blown.sim_manual_latched,
        "Fusível fixo deve queimar (sim_manual_latched: true)!"
    );
    assert!(f2_blown.sim_fault);
    assert!(!f2_blown.sim_active);
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
    // .cad agora é formato de exportação de folha única
    assert_eq!(ExportFormat::ALL.len(), 4);
    assert_eq!(
        ExportFormat::ALL,
        [
            ExportFormat::Cad,
            ExportFormat::Pdf,
            ExportFormat::Svg,
            ExportFormat::Png
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
    let export_cad_path = temp_dir.join("test_export_sheet1.cad");

    let export_res = cade_simu_rs::io::save_document(&export_cad_path, &test_doc);
    assert!(
        export_res.is_ok(),
        "Exportação para .cad deve ser bem sucedida"
    );

    // Valida que o arquivo gerado pode ser lido de volta perfeitamente
    let raw_cad = std::fs::read_to_string(&export_cad_path).expect("Leitura do .cad exportado");
    let parsed_doc = parse_cad_str(&raw_cad).expect("Parser do .cad exportado");
    assert_eq!(parsed_doc.items.len(), 2);
    assert_eq!(parsed_doc.title_block.title, "Motor Trifásico Folha 1");

    // Limpeza
    let _ = std::fs::remove_file(export_cad_path);
}
