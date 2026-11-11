//! Módulo de Exportação Real (Folha → SVG / PNG / PDF / CAD).
//!
//! Implementa os requisitos de exportação vetorial de alta resolução:
//! - **SVG**: Exporta folha ou seleção em SVG vetorial padrão, com Inter embutida, linecap/linejoin arredondados,
//!   arcos matematicamente exatos e elipses não facetadas.
//! - **PNG**: Renderização raster de impressão em 300 DPI (~3508 px para A4 paisagem) via `resvg`/`tiny-skia`.
//! - **PDF**: PDF vetorial puro e multipágina via `svg2pdf` e `pdf-writer`, com texto selecionável e curvas perfeitas.
//! - **CAD**: Exportação byte-exata no formato CP-1252 original do CADe_SIMU 4.0.
//! - **Cajetín Estruturado**: Moldura de legenda dividida em células (Entidade, Data, Título, Autor, Arquivo, Folha)
//!   com truncamento inteligente anti-transbordamento.
//! - **Modo Monocromático / Alto Contraste**: Preto e cinza de alto contraste para impressão técnica.
//! - **Bornes Opcionais**: Supressão dos círculos vermelhos nos pinos para documentos limpos.

use std::fmt;
use std::io::Write;

use flate2::write::ZlibEncoder;
use flate2::Compression;
use pdf_writer::Finish;

use crate::model::{
    transform_offset, transform_offset_f32, ProjectPage, SchematicDocument, SchematicItem,
};
use crate::symbols::find_symbol_ref;

/// Escala base do SVG em pixels por unidade de mundo CADe_SIMU a 300 DPI.
/// 1 unidade CADe = 8.0 pixels.
/// 1 módulo do grid (6 unidades CADe) = 48.0 pixels (4.0 mm).
/// 1 meio módulo (3 unidades CADe) = 24.0 pixels (2.0 mm).
pub const SVG_SCALE: f32 = 8.0;

/// Fonte TrueType Inter embutida para renderização independente do sistema operacional.
pub static INTER_FONT_DATA: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");

/// Cria opções padrão para o `resvg`/`usvg` configuradas com a fonte Inter embutida e fontes do sistema.
pub fn create_usvg_options() -> resvg::usvg::Options<'static> {
    let mut opt = resvg::usvg::Options::default();
    opt.fontdb_mut().load_system_fonts();
    opt.fontdb_mut().load_font_data(INTER_FONT_DATA.to_vec());
    opt.fontdb_mut().set_sans_serif_family("Inter");
    opt
}

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
            ExportFormat::Pdf => "Documento PDF Vetorial (*.pdf)",
            ExportFormat::Svg => "Gráfico Vetorial SVG (*.svg)",
            ExportFormat::Png => "Imagem PNG 300 DPI (*.png)",
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

/// Opções completas de exportação gráfica (SVG, PNG, PDF).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportOptions {
    /// Se true, inclui a moldura externa, réguas e carimbo de legenda (cajetín).
    pub include_cajetin: bool,
    /// Se true, desenha os círculos de bornes (#d32f2f) em cada terminal de conexão.
    /// Padrão é `false` para documentos de engenharia limpos sem ruído visual.
    pub show_terminals: bool,
    /// Se true, exporta em modo monocromático / alto contraste para impressão.
    pub monochrome: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            include_cajetin: true,
            show_terminals: false,
            monochrome: false,
        }
    }
}

