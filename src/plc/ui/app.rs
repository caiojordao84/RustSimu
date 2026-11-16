//! Top-level iced application for the PLC Ladder / FBD simulator.

use crate::plc::demo::motor_start_stop;
use crate::plc::model::{
    fbd_palette, ladder_palette, EditorMode, FbdBlockType, InputKind, LadderInstr,
    PaletteSelection, PlcProgram, Selection, SelectionTarget, SimMode, SimState,
};
use crate::plc::scan::scan_cycle;
use crate::plc::theme::{self, ACCENT, BG, BORDER, GOLD, GREEN, SURFACE, SURFACE2, TEXT, TEXT_DIM};
use crate::plc::ui::canvas::{CanvasMsg, PlcCanvas};
use iced::widget::{
    button, canvas, column, container, mouse_area, row, scrollable, text, text_input, Space, Stack,
};
use iced::{
    Alignment, Background, Border, Color, Element, Length, Subscription, Task, Theme, Vector,
};
use std::path::PathBuf;
use std::time::Duration;

const SCAN_HZ: u64 = 50; // 20 ms scan period while running

#[derive(Debug, Clone)]
pub enum Message {
    Tick,
    SetMode(EditorMode),
    Run,
    Stop,
    Step,
    Undo,
    Delete,
    Clear,
    AddRung,
    ZoomIn,
    ZoomOut,
    TogglePan,
    LoadDemo,
    Save,
    Load,
    PaletteLadder(LadderInstr),
    PaletteFbd(FbdBlockType),
    TogglePaletteCat(&'static str),
    Canvas(CanvasMsg),
    CycleInputKind(usize),
    InputPress(usize),
    InputRelease(usize),
    AddressEdited(String),
    ApplyAddress,
}

pub struct PlcSimuApp {
    pub program: PlcProgram,
    pub sim: SimState,
    pub mode: EditorMode,
    pub selection: Option<Selection>,
    pub palette_sel: Option<PaletteSelection>,
    pub collapsed_cats: Vec<&'static str>,
    pub scale: f32,
    pub offset: Vector,
    pub pan_tool: bool,
    pub wire_draft: Option<(u32, String)>,
    pub cursor: Option<iced::Point>,
    pub undo_stack: Vec<PlcProgram>,
    pub address_draft: String,
    pub status_hint: String,
}

impl PlcSimuApp {
    pub fn new() -> (Self, Task<Message>) {
        let (program, _) = motor_start_stop();
        let mut sim = SimState::default();
        sim.reset_runtime(&program.input_kinds);
        (
            Self {
                program,
                sim,
                mode: EditorMode::Ladder,
                selection: None,
                palette_sel: None,
                collapsed_cats: vec!["TIMERS", "COUNTERS", "COMPARE", "MATH", "STRUCTURE"],
                scale: 1.0,
                offset: Vector::new(10.0, 10.0),
                pan_tool: false,
                wire_draft: None,
                cursor: None,
                undo_stack: Vec::new(),
                address_draft: String::new(),
                status_hint: "STOP — edit mode — press Run. Toggle inputs in the I/O panel.".into(),
            },
            Task::none(),
        )
    }

    pub fn title(&self) -> String {
        "RustSimu PLC — Ladder / FBD Simulator".into()
    }

    pub fn theme(&self) -> Theme {
        Theme::custom(
            "plc-dark".into(),
            iced::theme::Palette {
                background: BG,
                text: TEXT,
                primary: ACCENT,
                success: GREEN,
                danger: theme::RED,
            },
        )
    }

