//! Built-in demo programs matching the reference screenshots.

use crate::plc::model::{
    Branch, FbdBlockType, InputKind, LadderElement, LadderInstr, PlcProgram, COLS,
};
use std::collections::HashMap;

/// Classic self-latching motor start/stop (Ladder + FBD).
///
/// Fail-safe Stop: physically NC push button (`I0.1` rests at 1). Ladder uses
/// XIC on that bit; FBD uses NOT into RS reset (reset-dominant).
pub fn motor_start_stop() -> (PlcProgram, HashMap<String, InputKind>) {
    let mut program = PlcProgram::default();
    let mut kinds = HashMap::new();

    program.active_preset = Some("motor-start-stop".into());
    program.input_labels = vec![
        "Start".into(),
        "Stop".into(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ];
    program.output_labels = vec![
        "Motor".into(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
        String::new(),
    ];
    kinds.insert("I0.1".into(), InputKind::MomNc);

    // Rung 0: (Start OR Seal-in) AND Stop-healthy -> Motor
    program.create_rung();
    program.rungs[0].comment =
        "Motor Start/Stop with seal-in (Stop: NC button, XIC instruction)".into();
    program.add_element(0, LadderInstr::ContactNo, 0, 0, "I0.0");
    program.add_element(0, LadderInstr::ContactNo, 2, 0, "I0.1");
    program.add_element(0, LadderInstr::Coil, COLS - 1, 0, "Q0.0");
    program.rungs[0].branches.push(Branch {
        start_col: 0,
        end_col: 1,
        elements: vec![LadderElement {
            instr: LadderInstr::ContactNo,
            col: 0,
            row: 1,
            address: "M0.0".into(),
            preset_ms: 0.0,
            state: false,
        }],
        in_power: false,
        power: false,
    });

    // Rung 1: Motor running -> Set seal-in
    program.create_rung();
    program.rungs[1].comment = "Seal-in memory bit".into();
    program.add_element(1, LadderInstr::ContactNo, 0, 0, "Q0.0");
    program.add_element(1, LadderInstr::CoilSet, COLS - 1, 0, "M0.0");

    // Rung 2: Stop pressed -> Reset seal-in
    program.create_rung();
    program.rungs[2].comment = "Stop pressed (bit=0) resets seal-in".into();
    program.add_element(2, LadderInstr::ContactNc, 0, 0, "I0.1");
    program.add_element(2, LadderInstr::CoilReset, COLS - 1, 0, "M0.0");

    // FBD: Start → RS.S; NOT(Stop) → RS.R; RS.Q → Motor
    let st = program.add_fbd_block(FbdBlockType::Input, 50.0, 100.0, Some("I0.0".into()));
    let sp = program.add_fbd_block(FbdBlockType::Input, 50.0, 260.0, Some("I0.1".into()));
    let inv = program.add_fbd_block(FbdBlockType::Not, 170.0, 260.0, None);
    let rs = program.add_fbd_block(FbdBlockType::Rs, 320.0, 170.0, Some("M0.0".into()));
    let out = program.add_fbd_block(FbdBlockType::Output, 510.0, 170.0, Some("Q0.0".into()));
    program.add_fbd_wire(st, "out", rs, "s");
    program.add_fbd_wire(sp, "out", inv, "in");
    program.add_fbd_wire(inv, "out", rs, "r");
    program.add_fbd_wire(rs, "q", out, "in");

    program.input_kinds = kinds.clone();
    (program, kinds)
}
