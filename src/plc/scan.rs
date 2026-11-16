//! PLC scan cycle: ladder evaluation + FBD fixpoint engine.

use crate::plc::model::{
    Branch, EditorMode, FbdBlock, FbdBlockType, LadderElement, LadderInstr, PlcProgram, SimMode,
    SimState, COLS,
};
use std::collections::{HashMap, HashSet};
use std::time::Instant;

fn get_address(sim: &SimState, addr: &str) -> bool {
    if addr.is_empty() {
        return false;
    }
    let ch = addr.chars().next().unwrap_or('\0');
    match ch {
        'I' => {
            if sim.scan_active {
                if let Some(v) = sim.input_snapshot.get(addr) {
                    return *v;
                }
            }
            *sim.inputs.get(addr).unwrap_or(&false)
        }
        'Q' => *sim.outputs.get(addr).unwrap_or(&false),
        'M' => *sim.memory.get(addr).unwrap_or(&false),
        'T' => {
            let (base, bit) = split_timer_addr(addr);
            let Some(t) = sim.timers.get(base) else {
                return false;
            };
            match bit {
                "TT" => t.tt,
                "EN" => t.en,
                _ => t.dn, // DN or bare T0
            }
        }
        _ => false,
    }
}

fn set_address(sim: &mut SimState, addr: &str, value: bool) {
    if addr.is_empty() {
        return;
    }
    match addr.chars().next().unwrap_or('\0') {
        'Q' => {
            sim.outputs.insert(addr.to_string(), value);
        }
        'M' => {
            sim.memory.insert(addr.to_string(), value);
        }
        _ => {}
    }
}

fn split_timer_addr(addr: &str) -> (&str, &str) {
    if let Some((base, bit)) = addr.split_once('.') {
        (base, bit)
    } else {
        (addr, "DN")
    }
}

fn edge_key(addr: &str) -> String {
    if addr.starts_with('T') && !addr.contains('.') {
        format!("{addr}.DN")
    } else {
        addr.to_string()
    }
}

fn evaluate_element(sim: &SimState, el: &LadderElement) -> bool {
    let addr = el.address.as_str();
    match el.instr {
        LadderInstr::ContactNo => get_address(sim, addr),
        LadderInstr::ContactNc => !get_address(sim, addr),
        LadderInstr::ContactPos => {
            let key = edge_key(addr);
            let cur = sim
                .prev_values
                .get(&key)
                .copied()
                .unwrap_or_else(|| get_address(sim, addr));
            let prev = sim.pre_prev_values.get(&key).copied().unwrap_or(false);
            cur && !prev
        }
        LadderInstr::ContactNeg => {
            let key = edge_key(addr);
            let cur = sim
                .prev_values
                .get(&key)
                .copied()
                .unwrap_or_else(|| get_address(sim, addr));
            let prev = sim.pre_prev_values.get(&key).copied().unwrap_or(false);
            !cur && prev
        }
        _ => false,
    }
}

fn execute_output(sim: &mut SimState, el: &LadderElement, power: bool) {
    let addr = el.address.as_str();
    match el.instr {
        LadderInstr::Coil => set_address(sim, addr, power),
        LadderInstr::CoilSet => {
            if power {
                set_address(sim, addr, true);
            }
        }
        LadderInstr::CoilReset => {
            if power {
                set_address(sim, addr, false);
            }
        }
        LadderInstr::CoilNeg => set_address(sim, addr, !power),
        _ => {}
    }
}

fn update_timer(sim: &mut SimState, el: &LadderElement, input_power: bool) -> bool {
    let addr = el.address.clone();
    let pre = if el.preset_ms.is_finite() && el.preset_ms >= 0.0 {
        el.preset_ms
    } else {
        3000.0
    };
    let t = sim.timers.entry(addr).or_default();
    t.pre = pre;
    t.en = input_power;

    if el.instr == LadderInstr::Ton {
        if input_power {
            if t.start_time.is_none() {
                t.start_time = Some(sim.sim_time);
            }
            let start = t.start_time.unwrap_or(sim.sim_time);
            t.acc = ((sim.sim_time - start) * 1000.0).min(t.pre);
            t.dn = t.acc >= t.pre;
            t.tt = !t.dn;
        } else {
            t.start_time = None;
            t.acc = 0.0;
            t.tt = false;
            t.dn = false;
        }
    }
    t.dn
}

fn power_through(sim: &mut SimState, el: &mut LadderElement, p: bool) -> bool {
    if el.instr == LadderInstr::Ton {
        let dn = update_timer(sim, el, p);
        el.state = dn;
        return dn;
    }
    let r = evaluate_element(sim, el);
    el.state = r;
    p && r
}