    pub fn subscription(&self) -> Subscription<Message> {
        if self.sim.mode == SimMode::Run {
            iced::time::every(Duration::from_millis(1000 / SCAN_HZ)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        }
    }

    fn push_undo(&mut self) {
        self.undo_stack.push(self.program.clone());
        if self.undo_stack.len() > 64 {
            self.undo_stack.remove(0);
        }
    }

    fn selected_address_mut(&mut self) -> Option<&mut String> {
        match self.selection.clone() {
            Some(Selection::Ladder { rung, target }) => {
                let rung = self.program.rungs.get_mut(rung)?;
                match target {
                    SelectionTarget::Element(i) => Some(&mut rung.elements.get_mut(i)?.address),
                    SelectionTarget::BranchElement { branch, index } => Some(
                        &mut rung
                            .branches
                            .get_mut(branch)?
                            .elements
                            .get_mut(index)?
                            .address,
                    ),
                    SelectionTarget::Rung => None,
                }
            }
            Some(Selection::FbdBlock(id)) => {
                let b = self.program.find_block_mut(id)?;
                if b.kind.needs_address() {
                    Some(&mut b.address)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn sync_address_draft(&mut self) {
        self.address_draft = match &self.selection {
            Some(Selection::Ladder { rung, target }) => {
                let Some(rung) = self.program.rungs.get(*rung) else {
                    return;
                };
                match target {
                    SelectionTarget::Element(i) => rung
                        .elements
                        .get(*i)
                        .map(|e| e.address.clone())
                        .unwrap_or_default(),
                    SelectionTarget::BranchElement { branch, index } => rung
                        .branches
                        .get(*branch)
                        .and_then(|b| b.elements.get(*index))
                        .map(|e| e.address.clone())
                        .unwrap_or_default(),
                    SelectionTarget::Rung => String::new(),
                }
            }
            Some(Selection::FbdBlock(id)) => self
                .program
                .find_block(*id)
                .map(|b| b.address.clone())
                .unwrap_or_default(),
            _ => String::new(),
        };
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Tick => {
                scan_cycle(
                    &mut self.program,
                    &mut self.sim,
                    self.mode,
                    1.0 / SCAN_HZ as f64,
                );
            }
            Message::SetMode(m) => {
                self.mode = m;
                self.palette_sel = None;
                self.wire_draft = None;
                self.selection = None;
                self.status_hint = match m {
                    EditorMode::Ladder => "Ladder mode — click palette, then a cell.".into(),
                    EditorMode::Fbd => {
                        "FBD mode — place blocks, click output port then input port to wire.".into()
                    }
                };
            }
            Message::Run => {
                self.sim.mode = SimMode::Run;
                self.status_hint = "RUN — scanning.".into();
            }
            Message::Stop => {
                self.sim.mode = SimMode::Stop;
                self.status_hint = "STOP — edit mode — press Run.".into();
            }
            Message::Step => {
                if self.sim.mode != SimMode::Fault {
                    self.sim.mode = SimMode::Stop;
                    scan_cycle(&mut self.program, &mut self.sim, self.mode, 0.02);
                    self.status_hint = format!("Stepped — cycle {}", self.sim.cycle_count);
                }
            }
            Message::Undo => {
                if let Some(prev) = self.undo_stack.pop() {
                    self.program = prev;
                    self.selection = None;
                }
            }
            Message::Delete => {
                self.push_undo();
                match self.selection.clone() {
                    Some(Selection::Ladder { rung, target }) => {
                        self.program.remove_selected_ladder(Some((rung, target)));
                        self.selection = None;
                    }
                    Some(Selection::FbdBlock(id)) => {
                        self.program.fbd_blocks.retain(|b| b.id != id);
                        self.program
                            .fbd_wires
                            .retain(|w| w.from.block_id != id && w.to.block_id != id);
                        self.selection = None;
                    }
                    Some(Selection::FbdWire(id)) => {
                        self.program.fbd_wires.retain(|w| w.id != id);
                        self.selection = None;
                    }
                    None => {}
                }
            }
            Message::Clear => {
                self.push_undo();
                self.program.rungs.clear();
                self.program.fbd_blocks.clear();
                self.program.fbd_wires.clear();
                self.sim.reset_runtime(&self.program.input_kinds);
                self.selection = None;
            }
            Message::AddRung => {
                self.push_undo();
                self.program.create_rung();
            }
            Message::ZoomIn => {
                self.scale = (self.scale * 1.15).min(3.0);
            }
            Message::ZoomOut => {
                self.scale = (self.scale / 1.15).max(0.4);
            }
            Message::TogglePan => {
                self.pan_tool = !self.pan_tool;
            }
            Message::LoadDemo => {
                self.push_undo();
                let (program, _) = motor_start_stop();
                self.program = program;
                self.sim.reset_runtime(&self.program.input_kinds);
                self.selection = None;
                self.status_hint = "Loaded motor start/stop demo.".into();
            }
            Message::Save => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("PLC JSON", &["json"])
                    .set_file_name("program.json")
                    .save_file()
                {
                    if let Ok(json) = serde_json::to_string_pretty(&self.program) {
                        let _ = std::fs::write(&path, json);
                        self.status_hint = format!("Saved {}", path.display());
                    }
                }
            }
            Message::Load => {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("PLC JSON", &["json"])
                    .pick_file()
                {
                    return self.load_path(path);
                }
            }
            Message::PaletteLadder(instr) => {
                self.palette_sel = Some(PaletteSelection::Ladder(instr));
                self.status_hint = format!("Place {} — click a ladder cell.", instr.label());
            }
            Message::PaletteFbd(kind) => {
                self.palette_sel = Some(PaletteSelection::Fbd(kind));
                self.status_hint = format!("Place {} — click the canvas.", kind.label());
            }
            Message::TogglePaletteCat(cat) => {
                if let Some(i) = self.collapsed_cats.iter().position(|c| *c == cat) {
                    self.collapsed_cats.remove(i);
                } else {
                    self.collapsed_cats.push(cat);
                }
            }
            Message::Canvas(msg) => self.handle_canvas(msg),
            Message::CycleInputKind(i) => {
                let addr = format!("I0.{i}");
                let cur = self
                    .program
                    .input_kinds
                    .get(&addr)
                    .copied()
                    .unwrap_or_default();
                let next = cur.next();
                if next == InputKind::Toggle {
                    self.program.input_kinds.remove(&addr);
                } else {
                    self.program.input_kinds.insert(addr.clone(), next);
                }
                self.sim.inputs.insert(addr, next.resting_value());
            }
            Message::InputPress(i) => {
                let addr = format!("I0.{i}");
                let kind = self
                    .program
                    .input_kinds
                    .get(&addr)
                    .copied()
                    .unwrap_or_default();
                match kind {
                    InputKind::MomNo => {
                        self.sim.inputs.insert(addr, true);
                    }
                    InputKind::MomNc => {
                        self.sim.inputs.insert(addr, false);
                    }
                    InputKind::Toggle => {
                        let cur = *self.sim.inputs.get(&addr).unwrap_or(&false);
                        self.sim.inputs.insert(addr, !cur);
                    }
                }
            }
            Message::InputRelease(i) => {
                let addr = format!("I0.{i}");
                let kind = self
                    .program
                    .input_kinds
                    .get(&addr)
                    .copied()
                    .unwrap_or_default();
                match kind {
                    InputKind::MomNo => {
                        self.sim.inputs.insert(addr, false);
                    }
                    InputKind::MomNc => {
                        self.sim.inputs.insert(addr, true);
                    }
                    InputKind::Toggle => {}
                }
            }
            Message::AddressEdited(s) => {
                self.address_draft = s;
            }
            Message::ApplyAddress => {
                let draft = self.address_draft.clone();
                if let Some(addr) = self.selected_address_mut() {
                    *addr = draft;
                }
            }
        }
        Task::none()
    }

    fn load_path(&mut self, path: PathBuf) -> Task<Message> {
        match std::fs::read_to_string(&path) {
            Ok(s) => match serde_json::from_str::<PlcProgram>(&s) {
                Ok(program) => {
                    self.push_undo();
                    self.program = program;
                    self.sim.reset_runtime(&self.program.input_kinds);
                    self.selection = None;
                    self.status_hint = format!("Loaded {}", path.display());
                }
                Err(e) => {
                    self.status_hint = format!("Load error: {e}");
                }
            },
            Err(e) => {
                self.status_hint = format!("Read error: {e}");
            }
        }
        Task::none()
    }

    fn handle_canvas(&mut self, msg: CanvasMsg) {
        match msg {
            CanvasMsg::CursorMoved(p) => self.cursor = Some(p),
            CanvasMsg::ZoomAt { factor, at } => {
                let before = PointWorld::from_screen(at, self.offset, self.scale);
                self.scale = (self.scale * factor).clamp(0.4, 3.0);
                let after = PointWorld::from_screen(at, self.offset, self.scale);
                self.offset.x += (after.x - before.x) * self.scale;
                self.offset.y += (after.y - before.y) * self.scale;
            }
            CanvasMsg::PanDelta(d) => {
                self.offset = self.offset + d;
            }
            CanvasMsg::ClearSelection => {
                self.selection = None;
                self.wire_draft = None;
            }
            CanvasMsg::CancelWire => {
                self.wire_draft = None;
            }
            CanvasMsg::SelectLadder { rung, target } => {
                // Clicking an existing element always selects it (placement uses empty cells).
                self.selection = Some(Selection::Ladder { rung, target });
                self.sync_address_draft();
            }
            CanvasMsg::PlaceLadder { rung, col, row } => {
                if let Some(PaletteSelection::Ladder(instr)) = self.palette_sel {
                    self.place_ladder(rung, (col, row), instr);
                } else {
                    self.selection = None;
                }
            }
            CanvasMsg::SelectFbdBlock(id) => {
                self.selection = Some(Selection::FbdBlock(id));
                self.wire_draft = None;
                self.sync_address_draft();
            }
            CanvasMsg::SelectFbdWire(id) => {
                self.selection = Some(Selection::FbdWire(id));
            }
            CanvasMsg::PlaceFbd { x, y } => {
                if let Some(PaletteSelection::Fbd(kind)) = self.palette_sel {
                    self.push_undo();
                    let id = self.program.add_fbd_block(kind, x, y, None);
                    self.selection = Some(Selection::FbdBlock(id));
                    self.palette_sel = None;
                    self.sync_address_draft();
                } else if self.wire_draft.is_some() {
                    self.wire_draft = None;
                } else {
                    self.selection = None;
                }
            }
            CanvasMsg::StartWire { block_id, port } => {
                self.wire_draft = Some((block_id, port));
                self.status_hint = "Click an input port to complete the wire.".into();
            }
            CanvasMsg::CompleteWire { block_id, port } => {
                if let Some((from_id, from_port)) = self.wire_draft.take() {
                    self.push_undo();
                    self.program
                        .add_fbd_wire(from_id, from_port, block_id, port);
                }
            }
        }
    }

    fn place_ladder(&mut self, rung: usize, (col, row): (usize, usize), instr: LadderInstr) {
        self.push_undo();
        if self.program.rungs.is_empty() {
            self.program.create_rung();
        }
        let rung = rung.min(self.program.rungs.len().saturating_sub(1));
        let addr = instr.default_address().to_string();
        if self.program.add_element(rung, instr, col, row, addr) {
            let idx = self.program.rungs[rung].elements.len() - 1;
            self.selection = Some(Selection::Ladder {
                rung,
                target: SelectionTarget::Element(idx),
            });
            self.palette_sel = None;
            self.sync_address_draft();
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        let header = container(
            row![
                text("RustSimu PLC").size(22).style(|_| text::Style {
                    color: Some(ACCENT)
                }),
                Space::with_width(Length::Fill),
                status_leds(self.sim.mode),
                text(&self.status_hint).size(12).style(|_| text::Style {
                    color: Some(TEXT_DIM)
                }),
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .padding(10)
        .style(|_| container::Style {
            background: Some(Background::Color(SURFACE)),
            border: Border {
                color: BORDER,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        });

        let body = row![
            self.view_palette(),
            column![
                self.view_canvas(),
                self.view_toolbar(),
                self.view_io_status(),
                self.view_props(),
            ]
            .spacing(8)
            .width(Length::Fill),
        ]
        .spacing(10);

        container(column![header, body].spacing(10))
            .padding(12)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(BG)),
                ..Default::default()
            })
            .into()
    }

    fn view_palette(&self) -> Element<'_, Message> {
        let mut col = column![text("INSTRUCTIONS").size(11).style(|_| text::Style {
            color: Some(TEXT_DIM)
        })]
        .spacing(4)
        .padding(8);

        match self.mode {
            EditorMode::Ladder => {
                for (cat, items) in ladder_palette() {
                    let collapsed = self.collapsed_cats.contains(&cat);
                    col = col.push(
                        button(
                            row![
                                text(if collapsed { "▸ " } else { "▾ " }).size(11),
                                text(cat).size(11).style(|_| text::Style {
                                    color: Some(theme::BLUE)
                                }),
                            ]
                            .spacing(4),
                        )
                        .on_press(Message::TogglePaletteCat(cat))
                        .style(ghost_btn),
                    );
                    if !collapsed {
                        for instr in items {
                            let active = self.palette_sel == Some(PaletteSelection::Ladder(instr));
                            col = col.push(
                                button(
                                    row![
                                        text(instr.symbol()).size(13).width(36),
                                        text(instr.label()).size(12),
                                    ]
                                    .spacing(6),
                                )
                                .on_press(Message::PaletteLadder(instr))
                                .style(if active {
                                    accent_btn
                                } else {
                                    ghost_btn
                                }),
                            );
                        }
                    }
                }
            }
            EditorMode::Fbd => {
                for (cat, items) in fbd_palette() {
                    let collapsed = self.collapsed_cats.contains(&cat);
                    col = col.push(
                        button(
                            row![
                                text(if collapsed { "▸ " } else { "▾ " }).size(11),
                                text(cat).size(11).style(|_| text::Style {
                                    color: Some(theme::BLUE)
                                }),
                            ]
                            .spacing(4),
                        )
                        .on_press(Message::TogglePaletteCat(cat))
                        .style(ghost_btn),
                    );
                    if !collapsed {
                        for kind in items {
                            let active = self.palette_sel == Some(PaletteSelection::Fbd(kind));
                            col = col.push(
                                button(text(kind.label()).size(12))
                                    .on_press(Message::PaletteFbd(kind))
                                    .style(if active { accent_btn } else { ghost_btn }),
                            );
                        }
                    }
                }
            }
        }

        container(scrollable(col).height(Length::Fill))
            .width(200)
            .height(Length::Fill)
            .style(|_| container::Style {
                background: Some(Background::Color(SURFACE)),
                border: Border {
                    color: BORDER,
                    width: 1.0,
                    radius: 10.0.into(),
                },
                ..Default::default()
            })
            .into()
    }

    fn view_canvas(&self) -> Element<'_, Message> {
        let canvas_widget: Element<'_, CanvasMsg> = canvas(PlcCanvas {
            program: &self.program,
            sim: &self.sim,
            mode: self.mode,
            selection: self.selection.as_ref(),
            scale: self.scale,
            offset: self.offset,
            wire_draft: self.wire_draft.clone(),
            cursor: self.cursor,
            pan_tool: self.pan_tool,
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into();

        let floating = column![
            button(text("+").size(14))
                .on_press(Message::ZoomIn)
                .style(ghost_btn),
            button(text("-").size(14))
                .on_press(Message::ZoomOut)
                .style(ghost_btn),
            button(text(if self.pan_tool { "PAN" } else { "pan" }).size(10))
                .on_press(Message::TogglePan)
                .style(if self.pan_tool { accent_btn } else { ghost_btn }),
            button(text("U").size(12))
                .on_press(Message::Undo)
                .style(ghost_btn),
        ]
        .spacing(4);

        let overlay = container(floating)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(12)
            .align_x(Alignment::End)
            .align_y(Alignment::End);

        container(Stack::with_children(vec![
            canvas_widget.map(Message::Canvas),
            overlay.into(),
        ]))
        .height(Length::FillPortion(5))
        .style(|_| container::Style {
            background: Some(Background::Color(BG)),
            border: Border {
                color: BORDER,
                width: 1.0,
                radius: 10.0.into(),
            },
            ..Default::default()
        })
        .into()
    }

    fn view_toolbar(&self) -> Element<'_, Message> {
        let ladder_active = self.mode == EditorMode::Ladder;
        let fbd_active = self.mode == EditorMode::Fbd;
        row![
            button(text("Ladder").size(13))
                .on_press(Message::SetMode(EditorMode::Ladder))
                .style(if ladder_active { accent_btn } else { ghost_btn }),
            button(text("FBD").size(13))
                .on_press(Message::SetMode(EditorMode::Fbd))
                .style(if fbd_active { accent_btn } else { ghost_btn }),
            Space::with_width(8),
            button(text("Run").size(13))
                .on_press(Message::Run)
                .style(accent_btn),
            button(text("Stop").size(13))
                .on_press(Message::Stop)
                .style(ghost_btn),
            button(text("Step").size(13))
                .on_press(Message::Step)
                .style(ghost_btn),
            button(text("Undo").size(13))
                .on_press(Message::Undo)
                .style(ghost_btn),
            button(text("Delete").size(13))
                .on_press(Message::Delete)
                .style(ghost_btn),
            button(text("Clear").size(13))
                .on_press(Message::Clear)
                .style(ghost_btn),
            button(text("+ Rung").size(13))
                .on_press(Message::AddRung)
                .style(ghost_btn),
            Space::with_width(8),
            button(text("Demo").size(13))
                .on_press(Message::LoadDemo)
                .style(ghost_btn),
            button(text("Save").size(13))
                .on_press(Message::Save)
                .style(ghost_btn),
            button(text("Load").size(13))
                .on_press(Message::Load)
                .style(ghost_btn),
            Space::with_width(Length::Fill),
            text("Click an instruction to place. Toggle inputs below.")
                .size(11)
                .style(|_| text::Style {
                    color: Some(TEXT_DIM)
                }),
        ]
        .spacing(6)
        .padding(8)
        .align_y(Alignment::Center)
        .into()
    }

    fn view_io_status(&self) -> Element<'_, Message> {
        let mut inputs = row![text("INPUTS").size(11).style(|_| text::Style {
            color: Some(TEXT_DIM)
        })]
        .spacing(6)
        .align_y(Alignment::Center);

        for i in 0..8 {
            let addr = format!("I0.{i}");
            let on = *self.sim.inputs.get(&addr).unwrap_or(&false);
            let kind = self
                .program
                .input_kinds
                .get(&addr)
                .copied()
                .unwrap_or_default();
            let label = self
                .program
                .input_labels
                .get(i)
                .cloned()
                .unwrap_or_default();
            let title = if label.is_empty() {
                format!("I0.{i}\n{}", kind.label())
            } else {
                format!("I0.{i} {label}\n{}", kind.label())
            };
            inputs = inputs.push(
                column![
                    mouse_area(container(text(title).size(10)).padding(8).style(move |_| {
                        container::Style {
                            background: Some(Background::Color(if on {
                                Color { a: 0.35, ..GOLD }
                            } else {
                                SURFACE2
                            })),
                            border: Border {
                                color: if on { GOLD } else { BORDER },
                                width: 1.0,
                                radius: 8.0.into(),
                            },
                            ..Default::default()
                        }
                    }),)
                    .on_press(Message::InputPress(i))
                    .on_release(Message::InputRelease(i)),
                    button(text("kind").size(9))
                        .on_press(Message::CycleInputKind(i))
                        .style(ghost_btn),
                ]
                .spacing(2)
                .align_x(Alignment::Center),
            );
        }

        let mut outputs = row![text("OUTPUTS").size(11).style(|_| text::Style {
            color: Some(TEXT_DIM)
        })]
        .spacing(6)
        .align_y(Alignment::Center);
        for i in 0..8 {
            let addr = format!("Q0.{i}");
            let on = *self.sim.outputs.get(&addr).unwrap_or(&false);
            let label = self
                .program
                .output_labels
                .get(i)
                .cloned()
                .unwrap_or_default();
            let title = if label.is_empty() {
                format!("Q0.{i}")
            } else {
                format!("Q0.{i} {label}")
            };
            outputs = outputs.push(
                container(text(title).size(10).style(move |_| text::Style {
                    color: Some(if on { GREEN } else { TEXT_DIM }),
                }))
                .padding(8)
                .style(move |_| container::Style {
                    background: Some(Background::Color(if on {
                        Color { a: 0.25, ..ACCENT }
                    } else {
                        SURFACE2
                    })),
                    border: Border {
                        color: BORDER,
                        width: 1.0,
                        radius: 8.0.into(),
                    },
                    ..Default::default()
                }),
            );
        }

        let status = column![
            text("STATUS").size(11).style(|_| text::Style {
                color: Some(TEXT_DIM)
            }),
            text(format!("SCAN TIME  {:.2} ms", self.sim.scan_time_ms))
                .size(12)
                .style(|_| text::Style { color: Some(GREEN) }),
            text(format!("CYCLE COUNT  {}", self.sim.cycle_count))
                .size(12)
                .style(|_| text::Style { color: Some(GREEN) }),
            text(format!(
                "STATE  {}",
                match self.sim.mode {
                    SimMode::Run => "RUNNING",
                    SimMode::Stop => "STOPPED",
                    SimMode::Fault => "FAULT",
                }
            ))
            .size(12)
            .style(|_| text::Style {
                color: Some(match self.sim.mode {
                    SimMode::Run => GREEN,
                    SimMode::Stop => GOLD,
                    SimMode::Fault => theme::RED,
                })
            }),
        ]
        .spacing(4)
        .padding(8)
        .width(160);

        container(
            row![
                column![inputs, outputs].spacing(8).width(Length::Fill),
                status,
            ]
            .spacing(10),
        )
        .padding(8)
        .style(|_| container::Style {
            background: Some(Background::Color(SURFACE)),
            border: Border {
                color: BORDER,
                width: 1.0,
                radius: 10.0.into(),
            },
            ..Default::default()
        })
        .into()
    }

