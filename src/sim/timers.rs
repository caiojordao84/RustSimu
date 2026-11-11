use std::collections::HashSet;

use crate::model::{SchematicItem, SimState};

/// Escala multiplicadora de tempo dos temporizadores industriais e formato CADe_SIMU original.
/// No formato `.cad`, o código da escala de tempo fica em `pins[2]` (e `pins[4]` para T_OFF cíclico):
/// - "0": x 1 Seg (fator 1.0)
/// - "1": x 10 Seg (fator 10.0)
/// - "2": x 60 Seg (fator 60.0 / 1 min)
/// - "3": x 0.1 Seg (fator 0.1 / décimos de segundo)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TimerScaleMultiplier {
    #[default]
    X1 = 0, // "0" no CADe_SIMU -> x 1 Seg
    X10 = 1,  // "1" no CADe_SIMU -> x 10 Seg
    X60 = 2,  // "2" no CADe_SIMU -> x 60 Seg (1 min)
    X0_1 = 3, // "3" no CADe_SIMU -> x 0.1 Seg (décimos de segundo)
}

impl TimerScaleMultiplier {
    pub const ALL: [TimerScaleMultiplier; 4] = [
        TimerScaleMultiplier::X0_1,
        TimerScaleMultiplier::X1,
        TimerScaleMultiplier::X10,
        TimerScaleMultiplier::X60,
    ];

    pub fn code(self) -> &'static str {
        match self {
            TimerScaleMultiplier::X1 => "0",
            TimerScaleMultiplier::X10 => "1",
            TimerScaleMultiplier::X60 => "2",
            TimerScaleMultiplier::X0_1 => "3",
        }
    }

    pub fn from_code(s: &str) -> Self {
        match s.trim() {
            "1" => TimerScaleMultiplier::X10,
            "2" => TimerScaleMultiplier::X60,
            "3" => TimerScaleMultiplier::X0_1,
            _ => TimerScaleMultiplier::X1,
        }
    }

    pub fn factor(self) -> f64 {
        match self {
            TimerScaleMultiplier::X0_1 => 0.1,
            TimerScaleMultiplier::X1 => 1.0,
            TimerScaleMultiplier::X10 => 10.0,
            TimerScaleMultiplier::X60 => 60.0,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            TimerScaleMultiplier::X0_1 => "x 0.1 Seg (Décimos)",
            TimerScaleMultiplier::X1 => "x 1 Seg (Segundos)",
            TimerScaleMultiplier::X10 => "x 10 Seg (Dezenas)",
            TimerScaleMultiplier::X60 => "x 60 Seg (Minutos)",
        }
    }
}

