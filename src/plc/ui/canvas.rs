//! Canvas rendering and hit-testing for Ladder and FBD editors.

use crate::plc::model::{
    EditorMode, FbdBlock, FbdBlockType, LadderInstr, PlcProgram, Selection, SelectionTarget,
    SimState, COLS,
};
use crate::plc::theme::{
    self, ACCENT, BG, BLUE, BORDER, CELL_H, CELL_W, COLD, CYAN, GOLD, GREEN, HOT, RAIL_X_L, RED,
    RUNG_GAP, RUNG_HEADER, SURFACE2, TEXT, TEXT_DIM,
};
use iced::mouse;
use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke, Text};
use iced::{Color, Point, Rectangle, Renderer, Size, Theme, Vector};

#[derive(Debug, Clone)]
pub enum CanvasMsg {
    SelectLadder {
        rung: usize,
        target: SelectionTarget,
    },
    SelectFbdBlock(u32),
    SelectFbdWire(u32),
    PlaceLadder {
        rung: usize,
        col: usize,
        row: usize,
    },
    PlaceFbd {
        x: f32,
        y: f32,
    },
    StartWire {
        block_id: u32,
        port: String,
    },
    CompleteWire {
        block_id: u32,
        port: String,
    },
    CancelWire,
    PanDelta(Vector),
    ZoomAt {
        factor: f32,
        at: Point,
    },
    CursorMoved(Point),
    ClearSelection,
}

pub struct PlcCanvas<'a> {
    pub program: &'a PlcProgram,
    pub sim: &'a SimState,
    pub mode: EditorMode,
    pub selection: Option<&'a Selection>,
    pub scale: f32,
    pub offset: Vector,
    pub wire_draft: Option<(u32, String)>,
    pub cursor: Option<Point>,
    pub pan_tool: bool,
}

impl<'a> PlcCanvas<'a> {
    fn to_world(&self, screen: Point) -> Point {
        Point::new(
            (screen.x - self.offset.x) / self.scale,
            (screen.y - self.offset.y) / self.scale,
        )
    }

    fn rail_right() -> f32 {
        RAIL_X_L + (COLS as f32 + 1.0) * CELL_W
    }

    fn rung_y(rung_idx: usize) -> f32 {
        20.0 + rung_idx as f32 * (CELL_H * 2.0 + RUNG_HEADER + RUNG_GAP)
    }

