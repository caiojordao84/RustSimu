use iced::mouse;
use iced::widget::canvas::{self, Canvas, Event, Frame, Geometry, Path, Stroke, Text};
use iced::{Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector};

use crate::model::{
    transform_offset, transform_offset_f32, Potential, SchematicDocument, SchematicItem,
};
use crate::sim::timer_preset_ms;
use crate::symbols::{find_symbol, parse_vector_cmd, VectorPrimitive};

/// Ferramenta ativa no editor esquemático.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorTool {
    Select,
    Pan,
    PlaceSymbol(i32),
    DrawWire(i32), // 4000=Fase, 4001=Neutro, 4002=PE, 4003=DC+, 4004=DC-
    PlaceJunction,
    Delete,
}

/// Mensagens emitidas pelo Canvas interativo para a aplicação principal.
#[derive(Debug, Clone)]
pub enum CanvasMessage {
    SelectItem(Option<i32>),
    MoveItem { id: i32, x1: i32, y1: i32 },
    PlaceComponent { tipo: i32, x: i32, y: i32 },
    AddWire {
        tipo: i32,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
    },
    AddJunction { x: i32, y: i32 },
    DeleteItem(i32),
    SimPressComponent(i32),
    SimReleaseComponent(i32),
    RotatePlacement,
    PanBy(f32, f32),
    ZoomAt { factor: f32, cursor: Point },
    CursorMoved(i32, i32),
    CancelAction,
}

/// Estado local de interação do mouse no Canvas (arraste de objetos, desenho de cabo, pan, botão pulsador segurado).
#[derive(Debug, Default)]
pub struct CanvasInteraction {
    pub dragging_item: Option<(i32, i32, i32)>, // (id, offset_x, offset_y)
    pub wire_start: Option<(i32, i32)>,
    pub panning_from: Option<Point>,
    pub hover_world: (i32, i32),
    pub sim_pressed_id: Option<i32>,
}

pub struct SchematicCanvas<'a> {
    pub doc: &'a SchematicDocument,
    pub tool: EditorTool,
    pub selected_id: Option<i32>,
    pub sim_running: bool,
    pub show_grid: bool,
    pub grid_density: crate::model::GridDensity,
    pub show_texts: bool,
    pub is_dark: bool,
    pub auto_connect: bool,
    pub auto_disconnect: bool,
    pub zoom: f32,
    pub pan: Vector,
    pub placement_orientation: i32,
    pub dragging_symbol: Option<i32>,
}

