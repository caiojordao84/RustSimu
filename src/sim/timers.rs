use std::collections::HashSet;

use crate::model::{SchematicItem, SimState};

/// Extrai o tempo programado (em milissegundos) de um temporizador `9004` (TON) ou `9005` (TOF).
/// No formato `.cad` original, o tempo em segundos fica em `pins[3]` (ou `value` no inspetor).
pub fn timer_preset_ms(pins: &[String; 9], value: &str) -> u64 {
    let sec = pins[3]
        .trim()
        .parse::<f64>()
        .ok()
        .or_else(|| value.trim().parse::<f64>().ok())
        .unwrap_or(3.0)
        .max(0.1);
    (sec * 1000.0).round() as u64
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

/// Avança os acumuladores de tempo dos temporizadores TON (`9004`) e TOF (`9005`) em `dt_ms`.
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
}
