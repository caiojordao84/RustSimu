//! Símbolos v1 no padrão do canvas RustSimu (mesmo pipeline QET/CADe).
//!
//! SVG em espaço de **pixels a 300 DPI** (`GRID_MODULE_PX_300DPI` = 48), com
//! `data-generator="cade2svg"` e bornes `class="qet-terminal"`. O canvas aplica
//! escala 0.125 (48 px → 6 unidades de grade).
//!
//! [`TerminalSimbolo`] usa **unidades de grade** (`GRID_MODULE_UNITS` = 6),
//! iguais a `SymbolDef.pins` / `world_pins`.

use crate::model::{GRID_MODULE_PX_300DPI, GRID_MODULE_UNITS, TERMINAL_RADIUS_PX_300DPI};
use crate::pneu::escala::{nome_pic, por_nome, terminais};
use crate::pneu::pic::{fmt, parse_pic_bin, Comando, Registro, UN_POR_GRADE};
use std::sync::OnceLock;

static PIC: OnceLock<Vec<Registro>> = OnceLock::new();

/// Catálogo `PIC.BIN` embutido.
pub fn catalogo_pic() -> &'static [Registro] {
    PIC.get_or_init(|| {
        let bytes = include_bytes!("PIC.BIN");
        parse_pic_bin(bytes).expect("PIC.BIN embutido deve decodificar")
    })
}

/// FluidSIM unidades → px a 300 DPI.
#[inline]
fn un_para_px(u: f64) -> f64 {
    u * f64::from(GRID_MODULE_PX_300DPI) / UN_POR_GRADE
}

/// px → unidades de grade do canvas (múltiplos de [`GRID_MODULE_UNITS`]).
#[inline]
fn px_para_world(px: f64) -> i32 {
    let g = f64::from(GRID_MODULE_UNITS);
    let world = px * g / f64::from(GRID_MODULE_PX_300DPI);
    (world / g).round() as i32 * GRID_MODULE_UNITS as i32
}

/// Snap de coordenada SVG (px @300 DPI) ao módulo de 48 px da padronização.
#[inline]
fn snap_svg_px(px: f64) -> f64 {
    let g = f64::from(GRID_MODULE_PX_300DPI);
    (px / g).round() * g
}

/// Coords do adaptador (múltiplos de 48 px) → unidades de grade (múltiplos de 6).
#[inline]
pub fn px_adaptador_para_world(px: i32) -> i32 {
    px_para_world(f64::from(px))
}

#[derive(Debug, Clone, PartialEq)]
pub struct TerminalSimbolo {
    pub nome: String,
    /// Offset em unidades de grade, relativo à âncora.
    pub x: i32,
    pub y: i32,
}

/// SVG (cade2svg) + bornes em unidades de grade.
pub fn svg_v1_classe(classe: &str, acionado: bool) -> Option<(String, Vec<TerminalSimbolo>)> {
    let pos = if acionado { 1.0 } else { 0.0 };
    svg_v1_classe_pos(classe, pos)
}

/// SVG cade2svg de um registro PIC (sem haste). Cache key sugerida: nome do pic.
pub fn svg_por_registro(rec: &Registro) -> String {
    svg_pic_cade(rec, None)
}

/// Como [`svg_v1_classe`], com posição contínua do êmbolo (`pos ∈ [0, 1]`).
pub fn svg_v1_classe_pos(classe: &str, pos: f64) -> Option<(String, Vec<TerminalSimbolo>)> {
    let pos = pos.clamp(0.0, 1.0);
    if classe == "PAL1" {
        return Some(svg_pal1(pos));
    }
    if classe.starts_with("PVW")
        && (classe.contains('4') || classe.contains('8') || classe.contains('5'))
    {
        return Some(svg_valve_5_2_solenoid(pos));
    }
    if let Some(svg) = svg_eletrico_p8(classe) {
        return Some(svg);
    }
    let cat = catalogo_pic();
    let rec = por_nome(cat, &nome_pic(classe))?;
    let terms = terminais_para_conceito(classe, rec);
    let haste = if classe == "PAL2" || classe == "PAL1" || classe == "152841" {
        Some(pos)
    } else {
        None
    };
    let svg = svg_pic_cade_com_terminais(rec, haste, &terms);
    Some((svg, terms))
}