    fn view_props(&self) -> Element<'_, Message> {
        let label = match &self.selection {
            Some(Selection::Ladder { rung, target }) => {
                let kind = self
                    .program
                    .rungs
                    .get(*rung)
                    .and_then(|r| match target {
                        SelectionTarget::Element(i) => r.elements.get(*i).map(|e| e.instr.label()),
                        SelectionTarget::BranchElement { branch, index } => r
                            .branches
                            .get(*branch)
                            .and_then(|b| b.elements.get(*index))
                            .map(|e| e.instr.label()),
                        SelectionTarget::Rung => Some("Rung"),
                    })
                    .unwrap_or("—");
                format!("Selected: {kind}")
            }
            Some(Selection::FbdBlock(id)) => self
                .program
                .find_block(*id)
                .map(|b| format!("Selected: {}", b.kind.label()))
                .unwrap_or_else(|| "Selected: —".into()),
            Some(Selection::FbdWire(_)) => "Selected: Wire".into(),
            None => "No selection".into(),
        };

        row![
            text(label).size(12),
            Space::with_width(16),
            text("Address").size(12).style(|_| text::Style {
                color: Some(TEXT_DIM)
            }),
            text_input("I0.0", &self.address_draft)
                .on_input(Message::AddressEdited)
                .on_submit(Message::ApplyAddress)
                .width(120)
                .size(13),
            button(text("Apply").size(12))
                .on_press(Message::ApplyAddress)
                .style(ghost_btn),
        ]
        .spacing(8)
        .padding(8)
        .align_y(Alignment::Center)
        .into()
    }
}