impl<'a> SchematicCanvas<'a> {
    pub fn view(self) -> Element<'a, CanvasMessage> {
        Canvas::new(self)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    }

    fn screen_to_world(&self, pt: Point) -> (f32, f32) {
        let wx = (pt.x - self.pan.x) / self.zoom;
        let wy = (pt.y - self.pan.y) / self.zoom;
        (wx, wy)
    }

    fn snap_to_grid(wx: f32, wy: f32) -> (i32, i32) {
        let sx = ((wx / 3.0).round() as i32) * 3;
        let sy = ((wy / 3.0).round() as i32) * 3;
        (sx, sy)
    }

    fn world_to_screen(&self, wx: f32, wy: f32) -> Point {
        Point::new(wx * self.zoom + self.pan.x, wy * self.zoom + self.pan.y)
    }

    /// Encontra o componente ou cabo sob a coordenada `(wx, wy)` do mundo (escala `.cad`).
    fn hit_test(&self, wx: i32, wy: i32) -> Option<i32> {
        // Prioriza símbolos sobre cabos
        for item in self.doc.items.iter().rev() {
            if let Some(sym) = find_symbol(item.tipo) {
                let (bx1, by1, bx2, by2) = (
                    sym.bounds.0 / 5,
                    sym.bounds.1 / 5,
                    sym.bounds.2 / 5,
                    sym.bounds.3 / 5,
                );
                let (p1x, p1y) = transform_offset(bx1, by1, item.orientation);
                let (p2x, p2y) = transform_offset(bx2, by2, item.orientation);
                let min_x = (item.x1 + p1x.min(p2x)) - 3;
                let max_x = (item.x1 + p1x.max(p2x)) + 3;
                let min_y = (item.y1 + p1y.min(p2y)) - 3;
                let max_y = (item.y1 + p1y.max(p2y)) + 3;
                if wx >= min_x && wx <= max_x && wy >= min_y && wy <= max_y {
                    return Some(item.id);
                }
            } else if let Some(ref qet_type) = item.qet_type {
                if let Some(parsed) = crate::svg_symbol::get_parsed_svg(qet_type) {
                    let (vx, vy, vw, vh) = parsed.view_box;
                    let min_x = item.x1 + vx.min(vx + vw).round() as i32 - 4;
                    let max_x = item.x1 + vx.max(vx + vw).round() as i32 + 4;
                    let min_y = item.y1 + vy.min(vy + vh).round() as i32 - 4;
                    let max_y = item.y1 + vy.max(vy + vh).round() as i32 + 4;
                    if wx >= min_x && wx <= max_x && wy >= min_y && wy <= max_y {
                        return Some(item.id);
                    }
                } else if (wx - item.x1).abs() <= 15 && (wy - item.y1).abs() <= 15 {
                    return Some(item.id);
                }
            } else if item.is_junction() {
                if (wx - item.x1).abs() <= 3 && (wy - item.y1).abs() <= 3 {
                    return Some(item.id);
                }
            }
        }
        // Em seguida, testa cabos
        for item in self.doc.items.iter().rev() {
            if item.is_wire() {
                let dx = (item.x2 - item.x1) as f32;
                let dy = (item.y2 - item.y1) as f32;
                let len_sq = dx * dx + dy * dy;
                if len_sq >= 1.0 {
                    let t = (((wx - item.x1) as f32) * dx + ((wy - item.y1) as f32) * dy) / len_sq;
                    if (0.0..=1.0).contains(&t) {
                        let px = item.x1 as f32 + t * dx;
                        let py = item.y1 as f32 + t * dy;
                        let dist = ((wx as f32 - px).powi(2) + (wy as f32 - py).powi(2)).sqrt();
                        if dist <= 3.0 {
                            return Some(item.id);
                        }
                    }
                }
            }
        }
        None
    }

    fn draw_sheet_and_cajetin(&self, frame: &mut Frame) {
        // Folha A4 na escala nativa `.cad` (ex: 330 x 240) ou dimensões estendidas de diagrama QET
        let sheet_w = if self.doc.settings.custom_height_mm > 420 {
            self.doc.settings.custom_height_mm as f32 + 30.0
        } else {
            (self.doc.settings.custom_height_mm.clamp(279, 420) as f32) + 45.0
        };
        let sheet_h = if self.doc.settings.custom_width_mm > 320 {
            self.doc.settings.custom_width_mm as f32 + 30.0
        } else {
            (self.doc.settings.custom_width_mm.clamp(210, 320) as f32) + 30.0
        };
        let p0 = self.world_to_screen(0.0, 0.0);
        let size = Size::new(sheet_w * self.zoom, sheet_h * self.zoom);

        let paper_color = if self.is_dark {
            Color::from_rgb(0.12, 0.13, 0.16)
        } else {
            Color::from_rgb(0.99, 0.99, 0.98)
        };
        let grid_color = if self.is_dark {
            Color::from_rgb(0.20, 0.22, 0.27)
        } else {
            Color::from_rgb(0.91, 0.92, 0.94)
        };
        let border_color = if self.is_dark {
            Color::from_rgb(0.55, 0.60, 0.70)
        } else {
            Color::from_rgb(0.15, 0.18, 0.24)
        };
        let text_color = if self.is_dark {
            Color::from_rgb(0.70, 0.75, 0.85)
        } else {
            Color::from_rgb(0.30, 0.35, 0.45)
        };

        // Sombra e fundo da folha
        frame.fill_rectangle(
            Point::new(p0.x + 4.0, p0.y + 4.0),
            size,
            Color::from_rgba(0.0, 0.0, 0.0, 0.22),
        );
        frame.fill_rectangle(p0, size, paper_color);

        // Grade de linhas suaves conforme densidade selecionada
        if self.show_grid {
            let step = self.grid_density.step();
            let grid_stroke = Stroke::default()
                .with_color(grid_color)
                .with_width(1.0);
            let mut gx = step;
            while (gx as f32) < sheet_w - 6.0 {
                let a = self.world_to_screen(gx as f32, 6.0);
                let b = self.world_to_screen(gx as f32, sheet_h - 6.0);
                frame.stroke(&Path::line(a, b), grid_stroke);
                gx += step;
            }
            let mut gy = step;
            while (gy as f32) < sheet_h - 6.0 {
                let a = self.world_to_screen(6.0, gy as f32);
                let b = self.world_to_screen(sheet_w - 6.0, gy as f32);
                frame.stroke(&Path::line(a, b), grid_stroke);
                gy += step;
            }
        }

        // Moldura externa e interna da folha A4
        let border_stroke = Stroke::default()
            .with_color(border_color)
            .with_width(1.5);
        frame.stroke(&Path::rectangle(p0, size), border_stroke);

        let inner_p0 = self.world_to_screen(6.0, 6.0);
        let inner_size = Size::new((sheet_w - 12.0) * self.zoom, (sheet_h - 12.0) * self.zoom);
        frame.stroke(&Path::rectangle(inner_p0, inner_size), border_stroke);

        // Referências de colunas (1..8) e linhas (A..F)
        for col in 0..8 {
            let cx = 6.0 + (col as f32 + 0.5) * ((sheet_w - 12.0) / 8.0);
            let pos = self.world_to_screen(cx, 1.2);
            frame.fill_text(Text {
                content: format!("{}", col + 1),
                position: pos,
                color: text_color,
                size: (3.2 * self.zoom).clamp(8.0, 14.0).into(),
                ..Text::default()
            });
        }
        for (row, ch) in ['A', 'B', 'C', 'D', 'E', 'F'].iter().enumerate() {
            let ry = 6.0 + (row as f32 + 0.5) * ((sheet_h - 12.0) / 6.0);
            let pos = self.world_to_screen(1.5, ry);
            frame.fill_text(Text {
                content: ch.to_string(),
                position: pos,
                color: text_color,
                size: (3.2 * self.zoom).clamp(8.0, 14.0).into(),
                ..Text::default()
            });
        }

        // Cajetín (Bloco de legenda no canto inferior direito)
        let caj_w = 115.0;
        let caj_h = 22.0;
        let caj_x = sheet_w - caj_w - 6.0;
        let caj_y = sheet_h - caj_h - 6.0;
        let cp = self.world_to_screen(caj_x, caj_y);
        let cs = Size::new(caj_w * self.zoom, caj_h * self.zoom);
        let caj_bg = if self.is_dark {
            Color::from_rgb(0.16, 0.18, 0.23)
        } else {
            Color::from_rgb(0.96, 0.97, 0.99)
        };
        frame.fill_rectangle(cp, cs, caj_bg);
        frame.stroke(&Path::rectangle(cp, cs), border_stroke);

        let tb = &self.doc.title_block;
        let font_sz = (3.0 * self.zoom).clamp(7.0, 13.0);
        let caj_text1 = if self.is_dark {
            Color::from_rgb(0.80, 0.84, 0.92)
        } else {
            Color::from_rgb(0.15, 0.18, 0.25)
        };
        let caj_text2 = if self.is_dark {
            Color::from_rgb(0.92, 0.95, 1.0)
        } else {
            Color::from_rgb(0.08, 0.12, 0.22)
        };
        let caj_text3 = if self.is_dark {
            Color::from_rgb(0.72, 0.77, 0.88)
        } else {
            Color::from_rgb(0.25, 0.30, 0.40)
        };
        frame.fill_text(Text {
            content: format!("Entidade: {}  |  Data: {}", tb.entity, tb.drawn_date),
            position: self.world_to_screen(caj_x + 3.0, caj_y + 2.5),
            color: caj_text1,
            size: font_sz.into(),
            ..Text::default()
        });
        frame.fill_text(Text {
            content: format!("Título: {} — {}", tb.title, tb.subtitle),
            position: self.world_to_screen(caj_x + 3.0, caj_y + 8.5),
            color: caj_text2,
            size: (font_sz * 1.08).into(),
            ..Text::default()
        });
        frame.fill_text(Text {
            content: format!(
                "Autor: {}  |  Arquivo: {}  |  Folha: {}",
                tb.drawn_by, tb.file_ref, tb.sheet
            ),
            position: self.world_to_screen(caj_x + 3.0, caj_y + 15.0),
            color: caj_text3,
            size: font_sz.into(),
            ..Text::default()
        });
    }

    pub fn wire_color(&self, item: &SchematicItem) -> (Color, f32) {
        if self.sim_running {
            match item.sim_potential {
                Potential::L1 => (Color::from_rgb(0.92, 0.15, 0.12), 2.5),
                Potential::L2 => (Color::from_rgb(0.90, 0.52, 0.05), 2.5),
                Potential::L3 => (Color::from_rgb(0.65, 0.18, 0.85), 2.5),
                Potential::N => (Color::from_rgb(0.08, 0.48, 0.95), 2.4),
                Potential::PE => (Color::from_rgb(0.12, 0.68, 0.22), 2.0),
                Potential::DcPos => (Color::from_rgb(0.95, 0.12, 0.20), 2.5),
                Potential::DcNeg => (Color::from_rgb(0.15, 0.25, 0.45), 2.4),
                Potential::ShortCircuit => (Color::from_rgb(1.0, 0.0, 0.0), 3.6),
                Potential::None => {
                    if item.color != 0 {
                        let r = (((item.color >> 16) & 0xFF) as f32 / 255.0) * 0.70;
                        let g = (((item.color >> 8) & 0xFF) as f32 / 255.0) * 0.70;
                        let b = ((item.color & 0xFF) as f32 / 255.0) * 0.70;
                        (Color::from_rgb(r, g, b), 1.6)
                    } else if self.is_dark {
                        (Color::from_rgb(0.55, 0.58, 0.66), 1.5)
                    } else {
                        (Color::from_rgb(0.45, 0.48, 0.54), 1.5)
                    }
                }
            }
        } else {
            if item.color != 0 {
                let r = ((item.color >> 16) & 0xFF) as f32 / 255.0;
                let g = ((item.color >> 8) & 0xFF) as f32 / 255.0;
                let b = (item.color & 0xFF) as f32 / 255.0;
                return (Color::from_rgb(r, g, b), 1.8);
            }
            if self.is_dark {
                match item.tipo {
                    4000 | 4018 => (Color::from_rgb(0.94, 0.58, 0.50), 1.8), // Fase / Comando (claro)
                    4009 | 4019 => (Color::from_rgb(0.45, 0.75, 1.0), 1.8),  // Neutro / 0V (azul claro)
                    4002 | 4010 => (Color::from_rgb(0.35, 0.90, 0.45), 1.6), // PE (verde claro)
                    4003 => (Color::from_rgb(1.0, 0.45, 0.48), 1.8),        // DC+ (vermelho claro)
                    4004 => (Color::from_rgb(0.85, 0.88, 0.95), 1.8),        // DC- (azul-acinzentado claro)
                    _ => (Color::from_rgb(0.88, 0.92, 0.98), 1.6),           // Genérico (traço claro)
                }
            } else {
                match item.tipo {
                    4000 | 4018 => (Color::from_rgb(0.55, 0.18, 0.12), 1.8), // Fase / Comando
                    4009 | 4019 => (Color::from_rgb(0.08, 0.38, 0.82), 1.8), // Neutro / 0V
                    4002 | 4010 => (Color::from_rgb(0.12, 0.62, 0.20), 1.6), // PE
                    4003 => (Color::from_rgb(0.82, 0.12, 0.15), 1.8),        // DC+
                    4004 => (Color::from_rgb(0.15, 0.15, 0.20), 1.8),        // DC-
                    _ => (Color::from_rgb(0.15, 0.18, 0.24), 1.6),
                }
            }
        }
    }

    fn draw_symbol_primitives(
        &self,
        frame: &mut Frame,
        origin_x: i32,
        origin_y: i32,
        orientation: i32,
        cmds: &[&str],
        stroke_color: Color,
        fill_active: Option<Color>,
        stroke_width: f32,
    ) {
        let stroke = Stroke::default()
            .with_color(stroke_color)
            .with_width(stroke_width);
        let ox = origin_x as f32;
        let oy = origin_y as f32;

        for &cmd in cmds {
            let Some(prim) = parse_vector_cmd(cmd) else {
                continue;
            };
            match prim {
                VectorPrimitive::Line { x1, y1, x2, y2 } => {
                    let (rx1, ry1) =
                        transform_offset_f32(x1 as f32 / 5.0, y1 as f32 / 5.0, orientation);
                    let (rx2, ry2) =
                        transform_offset_f32(x2 as f32 / 5.0, y2 as f32 / 5.0, orientation);
                    let a = self.world_to_screen(ox + rx1, oy + ry1);
                    let b = self.world_to_screen(ox + rx2, oy + ry2);
                    frame.stroke(&Path::line(a, b), stroke);
                }
                VectorPrimitive::Ellipse { x1, y1, x2, y2 } => {
                    let (rx1, ry1) =
                        transform_offset_f32(x1 as f32 / 5.0, y1 as f32 / 5.0, orientation);
                    let (rx2, ry2) =
                        transform_offset_f32(x2 as f32 / 5.0, y2 as f32 / 5.0, orientation);
                    let cx = ox + (rx1 + rx2) * 0.5;
                    let cy = oy + (ry1 + ry2) * 0.5;
                    let rad = (((rx2 - rx1).abs().max((ry2 - ry1).abs())) * 0.5) * self.zoom;
                    let center = self.world_to_screen(cx, cy);
                    let circle = Path::circle(center, rad);
                    if let Some(fill) = fill_active {
                        frame.fill(&circle, fill);
                    }
                    frame.stroke(&circle, stroke);
                }
                VectorPrimitive::Arc {
                    x1,
                    y1,
                    x2,
                    y2,
                    x3: _,
                    y3: _,
                    x4: _,
                    y4: _,
                } => {
                    let (rx1, ry1) =
                        transform_offset_f32(x1 as f32 / 5.0, y1 as f32 / 5.0, orientation);
                    let (rx2, ry2) =
                        transform_offset_f32(x2 as f32 / 5.0, y2 as f32 / 5.0, orientation);
                    let cx = ox + (rx1 + rx2) * 0.5;
                    let cy = oy + (ry1 + ry2) * 0.5;
                    let rad = (((rx2 - rx1).abs().min((ry2 - ry1).abs())) * 0.45) * self.zoom;
                    let center = self.world_to_screen(cx, cy);
                    frame.stroke(&Path::circle(center, rad), stroke);
                }
                VectorPrimitive::Polygon { pts } => {
                    let path = Path::new(|builder| {
                        for (idx, &(px, py)) in pts.iter().enumerate() {
                            let (rx, ry) =
                                transform_offset_f32(px as f32 / 5.0, py as f32 / 5.0, orientation);
                            let sc = self.world_to_screen(ox + rx, oy + ry);
                            if idx == 0 {
                                builder.move_to(sc);
                            } else {
                                builder.line_to(sc);
                            }
                        }
                        builder.close();
                    });
                    if let Some(fill) = fill_active {
                        frame.fill(&path, fill);
                    }
                    frame.stroke(&path, stroke);
                }
                VectorPrimitive::Text { x, y, text } => {
                    let (rx, ry) =
                        transform_offset_f32(x as f32 / 5.0, y as f32 / 5.0, orientation);
                    let pos = self.world_to_screen(ox + rx - 1.2, oy + ry - 1.0);
                    frame.fill_text(Text {
                        content: text,
                        position: pos,
                        color: stroke_color,
                        size: (3.2 * self.zoom).clamp(8.0, 16.0).into(),
                        ..Text::default()
                    });
                }
            }
        }
    }

    fn draw_svg_symbol(
        &self,
        frame: &mut Frame,
        origin_x: i32,
        origin_y: i32,
        orientation: i32,
        svg_path: &str,
        stroke_color: Color,
        stroke_width: f32,
        fill_active: Option<Color>,
        target_span: f32,
    ) {
        let Some(parsed) = crate::svg_symbol::get_parsed_svg(svg_path) else {
            return;
        };

        let stroke_regular = Stroke::default()
            .with_color(stroke_color)
            .with_width(stroke_width);

        let mut stroke_dashed = stroke_regular;
        stroke_dashed.line_dash = iced::widget::canvas::stroke::LineDash {
            segments: &[4.0, 3.0],
            offset: 0,
        };

        let mut stroke_qet_housing = Stroke::default()
            .with_color(stroke_color)
            .with_width((stroke_width * 0.65).max(0.8));
        stroke_qet_housing.line_dash = iced::widget::canvas::stroke::LineDash {
            segments: &[2.5, 2.5],
            offset: 0,
        };

        let default_fill = if self.is_dark {
            Color::from_rgba(0.90, 0.93, 0.98, 0.08)
        } else {
            Color::from_rgba(0.12, 0.14, 0.20, 0.08)
        };
        let white_fill = if self.is_dark {
            Color::from_rgb(0.16, 0.18, 0.23)
        } else {
            Color::from_rgb(0.99, 0.99, 1.0)
        };
        let ox = origin_x as f32;
        let oy = origin_y as f32;

        let resolve_shape_fill = |fill_mode: crate::svg_symbol::SvgFill| -> Option<Color> {
            match fill_mode {
                crate::svg_symbol::SvgFill::Solid => Some(stroke_color),
                crate::svg_symbol::SvgFill::White => Some(fill_active.unwrap_or(white_fill)),
                crate::svg_symbol::SvgFill::Tint => Some(fill_active.unwrap_or(default_fill)),
                crate::svg_symbol::SvgFill::None => {
                    if parsed.is_qet_native {
                        None
                    } else {
                        fill_active
                    }
                }
            }
        };

        for shape in &parsed.shapes {
            match shape {
                crate::svg_symbol::SvgShape::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    dashed,
                } => {
                    let (lx1, ly1) = parsed.map_point(*x1, *y1, target_span);
                    let (lx2, ly2) = parsed.map_point(*x2, *y2, target_span);
                    let (rx1, ry1) = transform_offset_f32(lx1, ly1, orientation);
                    let (rx2, ry2) = transform_offset_f32(lx2, ly2, orientation);
                    let a = self.world_to_screen(ox + rx1, oy + ry1);
                    let b = self.world_to_screen(ox + rx2, oy + ry2);
                    let stroke = if *dashed {
                        stroke_dashed
                    } else {
                        stroke_regular
                    };
                    frame.stroke(&Path::line(a, b), stroke);
                }
                crate::svg_symbol::SvgShape::Polyline { points } => {
                    if points.len() < 2 {
                        continue;
                    }
                    let path = Path::new(|builder| {
                        for (idx, &(px, py)) in points.iter().enumerate() {
                            let (lx, ly) = parsed.map_point(px, py, target_span);
                            let (rx, ry) = transform_offset_f32(lx, ly, orientation);
                            let sc = self.world_to_screen(ox + rx, oy + ry);
                            if idx == 0 {
                                builder.move_to(sc);
                            } else {
                                builder.line_to(sc);
                            }
                        }
                    });
                    frame.stroke(&path, stroke_regular);
                }
                crate::svg_symbol::SvgShape::Polygon { points, fill } => {
                    if points.is_empty() {
                        continue;
                    }
                    let path = Path::new(|builder| {
                        for (idx, &(px, py)) in points.iter().enumerate() {
                            let (lx, ly) = parsed.map_point(px, py, target_span);
                            let (rx, ry) = transform_offset_f32(lx, ly, orientation);
                            let sc = self.world_to_screen(ox + rx, oy + ry);
                            if idx == 0 {
                                builder.move_to(sc);
                            } else {
                                builder.line_to(sc);
                            }
                        }
                        builder.close();
                    });
                    if let Some(fill_col) = resolve_shape_fill(*fill) {
                        frame.fill(&path, fill_col);
                    }
                    frame.stroke(&path, stroke_regular);
                }
                crate::svg_symbol::SvgShape::Circle {
                    cx,
                    cy,
                    r,
                    is_terminal,
                    fill,
                } => {
                    if *is_terminal {
                        continue;
                    }
                    let (lx, ly) = parsed.map_point(*cx, *cy, target_span);
                    let (rx, ry) = transform_offset_f32(lx, ly, orientation);
                    let center = self.world_to_screen(ox + rx, oy + ry);
                    let rad = if parsed.is_qet_native {
                        r * self.zoom
                    } else if parsed.is_cade_svg {
                        (r / 5.0) * self.zoom
                    } else {
                        (r * (target_span / (parsed.term_bottom_y - parsed.term_top_y).abs().max(1.0)))
                            * self.zoom
                    };
                    let circle_path = Path::circle(center, rad.max(1.2));
                    if let Some(fill_col) = resolve_shape_fill(*fill) {
                        frame.fill(&circle_path, fill_col);
                    }
                    frame.stroke(&circle_path, stroke_regular);
                }
                crate::svg_symbol::SvgShape::Ellipse {
                    cx,
                    cy,
                    rx,
                    ry,
                    fill,
                } => {
                    let steps = 28;
                    let ellipse_path = Path::new(|builder| {
                        for i in 0..steps {
                            let ang = (i as f32 / steps as f32) * 2.0 * std::f32::consts::PI;
                            let px = cx + rx * ang.cos();
                            let py = cy + ry * ang.sin();
                            let (lx, ly) = parsed.map_point(px, py, target_span);
                            let (rot_x, rot_y) = transform_offset_f32(lx, ly, orientation);
                            let sc = self.world_to_screen(ox + rot_x, oy + rot_y);
                            if i == 0 {
                                builder.move_to(sc);
                            } else {
                                builder.line_to(sc);
                            }
                        }
                        builder.close();
                    });
                    if let Some(fill_col) = resolve_shape_fill(*fill) {
                        frame.fill(&ellipse_path, fill_col);
                    }
                    frame.stroke(&ellipse_path, stroke_regular);
                }
                crate::svg_symbol::SvgShape::Rect {
                    x,
                    y,
                    w,
                    h,
                    dashed,
                    fill,
                } => {
                    let pts = [
                        parsed.map_point(*x, *y, target_span),
                        parsed.map_point(*x + *w, *y, target_span),
                        parsed.map_point(*x + *w, *y + *h, target_span),
                        parsed.map_point(*x, *y + *h, target_span),
                    ];
                    let rect_path = Path::new(|builder| {
                        for (idx, &(lx, ly)) in pts.iter().enumerate() {
                            let (rx, ry) = transform_offset_f32(lx, ly, orientation);
                            let sc = self.world_to_screen(ox + rx, oy + ry);
                            if idx == 0 {
                                builder.move_to(sc);
                            } else {
                                builder.line_to(sc);
                            }
                        }
                        builder.close();
                    });
                    if let Some(fill_col) = resolve_shape_fill(*fill) {
                        frame.fill(&rect_path, fill_col);
                    }
                    let stroke = if *dashed && parsed.is_qet_native {
                        stroke_qet_housing
                    } else if *dashed {
                        stroke_dashed
                    } else {
                        stroke_regular
                    };
                    frame.stroke(&rect_path, stroke);
                }
                crate::svg_symbol::SvgShape::Text {
                    x,
                    y,
                    text,
                    size,
                    rotation,
                } => {
                    let (lx, ly) = parsed.map_point(*x, *y, target_span);
                    let (rx, ry) = transform_offset_f32(lx, ly, orientation);
                    let raw_pos = self.world_to_screen(ox + rx, oy + ry);
                    let (font_size, offset_y) = if parsed.is_qet_native {
                        let fs = (size * 1.15 * self.zoom).clamp(6.0, 22.0);
                        (fs, -fs * 0.82)
                    } else if parsed.is_cade_svg {
                        let fs = (size * 0.25 * self.zoom).clamp(8.0, 16.0);
                        (fs, 0.0)
                    } else {
                        let fs = (size * 0.3 * self.zoom).clamp(8.0, 16.0);
                        (fs, 0.0)
                    };

                    let total_rot_deg = match orientation.rem_euclid(4) {
                        1 => *rotation + 90.0,
                        2 => *rotation + 180.0,
                        3 => *rotation + 270.0,
                        _ => *rotation,
                    };

                    if total_rot_deg.abs() > 0.1 {
                        let rad = total_rot_deg.to_radians();
                        frame.with_save(|f| {
                            f.translate(Vector::new(raw_pos.x, raw_pos.y));
                            f.rotate(rad);
                            f.fill_text(Text {
                                content: text.clone(),
                                position: Point::new(0.0, offset_y),
                                color: stroke_color,
                                size: font_size.into(),
                                ..Text::default()
                            });
                        });
                    } else {
                        frame.fill_text(Text {
                            content: text.clone(),
                            position: Point::new(raw_pos.x, raw_pos.y + offset_y),
                            color: stroke_color,
                            size: font_size.into(),
                            ..Text::default()
                        });
                    }
                }
            }
        }
    }

    fn draw_item(&self, frame: &mut Frame, item: &SchematicItem) {
        let is_selected = self.selected_id == Some(item.id);

        if item.is_wire() {
            let (mut color, mut width) = self.wire_color(item);
            if is_selected {
                color = Color::from_rgb(0.0, 0.55, 1.0);
                width += 1.0;
            }
            let a = self.world_to_screen(item.x1 as f32, item.y1 as f32);
            let b = self.world_to_screen(item.x2 as f32, item.y2 as f32);
            let stroke = Stroke::default().with_color(color).with_width(width);
            frame.stroke(&Path::line(a, b), stroke);
            return;
        }

        if item.is_junction() {
            let (color, _) = self.wire_color(item);
            let center = self.world_to_screen(item.x1 as f32, item.y1 as f32);
            frame.fill(&Path::circle(center, (0.9 * self.zoom).clamp(2.2, 5.0)), color);
            return;
        }

        // Texto gráfico (tipo == 8)
        if item.tipo == 8 {
            if !item.trailing_str.is_empty() {
                let pos = self.world_to_screen(item.x1 as f32, item.y1 as f32);
                let text_color = if self.is_dark {
                    Color::from_rgb(0.90, 0.93, 0.98)
                } else {
                    Color::from_rgb(0.12, 0.16, 0.28)
                };
                frame.fill_text(Text {
                    content: item.trailing_str.clone(),
                    position: pos,
                    color: text_color,
                    size: (3.2 * self.zoom).clamp(8.5, 15.0).into(),
                    ..Text::default()
                });
            }
            return;
        }

        // Linha gráfica auxiliar (tipo == 1)
        if item.tipo == 1 {
            let a = self.world_to_screen(item.x1 as f32, item.y1 as f32);
            let b = self.world_to_screen(item.x2 as f32, item.y2 as f32);
            let line_color = if self.is_dark {
                Color::from_rgb(0.78, 0.82, 0.90)
            } else {
                Color::from_rgb(0.25, 0.28, 0.35)
            };
            frame.stroke(
                &Path::line(a, b),
                Stroke::default()
                    .with_color(line_color)
                    .with_width(1.2),
            );
            return;
        }

        // Retângulo gráfico (tipo == 2)
        if item.tipo == 2 {
            let min_x = item.x1.min(item.x2) as f32;
            let min_y = item.y1.min(item.y2) as f32;
            let w = (item.x2 - item.x1).abs() as f32;
            let h = (item.y2 - item.y1).abs() as f32;
            let tl = self.world_to_screen(min_x, min_y);
            let sz = Size::new(w * self.zoom, h * self.zoom);
            let rect_color = if self.is_dark {
                Color::from_rgb(0.78, 0.82, 0.90)
            } else {
                Color::from_rgb(0.25, 0.28, 0.35)
            };
            frame.stroke(
                &Path::rectangle(tl, sz),
                Stroke::default()
                    .with_color(rect_color)
                    .with_width(1.3),
            );
            return;
        }

        // Elemento nativo importado de projeto QElectroTech (.qet)
        if let Some(ref qet_type) = item.qet_type {
            let active_in_sim = self.sim_running && (item.sim_active || item.sim_manual_latched);
            let stroke_color = if is_selected {
                Color::from_rgb(0.05, 0.45, 0.90)
            } else if self.sim_running && item.sim_fault {
                Color::from_rgb(0.95, 0.15, 0.10)
            } else if active_in_sim {
                Color::from_rgb(0.08, 0.70, 0.25)
            } else if self.is_dark {
                Color::from_rgb(0.90, 0.93, 0.98)
            } else {
                Color::from_rgb(0.10, 0.12, 0.18)
            };

            let fill_active = if active_in_sim {
                Some(Color::from_rgba(0.15, 0.85, 0.35, 0.35))
            } else {
                None
            };

            if is_selected {
                if let Some(parsed) = crate::svg_symbol::get_parsed_svg(qet_type) {
                    let (vx, vy, vw, vh) = parsed.view_box;
                    let (p1x, p1y) = transform_offset_f32(vx, vy, item.orientation);
                    let (p2x, p2y) = transform_offset_f32(vx + vw, vy + vh, item.orientation);
                    let min_x = (item.x1 as f32 + p1x.min(p2x)) - 2.0;
                    let max_x = (item.x1 as f32 + p1x.max(p2x)) + 2.0;
                    let min_y = (item.y1 as f32 + p1y.min(p2y)) - 2.0;
                    let max_y = (item.y1 as f32 + p1y.max(p2y)) + 2.0;
                    let tl = self.world_to_screen(min_x, min_y);
                    let sz = Size::new((max_x - min_x) * self.zoom, (max_y - min_y) * self.zoom);
                    frame.fill_rectangle(tl, sz, Color::from_rgba(0.10, 0.55, 0.98, 0.10));
                    frame.stroke(
                        &Path::rectangle(tl, sz),
                        Stroke::default()
                            .with_color(Color::from_rgb(0.10, 0.55, 0.98))
                            .with_width(1.2),
                    );
                }
            }

            self.draw_svg_symbol(
                frame,
                item.x1,
                item.y1,
                item.orientation,
                qet_type,
                stroke_color,
                1.5,
                fill_active,
                1.0,
            );

            // Desenha tag do elemento
            if !item.tag.is_empty() {
                let tpos = self.world_to_screen((item.x1 - 6) as f32, (item.y1 - 6) as f32);
                let tag_color = if self.is_dark {
                    Color::from_rgb(0.45, 0.75, 1.0)
                } else {
                    Color::from_rgb(0.06, 0.22, 0.58)
                };
                frame.fill_text(Text {
                    content: item.tag.clone(),
                    position: tpos,
                    color: tag_color,
                    size: (3.0 * self.zoom).clamp(8.0, 15.0).into(),
                    ..Text::default()
                });
            }

            return;
        }

        let Some(sym) = find_symbol(item.tipo) else {
            return;
        };

        // Caixa de seleção quando selecionado
        if is_selected {
            let (bx1, by1, bx2, by2) = (
                sym.bounds.0 / 5,
                sym.bounds.1 / 5,
                sym.bounds.2 / 5,
                sym.bounds.3 / 5,
            );
            let (p1x, p1y) = transform_offset(bx1, by1, item.orientation);
            let (p2x, p2y) = transform_offset(bx2, by2, item.orientation);
            let min_x = (item.x1 + p1x.min(p2x) - 1) as f32;
            let max_x = (item.x1 + p1x.max(p2x) + 1) as f32;
            let min_y = (item.y1 + p1y.min(p2y) - 1) as f32;
            let max_y = (item.y1 + p1y.max(p2y) + 1) as f32;
            let tl = self.world_to_screen(min_x, min_y);
            let sz = Size::new((max_x - min_x) * self.zoom, (max_y - min_y) * self.zoom);
            frame.fill_rectangle(tl, sz, Color::from_rgba(0.10, 0.55, 0.98, 0.10));
            frame.stroke(
                &Path::rectangle(tl, sz),
                Stroke::default()
                    .with_color(Color::from_rgb(0.10, 0.55, 0.98))
                    .with_width(1.2),
            );
        }

        // Escolhe comandos vetoriais e cores conforme o estado de simulação
        let is_motor_active = matches!(item.tipo, 1000 | 1001 | 1002)
            && (item.sim_motor_dir != 0 || item.sim_motor_mode != crate::model::MotorMode::Off);
        let active_in_sim = self.sim_running && (item.sim_active || is_motor_active);
        let cmds = if active_in_sim {
            sym.active_cmds
        } else {
            sym.idle_cmds
        };

        let is_lamp = item.is_lamp_or_signaling();

        let stroke_color = if is_selected {
            Color::from_rgb(0.05, 0.45, 0.90)
        } else if self.sim_running && item.sim_fault {
            Color::from_rgb(0.95, 0.15, 0.10)
        } else if active_in_sim {
            if is_lamp {
                match item.color {
                    2 => Color::from_rgb(0.88, 0.14, 0.12), // Vermelho
                    3 => Color::from_rgb(0.85, 0.70, 0.05), // Amarelo
                    4 => Color::from_rgb(0.10, 0.45, 0.90), // Azul
                    0 => Color::from_rgb(0.70, 0.70, 0.70), // Branco
                    _ => Color::from_rgb(0.08, 0.65, 0.24), // Verde (código 1 ou outro)
                }
            } else {
                match item.tipo {
                    1000 | 1001 | 1002 => Color::from_rgb(0.08, 0.65, 0.24),
                    _ => Color::from_rgb(0.88, 0.14, 0.12),
                }
            }
        } else if self.is_dark {
            Color::from_rgb(0.90, 0.93, 0.98)
        } else {
            Color::from_rgb(0.10, 0.12, 0.18)
        };

        let fill_active = if active_in_sim {
            if is_lamp {
                match item.color {
                    2 => Some(Color::from_rgba(0.95, 0.20, 0.15, 0.75)), // Vermelho
                    3 => Some(Color::from_rgba(0.98, 0.85, 0.15, 0.75)), // Amarelo
                    4 => Some(Color::from_rgba(0.20, 0.55, 0.95, 0.75)), // Azul
                    0 => Some(Color::from_rgba(0.95, 0.95, 0.95, 0.85)), // Branco
                    _ => Some(Color::from_rgba(0.20, 0.95, 0.40, 0.75)), // Verde (padrão)
                }
            } else {
                match item.tipo {
                    9000 | 9004 | 9005 | 9013 => Some(Color::from_rgba(0.95, 0.25, 0.20, 0.35)),
                    1000 | 1001 | 1002 => Some(Color::from_rgba(0.15, 0.82, 0.35, 0.30)),
                    _ => None,
                }
            }
        } else if self.sim_running && item.sim_fault {
            Some(Color::from_rgba(0.95, 0.20, 0.15, 0.30))
        } else if is_lamp {
            // Em repouso (idle), preenchimento suave da lente para indicar visualmente a cor configurada
            match item.color {
                2 => Some(Color::from_rgba(0.90, 0.15, 0.15, 0.18)), // Vermelho suave
                3 => Some(Color::from_rgba(0.95, 0.85, 0.10, 0.22)), // Amarelo suave
                4 => Some(Color::from_rgba(0.15, 0.45, 0.90, 0.18)), // Azul suave
                0 => Some(Color::from_rgba(0.90, 0.90, 0.90, 0.25)), // Branco suave
                _ => Some(Color::from_rgba(0.10, 0.80, 0.20, 0.18)), // Verde suave (padrão)
            }
        } else {
            None
        };

        if let Some((svg_idle, svg_active)) = sym.svg_paths() {
            let svg_path = if active_in_sim {
                svg_active
            } else {
                svg_idle
            };
            let max_pin_y = sym.pins.iter().map(|p| p.conn_pos.1).fold(0, i32::max) as f32;
            let target_span = if max_pin_y > 0.0 { max_pin_y } else { 12.0 };
            self.draw_svg_symbol(
                frame,
                item.x1,
                item.y1,
                item.orientation,
                svg_path,
                stroke_color,
                1.8,
                fill_active,
                target_span,
            );
        } else {
            self.draw_symbol_primitives(
                frame,
                item.x1,
                item.y1,
                item.orientation,
                cmds,
                stroke_color,
                fill_active,
                1.6,
            );
        }

        // Desenha bornes (pinos) e seus números/nomes
        for (idx, pin_label, (wx, wy)) in sym.world_pins(item) {
            let pt = self.world_to_screen(wx as f32, wy as f32);
            frame.stroke(
                &Path::circle(pt, (0.55 * self.zoom).clamp(1.5, 3.2)),
                Stroke::default()
                    .with_color(Color::from_rgb(0.75, 0.18, 0.18))
                    .with_width(1.0),
            );
            if self.show_texts && self.doc.settings.show_pin_refs >= 1 && item.pin_visibility[idx] == 1 {
                let p_def = &sym.pins[idx];
                let (lx, ly) =
                    transform_offset(p_def.label_pos.0, p_def.label_pos.1, item.orientation);
                let lpos = self.world_to_screen(
                    (item.x1 + lx) as f32 + 0.4,
                    (item.y1 + ly) as f32 - 0.5,
                );
                let pin_color = if self.is_dark {
                    Color::from_rgb(0.75, 0.80, 0.90)
                } else {
                    Color::from_rgb(0.32, 0.36, 0.44)
                };
                frame.fill_text(Text {
                    content: pin_label,
                    position: lpos,
                    color: pin_color,
                    size: (2.4 * self.zoom).clamp(6.5, 11.5).into(),
                    ..Text::default()
                });
            }
        }

        // Desenha Tag (-KM1, -Q1, -S1, -M1)
        if self.show_texts && item.show_tag == 1 && !item.tag.is_empty() {
            let (tx, ty) = match item.tipo {
                3000..=3009 => (item.x1 - 5, item.y1 - 5),
                1000..=1012 => (item.x1 - 8, item.y1 + 9),
                _ => (item.x1 - 8, item.y1 + 3),
            };
            let tpos = self.world_to_screen(tx as f32, ty as f32);
            let tag_color = if self.is_dark {
                Color::from_rgb(0.45, 0.75, 1.0)
            } else {
                Color::from_rgb(0.06, 0.22, 0.58)
            };
            frame.fill_text(Text {
                content: item.tag.clone(),
                position: tpos,
                color: tag_color,
                size: (3.0 * self.zoom).clamp(7.5, 14.0).into(),
                ..Text::default()
            });
        }

        // Desenha Valor / Contagem do Temporizador / Status do Motor
        if self.sim_running && (item.tipo == 9004 || item.tipo == 9005) {
            let sec = item.sim_timer_elapsed_ms as f64 / 1000.0;
            let preset = timer_preset_ms(&item.pins, &item.value) as f64 / 1000.0;
            let vpos = self.world_to_screen((item.x1 + 5) as f32, (item.y1 + 4) as f32);
            frame.fill_text(Text {
                content: format!("{sec:.1}s/{preset:.0}s"),
                position: vpos,
                color: Color::from_rgb(0.80, 0.15, 0.10),
                size: (2.7 * self.zoom).clamp(7.0, 12.5).into(),
                ..Text::default()
            });
        } else if self.sim_running && (item.tipo == 1000 || item.tipo == 1001) {
            let status_txt = if item.sim_fault {
                "FALTA DE FASE!".to_string()
            } else if item.sim_motor_dir != 0 {
                let dir_str = if item.sim_motor_dir == 1 { "CW" } else { "CCW" };
                format!("{} ({dir_str})", item.sim_motor_mode.label())
            } else {
                String::new()
            };
            if !status_txt.is_empty() {
                let spos = self.world_to_screen((item.x1 - 2) as f32, (item.y1 + 25) as f32);
                frame.fill_text(Text {
                    content: status_txt,
                    position: spos,
                    color: if item.sim_fault {
                        Color::from_rgb(0.92, 0.12, 0.10)
                    } else {
                        Color::from_rgb(0.06, 0.58, 0.22)
                    },
                    size: (2.8 * self.zoom).clamp(7.5, 13.0).into(),
                    ..Text::default()
                });
            }
        } else if self.show_texts && item.show_value == 1 && !item.value.is_empty() {
            let vpos = self.world_to_screen((item.x1 + 5) as f32, (item.y1 + 5) as f32);
            let val_color = if self.is_dark {
                Color::from_rgb(0.75, 0.80, 0.90)
            } else {
                Color::from_rgb(0.25, 0.28, 0.35)
            };
            frame.fill_text(Text {
                content: format!("{}s", item.value),
                position: vpos,
                color: val_color,
                size: (2.7 * self.zoom).clamp(7.0, 12.5).into(),
                ..Text::default()
            });
        }
    }
}

