//! PLC program data model: ladder rungs, FBD graph, I/O address space.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const COLS: usize = 8;
pub const NUM_INPUTS: usize = 8;
pub const NUM_OUTPUTS: usize = 8;
pub const NUM_MEMORY: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum EditorMode {
    #[default]
    Ladder,
    Fbd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SimMode {
    #[default]
    Stop,
    Run,
    Fault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LadderInstr {
    ContactNo,
    ContactNc,
    ContactPos,
    ContactNeg,
    Coil,
    CoilSet,
    CoilReset,
    CoilNeg,
    Ton,
}

impl LadderInstr {
    pub fn label(self) -> &'static str {
        match self {
            Self::ContactNo => "NO Contact",
            Self::ContactNc => "NC Contact",
            Self::ContactPos => "Positive Edge",
            Self::ContactNeg => "Negative Edge",
            Self::Coil => "Output Coil",
            Self::CoilSet => "Set Coil (S)",
            Self::CoilReset => "Reset Coil (R)",
            Self::CoilNeg => "Negated Coil",
            Self::Ton => "TON Timer",
        }
    }

    pub fn symbol(self) -> &'static str {
        match self {
            Self::ContactNo => "| |",
            Self::ContactNc => "|/|",
            Self::ContactPos => "|P|",
            Self::ContactNeg => "|N|",
            Self::Coil => "( )",
            Self::CoilSet => "(S)",
            Self::CoilReset => "(R)",
            Self::CoilNeg => "(/)",
            Self::Ton => "TON",
        }
    }

    pub fn is_contact(self) -> bool {
        matches!(
            self,
            Self::ContactNo | Self::ContactNc | Self::ContactPos | Self::ContactNeg
        )
    }

    pub fn is_coil(self) -> bool {
        matches!(
            self,
            Self::Coil | Self::CoilSet | Self::CoilReset | Self::CoilNeg
        )
    }

    pub fn default_address(self) -> &'static str {
        match self {
            Self::ContactNo | Self::ContactPos | Self::ContactNeg => "I0.0",
            Self::ContactNc => "I0.1",
            Self::Coil | Self::CoilSet | Self::CoilReset | Self::CoilNeg => "Q0.0",
            Self::Ton => "T0",
        }
    }

    pub fn category(self) -> &'static str {
        match self {
            Self::ContactNo | Self::ContactNc | Self::ContactPos | Self::ContactNeg => "CONTACTS",
            Self::Coil | Self::CoilSet | Self::CoilReset | Self::CoilNeg => "COILS",
            Self::Ton => "TIMERS",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LadderElement {
    pub instr: LadderInstr,
    pub col: usize,
    pub row: usize,
    pub address: String,
    /// Timer preset in milliseconds (TON).
    #[serde(default)]
    pub preset_ms: f64,
    #[serde(skip)]
    pub state: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    pub start_col: usize,
    pub end_col: usize,
    pub elements: Vec<LadderElement>,
    #[serde(skip)]
    pub in_power: bool,
    #[serde(skip)]
    pub power: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rung {
    pub id: u32,
    pub comment: String,
    pub elements: Vec<LadderElement>,
    pub branches: Vec<Branch>,
    #[serde(skip)]
    pub output_power: bool,
    #[serde(skip)]
    pub seg_power: Vec<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FbdBlockType {
    Input,
    Output,
    Not,
    And,
    Or,
    Rs,
    Sr,
    Ton,
}

impl FbdBlockType {
    pub fn label(self) -> &'static str {
        match self {
            Self::Input => "Input Var",
            Self::Output => "Output Var",
            Self::Not => "NOT",
            Self::And => "AND",
            Self::Or => "OR",
            Self::Rs => "RS",
            Self::Sr => "SR",
            Self::Ton => "TON",
        }
    }

    pub fn category(self) -> &'static str {
        match self {
            Self::Input | Self::Output => "I/O",
            Self::Not | Self::And | Self::Or => "LOGIC",
            Self::Rs | Self::Sr => "LATCH",
            Self::Ton => "TIMERS",
        }
    }

    pub fn size(self) -> (f32, f32) {
        match self {
            Self::Input | Self::Output | Self::Not => (80.0, 48.0),
            Self::And | Self::Or => (90.0, 60.0),
            Self::Rs | Self::Sr => (100.0, 60.0),
            Self::Ton => (110.0, 70.0),
        }
    }

    pub fn inputs(self) -> &'static [&'static str] {
        match self {
            Self::Input => &[],
            Self::Output | Self::Not => &["in"],
            Self::And | Self::Or => &["in1", "in2"],
            Self::Rs | Self::Sr => &["s", "r"],
            Self::Ton => &["en"],
        }
    }

    pub fn outputs(self) -> &'static [&'static str] {
        match self {
            Self::Input => &["out"],
            Self::Output => &[],
            Self::Not | Self::And | Self::Or => &["out"],
            Self::Rs | Self::Sr => &["q"],
            Self::Ton => &["q"],
        }
    }

    pub fn default_address(self) -> &'static str {
        match self {
            Self::Input => "I0.0",
            Self::Output => "Q0.0",
            Self::Rs | Self::Sr => "M0.0",
            Self::Ton => "T0",
            _ => "",
        }
    }

    pub fn needs_address(self) -> bool {
        matches!(
            self,
            Self::Input | Self::Output | Self::Rs | Self::Sr | Self::Ton
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FbdBlock {
    pub id: u32,
    pub kind: FbdBlockType,
    pub x: f32,
    pub y: f32,
    pub address: String,
    #[serde(default)]
    pub preset_ms: f64,
    #[serde(skip)]
    pub in_ports: HashMap<String, bool>,
    #[serde(skip)]
    pub out_ports: HashMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FbdPortRef {
    pub block_id: u32,
    pub port: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FbdWire {
    pub id: u32,
    pub from: FbdPortRef,
    pub to: FbdPortRef,
    #[serde(skip)]
    pub powered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum InputKind {
    #[default]
    Toggle,
    MomNo,
    MomNc,
}

impl InputKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Toggle => "Switch",
            Self::MomNo => "NO",
            Self::MomNc => "NC",
        }
    }

    pub fn next(self) -> Self {
        match self {
            Self::Toggle => Self::MomNo,
            Self::MomNo => Self::MomNc,
            Self::MomNc => Self::Toggle,
        }
    }

    pub fn resting_value(self) -> bool {
        matches!(self, Self::MomNc)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimerState {
    pub en: bool,
    pub tt: bool,
    pub dn: bool,
    pub pre: f64,
    pub acc: f64,
    pub start_time: Option<f64>,
}

impl Default for TimerState {
    fn default() -> Self {
        Self {
            en: false,
            tt: false,
            dn: false,
            pre: 3000.0,
            acc: 0.0,
            start_time: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct SimState {
    pub mode: SimMode,
    pub inputs: HashMap<String, bool>,
    pub outputs: HashMap<String, bool>,
    pub memory: HashMap<String, bool>,
    pub timers: HashMap<String, TimerState>,
    pub input_snapshot: HashMap<String, bool>,
    pub prev_values: HashMap<String, bool>,
    pub pre_prev_values: HashMap<String, bool>,
    pub scan_active: bool,
    pub cycle_count: u64,
    pub scan_time_ms: f64,
    pub sim_time: f64,
}

impl Default for SimState {
    fn default() -> Self {
        let mut inputs = HashMap::new();
        let mut outputs = HashMap::new();
        let mut memory = HashMap::new();
        for i in 0..NUM_INPUTS {
            inputs.insert(format!("I0.{i}"), false);
            outputs.insert(format!("Q0.{i}"), false);
        }
        for m in 0..NUM_MEMORY {
            memory.insert(format!("M0.{m}"), false);
        }
        Self {
            mode: SimMode::Stop,
            inputs,
            outputs,
            memory,
            timers: HashMap::new(),
            input_snapshot: HashMap::new(),
            prev_values: HashMap::new(),
            pre_prev_values: HashMap::new(),
            scan_active: false,
            cycle_count: 0,
            scan_time_ms: 0.0,
            sim_time: 0.0,
        }
    }
}

impl SimState {
    pub fn reset_runtime(&mut self, input_kinds: &HashMap<String, InputKind>) {
        for (addr, v) in self.inputs.iter_mut() {
            let kind = input_kinds.get(addr).copied().unwrap_or_default();
            *v = kind.resting_value();
        }
        for v in self.outputs.values_mut() {
            *v = false;
        }
        for v in self.memory.values_mut() {
            *v = false;
        }
        self.timers.clear();
        self.input_snapshot.clear();
        self.prev_values.clear();
        self.pre_prev_values.clear();
        self.cycle_count = 0;
        self.scan_time_ms = 0.0;
        self.sim_time = 0.0;
        self.mode = SimMode::Stop;
        self.scan_active = false;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlcProgram {
    pub rungs: Vec<Rung>,
    pub fbd_blocks: Vec<FbdBlock>,
    pub fbd_wires: Vec<FbdWire>,
    pub input_labels: Vec<String>,
    pub output_labels: Vec<String>,
    pub input_kinds: HashMap<String, InputKind>,
    pub next_rung_id: u32,
    pub next_fbd_block_id: u32,
    pub next_fbd_wire_id: u32,
    pub active_preset: Option<String>,
}

impl Default for PlcProgram {
    fn default() -> Self {
        Self {
            rungs: Vec::new(),
            fbd_blocks: Vec::new(),
            fbd_wires: Vec::new(),
            input_labels: vec![String::new(); NUM_INPUTS],
            output_labels: vec![String::new(); NUM_OUTPUTS],
            input_kinds: HashMap::new(),
            next_rung_id: 1,
            next_fbd_block_id: 1,
            next_fbd_wire_id: 1,
            active_preset: None,
        }
    }
}

impl PlcProgram {
    pub fn create_rung(&mut self) -> &mut Rung {
        let id = self.next_rung_id;
        self.next_rung_id += 1;
        self.rungs.push(Rung {
            id,
            comment: String::new(),
            elements: Vec::new(),
            branches: Vec::new(),
            output_power: false,
            seg_power: Vec::new(),
        });
        self.rungs.last_mut().unwrap()
    }

    pub fn add_element(
        &mut self,
        rung_idx: usize,
        instr: LadderInstr,
        mut col: usize,
        mut row: usize,
        address: impl Into<String>,
    ) -> bool {
        let Some(rung) = self.rungs.get_mut(rung_idx) else {
            return false;
        };
        if instr.is_coil() {
            col = COLS - 1;
            row = 0;
        } else if instr.is_contact() && col >= COLS - 1 {
            col = COLS - 2;
        }
        if row > 0 && !instr.is_contact() {
            return false;
        }
        // Occupied cell?
        if rung.elements.iter().any(|e| e.col == col && e.row == row) {
            return false;
        }
        for br in &rung.branches {
            if br.elements.iter().any(|e| e.col == col && e.row == row) {
                return false;
            }
        }
        rung.elements.push(LadderElement {
            instr,
            col,
            row,
            address: address.into(),
            preset_ms: 3000.0,
            state: false,
        });
        true
    }

    pub fn add_fbd_block(
        &mut self,
        kind: FbdBlockType,
        x: f32,
        y: f32,
        address: Option<String>,
    ) -> u32 {
        let id = self.next_fbd_block_id;
        self.next_fbd_block_id += 1;
        let address = address.unwrap_or_else(|| kind.default_address().to_string());
        self.fbd_blocks.push(FbdBlock {
            id,
            kind,
            x,
            y,
            address,
            preset_ms: 3000.0,
            in_ports: HashMap::new(),
            out_ports: HashMap::new(),
        });
        id
    }

    pub fn add_fbd_wire(
        &mut self,
        from_block: u32,
        from_port: impl Into<String>,
        to_block: u32,
        to_port: impl Into<String>,
    ) -> Option<u32> {
        let to_port = to_port.into();
        // One wire per destination port.
        self.fbd_wires
            .retain(|w| !(w.to.block_id == to_block && w.to.port == to_port));
        let id = self.next_fbd_wire_id;
        self.next_fbd_wire_id += 1;
        self.fbd_wires.push(FbdWire {
            id,
            from: FbdPortRef {
                block_id: from_block,
                port: from_port.into(),
            },
            to: FbdPortRef {
                block_id: to_block,
                port: to_port,
            },
            powered: false,
        });
        Some(id)
    }

    pub fn find_block(&self, id: u32) -> Option<&FbdBlock> {
        self.fbd_blocks.iter().find(|b| b.id == id)
    }

    pub fn find_block_mut(&mut self, id: u32) -> Option<&mut FbdBlock> {
        self.fbd_blocks.iter_mut().find(|b| b.id == id)
    }

    pub fn remove_selected_ladder(&mut self, selection: Option<(usize, SelectionTarget)>) -> bool {
        let Some((rung_idx, target)) = selection else {
            return false;
        };
        let Some(rung) = self.rungs.get_mut(rung_idx) else {
            return false;
        };
        match target {
            SelectionTarget::Element(i) => {
                if i < rung.elements.len() {
                    rung.elements.remove(i);
                    return true;
                }
            }
            SelectionTarget::BranchElement { branch, index } => {
                if let Some(br) = rung.branches.get_mut(branch) {
                    if index < br.elements.len() {
                        br.elements.remove(index);
                        return true;
                    }
                }
            }
            SelectionTarget::Rung => {
                self.rungs.remove(rung_idx);
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionTarget {
    Element(usize),
    BranchElement { branch: usize, index: usize },
    Rung,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selection {
    Ladder {
        rung: usize,
        target: SelectionTarget,
    },
    FbdBlock(u32),
    FbdWire(u32),
}

/// Palette item the user is about to place.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteSelection {
    Ladder(LadderInstr),
    Fbd(FbdBlockType),
}

pub fn ladder_palette() -> Vec<(&'static str, Vec<LadderInstr>)> {
    vec![
        (
            "CONTACTS",
            vec![
                LadderInstr::ContactNo,
                LadderInstr::ContactNc,
                LadderInstr::ContactPos,
                LadderInstr::ContactNeg,
            ],
        ),
        (
            "COILS",
            vec![
                LadderInstr::Coil,
                LadderInstr::CoilSet,
                LadderInstr::CoilReset,
                LadderInstr::CoilNeg,
            ],
        ),
        ("TIMERS", vec![LadderInstr::Ton]),
    ]
}

pub fn fbd_palette() -> Vec<(&'static str, Vec<FbdBlockType>)> {
    vec![
        ("I/O", vec![FbdBlockType::Input, FbdBlockType::Output]),
        (
            "LOGIC",
            vec![FbdBlockType::Not, FbdBlockType::And, FbdBlockType::Or],
        ),
        ("LATCH", vec![FbdBlockType::Rs, FbdBlockType::Sr]),
        ("TIMERS", vec![FbdBlockType::Ton]),
    ]
}