/// Símbolos procedurais para instrumentos elétricos P8 / pólos (sem PIC).
fn svg_eletrico_p8(classe: &str) -> Option<(String, Vec<TerminalSimbolo>)> {
    let g = GRID_MODULE_UNITS as i32;
    let (label, pins): (&str, &[&str]) = match classe {
        "pol1" => ("0", &["1"]),
        "pol2" => ("+", &["1"]),
        "Generator" => ("G", &["1", "2"]),
        "VMeter" => ("V", &["1", "2"]),
        "LoopController" => ("LC", &["1", "2", "3"]),
        "LoopControllerPID" => ("PID", &["1", "2", "3"]),
        _ => return None,
    };
    let n = pins.len() as i32;
    let w = (n.max(2) + 1) * g;
    let h = 2 * g;
    let r = TERMINAL_RADIUS_PX_300DPI;
    let mut terms = Vec::new();
    let mut circles = String::new();
    for (i, nome) in pins.iter().enumerate() {
        let dx = (i as i32 - (n - 1) / 2) * g;
        let cx = dx * (GRID_MODULE_PX_300DPI as i32) / g;
        terms.push(TerminalSimbolo {
            nome: (*nome).into(),
            x: dx,
            y: 0,
        });
        circles.push_str(&format!(
            r#"    <circle class="qet-terminal" cx="{cx}" cy="0" r="{r}" fill="black" stroke="none"/>"#
        ));
        circles.push('\n');
    }
    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" data-generator="cade2svg" data-standard="48px" viewBox="{vx} {vy} {vw} {vh}" width="{vw}" height="{vh}">
  <g fill="none" stroke="black" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <rect x="{rx}" y="{ry}" width="{rw}" height="{rh}"/>
    <text x="0" y="4" text-anchor="middle" font-size="14" fill="black" stroke="none">{label}</text>
{circles}  </g>
</svg>
"##,
        vx = -(w as f32) / 2.0 - 4.0,
        vy = -(h as f32) / 2.0 - 4.0,
        vw = w + 8,
        vh = h + 8,
        rx = -w / 2 + g / 4,
        ry = -h / 4,
        rw = w - g / 2,
        rh = h / 2,
        label = label,
        circles = circles,
    );
    Some((svg, terms))
}

fn terminais_para_conceito(classe: &str, rec: &Registro) -> Vec<TerminalSimbolo> {
    let pins = crate::dictionary::global_concept_dictionary()
        .get_by_fluidsim_class(classe)
        .map(|c| c.canonical_pins.clone())
        .unwrap_or_default();
    let g = GRID_MODULE_UNITS as i32;
    let mut terms = terminais(rec);
    terms.sort_by_key(|t| t.numero);
    if terms.is_empty() {
        return pins
            .iter()
            .enumerate()
            .map(|(i, nome)| {
                let n = pins.len() as i32;
                let dx = (i as i32 - (n - 1) / 2) * g;
                TerminalSimbolo {
                    nome: nome.clone(),
                    x: dx,
                    y: 0,
                }
            })
            .collect();
    }
    let mut out = Vec::new();
    for t in &terms {
        let nome = pins
            .get((t.numero as usize).saturating_sub(1))
            .cloned()
            .unwrap_or_else(|| t.numero.to_string());
        if (nome == "A1" || nome == "A2") && classe.starts_with("PVW") {
            continue;
        }
        out.push(TerminalSimbolo {
            nome,
            x: px_para_world(t.x_px()),
            y: px_para_world(t.y_px()),
        });
    }
    if classe.starts_with("PVW") {
        for nome in &pins {
            if (*nome == "A1" || *nome == "A2") && !out.iter().any(|t| t.nome == *nome) {
                let dx = if *nome == "A1" { -g } else { g };
                out.push(TerminalSimbolo {
                    nome: nome.clone(),
                    x: dx,
                    y: -2 * g,
                });
            }
        }
    }
    out
}

/// Renderiza o PIC relativo à âncora, em px, no formato `cade2svg`.
///
/// Bornes caem em múltiplos de 48 px. Extremidades de linhas a ≤½ módulo de um
/// borne são atraídas para ele — assim a tinta dos ports fica no mesmo ponto
/// do `qet-terminal` (grade RustSimu).
pub fn svg_pic_cade(rec: &Registro, haste_pos: Option<f64>) -> String {
    svg_pic_cade_com_terminais(rec, haste_pos, &[])
}