fn evaluate_rung(
    sim: &mut SimState,
    rung_elements: &mut [LadderElement],
    branches: &mut [Branch],
) -> (bool, Vec<bool>) {
    let mut coil_idxs = Vec::new();
    let mut series_idxs = Vec::new();
    let mut sorted: Vec<usize> = (0..rung_elements.len()).collect();
    sorted.sort_by_key(|&i| rung_elements[i].col);

    for &i in &sorted {
        let el = &rung_elements[i];
        if el.instr.is_coil() {
            coil_idxs.push(i);
        } else if el.row == 0 && (el.instr.is_contact() || el.instr == LadderInstr::Ton) {
            series_idxs.push(i);
        }
    }

    // Pre-evaluate branches
    let mut branch_results: Vec<(usize, usize, bool)> = Vec::new();
    for br in branches.iter_mut() {
        let mut bels: Vec<usize> = (0..br.elements.len()).collect();
        bels.sort_by_key(|&i| br.elements[i].col);
        let mut branch_power = true;
        for &bi in &bels {
            branch_power = power_through(sim, &mut br.elements[bi], branch_power);
        }
        br.power = branch_power;
        branch_results.push((br.start_col, br.end_col, branch_power));
    }
    branch_results.sort_by_key(|(s, _, _)| *s);

    let mut power = true;
    let mut b_idx = 0;
    let mut col = 0;
    let mut seg_power = vec![true];

    while col < COLS {
        if b_idx < branch_results.len() && branch_results[b_idx].0 == col {
            let (start, end, br_power) = branch_results[b_idx];
            if let Some(br) = branches
                .iter_mut()
                .find(|b| b.start_col == start && b.end_col == end)
            {
                br.in_power = power;
            }
            let mut main_acc = true;
            for sc in start..=end {
                for &si in &series_idxs {
                    if rung_elements[si].col == sc {
                        let p_in = power && main_acc;
                        main_acc = power_through(sim, &mut rung_elements[si], p_in);
                    }
                }
                let sp = if sc < end {
                    power && main_acc
                } else {
                    power && (main_acc || br_power)
                };
                if seg_power.len() <= sc + 1 {
                    seg_power.resize(sc + 2, false);
                }
                seg_power[sc + 1] = sp;
            }
            power = power && (main_acc || br_power);
            col = end + 1;
            b_idx += 1;
        } else {
            for &sj in &series_idxs {
                if rung_elements[sj].col == col {
                    power = power_through(sim, &mut rung_elements[sj], power);
                }
            }
            if seg_power.len() <= col + 1 {
                seg_power.resize(col + 2, false);
            }
            seg_power[col + 1] = power;
            col += 1;
        }
    }

    for &ci in &coil_idxs {
        rung_elements[ci].state = power;
        // execute after marking state — need owned address/instr
    }
    // Execute coils (second pass to satisfy borrow checker cleanly)
    let coil_snapshot: Vec<(LadderInstr, String, bool)> = coil_idxs
        .iter()
        .map(|&ci| {
            (
                rung_elements[ci].instr,
                rung_elements[ci].address.clone(),
                power,
            )
        })
        .collect();
    for (instr, address, pwr) in coil_snapshot {
        let el = LadderElement {
            instr,
            col: 0,
            row: 0,
            address,
            preset_ms: 0.0,
            state: pwr,
        };
        execute_output(sim, &el, pwr);
    }

    (power, seg_power)
}

fn evaluate_ladder(program: &mut PlcProgram, sim: &mut SimState) {
    for rung in &mut program.rungs {
        let (power, seg) = evaluate_rung(sim, &mut rung.elements, &mut rung.branches);
        rung.output_power = power;
        rung.seg_power = seg;
    }
}

fn find_wire_to<'a>(
    wires: &'a [crate::plc::model::FbdWire],
    block_id: u32,
    port: &str,
) -> Option<&'a crate::plc::model::FbdWire> {
    wires
        .iter()
        .find(|w| w.to.block_id == block_id && w.to.port == port)
}

fn eval_fbd_block(
    sim: &SimState,
    blk: &FbdBlock,
    ins: &HashMap<String, bool>,
) -> HashMap<String, bool> {
    let mut out = HashMap::new();
    match blk.kind {
        FbdBlockType::Not => {
            out.insert("out".into(), !ins.get("in").copied().unwrap_or(false));
        }
        FbdBlockType::And => {
            out.insert(
                "out".into(),
                ins.get("in1").copied().unwrap_or(false)
                    && ins.get("in2").copied().unwrap_or(false),
            );
        }
        FbdBlockType::Or => {
            out.insert(
                "out".into(),
                ins.get("in1").copied().unwrap_or(false)
                    || ins.get("in2").copied().unwrap_or(false),
            );
        }
        FbdBlockType::Rs | FbdBlockType::Sr => {
            // During fixpoint, return stored memory (commit writes later).
            out.insert("q".into(), get_address(sim, &blk.address));
        }
        FbdBlockType::Ton => {
            out.insert(
                "q".into(),
                sim.timers.get(&blk.address).map(|t| t.dn).unwrap_or(false),
            );
        }
        FbdBlockType::Input | FbdBlockType::Output => {}
    }
    out
}