    fn draw_ladder(&self, frame: &mut Frame) {
        let rail_r = Self::rail_right();
        let n = self.program.rungs.len().max(1);

        // Dot grid
        {
            let mut y = 0.0;
            while y < 900.0 {
                let mut x = 0.0;
                while x < 1100.0 {
                    frame.fill(
                        &Path::circle(Point::new(x, y), 0.8),
                        Color { a: 0.25, ..BORDER },
                    );
                    x += 20.0;
                }
                y += 20.0;
            }
        }

        for ri in 0..self.program.rungs.len().max(1) {
            let y0 = Self::rung_y(ri);
            let h = CELL_H * 2.0 + RUNG_HEADER;

            // Rails
            let rail = Stroke::default().with_width(3.0).with_color(
                if self.sim.mode == crate::plc::model::SimMode::Run {
                    HOT
                } else {
                    COLD
                },
            );
            frame.stroke(
                &Path::line(Point::new(RAIL_X_L, y0), Point::new(RAIL_X_L, y0 + h)),
                rail,
            );
            frame.stroke(
                &Path::line(Point::new(rail_r, y0), Point::new(rail_r, y0 + h)),
                rail,
            );

            // Rung header
            frame.fill_text(Text {
                content: format!("Rung {ri}"),
                position: Point::new(RAIL_X_L + 8.0, y0 + 2.0),
                color: TEXT_DIM,
                size: iced::Pixels(11.0),
                ..Text::default()
            });

            if let Some(rung) = self.program.rungs.get(ri) {
                if !rung.comment.is_empty() {
                    frame.fill_text(Text {
                        content: rung.comment.clone(),
                        position: Point::new(RAIL_X_L + 70.0, y0 + 2.0),
                        color: TEXT_DIM,
                        size: iced::Pixels(10.0),
                        ..Text::default()
                    });
                }

                let mid_y = y0 + RUNG_HEADER + CELL_H * 0.55;
                let hot_wire =
                    rung.output_power || self.sim.mode != crate::plc::model::SimMode::Run;

                // Main horizontal bus
                let bus_color = if self.sim.mode == crate::plc::model::SimMode::Run
                    && rung.seg_power.first().copied().unwrap_or(false)
                {
                    HOT
                } else {
                    COLD
                };
                frame.stroke(
                    &Path::line(Point::new(RAIL_X_L, mid_y), Point::new(rail_r, mid_y)),
                    Stroke::default().with_width(2.0).with_color(bus_color),
                );

                // Branch wires
                for br in &rung.branches {
                    let by = mid_y + CELL_H * 0.85;
                    let x0 = RAIL_X_L + (br.start_col as f32 + 1.0) * CELL_W;
                    let x1 = RAIL_X_L + (br.end_col as f32 + 2.0) * CELL_W;
                    let col = if self.sim.mode == crate::plc::model::SimMode::Run && br.power {
                        HOT
                    } else {
                        COLD
                    };
                    frame.stroke(
                        &Path::line(Point::new(x0, mid_y), Point::new(x0, by)),
                        Stroke::default().with_width(2.0).with_color(col),
                    );
                    frame.stroke(
                        &Path::line(Point::new(x0, by), Point::new(x1, by)),
                        Stroke::default().with_width(2.0).with_color(col),
                    );
                    frame.stroke(
                        &Path::line(Point::new(x1, by), Point::new(x1, mid_y)),
                        Stroke::default().with_width(2.0).with_color(col),
                    );
                    for el in &br.elements {
                        self.draw_ladder_element(frame, el, y0, mid_y, by, false);
                    }
                }

                for (ei, el) in rung.elements.iter().enumerate() {
                    let selected = matches!(
                        self.selection,
                        Some(Selection::Ladder {
                            rung: r,
                            target: SelectionTarget::Element(i)
                        }) if *r == ri && *i == ei
                    );
                    let ey = if el.row == 0 {
                        mid_y
                    } else {
                        mid_y + CELL_H * 0.85
                    };
                    self.draw_ladder_element(frame, el, y0, mid_y, ey, selected);
                    let _ = hot_wire;
                }
            } else if ri == 0 && self.program.rungs.is_empty() {
                frame.fill_text(Text {
                    content: "Empty program — pick an instruction and click a cell, or load demo."
                        .into(),
                    position: Point::new(RAIL_X_L + 20.0, y0 + 40.0),
                    color: TEXT_DIM,
                    size: iced::Pixels(14.0),
                    ..Text::default()
                });
                let _ = n;
            }
        }
    }

