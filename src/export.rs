//! Módulo de Exportação Real (Folha → SVG / PNG / PDF / CAD).
//!
//! Implementa os requisitos de P1-#9 com fidelidade geométrica 1:1 à renderização
//! da tela (`canvas_view.rs`):
//! - **SVG**: Exporta folha (página) ou seleção de elementos em gráfico vetorial padrão.
//! - **PNG**: Exporta folha ou seleção renderizada com alta resolução via `resvg`/`tiny-skia`.
//! - **PDF**: Exporta projeto multi-páginas (todas as folhas), folha única ou seleção de elementos.
//! - **CAD**: Exporta folha em formato byte-exato original do CADe_SIMU (.cad).
//! - **Cajetín ON / OFF**: Permite incluir ou ocultar moldura, margens e bloco de legenda.

use std::fmt;
use std::io::Write;

use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::model::{
    transform_offset, transform_offset_f32, ProjectPage, SchematicDocument, SchematicItem,
};
use crate::symbols::find_symbol_ref;

/// Escala base do SVG em pixels por unidade de mundo CADe_SIMU.
/// Corresponde exatamente ao zoom 3.2x padrão do `canvas_view.rs`,
/// garantindo espessuras de linha, raios de círculos, tipografia e proporções 100% idênticas à tela.
pub const SVG_SCALE: f32 = 3.2;

/// Formatos de exportação suportados pelo CADe_SIMU.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    Cad,
    Pdf,
    Svg,
    Png,
}

impl ExportFormat {
    pub const ALL: [ExportFormat; 4] = [
        ExportFormat::Cad,
        ExportFormat::Pdf,
        ExportFormat::Svg,
        ExportFormat::Png,
    ];

    pub fn extension(&self) -> &'static str {
        match self {
            ExportFormat::Cad => "cad",
            ExportFormat::Pdf => "pdf",
            ExportFormat::Svg => "svg",
            ExportFormat::Png => "png",
        }
    }

    pub fn filter_label(&self) -> &'static str {
        match self {
            ExportFormat::Cad => "CADe_SIMU Original (*.cad)",
            ExportFormat::Pdf => "Documento PDF (*.pdf)",
            ExportFormat::Svg => "Gráfico Vetorial SVG (*.svg)",
            ExportFormat::Png => "Imagem PNG (*.png)",
        }
    }
}

impl fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExportFormat::Cad => write!(f, "CADe_SIMU Original (.cad)"),
            ExportFormat::Pdf => write!(f, "Documento PDF (.pdf)"),
            ExportFormat::Svg => write!(f, "Gráfico Vetorial (.svg)"),
            ExportFormat::Png => write!(f, "Imagem PNG (.png)"),
        }
    }
}

/// Escopo de abrangência da exportação.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportScope {
    /// Projeto completo (todas as páginas do projeto em um único arquivo PDF).
    Project,
    /// Página/folha única selecionada.
    Page,
    /// Apenas os elementos atualmente selecionados pelo usuário.
    Selection,
}

impl ExportScope {
    pub fn label(&self) -> &'static str {
        match self {
            ExportScope::Project => "Projeto Completo (todas as folhas)",
            ExportScope::Page => "Folha Atual / Selecionada",
            ExportScope::Selection => "Seleção de Elementos",
        }
    }

    /// Retorna os escopos válidos para um determinado formato de exportação.
    pub fn valid_scopes(format: ExportFormat, is_multipage: bool) -> Vec<ExportScope> {
        match format {
            ExportFormat::Cad => vec![ExportScope::Page],
            ExportFormat::Svg | ExportFormat::Png => {
                vec![ExportScope::Page, ExportScope::Selection]
            }
            ExportFormat::Pdf => {
                if is_multipage {
                    vec![
                        ExportScope::Project,
                        ExportScope::Page,
                        ExportScope::Selection,
                    ]
                } else {
                    vec![ExportScope::Page, ExportScope::Selection]
                }
            }
        }
    }
}

