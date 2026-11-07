use crate::model::{transform_offset, SchematicItem};

/// Categorias da paleta de componentes do CADe_SIMU 4.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolCategory {
    PowerSupply,
    Fuses,
    BreakersRelays,
    ContactorsSwitches,
    Motors,
    AuxiliaryContacts,
    PushbuttonsActuators,
    CoilsTimersLamps,
    WiresConnections,
}

impl SymbolCategory {
    pub const ALL: [SymbolCategory; 9] = [
        SymbolCategory::PowerSupply,
        SymbolCategory::Fuses,
        SymbolCategory::BreakersRelays,
        SymbolCategory::ContactorsSwitches,
        SymbolCategory::Motors,
        SymbolCategory::AuxiliaryContacts,
        SymbolCategory::PushbuttonsActuators,
        SymbolCategory::CoilsTimersLamps,
        SymbolCategory::WiresConnections,
    ];

    pub fn title(self) -> &'static str {
        match self {
            SymbolCategory::PowerSupply => "Alimentações",
            SymbolCategory::Fuses => "Fusíveis",
            SymbolCategory::BreakersRelays => "Disjuntores/Térmicos",
            SymbolCategory::ContactorsSwitches => "Contatores/Chaves",
            SymbolCategory::Motors => "Motores",
            SymbolCategory::AuxiliaryContacts => "Contatos NA/NC/Temp",
            SymbolCategory::PushbuttonsActuators => "Botões/Acionamentos",
            SymbolCategory::CoilsTimersLamps => "Bobinas/Sinalização",
            SymbolCategory::WiresConnections => "Cabos/Conexões",
        }
    }
}

/// Primitivas vetoriais extraídas do motor `Dibujo.cpp` (`0x008462f0`) de `CADe_SIMU.exe`.
#[derive(Debug, Clone)]
pub enum VectorPrimitive {
    Line {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
    },
    Ellipse {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
    },
    Arc {
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        x3: i32,
        y3: i32,
        x4: i32,
        y4: i32,
    },
    Polygon {
        pts: [(i32, i32); 4],
    },
    Text {
        x: i32,
        y: i32,
        text: String,
    },
}

/// Terminal de conexão elétrica de um símbolo, com coordenada local exata e coordenada do texto.
#[derive(Debug, Clone)]
pub struct SymbolPin {
    pub index: usize,
    pub default_label: &'static str,
    /// Coordenada local `(dx, dy)` de conexão do cabo (fórmula exata de `0x0084c0e9` em `CADe_SIMU.exe`).
    pub conn_pos: (i32, i32),
    /// Coordenada local `(lx, ly)` onde o número/nome do borne é desenhado (`NUME,+lx,+ly`).
    pub label_pos: (i32, i32),
}

/// Definição de um símbolo do catálogo CADe_SIMU.
#[derive(Debug, Clone)]
pub struct SymbolDef {
    pub tipo: i32,
    pub category: SymbolCategory,
    pub name: &'static str,
    pub short_label: &'static str,
    pub default_tag: &'static str,
    pub default_value: &'static str,
    pub pins: Vec<SymbolPin>,
    pub idle_cmds: &'static [&'static str],
    pub active_cmds: &'static [&'static str],
    /// Caixa delimitadora local `(min_x, min_y, max_x, max_y)` para seleção/clique.
    pub bounds: (i32, i32, i32, i32),
}

impl SymbolDef {
    /// Instancia um `SchematicItem` nas coordenadas `(x, y)` com os valores padrão de `CADe_SIMU.exe`.
    pub fn instantiate(&self, id: i32, x: i32, y: i32, orientation: i32) -> SchematicItem {
        let mut pins: [String; 9] = Default::default();
        for (idx, p) in self.pins.iter().enumerate().take(9) {
            pins[idx] = p.default_label.to_string();
        }
        SchematicItem {
            id,
            tipo: self.tipo,
            tag: self.default_tag.to_string(),
            value: self.default_value.to_string(),
            pins,
            pin_visibility: [1; 9],
            x1: x,
            y1: y,
            x2: x,
            y2: y,
            aux_coords: [
                self.bounds.0 / 5,
                self.bounds.1 / 5,
                self.bounds.2 / 5,
                self.bounds.3 / 5,
            ],
            color: if self.tipo == 9008 { 1 } else { 0 },
            orientation,
            subvariant: 0,
            show_tag: if self.default_tag.is_empty() { 0 } else { 1 },
            show_value: if self.default_value.is_empty() { 0 } else { 1 },
            extra_int1: 0,
            extra_int2: 0,
            trailing_str: String::new(),
            qet_type: None,
            qet_uuid: None,
        }
    }

    /// Retorna as coordenadas no mundo `(wx, wy)` de cada terminal após aplicar a orientação `0..=7`.
    pub fn world_pins(&self, item: &SchematicItem) -> Vec<(usize, String, (i32, i32))> {
        self.pins
            .iter()
            .enumerate()
            .map(|(idx, p)| {
                let (rx, ry) = transform_offset(p.conn_pos.0, p.conn_pos.1, item.orientation);
                let label = if idx < item.pins.len() && !item.pins[idx].is_empty() {
                    item.pins[idx].clone()
                } else {
                    p.default_label.to_string()
                };
                (idx, label, (item.x1 + rx, item.y1 + ry))
            })
            .collect()
    }

    /// Caminhos para os arquivos SVG vetoriais (estado de repouso e estado animado/acionado).
    pub fn svg_paths(&self) -> Option<(&'static str, &'static str)> {
        match self.tipo {
            // Botões Pulsadores QElectroTech integrados
            8000 => Some((
                "symbols_qet_all/electric/10_allpole/380_signaling_operating/20_push_buttons/push-button_3.svg",
                "symbols_qet_all/electric/10_allpole/380_signaling_operating/20_push_buttons/push-button_3_active.svg",
            )),
            8001 => Some((
                "symbols_qet_all/electric/10_allpole/380_signaling_operating/20_push_buttons/push-button__nc.svg",
                "symbols_qet_all/electric/10_allpole/380_signaling_operating/20_push_buttons/push-button__nc_active.svg",
            )),
            // Catálogo completo de componentes convertidos em SVG animado
            1000 => Some(("symbols_qet_all/cade_native/1000_motor_assíncrono_trifásico.svg", "symbols_qet_all/cade_native/1000_motor_assíncrono_trifásico_active.svg")),
            1001 => Some(("symbols_qet_all/cade_native/1001_motor_trifásico_6_terminais_estrela-triângulo.svg", "symbols_qet_all/cade_native/1001_motor_trifásico_6_terminais_estrela-triângulo_active.svg")),
            1002 => Some(("symbols_qet_all/cade_native/1002_motor_monofásico.svg", "symbols_qet_all/cade_native/1002_motor_monofásico_active.svg")),
            2000 => Some(("symbols_qet_all/cade_native/2000_contator_de_potência_2p.svg", "symbols_qet_all/cade_native/2000_contator_de_potência_2p_active.svg")),
            2001 => Some(("symbols_qet_all/cade_native/2001_contator_trifásico_3p.svg", "symbols_qet_all/cade_native/2001_contator_trifásico_3p_active.svg")),
            2004 => Some(("symbols_qet_all/cade_native/2004_chave_seccionadora_tripolar_3p.svg", "symbols_qet_all/cade_native/2004_chave_seccionadora_tripolar_3p_active.svg")),
            2006 => Some(("symbols_qet_all/cade_native/2006_contator_de_potência_1p.svg", "symbols_qet_all/cade_native/2006_contator_de_potência_1p_active.svg")),
            2008 => Some(("symbols_qet_all/cade_native/2008_contator_disjuntor_trifásico_3p.svg", "symbols_qet_all/cade_native/2008_contator_disjuntor_trifásico_3p_active.svg")),
            3000 => Some(("symbols_qet_all/cade_native/3000_alimentação_fase_l.svg", "symbols_qet_all/cade_native/3000_alimentação_fase_l_active.svg")),
            3001 => Some(("symbols_qet_all/cade_native/3001_alimentação_neutro_n.svg", "symbols_qet_all/cade_native/3001_alimentação_neutro_n_active.svg")),
            3002 => Some(("symbols_qet_all/cade_native/3002_proteção_terra_pe.svg", "symbols_qet_all/cade_native/3002_proteção_terra_pe_active.svg")),
            3003 => Some(("symbols_qet_all/cade_native/3003_alimentação_l___n.svg", "symbols_qet_all/cade_native/3003_alimentação_l___n_active.svg")),
            3004 => Some(("symbols_qet_all/cade_native/3004_alimentação_trifásica_l1_l2_l3.svg", "symbols_qet_all/cade_native/3004_alimentação_trifásica_l1_l2_l3_active.svg")),
            3005 => Some(("symbols_qet_all/cade_native/3005_alimentação_trifásica___pe_l1_l2_l3_pe.svg", "symbols_qet_all/cade_native/3005_alimentação_trifásica___pe_l1_l2_l3_pe_active.svg")),
            3008 => Some(("symbols_qet_all/cade_native/3008_alimentação_cc_positivo__.svg", "symbols_qet_all/cade_native/3008_alimentação_cc_positivo___active.svg")),
            3009 => Some(("symbols_qet_all/cade_native/3009_alimentação_cc_negativo_-.svg", "symbols_qet_all/cade_native/3009_alimentação_cc_negativo_-_active.svg")),
            5000 => Some(("symbols_qet_all/cade_native/5000_fusível_1p.svg", "symbols_qet_all/cade_native/5000_fusível_1p_active.svg")),
            5003 => Some(("symbols_qet_all/cade_native/5003_fusível_tripolar_3p.svg", "symbols_qet_all/cade_native/5003_fusível_tripolar_3p_active.svg")),
            5008 => Some(("symbols_qet_all/cade_native/5008_fusível_seccionador_3p.svg", "symbols_qet_all/cade_native/5008_fusível_seccionador_3p_active.svg")),
            6000 => Some(("symbols_qet_all/cade_native/6000_disjuntor_termomagnético_1p.svg", "symbols_qet_all/cade_native/6000_disjuntor_termomagnético_1p_active.svg")),
            // 6003: Disjuntor Termomagnético 3P (QElectroTech IEC - magneto-thermal_circuit_breaker_gv_4)
            6003 => Some((
                "symbols_qet_all/electric/10_allpole/200_fuses_protective_gears/12_magneto_thermal_circuit_breakers/magneto-thermal_circuit_breaker_gv_4.svg",
                "symbols_qet_all/electric/10_allpole/200_fuses_protective_gears/12_magneto_thermal_circuit_breakers/magneto-thermal_circuit_breaker_gv_4_active.svg",
            )),
            // 6004: Disjuntor Termomagnético 4P / Tetrapolar (QElectroTech IEC - magneto-thermal_circuit_breaker)
            6004 => Some((
                "symbols_qet_all/electric/10_allpole/200_fuses_protective_gears/12_magneto_thermal_circuit_breakers/magneto-thermal_circuit_breaker.svg",
                "symbols_qet_all/electric/10_allpole/200_fuses_protective_gears/12_magneto_thermal_circuit_breakers/magneto-thermal_circuit_breaker_active.svg",
            )),
            // 6007: Relé Térmico de Sobrecarga 3P (Símbolo Oficial QElectroTech IEC - thermal_relay_4 com linha pontilhada)
            6007 => Some((
                "symbols_qet_all/electric/10_allpole/200_fuses_protective_gears/30_thermal_relays/thermal_relay_4.svg",
                "symbols_qet_all/electric/10_allpole/200_fuses_protective_gears/30_thermal_relays/thermal_relay_4_active.svg",
            )),
            // 6008: Disjuntor Termomagnético 2P (QElectroTech IEC - magneto-thermal_circuit_breaker_gv_2p)
            6008 => Some((
                "symbols_qet_all/electric/10_allpole/200_fuses_protective_gears/12_magneto_thermal_circuit_breakers/magneto-thermal_circuit_breaker_gv_2p.svg",
                "symbols_qet_all/electric/10_allpole/200_fuses_protective_gears/12_magneto_thermal_circuit_breakers/magneto-thermal_circuit_breaker_gv_2p_active.svg",
            )),
            // 6009: Disjuntor-Motor / Guardamotor 3P (QElectroTech GV4)
            6009 => Some((
                "symbols_qet_all/electric/10_allpole/200_fuses_protective_gears/12_magneto_thermal_circuit_breakers/magneto-thermal_circuit_breaker_gv_4.svg",
                "symbols_qet_all/electric/10_allpole/200_fuses_protective_gears/12_magneto_thermal_circuit_breakers/magneto-thermal_circuit_breaker_gv_4_active.svg",
            )),
            7000 => Some(("symbols_qet_all/cade_native/7000_contato_auxiliar_na_no.svg", "symbols_qet_all/cade_native/7000_contato_auxiliar_na_no_active.svg")),
            7001 => Some(("symbols_qet_all/cade_native/7001_contato_auxiliar_nf_nc.svg", "symbols_qet_all/cade_native/7001_contato_auxiliar_nf_nc_active.svg")),
            7002 => Some(("symbols_qet_all/cade_native/7002_contato_duplo_na___nf.svg", "symbols_qet_all/cade_native/7002_contato_duplo_na___nf_active.svg")),
            7004 => Some(("symbols_qet_all/cade_native/7004_contato_temporizado_na_on-delay.svg", "symbols_qet_all/cade_native/7004_contato_temporizado_na_on-delay_active.svg")),
            7005 => Some(("symbols_qet_all/cade_native/7005_contato_temporizado_nf_on-delay.svg", "symbols_qet_all/cade_native/7005_contato_temporizado_nf_on-delay_active.svg")),
            7006 => Some(("symbols_qet_all/cade_native/7006_contato_temporizado_comutador_na_nf_on-delay.svg", "symbols_qet_all/cade_native/7006_contato_temporizado_comutador_na_nf_on-delay_active.svg")),
            7008 => Some(("symbols_qet_all/cade_native/7008_contato_temporizado_na_off-delay.svg", "symbols_qet_all/cade_native/7008_contato_temporizado_na_off-delay_active.svg")),
            7009 => Some(("symbols_qet_all/cade_native/7009_contato_temporizado_nf_off-delay.svg", "symbols_qet_all/cade_native/7009_contato_temporizado_nf_off-delay_active.svg")),
            8002 => Some(("symbols_qet_all/cade_native/8002_botão_pulsador_duplo_na_nf_no-nc.svg", "symbols_qet_all/cade_native/8002_botão_pulsador_duplo_na_nf_no-nc_active.svg")),
            8003 => Some(("symbols_qet_all/cade_native/8003_botão_pulsador_comutador_con.svg", "symbols_qet_all/cade_native/8003_botão_pulsador_comutador_con_active.svg")),
            // 8008: Chave Seletora / Interruptor NA (exato elemento DJ do CADe_SIMU)
            8008 => Some(("symbols_qet_all/cade_native/8004_chave_comutadora_-_retentiva_na.svg", "symbols_qet_all/cade_native/8004_chave_comutadora_-_retentiva_na_active.svg")),
            // 8012 e 8013: Sensor Fim de Curso NA e NF (QElectroTech IEC)
            8012 => Some((
                "symbols_qet_all/electric/10_allpole/390_sensors_instruments/41_limit_switches/limitswitch__no.svg",
                "symbols_qet_all/electric/10_allpole/390_sensors_instruments/41_limit_switches/limitswitch__no_2.svg",
            )),
            8013 => Some((
                "symbols_qet_all/electric/10_allpole/390_sensors_instruments/41_limit_switches/limitswitch__nc.svg",
                "symbols_qet_all/electric/10_allpole/390_sensors_instruments/41_limit_switches/limitswitch__nc_2.svg",
            )),
            // 8030, 8033, 8034: Botões de Parada de Emergência Retentivos com Trava Mecânica / Giro / Chave
            8030 => Some((
                "symbols_qet_all/electric/10_allpole/380_signaling_operating/20_push_buttons/emergency_stop__latch.svg",
                "symbols_qet_all/electric/10_allpole/380_signaling_operating/20_push_buttons/emergency_stop__nc.svg",
            )),
            8033 => Some((
                "symbols_qet_all/electric/10_allpole/380_signaling_operating/20_push_buttons/emergency_turn_stop_deverouiller.svg",
                "symbols_qet_all/electric/10_allpole/380_signaling_operating/20_push_buttons/emergency_turn_stop_deverouiller.svg",
            )),
            8034 => Some((
                "symbols_qet_all/electric/10_allpole/380_signaling_operating/20_push_buttons/emergency_stop_key_release.svg",
                "symbols_qet_all/electric/10_allpole/380_signaling_operating/20_push_buttons/emergency_stop_key_release.svg",
            )),
            // 8016 e 8017: Contatos Auxiliares do Relé Térmico (Símbolos Oficiais QElectroTech IEC)
            8016 => Some((
                "symbols_qet_all/electric/10_allpole/310_relays_contactors_contacts/02_contacts_cross_referencing/15_protection_contacts/contact_thermal_relay__no.svg",
                "symbols_qet_all/electric/10_allpole/310_relays_contactors_contacts/02_contacts_cross_referencing/15_protection_contacts/contact_thermal_relay__no_active.svg",
            )),
            8017 => Some((
                "symbols_qet_all/electric/10_allpole/310_relays_contactors_contacts/02_contacts_cross_referencing/15_protection_contacts/contact_thermal_relay__nc.svg",
                "symbols_qet_all/electric/10_allpole/310_relays_contactors_contacts/02_contacts_cross_referencing/15_protection_contacts/contact_thermal_relay__nc_active.svg",
            )),
            8023 => Some(("symbols_qet_all/cade_native/8023_botão_pulsador_2d_painel.svg", "symbols_qet_all/cade_native/8023_botão_pulsador_2d_painel_active.svg")),
            9000 => Some(("symbols_qet_all/cade_native/9000_bobina_de_contator_-_relé.svg", "symbols_qet_all/cade_native/9000_bobina_de_contator_-_relé_active.svg")),
            9004 => Some(("symbols_qet_all/cade_native/9004_temporizador_ao_trabalho_on-delay.svg", "symbols_qet_all/cade_native/9004_temporizador_ao_trabalho_on-delay_active.svg")),
            9005 => Some(("symbols_qet_all/cade_native/9005_temporizador_à_desconexão_off-delay.svg", "symbols_qet_all/cade_native/9005_temporizador_à_desconexão_off-delay_active.svg")),
            9008 => Some(("symbols_qet_all/cade_native/9008_sinaleiro_luminoso_piloto.svg", "symbols_qet_all/cade_native/9008_sinaleiro_luminoso_piloto_active.svg")),
            9013 => Some(("symbols_qet_all/cade_native/9013_sirene_-_alarme_sonoro_buzzer.svg", "symbols_qet_all/cade_native/9013_sirene_-_alarme_sonoro_buzzer_active.svg")),
            _ => None,
        }
    }
}