    fn draw_ladder_element(
        &self,
        frame: &mut Frame,
        el: &crate::plc::model::LadderElement,
        _y0: f32,
        mid_y: f32,
        ey: f32,
        selected: bool,
    ) {
        let cx = RAIL_X_L + (el.col as f32 + 1.0) * CELL_W + CELL_W * 0.5;
        let color = if self.sim.mode == crate::plc::model::SimMode::Run && el.state {
            HOT
        } else {
            TEXT
        };
        if selected {
            frame.stroke(
                &Path::rectangle(Point::new(cx - 28.0, ey - 22.0), Size::new(56.0, 44.0)),
                Stroke::default().with_width(1.5).with_color(CYAN),
            );
        }

        match el.instr {
            LadderInstr::ContactNo
            | LadderInstr::ContactNc
            | LadderInstr::ContactPos
            | LadderInstr::ContactNeg => {
                frame.stroke(
                    &Path::line(
                        Point::new(cx - 12.0, ey - 14.0),
                        Point::new(cx - 12.0, ey + 14.0),
                    ),
                    Stroke::default().with_width(2.5).with_color(color),
                );
                frame.stroke(
                    &Path::line(
                        Point::new(cx + 12.0, ey - 14.0),
                        Point::new(cx + 12.0, ey + 14.0),
                    ),
                    Stroke::default().with_width(2.5).with_color(color),
                );
                if el.instr == LadderInstr::ContactNc {
                    frame.stroke(
                        &Path::line(
                            Point::new(cx - 14.0, ey + 14.0),
                            Point::new(cx + 14.0, ey - 14.0),
                        ),
                        Stroke::default().with_width(2.0).with_color(color),
                    );
                }
                if el.instr == LadderInstr::ContactPos {
                    frame.fill_text(Text {
                        content: "P".into(),
                        position: Point::new(cx - 4.0, ey - 6.0),
                        color,
                        size: iced::Pixels(12.0),
                        ..Text::default()
                    });
                }
                if el.instr == LadderInstr::ContactNeg {
                    frame.fill_text(Text {
                        content: "N".into(),
                        position: Point::new(cx - 4.0, ey - 6.0),
                        color,
                        size: iced::Pixels(12.0),
                        ..Text::default()
                    });
                }
            }
            LadderInstr::Coil
            | LadderInstr::CoilSet
            | LadderInstr::CoilReset
            | LadderInstr::CoilNeg => {
                frame.stroke(
                    &Path::circle(Point::new(cx, ey), 16.0),
                    Stroke::default().with_width(2.5).with_color(color),
                );
                let mark = match el.instr {
                    LadderInstr::CoilSet => "S",
                    LadderInstr::CoilReset => "R",
                    LadderInstr::CoilNeg => "/",
                    _ => "",
                };
                if !mark.is_empty() {
                    frame.fill_text(Text {
                        content: mark.into(),
                        position: Point::new(cx - 5.0, ey - 7.0),
                        color,
                        size: iced::Pixels(14.0),
                        ..Text::default()
                    });
                }
            }
            LadderInstr::Ton => {
                frame.fill(
                    &Path::rectangle(Point::new(cx - 30.0, ey - 18.0), Size::new(60.0, 36.0)),
                    SURFACE2,
                );
                frame.stroke(
                    &Path::rectangle(Point::new(cx - 30.0, ey - 18.0), Size::new(60.0, 36.0)),
                    Stroke::default().with_width(1.5).with_color(color),
                );
                frame.fill_text(Text {
                    content: "TON".into(),
                    position: Point::new(cx - 14.0, ey - 8.0),
                    color,
                    size: iced::Pixels(12.0),
                    ..Text::default()
                });
            }
        }

        frame.fill_text(Text {
            content: el.address.clone(),
            position: Point::new(cx - 16.0, ey - 32.0),
            color: BLUE,
            size: iced::Pixels(11.0),
            ..Text::default()
        });

        // Label under coil if motor
        if el.instr.is_coil() {
            let _ = mid_y;
        }
    }