impl fmt::Display for ExportScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Escapa caracteres especiais XML/SVG.
fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Renderiza primitivas vetoriais de um símbolo SVG interno (QET ou gerado pelo cade2svg)
/// aplicando rigorosamente as transformações de `canvas_view.rs`.
#[allow(clippy::too_many_arguments)]
fn render_svg_symbol_to_svg(
    out: &mut String,
    origin_x: i32,
    origin_y: i32,
    orientation: i32,
    svg_path: &str,
    stroke_color: &str,
    stroke_width: f32,
    fill_active: Option<&str>,
    target_span: f32,
    scale: f32,
) -> bool {
    let Some(parsed) = crate::svg_symbol::get_parsed_svg(svg_path) else {
        return false;
    };
    if parsed.shapes.is_empty() {
        return false;
    }

    let ox = origin_x as f32;
    let oy = origin_y as f32;

    let resolve_fill = |fill_mode: crate::svg_symbol::SvgFill| -> Option<String> {
        match fill_mode {
            crate::svg_symbol::SvgFill::Solid => Some(stroke_color.to_string()),
            crate::svg_symbol::SvgFill::White => Some(fill_active.unwrap_or("#ffffff").to_string()),
            crate::svg_symbol::SvgFill::Tint => Some(fill_active.unwrap_or("#f1f5f9").to_string()),
            crate::svg_symbol::SvgFill::None => {
                if parsed.is_qet_native {
                    None
                } else {
                    fill_active.map(|s| s.to_string())
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
                let ax = (ox + rx1) * scale;
                let ay = (oy + ry1) * scale;
                let bx = (ox + rx2) * scale;
                let by = (oy + ry2) * scale;
                let dash_attr = if *dashed {
                    " stroke-dasharray=\"4,3\""
                } else {
                    ""
                };
                out.push_str(&format!(
                    r###"  <line x1="{ax:.2}" y1="{ay:.2}" x2="{bx:.2}" y2="{by:.2}" stroke="{stroke_color}" stroke-width="{stroke_width:.1}"{dash_attr}/>"###
                ));
                out.push('\n');
            }
            crate::svg_symbol::SvgShape::Polyline { points } => {
                if points.len() < 2 {
                    continue;
                }
                let pts: Vec<String> = points
                    .iter()
                    .map(|&(px, py)| {
                        let (lx, ly) = parsed.map_point(px, py, target_span);
                        let (rx, ry) = transform_offset_f32(lx, ly, orientation);
                        format!("{:.2},{:.2}", (ox + rx) * scale, (oy + ry) * scale)
                    })
                    .collect();
                out.push_str(&format!(
                    r###"  <polyline points="{}" stroke="{stroke_color}" stroke-width="{stroke_width:.1}" fill="none"/>"###,
                    pts.join(" ")
                ));
                out.push('\n');
            }
            crate::svg_symbol::SvgShape::Polygon { points, fill } => {
                if points.is_empty() {
                    continue;
                }
                let pts: Vec<String> = points
                    .iter()
                    .map(|&(px, py)| {
                        let (lx, ly) = parsed.map_point(px, py, target_span);
                        let (rx, ry) = transform_offset_f32(lx, ly, orientation);
                        format!("{:.2},{:.2}", (ox + rx) * scale, (oy + ry) * scale)
                    })
                    .collect();
                let fill_attr = match resolve_fill(*fill) {
                    Some(col) => format!(" fill=\"{col}\""),
                    None => " fill=\"none\"".to_string(),
                };
                out.push_str(&format!(
                    r###"  <polygon points="{}" stroke="{stroke_color}" stroke-width="{stroke_width:.1}"{fill_attr}/>"###,
                    pts.join(" ")
                ));
                out.push('\n');
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
                let center_x = (ox + rx) * scale;
                let center_y = (oy + ry) * scale;
                let rad = if parsed.is_cade_svg {
                    (*r / 5.0) * scale
                } else if target_span > 1.0
                    && (parsed.term_bottom_y - parsed.term_top_y).abs() > 0.1
                {
                    (*r * (target_span / (parsed.term_bottom_y - parsed.term_top_y).abs())) * scale
                } else if parsed.is_qet_native {
                    *r * scale
                } else {
                    (*r * 0.30) * scale
                };
                let rad = rad.max(1.2);
                let fill_attr = match resolve_fill(*fill) {
                    Some(col) => format!(" fill=\"{col}\""),
                    None => " fill=\"none\"".to_string(),
                };
                out.push_str(&format!(
                    r###"  <circle cx="{center_x:.2}" cy="{center_y:.2}" r="{rad:.2}" stroke="{stroke_color}" stroke-width="{stroke_width:.1}"{fill_attr}/>"###
                ));
                out.push('\n');
            }
            crate::svg_symbol::SvgShape::Ellipse {
                cx,
                cy,
                rx,
                ry,
                fill,
            } => {
                // Converte a elipse em polígono suave de 28 vértices mapeados pontualmente,
                // exatamente como faz o CanvasView em canvas_view.rs (linhas 615..635).
                // Isso elimina qualquer distorção geométrica entre eixos e orientações rotacionadas.
                let steps = 28;
                let mut pts = Vec::with_capacity(steps);
                for i in 0..steps {
                    let ang = (i as f32 / steps as f32) * 2.0 * std::f32::consts::PI;
                    let px = cx + rx * ang.cos();
                    let py = cy + ry * ang.sin();
                    let (lx, ly) = parsed.map_point(px, py, target_span);
                    let (rot_x, rot_y) = transform_offset_f32(lx, ly, orientation);
                    let sc_x = (ox + rot_x) * scale;
                    let sc_y = (oy + rot_y) * scale;
                    pts.push(format!("{sc_x:.2},{sc_y:.2}"));
                }
                let fill_attr = match resolve_fill(*fill) {
                    Some(col) => format!(" fill=\"{col}\""),
                    None => " fill=\"none\"".to_string(),
                };
                out.push_str(&format!(
                    r###"  <polygon points="{}" stroke="{stroke_color}" stroke-width="{stroke_width:.1}"{fill_attr}/>"###,
                    pts.join(" ")
                ));
                out.push('\n');
            }
            crate::svg_symbol::SvgShape::Rect {
                x,
                y,
                w,
                h,
                dashed,
                fill,
            } => {
                let corners = [
                    parsed.map_point(*x, *y, target_span),
                    parsed.map_point(*x + *w, *y, target_span),
                    parsed.map_point(*x + *w, *y + *h, target_span),
                    parsed.map_point(*x, *y + *h, target_span),
                ];
                let pts: Vec<String> = corners
                    .iter()
                    .map(|&(lx, ly)| {
                        let (rx, ry) = transform_offset_f32(lx, ly, orientation);
                        format!("{:.2},{:.2}", (ox + rx) * scale, (oy + ry) * scale)
                    })
                    .collect();
                let fill_attr = match resolve_fill(*fill) {
                    Some(col) => format!(" fill=\"{col}\""),
                    None => " fill=\"none\"".to_string(),
                };
                let dash_attr = if *dashed {
                    " stroke-dasharray=\"3,2\""
                } else {
                    ""
                };
                out.push_str(&format!(
                    r###"  <polygon points="{}" stroke="{stroke_color}" stroke-width="{stroke_width:.1}"{fill_attr}{dash_attr}/>"###,
                    pts.join(" ")
                ));
                out.push('\n');
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
                let raw_pos_x = (ox + rx) * scale;
                let raw_pos_y = (oy + ry) * scale;
                let (font_size, offset_y) = if parsed.is_qet_native {
                    let fs = (size * 1.15 * scale).clamp(6.0, 22.0);
                    (fs, -fs * 0.82)
                } else if parsed.is_cade_svg {
                    let fs = (size * 0.25 * scale).clamp(8.0, 16.0);
                    (fs, 0.0)
                } else {
                    let fs = (size * 0.30 * scale).clamp(5.0, 16.0);
                    (fs, -fs * 0.82)
                };

                let total_rot_deg = match orientation.rem_euclid(4) {
                    1 => *rotation + 90.0,
                    2 => *rotation + 180.0,
                    3 => *rotation + 270.0,
                    _ => *rotation,
                };

                if total_rot_deg.abs() > 0.1 {
                    out.push_str(&format!(
                        r###"  <text transform="translate({raw_pos_x:.2}, {raw_pos_y:.2}) rotate({total_rot_deg:.1})" y="{offset_y:.2}" font-size="{font_size:.1}" font-family="sans-serif" dominant-baseline="hanging" fill="{stroke_color}">{}</text>"###,
                        xml_escape(text)
                    ));
                } else {
                    out.push_str(&format!(
                        r###"  <text x="{raw_pos_x:.2}" y="{:.2}" font-size="{font_size:.1}" font-family="sans-serif" dominant-baseline="hanging" fill="{stroke_color}">{}</text>"###,
                        raw_pos_y + offset_y,
                        xml_escape(text)
                    ));
                }
                out.push('\n');
            }
        }
    }

    true
}

/// Renderiza primitivas vetoriais nativas do CADe_SIMU C++ (`idle_cmds`)
/// aplicando rigorosamente as transformações de `canvas_view.rs`.
#[allow(clippy::too_many_arguments)]
fn render_symbol_primitives_to_svg(
    out: &mut String,
    origin_x: i32,
    origin_y: i32,
    orientation: i32,
    cmds: &[&str],
    stroke_color: &str,
    fill_active: Option<&str>,
    stroke_width: f32,
    scale: f32,
) {
    let ox = origin_x as f32;
    let oy = origin_y as f32;

    for &cmd in cmds {
        let Some(prim) = crate::symbols::parse_vector_cmd(cmd) else {
            continue;
        };
        match prim {
            crate::symbols::VectorPrimitive::Line { x1, y1, x2, y2 } => {
                let (rx1, ry1) =
                    transform_offset_f32(x1 as f32 / 5.0, y1 as f32 / 5.0, orientation);
                let (rx2, ry2) =
                    transform_offset_f32(x2 as f32 / 5.0, y2 as f32 / 5.0, orientation);
                let ax = (ox + rx1) * scale;
                let ay = (oy + ry1) * scale;
                let bx = (ox + rx2) * scale;
                let by = (oy + ry2) * scale;
                out.push_str(&format!(
                    r###"  <line x1="{ax:.2}" y1="{ay:.2}" x2="{bx:.2}" y2="{by:.2}" stroke="{stroke_color}" stroke-width="{stroke_width:.1}"/>"###
                ));
                out.push('\n');
            }
            crate::symbols::VectorPrimitive::Ellipse { x1, y1, x2, y2 } => {
                let (rx1, ry1) =
                    transform_offset_f32(x1 as f32 / 5.0, y1 as f32 / 5.0, orientation);
                let (rx2, ry2) =
                    transform_offset_f32(x2 as f32 / 5.0, y2 as f32 / 5.0, orientation);
                let cx = ox + (rx1 + rx2) * 0.5;
                let cy = oy + (ry1 + ry2) * 0.5;
                let rad = (((rx2 - rx1).abs().max((ry2 - ry1).abs())) * 0.5) * scale;
                let fill_attr = match fill_active {
                    Some(col) => format!(" fill=\"{col}\""),
                    None => " fill=\"none\"".to_string(),
                };
                out.push_str(&format!(
                    r###"  <circle cx="{:.2}" cy="{:.2}" r="{rad:.2}" stroke="{stroke_color}" stroke-width="{stroke_width:.1}"{fill_attr}/>"###,
                    cx * scale,
                    cy * scale
                ));
                out.push('\n');
            }
            crate::symbols::VectorPrimitive::Arc { x1, y1, x2, y2, .. } => {
                let (rx1, ry1) =
                    transform_offset_f32(x1 as f32 / 5.0, y1 as f32 / 5.0, orientation);
                let (rx2, ry2) =
                    transform_offset_f32(x2 as f32 / 5.0, y2 as f32 / 5.0, orientation);
                let cx = ox + (rx1 + rx2) * 0.5;
                let cy = oy + (ry1 + ry2) * 0.5;
                let rad = (((rx2 - rx1).abs().min((ry2 - ry1).abs())) * 0.45) * scale;
                out.push_str(&format!(
                    r###"  <circle cx="{:.2}" cy="{:.2}" r="{rad:.2}" stroke="{stroke_color}" stroke-width="{stroke_width:.1}" fill="none"/>"###,
                    cx * scale,
                    cy * scale
                ));
                out.push('\n');
            }
            crate::symbols::VectorPrimitive::Polygon { pts } => {
                let pts_str: Vec<String> = pts
                    .iter()
                    .map(|&(px, py)| {
                        let (rx, ry) =
                            transform_offset_f32(px as f32 / 5.0, py as f32 / 5.0, orientation);
                        format!("{:.2},{:.2}", (ox + rx) * scale, (oy + ry) * scale)
                    })
                    .collect();
                let fill_attr = match fill_active {
                    Some(col) => format!(" fill=\"{col}\""),
                    None => " fill=\"none\"".to_string(),
                };
                out.push_str(&format!(
                    r###"  <polygon points="{}" stroke="{stroke_color}" stroke-width="{stroke_width:.1}"{fill_attr}/>"###,
                    pts_str.join(" ")
                ));
                out.push('\n');
            }
            crate::symbols::VectorPrimitive::Text { x, y, text } => {
                let (rx, ry) = transform_offset_f32(x as f32 / 5.0, y as f32 / 5.0, orientation);
                let pos_x = (ox + rx - 1.2) * scale;
                let pos_y = (oy + ry - 1.0) * scale;
                let font_size = (3.2 * scale).clamp(8.0, 16.0);
                out.push_str(&format!(
                    r###"  <text x="{pos_x:.2}" y="{pos_y:.2}" font-size="{font_size:.1}" font-family="sans-serif" dominant-baseline="hanging" fill="{stroke_color}">{}</text>"###,
                    xml_escape(&text)
                ));
                out.push('\n');
            }
        }
    }
}

/// Gera uma representação SVG pura a partir de um `SchematicDocument`.
///
/// - `items_filter`: Se `Some(ids)`, exporta apenas os itens com esses IDs (modo Seleção)
///   com bounding box ajustado. Se `None`, exporta a folha inteira.
/// - `include_cajetin`: Se `true` e não for modo seleção, inclui a moldura e a tabela do cajetín.
pub fn export_to_svg(
    doc: &SchematicDocument,
    items_filter: Option<&[i32]>,
    include_cajetin: bool,
) -> String {
    let scale = SVG_SCALE;

    let sheet_w = if doc.settings.custom_height_mm > 420 {
        doc.settings.custom_height_mm as f32 + 30.0
    } else {
        (doc.settings.custom_height_mm.clamp(279, 420) as f32) + 45.0
    };
    let sheet_h = if doc.settings.custom_width_mm > 320 {
        doc.settings.custom_width_mm as f32 + 30.0
    } else {
        (doc.settings.custom_width_mm.clamp(210, 320) as f32) + 30.0
    };

    let is_selection = items_filter.is_some();
    let (view_x, view_y, view_w, view_h) = if let Some(filter) = items_filter {
        let sel_items: Vec<&SchematicItem> = doc
            .items
            .iter()
            .filter(|i| filter.contains(&i.id))
            .collect();

        if sel_items.is_empty() {
            (0.0, 0.0, sheet_w * scale, sheet_h * scale)
        } else {
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;

            for item in &sel_items {
                if item.is_wire() {
                    min_x = min_x.min(item.x1 as f32).min(item.x2 as f32);
                    max_x = max_x.max(item.x1 as f32).max(item.x2 as f32);
                    min_y = min_y.min(item.y1 as f32).min(item.y2 as f32);
                    max_y = max_y.max(item.y1 as f32).max(item.y2 as f32);
                } else if let Some(qet_type) = item.symbol_id_or_slug() {
                    if let Some(parsed) = crate::svg_symbol::get_parsed_svg(qet_type) {
                        let (vx, vy, vw, vh) = parsed.view_box;
                        let (p1x, p1y) = transform_offset_f32(vx, vy, item.orientation);
                        let (p2x, p2y) = transform_offset_f32(vx + vw, vy + vh, item.orientation);
                        min_x = min_x.min((item.x1 as f32 + p1x.min(p2x)) - 2.0);
                        max_x = max_x.max((item.x1 as f32 + p1x.max(p2x)) + 2.0);
                        min_y = min_y.min((item.y1 as f32 + p1y.min(p2y)) - 2.0);
                        max_y = max_y.max((item.y1 as f32 + p1y.max(p2y)) + 2.0);
                    } else {
                        min_x = min_x.min(item.x1 as f32 - 4.0);
                        max_x = max_x.max(item.x1 as f32 + 10.0);
                        min_y = min_y.min(item.y1 as f32 - 4.0);
                        max_y = max_y.max(item.y1 as f32 + 10.0);
                    }
                } else if let Some(sym) = find_symbol_ref(item.tipo) {
                    let (bx1, by1, bx2, by2) = (
                        sym.bounds.0 / 5,
                        sym.bounds.1 / 5,
                        sym.bounds.2 / 5,
                        sym.bounds.3 / 5,
                    );
                    let (p1x, p1y) = transform_offset(bx1, by1, item.orientation);
                    let (p2x, p2y) = transform_offset(bx2, by2, item.orientation);
                    min_x = min_x.min((item.x1 + p1x.min(p2x) - 1) as f32);
                    max_x = max_x.max((item.x1 + p1x.max(p2x) + 1) as f32);
                    min_y = min_y.min((item.y1 + p1y.min(p2y) - 1) as f32);
                    max_y = max_y.max((item.y1 + p1y.max(p2y) + 1) as f32);
                } else {
                    min_x = min_x.min(item.x1 as f32 - 2.0);
                    max_x = max_x.max(item.x1 as f32 + 10.0);
                    min_y = min_y.min(item.y1 as f32 - 2.0);
                    max_y = max_y.max(item.y1 as f32 + 10.0);
                }
            }

            let pad = 8.0;
            let vx = (min_x - pad).max(0.0) * scale;
            let vy = (min_y - pad).max(0.0) * scale;
            let vw = (max_x - min_x + pad * 2.0).max(30.0) * scale;
            let vh = (max_y - min_y + pad * 2.0).max(25.0) * scale;
            (vx, vy, vw, vh)
        }
    } else {
        (0.0, 0.0, sheet_w * scale, sheet_h * scale)
    };

    let px_w = view_w.round();
    let px_h = view_h.round();

    let mut out = String::with_capacity(32_768);
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        r###"<svg xmlns="http://www.w3.org/2000/svg" width="{px_w:.0}" height="{px_h:.0}" viewBox="{view_x:.1} {view_y:.1} {view_w:.1} {view_h:.1}">"###
    ));
    out.push('\n');

    // Fundo branco do papel
    out.push_str(&format!(
        r###"  <rect x="{view_x:.1}" y="{view_y:.1}" width="{view_w:.1}" height="{view_h:.1}" fill="#ffffff"/>"###
    ));
    out.push('\n');

    // Moldura e Cajetín (apenas se página completa e cajetin ativado)
    if !is_selection && include_cajetin {
        let border_stroke_w = 1.5;

        // Moldura externa da folha
        let outer_w = sheet_w * scale;
        let outer_h = sheet_h * scale;
        out.push_str(&format!(
            r###"  <rect x="0" y="0" width="{outer_w:.1}" height="{outer_h:.1}" fill="none" stroke="#1b2230" stroke-width="{border_stroke_w}"/>"###
        ));
        out.push('\n');

        // Moldura interna da folha
        let inner_x = 6.0 * scale;
        let inner_y = 6.0 * scale;
        let inner_w = (sheet_w - 12.0) * scale;
        let inner_h = (sheet_h - 12.0) * scale;
        out.push_str(&format!(
            r###"  <rect x="{inner_x:.1}" y="{inner_y:.1}" width="{inner_w:.1}" height="{inner_h:.1}" fill="none" stroke="#1b2230" stroke-width="{border_stroke_w}"/>"###
        ));
        out.push('\n');

        // Referências numéricas de colunas (1..8)
        let font_label_sz = (3.2 * scale).clamp(8.0, 14.0);
        for col in 0..8 {
            let cx = (6.0 + (col as f32 + 0.5) * ((sheet_w - 12.0) / 8.0)) * scale;
            out.push_str(&format!(
                r###"  <text x="{cx:.1}" y="{:.1}" text-anchor="middle" font-size="{font_label_sz:.1}" font-family="sans-serif" dominant-baseline="hanging" fill="#5c6675">{}</text>"###,
                1.2 * scale,
                col + 1
            ));
            out.push('\n');
            out.push_str(&format!(
                r###"  <text x="{cx:.1}" y="{:.1}" text-anchor="middle" font-size="{font_label_sz:.1}" font-family="sans-serif" dominant-baseline="hanging" fill="#5c6675">{}</text>"###,
                (sheet_h - 4.5) * scale,
                col + 1
            ));
            out.push('\n');
        }

        // Referências alfabéticas de linhas (A..F)
        for (row, ch) in ['A', 'B', 'C', 'D', 'E', 'F'].iter().enumerate() {
            let ry = (6.0 + (row as f32 + 0.5) * ((sheet_h - 12.0) / 6.0)) * scale;
            out.push_str(&format!(
                r###"  <text x="{:.1}" y="{ry:.1}" text-anchor="middle" font-size="{font_label_sz:.1}" font-family="sans-serif" dominant-baseline="hanging" fill="#5c6675">{}</text>"###,
                1.8 * scale,
                ch
            ));
            out.push('\n');
            out.push_str(&format!(
                r###"  <text x="{:.1}" y="{ry:.1}" text-anchor="middle" font-size="{font_label_sz:.1}" font-family="sans-serif" dominant-baseline="hanging" fill="#5c6675">{}</text>"###,
                (sheet_w - 4.5) * scale,
                ch
            ));
            out.push('\n');
        }

        // Bloco de legenda (Cajetín) no canto inferior direito
        let caj_w = 115.0;
        let caj_h = 22.0;
        let caj_x = sheet_w - caj_w - 6.0;
        let caj_y = sheet_h - caj_h - 6.0;
        let cp_x = caj_x * scale;
        let cp_y = caj_y * scale;
        let cs_w = caj_w * scale;
        let cs_h = caj_h * scale;

        out.push_str(&format!(
            r###"  <rect x="{cp_x:.1}" y="{cp_y:.1}" width="{cs_w:.1}" height="{cs_h:.1}" fill="#f4f6f9" stroke="#1b2230" stroke-width="{border_stroke_w}"/>"###
        ));
        out.push('\n');
        out.push_str(&format!(
            r###"  <line x1="{cp_x:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#d3dae4" stroke-width="1.0"/>"###,
            (caj_y + 7.0) * scale,
            cp_x + cs_w,
            (caj_y + 7.0) * scale
        ));
        out.push('\n');
        out.push_str(&format!(
            r###"  <line x1="{cp_x:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#d3dae4" stroke-width="1.0"/>"###,
            (caj_y + 14.0) * scale,
            cp_x + cs_w,
            (caj_y + 14.0) * scale
        ));
        out.push('\n');

        let tb = &doc.title_block;
        let font_sz = (3.0 * scale).clamp(7.0, 13.0);
        out.push_str(&format!(
            r###"  <text x="{:.1}" y="{:.1}" font-size="{font_sz:.1}" font-family="sans-serif" dominant-baseline="hanging" fill="#5c6675">Entidade: {}  |  Data: {}</text>"###,
            (caj_x + 3.0) * scale,
            (caj_y + 2.5) * scale,
            xml_escape(&tb.entity),
            xml_escape(&tb.drawn_date)
        ));
        out.push('\n');
        out.push_str(&format!(
            r###"  <text x="{:.1}" y="{:.1}" font-size="{:.1}" font-weight="bold" font-family="sans-serif" dominant-baseline="hanging" fill="#0f141c">Título: {} - {}</text>"###,
            (caj_x + 3.0) * scale,
            (caj_y + 8.5) * scale,
            font_sz * 1.08,
            xml_escape(&tb.title),
            xml_escape(&tb.subtitle)
        ));
        out.push('\n');
        out.push_str(&format!(
            r###"  <text x="{:.1}" y="{:.1}" font-size="{font_sz:.1}" font-family="sans-serif" dominant-baseline="hanging" fill="#5c6675">Autor: {}  |  Arquivo: {}  |  Folha: {}</text>"###,
            (caj_x + 3.0) * scale,
            (caj_y + 15.0) * scale,
            xml_escape(&tb.drawn_by),
            xml_escape(&tb.file_ref),
            xml_escape(&tb.sheet)
        ));
        out.push('\n');
    }

    // 1. Desenha cabos primeiro (como no canvas_view)
    for item in &doc.items {
        if let Some(filter) = items_filter {
            if !filter.contains(&item.id) {
                continue;
            }
        }
        if item.is_wire() {
            let (stroke_color, stroke_width) = if item.color != 0 {
                let r = ((item.color >> 16) & 0xFF) as u8;
                let g = ((item.color >> 8) & 0xFF) as u8;
                let b = (item.color & 0xFF) as u8;
                (format!("#{r:02x}{g:02x}{b:02x}"), 1.8)
            } else {
                match item.tipo {
                    4000 | 4018 => ("#8c2e1f".to_string(), 1.8), // L1 / Potência (Castanho avermelhado)
                    4009 | 4019 => ("#1461d1".to_string(), 1.8), // Neutro N / 0V (Azul)
                    4002 | 4010 => ("#1f9e33".to_string(), 1.6), // Terra PE (Verde)
                    4003 => ("#d11f26".to_string(), 1.8),        // DC+ (Vermelho)
                    4004 => ("#262633".to_string(), 1.8),        // DC- (Preto/Azulado)
                    _ => ("#262e3d".to_string(), 1.6),           // Genérico
                }
            };
            let x1 = item.x1 as f32 * scale;
            let y1 = item.y1 as f32 * scale;
            let x2 = item.x2 as f32 * scale;
            let y2 = item.y2 as f32 * scale;
            out.push_str(&format!(
                r###"  <line x1="{x1:.1}" y1="{y1:.1}" x2="{x2:.1}" y2="{y2:.1}" stroke="{stroke_color}" stroke-width="{stroke_width}" stroke-linecap="round"/>"###
            ));
            out.push('\n');
        }
    }

    // 2. Desenha junções, textos e símbolos
    for item in &doc.items {
        if let Some(filter) = items_filter {
            if !filter.contains(&item.id) {
                continue;
            }
        }
        if item.is_wire() {
            continue;
        }

        // Junções (nós de conexão)
        if item.is_junction() {
            let color = if item.color != 0 {
                let r = ((item.color >> 16) & 0xFF) as u8;
                let g = ((item.color >> 8) & 0xFF) as u8;
                let b = (item.color & 0xFF) as u8;
                format!("#{r:02x}{g:02x}{b:02x}")
            } else {
                match item.tipo {
                    4000 | 4018 => "#8c2e1f".to_string(),
                    4009 | 4019 => "#1461d1".to_string(),
                    4002 | 4010 => "#1f9e33".to_string(),
                    4003 => "#d11f26".to_string(),
                    4004 => "#262633".to_string(),
                    _ => "#262e3d".to_string(),
                }
            };
            let cx = item.x1 as f32 * scale;
            let cy = item.y1 as f32 * scale;
            let rad = (0.9 * scale).clamp(2.2, 5.0);
            out.push_str(&format!(
                r###"  <circle cx="{cx:.1}" cy="{cy:.1}" r="{rad:.1}" fill="{color}"/>"###
            ));
            out.push('\n');
            continue;
        }

        // Texto livre (tipo == 8)
        if item.tipo == 8 {
            if !item.trailing_str.is_empty() {
                let px = item.x1 as f32 * scale;
                let py = item.y1 as f32 * scale;
                let font_sz = (3.2 * scale).clamp(8.5, 15.0);
                out.push_str(&format!(
                    r###"  <text x="{px:.1}" y="{py:.1}" font-size="{font_sz:.1}" font-family="sans-serif" dominant-baseline="hanging" fill="#0f141c">{}</text>"###,
                    xml_escape(&item.trailing_str)
                ));
                out.push('\n');
            }
            continue;
        }

        // Linha gráfica auxiliar (tipo == 1)
        if item.tipo == 1 {
            let ax = item.x1 as f32 * scale;
            let ay = item.y1 as f32 * scale;
            let bx = item.x2 as f32 * scale;
            let by = item.y2 as f32 * scale;
            out.push_str(&format!(
                r###"  <line x1="{ax:.1}" y1="{ay:.1}" x2="{bx:.1}" y2="{by:.1}" stroke="#1b2230" stroke-width="1.2"/>"###
            ));
            out.push('\n');
            continue;
        }

        // Retângulo gráfico (tipo == 2)
        if item.tipo == 2 {
            let min_x = item.x1.min(item.x2) as f32 * scale;
            let min_y = item.y1.min(item.y2) as f32 * scale;
            let w = (item.x2 - item.x1).abs() as f32 * scale;
            let h = (item.y2 - item.y1).abs() as f32 * scale;
            out.push_str(&format!(
                r###"  <rect x="{min_x:.1}" y="{min_y:.1}" width="{w:.1}" height="{h:.1}" stroke="#1b2230" stroke-width="1.3" fill="none"/>"###
            ));
            out.push('\n');
            continue;
        }

        let is_lamp = item.is_lamp_or_signaling();
        let fill_lamp = if is_lamp {
            match item.color {
                1 => Some("#fee2e2"), // Vermelho suave
                2 => Some("#fef3c7"), // Amarelo suave
                3 => Some("#e0f2fe"), // Azul suave
                4 => Some("#f1f5f9"), // Branco suave
                _ => Some("#dcfce7"), // Verde suave
            }
        } else {
            None
        };

        // Elemento nativo importado de projeto QElectroTech (.qet) ou símbolo canônico D2
        if let Some(qet_type) = item.symbol_id_or_slug() {
            render_svg_symbol_to_svg(
                &mut out,
                item.x1,
                item.y1,
                item.orientation,
                qet_type,
                "#262e3d",
                1.5,
                fill_lamp,
                1.0,
                scale,
            );
            if item.show_tag == 1 && !item.tag.is_empty() {
                let tpos_x = (item.x1 - 6) as f32 * scale;
                let tpos_y = (item.y1 - 6) as f32 * scale;
                let font_sz = (3.0 * scale).clamp(8.0, 15.0);
                out.push_str(&format!(
                    r###"  <text x="{tpos_x:.1}" y="{tpos_y:.1}" font-size="{font_sz:.1}" font-weight="bold" font-family="sans-serif" dominant-baseline="hanging" fill="#8c2e1f">{}</text>"###,
                    xml_escape(&item.tag)
                ));
                out.push('\n');
            }
            continue;
        }

        // Símbolos eletrotécnicos CADe_SIMU
        let Some(sym) = find_symbol_ref(item.tipo) else {
            // Fallback para símbolo desconhecido
            let tl_x = (item.x1 - 2) as f32 * scale;
            let tl_y = (item.y1 - 2) as f32 * scale;
            let sz_w = 12.0 * scale;
            let sz_h = 14.0 * scale;
            out.push_str(&format!(
                r###"  <rect x="{tl_x:.1}" y="{tl_y:.1}" width="{sz_w:.1}" height="{sz_h:.1}" fill="none" stroke="#b87500" stroke-width="1.3" stroke-dasharray="3,2"/>"###
            ));
            out.push('\n');
            let font_sz = (2.8 * scale).clamp(7.5, 13.0);
            let label = if !item.tag.is_empty() {
                format!("{}:{}", item.tag, item.tipo)
            } else {
                format!("?{}", item.tipo)
            };
            out.push_str(&format!(
                r###"  <text x="{:.1}" y="{:.1}" font-size="{font_sz:.1}" font-family="sans-serif" dominant-baseline="hanging" fill="#b87500">{}</text>"###,
                tl_x + 2.0 * scale,
                tl_y + 2.0 * scale,
                xml_escape(&label)
            ));
            out.push('\n');
            continue;
        };

        let stroke_col = "#262e3d";
        let mut drew_svg = false;
        if let Some((svg_idle, _)) = sym.svg_paths() {
            let max_pin_y = sym.pins.iter().map(|p| p.conn_pos.1).fold(0, i32::max) as f32;
            let target_span = if max_pin_y > 0.0 { max_pin_y } else { 12.0 };
            drew_svg = render_svg_symbol_to_svg(
                &mut out,
                item.x1,
                item.y1,
                item.orientation,
                svg_idle,
                stroke_col,
                1.8,
                fill_lamp,
                target_span,
                scale,
            );
        }

        if !drew_svg {
            render_symbol_primitives_to_svg(
                &mut out,
                item.x1,
                item.y1,
                item.orientation,
                sym.idle_cmds,
                stroke_col,
                fill_lamp,
                1.6,
                scale,
            );
        }

        // Desenha bornes (pinos) e suas referências numéricas (13/14, A1/A2, U1/V1/W1)
        for (idx, pin_label, (wx, wy)) in sym.world_pins(item) {
            let pt_x = wx as f32 * scale;
            let pt_y = wy as f32 * scale;
            let pin_rad = (0.55 * scale).clamp(1.5, 3.2);
            out.push_str(&format!(
                r###"  <circle cx="{pt_x:.1}" cy="{pt_y:.1}" r="{pin_rad:.1}" stroke="#d32f2f" stroke-width="1.0" fill="none"/>"###
            ));
            out.push('\n');
            if doc.settings.show_pin_refs >= 1 && item.pin_visibility[idx] == 1 {
                let p_def = &sym.pins[idx];
                let (lx, ly) =
                    transform_offset(p_def.label_pos.0, p_def.label_pos.1, item.orientation);
                let lpos_x = ((item.x1 + lx) as f32 + 0.4) * scale;
                let lpos_y = ((item.y1 + ly) as f32 - 0.5) * scale;
                let font_sz = (2.4 * scale).clamp(6.5, 11.5);
                out.push_str(&format!(
                    r###"  <text x="{lpos_x:.1}" y="{lpos_y:.1}" font-size="{font_sz:.1}" font-family="sans-serif" dominant-baseline="hanging" fill="#5c6675">{}</text>"###,
                    xml_escape(&pin_label)
                ));
                out.push('\n');
            }
        }

        // Tag do componente (-KM1, -Q1, -S1, -M1)
        if item.show_tag == 1 && !item.tag.is_empty() {
            let (tx, ty) = match item.tipo {
                3000..=3009 => (item.x1 - 5, item.y1 - 5),
                1000..=1012 => (item.x1 - 8, item.y1 + 9),
                _ => (item.x1 - 8, item.y1 + 3),
            };
            let tpos_x = tx as f32 * scale;
            let tpos_y = ty as f32 * scale;
            let font_sz = (3.0 * scale).clamp(7.5, 14.0);
            out.push_str(&format!(
                r###"  <text x="{tpos_x:.1}" y="{tpos_y:.1}" font-size="{font_sz:.1}" font-weight="bold" font-family="sans-serif" dominant-baseline="hanging" fill="#8c2e1f">{}</text>"###,
                xml_escape(&item.tag)
            ));
            out.push('\n');
        }

        // Valor do componente (tempo em segundos, ohms, etc.)
        if item.show_value == 1 && !item.value.is_empty() {
            let vpos_x = (item.x1 + 5) as f32 * scale;
            let vpos_y = (item.y1 + 5) as f32 * scale;
            let font_sz = (2.7 * scale).clamp(7.0, 12.5);
            out.push_str(&format!(
                r###"  <text x="{vpos_x:.1}" y="{vpos_y:.1}" font-size="{font_sz:.1}" font-family="sans-serif" dominant-baseline="hanging" fill="#5c6675">{}s</text>"###,
                xml_escape(&item.value)
            ));
            out.push('\n');
        }
    }

    out.push_str("</svg>\n");
    out
}