/// Faz o parse de uma string de comando vetorial ASCII de `.rdata` do `CADe_SIMU.exe`
/// (`LINE,+000,+000,+000,+015`, `ELIP,-015,+015,+075,+105`, `POLI,...`, `ARCO,...`, `TEXT,...`).
pub fn parse_vector_cmd(cmd: &str) -> Option<VectorPrimitive> {
    let parts: Vec<&str> = cmd.split(',').collect();
    let kind = *parts.first()?;
    let parse_i32 = |s: &str| -> Option<i32> { s.trim().parse::<i32>().ok() };

    match kind {
        "LINE" if parts.len() >= 5 => Some(VectorPrimitive::Line {
            x1: parse_i32(parts[1])?,
            y1: parse_i32(parts[2])?,
            x2: parse_i32(parts[3])?,
            y2: parse_i32(parts[4])?,
        }),
        "ELIP" if parts.len() >= 5 => Some(VectorPrimitive::Ellipse {
            x1: parse_i32(parts[1])?,
            y1: parse_i32(parts[2])?,
            x2: parse_i32(parts[3])?,
            y2: parse_i32(parts[4])?,
        }),
        "ARCO" | "CUER" if parts.len() >= 9 => Some(VectorPrimitive::Arc {
            x1: parse_i32(parts[1])?,
            y1: parse_i32(parts[2])?,
            x2: parse_i32(parts[3])?,
            y2: parse_i32(parts[4])?,
            x3: parse_i32(parts[5])?,
            y3: parse_i32(parts[6])?,
            x4: parse_i32(parts[7])?,
            y4: parse_i32(parts[8])?,
        }),
        "POLI" if parts.len() >= 9 => Some(VectorPrimitive::Polygon {
            pts: [
                (parse_i32(parts[1])?, parse_i32(parts[2])?),
                (parse_i32(parts[3])?, parse_i32(parts[4])?),
                (parse_i32(parts[5])?, parse_i32(parts[6])?),
                (parse_i32(parts[7])?, parse_i32(parts[8])?),
            ],
        }),
        "TEXT" if parts.len() >= 4 => {
            let x = parse_i32(parts[1])?;
            let y = parse_i32(parts[2])?;
            let text = parts[3].trim().to_string();
            if text.contains('%') {
                None
            } else {
                Some(VectorPrimitive::Text { x, y, text })
            }
        }
        _ => None,
    }
}

/// Constrói um `SymbolPin` usando a fórmula exata de `NUME,+nx,+ny` em `0x0084c0e9` de `CADe_SIMU.exe`
/// convertida pela constante `ds:0xb4d278 = 5.0` para a escala `.cad`:
/// `conn_x = (nx - 5) / 5`, `conn_y = if ny <= 0 { ny / 5 } else { (ny + 15) / 5 }`.
fn nume_pin(index: usize, default_label: &'static str, nx: i32, ny: i32) -> SymbolPin {
    let conn_x = (nx - 5) / 5;
    let conn_y = if ny <= 0 { ny / 5 } else { (ny + 15) / 5 };
    SymbolPin {
        index,
        default_label,
        conn_pos: (conn_x, conn_y),
        label_pos: (nx / 5, ny / 5),
    }
}

/// Constrói um pino de alimentação (`tipo 3000..3011` tratado em `0x0084d52a`) na escala `.cad` (`1/5`).
fn supply_pin(index: usize, default_label: &'static str, conn_x: i32, conn_y: i32) -> SymbolPin {
    SymbolPin {
        index,
        default_label,
        conn_pos: (conn_x / 5, conn_y / 5),
        label_pos: ((conn_x - 6) / 5, (conn_y - 32) / 5),
    }
}

use std::sync::LazyLock;

static CATALOG: LazyLock<Vec<SymbolDef>> = LazyLock::new(build_symbol_catalog);
static CATALOG_INDEX: LazyLock<std::collections::HashMap<i32, usize>> = LazyLock::new(|| {
    CATALOG
        .iter()
        .enumerate()
        .map(|(idx, s)| (s.tipo, idx))
        .collect()
});

/// Retorna a fatia estática do catálogo completo de símbolos extraídos de `CADe_SIMU.exe`.
pub fn symbol_catalog() -> &'static [SymbolDef] {
    &CATALOG
}