    fn draw_fbd(&self, frame: &mut Frame) {
        // Dot grid
        {
            let mut y = 0.0;
            while y < 900.0 {
                let mut x = 0.0;
                while x < 1100.0 {
                    frame.fill(
                        &Path::circle(Point::new(x, y), 0.8),
                        Color { a: 0.25, ..BORDER },
                    );
                    x += 20.0;
                }
                y += 20.0;
            }
        }

        for w in &self.program.fbd_wires {
            let Some(from) = self.program.find_block(w.from.block_id) else {
                continue;
            };
            let Some(to) = self.program.find_block(w.to.block_id) else {
                continue;
            };
            let p0 = port_pos(from, &w.from.port, true);
            let p1 = port_pos(to, &w.to.port, false);
            let selected = matches!(self.selection, Some(Selection::FbdWire(id)) if *id == w.id);
            let color = if selected {
                CYAN
            } else if w.powered {
                HOT
            } else {
                COLD
            };
            let mid_x = (p0.x + p1.x) * 0.5;
            let path = Path::new(|b| {
                b.move_to(p0);
                b.line_to(Point::new(mid_x, p0.y));
                b.line_to(Point::new(mid_x, p1.y));
                b.line_to(p1);
            });
            frame.stroke(&path, Stroke::default().with_width(2.0).with_color(color));
        }

        if let (Some((bid, port)), Some(cursor)) = (&self.wire_draft, self.cursor) {
            if let Some(from) = self.program.find_block(*bid) {
                let p0 = port_pos(from, port, true);
                let world = self.to_world(cursor);
                frame.stroke(
                    &Path::line(p0, world),
                    Stroke::default().with_width(1.5).with_color(GOLD),
                );
            }
        }

        for b in &self.program.fbd_blocks {
            let selected = matches!(self.selection, Some(Selection::FbdBlock(id)) if *id == b.id);
            draw_fbd_block(
                frame,
                b,
                selected,
                self.sim.mode == crate::plc::model::SimMode::Run,
            );
        }

        if self.program.fbd_blocks.is_empty() {
            frame.fill_text(Text {
                content: "Empty FBD — pick a block from the palette and click to place.".into(),
                position: Point::new(40.0, 40.0),
                color: TEXT_DIM,
                size: iced::Pixels(14.0),
                ..Text::default()
            });
        }
    }

    fn hit_ladder(&self, world: Point) -> Option<CanvasMsg> {
        for (ri, rung) in self.program.rungs.iter().enumerate() {
            let y0 = Self::rung_y(ri);
            let mid_y = y0 + RUNG_HEADER + CELL_H * 0.55;
            for (ei, el) in rung.elements.iter().enumerate() {
                let ey = if el.row == 0 {
                    mid_y
                } else {
                    mid_y + CELL_H * 0.85
                };
                let cx = RAIL_X_L + (el.col as f32 + 1.0) * CELL_W + CELL_W * 0.5;
                if (world.x - cx).abs() < 28.0 && (world.y - ey).abs() < 22.0 {
                    return Some(CanvasMsg::SelectLadder {
                        rung: ri,
                        target: SelectionTarget::Element(ei),
                    });
                }
            }
            for (bi, br) in rung.branches.iter().enumerate() {
                let by = mid_y + CELL_H * 0.85;
                for (ei, el) in br.elements.iter().enumerate() {
                    let cx = RAIL_X_L + (el.col as f32 + 1.0) * CELL_W + CELL_W * 0.5;
                    if (world.x - cx).abs() < 28.0 && (world.y - by).abs() < 22.0 {
                        return Some(CanvasMsg::SelectLadder {
                            rung: ri,
                            target: SelectionTarget::BranchElement {
                                branch: bi,
                                index: ei,
                            },
                        });
                    }
                }
            }
            // Empty cell placement
            if world.y >= y0 && world.y <= y0 + CELL_H * 2.0 + RUNG_HEADER {
                let col = ((world.x - RAIL_X_L) / CELL_W).floor() as i32 - 1;
                if (0..COLS as i32).contains(&col) {
                    let row = if world.y > mid_y + CELL_H * 0.4 { 1 } else { 0 };
                    return Some(CanvasMsg::PlaceLadder {
                        rung: ri,
                        col: col as usize,
                        row,
                    });
                }
            }
        }
        // Click below last rung → treat as place on last / create handled in app
        if self.program.rungs.is_empty() {
            return Some(CanvasMsg::PlaceLadder {
                rung: 0,
                col: 0,
                row: 0,
            });
        }
        None
    }