/// Renderiza um SVG para bytes de imagem PNG utilizando `resvg` e `tiny-skia`.
pub fn render_svg_to_png(svg_str: &str, scale: f32) -> Result<Vec<u8>, String> {
    let mut opt = resvg::usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();

    let tree = resvg::usvg::Tree::from_str(svg_str, &opt)
        .map_err(|e| format!("Falha ao analisar SVG para PNG: {e}"))?;

    let size = tree.size().to_int_size();
    let target_w = ((size.width() as f32) * scale).round() as u32;
    let target_h = ((size.height() as f32) * scale).round() as u32;

    let mut pixmap = tiny_skia::Pixmap::new(target_w, target_h)
        .ok_or_else(|| "Falha ao alocar buffer para renderizar PNG".to_string())?;

    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    pixmap
        .encode_png()
        .map_err(|e| format!("Erro ao codificar bytes PNG: {e}"))
}

/// Exporta uma folha ou seleção para PNG em alta resolução (escala padrão 1.5x sobre o SVG padrão).
pub fn export_to_png(
    doc: &SchematicDocument,
    items_filter: Option<&[i32]>,
    include_cajetin: bool,
    scale: f32,
) -> Result<Vec<u8>, String> {
    let svg = export_to_svg(doc, items_filter, include_cajetin);
    render_svg_to_png(&svg, scale)
}

