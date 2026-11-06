//! Módulo de Exportação Real (Folha → SVG / PNG / PDF / CAD).
//!
//! Implementa os requisitos de P1-#9:
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
            (0.0, 0.0, sheet_w, sheet_h)
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
                } else {
                    min_x = min_x.min(item.x1 as f32);
                    max_x = max_x.max(item.x1 as f32 + 10.0);
                    min_y = min_y.min(item.y1 as f32);
                    max_y = max_y.max(item.y1 as f32 + 10.0);
                }
            }

            let pad = 12.0;
            let vx = (min_x - pad).max(0.0);
            let vy = (min_y - pad).max(0.0);
            let vw = (max_x - min_x + pad * 2.0).max(40.0);
            let vh = (max_y - min_y + pad * 2.0).max(30.0);
            (vx, vy, vw, vh)
        }
    } else {
        (0.0, 0.0, sheet_w, sheet_h)
    };

    let px_w = (view_w * 3.5).round();
    let px_h = (view_h * 3.5).round();

    let mut out = String::with_capacity(32_768);
    out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        r###"<svg xmlns="http://www.w3.org/2000/svg" width="{px_w}" height="{px_h}" viewBox="{view_x:.1} {view_y:.1} {view_w:.1} {view_h:.1}">"###
    ));
    out.push('\n');

    // Fundo branco
    out.push_str(&format!(
        r###"  <rect x="{view_x:.1}" y="{view_y:.1}" width="{view_w:.1}" height="{view_h:.1}" fill="#ffffff"/>"###
    ));
    out.push('\n');

    // Moldura e Cajetín (apenas se página completa e cajetin ativado)
    if !is_selection && include_cajetin {
        // Molduras externa e interna
        out.push_str(&format!(
            r###"  <rect x="0" y="0" width="{sheet_w:.1}" height="{sheet_h:.1}" fill="none" stroke="#1e293b" stroke-width="1.5"/>"###
        ));
        out.push('\n');
        out.push_str(&format!(
            r###"  <rect x="6" y="6" width="{:.1}" height="{:.1}" fill="none" stroke="#1e293b" stroke-width="1.5"/>"###,
            sheet_w - 12.0,
            sheet_h - 12.0
        ));
        out.push('\n');

        // Referências numéricas de colunas (1..8)
        for col in 0..8 {
            let cx = 6.0 + (col as f32 + 0.5) * ((sheet_w - 12.0) / 8.0);
            out.push_str(&format!(
                r###"  <text x="{cx:.1}" y="4.5" text-anchor="middle" font-size="2.8" fill="#64748b" font-family="sans-serif">{}</text>"###,
                col + 1
            ));
            out.push('\n');
            out.push_str(&format!(
                r###"  <text x="{cx:.1}" y="{:.1}" text-anchor="middle" font-size="2.8" fill="#64748b" font-family="sans-serif">{}</text>"###,
                sheet_h - 1.5,
                col + 1
            ));
            out.push('\n');
        }

        // Referências alfabéticas de linhas (A..F)
        for (row, ch) in ['A', 'B', 'C', 'D', 'E', 'F'].iter().enumerate() {
            let ry = 6.0 + (row as f32 + 0.5) * ((sheet_h - 12.0) / 6.0);
            out.push_str(&format!(
                r###"  <text x="3.5" y="{ry:.1}" text-anchor="middle" dominant-baseline="central" font-size="2.8" fill="#64748b" font-family="sans-serif">{}</text>"###,
                ch
            ));
            out.push('\n');
            out.push_str(&format!(
                r###"  <text x="{:.1}" y="{ry:.1}" text-anchor="middle" dominant-baseline="central" font-size="2.8" fill="#64748b" font-family="sans-serif">{}</text>"###,
                sheet_w - 2.5,
                ch
            ));
            out.push('\n');
        }

        // Bloco de legenda (Cajetín) no canto inferior direito
        let caj_w = 115.0;
        let caj_h = 22.0;
        let caj_x = sheet_w - caj_w - 6.0;
        let caj_y = sheet_h - caj_h - 6.0;

        out.push_str(&format!(
            r###"  <rect x="{caj_x:.1}" y="{caj_y:.1}" width="{caj_w:.1}" height="{caj_h:.1}" fill="#f8fafc" stroke="#1e293b" stroke-width="1.2"/>"###
        ));
        out.push('\n');
        out.push_str(&format!(
            r###"  <line x1="{caj_x:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#cbd5e1" stroke-width="0.8"/>"###,
            caj_y + 7.0,
            caj_x + caj_w,
            caj_y + 7.0
        ));
        out.push('\n');
        out.push_str(&format!(
            r###"  <line x1="{caj_x:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="#cbd5e1" stroke-width="0.8"/>"###,
            caj_y + 14.0,
            caj_x + caj_w,
            caj_y + 14.0
        ));
        out.push('\n');

        let tb = &doc.title_block;
        out.push_str(&format!(
            r###"  <text x="{:.1}" y="{:.1}" font-size="2.6" fill="#475569" font-family="sans-serif">Entidade: {}  |  Data: {}</text>"###,
            caj_x + 3.0,
            caj_y + 4.8,
            xml_escape(&tb.entity),
            xml_escape(&tb.drawn_date)
        ));
        out.push('\n');
        out.push_str(&format!(
            r###"  <text x="{:.1}" y="{:.1}" font-size="3.0" font-weight="bold" fill="#0f172a" font-family="sans-serif">Título: {} - {}</text>"###,
            caj_x + 3.0,
            caj_y + 11.5,
            xml_escape(&tb.title),
            xml_escape(&tb.subtitle)
        ));
        out.push('\n');
        out.push_str(&format!(
            r###"  <text x="{:.1}" y="{:.1}" font-size="2.6" fill="#475569" font-family="sans-serif">Folha: {}  |  Arquivo: {}</text>"###,
            caj_x + 3.0,
            caj_y + 18.5,
            xml_escape(&tb.sheet),
            xml_escape(&tb.file_ref)
        ));
        out.push('\n');
    }

    // 1. Desenha condutores / fios primeiro
    for item in &doc.items {
        if let Some(filter) = items_filter {
            if !filter.contains(&item.id) {
                continue;
            }
        }
        if item.is_wire() {
            let (color, width) = match item.tipo {
                4000 => ("#8c2e1f", 1.5), // L1
                4002 => ("#000000", 1.5), // L2
                4003 => ("#935a29", 1.5), // L3
                4004 => ("#0284c7", 1.5), // N
                4005 => ("#16a34a", 1.5), // PE
                4006 => ("#dc2626", 1.5), // DC+
                4007 => ("#2563eb", 1.5), // DC-
                _ => ("#8c2e1f", 1.5),
            };
            out.push_str(&format!(
                r###"  <line x1="{}" y1="{}" x2="{}" y2="{}" stroke="{color}" stroke-width="{width}" stroke-linecap="round"/>"###,
                item.x1, item.y1, item.x2, item.y2
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
            let color = match item.tipo {
                4004 => "#0284c7",
                4005 => "#16a34a",
                4006 => "#dc2626",
                4007 => "#2563eb",
                _ => "#8c2e1f",
            };
            out.push_str(&format!(
                r###"  <circle cx="{}" cy="{}" r="1.6" fill="{color}"/>"###,
                item.x1, item.y1
            ));
            out.push('\n');
            continue;
        }

        // Texto livre (tipo == 8)
        if item.tipo == 8 {
            if !item.trailing_str.is_empty() {
                out.push_str(&format!(
                    r###"  <text x="{}" y="{}" font-size="3.6" font-family="sans-serif" fill="#0f172a">{}</text>"###,
                    item.x1,
                    item.y1,
                    xml_escape(&item.trailing_str)
                ));
                out.push('\n');
            }
            continue;
        }

        // Símbolos eletrotécnicos
        let ox = item.x1 as f32;
        let oy = item.y1 as f32;
        let orientation = item.orientation;

        let Some(sym) = find_symbol_ref(item.tipo) else {
            // Fallback para símbolo desconhecido
            out.push_str(&format!(
                r###"  <rect x="{}" y="{}" width="12" height="14" fill="none" stroke="#d97706" stroke-width="1.2" stroke-dasharray="3,2"/>"###,
                item.x1 - 2,
                item.y1 - 2
            ));
            out.push('\n');
            out.push_str(&format!(
                r###"  <text x="{}" y="{}" font-size="2.6" fill="#d97706" font-family="sans-serif">{}</text>"###,
                item.x1,
                item.y1 + 4,
                item.tipo
            ));
            out.push('\n');
            continue;
        };

        let stroke_col = "#0f172a";
        let is_lamp = item.is_lamp_or_signaling();
        let fill_lamp = if is_lamp {
            match item.color {
                1 => Some("#fee2e2"), // Vermelho
                2 => Some("#fef3c7"), // Amarelo
                3 => Some("#e0f2fe"), // Azul
                4 => Some("#f1f5f9"), // Branco
                _ => Some("#dcfce7"), // Verde
            }
        } else {
            None
        };

        let mut drew_svg = false;
        if let Some((svg_idle, _)) = sym.svg_paths() {
            let max_pin_y = sym.pins.iter().map(|p| p.conn_pos.1).fold(0, i32::max) as f32;
            let target_span = if max_pin_y > 0.0 { max_pin_y } else { 12.0 };

            if let Some(parsed) = crate::svg_symbol::get_parsed_svg(svg_idle) {
                if !parsed.shapes.is_empty() {
                    drew_svg = true;
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
                                let dash_attr = if *dashed {
                                    " stroke-dasharray=\"4,3\""
                                } else {
                                    ""
                                };
                                out.push_str(&format!(
                                    r###"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{stroke_col}" stroke-width="1.2"{dash_attr}/>"###,
                                    ox + rx1,
                                    oy + ry1,
                                    ox + rx2,
                                    oy + ry2
                                ));
                                out.push('\n');
                            }
                            crate::svg_symbol::SvgShape::Polyline { points } => {
                                if points.is_empty() {
                                    continue;
                                }
                                let pts: Vec<String> = points
                                    .iter()
                                    .map(|&(px, py)| {
                                        let (lx, ly) = parsed.map_point(px, py, target_span);
                                        let (rx, ry) = transform_offset_f32(lx, ly, orientation);
                                        format!("{:.2},{:.2}", ox + rx, oy + ry)
                                    })
                                    .collect();
                                out.push_str(&format!(
                                    r###"  <polyline points="{}" stroke="{stroke_col}" stroke-width="1.2" fill="none"/>"###,
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
                                        format!("{:.2},{:.2}", ox + rx, oy + ry)
                                    })
                                    .collect();
                                let fill_col = fill_lamp.unwrap_or(match fill {
                                    crate::svg_symbol::SvgFill::Solid => "#0f172a",
                                    crate::svg_symbol::SvgFill::White => "#ffffff",
                                    crate::svg_symbol::SvgFill::Tint => "#f1f5f9",
                                    crate::svg_symbol::SvgFill::None => "none",
                                });
                                out.push_str(&format!(
                                    r###"  <polygon points="{}" stroke="{stroke_col}" stroke-width="1.2" fill="{fill_col}"/>"###,
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
                                let rad = if parsed.is_qet_native {
                                    *r
                                } else if parsed.is_cade_svg {
                                    *r / 5.0
                                } else {
                                    *r * (target_span
                                        / (parsed.term_bottom_y - parsed.term_top_y).abs().max(1.0))
                                };
                                let fill_col = fill_lamp.unwrap_or(match fill {
                                    crate::svg_symbol::SvgFill::Solid => "#0f172a",
                                    crate::svg_symbol::SvgFill::White => "#ffffff",
                                    crate::svg_symbol::SvgFill::Tint => "#f1f5f9",
                                    crate::svg_symbol::SvgFill::None => "none",
                                });
                                out.push_str(&format!(
                                    r###"  <circle cx="{:.2}" cy="{:.2}" r="{:.2}" stroke="{stroke_col}" stroke-width="1.2" fill="{fill_col}"/>"###,
                                    ox + rx,
                                    oy + ry,
                                    rad
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
                                let (rx_pt, ry_pt) = transform_offset_f32(lx, ly, orientation);
                                let (rad_x, rad_y) = if orientation % 2 != 0 {
                                    (*ry, *rx)
                                } else {
                                    (*rx, *ry)
                                };
                                let fill_col = fill_lamp.unwrap_or(match fill {
                                    crate::svg_symbol::SvgFill::Solid => "#0f172a",
                                    crate::svg_symbol::SvgFill::White => "#ffffff",
                                    crate::svg_symbol::SvgFill::Tint => "#f1f5f9",
                                    crate::svg_symbol::SvgFill::None => "none",
                                });
                                out.push_str(&format!(
                                    r###"  <ellipse cx="{:.2}" cy="{:.2}" rx="{:.2}" ry="{:.2}" stroke="{stroke_col}" stroke-width="1.2" fill="{fill_col}"/>"###,
                                    ox + rx_pt,
                                    oy + ry_pt,
                                    rad_x,
                                    rad_y
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
                                        format!("{:.2},{:.2}", ox + rx, oy + ry)
                                    })
                                    .collect();
                                let dash_attr = if *dashed {
                                    " stroke-dasharray=\"4,3\""
                                } else {
                                    ""
                                };
                                let fill_col = fill_lamp.unwrap_or(match fill {
                                    crate::svg_symbol::SvgFill::Solid => "#0f172a",
                                    crate::svg_symbol::SvgFill::White => "#ffffff",
                                    crate::svg_symbol::SvgFill::Tint => "#f1f5f9",
                                    crate::svg_symbol::SvgFill::None => "none",
                                });
                                out.push_str(&format!(
                                    r###"  <polygon points="{}" stroke="{stroke_col}" stroke-width="1.2" fill="{fill_col}"{dash_attr}/>"###,
                                    pts.join(" ")
                                ));
                                out.push('\n');
                            }
                            crate::svg_symbol::SvgShape::Text {
                                x, y, text, size, ..
                            } => {
                                let (lx, ly) = parsed.map_point(*x, *y, target_span);
                                let (rx, ry) = transform_offset_f32(lx, ly, orientation);
                                let font_sz = if parsed.is_qet_native {
                                    *size * 0.9
                                } else {
                                    *size * 0.25
                                };
                                out.push_str(&format!(
                                    r###"  <text x="{:.2}" y="{:.2}" font-size="{:.2}" font-family="sans-serif" fill="{stroke_col}">{}</text>"###,
                                    ox + rx,
                                    oy + ry,
                                    font_sz,
                                    xml_escape(text)
                                ));
                                out.push('\n');
                            }
                        }
                    }
                }
            }
        }

        // Fallback de comandos nativos do CADe
        if !drew_svg {
            for &cmd in sym.idle_cmds {
                if let Some(prim) = crate::symbols::parse_vector_cmd(cmd) {
                    match prim {
                        crate::symbols::VectorPrimitive::Line { x1, y1, x2, y2 } => {
                            let (rx1, ry1) =
                                transform_offset_f32(x1 as f32 / 5.0, y1 as f32 / 5.0, orientation);
                            let (rx2, ry2) =
                                transform_offset_f32(x2 as f32 / 5.0, y2 as f32 / 5.0, orientation);
                            out.push_str(&format!(
                                r###"  <line x1="{:.2}" y1="{:.2}" x2="{:.2}" y2="{:.2}" stroke="{stroke_col}" stroke-width="1.2"/>"###,
                                ox + rx1,
                                oy + ry1,
                                ox + rx2,
                                oy + ry2
                            ));
                            out.push('\n');
                        }
                        crate::symbols::VectorPrimitive::Ellipse { x1, y1, x2, y2 } => {
                            let cx = (x1 + x2) as f32 * 0.1;
                            let cy = (y1 + y2) as f32 * 0.1;
                            let rx = (x2 - x1).abs() as f32 * 0.1;
                            let ry = (y2 - y1).abs() as f32 * 0.1;
                            let (rcx, rcy) = transform_offset_f32(cx, cy, orientation);
                            let (erx, ery) = if orientation % 2 != 0 {
                                (ry, rx)
                            } else {
                                (rx, ry)
                            };
                            let fill_col = fill_lamp.unwrap_or("none");
                            out.push_str(&format!(
                                r###"  <ellipse cx="{:.2}" cy="{:.2}" rx="{:.2}" ry="{:.2}" stroke="{stroke_col}" stroke-width="1.2" fill="{fill_col}"/>"###,
                                ox + rcx,
                                oy + rcy,
                                erx,
                                ery
                            ));
                            out.push('\n');
                        }
                        crate::symbols::VectorPrimitive::Arc { x1, y1, x2, y2, .. } => {
                            let (rx1, ry1) =
                                transform_offset_f32(x1 as f32 / 5.0, y1 as f32 / 5.0, orientation);
                            let (rx2, ry2) =
                                transform_offset_f32(x2 as f32 / 5.0, y2 as f32 / 5.0, orientation);
                            let r = ((rx2 - rx1).hypot(ry2 - ry1) * 0.5).max(1.0);
                            out.push_str(&format!(
                                r###"  <path d="M {:.2} {:.2} A {:.2} {:.2} 0 0 1 {:.2} {:.2}" stroke="{stroke_col}" stroke-width="1.2" fill="none"/>"###,
                                ox + rx1,
                                oy + ry1,
                                r,
                                r,
                                ox + rx2,
                                oy + ry2
                            ));
                            out.push('\n');
                        }
                        crate::symbols::VectorPrimitive::Polygon { pts } => {
                            let pts_str: Vec<String> = pts
                                .iter()
                                .map(|&(px, py)| {
                                    let (rx, ry) = transform_offset_f32(
                                        px as f32 / 5.0,
                                        py as f32 / 5.0,
                                        orientation,
                                    );
                                    format!("{:.2},{:.2}", ox + rx, oy + ry)
                                })
                                .collect();
                            let fill_col = fill_lamp.unwrap_or("none");
                            out.push_str(&format!(
                                r###"  <polygon points="{}" stroke="{stroke_col}" stroke-width="1.2" fill="{fill_col}"/>"###,
                                pts_str.join(" ")
                            ));
                            out.push('\n');
                        }
                        crate::symbols::VectorPrimitive::Text { x, y, text } => {
                            let (rx, ry) =
                                transform_offset_f32(x as f32 / 5.0, y as f32 / 5.0, orientation);
                            out.push_str(&format!(
                                r###"  <text x="{:.2}" y="{:.2}" font-size="2.8" font-family="sans-serif" fill="{stroke_col}">{}</text>"###,
                                ox + rx,
                                oy + ry,
                                xml_escape(&text)
                            ));
                            out.push('\n');
                        }
                    }
                }
            }
        }

        // Pinos de conexão (bornes)
        for (idx, pin_label, (wx, wy)) in sym.world_pins(item) {
            out.push_str(&format!(
                r###"  <circle cx="{wx}" cy="{wy}" r="0.8" stroke="#dc2626" stroke-width="0.6" fill="none"/>"###
            ));
            out.push('\n');
            if doc.settings.show_pin_refs >= 1 && item.pin_visibility[idx] == 1 {
                let p_def = &sym.pins[idx];
                let (lx, ly) =
                    transform_offset(p_def.label_pos.0, p_def.label_pos.1, item.orientation);
                out.push_str(&format!(
                    r###"  <text x="{:.1}" y="{:.1}" font-size="2.4" font-family="sans-serif" fill="#64748b">{}</text>"###,
                    (item.x1 + lx) as f32 + 0.4,
                    (item.y1 + ly) as f32 - 0.5,
                    xml_escape(&pin_label)
                ));
                out.push('\n');
            }
        }

        // Tag e Valor do componente
        if !item.tag.is_empty() {
            out.push_str(&format!(
                r###"  <text x="{}" y="{}" font-size="3.2" font-weight="bold" font-family="sans-serif" fill="#0f172a">{}</text>"###,
                item.x1 - 6,
                item.y1 - 6,
                xml_escape(&item.tag)
            ));
            out.push('\n');
        }
        if item.show_value == 1 && !item.value.is_empty() {
            out.push_str(&format!(
                r###"  <text x="{}" y="{}" font-size="2.6" font-family="sans-serif" fill="#64748b">{}</text>"###,
                item.x1 - 6,
                item.y1 + 4,
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

/// Exporta uma folha ou seleção para PNG em alta resolução (escala padrão 2.5x).
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
    let scale = 2.0; // Resolução otimizada para PDF nítido e leve
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

    // Dimensões da página em pontos PDF (A4 padrão: ~842 x 595 pt ou proporcional)
    let width_pt = (size.width() as f32 * 2.5).round() as u32;
    let height_pt = (size.height() as f32 * 2.5).round() as u32;

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