pub fn svg_pic_cade_com_terminais(
    rec: &Registro,
    haste_pos: Option<f64>,
    extra_terms: &[TerminalSimbolo],
) -> String {
    let ax = f64::from(rec.x0);
    let ay = f64::from(rec.y0);
    let to_px = |ux: f64, uy: f64| -> (f64, f64) { (un_para_px(ux - ax), un_para_px(uy - ay)) };

    #[derive(Clone)]
    enum Prim {
        Line {
            x1: f64,
            y1: f64,
            x2: f64,
            y2: f64,
            sw: Option<f64>,
        },
        Circle {
            cx: f64,
            cy: f64,
            r: f64,
            fill: bool,
        },
        Rect {
            x: f64,
            y: f64,
            w: f64,
            h: f64,
        },
        Path(String),
    }

    let mut prims: Vec<Prim> = Vec::new();
    for Comando { op, ps } in &rec.comandos {
        let p = |i: usize| f64::from(ps[i]);
        match *op {
            1 | 7 => {
                let (x1, y1) = to_px(ax + p(4), ay + p(5));
                let (x2, y2) = to_px(ax + p(6), ay + p(7));
                // No manômetro (152841.PIC), a haste diagonal longa do ponteiro (~86 px)
                // é ocultada durante a simulação (haste_pos > 0) para exibir o visor digital
                // de pressão no centro do círculo, preservando apenas a ponta da seta e o bico inferior.
                if rec.nome == "152841.PIC"
                    && haste_pos.is_some_and(|hp| hp > 0.0)
                    && (x2 - x1).hypot(y2 - y1) > 30.0
                {
                    continue;
                }
                prims.push(Prim::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    sw: None,
                });
            }
            3 | 5 => {
                let (cx, cy) = to_px(ax + p(4), ay + p(5));
                prims.push(Prim::Circle {
                    cx,
                    cy,
                    r: un_para_px(p(2)).max(0.5),
                    fill: *op == 5,
                });
            }
            8 => {
                let (x1, y1) = to_px(ax + f64::from(ps[4]), ay + f64::from(ps[5]));
                let (x2, y2) = to_px(ax + f64::from(ps[6]), ay + f64::from(ps[7]));
                prims.push(Prim::Rect {
                    x: x1.min(x2),
                    y: y1.min(y2),
                    w: (x2 - x1).abs(),
                    h: (y2 - y1).abs(),
                });
            }
            4 => {
                let (cx, cy) = to_px(ax + p(4), ay + p(5));
                let r = un_para_px(p(2)).max(0.5);
                let a0 = (ps[0] as f64 / 1024.0).to_radians();
                let a1 = (ps[1] as f64 / 1024.0).to_radians();
                let x0 = cx + r * a0.cos();
                let y0 = cy + r * a0.sin();
                let x1 = cx + r * a1.cos();
                let y1 = cy + r * a1.sin();
                let delta = ((ps[1] - ps[0]) as f64 / 1024.0).rem_euclid(360.0);
                let large = i32::from(delta > 180.0);
                prims.push(Prim::Path(format!(
                    r#"M {} {} A {} {} 0 {} 0 {} {}"#,
                    fmt(x0),
                    fmt(y0),
                    fmt(r),
                    fmt(r),
                    large,
                    fmt(x1),
                    fmt(y1)
                )));
            }
            6 => {
                let (x0, y0) = to_px(ax + p(4), ay + p(5));
                let (x1, y1) = to_px(ax + p(6), ay + p(7));
                let (x2, y2) = to_px(ax + p(2), ay + p(3));
                let (x3, y3) = to_px(ax + p(0), ay + p(1));
                prims.push(Prim::Path(format!(
                    r#"M {} {} C {} {} {} {} {} {}"#,
                    fmt(x0),
                    fmt(y0),
                    fmt(x1),
                    fmt(y1),
                    fmt(x2),
                    fmt(y2),
                    fmt(x3),
                    fmt(y3)
                )));
            }
            _ => {}
        }
    }

    if let Some(pos) = haste_pos {
        if rec.nome != "152841.PIC" {
            let frac = pos.clamp(0.0, 1.0);
            if frac > 0.02 {
                let x2 = un_para_px(UN_POR_GRADE * 2.5 * frac);
                prims.push(Prim::Line {
                    x1: 0.0,
                    y1: 0.0,
                    x2,
                    y2: 0.0,
                    sw: Some(3.0),
                });
            }
        }
    }

    // Bornes na grade 48 px.
    let mut terms: Vec<(f64, f64)> = terminais(rec)
        .iter()
        .map(|t| {
            (
                snap_svg_px(un_para_px(f64::from(t.x_un))),
                snap_svg_px(un_para_px(f64::from(t.y_un))),
            )
        })
        .collect();

    for et in extra_terms {
        let epx = snap_svg_px(
            et.x as f64 * (f64::from(GRID_MODULE_PX_300DPI) / f64::from(GRID_MODULE_UNITS)),
        );
        let epy = snap_svg_px(
            et.y as f64 * (f64::from(GRID_MODULE_PX_300DPI) / f64::from(GRID_MODULE_UNITS)),
        );
        if !terms
            .iter()
            .any(|(tx, ty)| (*tx - epx).abs() < 1.0 && (*ty - epy).abs() < 1.0)
        {
            terms.push((epx, epy));
        }
    }

    // Atrai extremidades de linha para o borne mais próximo (≤ 24 px = ½ módulo),
    // sem colapsar linhas curtas (como o pescoço de 14.4 px do manômetro 152841) num único ponto.
    let lim = f64::from(GRID_MODULE_PX_300DPI) * 0.5;
    let atrai = |x: &mut f64, y: &mut f64| {
        let mut best = None;
        for &(tx, ty) in &terms {
            let d = (*x - tx).hypot(*y - ty);
            if d <= lim && best.is_none_or(|(bd, _, _)| d < bd) {
                best = Some((d, tx, ty));
            }
        }
        if let Some((_, tx, ty)) = best {
            *x = tx;
            *y = ty;
        }
    };
    for prim in &mut prims {
        if let Prim::Line { x1, y1, x2, y2, .. } = prim {
            let mut nx1 = *x1;
            let mut ny1 = *y1;
            let mut nx2 = *x2;
            let mut ny2 = *y2;
            atrai(&mut nx1, &mut ny1);
            atrai(&mut nx2, &mut ny2);
            if (nx1 - nx2).hypot(ny1 - ny2) > 0.5 {
                *x1 = nx1;
                *y1 = ny1;
                *x2 = nx2;
                *y2 = ny2;
            } else {
                let d1 = (*x1 - nx1).hypot(*y1 - ny1);
                let d2 = (*x2 - nx2).hypot(*y2 - ny2);
                if d1 <= d2 {
                    *x1 = nx1;
                    *y1 = ny1;
                } else {
                    *x2 = nx2;
                    *y2 = ny2;
                }
            }
        }
    }

    let mut min_x = 0.0f64;
    let mut min_y = 0.0f64;
    let mut max_x = 0.0f64;
    let mut max_y = 0.0f64;
    let mut expand = |x: f64, y: f64| {
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    };
    expand(0.0, 0.0);

    let mut partes: Vec<String> = Vec::new();
    for prim in &prims {
        match prim {
            Prim::Line { x1, y1, x2, y2, sw } => {
                expand(*x1, *y1);
                expand(*x2, *y2);
                if let Some(w) = sw {
                    partes.push(format!(
                        r#"<line x1="{}" y1="{}" x2="{}" y2="{}" stroke-width="{}"/>"#,
                        fmt(*x1),
                        fmt(*y1),
                        fmt(*x2),
                        fmt(*y2),
                        fmt(*w)
                    ));
                } else {
                    partes.push(format!(
                        r#"<line x1="{}" y1="{}" x2="{}" y2="{}"/>"#,
                        fmt(*x1),
                        fmt(*y1),
                        fmt(*x2),
                        fmt(*y2)
                    ));
                }
            }
            Prim::Circle { cx, cy, r, fill } => {
                expand(*cx - *r, *cy - *r);
                expand(*cx + *r, *cy + *r);
                if *fill {
                    partes.push(format!(
                        r#"<circle cx="{}" cy="{}" r="{}" fill="black"/>"#,
                        fmt(*cx),
                        fmt(*cy),
                        fmt(*r)
                    ));
                } else {
                    partes.push(format!(
                        r#"<circle cx="{}" cy="{}" r="{}"/>"#,
                        fmt(*cx),
                        fmt(*cy),
                        fmt(*r)
                    ));
                }
            }
            Prim::Rect { x, y, w, h } => {
                expand(*x, *y);
                expand(*x + *w, *y + *h);
                partes.push(format!(
                    r#"<rect x="{}" y="{}" width="{}" height="{}"/>"#,
                    fmt(*x),
                    fmt(*y),
                    fmt(*w),
                    fmt(*h)
                ));
            }
            Prim::Path(d) => {
                partes.push(format!(r#"<path d="{d}"/>"#));
            }
        }
    }

    let r_term = f64::from(TERMINAL_RADIUS_PX_300DPI);
    for &(cx, cy) in &terms {
        expand(cx - r_term, cy - r_term);
        expand(cx + r_term, cy + r_term);
        partes.push(format!(
            r#"<circle class="qet-terminal" cx="{}" cy="{}" r="{}" fill="black" stroke="none"/>"#,
            fmt(cx),
            fmt(cy),
            fmt(r_term)
        ));
    }

    let pad = 4.0;
    min_x -= pad;
    min_y -= pad;
    max_x += pad;
    max_y += pad;
    let vw = (max_x - min_x).max(8.0);
    let vh = (max_y - min_y).max(8.0);
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" data-generator="cade2svg" data-standard="48px" viewBox="{vx} {vy} {vw} {vh}" width="{vw}" height="{vh}">
  <g fill="none" stroke="#000" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
{corpo}
  </g>
</svg>
"##,
        vx = fmt(min_x),
        vy = fmt(min_y),
        vw = fmt(vw),
        vh = fmt(vh),
        corpo = partes.join("\n    "),
    )
}