    fn hit_fbd(&self, world: Point) -> Option<CanvasMsg> {
        // Ports first
        for b in self.program.fbd_blocks.iter().rev() {
            for p in b.kind.outputs() {
                let pos = port_pos(b, p, true);
                if (world.x - pos.x).abs() <= 8.0 && (world.y - pos.y).abs() <= 8.0 {
                    return Some(CanvasMsg::StartWire {
                        block_id: b.id,
                        port: (*p).to_string(),
                    });
                }
            }
            for p in b.kind.inputs() {
                let pos = port_pos(b, p, false);
                if (world.x - pos.x).abs() <= 8.0
                    && (world.y - pos.y).abs() <= 8.0
                    && self.wire_draft.is_some()
                {
                    return Some(CanvasMsg::CompleteWire {
                        block_id: b.id,
                        port: (*p).to_string(),
                    });
                }
            }
            let (w, h) = b.kind.size();
            if world.x >= b.x && world.x <= b.x + w && world.y >= b.y && world.y <= b.y + h {
                return Some(CanvasMsg::SelectFbdBlock(b.id));
            }
        }
        // Wire hit (rough)
        for w in &self.program.fbd_wires {
            let Some(from) = self.program.find_block(w.from.block_id) else {
                continue;
            };
            let Some(to) = self.program.find_block(w.to.block_id) else {
                continue;
            };
            let p0 = port_pos(from, &w.from.port, true);
            let p1 = port_pos(to, &w.to.port, false);
            let mid_x = (p0.x + p1.x) * 0.5;
            if near_seg(world, p0, Point::new(mid_x, p0.y), 6.0)
                || near_seg(world, Point::new(mid_x, p0.y), Point::new(mid_x, p1.y), 6.0)
                || near_seg(world, Point::new(mid_x, p1.y), p1, 6.0)
            {
                return Some(CanvasMsg::SelectFbdWire(w.id));
            }
        }
        Some(CanvasMsg::PlaceFbd {
            x: world.x,
            y: world.y,
        })
    }
}

fn near_seg(p: Point, a: Point, b: Point, tol: f32) -> bool {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let len2 = dx * dx + dy * dy;
    if len2 < 1e-6 {
        return (p.x - a.x).hypot(p.y - a.y) <= tol;
    }
    let t = ((p.x - a.x) * dx + (p.y - a.y) * dy) / len2;
    let t = t.clamp(0.0, 1.0);
    let qx = a.x + t * dx;
    let qy = a.y + t * dy;
    (p.x - qx).hypot(p.y - qy) <= tol
}

pub fn port_pos(block: &FbdBlock, port: &str, is_output: bool) -> Point {
    let (w, h) = block.kind.size();
    let names: &[&str] = if is_output {
        block.kind.outputs()
    } else {
        block.kind.inputs()
    };
    let idx = names.iter().position(|n| *n == port).unwrap_or(0);
    let n = names.len().max(1) as f32;
    let y = block.y + h * ((idx as f32 + 0.5) / n);
    let x = if is_output { block.x + w } else { block.x };
    Point::new(x, y)
}