/// Constrói o catálogo completo de símbolos na primeira inicialização.
fn build_symbol_catalog() -> Vec<SymbolDef> {
    vec![
        // ======================== 1. ALIMENTAÇÕES (3000..3009) ========================
        SymbolDef {
            tipo: 3000,
            category: SymbolCategory::PowerSupply,
            name: "Alimentação Fase (L)",
            short_label: "Fase L",
            default_tag: "-X",
            default_value: "",
            pins: vec![supply_pin(0, "L", 0, 0)],
            idle_cmds: &["ELIP,-005,-020,+005,-010", "LINE,+000,-010,+000,+000"],
            active_cmds: &["ELIP,-005,-020,+005,-010", "LINE,+000,-010,+000,+000"],
            bounds: (-10, -35, 15, 5),
        },
        SymbolDef {
            tipo: 3001,
            category: SymbolCategory::PowerSupply,
            name: "Alimentação Neutro (N)",
            short_label: "Neutro N",
            default_tag: "-X",
            default_value: "",
            pins: vec![supply_pin(0, "N", 0, 0)],
            idle_cmds: &["ELIP,-005,-020,+005,-010", "LINE,+000,-010,+000,+000"],
            active_cmds: &["ELIP,-005,-020,+005,-010", "LINE,+000,-010,+000,+000"],
            bounds: (-10, -35, 15, 5),
        },
        SymbolDef {
            tipo: 3002,
            category: SymbolCategory::PowerSupply,
            name: "Proteção Terra (PE)",
            short_label: "Terra PE",
            default_tag: "-X",
            default_value: "",
            pins: vec![supply_pin(0, "PE", 0, 0)],
            idle_cmds: &[
                "LINE,-010,-020,+010,-020",
                "LINE,-007,-017,+007,-017",
                "LINE,-004,-014,+004,-014",
                "LINE,+000,-020,+000,+000",
            ],
            active_cmds: &[
                "LINE,-010,-020,+010,-020",
                "LINE,-007,-017,+007,-017",
                "LINE,-004,-014,+004,-014",
                "LINE,+000,-020,+000,+000",
            ],
            bounds: (-12, -35, 15, 5),
        },
        SymbolDef {
            tipo: 3003,
            category: SymbolCategory::PowerSupply,
            name: "Alimentação L + N",
            short_label: "L + N",
            default_tag: "-X",
            default_value: "",
            pins: vec![supply_pin(0, "L", 0, 0), supply_pin(1, "N", 30, 0)],
            idle_cmds: &[
                "ELIP,-005,-020,+005,-010",
                "LINE,+000,-010,+000,+000",
                "ELIP,+025,-020,+035,-010",
                "LINE,+030,-010,+030,+000",
            ],
            active_cmds: &[
                "ELIP,-005,-020,+005,-010",
                "LINE,+000,-010,+000,+000",
                "ELIP,+025,-020,+035,-010",
                "LINE,+030,-010,+030,+000",
            ],
            bounds: (-10, -35, 45, 5),
        },
        SymbolDef {
            tipo: 3004,
            category: SymbolCategory::PowerSupply,
            name: "Alimentação Trifásica (L1+L2+L3)",
            short_label: "L1 L2 L3",
            default_tag: "-X",
            default_value: "",
            pins: vec![
                supply_pin(0, "L1", 0, 0),
                supply_pin(1, "L2", 30, 0),
                supply_pin(2, "L3", 60, 0),
            ],
            idle_cmds: &[
                "ELIP,-005,-020,+005,-010",
                "LINE,+000,-010,+000,+000",
                "ELIP,+025,-020,+035,-010",
                "LINE,+030,-010,+030,+000",
                "ELIP,+055,-020,+065,-010",
                "LINE,+060,-010,+060,+000",
            ],
            active_cmds: &[
                "ELIP,-005,-020,+005,-010",
                "LINE,+000,-010,+000,+000",
                "ELIP,+025,-020,+035,-010",
                "LINE,+030,-010,+030,+000",
                "ELIP,+055,-020,+065,-010",
                "LINE,+060,-010,+060,+000",
            ],
            bounds: (-10, -35, 75, 5),
        },
        SymbolDef {
            tipo: 3005,
            category: SymbolCategory::PowerSupply,
            name: "Alimentação Trifásica + PE (L1+L2+L3+PE)",
            short_label: "3F + PE",
            default_tag: "-X",
            default_value: "",
            pins: vec![
                supply_pin(0, "L1", 0, 0),
                supply_pin(1, "L2", 30, 0),
                supply_pin(2, "L3", 60, 0),
                supply_pin(3, "PE", 90, 0),
            ],
            idle_cmds: &[
                "ELIP,-005,-020,+005,-010",
                "LINE,+000,-010,+000,+000",
                "ELIP,+025,-020,+035,-010",
                "LINE,+030,-010,+030,+000",
                "ELIP,+055,-020,+065,-010",
                "LINE,+060,-010,+060,+000",
                "LINE,+080,-020,+100,-020",
                "LINE,+083,-017,+097,-017",
                "LINE,+086,-014,+094,-014",
                "LINE,+090,-020,+090,+000",
            ],
            active_cmds: &[
                "ELIP,-005,-020,+005,-010",
                "LINE,+000,-010,+000,+000",
                "ELIP,+025,-020,+035,-010",
                "LINE,+030,-010,+030,+000",
                "ELIP,+055,-020,+065,-010",
                "LINE,+060,-010,+060,+000",
                "LINE,+080,-020,+100,-020",
                "LINE,+083,-017,+097,-017",
                "LINE,+086,-014,+094,-014",
                "LINE,+090,-020,+090,+000",
            ],
            bounds: (-10, -35, 105, 5),
        },
        SymbolDef {
            tipo: 3008,
            category: SymbolCategory::PowerSupply,
            name: "Alimentação CC Positivo (+)",
            short_label: "DC (+)",
            default_tag: "-X",
            default_value: "",
            pins: vec![supply_pin(0, "+", 0, 0)],
            idle_cmds: &["ELIP,-005,-020,+005,-010", "LINE,+000,-010,+000,+000"],
            active_cmds: &["ELIP,-005,-020,+005,-010", "LINE,+000,-010,+000,+000"],
            bounds: (-10, -35, 15, 5),
        },
        SymbolDef {
            tipo: 3009,
            category: SymbolCategory::PowerSupply,
            name: "Alimentação CC Negativo (-)",
            short_label: "DC (-)",
            default_tag: "-X",
            default_value: "",
            pins: vec![supply_pin(0, "-", 0, 0)],
            idle_cmds: &["ELIP,-005,-020,+005,-010", "LINE,+000,-010,+000,+000"],
            active_cmds: &["ELIP,-005,-020,+005,-010", "LINE,+000,-010,+000,+000"],
            bounds: (-10, -35, 15, 5),
        },
        // ======================== 2. FUSÍVEIS (5000..5008) ========================
        SymbolDef {
            tipo: 5000,
            category: SymbolCategory::Fuses,
            name: "Fusível 1P",
            short_label: "Fusível 1P",
            default_tag: "-F",
            default_value: "",
            pins: vec![nume_pin(0, "1", 5, 0), nume_pin(1, "2", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+060",
                "LINE,-007,+015,+007,+015",
                "LINE,-007,+045,+007,+045",
                "LINE,-007,+015,-007,+045",
                "LINE,+007,+015,+007,+045",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+025",
                "LINE,+000,+035,+000,+060",
                "LINE,-007,+015,+007,+015",
                "LINE,-007,+045,+007,+045",
                "LINE,-007,+015,-007,+045",
                "LINE,+007,+015,+007,+045",
            ],
            bounds: (-12, 0, 15, 60),
        },
        SymbolDef {
            tipo: 5003,
            category: SymbolCategory::Fuses,
            name: "Fusível Tripolar 3P",
            short_label: "Fusível 3P",
            default_tag: "-F",
            default_value: "",
            pins: vec![
                nume_pin(0, "1", 5, 0),
                nume_pin(1, "3", 35, 0),
                nume_pin(2, "5", 65, 0),
                nume_pin(3, "2", 5, 45),
                nume_pin(4, "4", 35, 45),
                nume_pin(5, "6", 65, 45),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+060",
                "LINE,-007,+015,+007,+015",
                "LINE,-007,+045,+007,+045",
                "LINE,-007,+015,-007,+045",
                "LINE,+007,+015,+007,+045",
                "LINE,+030,+000,+030,+060",
                "LINE,+023,+015,+037,+015",
                "LINE,+023,+045,+037,+045",
                "LINE,+023,+015,+023,+045",
                "LINE,+037,+015,+037,+045",
                "LINE,+060,+000,+060,+060",
                "LINE,+053,+015,+067,+015",
                "LINE,+053,+045,+067,+045",
                "LINE,+053,+015,+053,+045",
                "LINE,+067,+015,+067,+045",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+060",
                "LINE,-007,+015,+007,+015",
                "LINE,-007,+045,+007,+045",
                "LINE,-007,+015,-007,+045",
                "LINE,+007,+015,+007,+045",
                "LINE,+030,+000,+030,+060",
                "LINE,+023,+015,+037,+015",
                "LINE,+023,+045,+037,+045",
                "LINE,+023,+015,+023,+045",
                "LINE,+037,+015,+037,+045",
                "LINE,+060,+000,+060,+060",
                "LINE,+053,+015,+067,+015",
                "LINE,+053,+045,+067,+045",
                "LINE,+053,+015,+053,+045",
                "LINE,+067,+015,+067,+045",
            ],
            bounds: (-12, 0, 75, 60),
        },
        SymbolDef {
            tipo: 5008,
            category: SymbolCategory::Fuses,
            name: "Fusível Seccionador 3P",
            short_label: "Secc. Fus 3P",
            default_tag: "-F",
            default_value: "",
            pins: vec![
                nume_pin(0, "1", 5, 0),
                nume_pin(1, "3", 35, 0),
                nume_pin(2, "5", 65, 0),
                nume_pin(3, "2", 5, 45),
                nume_pin(4, "4", 35, 45),
                nume_pin(5, "6", 65, 45),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,+060,+000,+060,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+060,+045,+060,+060",
                "LINE,-015,+015,+000,+045",
                "LINE,+015,+015,+030,+045",
                "LINE,+045,+015,+060,+045",
                "POLI,-016,+020,-004,+014,+004,+032,-008,+038",
                "POLI,+014,+020,+026,+014,+034,+032,+022,+038",
                "POLI,+044,+020,+056,+014,+064,+032,+052,+038",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+060",
                "LINE,+030,+000,+030,+060",
                "LINE,+060,+000,+060,+060",
                "POLI,-007,+018,+007,+018,+007,+042,-007,+042",
                "POLI,+023,+018,+037,+018,+037,+042,+023,+042",
                "POLI,+053,+018,+067,+018,+067,+042,+053,+042",
            ],
            bounds: (-18, 0, 75, 60),
        },
        // ======================== 3. DISJUNTORES E RELÉS TÉRMICOS (6000..6009) ========================
        // 6000: Disjuntor Termomagnético 1P
        SymbolDef {
            tipo: 6000,
            category: SymbolCategory::BreakersRelays,
            name: "Disjuntor Termomagnético 1P",
            short_label: "Disjuntor 1P",
            default_tag: "-Q",
            default_value: "",
            pins: vec![nume_pin(0, "1", 5, 0), nume_pin(1, "2", 5, 60)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,-015,+015,+000,+045",
                "LINE,+000,+045,+000,+060",
                "POLI,-020,+024,-012,+020,-008,+028,-016,+032",
                "LINE,-007,+030,+003,+030",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,-005,+015,+000,+045",
                "LINE,+000,+045,+000,+060",
                "LINE,-005,+015,+000,+015",
                "POLI,-010,+024,-002,+020,+002,+028,-006,+032",
                "LINE,+003,+030,+013,+030",
            ],
            bounds: (-22, 0, 15, 60),
        },
        // 6008: Disjuntor Termomagnético 2P (com 2 pontos - QElectroTech GV 2P)
        SymbolDef {
            tipo: 6008,
            category: SymbolCategory::BreakersRelays,
            name: "Disjuntor Termomagnético 2P",
            short_label: "Disjuntor 2P",
            default_tag: "-Q",
            default_value: "",
            pins: vec![
                nume_pin(0, "1", 5, 0),
                nume_pin(1, "3", 35, 0),
                nume_pin(2, "2", 5, 90),
                nume_pin(3, "4", 35, 90),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,-015,+015,+000,+045",
                "LINE,+015,+015,+030,+045",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "POLI,-020,+024,-012,+020,-008,+028,-016,+032",
                "POLI,+010,+024,+018,+020,+022,+028,+014,+032",
                "LINE,-007,+030,+003,+030",
                "LINE,+018,+030,+028,+030",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,-005,+015,+000,+045",
                "LINE,+025,+015,+030,+045",
                "LINE,-005,+015,+000,+015",
                "LINE,+025,+015,+030,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+003,+030,+013,+030",
                "LINE,+028,+030,+038,+030",
                "POLI,-010,+024,-002,+020,+002,+028,-006,+032",
                "POLI,+020,+024,+028,+020,+032,+028,+024,+032",
            ],
            bounds: (-20, 0, 45, 60),
        },
        // 6003: Disjuntor Termomagnético 3P (com 3 pontos - QElectroTech GV 4)
        SymbolDef {
            tipo: 6003,
            category: SymbolCategory::BreakersRelays,
            name: "Disjuntor Termomagnético 3P",
            short_label: "Disjuntor 3P",
            default_tag: "-Q",
            default_value: "",
            pins: vec![
                nume_pin(0, "1", 5, 0),
                nume_pin(1, "3", 35, 0),
                nume_pin(2, "5", 65, 0),
                nume_pin(3, "2", 5, 90),
                nume_pin(4, "4", 35, 90),
                nume_pin(5, "6", 65, 90),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-015,+015,+000,+045",
                "POLI,-020,+024,-012,+020,-008,+028,-016,+032",
                "LINE,+030,+000,+030,+015",
                "LINE,+030,+045,+030,+060",
                "LINE,+015,+015,+030,+045",
                "POLI,+010,+024,+018,+020,+022,+028,+014,+032",
                "LINE,+060,+000,+060,+015",
                "LINE,+060,+045,+060,+060",
                "LINE,+045,+015,+060,+045",
                "LINE,-007,+030,+003,+030",
                "LINE,+018,+030,+028,+030",
                "LINE,+042,+030,+052,+030",
                "POLI,+040,+024,+048,+020,+052,+028,+044,+032",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,+060,+000,+060,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+060,+045,+060,+060",
                "LINE,-005,+015,+000,+045",
                "LINE,+025,+015,+030,+045",
                "LINE,+055,+015,+060,+045",
                "LINE,-005,+015,+000,+015",
                "LINE,+025,+015,+030,+015",
                "LINE,+055,+015,+060,+015",
                "LINE,+003,+030,+013,+030",
                "LINE,+028,+030,+038,+030",
                "LINE,+052,+030,+062,+030",
                "POLI,-010,+024,-002,+020,+002,+028,-006,+032",
                "POLI,+020,+024,+028,+020,+032,+028,+024,+032",
                "POLI,+050,+024,+058,+020,+062,+028,+054,+032",
            ],
            bounds: (-22, 0, 75, 105),
        },
        // 6004: Disjuntor Termomagnético 4P / Tetrapolar (com 4 pontos - QElectroTech)
        SymbolDef {
            tipo: 6004,
            category: SymbolCategory::BreakersRelays,
            name: "Disjuntor Termomagnético 4P",
            short_label: "Disjuntor 4P",
            default_tag: "-Q",
            default_value: "",
            pins: vec![
                nume_pin(0, "N", 5, 0),
                nume_pin(1, "1", 35, 0),
                nume_pin(2, "3", 65, 0),
                nume_pin(3, "5", 95, 0),
                nume_pin(4, "N", 5, 90),
                nume_pin(5, "2", 35, 90),
                nume_pin(6, "4", 65, 90),
                nume_pin(7, "6", 95, 90),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-015,+015,+000,+045",
                "LINE,+030,+000,+030,+015",
                "LINE,+030,+045,+030,+060",
                "LINE,+015,+015,+030,+045",
                "LINE,+060,+000,+060,+015",
                "LINE,+060,+045,+060,+060",
                "LINE,+045,+015,+060,+045",
                "LINE,+090,+000,+090,+015",
                "LINE,+090,+045,+090,+060",
                "LINE,+075,+015,+090,+045",
                "LINE,-007,+030,+003,+030",
                "LINE,+018,+030,+028,+030",
                "LINE,+042,+030,+052,+030",
                "LINE,+072,+030,+082,+030",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,+060,+000,+060,+015",
                "LINE,+090,+000,+090,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+060,+045,+060,+060",
                "LINE,+090,+045,+090,+060",
                "LINE,-005,+015,+000,+045",
                "LINE,+025,+015,+030,+045",
                "LINE,+055,+015,+060,+045",
                "LINE,+085,+015,+090,+045",
                "LINE,+003,+030,+013,+030",
                "LINE,+028,+030,+038,+030",
                "LINE,+052,+030,+062,+030",
                "LINE,+078,+030,+088,+030",
            ],
            bounds: (-22, 0, 105, 105),
        },
        // 6007: Relé Térmico de Sobrecarga 3P (thermal_relay_4)
        SymbolDef {
            tipo: 6007,
            category: SymbolCategory::BreakersRelays,
            name: "Relé Térmico de Sobrecarga 3P",
            short_label: "Relé Térmico",
            default_tag: "-F",
            default_value: "",
            pins: vec![
                nume_pin(0, "1", 5, 0),
                nume_pin(1, "3", 35, 0),
                nume_pin(2, "5", 65, 0),
                nume_pin(3, "2", 5, 45),
                nume_pin(4, "4", 35, 45),
                nume_pin(5, "6", 65, 45),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+020",
                "LINE,+000,+020,-010,+020",
                "LINE,-010,+020,-010,+035",
                "LINE,-010,+035,+000,+035",
                "LINE,+000,+035,+000,+060",
                "LINE,+030,+000,+030,+020",
                "LINE,+030,+020,+020,+020",
                "LINE,+020,+020,+020,+035",
                "LINE,+020,+035,+030,+035",
                "LINE,+030,+035,+030,+060",
                "LINE,+060,+000,+060,+020",
                "LINE,+060,+020,+050,+020",
                "LINE,+050,+020,+050,+035",
                "LINE,+050,+035,+060,+035",
                "LINE,+060,+035,+060,+060",
                "LINE,-030,+015,-015,+015",
                "LINE,-015,+015,-015,+045",
                "LINE,-015,+045,-030,+045",
                "LINE,-030,+045,-030,+015",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+020",
                "LINE,+000,+020,-010,+020",
                "LINE,-010,+020,-010,+035",
                "LINE,-010,+035,+000,+035",
                "LINE,+000,+035,+000,+060",
                "LINE,+030,+000,+030,+020",
                "LINE,+030,+020,+020,+020",
                "LINE,+020,+020,+020,+035",
                "LINE,+020,+035,+030,+035",
                "LINE,+030,+035,+030,+060",
                "LINE,+060,+000,+060,+020",
                "LINE,+060,+020,+050,+020",
                "LINE,+050,+020,+050,+035",
                "LINE,+050,+035,+060,+035",
                "LINE,+060,+035,+060,+060",
                "POLI,-030,+015,-015,+015,-015,+045,-030,+045",
            ],
            bounds: (-32, 0, 75, 60),
        },
        SymbolDef {
            tipo: 6009,
            category: SymbolCategory::BreakersRelays,
            name: "Disjuntor-Motor Guardamotor 3P",
            short_label: "Disj-Motor 3P",
            default_tag: "-Q",
            default_value: "",
            pins: vec![
                nume_pin(0, "1", 5, 0),
                nume_pin(1, "3", 35, 0),
                nume_pin(2, "5", 65, 0),
                nume_pin(3, "2", 5, 90),
                nume_pin(4, "4", 35, 90),
                nume_pin(5, "6", 65, 90),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,+060,+000,+060,+015",
                "LINE,-015,+015,+000,+045",
                "LINE,+015,+015,+030,+045",
                "LINE,+045,+015,+060,+045",
                "LINE,+000,+045,+000,+105",
                "LINE,+030,+045,+030,+105",
                "LINE,+060,+045,+060,+105",
                "POLI,-015,+045,+075,+045,+075,+090,-015,+090",
                "LINE,-015,+065,+075,+065",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,+060,+000,+060,+015",
                "LINE,-005,+015,+000,+045",
                "LINE,+025,+015,+030,+045",
                "LINE,+055,+015,+060,+045",
                "LINE,-005,+015,+000,+015",
                "LINE,+025,+015,+030,+015",
                "LINE,+055,+015,+060,+015",
                "LINE,+000,+045,+000,+105",
                "LINE,+030,+045,+030,+105",
                "LINE,+060,+045,+060,+105",
                "POLI,-015,+045,+075,+045,+075,+090,-015,+090",
                "LINE,-015,+065,+075,+065",
            ],
            bounds: (-20, 0, 80, 105),
        },
        // ======================== 4. CONTATORES E CHAVES (2000..2004) ========================
        SymbolDef {
            tipo: 2006,
            category: SymbolCategory::ContactorsSwitches,
            name: "Contator de Potência 1P",
            short_label: "Contator 1P",
            default_tag: "-KM",
            default_value: "",
            pins: vec![nume_pin(0, "1", 5, 0), nume_pin(1, "2", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-015,+015,+000,+045",
                "ARCO,-005,+010,+005,+020,-005,+015,+005,+015",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-005,+015,+000,+045",
                "LINE,-005,+015,+000,+015",
                "ARCO,-005,+010,+005,+020,-005,+015,+005,+015",
            ],
            bounds: (-18, 0, 15, 60),
        },
        SymbolDef {
            tipo: 2000,
            category: SymbolCategory::ContactorsSwitches,
            name: "Contator de Potência 2P",
            short_label: "Contator 2P",
            default_tag: "-KM",
            default_value: "",
            pins: vec![
                nume_pin(0, "1", 5, 0),
                nume_pin(1, "3", 35, 0),
                nume_pin(2, "2", 5, 45),
                nume_pin(3, "4", 35, 45),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,-015,+015,+000,+045",
                "LINE,+015,+015,+030,+045",
                "LINE,-007,+030,+003,+030",
                "LINE,+018,+030,+028,+030",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,-005,+015,+000,+045",
                "LINE,+025,+015,+030,+045",
                "LINE,-005,+015,+000,+015",
                "LINE,+025,+015,+030,+015",
                "LINE,+003,+030,+013,+030",
                "LINE,+028,+030,+038,+030",
            ],
            bounds: (-18, 0, 45, 60),
        },
        SymbolDef {
            tipo: 2001,
            category: SymbolCategory::ContactorsSwitches,
            name: "Contator Trifásico 3P",
            short_label: "Contator 3P",
            default_tag: "-KM",
            default_value: "",
            pins: vec![
                nume_pin(0, "1", 5, 0),
                nume_pin(1, "3", 35, 0),
                nume_pin(2, "5", 65, 0),
                nume_pin(3, "2", 5, 45),
                nume_pin(4, "4", 35, 45),
                nume_pin(5, "6", 65, 45),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,+060,+000,+060,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+060,+045,+060,+060",
                "LINE,-015,+015,+000,+045",
                "LINE,+015,+015,+030,+045",
                "LINE,+045,+015,+060,+045",
                "LINE,-007,+030,+003,+030",
                "LINE,+018,+030,+028,+030",
                "LINE,+042,+030,+052,+030",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,+060,+000,+060,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+060,+045,+060,+060",
                "LINE,-005,+015,+000,+045",
                "LINE,+025,+015,+030,+045",
                "LINE,+055,+015,+060,+045",
                "LINE,-005,+015,+000,+015",
                "LINE,+025,+015,+030,+015",
                "LINE,+055,+015,+060,+015",
                "LINE,+003,+030,+013,+030",
                "LINE,+028,+030,+038,+030",
                "LINE,+052,+030,+062,+030",
            ],
            bounds: (-18, 0, 75, 60),
        },
        SymbolDef {
            tipo: 2004,
            category: SymbolCategory::ContactorsSwitches,
            name: "Chave Seccionadora Tripolar 3P",
            short_label: "Chave 3P",
            default_tag: "-S",
            default_value: "",
            pins: vec![
                nume_pin(0, "1", 5, 0),
                nume_pin(1, "3", 35, 0),
                nume_pin(2, "5", 65, 0),
                nume_pin(3, "2", 5, 45),
                nume_pin(4, "4", 35, 45),
                nume_pin(5, "6", 65, 45),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,+060,+000,+060,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+060,+045,+060,+060",
                "LINE,-015,+015,+000,+045",
                "LINE,+015,+015,+030,+045",
                "LINE,+045,+015,+060,+045",
                "LINE,-025,+030,+052,+030",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,+060,+000,+060,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+060,+045,+060,+060",
                "LINE,-005,+015,+000,+045",
                "LINE,+025,+015,+030,+045",
                "LINE,+055,+015,+060,+045",
                "LINE,-005,+015,+000,+015",
                "LINE,+025,+015,+030,+015",
                "LINE,+055,+015,+060,+015",
                "LINE,-015,+030,+062,+030",
            ],
            bounds: (-28, 0, 75, 60),
        },
        // ======================== 5. MOTORES (1000..1006) ========================
        SymbolDef {
            tipo: 1000,
            category: SymbolCategory::Motors,
            name: "Motor Assíncrono Trifásico",
            short_label: "Motor 3~",
            default_tag: "-M",
            default_value: "",
            pins: vec![
                nume_pin(0, "U1", 5, 0),
                nume_pin(1, "V1", 35, 0),
                nume_pin(2, "W1", 65, 0),
                nume_pin(3, "PE", 95, 0),
            ],
            idle_cmds: &[
                "ELIP,-015,+015,+075,+105",
                "LINE,+000,+000,+000,+027",
                "LINE,+030,+000,+030,+015",
                "LINE,+060,+000,+060,+027",
                "LINE,+090,+000,+090,+060",
                "LINE,+090,+060,+075,+060",
                "TEXT,+026,+040,M    ,0",
                "TEXT,+012,+062,3~   ,0",
            ],
            active_cmds: &[
                "ELIP,-015,+015,+075,+105",
                "ARCO,+000,+030,+060,+090,+048,+048,+011,+048",
                "POLI,+055,+044,+045,+039,+051,+034,+055,+044",
                "LINE,+000,+000,+000,+027",
                "LINE,+030,+000,+030,+015",
                "LINE,+060,+000,+060,+027",
                "LINE,+090,+000,+090,+060",
                "LINE,+090,+060,+075,+060",
                "TEXT,+026,+040,M    ,0",
                "TEXT,+012,+062,3~   ,0",
            ],
            bounds: (-18, 0, 100, 108),
        },
        SymbolDef {
            tipo: 1001,
            category: SymbolCategory::Motors,
            name: "Motor Trifásico 6 Terminais (Estrela-Triângulo)",
            short_label: "Motor 3~ Y/Δ",
            default_tag: "-M",
            default_value: "",
            pins: vec![
                nume_pin(0, "U1", 5, 0),
                nume_pin(1, "V1", 35, 0),
                nume_pin(2, "W1", 65, 0),
                nume_pin(3, "PE", 95, 0),
                nume_pin(4, "W2", 5, 105),
                nume_pin(5, "U2", 35, 105),
                nume_pin(6, "V2", 65, 105),
            ],
            idle_cmds: &[
                "ELIP,-015,+015,+075,+105",
                "LINE,+000,+000,+000,+027",
                "LINE,+030,+000,+030,+015",
                "LINE,+060,+000,+060,+027",
                "LINE,+090,+000,+090,+060",
                "LINE,+090,+060,+080,+060",
                "LINE,+000,+093,+000,+120",
                "LINE,+030,+105,+030,+120",
                "LINE,+060,+093,+060,+120",
                "TEXT,+026,+040,M    ,0",
                "TEXT,+012,+062,3~   ,0",
            ],
            active_cmds: &[
                "ELIP,-015,+015,+075,+105",
                "ARCO,+000,+030,+060,+090,+048,+048,+011,+048",
                "POLI,+055,+044,+045,+039,+051,+034,+055,+044",
                "LINE,+000,+000,+000,+027",
                "LINE,+030,+000,+030,+015",
                "LINE,+060,+000,+060,+027",
                "LINE,+090,+000,+090,+060",
                "LINE,+090,+060,+080,+060",
                "LINE,+000,+093,+000,+120",
                "LINE,+030,+105,+030,+120",
                "LINE,+060,+093,+060,+120",
                "TEXT,+026,+040,M    ,0",
                "TEXT,+012,+062,3~   ,0",
            ],
            bounds: (-18, 0, 100, 125),
        },
        SymbolDef {
            tipo: 1002,
            category: SymbolCategory::Motors,
            name: "Motor Monofásico",
            short_label: "Motor 1~",
            default_tag: "-M",
            default_value: "",
            pins: vec![
                nume_pin(0, "U1", 5, 0),
                nume_pin(1, "V1", 65, 0),
                nume_pin(2, "PE", 95, 0),
            ],
            idle_cmds: &[
                "ELIP,-015,+015,+075,+105",
                "LINE,+000,+000,+000,+027",
                "LINE,+060,+000,+060,+027",
                "LINE,+090,+000,+090,+060",
                "LINE,+090,+060,+075,+060",
                "TEXT,+026,+040,M    ,0",
                "TEXT,+012,+062,1~   ,0",
            ],
            active_cmds: &[
                "ELIP,-015,+015,+075,+105",
                "ARCO,+000,+030,+060,+090,+048,+048,+011,+048",
                "POLI,+055,+044,+045,+039,+051,+034,+055,+044",
                "LINE,+000,+000,+000,+027",
                "LINE,+060,+000,+060,+027",
                "LINE,+090,+000,+090,+060",
                "LINE,+090,+060,+075,+060",
                "TEXT,+026,+040,M    ,0",
                "TEXT,+012,+062,1~   ,0",
            ],
            bounds: (-18, 0, 100, 108),
        },
        // ======================== 6. CONTATOS AUXILIARES E TEMPORIZADOS (7000..7009) ========================
        SymbolDef {
            tipo: 7000,
            category: SymbolCategory::AuxiliaryContacts,
            name: "Contato Auxiliar NA (NO)",
            short_label: "Contato NA",
            default_tag: "-KM",
            default_value: "",
            pins: vec![nume_pin(0, "13", 5, 0), nume_pin(1, "14", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-015,+015,+000,+045",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-005,+015,+000,+045",
                "LINE,-005,+015,+000,+015",
            ],
            bounds: (-18, 0, 15, 60),
        },
        SymbolDef {
            tipo: 7001,
            category: SymbolCategory::AuxiliaryContacts,
            name: "Contato Auxiliar NF (NC)",
            short_label: "Contato NF",
            default_tag: "-KM",
            default_value: "",
            pins: vec![nume_pin(0, "11", 5, 0), nume_pin(1, "12", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+015,+015,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+022,+018,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
            ],
            bounds: (-12, 0, 22, 60),
        },
        SymbolDef {
            tipo: 7002,
            category: SymbolCategory::AuxiliaryContacts,
            name: "Contato Duplo NA + NF",
            short_label: "Cont. NA+NF",
            default_tag: "-KM",
            default_value: "",
            pins: vec![
                nume_pin(0, "11", 5, 0),
                nume_pin(1, "13", 50, 0),
                nume_pin(2, "12", 5, 45),
                nume_pin(3, "14", 50, 45),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+015,+015,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+045,+000,+045,+015",
                "LINE,+045,+045,+045,+060",
                "LINE,+030,+015,+045,+045",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+022,+018,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+045,+000,+045,+015",
                "LINE,+045,+045,+045,+060",
                "LINE,+040,+015,+045,+045",
                "LINE,+040,+015,+045,+015",
            ],
            bounds: (-15, 0, 58, 60),
        },
        SymbolDef {
            tipo: 7004,
            category: SymbolCategory::AuxiliaryContacts,
            name: "Contato Temporizado NA (On-Delay)",
            short_label: "Temp NA (ON)",
            default_tag: "-KT",
            default_value: "",
            pins: vec![nume_pin(0, "67", 5, 0), nume_pin(1, "68", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-015,+015,+000,+045",
                "LINE,-020,+028,-006,+028",
                "LINE,-020,+032,-008,+032",
                "ARCO,-032,+020,-012,+040,-020,+020,-020,+040",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-005,+015,+000,+045",
                "LINE,-005,+015,+000,+015",
                "LINE,-020,+028,-002,+028",
                "LINE,-020,+032,-003,+032",
                "ARCO,-032,+020,-012,+040,-020,+020,-020,+040",
            ],
            bounds: (-32, 0, 15, 60),
        },
        SymbolDef {
            tipo: 7005,
            category: SymbolCategory::AuxiliaryContacts,
            name: "Contato Temporizado NF (On-Delay)",
            short_label: "Temp NF (ON)",
            default_tag: "-KT",
            default_value: "",
            pins: vec![nume_pin(0, "55", 5, 0), nume_pin(1, "56", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+015,+015,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,-020,+028,+008,+028",
                "LINE,-020,+032,+006,+032",
                "ARCO,-032,+020,-012,+040,-020,+020,-020,+040",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+022,+018,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,-020,+028,+012,+028",
                "LINE,-020,+032,+010,+032",
                "ARCO,-032,+020,-012,+040,-020,+020,-020,+040",
            ],
            bounds: (-32, 0, 22, 60),
        },
        SymbolDef {
            tipo: 7006,
            category: SymbolCategory::AuxiliaryContacts,
            name: "Contato Temporizado Comutador NA+NF (On-Delay)",
            short_label: "Temp NA+NF (ON)",
            default_tag: "-KT",
            default_value: "",
            pins: vec![
                nume_pin(0, "15", 5, 0),
                nume_pin(1, "17", 50, 0),
                nume_pin(2, "16", 5, 45),
                nume_pin(3, "18", 50, 45),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+015,+015,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,+045,+000,+045,+015",
                "LINE,+045,+045,+045,+060",
                "LINE,+030,+015,+045,+045",
                "ARCO,-032,+020,-012,+040,-020,+020,-020,+040",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+022,+018,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+045,+000,+045,+015",
                "LINE,+045,+045,+045,+060",
                "LINE,+040,+015,+045,+045",
                "LINE,+040,+015,+045,+015",
                "ARCO,-032,+020,-012,+040,-020,+020,-020,+040",
            ],
            bounds: (-32, 0, 58, 60),
        },
        SymbolDef {
            tipo: 7008,
            category: SymbolCategory::AuxiliaryContacts,
            name: "Contato Temporizado NA (Off-Delay)",
            short_label: "Temp NA (OFF)",
            default_tag: "-KT",
            default_value: "",
            pins: vec![nume_pin(0, "27", 5, 0), nume_pin(1, "28", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-015,+015,+000,+045",
                "LINE,-020,+028,-006,+028",
                "LINE,-020,+032,-008,+032",
                "ARCO,-028,+020,-008,+040,-020,+040,-020,+020",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-005,+015,+000,+045",
                "LINE,-005,+015,+000,+015",
                "LINE,-020,+028,-002,+028",
                "LINE,-020,+032,-003,+032",
                "ARCO,-028,+020,-008,+040,-020,+040,-020,+020",
            ],
            bounds: (-32, 0, 15, 60),
        },
        SymbolDef {
            tipo: 7009,
            category: SymbolCategory::AuxiliaryContacts,
            name: "Contato Temporizado NF (Off-Delay)",
            short_label: "Temp NF (OFF)",
            default_tag: "-KT",
            default_value: "",
            pins: vec![nume_pin(0, "15", 5, 0), nume_pin(1, "16", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+015,+015,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,-020,+028,+008,+028",
                "LINE,-020,+032,+006,+032",
                "ARCO,-028,+020,-008,+040,-020,+040,-020,+020",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+022,+018,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,-020,+028,+012,+028",
                "LINE,-020,+032,+010,+032",
                "ARCO,-028,+020,-008,+040,-020,+040,-020,+020",
            ],
            bounds: (-32, 0, 22, 60),
        },
        // ======================== 7. BOTÕES E ACIONAMENTOS (8000..8017) ========================
        SymbolDef {
            tipo: 8000,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Botão Pulsador NA (NO)",
            short_label: "Botão NA",
            default_tag: "-S",
            default_value: "",
            pins: vec![nume_pin(0, "13", 5, 0), nume_pin(1, "14", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-015,+015,+000,+045",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-007,+030",
                "LINE,-030,+023,-030,+037",
                "LINE,-030,+023,-023,+023",
                "LINE,-030,+037,-023,+037",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-005,+015,+000,+045",
                "LINE,-005,+015,+000,+015",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+001,+030",
                "LINE,-020,+023,-020,+037",
                "LINE,-020,+023,-013,+023",
                "LINE,-020,+037,-013,+037",
            ],
            bounds: (-32, 0, 15, 60),
        },
        SymbolDef {
            tipo: 8001,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Botão Pulsador NF (NC)",
            short_label: "Botão NF",
            default_tag: "-S",
            default_value: "",
            pins: vec![nume_pin(0, "11", 5, 0), nume_pin(1, "12", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+015,+015,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-005,+030",
                "LINE,+000,+030,+008,+030",
                "LINE,-030,+023,-030,+037",
                "LINE,-030,+023,-023,+023",
                "LINE,-030,+037,-023,+037",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+022,+018,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+005,+030",
                "LINE,-020,+023,-020,+037",
                "LINE,-020,+023,-013,+023",
                "LINE,-020,+037,-013,+037",
            ],
            bounds: (-32, 0, 22, 60),
        },
        SymbolDef {
            tipo: 8002,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Botão Pulsador Duplo NA+NF (NO-NC)",
            short_label: "Botão NA+NF",
            default_tag: "-S",
            default_value: "",
            pins: vec![
                nume_pin(0, "11", 5, 0),
                nume_pin(1, "13", 50, 0),
                nume_pin(2, "12", 5, 45),
                nume_pin(3, "14", 50, 45),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+045,+000,+045,+015",
                "LINE,+045,+045,+045,+060",
                "LINE,+015,+015,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,+030,+015,+045,+045",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-005,+030",
                "LINE,+000,+030,+010,+030",
                "LINE,+015,+030,+025,+030",
                "LINE,+030,+030,+038,+030",
                "LINE,-030,+023,-030,+037",
                "LINE,-030,+023,-023,+023",
                "LINE,-030,+037,-023,+037",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+045,+000,+045,+015",
                "LINE,+045,+045,+045,+060",
                "LINE,+022,+018,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,+040,+015,+045,+045",
                "LINE,+040,+015,+045,+015",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+005,+030",
                "LINE,+010,+030,+020,+030",
                "LINE,+025,+030,+035,+030",
                "LINE,+040,+030,+048,+030",
                "LINE,-020,+023,-020,+037",
                "LINE,-020,+023,-013,+023",
                "LINE,-020,+037,-013,+037",
            ],
            bounds: (-32, 0, 58, 60),
        },
        SymbolDef {
            tipo: 8003,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Botão Pulsador Comutador (CON)",
            short_label: "Botão CON",
            default_tag: "-S",
            default_value: "",
            pins: vec![
                nume_pin(0, "11", 20, 0),
                nume_pin(1, "12", 5, 45),
                nume_pin(2, "14", 35, 45),
            ],
            idle_cmds: &[
                "LINE,+015,+000,+015,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+015,+015,+003,+045",
                "LINE,+000,+042,+000,+045",
                "LINE,+000,+042,+007,+042",
                "LINE,+030,+042,+030,+045",
                "LINE,+030,+042,+023,+042",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-005,+030",
                "LINE,+000,+030,+010,+030",
                "LINE,-030,+023,-030,+037",
                "LINE,-030,+023,-023,+023",
                "LINE,-030,+037,-023,+037",
            ],
            active_cmds: &[
                "LINE,+015,+000,+015,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+015,+015,+027,+045",
                "LINE,+000,+042,+000,+045",
                "LINE,+000,+042,+007,+042",
                "LINE,+030,+042,+030,+045",
                "LINE,+030,+042,+023,+042",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+005,+030",
                "LINE,+010,+030,+020,+030",
                "LINE,-020,+023,-020,+037",
                "LINE,-020,+023,-013,+023",
                "LINE,-020,+037,-013,+037",
            ],
            bounds: (-32, 0, 42, 60),
        },
        SymbolDef {
            tipo: 8004,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Botão Cogumelo NA (Pulsador Seta NA)",
            short_label: "Cogumelo NA",
            default_tag: "-S",
            default_value: "",
            pins: vec![nume_pin(0, "13", 5, 0), nume_pin(1, "14", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-015,+015,+000,+045",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-007,+030",
                "ARCO,-030,+020,-010,+040,-020,+020,-020,+040",
                "LINE,-020,+020,-020,+040",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-005,+015,+000,+045",
                "LINE,-005,+015,+000,+015",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+001,+030",
                "ARCO,-020,+020,+000,+040,-010,+020,-010,+040",
                "LINE,-010,+020,-010,+040",
            ],
            bounds: (-32, 0, 15, 60),
        },
        SymbolDef {
            tipo: 8005,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Botão Cogumelo NF (Pulsador Seta NF)",
            short_label: "Cogumelo NF",
            default_tag: "-S",
            default_value: "",
            pins: vec![nume_pin(0, "11", 5, 0), nume_pin(1, "12", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+015,+015,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-005,+030",
                "LINE,+000,+030,+008,+030",
                "ARCO,-030,+020,-010,+040,-020,+020,-020,+040",
                "LINE,-020,+020,-020,+040",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+022,+018,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+005,+030",
                "ARCO,-020,+020,+000,+040,-010,+020,-010,+040",
                "LINE,-010,+020,-010,+040",
            ],
            bounds: (-32, 0, 22, 60),
        },
        SymbolDef {
            tipo: 8006,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Botão Cogumelo Duplo NA+NF (Pulsador Seta NA-NC)",
            short_label: "Cogumelo NA+NF",
            default_tag: "-S",
            default_value: "",
            pins: vec![
                nume_pin(0, "11", 5, 0),
                nume_pin(1, "13", 50, 0),
                nume_pin(2, "12", 5, 45),
                nume_pin(3, "14", 50, 45),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+045,+000,+045,+015",
                "LINE,+045,+045,+045,+060",
                "LINE,+015,+015,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,+030,+015,+045,+045",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-005,+030",
                "LINE,+000,+030,+010,+030",
                "LINE,+015,+030,+025,+030",
                "LINE,+030,+030,+038,+030",
                "ARCO,-030,+020,-010,+040,-020,+020,-020,+040",
                "LINE,-020,+020,-020,+040",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+045,+000,+045,+015",
                "LINE,+045,+045,+045,+060",
                "LINE,+022,+018,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,+040,+015,+045,+045",
                "LINE,+040,+015,+045,+015",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+005,+030",
                "LINE,+010,+030,+020,+030",
                "LINE,+025,+030,+035,+030",
                "LINE,+040,+030,+048,+030",
                "ARCO,-020,+020,+000,+040,-010,+020,-010,+040",
                "LINE,-010,+020,-010,+040",
            ],
            bounds: (-32, 0, 58, 60),
        },
        SymbolDef {
            tipo: 8007,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Botão Cogumelo Comutador (Pulsador Seta CON)",
            short_label: "Cogumelo CON",
            default_tag: "-S",
            default_value: "",
            pins: vec![
                nume_pin(0, "11", 20, 0),
                nume_pin(1, "12", 5, 45),
                nume_pin(2, "14", 35, 45),
            ],
            idle_cmds: &[
                "LINE,+015,+000,+015,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+015,+015,+003,+045",
                "LINE,+000,+042,+000,+045",
                "LINE,+000,+042,+007,+042",
                "LINE,+030,+042,+030,+045",
                "LINE,+030,+042,+023,+042",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-005,+030",
                "LINE,+000,+030,+010,+030",
                "ARCO,-030,+020,-010,+040,-020,+020,-020,+040",
                "LINE,-020,+020,-020,+040",
            ],
            active_cmds: &[
                "LINE,+015,+000,+015,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+015,+015,+027,+045",
                "LINE,+000,+042,+000,+045",
                "LINE,+000,+042,+007,+042",
                "LINE,+030,+042,+030,+045",
                "LINE,+030,+042,+023,+042",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+005,+030",
                "LINE,+010,+030,+020,+030",
                "ARCO,-020,+020,+000,+040,-010,+020,-010,+040",
                "LINE,-010,+020,-010,+040",
            ],
            bounds: (-32, 0, 42, 60),
        },
        SymbolDef {
            tipo: 8008,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Chave Seletora / Interruptor NA (NO Switch)",
            short_label: "Interruptor NA",
            default_tag: "-S",
            default_value: "",
            pins: vec![nume_pin(0, "13", 5, 0), nume_pin(1, "14", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-015,+015,+000,+045",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-007,+030",
                "LINE,-030,+020,-030,+040",
                "LINE,-030,+020,-037,+020",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-005,+015,+000,+045",
                "LINE,-005,+015,+000,+015",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+002,+030",
                "LINE,-020,+020,-020,+040",
                "LINE,-020,+020,-027,+020",
            ],
            bounds: (-38, 0, 15, 60),
        },
        SymbolDef {
            tipo: 8009,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Chave Seletora / Interruptor NF (NC Switch)",
            short_label: "Interruptor NF",
            default_tag: "-S",
            default_value: "",
            pins: vec![nume_pin(0, "11", 5, 0), nume_pin(1, "12", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+015,+015,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-005,+030",
                "LINE,+000,+030,+008,+030",
                "LINE,-030,+020,-030,+040",
                "LINE,-030,+020,-037,+020",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+022,+018,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+005,+030",
                "LINE,-020,+020,-020,+040",
                "LINE,-020,+020,-027,+020",
            ],
            bounds: (-38, 0, 22, 60),
        },
        SymbolDef {
            tipo: 8010,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Chave Seletora / Interruptor Duplo NA+NF (NO-NC Switch)",
            short_label: "Interruptor NA+NF",
            default_tag: "-S",
            default_value: "",
            pins: vec![
                nume_pin(0, "11", 5, 0),
                nume_pin(1, "13", 50, 0),
                nume_pin(2, "12", 5, 45),
                nume_pin(3, "14", 50, 45),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+045,+000,+045,+015",
                "LINE,+045,+045,+045,+060",
                "LINE,+015,+015,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,+030,+015,+045,+045",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-005,+030",
                "LINE,+000,+030,+010,+030",
                "LINE,+015,+030,+025,+030",
                "LINE,+030,+030,+038,+030",
                "LINE,-030,+020,-030,+040",
                "LINE,-030,+020,-037,+020",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+045,+000,+045,+015",
                "LINE,+045,+045,+045,+060",
                "LINE,+022,+018,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,+040,+015,+045,+045",
                "LINE,+040,+015,+045,+015",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+005,+030",
                "LINE,+010,+030,+020,+030",
                "LINE,+025,+030,+035,+030",
                "LINE,+040,+030,+048,+030",
                "LINE,-020,+020,-020,+040",
                "LINE,-020,+020,-027,+020",
            ],
            bounds: (-38, 0, 58, 60),
        },
        SymbolDef {
            tipo: 8011,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Chave Seletora / Interruptor Comutador (CON/COM Switch)",
            short_label: "Interruptor CON",
            default_tag: "-S",
            default_value: "",
            pins: vec![
                nume_pin(0, "11", 20, 0),
                nume_pin(1, "12", 5, 45),
                nume_pin(2, "14", 35, 45),
            ],
            idle_cmds: &[
                "LINE,+015,+000,+015,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+015,+015,+003,+045",
                "LINE,+000,+042,+000,+045",
                "LINE,+000,+042,+007,+042",
                "LINE,+030,+042,+030,+045",
                "LINE,+030,+042,+023,+042",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-005,+030",
                "LINE,+000,+030,+010,+030",
                "LINE,-030,+020,-030,+040",
                "LINE,-030,+020,-037,+020",
            ],
            active_cmds: &[
                "LINE,+015,+000,+015,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+015,+015,+027,+045",
                "LINE,+000,+042,+000,+045",
                "LINE,+000,+042,+007,+042",
                "LINE,+030,+042,+030,+045",
                "LINE,+030,+042,+023,+042",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+005,+030",
                "LINE,+010,+030,+020,+030",
                "LINE,-020,+020,-020,+040",
                "LINE,-020,+020,-027,+020",
            ],
            bounds: (-38, 0, 42, 60),
        },
        SymbolDef {
            tipo: 8012,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Sensor Fim de Curso NA (Limit Switch NO)",
            short_label: "Fim Curso NA",
            default_tag: "-S",
            default_value: "",
            pins: vec![nume_pin(0, "13", 5, 0), nume_pin(1, "14", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-015,+015,+000,+045",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-007,+030",
                "ARCO,-030,+020,-010,+040,-020,+040,-020,+020",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-005,+015,+000,+045",
                "LINE,-005,+015,+000,+015",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+001,+030",
                "ARCO,-020,+020,+000,+040,-010,+040,-010,+020",
            ],
            bounds: (-32, 0, 15, 60),
        },
        SymbolDef {
            tipo: 8013,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Sensor Fim de Curso NF (Limit Switch NC)",
            short_label: "Fim Curso NF",
            default_tag: "-S",
            default_value: "",
            pins: vec![nume_pin(0, "11", 5, 0), nume_pin(1, "12", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+015,+015,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-005,+030",
                "LINE,+000,+030,+008,+030",
                "ARCO,-030,+020,-010,+040,-020,+040,-020,+020",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+022,+018,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+005,+030",
                "ARCO,-020,+020,+000,+040,-010,+040,-010,+020",
            ],
            bounds: (-32, 0, 22, 60),
        },
        SymbolDef {
            tipo: 8014,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Sensor Fim de Curso Duplo NA+NF (Limit Switch NO-NC)",
            short_label: "Fim Curso NA+NF",
            default_tag: "-S",
            default_value: "",
            pins: vec![
                nume_pin(0, "11", 5, 0),
                nume_pin(1, "13", 50, 0),
                nume_pin(2, "12", 5, 45),
                nume_pin(3, "14", 50, 45),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+045,+000,+045,+015",
                "LINE,+045,+045,+045,+060",
                "LINE,+015,+015,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,+030,+015,+045,+045",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-005,+030",
                "LINE,+000,+030,+010,+030",
                "LINE,+015,+030,+025,+030",
                "LINE,+030,+030,+038,+030",
                "ARCO,-030,+020,-010,+040,-020,+040,-020,+020",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+045,+000,+045,+015",
                "LINE,+045,+045,+045,+060",
                "LINE,+022,+018,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,+040,+015,+045,+045",
                "LINE,+040,+015,+045,+015",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+005,+030",
                "LINE,+010,+030,+020,+030",
                "LINE,+025,+030,+035,+030",
                "LINE,+040,+030,+048,+030",
                "ARCO,-020,+020,+000,+040,-010,+040,-010,+020",
            ],
            bounds: (-32, 0, 58, 60),
        },
        SymbolDef {
            tipo: 8015,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Sensor Fim de Curso Comutador (Limit Switch CON/COM)",
            short_label: "Fim Curso CON",
            default_tag: "-S",
            default_value: "",
            pins: vec![
                nume_pin(0, "11", 20, 0),
                nume_pin(1, "12", 5, 45),
                nume_pin(2, "14", 35, 45),
            ],
            idle_cmds: &[
                "LINE,+015,+000,+015,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+015,+015,+003,+045",
                "LINE,+000,+042,+000,+045",
                "LINE,+000,+042,+007,+042",
                "LINE,+030,+042,+030,+045",
                "LINE,+030,+042,+023,+042",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-005,+030",
                "LINE,+000,+030,+010,+030",
                "ARCO,-030,+020,-010,+040,-020,+040,-020,+020",
            ],
            active_cmds: &[
                "LINE,+015,+000,+015,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+015,+015,+027,+045",
                "LINE,+000,+042,+000,+045",
                "LINE,+000,+042,+007,+042",
                "LINE,+030,+042,+030,+045",
                "LINE,+030,+042,+023,+042",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+005,+030",
                "LINE,+010,+030,+020,+030",
                "ARCO,-020,+020,+000,+040,-010,+040,-010,+020",
            ],
            bounds: (-32, 0, 42, 60),
        },
        SymbolDef {
            tipo: 8016,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Contato Auxiliar Relé Térmico NA (97-98)",
            short_label: "Térm. NA 97-98",
            default_tag: "-F",
            default_value: "",
            pins: vec![nume_pin(0, "97", 5, 0), nume_pin(1, "98", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-015,+015,+000,+045",
                "LINE,+019,+020,+019,+025",
                "LINE,+019,+025,+025,+025",
                "LINE,+025,+025,+025,+035",
                "LINE,+019,+035,+025,+035",
                "LINE,+019,+035,+019,+040",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-007,+030",
                "LINE,-030,+030,-030,+040",
                "LINE,-023,+030,-023,+040",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-005,+015,+000,+045",
                "LINE,-005,+015,+000,+015",
                "LINE,+019,+020,+019,+025",
                "LINE,+019,+025,+025,+025",
                "LINE,+025,+025,+025,+035",
                "LINE,+019,+035,+025,+035",
                "LINE,+019,+035,+019,+040",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+003,+030",
                "LINE,-020,+030,-020,+040",
                "LINE,-013,+030,-013,+040",
            ],
            bounds: (-32, 0, 28, 60),
        },
        SymbolDef {
            tipo: 8017,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Contato Auxiliar Relé Térmico NF (95-96)",
            short_label: "Térm. NF 95-96",
            default_tag: "-F",
            default_value: "",
            pins: vec![nume_pin(0, "95", 5, 0), nume_pin(1, "96", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+015,+015,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-005,+030",
                "LINE,+000,+030,+008,+030",
                "LINE,-030,+030,-030,+040",
                "LINE,-023,+030,-023,+040",
                "LINE,+019,+020,+019,+025",
                "LINE,+019,+025,+025,+025",
                "LINE,+025,+025,+025,+035",
                "LINE,+019,+035,+025,+035",
                "LINE,+019,+035,+019,+040",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+022,+018,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+005,+030",
                "LINE,-020,+030,-020,+040",
                "LINE,-013,+030,-013,+040",
                "LINE,+019,+020,+019,+025",
                "LINE,+019,+025,+025,+025",
                "LINE,+025,+025,+025,+035",
                "LINE,+019,+035,+025,+035",
                "LINE,+019,+035,+019,+040",
            ],
            bounds: (-32, 0, 28, 60),
        },
        SymbolDef {
            tipo: 8018,
            category: SymbolCategory::AuxiliaryContacts,
            name: "Contato Auxiliar Relé Térmico Duplo NA+NF (95-96 / 97-98)",
            short_label: "Térm. NA+NF",
            default_tag: "-F",
            default_value: "",
            pins: vec![
                nume_pin(0, "95", 5, 0),
                nume_pin(1, "97", 50, 0),
                nume_pin(2, "96", 5, 45),
                nume_pin(3, "98", 50, 45),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+045,+000,+045,+015",
                "LINE,+045,+045,+045,+060",
                "LINE,+015,+015,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,+030,+015,+045,+045",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-005,+030",
                "LINE,+000,+030,+010,+030",
                "LINE,+015,+030,+025,+030",
                "LINE,+030,+030,+038,+030",
                "LINE,-030,+030,-030,+040",
                "LINE,-023,+030,-023,+040",
                "LINE,+019,+020,+019,+025",
                "LINE,+019,+025,+025,+025",
                "LINE,+025,+025,+025,+035",
                "LINE,+019,+035,+025,+035",
                "LINE,+019,+035,+019,+040",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+045,+000,+045,+015",
                "LINE,+045,+045,+045,+060",
                "LINE,+022,+018,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,+040,+015,+045,+045",
                "LINE,+040,+015,+045,+015",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+005,+030",
                "LINE,+010,+030,+020,+030",
                "LINE,+025,+030,+035,+030",
                "LINE,+040,+030,+048,+030",
                "LINE,-020,+030,-020,+040",
                "LINE,-013,+030,-013,+040",
                "LINE,+019,+020,+019,+025",
                "LINE,+019,+025,+025,+025",
                "LINE,+025,+025,+025,+035",
                "LINE,+019,+035,+025,+035",
                "LINE,+019,+035,+019,+040",
            ],
            bounds: (-32, 0, 58, 60),
        },
        SymbolDef {
            tipo: 8019,
            category: SymbolCategory::AuxiliaryContacts,
            name: "Contato Auxiliar Relé Térmico Comutador (95-96-98)",
            short_label: "Térm. CON",
            default_tag: "-F",
            default_value: "",
            pins: vec![
                nume_pin(0, "95", 20, 0),
                nume_pin(1, "96", 5, 45),
                nume_pin(2, "98", 35, 45),
            ],
            idle_cmds: &[
                "LINE,+015,+000,+015,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+015,+015,+003,+045",
                "LINE,+000,+042,+000,+045",
                "LINE,+000,+042,+007,+042",
                "LINE,+030,+042,+030,+045",
                "LINE,+030,+042,+023,+042",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-005,+030",
                "LINE,+000,+030,+010,+030",
                "LINE,-030,+030,-030,+040",
                "LINE,-023,+030,-023,+040",
                "LINE,+019,+020,+019,+025",
                "LINE,+019,+025,+025,+025",
                "LINE,+025,+025,+025,+035",
                "LINE,+019,+035,+025,+035",
                "LINE,+019,+035,+019,+040",
            ],
            active_cmds: &[
                "LINE,+015,+000,+015,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+015,+015,+027,+045",
                "LINE,+000,+042,+000,+045",
                "LINE,+000,+042,+007,+042",
                "LINE,+030,+042,+030,+045",
                "LINE,+030,+042,+023,+042",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+005,+030",
                "LINE,+010,+030,+020,+030",
                "LINE,-020,+030,-020,+040",
                "LINE,-013,+030,-013,+040",
                "LINE,+019,+020,+019,+025",
                "LINE,+019,+025,+025,+025",
                "LINE,+025,+025,+025,+035",
                "LINE,+019,+035,+025,+035",
                "LINE,+019,+035,+019,+040",
            ],
            bounds: (-32, 0, 42, 60),
        },
        SymbolDef {
            tipo: 8020,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Chave Seletora 3 Posições (I-0-II)",
            short_label: "Seletora I-0-II",
            default_tag: "-S",
            default_value: "",
            pins: vec![
                nume_pin(0, "1", 5, 0),
                nume_pin(1, "3", 35, 0),
                nume_pin(2, "2", 5, 45),
                nume_pin(3, "4", 35, 45),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,-015,+015,+000,+045",
                "LINE,+015,+015,+030,+045",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-007,+030",
                "LINE,-030,+020,-030,+040",
                "LINE,-030,+020,-037,+020",
                "LINE,-030,+040,-037,+040",
                "TEXT,-045,+030,0",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,-005,+015,+000,+045",
                "LINE,-005,+015,+000,+015",
                "LINE,+015,+015,+030,+045",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+002,+030",
                "LINE,-020,+020,-020,+040",
                "LINE,-020,+020,-027,+020",
                "TEXT,-035,+030,I",
            ],
            bounds: (-48, 0, 45, 60),
        },
        SymbolDef {
            tipo: 8021,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Chave Seletora 3 Posições (I-II-III)",
            short_label: "Seletora I-II-III",
            default_tag: "-S",
            default_value: "",
            pins: vec![
                nume_pin(0, "1", 5, 0),
                nume_pin(1, "3", 35, 0),
                nume_pin(2, "5", 65, 0),
                nume_pin(3, "2", 5, 45),
                nume_pin(4, "4", 35, 45),
                nume_pin(5, "6", 65, 45),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,+060,+000,+060,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+060,+045,+060,+060",
                "LINE,-015,+015,+000,+045",
                "LINE,+015,+015,+030,+045",
                "LINE,+045,+015,+060,+045",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-007,+030",
                "LINE,-030,+020,-030,+040",
                "LINE,-030,+020,-037,+020",
                "TEXT,-045,+030,I",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,+060,+000,+060,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+060,+045,+060,+060",
                "LINE,-005,+015,+000,+045",
                "LINE,-005,+015,+000,+015",
                "LINE,+025,+015,+030,+045",
                "LINE,+025,+015,+030,+015",
                "LINE,+045,+015,+060,+045",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+002,+030",
                "LINE,-020,+020,-020,+040",
                "LINE,-020,+020,-027,+020",
                "TEXT,-035,+030,II",
            ],
            bounds: (-48, 0, 75, 60),
        },
        SymbolDef {
            tipo: 8022,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Chave Inversora / Cruzamento (Crossover Switch)",
            short_label: "Chave Crossover",
            default_tag: "-S",
            default_value: "",
            pins: vec![
                nume_pin(0, "1", 5, 0),
                nume_pin(1, "3", 35, 0),
                nume_pin(2, "2", 5, 45),
                nume_pin(3, "4", 35, 45),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+000,+015,+000,+045",
                "LINE,+030,+015,+030,+045",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-007,+030",
                "LINE,-030,+020,-030,+040",
                "LINE,-030,+020,-037,+020",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+000,+015,+030,+045",
                "LINE,+030,+015,+000,+045",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+002,+030",
                "LINE,-020,+020,-020,+040",
                "LINE,-020,+020,-027,+020",
            ],
            bounds: (-38, 0, 45, 60),
        },
        // ======================== 8. BOBINAS, TEMPORIZADORES E SINALIZAÇÃO (9000..9013) ========================
        SymbolDef {
            tipo: 9000,
            category: SymbolCategory::CoilsTimersLamps,
            name: "Bobina de Contator / Relé",
            short_label: "Bobina KM",
            default_tag: "-KM",
            default_value: "",
            pins: vec![nume_pin(0, "A1", 5, 0), nume_pin(1, "A2", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+020",
                "LINE,+000,+040,+000,+060",
                "LINE,-020,+020,+020,+020",
                "LINE,-020,+040,+020,+040",
                "LINE,-020,+020,-020,+040",
                "LINE,+020,+020,+020,+040",
            ],
            active_cmds: &[
                "POLI,-020,+020,+020,+020,+020,+040,-020,+040",
                "LINE,+000,+000,+000,+020",
                "LINE,+000,+040,+000,+060",
                "LINE,-020,+020,+020,+020",
                "LINE,-020,+040,+020,+040",
                "LINE,-020,+020,-020,+040",
                "LINE,+020,+020,+020,+040",
            ],
            bounds: (-22, 0, 22, 60),
        },
        SymbolDef {
            tipo: 9004,
            category: SymbolCategory::CoilsTimersLamps,
            name: "Temporizador ao Trabalho (On-Delay)",
            short_label: "Temp ON (KT)",
            default_tag: "-KT",
            default_value: "3",
            pins: vec![nume_pin(0, "A1", 5, 0), nume_pin(1, "A2", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+020",
                "LINE,+000,+040,+000,+060",
                "LINE,-020,+020,+020,+020",
                "LINE,-020,+040,+020,+040",
                "LINE,-020,+020,-020,+040",
                "LINE,+020,+020,+020,+040",
                "LINE,-005,+020,-005,+040",
                "LINE,-020,+040,-005,+020",
                "LINE,-020,+020,-005,+040",
            ],
            active_cmds: &[
                "POLI,-020,+020,+020,+020,+020,+040,-020,+040",
                "LINE,+000,+000,+000,+020",
                "LINE,+000,+040,+000,+060",
                "LINE,-020,+020,+020,+020",
                "LINE,-020,+040,+020,+040",
                "LINE,-020,+020,-020,+040",
                "LINE,+020,+020,+020,+040",
                "LINE,-005,+020,-005,+040",
                "LINE,-020,+040,-005,+020",
                "LINE,-020,+020,-005,+040",
            ],
            bounds: (-22, 0, 35, 60),
        },
        SymbolDef {
            tipo: 9005,
            category: SymbolCategory::CoilsTimersLamps,
            name: "Temporizador à Desconexão (Off-Delay)",
            short_label: "Temp OFF (KT)",
            default_tag: "-KT",
            default_value: "3",
            pins: vec![nume_pin(0, "A1", 5, 0), nume_pin(1, "A2", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+020",
                "LINE,+000,+040,+000,+060",
                "LINE,-020,+020,+020,+020",
                "LINE,-020,+040,+020,+040",
                "LINE,-020,+020,-020,+040",
                "LINE,+020,+020,+020,+040",
                "POLI,-020,+020,-005,+020,-005,+040,-020,+040",
            ],
            active_cmds: &[
                "POLI,-020,+020,+020,+020,+020,+040,-020,+040",
                "LINE,+000,+000,+000,+020",
                "LINE,+000,+040,+000,+060",
                "LINE,-020,+020,+020,+020",
                "LINE,-020,+040,+020,+040",
                "LINE,-020,+020,-020,+040",
                "LINE,+020,+020,+020,+040",
                "POLI,-020,+020,-005,+020,-005,+040,-020,+040",
            ],
            bounds: (-22, 0, 35, 60),
        },
        SymbolDef {
            tipo: 9008,
            category: SymbolCategory::CoilsTimersLamps,
            name: "Sinaleiro Luminoso (Piloto)",
            short_label: "Piloto (H)",
            default_tag: "-H",
            default_value: "",
            pins: vec![nume_pin(0, "X1", 5, 0), nume_pin(1, "X2", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "ELIP,-015,+015,+015,+045",
                "LINE,-010,+020,+010,+040",
                "LINE,-010,+040,+010,+020",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "ELIP,-015,+015,+015,+045",
                "LINE,-010,+020,+010,+040",
                "LINE,-010,+040,+010,+020",
                "LINE,-022,+030,-017,+030",
                "LINE,+017,+030,+022,+030",
                "LINE,-016,+014,-012,+018",
                "LINE,+012,+018,+016,+014",
            ],
            bounds: (-22, 0, 22, 60),
        },
        SymbolDef {
            tipo: 9013,
            category: SymbolCategory::CoilsTimersLamps,
            name: "Sirene / Alarme Sonoro (Buzzer)",
            short_label: "Sirene (H)",
            default_tag: "-H",
            default_value: "",
            pins: vec![nume_pin(0, "X1", 5, 0), nume_pin(1, "X2", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+020",
                "LINE,+000,+040,+000,+060",
                "LINE,-010,+020,+010,+020",
                "LINE,+010,+020,+010,+040",
                "LINE,+010,+040,-010,+040",
                "LINE,-010,+040,-010,+020",
                "LINE,+010,+027,+033,+022",
                "LINE,+010,+033,+043,+038",
                "LINE,+033,+022,+043,+038",
            ],
            active_cmds: &[
                "POLI,-010,+020,+010,+020,+010,+040,-010,+040",
                "POLI,+010,+027,+033,+022,+043,+038,+010,+033",
                "LINE,+000,+000,+000,+020",
                "LINE,+000,+040,+000,+060",
            ],
            bounds: (-15, 0, 45, 60),
        },
        SymbolDef {
            tipo: 2008,
            category: SymbolCategory::ContactorsSwitches,
            name: "Contator Disjuntor Trifásico 3P",
            short_label: "Contator-Disj 3P",
            default_tag: "-KM",
            default_value: "",
            pins: vec![
                nume_pin(0, "1", 5, 0),
                nume_pin(1, "3", 35, 0),
                nume_pin(2, "5", 65, 0),
                nume_pin(3, "2", 5, 45),
                nume_pin(4, "4", 35, 45),
                nume_pin(5, "6", 65, 45),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,+060,+000,+060,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+060,+045,+060,+060",
                "LINE,-015,+015,+000,+045",
                "LINE,+015,+015,+030,+045",
                "LINE,+045,+015,+060,+045",
                "LINE,-007,+030,+003,+030",
                "LINE,+018,+030,+028,+030",
                "LINE,+042,+030,+052,+030",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+030,+000,+030,+015",
                "LINE,+060,+000,+060,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+030,+045,+030,+060",
                "LINE,+060,+045,+060,+060",
                "LINE,-005,+015,+000,+045",
                "LINE,+025,+015,+030,+045",
                "LINE,+055,+015,+060,+045",
                "LINE,-005,+015,+000,+015",
                "LINE,+025,+015,+030,+015",
                "LINE,+055,+015,+060,+015",
                "LINE,+003,+030,+013,+030",
                "LINE,+028,+030,+038,+030",
                "LINE,+052,+030,+062,+030",
            ],
            bounds: (-18, 0, 75, 60),
        },
        SymbolDef {
            tipo: 8023,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Botão Pulsador 2D (Painel)",
            short_label: "Botão 2D",
            default_tag: "-S",
            default_value: "",
            pins: vec![],
            idle_cmds: &["ELIP,-020,-020,+020,+020", "ELIP,-015,-015,+015,+015"],
            active_cmds: &["ELIP,-020,-020,+020,+020", "ELIP,-012,-012,+012,+012"],
            bounds: (-22, -22, 22, 22),
        },
        SymbolDef {
            tipo: 8030,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Botão de Emergência com Trava NF (Emergency Stop Latch NC)",
            short_label: "Emergência NF",
            default_tag: "-S",
            default_value: "",
            pins: vec![nume_pin(0, "11", 5, 0), nume_pin(1, "12", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+015,+015,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-005,+030",
                "LINE,+000,+030,+008,+030",
                "ARCO,-030,+020,-010,+040,-020,+020,-020,+040",
                "LINE,-020,+020,-020,+040",
                "LINE,-030,+020,-025,+020",
                "LINE,-025,+020,-025,+025",
                "LINE,-030,+025,-025,+025",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+022,+018,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+005,+030",
                "ARCO,-020,+020,+000,+040,-010,+020,-010,+040",
                "LINE,-010,+020,-010,+040",
                "LINE,-020,+020,-015,+020",
                "LINE,-015,+020,-015,+025",
                "LINE,-020,+025,-015,+025",
            ],
            bounds: (-35, 0, 22, 60),
        },
        SymbolDef {
            tipo: 8031,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Botão de Emergência com Trava NA (Emergency Stop Latch NO)",
            short_label: "Emergência NA",
            default_tag: "-S",
            default_value: "",
            pins: vec![nume_pin(0, "13", 5, 0), nume_pin(1, "14", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-015,+015,+000,+045",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-007,+030",
                "ARCO,-030,+020,-010,+040,-020,+020,-020,+040",
                "LINE,-020,+020,-020,+040",
                "LINE,-030,+020,-025,+020",
                "LINE,-025,+020,-025,+025",
                "LINE,-030,+025,-025,+025",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,-005,+015,+000,+045",
                "LINE,-005,+015,+000,+015",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+001,+030",
                "ARCO,-020,+020,+000,+040,-010,+020,-010,+040",
                "LINE,-010,+020,-010,+040",
                "LINE,-020,+020,-015,+020",
                "LINE,-015,+020,-015,+025",
                "LINE,-020,+025,-015,+025",
            ],
            bounds: (-35, 0, 15, 60),
        },
        SymbolDef {
            tipo: 8032,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Botão de Emergência com Trava Duplo NA+NF (Emergency Stop NO-NC)",
            short_label: "Emergência NA+NF",
            default_tag: "-S",
            default_value: "",
            pins: vec![
                nume_pin(0, "11", 5, 0),
                nume_pin(1, "13", 50, 0),
                nume_pin(2, "12", 5, 45),
                nume_pin(3, "14", 50, 45),
            ],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+045,+000,+045,+015",
                "LINE,+045,+045,+045,+060",
                "LINE,+015,+015,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,+030,+015,+045,+045",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-005,+030",
                "LINE,+000,+030,+010,+030",
                "LINE,+015,+030,+025,+030",
                "LINE,+030,+030,+038,+030",
                "ARCO,-030,+020,-010,+040,-020,+020,-020,+040",
                "LINE,-020,+020,-020,+040",
                "LINE,-030,+020,-025,+020",
                "LINE,-025,+020,-025,+025",
                "LINE,-030,+025,-025,+025",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+045,+000,+045,+015",
                "LINE,+045,+045,+045,+060",
                "LINE,+022,+018,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,+040,+015,+045,+045",
                "LINE,+040,+015,+045,+015",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+005,+030",
                "LINE,+010,+030,+020,+030",
                "LINE,+025,+030,+035,+030",
                "LINE,+040,+030,+048,+030",
                "ARCO,-020,+020,+000,+040,-010,+020,-010,+040",
                "LINE,-010,+020,-010,+040",
                "LINE,-020,+020,-015,+020",
                "LINE,-015,+020,-015,+025",
                "LINE,-020,+025,-015,+025",
            ],
            bounds: (-35, 0, 58, 60),
        },
        SymbolDef {
            tipo: 8033,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Botão de Emergência Girar para Destravar NF (Emergency Turn-to-Release)",
            short_label: "Emergência Giro NF",
            default_tag: "-S",
            default_value: "",
            pins: vec![nume_pin(0, "11", 5, 0), nume_pin(1, "12", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+015,+015,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-005,+030",
                "LINE,+000,+030,+008,+030",
                "ARCO,-030,+020,-010,+040,-020,+020,-020,+040",
                "LINE,-020,+020,-020,+040",
                "ARCO,-026,+010,-014,+022,-020,+010,-020,+022",
                "LINE,-020,+010,-017,+008",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+022,+018,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+005,+030",
                "ARCO,-020,+020,+000,+040,-010,+020,-010,+040",
                "LINE,-010,+020,-010,+040",
                "ARCO,-016,+010,-004,+022,-010,+010,-010,+022",
                "LINE,-010,+010,-007,+008",
            ],
            bounds: (-35, 0, 22, 60),
        },
        SymbolDef {
            tipo: 8034,
            category: SymbolCategory::PushbuttonsActuators,
            name: "Botão de Emergência com Chave NF (Emergency Key-Release)",
            short_label: "Emergência Chave NF",
            default_tag: "-S",
            default_value: "",
            pins: vec![nume_pin(0, "11", 5, 0), nume_pin(1, "12", 5, 45)],
            idle_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+015,+015,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,+000,+015,+000,+018",
                "LINE,-030,+030,-020,+030",
                "LINE,-015,+030,-005,+030",
                "LINE,+000,+030,+008,+030",
                "ARCO,-030,+020,-010,+040,-020,+020,-020,+040",
                "LINE,-020,+020,-020,+040",
                "LINE,-025,+012,-020,+012",
                "LINE,-025,+012,-025,+018",
                "ELIP,-029,+015,-025,+021",
            ],
            active_cmds: &[
                "LINE,+000,+000,+000,+015",
                "LINE,+000,+045,+000,+060",
                "LINE,+022,+018,+000,+045",
                "LINE,+000,+018,+018,+018",
                "LINE,-020,+030,-010,+030",
                "LINE,-005,+030,+005,+030",
                "ARCO,-020,+020,+000,+040,-010,+020,-010,+040",
                "LINE,-010,+020,-010,+040",
                "LINE,-015,+012,-010,+012",
                "LINE,-015,+012,-015,+018",
                "ELIP,-019,+015,-015,+021",
            ],
            bounds: (-35, 0, 22, 60),
        },
    ]
}

/// Busca a referência estática a um símbolo pelo seu código `tipo` sem clonar (O(1)).
pub fn find_symbol_ref(tipo: i32) -> Option<&'static SymbolDef> {
    let &idx = CATALOG_INDEX.get(&tipo)?;
    CATALOG.get(idx)
}

/// Busca um símbolo pelo seu código `tipo`.
pub fn find_symbol(tipo: i32) -> Option<SymbolDef> {
    find_symbol_ref(tipo).cloned()
}

/// Tipos de contatos elétricos e botoeiras segundo a norma IEC 60947-5-1 / EN 60947.
/// O primeiro dígito (1..=9) é o número sequencial do contato no mesmo componente.
/// O segundo dígito (função) indica o estado normal do contato:
/// - .1 / .2: Normalmente Fechado (NF / NC)
/// - .3 / .4: Normalmente Aberto (NA / NO)
/// - .5 / .6: NF especial / temporizado / desarme térmico (ex: 95-96)
/// - .7 / .8: NA especial / temporizado / desarme térmico (ex: 97-98)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactKind {
    /// Contato NA (NO) com 2 bornes (.3 e .4), e.g. 13-14, 23-24, 33-34...
    NO,
    /// Contato NF (NC) com 2 bornes (.1 e .2), e.g. 11-12, 21-22, 31-32...
    NC,
    /// Contato Duplo NA+NF em bloco único com 4 bornes (.1, .3, .2, .4), e.g. 11, 13, 12, 14.
    Double,
    /// Contato Comutador / Inversor (CON) com 3 bornes (.1 comum, .2 NF, .4 NA), e.g. 11, 12, 14.
    Changeover,
    /// Contato Temporizado / Especial NA com 2 bornes (.7 e .8), e.g. 67-68, 17-18, 97-98.
    TimedNO,
    /// Contato Temporizado / Especial NF com 2 bornes (.5 e .6), e.g. 55-56, 15-16, 95-96.
    TimedNC,
    /// Contato Temporizado Duplo NA+NF com 4 bornes (.5, .7, .6, .8), e.g. 15, 17, 16, 18.
    TimedDouble,
    /// Contato Térmico Comutador CON com 3 bornes (.5 comum, .6 NF, .8 NA), e.g. 95, 96, 98.
    ThermalChangeover,
}

impl ContactKind {
    /// Gera a numeração dos bornes segundo a dezena sequencial `digit` (1..=9) e a norma IEC 60947.
    pub fn generate_pins(self, digit: u8) -> Vec<String> {
        let d = digit.clamp(1, 9);
        match self {
            ContactKind::NO => vec![format!("{d}3"), format!("{d}4")],
            ContactKind::NC => vec![format!("{d}1"), format!("{d}2")],
            ContactKind::Double => vec![
                format!("{d}1"),
                format!("{d}3"),
                format!("{d}2"),
                format!("{d}4"),
            ],
            ContactKind::Changeover => vec![format!("{d}1"), format!("{d}2"), format!("{d}4")],
            ContactKind::TimedNO => vec![format!("{d}7"), format!("{d}8")],
            ContactKind::TimedNC => vec![format!("{d}5"), format!("{d}6")],
            ContactKind::TimedDouble => vec![
                format!("{d}5"),
                format!("{d}7"),
                format!("{d}6"),
                format!("{d}8"),
            ],
            ContactKind::ThermalChangeover => {
                vec![format!("{d}5"), format!("{d}6"), format!("{d}8")]
            }
        }
    }
}

/// Identifica se um símbolo é um contato auxiliar, botão ou chave seletora com numeração sequencial IEC.
pub fn detect_contact_kind(tipo: i32, default_pins: &[SymbolPin]) -> Option<ContactKind> {
    match tipo {
        7000 | 8000 | 8004 | 8008 | 8012 | 8031 => Some(ContactKind::NO),
        7001 | 8001 | 8005 | 8009 | 8013 | 8030 | 8033 | 8034 => Some(ContactKind::NC),
        7002 | 8002 | 8006 | 8010 | 8014 | 8032 => Some(ContactKind::Double),
        8003 | 8007 | 8011 | 8015 => Some(ContactKind::Changeover),
        7004 | 7008 | 8016 => Some(ContactKind::TimedNO),
        7005 | 7009 | 8017 => Some(ContactKind::TimedNC),
        7006 | 8018 => Some(ContactKind::TimedDouble),
        8019 => Some(ContactKind::ThermalChangeover),
        // Símbolos que NUNCA são contatos subordinados (devem manter seus bornes fixos de catálogo):
        9000 | 9004 | 9005 // Bobinas e temporizadores: sempre A1-A2 (IEC 60947-1 / EN 50012)
        | 9008 // Sinaleiros luminosos: sempre X1-X2
        | 9013 // Sirenes / alarmes sonoros
        | 1000 | 1001 | 1002 // Motores: sempre U, V, W, PE
        | 2000 | 2001 | 2006 | 2008 // Contatores de força: sempre 1-2, 3-4, 5-6
        | 5000 | 5003 | 5008 // Fusíveis: 1-2, 3-4, 5-6
        | 6000 | 6003 | 6004 | 6007 | 6008 | 6009 // Disjuntores e relé térmico de força
        | 8020 | 8021 | 8022 // Chaves comutadoras multiseletoras de polos fixos
        | 3000..=3009 // Linhas de alimentação L, N, PE
        => None,
        _ => {
            // Detecção heurística estrita para símbolos customizados/SVG/QET:
            // Contatos auxiliares IEC possuem 2 dígitos numéricos (ex: 13-14, 21-22).
            // NUNCA podem iniciar com letras como 'A' (A1-A2), 'X' (X1-X2), etc.
            if default_pins.len() == 2 {
                let p0 = default_pins[0].default_label.trim();
                let p1 = default_pins[1].default_label.trim();
                if p0.len() == 2 && p1.len() == 2
                    && p0.chars().all(|c| c.is_ascii_digit())
                    && p1.chars().all(|c| c.is_ascii_digit())
                {
                    if p0.ends_with('3') && p1.ends_with('4') {
                        Some(ContactKind::NO)
                    } else if p0.ends_with('1') && p1.ends_with('2') {
                        Some(ContactKind::NC)
                    } else if p0.ends_with('7') && p1.ends_with('8') {
                        Some(ContactKind::TimedNO)
                    } else if p0.ends_with('5') && p1.ends_with('6') {
                        Some(ContactKind::TimedNC)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else if default_pins.len() == 3 {
                let p0 = default_pins[0].default_label.trim();
                let p1 = default_pins[1].default_label.trim();
                let p2 = default_pins[2].default_label.trim();
                if p0.len() == 2 && p1.len() == 2 && p2.len() == 2
                    && p0.chars().all(|c| c.is_ascii_digit())
                    && p1.chars().all(|c| c.is_ascii_digit())
                    && p2.chars().all(|c| c.is_ascii_digit())
                {
                    if p0.ends_with('1') && p1.ends_with('2') && p2.ends_with('4') {
                        Some(ContactKind::Changeover)
                    } else if p0.ends_with('5') && p1.ends_with('6') && p2.ends_with('8') {
                        Some(ContactKind::ThermalChangeover)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else if default_pins.len() == 4 {
                let p0 = default_pins[0].default_label.trim();
                let p1 = default_pins[1].default_label.trim();
                let p2 = default_pins[2].default_label.trim();
                let p3 = default_pins[3].default_label.trim();
                if p0.len() == 2 && p1.len() == 2 && p2.len() == 2 && p3.len() == 2
                    && p0.chars().all(|c| c.is_ascii_digit())
                    && p1.chars().all(|c| c.is_ascii_digit())
                    && p2.chars().all(|c| c.is_ascii_digit())
                    && p3.chars().all(|c| c.is_ascii_digit())
                {
                    if p0.ends_with('1') && p1.ends_with('3') && p2.ends_with('2') && p3.ends_with('4') {
                        Some(ContactKind::Double)
                    } else if p0.ends_with('5') && p1.ends_with('7') && p2.ends_with('6') && p3.ends_with('8') {
                        Some(ContactKind::TimedDouble)
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        }
    }
}

/// Detecta a sequência numérica atual (1..=9) de um componente a partir dos seus bornes atuais.
pub fn detect_current_sequence(pins: &[String], kind: ContactKind) -> Option<u8> {
    let p0 = pins.first()?.trim();
    if p0.len() != 2 || !p0.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let first_char = p0.chars().next()?;
    let digit = first_char.to_digit(10)? as u8;
    if !(1..=9).contains(&digit) {
        return None;
    }
    // Confirma se o primeiro borne condiz com o padrão esperado para este tipo de contato
    let expected = kind.generate_pins(digit);
    if let Some(exp_first) = expected.first() {
        if p0 == exp_first.as_str() {
            return Some(digit);
        }
    }
    Some(digit)
}

/// Varre o esquema (`items`) em busca de outros contatos com a mesma tag e retorna o conjunto
/// de sequências numéricas (1..=9) já ocupadas.
pub fn find_occupied_sequences_for_tag(
    items: &[SchematicItem],
    target_tag: &str,
    exclude_item_id: Option<i32>,
) -> std::collections::HashSet<u8> {
    let mut occupied = std::collections::HashSet::new();
    let norm_tag = target_tag.trim();
    if norm_tag.is_empty() {
        return occupied;
    }

    for item in items {
        if Some(item.id) == exclude_item_id {
            continue;
        }
        if !item.tag.trim().eq_ignore_ascii_case(norm_tag) {
            continue;
        }

        if let Some(sym) = find_symbol(item.tipo) {
            if let Some(kind) = detect_contact_kind(item.tipo, &sym.pins) {
                if let Some(seq) = detect_current_sequence(&item.pins, kind) {
                    occupied.insert(seq);
                } else {
                    // Fallback para bornes customizados que iniciam com 1..9 e terminam em dígito funcional 1..8
                    for pin in &item.pins {
                        let p = pin.trim();
                        if p.len() == 2 && p.chars().all(|c| c.is_ascii_digit()) {
                            if let Some(c0) = p.chars().next() {
                                if let Some(d) = c0.to_digit(10) {
                                    if (1..=9).contains(&d) {
                                        if let Some(c1) = p.chars().nth(1) {
                                            if ('1'..='8').contains(&c1) {
                                                occupied.insert(d as u8);
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    occupied
}

/// Classificação funcional de um componente para hierarquia e sequenciamento de tags (IEC 81346 / IEC 60204-1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApparatusRole {
    /// Bobina de contator ou relé de comando (A1-A2, ex: -KM1, -KM2, -KA1)
    Coil,
    /// Temporizador / relé temporizado (A1-A2, ex: -KT1, -KT2)
    Timer,
    /// Relé térmico de sobrecarga de potência (ex: -F1, -F2)
    ThermalRelay,
    /// Disjuntor termomagnético / motor (ex: -Q1, -Q2)
    Breaker,
    /// Fusível de proteção (ex: -F1, -F2)
    Fuse,
    /// Motor elétrico (ex: -M1, -M2)
    Motor,
    /// Botoeira / chave seletora / interruptor de comando (ex: -S1, -S2, -B1, -B2)
    SwitchOrButton,
    /// Sinaleiro luminoso / piloto / sirene (ex: -H1, -H2)
    Signaling,
    /// Polos principais de potência de contator (1-2, 3-4, 5-6, vinculados a uma bobina -KM)
    PowerContactorPoles,
    /// Contato auxiliar de contator (NA/NF, vinculado a uma bobina -KM)
    AuxiliaryContact,
    /// Contato temporizado (NA/NF, vinculado a um relé temporizador -KT)
    TimedContact,
    /// Contato auxiliar de relé térmico (95-96 / 97-98, vinculado a um relé térmico -F)
    ThermalAuxContact,
    /// Alimentação ou outro componente passivo
    Other,
}

impl ApparatusRole {
    /// Indica se o aparelho é primário/independente, devendo gerar uma tag sequencial própria (-KM1, -KM2, -S1, -S2, etc.).
    pub fn is_independent_apparatus(self) -> bool {
        matches!(
            self,
            ApparatusRole::Coil
                | ApparatusRole::Timer
                | ApparatusRole::ThermalRelay
                | ApparatusRole::Breaker
                | ApparatusRole::Fuse
                | ApparatusRole::Motor
                | ApparatusRole::SwitchOrButton
                | ApparatusRole::Signaling
        )
    }

    /// Indica se é um contato subordinado a um aparelho atuador pai.
    pub fn is_subordinate_contact(self) -> bool {
        matches!(
            self,
            ApparatusRole::PowerContactorPoles
                | ApparatusRole::AuxiliaryContact
                | ApparatusRole::TimedContact
                | ApparatusRole::ThermalAuxContact
        )
    }

    /// Papel do aparelho pai necessário para acionar este contato.
    pub fn required_parent_role(self) -> Option<ApparatusRole> {
        match self {
            ApparatusRole::PowerContactorPoles | ApparatusRole::AuxiliaryContact => {
                Some(ApparatusRole::Coil)
            }
            ApparatusRole::TimedContact => Some(ApparatusRole::Timer),
            ApparatusRole::ThermalAuxContact => Some(ApparatusRole::ThermalRelay),
            _ => None,
        }
    }

    /// Nome legível em português do aparelho pai esperado.
    pub fn parent_display_name(self) -> &'static str {
        match self {
            ApparatusRole::PowerContactorPoles | ApparatusRole::AuxiliaryContact => {
                "bobina de contator (-KM)"
            }
            ApparatusRole::TimedContact => "relé temporizador (-KT)",
            ApparatusRole::ThermalAuxContact => "relé térmico (-F)",
            _ => "aparelho pai",
        }
    }
}

pub fn classify_apparatus_role(tipo: i32) -> ApparatusRole {
    match tipo {
        9000 => ApparatusRole::Coil,
        9004 | 9005 => ApparatusRole::Timer,
        6007 => ApparatusRole::ThermalRelay,
        6000 | 6003 | 6004 | 6008 | 6009 => ApparatusRole::Breaker,
        5000 | 5003 | 5008 => ApparatusRole::Fuse,
        1000..=1002 => ApparatusRole::Motor,
        8000..=8015 | 8020..=8023 | 8030..=8034 | 2004 => ApparatusRole::SwitchOrButton,
        9008 | 9013 => ApparatusRole::Signaling,
        2000 | 2001 | 2006 | 2008 => ApparatusRole::PowerContactorPoles,
        7000..=7002 => ApparatusRole::AuxiliaryContact,
        7004..=7006 | 7008..=7009 => ApparatusRole::TimedContact,
        8016..=8019 => ApparatusRole::ThermalAuxContact,
        _ => ApparatusRole::Other,
    }
}

/// Extrai o prefixo literal de uma tag (ex: "-KM" de "-KM1", "-S" de "-S2", "-B" de "-B1").
pub fn extract_tag_prefix(tag: &str) -> &str {
    let t = tag.trim();
    if let Some(pos) = t.rfind(|c: char| !c.is_ascii_digit()) {
        &t[..=pos]
    } else {
        t
    }
}

/// Calcula a próxima tag sequencial para um aparelho considerando elementos do mesmo tipo/papel.
pub fn next_sequential_tag_for_role(
    items: &[SchematicItem],
    role: ApparatusRole,
    prefix: &str,
) -> String {
    let base = prefix.trim();
    if base.is_empty() {
        return String::new();
    }
    let mut used_numbers = std::collections::HashSet::new();
    let base_upper = base.to_ascii_uppercase();

    for item in items {
        if classify_apparatus_role(item.tipo) == role {
            let tag = item.tag.trim();
            let tag_upper = tag.to_ascii_uppercase();
            if tag_upper.starts_with(&base_upper) {
                let remainder = &tag_upper[base_upper.len()..];
                if let Ok(num) = remainder.parse::<u32>() {
                    used_numbers.insert(num);
                } else if remainder.is_empty() {
                    used_numbers.insert(1);
                }
            }
        }
    }

    let mut next_num = 1;
    while used_numbers.contains(&next_num) {
        next_num += 1;
    }
    format!("{base}{next_num}")
}

/// Calcula a próxima tag sequencial para um prefixo genérico.
pub fn next_sequential_tag_for_prefix(items: &[SchematicItem], prefix: &str) -> String {
    let base = prefix.trim();
    if base.is_empty() {
        return String::new();
    }
    let mut used_numbers = std::collections::HashSet::new();
    let base_upper = base.to_ascii_uppercase();

    for item in items {
        let tag = item.tag.trim();
        let tag_upper = tag.to_ascii_uppercase();
        if tag_upper.starts_with(&base_upper) {
            let remainder = &tag_upper[base_upper.len()..];
            if let Ok(num) = remainder.parse::<u32>() {
                used_numbers.insert(num);
            } else if remainder.is_empty() {
                // Se existe apenas o prefixo sem número (ex: "-KM"), consideramos número 1 como ocupado
                used_numbers.insert(1);
            }
        }
    }

    let mut next_num = 1;
    while used_numbers.contains(&next_num) {
        next_num += 1;
    }
    format!("{base}{next_num}")
}

/// Verifica se existe um aparelho atuador pai no esquema para o contato selecionado.
pub fn find_parent_apparatus<'a>(
    items: &'a [SchematicItem],
    subordinate_item: &SchematicItem,
) -> Option<&'a SchematicItem> {
    let role = classify_apparatus_role(subordinate_item.tipo);
    let parent_role = role.required_parent_role()?;
    let norm_tag = subordinate_item.tag.trim();
    if norm_tag.is_empty() {
        return None;
    }

    items.iter().find(|i| {
        classify_apparatus_role(i.tipo) == parent_role
            && i.tag.trim().eq_ignore_ascii_case(norm_tag)
    })
}

/// Lista todas as tags de aparelhos pais compatíveis existentes no esquema (ex: todas as bobinas "-KM1", "-KM2").
pub fn list_available_parent_tags(
    items: &[SchematicItem],
    subordinate_role: ApparatusRole,
) -> Vec<String> {
    let Some(parent_role) = subordinate_role.required_parent_role() else {
        return Vec::new();
    };
    let mut tags = Vec::new();
    for i in items {
        if classify_apparatus_role(i.tipo) == parent_role {
            let t = i.tag.trim();
            if !t.is_empty()
                && !tags
                    .iter()
                    .any(|existing: &String| existing.eq_ignore_ascii_case(t))
            {
                tags.push(t.to_string());
            }
        }
    }
    tags.sort();
    tags
}

/// Verifica se já existe outro aparelho independente do mesmo tipo com a mesma tag (duplicidade indevida).
pub fn find_duplicate_apparatus(items: &[SchematicItem], item: &SchematicItem) -> bool {
    let role = classify_apparatus_role(item.tipo);
    if !role.is_independent_apparatus() {
        return false;
    }
    let norm_tag = item.tag.trim();
    if norm_tag.is_empty() {
        return false;
    }

    items.iter().any(|other| {
        other.id != item.id
            && classify_apparatus_role(other.tipo) == role
            && other.tag.trim().eq_ignore_ascii_case(norm_tag)
    })
}

/// Adapta as cores de um documento SVG para tema claro ou escuro.
/// No tema escuro, substitui traços e preenchimentos escuros/pretos por traços claros (#E2E8F0)
/// e suaviza preenchimentos brancos para (#1E293B).
pub fn adapt_svg_colors(svg_content: &str, is_dark: bool) -> String {
    if !is_dark {
        return svg_content.to_string();
    }
    let mut s = svg_content.to_string();

    // Lista de padrões de traço escuro para substituir por claro (#E2E8F0)
    let dark_strokes = [
        "stroke=\"black\"",
        "stroke='black'",
        "stroke=\"#111827\"",
        "stroke='#111827'",
        "stroke=\"#000000\"",
        "stroke='#000000'",
        "stroke=\"#000\"",
        "stroke='#000'",
        "stroke=\"HTMLGrayBlack\"",
        "stroke='HTMLGrayBlack'",
        "stroke=\"#1f2937\"",
        "stroke='#1f2937'",
        "stroke=\"#212529\"",
        "stroke='#212529'",
        "stroke=\"#222222\"",
        "stroke='#222222'",
        "stroke=\"#222\"",
        "stroke='#222'",
        "stroke=\"#333333\"",
        "stroke='#333333'",
        "stroke=\"#333\"",
        "stroke='#333'",
        "stroke=\"#1a1a1a\"",
        "stroke='#1a1a1a'",
        "stroke=\"#111\"",
        "stroke='#111'",
    ];
    for ds in dark_strokes {
        s = s.replace(ds, "stroke=\"#E2E8F0\"");
    }

    // Lista de padrões de preenchimento escuro (sólido) para substituir por claro (#E2E8F0)
    let dark_fills = [
        "fill=\"black\"",
        "fill='black'",
        "fill=\"#111827\"",
        "fill='#111827'",
        "fill=\"#000000\"",
        "fill='#000000'",
        "fill=\"#000\"",
        "fill='#000'",
        "fill=\"HTMLGrayBlack\"",
        "fill='HTMLGrayBlack'",
        "fill=\"#1f2937\"",
        "fill='#1f2937'",
        "fill=\"#212529\"",
        "fill='#212529'",
        "fill=\"#222222\"",
        "fill='#222222'",
        "fill=\"#222\"",
        "fill='#222'",
        "fill=\"#333333\"",
        "fill='#333333'",
        "fill=\"#333\"",
        "fill='#333'",
    ];
    for df in dark_fills {
        s = s.replace(df, "fill=\"#E2E8F0\"");
    }

    // Substitui preenchimento branco opaco por tom escuro suave (#1E293B)
    let white_fills = [
        "fill=\"white\"",
        "fill='white'",
        "fill=\"#ffffff\"",
        "fill='#ffffff'",
        "fill=\"#fff\"",
        "fill='#fff'",
        "fill=\"HTMLWhiteIvory\"",
        "fill='HTMLWhiteIvory'",
        "fill=\"HTMLWhiteSnow\"",
        "fill='HTMLWhiteSnow'",
        "fill=\"HTMLWhiteGhostWhite\"",
        "fill='HTMLWhiteGhostWhite'",
        "fill=\"HTMLWhiteAliceBlue\"",
        "fill='HTMLWhiteAliceBlue'",
    ];
    for wf in white_fills {
        s = s.replace(wf, "fill=\"#1E293B\"");
    }

    s
}

/// Gera uma string SVG a partir dos comandos vetoriais do símbolo com o traço especificado.
pub fn generate_svg_from_cmds(
    title: &str,
    cmds: &[&str],
    pins: &[SymbolPin],
    bounds: (i32, i32, i32, i32),
    stroke_color: &str,
    is_active: bool,
) -> String {
    let margin = 10;
    let min_x = bounds.0 - margin;
    let min_y = bounds.1 - margin;
    let width = ((bounds.2 - bounds.0).abs() + margin * 2).max(10);
    let height = ((bounds.3 - bounds.1).abs() + margin * 2).max(10);

    let mut svg = String::new();
    svg.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"{min_x} {min_y} {width} {height}\" data-generator=\"cade2svg\">\n"
    ));
    svg.push_str(&format!("  <title>{title}</title>\n"));
    svg.push_str(&format!(
        "  <g stroke=\"{stroke_color}\" stroke-width=\"1.8\" fill=\"none\" stroke-linecap=\"round\" stroke-linejoin=\"round\">\n"
    ));

    for &cmd in cmds {
        if let Some(prim) = parse_vector_cmd(cmd) {
            match prim {
                VectorPrimitive::Line { x1, y1, x2, y2 } => {
                    svg.push_str(&format!(
                        "    <line x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" />\n"
                    ));
                }
                VectorPrimitive::Ellipse { x1, y1, x2, y2 } => {
                    let cx = (x1 + x2) as f32 * 0.5;
                    let cy = (y1 + y2) as f32 * 0.5;
                    let rx = (x2 - x1).abs() as f32 * 0.5;
                    let ry = (y2 - y1).abs() as f32 * 0.5;
                    let fill = if is_active { "#fee2e2" } else { "none" };
                    svg.push_str(&format!(
                        "    <ellipse cx=\"{cx}\" cy=\"{cy}\" rx=\"{rx}\" ry=\"{ry}\" fill=\"{fill}\" />\n"
                    ));
                }
                VectorPrimitive::Polygon { pts } => {
                    let pts_str = pts
                        .iter()
                        .map(|(x, y)| format!("{x},{y}"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let fill = if is_active { "#fee2e2" } else { "none" };
                    svg.push_str(&format!(
                        "    <polygon points=\"{pts_str}\" fill=\"{fill}\" />\n"
                    ));
                }
                VectorPrimitive::Arc { x1, y1, x2, y2, .. } => {
                    let cx = (x1 + x2) as f32 * 0.5;
                    let cy = (y1 + y2) as f32 * 0.5;
                    let r = ((x2 - x1).abs().min((y2 - y1).abs()) as f32) * 0.45;
                    svg.push_str(&format!(
                        "    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{r}\" />\n"
                    ));
                }
                VectorPrimitive::Text { x, y, text } => {
                    svg.push_str(&format!(
                        "    <text x=\"{x}\" y=\"{y}\" font-size=\"10\" fill=\"{stroke_color}\" stroke=\"none\">{text}</text>\n"
                    ));
                }
            }
        }
    }

    // Desenha os bornes / terminais
    for pin in pins {
        let (px, py) = (pin.conn_pos.0 * 5, pin.conn_pos.1 * 5);
        svg.push_str(&format!(
            "    <circle class=\"qet-terminal\" cx=\"{px}\" cy=\"{py}\" r=\"2.5\" fill=\"#dc2626\" stroke=\"none\" data-label=\"{}\" />\n",
            pin.default_label
        ));
    }

    svg.push_str("  </g>\n");
    svg.push_str("</svg>\n");

    svg
}

/// Obtém o conteúdo SVG do símbolo já devidamente adaptado para o tema (claro ou escuro).
pub fn get_themed_symbol_svg_string(sym: &SymbolDef, is_dark: bool) -> String {
    // 1. Tenta carregar do caminho SVG declarado no símbolo (QElectroTech ou especializado)
    if let Some((svg_idle_path, _)) = sym.svg_paths() {
        let full_path = crate::svg_symbol::resolve_svg_path(svg_idle_path);
        if let Ok(content) = std::fs::read_to_string(&full_path) {
            return adapt_svg_colors(&content, is_dark);
        }
    }

    // 2. Tenta encontrar arquivo gerado correspondente em symbols_qet_all/cade_native/{tipo:04}_*.svg
    let clean_name = sym
        .name
        .replace('/', "-")
        .replace(' ', "_")
        .replace(['(', ')'], "")
        .replace('+', "_")
        .to_lowercase();
    let base_filename = format!("{:04}_{}", sym.tipo, clean_name);
    let native_path = format!("symbols_qet_all/cade_native/{base_filename}.svg");
    let full_native = crate::svg_symbol::resolve_svg_path(&native_path);
    if let Ok(content) = std::fs::read_to_string(&full_native) {
        return adapt_svg_colors(&content, is_dark);
    }

    // 3. Fallback dinâmico: gera o SVG diretamente a partir dos comandos vetoriais e pinos do símbolo
    let stroke_color = if is_dark { "#E2E8F0" } else { "#111827" };
    generate_svg_from_cmds(
        sym.name,
        sym.idle_cmds,
        &sym.pins,
        sym.bounds,
        stroke_color,
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contact_kind_and_no_phantom_types() {
        // Contatos instantâneos válidos
        assert_eq!(detect_contact_kind(7000, &[]), Some(ContactKind::NO));
        assert_eq!(detect_contact_kind(7001, &[]), Some(ContactKind::NC));
        assert_eq!(detect_contact_kind(7002, &[]), Some(ContactKind::Double));

        // Tipos fantasma 7003 e 7007 não devem ser classificados como contatos válidos
        assert_eq!(detect_contact_kind(7003, &[]), None);
        assert_eq!(detect_contact_kind(7007, &[]), None);

        // Contatos temporizados válidos (On-Delay e Off-Delay)
        assert_eq!(detect_contact_kind(7004, &[]), Some(ContactKind::TimedNO));
        assert_eq!(detect_contact_kind(7005, &[]), Some(ContactKind::TimedNC));
        assert_eq!(
            detect_contact_kind(7006, &[]),
            Some(ContactKind::TimedDouble)
        );
        assert_eq!(detect_contact_kind(7008, &[]), Some(ContactKind::TimedNO));
        assert_eq!(detect_contact_kind(7009, &[]), Some(ContactKind::TimedNC));

        // Comutador SPDT válido de botão
        assert_eq!(
            detect_contact_kind(8003, &[]),
            Some(ContactKind::Changeover)
        );
    }

    #[test]
    fn test_apparatus_role_and_no_phantom_types() {
        assert_eq!(
            classify_apparatus_role(7000),
            ApparatusRole::AuxiliaryContact
        );
        assert_eq!(
            classify_apparatus_role(7001),
            ApparatusRole::AuxiliaryContact
        );
        assert_eq!(
            classify_apparatus_role(7002),
            ApparatusRole::AuxiliaryContact
        );
        assert_eq!(classify_apparatus_role(7003), ApparatusRole::Other);

        assert_eq!(classify_apparatus_role(7004), ApparatusRole::TimedContact);
        assert_eq!(classify_apparatus_role(7005), ApparatusRole::TimedContact);
        assert_eq!(classify_apparatus_role(7006), ApparatusRole::TimedContact);
        assert_eq!(classify_apparatus_role(7007), ApparatusRole::Other);
        assert_eq!(classify_apparatus_role(7008), ApparatusRole::TimedContact);
        assert_eq!(classify_apparatus_role(7009), ApparatusRole::TimedContact);
    }
}