impl From<bool> for ExportOptions {
    fn from(include_cajetin: bool) -> Self {
        Self {
            include_cajetin,
            show_terminals: false,
            monochrome: false,
        }
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

/// Trunca strings longas adicionando reticências para proteger células do cajetín.
fn truncate_str(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
        format!("{truncated}...")
    }
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
                    (*r * parsed.scale_factor()) * scale
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
                let (lx, ly) = parsed.map_point(*cx, *cy, target_span);
                let (rot_x, rot_y) = transform_offset_f32(lx, ly, orientation);
                let sc_cx = (ox + rot_x) * scale;
                let sc_cy = (oy + rot_y) * scale;

                let scale_factor = if parsed.is_cade_svg {
                    scale / 5.0
                } else if target_span > 1.0
                    && (parsed.term_bottom_y - parsed.term_top_y).abs() > 0.1
                {
                    (target_span / (parsed.term_bottom_y - parsed.term_top_y).abs()) * scale
                } else if parsed.is_qet_native {
                    scale
                } else {
                    parsed.scale_factor() * scale
                };

                let (m_rx, m_ry) = if orientation.rem_euclid(2) == 1 {
                    (*ry * scale_factor, *rx * scale_factor)
                } else {
                    (*rx * scale_factor, *ry * scale_factor)
                };

                let fill_attr = match resolve_fill(*fill) {
                    Some(col) => format!(" fill=\"{col}\""),
                    None => " fill=\"none\"".to_string(),
                };
                out.push_str(&format!(
                    r###"  <ellipse cx="{sc_cx:.2}" cy="{sc_cy:.2}" rx="{m_rx:.2}" ry="{m_ry:.2}" stroke="{stroke_color}" stroke-width="{stroke_width:.1}"{fill_attr}/>"###
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
                    let fs = (size * parsed.scale_factor() * scale).clamp(5.0, 16.0);
                    (fs, -fs * 0.82)
                };

                let total_rot_deg = match orientation.rem_euclid(4) {
                    1 => *rotation + 90.0,
                    2 => *rotation + 180.0,
                    3 => *rotation + 270.0,
                    _ => *rotation,
                };

                let baseline_offset = font_size * 0.78;
                if total_rot_deg.abs() > 0.1 {
                    out.push_str(&format!(
                        r###"  <text transform="translate({raw_pos_x:.2}, {raw_pos_y:.2}) rotate({total_rot_deg:.1})" y="{:.2}" font-size="{font_size:.1}" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{stroke_color}">{}</text>"###,
                        offset_y + baseline_offset,
                        xml_escape(text)
                    ));
                } else {
                    out.push_str(&format!(
                        r###"  <text x="{raw_pos_x:.2}" y="{:.2}" font-size="{font_size:.1}" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{stroke_color}">{}</text>"###,
                        raw_pos_y + offset_y + baseline_offset,
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
                let cx = (x1 + x2) as f32 / 10.0;
                let cy = (y1 + y2) as f32 / 10.0;
                let rx = ((x2 - x1).abs() as f32 / 10.0).max(0.05);
                let ry = ((y2 - y1).abs() as f32 / 10.0).max(0.05);
                let (rot_cx, rot_cy) = transform_offset_f32(cx, cy, orientation);
                let (r_rx, r_ry) = if orientation.rem_euclid(2) == 1 {
                    (ry, rx)
                } else {
                    (rx, ry)
                };
                let center_x = (ox + rot_cx) * scale;
                let center_y = (oy + rot_cy) * scale;
                let rad_x = r_rx * scale;
                let rad_y = r_ry * scale;
                let fill_attr = match fill_active {
                    Some(col) => format!(" fill=\"{col}\""),
                    None => " fill=\"none\"".to_string(),
                };
                out.push_str(&format!(
                    r###"  <ellipse cx="{center_x:.2}" cy="{center_y:.2}" rx="{rad_x:.2}" ry="{rad_y:.2}" stroke="{stroke_color}" stroke-width="{stroke_width:.1}"{fill_attr}/>"###
                ));
                out.push('\n');
            }
            crate::symbols::VectorPrimitive::Arc {
                x1,
                y1,
                x2,
                y2,
                x3,
                y3,
                x4,
                y4,
            } => {
                let geom =
                    crate::symbols::VectorPrimitive::arc_geometry(x1, y1, x2, y2, x3, y3, x4, y4);
                let (rot_sx, rot_sy) =
                    transform_offset_f32(geom.start_x, geom.start_y, orientation);
                let (rot_ex, rot_ey) = transform_offset_f32(geom.end_x, geom.end_y, orientation);
                let (r_rx, r_ry) = if orientation.rem_euclid(2) == 1 {
                    (geom.ry, geom.rx)
                } else {
                    (geom.rx, geom.ry)
                };
                let sx = (ox + rot_sx) * scale;
                let sy = (oy + rot_sy) * scale;
                let ex = (ox + rot_ex) * scale;
                let ey = (oy + rot_ey) * scale;
                let rad_x = r_rx * scale;
                let rad_y = r_ry * scale;
                let large_arc_flag = if geom.large_arc { 1 } else { 0 };
                let sweep_flag = if orientation.rem_euclid(8) >= 4 { 1 } else { 0 };
                out.push_str(&format!(
                    r###"  <path d="M {sx:.2} {sy:.2} A {rad_x:.2} {rad_y:.2} 0 {large_arc_flag} {sweep_flag} {ex:.2} {ey:.2}" stroke="{stroke_color}" stroke-width="{stroke_width:.1}" fill="none"/>"###
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
                let baseline_y = pos_y + font_size * 0.78;
                out.push_str(&format!(
                    r###"  <text x="{pos_x:.2}" y="{baseline_y:.2}" font-size="{font_size:.1}" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{stroke_color}">{}</text>"###,
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
///   com bounding box ajustado sem cortar rótulos. Se `None`, exporta a folha inteira.
/// - `options`: Configurações de cajetín, exibição de bornes e modo monocromático.
pub fn export_to_svg(
    doc: &SchematicDocument,
    items_filter: Option<&[i32]>,
    options: impl Into<ExportOptions>,
) -> String {
    let opts = options.into();
    let scale = SVG_SCALE;

    // Folha A4 padrão em escala CADe (330 x 233.35) preservando rigorosamente a razão de aspecto √2 (~1.4142):
    let (sheet_w, sheet_h) =
        if doc.settings.custom_width_mm > 0 && doc.settings.custom_height_mm > 0 {
            let w_mm = doc.settings.custom_width_mm as f32;
            let h_mm = doc.settings.custom_height_mm as f32;
            let w_cad = (w_mm * (330.0 / 297.0)).max(100.0);
            let h_cad = (h_mm * (330.0 / 297.0)).max(100.0);
            (w_cad, h_cad)
        } else {
            (330.0, 330.0 * (210.0 / 297.0))
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
                        let s = parsed.scale_factor();
                        let (p1x, p1y) = transform_offset_f32(vx * s, vy * s, item.orientation);
                        let (p2x, p2y) =
                            transform_offset_f32((vx + vw) * s, (vy + vh) * s, item.orientation);
                        min_x = min_x.min(item.x1 as f32 + p1x.min(p2x));
                        max_x = max_x.max(item.x1 as f32 + p1x.max(p2x));
                        min_y = min_y.min(item.y1 as f32 + p1y.min(p2y));
                        max_y = max_y.max(item.y1 as f32 + p1y.max(p2y));
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
                    min_x = min_x.min((item.x1 + p1x.min(p2x)) as f32);
                    max_x = max_x.max((item.x1 + p1x.max(p2x)) as f32);
                    min_y = min_y.min((item.y1 + p1y.min(p2y)) as f32);
                    max_y = max_y.max((item.y1 + p1y.max(p2y)) as f32);

                    // Inclui posições de pinos e rótulos de bornes
                    for (idx, _lbl, (wx, wy)) in sym.world_pins(item) {
                        min_x = min_x.min(wx as f32);
                        max_x = max_x.max(wx as f32);
                        min_y = min_y.min(wy as f32);
                        max_y = max_y.max(wy as f32);
                        if doc.settings.show_pin_refs >= 1 && item.pin_visibility[idx] == 1 {
                            let p_def = &sym.pins[idx];
                            let (lx, ly) = transform_offset(
                                p_def.label_pos.0,
                                p_def.label_pos.1,
                                item.orientation,
                            );
                            min_x = min_x.min((item.x1 + lx) as f32 - 1.0);
                            max_x = max_x.max((item.x1 + lx) as f32 + 4.0);
                            min_y = min_y.min((item.y1 + ly) as f32 - 1.0);
                            max_y = max_y.max((item.y1 + ly) as f32 + 3.0);
                        }
                    }
                } else {
                    min_x = min_x.min(item.x1 as f32 - 2.0);
                    max_x = max_x.max(item.x1 as f32 + 10.0);
                    min_y = min_y.min(item.y1 as f32 - 2.0);
                    max_y = max_y.max(item.y1 as f32 + 10.0);
                }

                // Inclui tag do componente no bounding box para evitar corte
                if item.show_tag == 1 && !item.tag.is_empty() {
                    let (tx, ty) = match item.tipo {
                        3000..=3009 => (item.x1 - 5, item.y1 - 5),
                        1000..=1012 => (item.x1 - 8, item.y1 + 9),
                        _ => (item.x1 - 8, item.y1 + 3),
                    };
                    let tag_width = (item.tag.len() as f32) * 2.4;
                    min_x = min_x.min(tx as f32 - 1.0);
                    max_x = max_x.max(tx as f32 + tag_width + 1.0);
                    min_y = min_y.min(ty as f32 - 1.0);
                    max_y = max_y.max(ty as f32 + 4.0);
                }
            }

            let pad = 6.0;
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

    // Definições de estilo e fontes
    out.push_str(
        r###"  <defs>
    <style>
      text { font-family: 'Inter', Arial, 'DejaVu Sans', sans-serif; }
    </style>
  </defs>
"###,
    );

    // Fundo branco do papel
    out.push_str(&format!(
        r###"  <rect x="{view_x:.1}" y="{view_y:.1}" width="{view_w:.1}" height="{view_h:.1}" fill="#ffffff"/>"###
    ));
    out.push('\n');

    // Moldura e Cajetín (apenas se página completa e cajetin ativado)
    if !is_selection && opts.include_cajetin {
        let border_stroke_w = 1.5;
        let border_col = if opts.monochrome {
            "#000000"
        } else {
            "#1b2230"
        };
        let guide_text_col = if opts.monochrome {
            "#475569"
        } else {
            "#5c6675"
        };

        // Moldura externa da folha
        let outer_w = sheet_w * scale;
        let outer_h = sheet_h * scale;
        out.push_str(&format!(
            r###"  <rect x="0" y="0" width="{outer_w:.1}" height="{outer_h:.1}" fill="none" stroke="{border_col}" stroke-width="{border_stroke_w}"/>"###
        ));
        out.push('\n');

        // Moldura interna da folha
        let inner_x = 6.0 * scale;
        let inner_y = 6.0 * scale;
        let inner_w = (sheet_w - 12.0) * scale;
        let inner_h = (sheet_h - 12.0) * scale;
        out.push_str(&format!(
            r###"  <rect x="{inner_x:.1}" y="{inner_y:.1}" width="{inner_w:.1}" height="{inner_h:.1}" fill="none" stroke="{border_col}" stroke-width="{border_stroke_w}"/>"###
        ));
        out.push('\n');

        // Referências numéricas de colunas (1..8)
        let font_label_sz = (3.2 * scale).clamp(8.0, 14.0);
        let label_baseline_offset = font_label_sz * 0.78;
        for col in 0..8 {
            let cx = (6.0 + (col as f32 + 0.5) * ((sheet_w - 12.0) / 8.0)) * scale;
            let y_top = 1.2 * scale + label_baseline_offset;
            let y_bot = (sheet_h - 4.5) * scale + label_baseline_offset;
            out.push_str(&format!(
                r###"  <text x="{cx:.1}" y="{y_top:.1}" text-anchor="middle" font-size="{font_label_sz:.1}" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{guide_text_col}">{}</text>"###,
                col + 1
            ));
            out.push('\n');
            out.push_str(&format!(
                r###"  <text x="{cx:.1}" y="{y_bot:.1}" text-anchor="middle" font-size="{font_label_sz:.1}" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{guide_text_col}">{}</text>"###,
                col + 1
            ));
            out.push('\n');
        }

        // Referências alfabéticas de linhas (A..F)
        for (row, ch) in ['A', 'B', 'C', 'D', 'E', 'F'].iter().enumerate() {
            let ry = (6.0 + (row as f32 + 0.5) * ((sheet_h - 12.0) / 6.0)) * scale
                + label_baseline_offset;
            out.push_str(&format!(
                r###"  <text x="{:.1}" y="{ry:.1}" text-anchor="middle" font-size="{font_label_sz:.1}" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{guide_text_col}">{}</text>"###,
                1.8 * scale,
                ch
            ));
            out.push('\n');
            out.push_str(&format!(
                r###"  <text x="{:.1}" y="{ry:.1}" text-anchor="middle" font-size="{font_label_sz:.1}" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{guide_text_col}">{}</text>"###,
                (sheet_w - 4.5) * scale,
                ch
            ));
            out.push('\n');
        }

        // Bloco de legenda (Cajetín) estruturado em células no canto inferior direito
        let caj_w = 115.0;
        let caj_h = 22.0;
        let caj_x = sheet_w - caj_w - 6.0;
        let caj_y = sheet_h - caj_h - 6.0;
        let cp_x = caj_x * scale;
        let cp_y = caj_y * scale;
        let cs_w = caj_w * scale;
        let cs_h = caj_h * scale;

        let caj_bg = if opts.monochrome {
            "#ffffff"
        } else {
            "#f8fafc"
        };
        let div_line = if opts.monochrome {
            "#94a3b8"
        } else {
            "#cbd5e1"
        };

        out.push_str(&format!(
            r###"  <rect x="{cp_x:.1}" y="{cp_y:.1}" width="{cs_w:.1}" height="{cs_h:.1}" fill="{caj_bg}" stroke="{border_col}" stroke-width="{border_stroke_w}"/>"###
        ));
        out.push('\n');

        // Divisórias horizontais (y = 7.0 e y = 14.0)
        let line_y1 = (caj_y + 7.0) * scale;
        let line_y2 = (caj_y + 14.0) * scale;
        out.push_str(&format!(
            r###"  <line x1="{cp_x:.1}" y1="{line_y1:.1}" x2="{:.1}" y2="{line_y1:.1}" stroke="{div_line}" stroke-width="1.0"/>"###,
            cp_x + cs_w
        ));
        out.push('\n');
        out.push_str(&format!(
            r###"  <line x1="{cp_x:.1}" y1="{line_y2:.1}" x2="{:.1}" y2="{line_y2:.1}" stroke="{div_line}" stroke-width="1.0"/>"###,
            cp_x + cs_w
        ));
        out.push('\n');

        // Divisória vertical Linha 1 (x = 75.0: Entidade | Data)
        let col_x1 = (caj_x + 75.0) * scale;
        out.push_str(&format!(
            r###"  <line x1="{col_x1:.1}" y1="{cp_y:.1}" x2="{col_x1:.1}" y2="{line_y1:.1}" stroke="{div_line}" stroke-width="1.0"/>"###
        ));
        out.push('\n');

        // Divisórias verticais Linha 3 (x = 45.0 e x = 90.0: Desenhado por | Arquivo | Folha)
        let col_x2 = (caj_x + 45.0) * scale;
        let col_x3 = (caj_x + 90.0) * scale;
        out.push_str(&format!(
            r###"  <line x1="{col_x2:.1}" y1="{line_y2:.1}" x2="{col_x2:.1}" y2="{:.1}" stroke="{div_line}" stroke-width="1.0"/>"###,
            cp_y + cs_h
        ));
        out.push('\n');
        out.push_str(&format!(
            r###"  <line x1="{col_x3:.1}" y1="{line_y2:.1}" x2="{col_x3:.1}" y2="{:.1}" stroke="{div_line}" stroke-width="1.0"/>"###,
            cp_y + cs_h
        ));
        out.push('\n');

        let tb = &doc.title_block;
        let lbl_col = if opts.monochrome {
            "#475569"
        } else {
            "#64748b"
        };
        let val_col = if opts.monochrome {
            "#000000"
        } else {
            "#0f172a"
        };
        let lbl_font_sz = (1.8 * scale).clamp(5.0, 9.0);
        let val_font_sz = (2.6 * scale).clamp(7.0, 12.0);

        // Célula 1,1: Entidade
        let c1_x = (caj_x + 2.0) * scale;
        let c1_y = (caj_y + 2.0) * scale + lbl_font_sz * 0.78;
        let c1_val_y = (caj_y + 4.8) * scale + val_font_sz * 0.78;
        out.push_str(&format!(
            r###"  <text x="{c1_x:.1}" y="{c1_y:.1}" font-size="{lbl_font_sz:.1}" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{lbl_col}">Entidade:</text>"###
        ));
        out.push('\n');
        out.push_str(&format!(
            r###"  <text x="{c1_x:.1}" y="{c1_val_y:.1}" font-size="{val_font_sz:.1}" font-weight="bold" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{val_col}">{}</text>"###,
            xml_escape(&truncate_str(&tb.entity, 35))
        ));
        out.push('\n');

        // Célula 1,2: Data
        let c2_x = (caj_x + 77.0) * scale;
        out.push_str(&format!(
            r###"  <text x="{c2_x:.1}" y="{c1_y:.1}" font-size="{lbl_font_sz:.1}" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{lbl_col}">DATA</text>"###
        ));
        out.push('\n');
        out.push_str(&format!(
            r###"  <text x="{c2_x:.1}" y="{c1_val_y:.1}" font-size="{val_font_sz:.1}" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{val_col}">{}</text>"###,
            xml_escape(&truncate_str(&tb.drawn_date, 16))
        ));
        out.push('\n');

        // Célula 2: Título e Subtítulo
        let c3_y = (caj_y + 8.8) * scale + lbl_font_sz * 0.78;
        let c3_val_y = (caj_y + 11.8) * scale + val_font_sz * 0.78;
        let full_title = if tb.subtitle.is_empty() {
            tb.title.clone()
        } else {
            format!("{} — {}", tb.title, tb.subtitle)
        };
        let title_font_sz = (2.9 * scale).clamp(8.0, 13.5);
        out.push_str(&format!(
            r###"  <text x="{c1_x:.1}" y="{c3_y:.1}" font-size="{lbl_font_sz:.1}" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{lbl_col}">TÍTULO DO PROJETO</text>"###
        ));
        out.push('\n');
        out.push_str(&format!(
            r###"  <text x="{c1_x:.1}" y="{c3_val_y:.1}" font-size="{title_font_sz:.1}" font-weight="bold" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{val_col}">{}</text>"###,
            xml_escape(&truncate_str(&full_title, 55))
        ));
        out.push('\n');

        // Célula 3,1: Desenhado por
        let c4_y = (caj_y + 15.6) * scale + lbl_font_sz * 0.78;
        let c4_val_y = (caj_y + 18.8) * scale + val_font_sz * 0.78;
        out.push_str(&format!(
            r###"  <text x="{c1_x:.1}" y="{c4_y:.1}" font-size="{lbl_font_sz:.1}" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{lbl_col}">DESENHADO POR</text>"###
        ));
        out.push('\n');
        out.push_str(&format!(
            r###"  <text x="{c1_x:.1}" y="{c4_val_y:.1}" font-size="{val_font_sz:.1}" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{val_col}">{}</text>"###,
            xml_escape(&truncate_str(&tb.drawn_by, 22))
        ));
        out.push('\n');

        // Célula 3,2: Arquivo
        let c5_x = (caj_x + 47.0) * scale;
        out.push_str(&format!(
            r###"  <text x="{c5_x:.1}" y="{c4_y:.1}" font-size="{lbl_font_sz:.1}" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{lbl_col}">ARQUIVO</text>"###
        ));
        out.push('\n');
        out.push_str(&format!(
            r###"  <text x="{c5_x:.1}" y="{c4_val_y:.1}" font-size="{val_font_sz:.1}" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{val_col}">{}</text>"###,
            xml_escape(&truncate_str(&tb.file_ref, 22))
        ));
        out.push('\n');

        // Célula 3,3: Folha
        let c6_x = (caj_x + 92.0) * scale;
        out.push_str(&format!(
            r###"  <text x="{c6_x:.1}" y="{c4_y:.1}" font-size="{lbl_font_sz:.1}" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{lbl_col}">FOLHA</text>"###
        ));
        out.push('\n');
        out.push_str(&format!(
            r###"  <text x="{c6_x:.1}" y="{c4_val_y:.1}" font-size="{val_font_sz:.1}" font-weight="bold" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{val_col}">{}</text>"###,
            xml_escape(&truncate_str(&tb.sheet, 10))
        ));
        out.push('\n');
    }

    // Grupo com pontas e junções arredondadas para todos os elementos gráficos
    out.push_str("  <g stroke-linecap=\"round\" stroke-linejoin=\"round\">\n");

    // 1. Cabos (fios) primeiro
    for item in &doc.items {
        if let Some(filter) = items_filter {
            if !filter.contains(&item.id) {
                continue;
            }
        }
        if !item.is_wire() {
            continue;
        }

        let ax = item.x1 as f32 * scale;
        let ay = item.y1 as f32 * scale;
        let bx = item.x2 as f32 * scale;
        let by = item.y2 as f32 * scale;

        let (stroke_color, stroke_w) = if opts.monochrome {
            ("#000000".to_string(), 1.4)
        } else if item.color != 0 {
            let r_col = ((item.color >> 16) & 0xFF) as u8;
            let g_col = ((item.color >> 8) & 0xFF) as u8;
            let b_col = (item.color & 0xFF) as u8;
            (format!("#{r_col:02x}{g_col:02x}{b_col:02x}"), 1.6)
        } else {
            let col = match item.tipo {
                30..=39 => "#1b2230",
                40..=49 => "#15803d",
                50..=59 => "#2563eb",
                60..=69 => "#b45309",
                70..=79 => "#b91c1c",
                80..=89 => "#0284c7",
                _ => "#1b2230",
            };
            (col.to_string(), 1.6)
        };

        out.push_str(&format!(
            r###"  <line x1="{ax:.1}" y1="{ay:.1}" x2="{bx:.1}" y2="{by:.1}" stroke="{stroke_color}" stroke-width="{stroke_w:.1}"/>"###
        ));
        out.push('\n');
    }

    // 2. Junções (nós de conexão elétrica)
    for item in &doc.items {
        if let Some(filter) = items_filter {
            if !filter.contains(&item.id) {
                continue;
            }
        }
        if item.is_junction() {
            let cx = item.x1 as f32 * scale;
            let cy = item.y1 as f32 * scale;
            let r = 0.9 * scale;
            let dot_col = if opts.monochrome {
                "#000000".to_string()
            } else if item.color != 0 {
                let r_col = ((item.color >> 16) & 0xFF) as u8;
                let g_col = ((item.color >> 8) & 0xFF) as u8;
                let b_col = (item.color & 0xFF) as u8;
                format!("#{r_col:02x}{g_col:02x}{b_col:02x}")
            } else {
                match item.tipo {
                    4000 | 4018 => "#8c2e1f".to_string(),
                    4009 | 4019 => "#1461d1".to_string(),
                    4002 | 4010 => "#1f9e33".to_string(),
                    4003..=4008 => "#8c2e1f".to_string(),
                    _ => "#1b2230".to_string(),
                }
            };
            out.push_str(&format!(
                r###"  <circle cx="{cx:.1}" cy="{cy:.1}" r="{r:.1}" fill="{dot_col}"/>"###
            ));
            out.push('\n');
        }
    }

    // 3. Símbolos e Componentes
    for item in &doc.items {
        if let Some(filter) = items_filter {
            if !filter.contains(&item.id) {
                continue;
            }
        }
        if item.is_wire() {
            continue;
        }

        // Texto livre (tipo == 3)
        if item.tipo == 3 {
            let px = item.x1 as f32 * scale;
            let py = item.y1 as f32 * scale;
            if !item.trailing_str.is_empty() {
                let font_sz = (3.2 * scale).clamp(8.5, 15.0);
                let text_baseline = py + font_sz * 0.78;
                let text_col = if opts.monochrome {
                    "#000000"
                } else {
                    "#0f141c"
                };
                out.push_str(&format!(
                    r###"  <text x="{px:.1}" y="{text_baseline:.1}" font-size="{font_sz:.1}" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{text_col}">{}</text>"###,
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
            let stroke_col = if opts.monochrome {
                "#000000".to_string()
            } else if item.color != 0 {
                let r_col = ((item.color >> 16) & 0xFF) as u8;
                let g_col = ((item.color >> 8) & 0xFF) as u8;
                let b_col = (item.color & 0xFF) as u8;
                format!("#{r_col:02x}{g_col:02x}{b_col:02x}")
            } else {
                "#1b2230".to_string()
            };
            out.push_str(&format!(
                r###"  <line x1="{ax:.1}" y1="{ay:.1}" x2="{bx:.1}" y2="{by:.1}" stroke="{stroke_col}" stroke-width="1.2"/>"###
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
            let stroke_col = if opts.monochrome {
                "#000000".to_string()
            } else if item.color != 0 {
                let r_col = ((item.color >> 16) & 0xFF) as u8;
                let g_col = ((item.color >> 8) & 0xFF) as u8;
                let b_col = (item.color & 0xFF) as u8;
                format!("#{r_col:02x}{g_col:02x}{b_col:02x}")
            } else {
                "#1b2230".to_string()
            };
            out.push_str(&format!(
                r###"  <rect x="{min_x:.1}" y="{min_y:.1}" width="{w:.1}" height="{h:.1}" stroke="{stroke_col}" stroke-width="1.3" fill="none"/>"###
            ));
            out.push('\n');
            continue;
        }

        // Símbolos QET ou gerados pelo cade2svg
        if let Some(qet_path) = item.symbol_id_or_slug() {
            let stroke_col = if opts.monochrome {
                "#000000".to_string()
            } else if item.color != 0 {
                let r_col = ((item.color >> 16) & 0xFF) as u8;
                let g_col = ((item.color >> 8) & 0xFF) as u8;
                let b_col = (item.color & 0xFF) as u8;
                format!("#{r_col:02x}{g_col:02x}{b_col:02x}")
            } else {
                "#111827".to_string()
            };
            render_svg_symbol_to_svg(
                &mut out,
                item.x1,
                item.y1,
                item.orientation,
                qet_path,
                &stroke_col,
                1.5,
                None,
                1.0,
                scale,
            );
            if opts.show_terminals {
                if let Some(parsed) = crate::svg_symbol::get_parsed_svg(qet_path) {
                    let pin_rad = (crate::model::TERMINAL_RADIUS_UNITS * scale).max(1.2);
                    let pin_col = if opts.monochrome {
                        "#000000"
                    } else {
                        "#d32f2f"
                    };
                    for shape in &parsed.shapes {
                        if let crate::svg_symbol::SvgShape::Circle {
                            cx,
                            cy,
                            is_terminal: true,
                            ..
                        } = shape
                        {
                            let (lx, ly) = parsed.map_point(*cx, *cy, 1.0);
                            let (rx, ry) = transform_offset_f32(lx, ly, item.orientation);
                            let pt_x = ((item.x1 as f32) + rx) * scale;
                            let pt_y = ((item.y1 as f32) + ry) * scale;
                            out.push_str(&format!(
                                r###"  <circle cx="{pt_x:.1}" cy="{pt_y:.1}" r="{pin_rad:.1}" stroke="{pin_col}" stroke-width="1.0" fill="none"/>"###
                            ));
                            out.push('\n');
                        }
                    }
                }
            }
            continue;
        }

        let Some(sym) = find_symbol_ref(item.tipo) else {
            continue;
        };

        let stroke_color = if opts.monochrome {
            "#000000".to_string()
        } else if item.color != 0 && !item.is_lamp_or_signaling() {
            let r_col = ((item.color >> 16) & 0xFF) as u8;
            let g_col = ((item.color >> 8) & 0xFF) as u8;
            let b_col = (item.color & 0xFF) as u8;
            format!("#{r_col:02x}{g_col:02x}{b_col:02x}")
        } else {
            "#111827".to_string()
        };
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
                &stroke_color,
                1.8,
                None,
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
                &stroke_color,
                None,
                1.6,
                scale,
            );
        }

        // Desenha bornes (pinos) se a opção estiver ativada
        if opts.show_terminals {
            for (_idx, _pin_label, (wx, wy)) in sym.world_pins(item) {
                let pt_x = wx as f32 * scale;
                let pt_y = wy as f32 * scale;
                let pin_rad = (crate::model::TERMINAL_RADIUS_UNITS * scale).max(1.2);
                let pin_col = if opts.monochrome {
                    "#000000"
                } else {
                    "#d32f2f"
                };
                out.push_str(&format!(
                    r###"  <circle cx="{pt_x:.1}" cy="{pt_y:.1}" r="{pin_rad:.1}" stroke="{pin_col}" stroke-width="1.0" fill="none"/>"###
                ));
                out.push('\n');
            }
        }

        // Referências numéricas dos pinos (13/14, A1/A2, U1/V1/W1)
        if doc.settings.show_pin_refs >= 1 {
            for (idx, pin_label, (_wx, _wy)) in sym.world_pins(item) {
                if item.pin_visibility[idx] == 1 {
                    let p_def = &sym.pins[idx];
                    let (lx, ly) =
                        transform_offset(p_def.label_pos.0, p_def.label_pos.1, item.orientation);
                    let lpos_x = ((item.x1 + lx) as f32 + 0.4) * scale;
                    let lpos_y = ((item.y1 + ly) as f32 - 0.5) * scale;
                    let font_sz = (2.4 * scale).clamp(6.5, 11.5);
                    let baseline_y = lpos_y + font_sz * 0.78;
                    let text_col = if opts.monochrome {
                        "#374151"
                    } else {
                        "#5c6675"
                    };
                    out.push_str(&format!(
                        r###"  <text x="{lpos_x:.1}" y="{baseline_y:.1}" font-size="{font_sz:.1}" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{text_col}">{}</text>"###,
                        xml_escape(&pin_label)
                    ));
                    out.push('\n');
                }
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
            let baseline_y = tpos_y + font_sz * 0.78;
            let tag_col = if opts.monochrome {
                "#000000"
            } else {
                "#8c2e1f"
            };
            out.push_str(&format!(
                r###"  <text x="{tpos_x:.1}" y="{baseline_y:.1}" font-size="{font_sz:.1}" font-weight="bold" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{tag_col}">{}</text>"###,
                xml_escape(&item.tag)
            ));
            out.push('\n');
        }

        // Valor do componente (tempo em segundos, ohms, etc.)
        if item.show_value == 1 && !item.value.is_empty() {
            let vpos_x = (item.x1 + 5) as f32 * scale;
            let vpos_y = (item.y1 + 5) as f32 * scale;
            let font_sz = (2.7 * scale).clamp(7.0, 12.5);
            let baseline_y = vpos_y + font_sz * 0.78;
            let val_col = if opts.monochrome {
                "#374151"
            } else {
                "#5c6675"
            };
            out.push_str(&format!(
                r###"  <text x="{vpos_x:.1}" y="{baseline_y:.1}" font-size="{font_sz:.1}" font-family="Inter, Arial, DejaVu Sans, sans-serif" fill="{val_col}">{}s</text>"###,
                xml_escape(&item.value)
            ));
            out.push('\n');
        }
    }

    // Fecha o grupo com linecap/linejoin arredondados
    out.push_str("  </g>\n");
    out.push_str("</svg>\n");
    out
}

/// Renderiza uma representação SVG para bytes de imagem PNG com suporte a escala de DPI.
pub fn render_svg_to_png(svg_str: &str, scale: f32) -> Result<Vec<u8>, String> {
    let opt = create_usvg_options();
    let tree = resvg::usvg::Tree::from_str(svg_str, &opt)
        .map_err(|e| format!("Falha ao analisar SVG: {e}"))?;

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

/// Exporta uma folha ou seleção para PNG em alta resolução (300 DPI por padrão para folha inteira ~3508 px).
pub fn export_to_png(
    doc: &SchematicDocument,
    items_filter: Option<&[i32]>,
    options: impl Into<ExportOptions>,
    scale: impl Into<Option<f32>>,
) -> Result<Vec<u8>, String> {
    let opts = options.into();
    let svg = export_to_svg(doc, items_filter, opts);

    let opt = create_usvg_options();
    let tree = resvg::usvg::Tree::from_str(&svg, &opt)
        .map_err(|e| format!("Falha ao analisar SVG para PNG: {e}"))?;

    let size = tree.size().to_int_size();
    let eff_scale = scale.into().unwrap_or_else(|| {
        // Padrão 300 DPI para A4 horizontal: ~3508 px de largura
        (3508.0 / size.width() as f32).max(2.0)
    });

    render_svg_to_png(&svg, eff_scale)
}

/// Exporta uma folha ou seleção de elementos para documento PDF vetorial puro (via `svg2pdf`).
pub fn export_doc_to_pdf(
    doc: &SchematicDocument,
    items_filter: Option<&[i32]>,
    options: impl Into<ExportOptions>,
) -> Result<Vec<u8>, String> {
    let opts = options.into();
    let svg = export_to_svg(doc, items_filter, opts);

    let usvg_opt = create_usvg_options();
    let tree = resvg::usvg::Tree::from_str(&svg, &usvg_opt)
        .map_err(|e| format!("Falha ao analisar SVG para PDF: {e}"))?;

    let mut pdf_bytes = svg2pdf::to_pdf(
        &tree,
        svg2pdf::ConversionOptions::default(),
        svg2pdf::PageOptions::default(),
    );
    if pdf_bytes.starts_with(b"%PDF-1.7") {
        pdf_bytes[7] = b'4';
    }
    Ok(pdf_bytes)
}

/// Exporta um projeto completo multipáginas para um único documento PDF vetorial (uma página por folha).
pub fn export_project_to_pdf(
    pages: &[ProjectPage],
    options: impl Into<ExportOptions>,
) -> Result<Vec<u8>, String> {
    if pages.is_empty() {
        return Err("Nenhuma página disponível no projeto para exportar.".to_string());
    }
    let opts = options.into();

    if pages.len() == 1 {
        return export_doc_to_pdf(&pages[0].doc, None, opts);
    }

    let usvg_opt = create_usvg_options();
    let mut pdf = pdf_writer::Pdf::new();
    pdf.set_version(1, 4);
    let mut alloc = pdf_writer::Ref::new(1);
    let catalog_id = alloc.bump();
    let page_tree_id = alloc.bump();

    let mut page_ids = Vec::with_capacity(pages.len());
    let mut page_chunks = Vec::with_capacity(pages.len());

    for (page_idx, page) in pages.iter().enumerate() {
        let svg = export_to_svg(&page.doc, None, opts);
        let tree = resvg::usvg::Tree::from_str(&svg, &usvg_opt)
            .map_err(|e| format!("Falha ao analisar folha {page_idx} para PDF: {e}"))?;

        let (svg_chunk, raw_svg_id) =
            svg2pdf::to_chunk(&tree, svg2pdf::ConversionOptions::default());

        let page_id = alloc.bump();
        let content_id = alloc.bump();

        let mut id_map = std::collections::HashMap::new();
        let renumbered_chunk =
            svg_chunk.renumber(|old| *id_map.entry(old).or_insert_with(|| alloc.bump()));
        let svg_xobj_id = *id_map
            .get(&raw_svg_id)
            .ok_or_else(|| format!("ID do objeto SVG não encontrado na folha {page_idx}"))?;

        let svg_w = tree.size().width();
        let svg_h = tree.size().height();

        let page_w_pt = 841.89_f32;
        let page_h_pt = 595.28_f32;

        let scale_x = page_w_pt / svg_w;
        let scale_y = page_h_pt / svg_h;
        let fit_scale = scale_x.min(scale_y);

        let final_w = svg_w * fit_scale;
        let final_h = svg_h * fit_scale;
        let offset_x = (page_w_pt - final_w) * 0.5;
        let offset_y = (page_h_pt - final_h) * 0.5;

        let svg_name = pdf_writer::Name(b"SvgSheet");

        let mut pdf_page = pdf.page(page_id);
        pdf_page.media_box(pdf_writer::Rect::new(0.0, 0.0, page_w_pt, page_h_pt));
        pdf_page.parent(page_tree_id);
        pdf_page.contents(content_id);

        let mut resources = pdf_page.resources();
        resources.x_objects().pair(svg_name, svg_xobj_id);
        resources.finish();
        pdf_page.finish();

        let mut content = pdf_writer::Content::new();
        content.save_state();
        content.transform([fit_scale, 0.0, 0.0, fit_scale, offset_x, offset_y]);
        content.x_object(svg_name);
        content.restore_state();

        let stream_bytes = content.finish();
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&stream_bytes)
            .map_err(|e| e.to_string())?;
        let compressed_stream = encoder.finish().map_err(|e| e.to_string())?;

        pdf.stream(content_id, &compressed_stream)
            .filter(pdf_writer::Filter::FlateDecode)
            .finish();

        page_ids.push(page_id);
        page_chunks.push(renumbered_chunk);
    }

    pdf.catalog(catalog_id).pages(page_tree_id);
    pdf.pages(page_tree_id)
        .kids(page_ids)
        .count(pages.len() as i32);

    for chunk in page_chunks {
        pdf.extend(&chunk);
    }

    Ok(pdf.finish())
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
        // Cajetín estruturado ativado
        assert!(svg.contains("Entidade:"));
        assert!(svg.contains("Partida Direta Trifasica"));
        // Wires e componentes
        assert!(svg.contains("<line"));
        assert!(svg.contains("-Q1"));
        // Grupo de cantos arredondados
        assert!(svg.contains("<g stroke-linecap=\"round\" stroke-linejoin=\"round\">"));
        // Bornes desligados por padrão
        assert!(!svg.contains("#d32f2f"));
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
    fn test_export_to_svg_with_terminals_and_monochrome() {
        let doc = build_dol_starter_preset();
        let options = ExportOptions {
            include_cajetin: true,
            show_terminals: true,
            monochrome: true,
        };
        let svg = export_to_svg(&doc, None, options);

        // Com bornes ativados
        assert!(svg.contains("<circle"));
        // Modo monocromático
        assert!(!svg.contains("#8c2e1f")); // Tags em preto, sem marrom
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
    fn test_export_to_pdf_vector_single_and_multipage() {
        let doc1 = build_dol_starter_preset();
        let pdf_single = export_doc_to_pdf(&doc1, None, true).expect("PDF single");

        assert!(pdf_single.starts_with(b"%PDF-"));
        assert!(pdf_single.ends_with(b"%%EOF\n") || pdf_single.ends_with(b"%%EOF"));

        // Multipage
        let page1 = ProjectPage::new(1, "Potência".to_string(), doc1.clone());
        let page2 = ProjectPage::new(2, "Comando".to_string(), doc1);
        let pdf_multi = export_project_to_pdf(&[page1, page2], true).expect("PDF multi");

        assert!(pdf_multi.starts_with(b"%PDF-"));
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

    #[test]
    fn test_export_vector_primitives_arc_and_ellipse() {
        use crate::model::{SchematicDocument, SchematicItem};

        let mut doc = SchematicDocument::default();
        // Item com tipo 8004 (Botão Cogumelo NA, contém comando ARCO nativo sem SVG pré-compilado)
        doc.items
            .push(SchematicItem::new_symbol(1, 8004, 50, 50, "-S1"));
        // Item com tipo 3000 (Alimentação Fase, contém comando ELIP nos idle_cmds)
        doc.items
            .push(SchematicItem::new_symbol(2, 3000, 20, 20, "-X1"));

        let svg = export_to_svg(&doc, None, false);
        // Deve conter o path com arco vetorial (A rx ry ...) e NÃO circle
        assert!(svg.contains("<path d=\"M "));
        assert!(svg.contains(" A "));
        // Deve conter ellipse com rx e ry reais e NÃO circle
        assert!(svg.contains("<ellipse "));
        assert!(svg.contains("rx=\""));
        assert!(svg.contains("ry=\""));
    }
}