struct PointWorld {
    x: f32,
    y: f32,
}

impl PointWorld {
    fn from_screen(p: iced::Point, offset: Vector, scale: f32) -> Self {
        Self {
            x: (p.x - offset.x) / scale,
            y: (p.y - offset.y) / scale,
        }
    }
}

fn status_leds(mode: SimMode) -> Element<'static, Message> {
    let led = |label: &'static str, on: bool, color: Color| {
        row![
            text("*").size(12).style(move |_| text::Style {
                color: Some(if on { color } else { TEXT_DIM })
            }),
            text(label).size(11).style(move |_| text::Style {
                color: Some(if on { color } else { TEXT_DIM })
            }),
        ]
        .spacing(3)
    };
    row![
        led("RUN", mode == SimMode::Run, GREEN),
        led("STOP", mode == SimMode::Stop, GOLD),
        led("FAULT", mode == SimMode::Fault, theme::RED),
    ]
    .spacing(10)
    .into()
}

fn ghost_btn(theme: &Theme, status: button::Status) -> button::Style {
    let _ = theme;
    let bg = match status {
        button::Status::Hovered => SURFACE2,
        _ => SURFACE,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: TEXT,
        border: Border {
            color: BORDER,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}

fn accent_btn(theme: &Theme, status: button::Status) -> button::Style {
    let _ = theme;
    let bg = match status {
        button::Status::Hovered => Color {
            r: ACCENT.r * 1.1,
            g: ACCENT.g * 1.1,
            b: ACCENT.b * 1.1,
            a: 1.0,
        },
        _ => ACCENT,
    };
    button::Style {
        background: Some(Background::Color(bg)),
        text_color: Color::WHITE,
        border: Border {
            color: ACCENT,
            width: 1.0,
            radius: 6.0.into(),
        },
        ..Default::default()
    }
}