/// Cilindro simples ação ISO 1219 em px/`cade2svg`. Âncora = borne 1 (0,0).
/// `pos ∈ [0, 1]` interpola a posição do pistão e da haste entre recolhido (0.0) e avançado (1.0).
/// Sem setas diagonais (amortecimento não aplicável ao símbolo padrão).
pub fn svg_pal1(pos: f64) -> (String, Vec<TerminalSimbolo>) {
    let t = pos.clamp(0.0, 1.0);
    let r = TERMINAL_RADIUS_PX_300DPI;
    let px = 16.0 + t * 148.0;
    let rod_end = 216.0 + t * 36.0;

    let dist = 180.0 - px;
    let spring_path = if dist > 24.0 {
        let step = dist / 8.0;
        let mut d = format!("M {} -48", fmt(px));
        for i in 0..8 {
            let x_mid = px + (i as f64 + 0.5) * step;
            let y_peak = if i % 2 == 0 { -62.0 } else { -34.0 };
            d.push_str(&format!(" L {} {}", fmt(x_mid), fmt(y_peak)));
        }
        d.push_str(" L 180 -48");
        format!(r#"<path d="{d}" stroke-width="1.5"/>"#)
    } else {
        let mid = px + (180.0 - px) * 0.5;
        format!(
            r#"<line x1="{}" y1="-64" x2="{}" y2="-32" stroke-width="1.5"/><line x1="{}" y1="-32" x2="180" y2="-64" stroke-width="1.5"/>"#,
            fmt(px),
            fmt(mid),
            fmt(mid),
        )
    };

    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" data-generator="cade2svg" data-standard="48px" viewBox="-20 -84 290 96" width="290" height="96">
  <g fill="none" stroke="#000" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <line x1="-12" y1="-24" x2="180" y2="-24"/>
    <line x1="-12" y1="-72" x2="180" y2="-72"/>
    <line x1="-12" y1="-72" x2="-12" y2="-24"/>
    <line x1="180" y1="-72" x2="180" y2="-56"/>
    <line x1="180" y1="-40" x2="180" y2="-24"/>
    <rect x="{piston_x}" y="-70" width="8" height="44" fill="#000" stroke="#000"/>
    <line x1="{px}" y1="-48" x2="{rod_end}" y2="-48" stroke-width="3"/>
    {spring_path}
    <line x1="0" y1="0" x2="0" y2="-24"/>
    <circle class="qet-terminal" cx="0" cy="0" r="{r}" fill="#000" stroke="none"/>
  </g>
</svg>
"##,
        piston_x = fmt(px - 8.0),
        px = fmt(px),
        rod_end = fmt(rod_end),
        spring_path = spring_path,
        r = r,
    );
    let terms = vec![TerminalSimbolo {
        nome: "1".into(),
        x: 0,
        y: 0,
    }];
    (svg, terms)
}