fn draw_fbd_block(frame: &mut Frame, b: &FbdBlock, selected: bool, running: bool) {
    let (w, h) = b.kind.size();
    let energized = running && b.out_ports.values().any(|v| *v)
        || (b.kind == FbdBlockType::Output && b.in_ports.get("in") == Some(&true));
    let border = if selected {
        GOLD
    } else if energized {
        HOT
    } else {
        COLD
    };
    frame.fill(
        &Path::rectangle(Point::new(b.x, b.y), Size::new(w, h)),
        Color {
            a: 0.95,
            ..theme::SURFACE
        },
    );
    frame.stroke(
        &Path::rectangle(Point::new(b.x, b.y), Size::new(w, h)),
        Stroke::default().with_width(1.8).with_color(border),
    );
    frame.fill_text(Text {
        content: b.kind.label().into(),
        position: Point::new(b.x + 10.0, b.y + 8.0),
        color: if energized { HOT } else { TEXT },
        size: iced::Pixels(12.0),
        ..Text::default()
    });
    if b.kind.needs_address() && !b.address.is_empty() {
        frame.fill_text(Text {
            content: b.address.clone(),
            position: Point::new(b.x + 10.0, b.y + h - 18.0),
            color: BLUE,
            size: iced::Pixels(11.0),
            ..Text::default()
        });
    }
    for p in b.kind.inputs() {
        let pos = port_pos(b, p, false);
        let hot = b.in_ports.get(*p).copied().unwrap_or(false);
        frame.fill(&Path::circle(pos, 4.0), if hot { GREEN } else { SURFACE2 });
        frame.stroke(
            &Path::circle(pos, 4.0),
            Stroke::default()
                .with_width(1.0)
                .with_color(if hot { HOT } else { ACCENT }),
        );
        frame.fill_text(Text {
            content: (*p).to_uppercase(),
            position: Point::new(pos.x + 6.0, pos.y - 5.0),
            color: TEXT_DIM,
            size: iced::Pixels(9.0),
            ..Text::default()
        });
    }
    for p in b.kind.outputs() {
        let pos = port_pos(b, p, true);
        let hot = b.out_ports.get(*p).copied().unwrap_or(false);
        frame.fill(&Path::circle(pos, 4.0), if hot { GREEN } else { SURFACE2 });
        frame.stroke(
            &Path::circle(pos, 4.0),
            Stroke::default()
                .with_width(1.0)
                .with_color(if hot { HOT } else { ACCENT }),
        );
        frame.fill_text(Text {
            content: (*p).to_uppercase(),
            position: Point::new(pos.x - 18.0, pos.y - 5.0),
            color: TEXT_DIM,
            size: iced::Pixels(9.0),
            ..Text::default()
        });
    }
    let _ = RED;
    let _ = BG;
}

impl<'a> canvas::Program<CanvasMsg> for PlcCanvas<'a> {
    type State = ();

    fn update(
        &self,
        _state: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<CanvasMsg>) {
        let Some(pos) = cursor.position_in(bounds) else {
            return (canvas::event::Status::Ignored, None);
        };

        match event {
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => (
                canvas::event::Status::Captured,
                Some(CanvasMsg::CursorMoved(pos)),
            ),
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let factor = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => {
                        if y > 0.0 {
                            1.1
                        } else {
                            1.0 / 1.1
                        }
                    }
                    mouse::ScrollDelta::Pixels { y, .. } => {
                        if y > 0.0 {
                            1.05
                        } else {
                            1.0 / 1.05
                        }
                    }
                };
                (
                    canvas::event::Status::Captured,
                    Some(CanvasMsg::ZoomAt { factor, at: pos }),
                )
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle))
            | canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
                if self.pan_tool =>
            {
                (canvas::event::Status::Captured, None)
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let world = self.to_world(pos);
                let msg = match self.mode {
                    EditorMode::Ladder => self.hit_ladder(world),
                    EditorMode::Fbd => self.hit_fbd(world),
                };
                (
                    canvas::event::Status::Captured,
                    msg.or(Some(CanvasMsg::ClearSelection)),
                )
            }
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                if self.wire_draft.is_some() {
                    (canvas::event::Status::Captured, Some(CanvasMsg::CancelWire))
                } else {
                    (
                        canvas::event::Status::Captured,
                        Some(CanvasMsg::ClearSelection),
                    )
                }
            }
            _ => (canvas::event::Status::Ignored, None),
        }
    }

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), BG);
        frame.with_save(|frame| {
            frame.translate(self.offset);
            frame.scale(self.scale);
            match self.mode {
                EditorMode::Ladder => self.draw_ladder(frame),
                EditorMode::Fbd => self.draw_fbd(frame),
            }
        });
        vec![frame.into_geometry()]
    }

    fn mouse_interaction(
        &self,
        _state: &Self::State,
        _bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        if self.pan_tool {
            mouse::Interaction::Grab
        } else if self.wire_draft.is_some() {
            mouse::Interaction::Crosshair
        } else {
            mouse::Interaction::Pointer
        }
    }
}