fn commit_fbd_block(sim: &mut SimState, blk: &FbdBlock, ins: &HashMap<String, bool>) {
    match blk.kind {
        FbdBlockType::Rs => {
            // Reset-dominant
            if ins.get("r").copied().unwrap_or(false) {
                set_address(sim, &blk.address, false);
            } else if ins.get("s").copied().unwrap_or(false) {
                set_address(sim, &blk.address, true);
            }
        }
        FbdBlockType::Sr => {
            // Set-dominant
            if ins.get("s").copied().unwrap_or(false) {
                set_address(sim, &blk.address, true);
            } else if ins.get("r").copied().unwrap_or(false) {
                set_address(sim, &blk.address, false);
            }
        }
        FbdBlockType::Ton => {
            let el = LadderElement {
                instr: LadderInstr::Ton,
                col: 0,
                row: 0,
                address: blk.address.clone(),
                preset_ms: blk.preset_ms,
                state: false,
            };
            let en = ins.get("en").copied().unwrap_or(false);
            update_timer(sim, &el, en);
        }
        _ => {}
    }
}

fn evaluate_fbd(program: &mut PlcProgram, sim: &mut SimState) {
    // Cull orphan wires
    let live: HashSet<u32> = program.fbd_blocks.iter().map(|b| b.id).collect();
    program
        .fbd_wires
        .retain(|w| live.contains(&w.from.block_id) && live.contains(&w.to.block_id));

    for w in &mut program.fbd_wires {
        w.powered = false;
    }

    // Seed inputs
    for b in &mut program.fbd_blocks {
        if b.kind == FbdBlockType::Input {
            b.out_ports
                .insert("out".into(), get_address(sim, &b.address));
        }
    }

    const MAX_PASSES: usize = 20;
    for _ in 0..MAX_PASSES {
        let mut changed = false;
        let block_ids: Vec<u32> = program.fbd_blocks.iter().map(|b| b.id).collect();
        for id in block_ids {
            let kind = match program.find_block(id) {
                Some(b) => b.kind,
                None => continue,
            };
            if matches!(kind, FbdBlockType::Input | FbdBlockType::Output) {
                continue;
            }

            let mut ins = HashMap::new();
            for port in kind.inputs() {
                let val = find_wire_to(&program.fbd_wires, id, port)
                    .and_then(|w| {
                        program
                            .find_block(w.from.block_id)
                            .and_then(|src| src.out_ports.get(&w.from.port).copied())
                    })
                    .unwrap_or(false);
                ins.insert((*port).to_string(), val);
            }

            let blk_snapshot = program.find_block(id).cloned().unwrap();
            let new_out = eval_fbd_block(sim, &blk_snapshot, &ins);

            if let Some(blk) = program.find_block_mut(id) {
                blk.in_ports = ins;
                for (k, v) in new_out {
                    if blk.out_ports.get(&k) != Some(&v) {
                        blk.out_ports.insert(k, v);
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }

    // Commit latches / timers
    let commits: Vec<(u32, HashMap<String, bool>)> = program
        .fbd_blocks
        .iter()
        .map(|b| (b.id, b.in_ports.clone()))
        .collect();
    for (id, ins) in commits {
        if let Some(blk) = program.find_block(id).cloned() {
            commit_fbd_block(sim, &blk, &ins);
            // Refresh latch/timer Q after commit
            if matches!(
                blk.kind,
                FbdBlockType::Rs | FbdBlockType::Sr | FbdBlockType::Ton
            ) {
                if let Some(b) = program.find_block_mut(id) {
                    let q = match b.kind {
                        FbdBlockType::Ton => {
                            sim.timers.get(&b.address).map(|t| t.dn).unwrap_or(false)
                        }
                        _ => get_address(sim, &b.address),
                    };
                    b.out_ports.insert("q".into(), q);
                }
            }
        }
    }

    // Commit outputs
    let output_ids: Vec<u32> = program
        .fbd_blocks
        .iter()
        .filter(|b| b.kind == FbdBlockType::Output)
        .map(|b| b.id)
        .collect();
    for id in output_ids {
        let ov = find_wire_to(&program.fbd_wires, id, "in")
            .and_then(|w| {
                program
                    .find_block(w.from.block_id)
                    .and_then(|src| src.out_ports.get(&w.from.port).copied())
            })
            .unwrap_or(false);
        if let Some(ob) = program.find_block_mut(id) {
            ob.in_ports.insert("in".into(), ov);
            let addr = ob.address.clone();
            set_address(sim, &addr, ov);
        }
    }

    // Mark wire power (snapshot outputs first to avoid borrow clashes)
    let outs: HashMap<(u32, String), bool> = program
        .fbd_blocks
        .iter()
        .flat_map(|b| {
            b.out_ports
                .iter()
                .map(|(p, v)| ((b.id, p.clone()), *v))
                .collect::<Vec<_>>()
        })
        .collect();
    for w in &mut program.fbd_wires {
        w.powered = outs
            .get(&(w.from.block_id, w.from.port.clone()))
            .copied()
            .unwrap_or(false);
    }
}

/// Run one PLC scan cycle for the active editor mode.
pub fn scan_cycle(program: &mut PlcProgram, sim: &mut SimState, mode: EditorMode, dt: f64) {
    if sim.mode == SimMode::Fault {
        return;
    }
    let start = Instant::now();
    sim.sim_time += dt;

    // Roll edge history
    sim.pre_prev_values = std::mem::take(&mut sim.prev_values);
    let mut prev = HashMap::new();
    for (k, v) in &sim.inputs {
        prev.insert(k.clone(), *v);
    }
    for (k, v) in &sim.outputs {
        prev.insert(k.clone(), *v);
    }
    for (k, v) in &sim.memory {
        prev.insert(k.clone(), *v);
    }
    for (k, t) in &sim.timers {
        prev.insert(format!("{k}.DN"), t.dn);
        prev.insert(format!("{k}.TT"), t.tt);
        prev.insert(format!("{k}.EN"), t.en);
    }
    sim.prev_values = prev;

    sim.input_snapshot = sim.inputs.clone();
    sim.scan_active = true;

    match mode {
        EditorMode::Fbd => evaluate_fbd(program, sim),
        EditorMode::Ladder => evaluate_ladder(program, sim),
    }

    sim.scan_active = false;
    sim.cycle_count = sim.cycle_count.saturating_add(1);
    sim.scan_time_ms = start.elapsed().as_secs_f64() * 1000.0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plc::demo::motor_start_stop;

    #[test]
    fn motor_seal_in_ladder() {
        let (mut program, mut kinds) = motor_start_stop();
        let mut sim = SimState::default();
        sim.reset_runtime(&kinds);
        // Stop NC rests ON
        assert!(*sim.inputs.get("I0.1").unwrap());

        // Press Start
        sim.inputs.insert("I0.0".into(), true);
        scan_cycle(&mut program, &mut sim, EditorMode::Ladder, 0.01);
        assert!(*sim.outputs.get("Q0.0").unwrap(), "motor should start");
        assert!(*sim.memory.get("M0.0").unwrap(), "seal-in should set");

        // Release Start — seal-in holds
        sim.inputs.insert("I0.0".into(), false);
        scan_cycle(&mut program, &mut sim, EditorMode::Ladder, 0.01);
        assert!(
            *sim.outputs.get("Q0.0").unwrap(),
            "motor should stay running via seal-in"
        );

        // Press Stop (mom-nc → bit drops)
        sim.inputs.insert("I0.1".into(), false);
        scan_cycle(&mut program, &mut sim, EditorMode::Ladder, 0.01);
        assert!(!*sim.outputs.get("Q0.0").unwrap(), "motor should stop");
        assert!(!*sim.memory.get("M0.0").unwrap(), "seal-in should reset");
        let _ = &mut kinds;
    }

    #[test]
    fn motor_seal_in_fbd() {
        let (mut program, kinds) = motor_start_stop();
        let mut sim = SimState::default();
        sim.reset_runtime(&kinds);

        sim.inputs.insert("I0.0".into(), true);
        scan_cycle(&mut program, &mut sim, EditorMode::Fbd, 0.01);
        assert!(*sim.outputs.get("Q0.0").unwrap());

        sim.inputs.insert("I0.0".into(), false);
        scan_cycle(&mut program, &mut sim, EditorMode::Fbd, 0.01);
        assert!(*sim.outputs.get("Q0.0").unwrap(), "RS latch holds");

        sim.inputs.insert("I0.1".into(), false); // stop pressed
        scan_cycle(&mut program, &mut sim, EditorMode::Fbd, 0.01);
        assert!(!*sim.outputs.get("Q0.0").unwrap());
    }
}