/// Válvula 5/2 direcional com solenóide e retorno por mola ISO 1219 em px/`cade2svg`.
/// Portas 4 e 2 no topo (y=0); portas 5, 1, 3 na base (y=192).
/// Silenciadores integrados nos escapes 5 e 3.
pub fn svg_valve_5_2_solenoid(pos: f64) -> (String, Vec<TerminalSimbolo>) {
    let r = TERMINAL_RADIUS_PX_300DPI;
    let g = GRID_MODULE_UNITS as i32;
    let acionada = pos > 0.5;

    let vias_internas = if !acionada {
        r##"    <!-- Estado Repouso: caixa esquerda (1->4, 2->3), caixa direita ativa (1->2, 4->5) -->
    <line x1="-48" y1="144" x2="-72" y2="48"/>
    <polygon points="-72,48 -66,58 -76,55" fill="#000" stroke="#000"/>
    <line x1="-24" y1="48" x2="-24" y2="144"/>
    <polygon points="-24,144 -28,134 -20,134" fill="#000" stroke="#000"/>
    <line x1="-72" y1="144" x2="-72" y2="132"/>
    <line x1="-78" y1="132" x2="-66" y2="132"/>
    <line x1="0" y1="144" x2="48" y2="48"/>
    <polygon points="48,48 40,56 48,60" fill="#000" stroke="#000"/>
    <line x1="24" y1="48" x2="24" y2="144"/>
    <polygon points="24,144 20,134 28,134" fill="#000" stroke="#000"/>
    <line x1="72" y1="144" x2="72" y2="132"/>
    <line x1="66" y1="132" x2="78" y2="132"/>"##
    } else {
        r##"    <!-- Estado Acionado (1Y): comuta vias ativas para (1->4, 2->3) -->
    <line x1="-72" y1="48" x2="-72" y2="144"/>
    <polygon points="-72,144 -76,134 -68,134" fill="#000" stroke="#000"/>
    <line x1="-48" y1="144" x2="-24" y2="48"/>
    <polygon points="-24,48 -32,56 -24,60" fill="#000" stroke="#000"/>
    <line x1="0" y1="144" x2="-48" y2="48"/>
    <polygon points="-48,48 -48,60 -40,56" fill="#000" stroke="#000"/>
    <line x1="48" y1="48" x2="48" y2="144"/>
    <polygon points="48,144 44,134 52,134" fill="#000" stroke="#000"/>
    <line x1="-48" y1="144" x2="-48" y2="132"/>
    <line x1="-54" y1="132" x2="-42" y2="132"/>"##
    };

    let svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" data-generator="cade2svg" data-standard="48px" viewBox="-160 -12 300 232" width="300" height="232">
  <g fill="none" stroke="#000" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
    <rect x="-96" y="48" width="192" height="96"/>
    <line x1="0" y1="48" x2="0" y2="144"/>
{vias_internas}
    <!-- Condutores de topo (4 e 2) -->"##,
        vias_internas = vias_internas,
    );
    let svg_rest = format!(
        r##"
    <line x1="-48" y1="0" x2="-48" y2="48"/>
    <line x1="48" y1="0" x2="48" y2="48"/>
    <!-- Condutores de base (5, 1, 3) -->
    <line x1="-48" y1="144" x2="-48" y2="192"/>
    <line x1="0" y1="144" x2="0" y2="192"/>
    <line x1="48" y1="144" x2="48" y2="192"/>
    <!-- Silenciadores nos escapes 5 e 3 -->
    <polygon points="-56,192 -40,192 -48,206" fill="none" stroke-width="1.5"/>
    <polygon points="40,192 56,192 48,206" fill="none" stroke-width="1.5"/>
    <!-- Solenóide esquerdo (1Y) -->
    <rect x="-132" y="60" width="36" height="72"/>
    <line x1="-132" y1="132" x2="-96" y2="60"/>
    <!-- Acionamento manual auxiliar -->
    <line x1="-146" y1="96" x2="-132" y2="96"/>
    <line x1="-146" y1="84" x2="-146" y2="108"/>
    <!-- Mola direita -->
    <path d="M 96 96 L 101 84 L 107 108 L 113 84 L 119 108 L 123 84 L 126 96"/>
    <!-- Bornes qet-terminal -->
    <circle class="qet-terminal" cx="-48" cy="0" r="{r}" fill="#000" stroke="none"/>
    <circle class="qet-terminal" cx="48" cy="0" r="{r}" fill="#000" stroke="none"/>
    <circle class="qet-terminal" cx="-48" cy="192" r="{r}" fill="#000" stroke="none"/>
    <circle class="qet-terminal" cx="0" cy="192" r="{r}" fill="#000" stroke="none"/>
    <circle class="qet-terminal" cx="48" cy="192" r="{r}" fill="#000" stroke="none"/>
  </g>
</svg>
"##,
        r = r,
    );
    let svg = format!("{svg}{svg_rest}");

    let terms = vec![
        TerminalSimbolo {
            nome: "4".into(),
            x: -g,
            y: 0,
        },
        TerminalSimbolo {
            nome: "2".into(),
            x: g,
            y: 0,
        },
        TerminalSimbolo {
            nome: "5".into(),
            x: -g,
            y: 4 * g,
        },
        TerminalSimbolo {
            nome: "1".into(),
            x: 0,
            y: 4 * g,
        },
        TerminalSimbolo {
            nome: "3".into(),
            x: g,
            y: 4 * g,
        },
    ];
    (svg, terms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::svg_symbol::parse_svg_content;

    #[test]
    fn pic_embutido_e_pal1_padrao_canvas() {
        crate::pneu::palette::force_reregister_svgs();
        assert!(catalogo_pic().len() >= 300);
        let (svg, terms) = svg_pal1(0.0);
        assert!(svg.contains("data-generator=\"cade2svg\""));
        assert!(svg.contains("qet-terminal"));
        assert_eq!(terms.len(), 1);
        let parsed = parse_svg_content(&svg).expect("parse PAL1");
        assert!(parsed.is_cade_svg);
        assert!((parsed.normalized_scale - 0.125).abs() < 0.01);

        let (svg2, t2) = svg_v1_classe("PVWM8", false).expect("PVWM8");
        assert!(svg2.contains("data-generator=\"cade2svg\""));
        assert!(svg2.contains("qet-terminal"));
        assert!(!t2.is_empty());
        let g = GRID_MODULE_UNITS as i32;
        assert!(t2.iter().all(|t| t.x % g == 0 && t.y % g == 0));
        let parsed2 = parse_svg_content(&svg2).expect("parse PVWM8");
        assert!(parsed2.is_cade_svg);
        assert!(parsed2.shapes.len() > 2);
        // Após escala, a caixa deve caber em dezenas de unidades (não milhares).
        let s = parsed2.scale_factor();
        let w = parsed2.view_box.2 * s;
        let h = parsed2.view_box.3 * s;
        assert!(w > 1.0 && w < 80.0, "largura mundo={w}");
        assert!(h > 1.0 && h < 80.0, "altura mundo={h}");
    }

    #[test]
    fn conversao_adaptador_px_para_world() {
        assert_eq!(px_adaptador_para_world(0), 0);
        assert_eq!(px_adaptador_para_world(48), 6);
        assert_eq!(px_adaptador_para_world(96), 12);
        assert_eq!(px_adaptador_para_world(-48), -6);
    }

    #[test]
    fn todo_record_com_ink_box_gera_svg_parseavel() {
        let cat = catalogo_pic();
        assert_eq!(cat.len(), 347);
        let mut ok = 0usize;
        for rec in cat {
            if rec.w == 0 || rec.h == 0 {
                continue;
            }
            let svg = svg_por_registro(rec);
            assert!(
                svg.contains("data-generator=\"cade2svg\""),
                "pic {} sem cade2svg",
                rec.nome
            );
            let parsed = parse_svg_content(&svg).unwrap_or_else(|e| panic!("{}: {e}", rec.nome));
            assert!(parsed.is_cade_svg, "{}", rec.nome);
            ok += 1;
        }
        assert!(ok > 300, "esperava >300 pics com tinta, got {ok}");
    }
}