impl<'a> canvas::Program<CanvasMessage> for SchematicCanvas<'a> {
    type State = CanvasInteraction;

    fn update(
        &self,
        state: &mut Self::State,
        event: Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (canvas::event::Status, Option<CanvasMessage>) {
        // Garante que soltar o botão do mouse sempre libera botões pulsadores ou arrastes,
        // mesmo que o cursor tenha saído levemente dos limites do Canvas.
        match event {
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.dragging_item = None;
                if let Some(pressed_id) = state.sim_pressed_id.take() {
                    return (
                        canvas::event::Status::Captured,
                        Some(CanvasMessage::SimReleaseComponent(pressed_id)),
                    );
                }
                if let Some(drag_tipo) = self.dragging_symbol {
                    if let Some(cursor_pos) = cursor.position_in(bounds) {
                        let (wx_f, wy_f) = self.screen_to_world(cursor_pos);
                        let (snapped_x, snapped_y) = Self::snap_to_grid(wx_f, wy_f);
                        return (
                            canvas::event::Status::Captured,
                            Some(CanvasMessage::PlaceComponent {
                                tipo: drag_tipo,
                                x: snapped_x,
                                y: snapped_y,
                            }),
                        );
                    }
                }
                return (canvas::event::Status::Captured, None);
            }
            Event::Mouse(mouse::Event::ButtonReleased(
                mouse::Button::Middle | mouse::Button::Right,
            )) => {
                state.panning_from = None;
                return (canvas::event::Status::Captured, None);
            }
            _ => {}
        }

        let Some(cursor_pos) = cursor.position_in(bounds) else {
            return (canvas::event::Status::Ignored, None);
        };

        let (wx_f, wy_f) = self.screen_to_world(cursor_pos);
        let (snapped_x, snapped_y) = Self::snap_to_grid(wx_f, wy_f);

        match event {
            Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                state.hover_world = (snapped_x, snapped_y);

                if let Some(prev_pt) = state.panning_from {
                    let dx = cursor_pos.x - prev_pt.x;
                    let dy = cursor_pos.y - prev_pt.y;
                    state.panning_from = Some(cursor_pos);
                    return (
                        canvas::event::Status::Captured,
                        Some(CanvasMessage::PanBy(dx, dy)),
                    );
                }

                if !self.sim_running {
                    if let Some((drag_id, off_x, off_y)) = state.dragging_item {
                        return (
                            canvas::event::Status::Captured,
                            Some(CanvasMessage::MoveItem {
                                id: drag_id,
                                x1: snapped_x - off_x,
                                y1: snapped_y - off_y,
                            }),
                        );
                    }
                }

                (
                    canvas::event::Status::Captured,
                    Some(CanvasMessage::CursorMoved(snapped_x, snapped_y)),
                )
            }

            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Middle)) => {
                state.panning_from = Some(cursor_pos);
                (canvas::event::Status::Captured, None)
            }

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Middle)) => {
                state.panning_from = None;
                (canvas::event::Status::Captured, None)
            }

            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right)) => {
                if state.wire_start.is_some() {
                    state.wire_start = None;
                    return (canvas::event::Status::Captured, Some(CanvasMessage::CancelAction));
                }
                if self.tool != EditorTool::Select || self.dragging_symbol.is_some() {
                    return (
                        canvas::event::Status::Captured,
                        Some(CanvasMessage::CancelAction),
                    );
                }
                state.panning_from = Some(cursor_pos);
                (canvas::event::Status::Captured, None)
            }

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Right)) => {
                state.panning_from = None;
                (canvas::event::Status::Captured, None)
            }

            Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                let y = match delta {
                    mouse::ScrollDelta::Lines { y, .. } => y,
                    mouse::ScrollDelta::Pixels { y, .. } => y / 40.0,
                };
                if y.abs() > 0.01 {
                    let factor = if y > 0.0 { 1.12 } else { 1.0 / 1.12 };
                    return (
                        canvas::event::Status::Captured,
                        Some(CanvasMessage::ZoomAt {
                            factor,
                            cursor: cursor_pos,
                        }),
                    );
                }
                (canvas::event::Status::Ignored, None)
            }

            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                let hit = self.hit_test(wx_f.round() as i32, wy_f.round() as i32);

                if self.sim_running {
                    if let Some(id) = hit {
                        state.sim_pressed_id = Some(id);
                        return (
                            canvas::event::Status::Captured,
                            Some(CanvasMessage::SimPressComponent(id)),
                        );
                    }
                    return (canvas::event::Status::Captured, None);
                }

                match self.tool {
                    EditorTool::Pan => {
                        state.panning_from = Some(cursor_pos);
                        return (canvas::event::Status::Captured, None);
                    }
                    EditorTool::Select => {
                        if let Some(id) = hit {
                            if let Some(item) = self.doc.items.iter().find(|i| i.id == id) {
                                if !item.is_wire() {
                                    state.dragging_item =
                                        Some((id, snapped_x - item.x1, snapped_y - item.y1));
                                }
                            }
                            return (
                                canvas::event::Status::Captured,
                                Some(CanvasMessage::SelectItem(Some(id))),
                            );
                        } else {
                            return (
                                canvas::event::Status::Captured,
                                Some(CanvasMessage::SelectItem(None)),
                            );
                        }
                    }
                    EditorTool::PlaceSymbol(tipo) => (
                        canvas::event::Status::Captured,
                        Some(CanvasMessage::PlaceComponent {
                            tipo,
                            x: snapped_x,
                            y: snapped_y,
                        }),
                    ),
                    EditorTool::PlaceJunction => (
                        canvas::event::Status::Captured,
                        Some(CanvasMessage::AddJunction {
                            x: snapped_x,
                            y: snapped_y,
                        }),
                    ),
                    EditorTool::DrawWire(wire_tipo) => {
                        if let Some((sx, sy)) = state.wire_start {
                            if (sx, sy) != (snapped_x, snapped_y) {
                                // Segmento ortogonal (mantém X ou Y se próximo, ou traça direto)
                                state.wire_start = Some((snapped_x, snapped_y));
                                return (
                                    canvas::event::Status::Captured,
                                    Some(CanvasMessage::AddWire {
                                        tipo: wire_tipo,
                                        x1: sx,
                                        y1: sy,
                                        x2: snapped_x,
                                        y2: snapped_y,
                                    }),
                                );
                            }
                        } else {
                            state.wire_start = Some((snapped_x, snapped_y));
                        }
                        (canvas::event::Status::Captured, None)
                    }
                    EditorTool::Delete => {
                        if let Some(id) = hit {
                            return (
                                canvas::event::Status::Captured,
                                Some(CanvasMessage::DeleteItem(id)),
                            );
                        }
                        (canvas::event::Status::Captured, None)
                    }
                }
            }

            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                state.dragging_item = None;
                if matches!(self.tool, EditorTool::Pan) {
                    state.panning_from = None;
                }
                (canvas::event::Status::Captured, None)
            }

            _ => (canvas::event::Status::Ignored, None),
        }
    }

    fn draw(
        &self,
        state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // Fundo da área de trabalho do CADe_SIMU
        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            Color::from_rgb(0.82, 0.85, 0.89),
        );

        // Folha A4 + grade + Cajetín
        self.draw_sheet_and_cajetin(&mut frame);

        // Desenha cabos primeiro, depois componentes e junções por cima
        for item in &self.doc.items {
            if item.is_wire() {
                self.draw_item(&mut frame, item);
            }
        }
        for item in &self.doc.items {
            if !item.is_wire() {
                self.draw_item(&mut frame, item);
            }
        }

        // Preview fantasma ao posicionar símbolo ou traçar cabo
        if !self.sim_running {
            let (hx, hy) = state.hover_world;
            let active_sym = match self.tool {
                EditorTool::PlaceSymbol(tipo) => Some(tipo),
                _ => self.dragging_symbol,
            };
            if let Some(tipo) = active_sym {
                if let Some(sym) = find_symbol(tipo) {
                    if let Some((svg_idle, _)) = sym.svg_paths() {
                        let max_pin_y = sym.pins.iter().map(|p| p.conn_pos.1).fold(0, i32::max) as f32;
                        let target_span = if max_pin_y > 0.0 { max_pin_y } else { 12.0 };
                        self.draw_svg_symbol(
                            &mut frame,
                            hx,
                            hy,
                            self.placement_orientation,
                            svg_idle,
                            Color::from_rgba(0.10, 0.48, 0.92, 0.65),
                            1.8,
                            None,
                            target_span,
                        );
                    } else {
                        self.draw_symbol_primitives(
                            &mut frame,
                            hx,
                            hy,
                            self.placement_orientation,
                            sym.idle_cmds,
                            Color::from_rgba(0.10, 0.48, 0.92, 0.65),
                            None,
                            1.8,
                        );
                    }
                }
            }
            match self.tool {
                EditorTool::DrawWire(_) => {
                    let hpt = self.world_to_screen(hx as f32, hy as f32);
                    frame.stroke(
                        &Path::circle(hpt, 3.5 * self.zoom),
                        Stroke::default()
                            .with_color(Color::from_rgb(0.10, 0.50, 0.95))
                            .with_width(1.5),
                    );
                    if let Some((sx, sy)) = state.wire_start {
                        let spt = self.world_to_screen(sx as f32, sy as f32);
                        frame.stroke(
                            &Path::line(spt, hpt),
                            Stroke::default()
                                .with_color(Color::from_rgba(0.10, 0.50, 0.95, 0.80))
                                .with_width(2.0),
                        );
                    }
                }
                EditorTool::PlaceJunction => {
                    let hpt = self.world_to_screen(hx as f32, hy as f32);
                    frame.fill(
                        &Path::circle(hpt, 3.8 * self.zoom),
                        Color::from_rgba(0.10, 0.50, 0.95, 0.75),
                    );
                }
                _ => {}
            }
        }

        vec![frame.into_geometry()]
    }
}