/// Estrutura de dados interna para montagem de páginas PDF.
struct RawPdfPage {
    width_pt: u32,
    height_pt: u32,
    width_px: u32,
    height_px: u32,
    compressed_rgb: Vec<u8>,
}

/// Gera um documento PDF-1.4 compatível a partir de uma lista de páginas brutas.
fn build_pdf_document(pages: &[RawPdfPage]) -> Vec<u8> {
    let mut out = Vec::with_capacity(65_536);
    out.extend_from_slice(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n");

    let mut offsets = vec![0usize];

    // Objeto 1: Catalog
    offsets.push(out.len());
    out.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");

    // Objeto 2: Pages
    offsets.push(out.len());
    let count = pages.len();
    let mut kids_str = String::new();
    for i in 0..count {
        kids_str.push_str(&format!("{} 0 R ", 3 + i * 3));
    }
    out.extend_from_slice(
        format!("2 0 obj\n<< /Type /Pages /Kids [{kids_str}] /Count {count} >>\nendobj\n")
            .as_bytes(),
    );

    // Objetos de cada página: (base = 3 + i * 3)
    // base + 0: Page
    // base + 1: Contents stream
    // base + 2: Image XObject
    for (i, p) in pages.iter().enumerate() {
        let page_id = 3 + i * 3;
        let cont_id = 4 + i * 3;
        let img_id = 5 + i * 3;

        // 1. Page
        offsets.push(out.len());
        let page_dict = format!(
            "{page_id} 0 obj\n<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {} {}] /Resources << /XObject << /Im1 {img_id} 0 R >> >> /Contents {cont_id} 0 R >>\nendobj\n",
            p.width_pt, p.height_pt
        );
        out.extend_from_slice(page_dict.as_bytes());

        // 2. Contents
        offsets.push(out.len());
        let content_stream = format!("q {} 0 0 {} 0 0 cm /Im1 Do Q\n", p.width_pt, p.height_pt);
        let cont_dict = format!(
            "{cont_id} 0 obj\n<< /Length {} >>\nstream\n{}endstream\nendobj\n",
            content_stream.len(),
            content_stream
        );
        out.extend_from_slice(cont_dict.as_bytes());

        // 3. Image XObject
        offsets.push(out.len());
        let img_header = format!(
            "{img_id} 0 obj\n<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode /Length {} >>\nstream\n",
            p.width_px,
            p.height_px,
            p.compressed_rgb.len()
        );
        out.extend_from_slice(img_header.as_bytes());
        out.extend_from_slice(&p.compressed_rgb);
        out.extend_from_slice(b"\nendstream\nendobj\n");
    }

    // Tabela de Referências Cruzadas (xref)
    let startxref = out.len();
    let total_objs = offsets.len();
    out.extend_from_slice(format!("xref\n0 {total_objs}\n0000000000 65535 f \r\n").as_bytes());

    for &off in &offsets[1..] {
        out.extend_from_slice(format!("{off:010} 00000 n \r\n").as_bytes());
    }

    // Trailer
    out.extend_from_slice(
        format!("trailer\n<< /Size {total_objs} /Root 1 0 R >>\nstartxref\n{startxref}\n%%EOF\n")
            .as_bytes(),
    );

    out
}

/// Renderiza uma folha/seleção como imagem RGB compactada com FlateDecode para PDF.
fn render_doc_for_pdf(
    doc: &SchematicDocument,
    items_filter: Option<&[i32]>,
    include_cajetin: bool,
) -> Result<RawPdfPage, String> {
    let svg = export_to_svg(doc, items_filter, include_cajetin);

    let mut opt = resvg::usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();

    let tree = resvg::usvg::Tree::from_str(&svg, &opt)
        .map_err(|e| format!("Falha ao analisar SVG para PDF: {e}"))?;

    let size = tree.size().to_int_size();
    let scale = 1.5; // Resolução otimizada para PDF nítido e leve
    let target_w = ((size.width() as f32) * scale).round() as u32;
    let target_h = ((size.height() as f32) * scale).round() as u32;

    let mut pixmap = tiny_skia::Pixmap::new(target_w, target_h)
        .ok_or_else(|| "Falha ao alocar buffer para renderizar página PDF".to_string())?;

    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    // Extrai scanlines RGB (3 bytes por pixel) a partir de RGBA
    let rgba = pixmap.data();
    let mut rgb = Vec::with_capacity(rgba.len() / 4 * 3);
    for chunk in rgba.as_chunks::<4>().0 {
        rgb.push(chunk[0]);
        rgb.push(chunk[1]);
        rgb.push(chunk[2]);
    }

    // Compacta com zlib/deflate
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder
        .write_all(&rgb)
        .map_err(|e| format!("Falha ao comprimir imagem para PDF: {e}"))?;
    let compressed_rgb = encoder
        .finish()
        .map_err(|e| format!("Falha ao finalizar compressão para PDF: {e}"))?;

    // Dimensões da página em pontos PDF proporcionais (A4 landscape: base 842 pt)
    let width_pt = 842;
    let height_pt = ((842.0 * size.height() as f32 / size.width() as f32).round() as u32).max(100);

    Ok(RawPdfPage {
        width_pt,
        height_pt,
        width_px: target_w,
        height_px: target_h,
        compressed_rgb,
    })
}

/// Exporta uma folha ou seleção de elementos para documento PDF.
pub fn export_doc_to_pdf(
    doc: &SchematicDocument,
    items_filter: Option<&[i32]>,
    include_cajetin: bool,
) -> Result<Vec<u8>, String> {
    let page = render_doc_for_pdf(doc, items_filter, include_cajetin)?;
    Ok(build_pdf_document(&[page]))
}

/// Exporta um projeto completo multipáginas para um único documento PDF (uma página por folha).
pub fn export_project_to_pdf(
    pages: &[ProjectPage],
    include_cajetin: bool,
) -> Result<Vec<u8>, String> {
    if pages.is_empty() {
        return Err("Nenhuma página disponível no projeto para exportar.".to_string());
    }

    let mut pdf_pages = Vec::with_capacity(pages.len());
    for page in pages {
        let pdf_page = render_doc_for_pdf(&page.doc, None, include_cajetin)?;
        pdf_pages.push(pdf_page);
    }

    Ok(build_pdf_document(&pdf_pages))
}

/// Exporta uma folha em formato CADe_SIMU original (.cad) em bytes CP-1252.
/// Se `include_cajetin` for falso, os campos do cajetín são limpos antes da gravação.
pub fn export_to_cad(doc: &SchematicDocument, include_cajetin: bool) -> Vec<u8> {
    if include_cajetin {
        crate::io::serialize_cad_bytes(doc)
    } else {
        let mut clean_doc = doc.clone();
        clean_doc.title_block = crate::model::TitleBlock {
            drawn_date: String::new(),
            drawn_by: String::new(),
            checked_date: String::new(),
            checked_by: String::new(),
            entity: String::new(),
            title: String::new(),
            subtitle: String::new(),
            sheet: String::new(),
            file_ref: String::new(),
            signature: String::new(),
        };
        crate::io::serialize_cad_bytes(&clean_doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::io::build_dol_starter_preset;

    #[test]
    fn test_export_to_svg_contains_valid_xml_and_elements() {
        let doc = build_dol_starter_preset();
        let svg = export_to_svg(&doc, None, true);

        assert!(svg.starts_with("<?xml version=\"1.0\""));
        assert!(svg.contains("<svg"));
        assert!(svg.ends_with("</svg>\n"));
        // Cajetín ativado
        assert!(svg.contains("Entidade:"));
        assert!(svg.contains("Partida Direta Trifasica"));
        // Wires e componentes
        assert!(svg.contains("<line"));
        assert!(svg.contains("-Q1"));
    }

    #[test]
    fn test_export_to_svg_without_cajetin() {
        let doc = build_dol_starter_preset();
        let svg = export_to_svg(&doc, None, false);

        assert!(svg.starts_with("<?xml version=\"1.0\""));
        assert!(!svg.contains("Entidade:"));
        assert!(!svg.contains("Partida Direta Trifasica"));
        // Mas contém os componentes do circuito
        assert!(svg.contains("-Q1"));
    }

    #[test]
    fn test_export_selection_to_svg() {
        let doc = build_dol_starter_preset();
        let item_q1 = doc
            .items
            .iter()
            .find(|i| i.tag == "-Q1")
            .expect("Item -Q1 deve existir no preset");
        let svg = export_to_svg(&doc, Some(&[item_q1.id]), false);

        assert!(svg.contains("-Q1"));
        assert!(!svg.contains("Entidade:"));
    }

    #[test]
    fn test_export_to_png_produces_valid_png_bytes() {
        let doc = build_dol_starter_preset();
        let png = export_to_png(&doc, None, true, 1.0).expect("Render PNG");

        assert!(png.len() > 100);
        // Assinatura mágica do PNG: 0x89 0x50 0x4E 0x47 0x0D 0x0A 0x1A 0x0A
        assert_eq!(&png[0..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn test_export_to_pdf_single_and_multipage() {
        let doc1 = build_dol_starter_preset();
        let pdf_single = export_doc_to_pdf(&doc1, None, true).expect("PDF single");

        assert!(pdf_single.starts_with(b"%PDF-1.4"));
        assert!(pdf_single.ends_with(b"%%EOF\n"));

        // Multipage
        let page1 = ProjectPage::new(1, "Potência".to_string(), doc1.clone());
        let page2 = ProjectPage::new(2, "Comando".to_string(), doc1);
        let pdf_multi = export_project_to_pdf(&[page1, page2], true).expect("PDF multi");

        assert!(pdf_multi.starts_with(b"%PDF-1.4"));
        assert!(pdf_multi
            .windows(b"/Count 2".len())
            .any(|w| w == b"/Count 2"));
    }

    #[test]
    fn test_export_to_cad_cajetin_toggle() {
        let doc = build_dol_starter_preset();
        let cad_with = export_to_cad(&doc, true);
        let cad_without = export_to_cad(&doc, false);

        let parsed_with = crate::io::parse_cad_bytes(&cad_with).expect("Parse com cajetin");
        let parsed_without = crate::io::parse_cad_bytes(&cad_without).expect("Parse sem cajetin");

        assert_eq!(parsed_with.title_block.title, "Partida Direta Trifasica");
        assert_eq!(parsed_without.title_block.title, "");
    }
}