impl std::fmt::Display for TimerScaleMultiplier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Formata a duração em segundos de forma limpa e adaptativa (sem zeros desnecessários, mas preservando centésimos).
pub fn format_timer_seconds(sec: f64) -> String {
    if (sec - sec.round()).abs() < 1e-6 {
        format!("{:.0}", sec)
    } else if (sec * 10.0 - (sec * 10.0).round()).abs() < 1e-4 {
        format!("{:.1}", sec)
    } else if (sec * 100.0 - (sec * 100.0).round()).abs() < 1e-3 {
        format!("{:.2}", sec)
    } else {
        let s = format!("{:.3}", sec);
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

/// Extrai a escala multiplicadora de tempo (`pins[2]`).
pub fn timer_scale_multiplier(pins: &[String; 9]) -> TimerScaleMultiplier {
    TimerScaleMultiplier::from_code(&pins[2])
}

/// Extrai o tempo programado efetivo (em milissegundos) de um temporizador `9004` (TON) ou `9005` (TOF).
/// No formato `.cad` e no CADe_SIMU original, o tempo total em segundos fica em `pins[3]` (ou `value` no inspetor).
pub fn timer_preset_ms(pins: &[String; 9], value: &str) -> u64 {
    let sec = pins[3]
        .trim()
        .parse::<f64>()
        .ok()
        .or_else(|| value.trim().parse::<f64>().ok())
        .unwrap_or(3.0)
        .max(0.01);
    (sec * 1000.0).round() as u64
}

/// Extrai o valor base de tempo (tempo total efetivo dividido pelo fator multiplicador).
pub fn timer_base_value(pins: &[String; 9], value: &str) -> f64 {
    let mult = timer_scale_multiplier(pins);
    let total_sec = pins[3]
        .trim()
        .parse::<f64>()
        .ok()
        .or_else(|| value.trim().parse::<f64>().ok())
        .unwrap_or(3.0);
    (total_sec / mult.factor()).max(0.1)
}

/// Extrai a escala multiplicadora de tempo de repouso/desligado (T_OFF) para temporizador cíclico (`pins[4]`).
pub fn timer_off_scale_multiplier(pins: &[String; 9]) -> TimerScaleMultiplier {
    TimerScaleMultiplier::from_code(&pins[4])
}

/// Extrai o tempo desligado efetivo (em milissegundos) de um temporizador cíclico (`pins[5]`).
pub fn timer_off_preset_ms(pins: &[String; 9]) -> u64 {
    let sec = pins[5].trim().parse::<f64>().ok().unwrap_or(1.0).max(0.01);
    (sec * 1000.0).round() as u64
}

/// Extrai o valor base de tempo desligado (T_OFF efetivo dividido pelo multiplicador).
pub fn timer_off_base_value(pins: &[String; 9]) -> f64 {
    let mult = timer_off_scale_multiplier(pins);
    let total_sec = pins[5].trim().parse::<f64>().ok().unwrap_or(1.0);
    (total_sec / mult.factor()).max(0.1)
}

/// Estado agregado dos temporizadores TON e TOF ativos no circuito por tag.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActiveTimers {
    pub ton_triggered: HashSet<String>,
    pub tof_triggered: HashSet<String>,
}

impl ActiveTimers {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Coleta o conjunto inicial de temporizadores TON e TOF que já se encontram disparados
/// antes do início do ciclo de resolução de cascata.
pub fn collect_initial_timers(items: &[SchematicItem], sim: &SimState) -> ActiveTimers {
    let mut timers = ActiveTimers::new();
    for item in items {
        let tag = item.tag.trim().to_uppercase();
        if tag.is_empty() {
            continue;
        }
        let s = sim.get(item.id);
        match item.tipo {
            9004 if item.subvariant == 2 => {
                let preset_on_ms = timer_preset_ms(&item.pins, &item.value);
                let preset_off_ms = timer_off_preset_ms(&item.pins);
                let total_cycle_ms = preset_on_ms.saturating_add(preset_off_ms).max(100);
                if s.active && (s.timer_elapsed_ms % total_cycle_ms) < preset_on_ms {
                    timers.ton_triggered.insert(tag);
                }
            }
            9004 => {
                let preset_ms = timer_preset_ms(&item.pins, &item.value);
                if s.active && s.timer_elapsed_ms >= preset_ms {
                    timers.ton_triggered.insert(tag);
                }
            }
            9005 if (s.active || s.timer_elapsed_ms > 0) => {
                timers.tof_triggered.insert(tag);
            }
            _ => {}
        }
    }
    timers
}

/// Avança os acumuladores de tempo dos temporizadores TON (`9004`), TOF (`9005`) e Cíclico (`subvariant 2`) em `dt_ms`.
/// Retorna `true` se houve alteração no conjunto de temporizadores disparados (necessitando nova cascata).
pub fn advance_timers(
    items: &[SchematicItem],
    sim: &mut SimState,
    dt_ms: u64,
    active_timers: &mut ActiveTimers,
) -> bool {
    if dt_ms == 0 {
        return false;
    }

    let mut new_ton = HashSet::new();
    let mut new_tof = HashSet::new();

    for item in items {
        let tag = item.tag.trim().to_uppercase();
        let s = sim.get_mut(item.id);
        match item.tipo {
            9004 if item.subvariant == 2 => {
                let preset_on_ms = timer_preset_ms(&item.pins, &item.value);
                let preset_off_ms = timer_off_preset_ms(&item.pins);
                let total_cycle_ms = preset_on_ms.saturating_add(preset_off_ms).max(100);
                if s.active {
                    s.timer_elapsed_ms = s.timer_elapsed_ms.saturating_add(dt_ms);
                    let cycle_pos = s.timer_elapsed_ms % total_cycle_ms;
                    if cycle_pos < preset_on_ms {
                        s.timer_phase = 0;
                        if !tag.is_empty() {
                            new_ton.insert(tag);
                        }
                    } else {
                        s.timer_phase = 1;
                    }
                } else {
                    s.timer_elapsed_ms = 0;
                    s.timer_phase = 0;
                }
            }
            9004 => {
                let preset_ms = timer_preset_ms(&item.pins, &item.value);
                if s.active {
                    s.timer_elapsed_ms = s.timer_elapsed_ms.saturating_add(dt_ms);
                } else {
                    s.timer_elapsed_ms = 0;
                }
                if s.active && s.timer_elapsed_ms >= preset_ms && !tag.is_empty() {
                    new_ton.insert(tag);
                }
            }
            9005 => {
                let preset_ms = timer_preset_ms(&item.pins, &item.value);
                if s.active {
                    s.timer_elapsed_ms = preset_ms;
                } else {
                    s.timer_elapsed_ms = s.timer_elapsed_ms.saturating_sub(dt_ms);
                }
                if (s.active || s.timer_elapsed_ms > 0) && !tag.is_empty() {
                    new_tof.insert(tag);
                }
            }
            _ => {}
        }
    }

    let changed = new_ton != active_timers.ton_triggered || new_tof != active_timers.tof_triggered;
    if changed {
        active_timers.ton_triggered = new_ton;
        active_timers.tof_triggered = new_tof;
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_preset_parsing() {
        let mut pins: [String; 9] = Default::default();
        pins[3] = "5.5".to_string();
        assert_eq!(timer_preset_ms(&pins, ""), 5500);

        pins[3] = "".to_string();
        assert_eq!(timer_preset_ms(&pins, "2.0"), 2000);

        // Fallback para 3 segundos se vazio ou inválido
        assert_eq!(timer_preset_ms(&pins, "abc"), 3000);
    }

    #[test]
    fn test_timer_scale_multipliers() {
        let mut pins: [String; 9] = Default::default();
        pins[3] = "50".to_string();

        // Escala x10 Seg com total de 50s -> base 5.0
        pins[2] = "1".to_string();
        assert_eq!(timer_preset_ms(&pins, ""), 50000);
        assert_eq!(timer_scale_multiplier(&pins), TimerScaleMultiplier::X10);
        assert!((timer_base_value(&pins, "") - 5.0).abs() < 1e-6);

        // Escala x60 Seg com total de 300s -> base 5.0
        pins[2] = "2".to_string();
        pins[3] = "300".to_string();
        assert_eq!(timer_preset_ms(&pins, ""), 300000);
        assert_eq!(timer_scale_multiplier(&pins), TimerScaleMultiplier::X60);
        assert!((timer_base_value(&pins, "") - 5.0).abs() < 1e-6);

        // Escala x0.1 Seg com total de 0.5s -> base 5.0
        pins[2] = "3".to_string();
        pins[3] = "0.5".to_string();
        assert_eq!(timer_preset_ms(&pins, ""), 500);
        assert_eq!(timer_scale_multiplier(&pins), TimerScaleMultiplier::X0_1);
        assert!((timer_base_value(&pins, "") - 5.0).abs() < 1e-6);
    }

    #[test]
    fn test_format_timer_seconds() {
        assert_eq!(format_timer_seconds(3.0), "3");
        assert_eq!(format_timer_seconds(8.5), "8.5");
        assert_eq!(format_timer_seconds(0.85), "0.85");
        assert_eq!(format_timer_seconds(0.5), "0.5");
        assert_eq!(format_timer_seconds(60.0), "60");
        assert_eq!(format_timer_seconds(0.05), "0.05");
    }

    #[test]
    fn test_cyclic_timer_advance_and_phase_toggling() {
        let mut item = SchematicItem::new_symbol(1, 9004, 0, 0, "-KT1");
        item.subvariant = 2; // Cíclico
        item.pins = [
            "A1".to_string(),
            "A2".to_string(),
            "0".to_string(), // T_ON mult: x1s
            "2".to_string(), // T_ON efetivo: 2s (2000ms)
            "0".to_string(), // T_OFF mult: x1s
            "1".to_string(), // T_OFF efetivo: 1s (1000ms)
            String::new(),
            String::new(),
            String::new(),
        ];
        let doc_items = vec![item];

        let mut sim = SimState::new();
        let mut active = ActiveTimers::new();

        // Energiza o temporizador
        sim.get_mut(1).active = true;

        // No início (t=500ms): dentro do T_ON (0..2000ms) -> disparado
        advance_timers(&doc_items, &mut sim, 500, &mut active);
        assert_eq!(sim.get(1).timer_phase, 0);
        assert!(active.ton_triggered.contains("-KT1"));

        // Avança até 1500ms total -> ainda no T_ON
        advance_timers(&doc_items, &mut sim, 1000, &mut active);
        assert_eq!(sim.get(1).timer_phase, 0);
        assert!(active.ton_triggered.contains("-KT1"));

        // Avança mais 600ms (total 2100ms) -> entrou no T_OFF (2000..3000ms)
        advance_timers(&doc_items, &mut sim, 600, &mut active);
        assert_eq!(sim.get(1).timer_phase, 1);
        assert!(!active.ton_triggered.contains("-KT1"));

        // Avança mais 1000ms (total 3100ms) -> reiniciou o ciclo (3100 % 3000 = 100ms) -> volta a T_ON
        advance_timers(&doc_items, &mut sim, 1000, &mut active);
        assert_eq!(sim.get(1).timer_phase, 0);
        assert!(active.ton_triggered.contains("-KT1"));
    }
}
