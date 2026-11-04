use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Modo de preenchimento interno de formas vetoriais QET/SVG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SvgFill {
    None,
    /// Preenchimento sólido na cor do traço (`filling:black`, `fill="black"`): escovas de motor, placas de bateria, junções.
    Solid,
    /// Preenchimento opaco branco/fundo (`filling:white`, `filling:HTMLWhiteIvory`, `fill="white"`): rotor de motor, fundo de diagramas.
    White,
    /// Preenchimento colorido translúcido suave (`filling:lightgray`, `filling:blue`, etc.).
    Tint,
}

/// Primitivas vetoriais geométricas extraídas de um ficheiro SVG do QElectroTech.
#[derive(Debug, Clone)]
pub enum SvgShape {
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        dashed: bool,
    },
    Polyline {
        points: Vec<(f32, f32)>,
    },
    Polygon {
        points: Vec<(f32, f32)>,
        fill: SvgFill,
    },
    Circle {
        cx: f32,
        cy: f32,
        r: f32,
        is_terminal: bool,
        fill: SvgFill,
    },
    Ellipse {
        cx: f32,
        cy: f32,
        rx: f32,
        ry: f32,
        fill: SvgFill,
    },
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        dashed: bool,
        fill: SvgFill,
    },
    Text {
        x: f32,
        y: f32,
        text: String,
        size: f32,
        rotation: f32,
    },
}

/// Símbolo vetorial SVG pronto para ser renderizado no Canvas do CADe_SIMU.
#[derive(Debug, Clone)]
pub struct ParsedSvgSymbol {
    pub view_box: (f32, f32, f32, f32),
    pub shapes: Vec<SvgShape>,
    pub term_top_y: f32,
    pub term_bottom_y: f32,
    pub min_term_x: f32,
    pub max_term_x: f32,
    pub is_cade_svg: bool,
    pub is_qet_native: bool,
}

impl ParsedSvgSymbol {
    /// Mapeia uma coordenada local do SVG `(sx, sy)` para o sistema de coordenadas de grade do CADe_SIMU.
    ///
    /// Se for um símbolo nativo QET, mantém a escala 1:1 original em pixels.
    /// Se o SVG foi gerado a partir do CADe, a escala é exatamente 1:5 (divisão por 5.0).
    /// Se for importado de bibliotecas externas (como QET adaptado ao CADe), utiliza o alinhamento exato de bornes/terminais.
    pub fn map_point(&self, sx: f32, sy: f32, target_span: f32) -> (f32, f32) {
        if self.is_qet_native {
            (sx, sy)
        } else if self.is_cade_svg {
            (sx / 5.0, sy / 5.0)
        } else {
            let svg_span_y = (self.term_bottom_y - self.term_top_y).abs().max(1.0);
            let scale_y = target_span / svg_span_y;
            let span_x = (self.max_term_x - self.min_term_x).abs();
            // Para elementos multipolares QET (distância entre polos consecutivos = 20.0),
            // a escala x ideal é 6.0 / 20.0 = 0.3 para cravar em passos de 6 unidades na grade (0, 6, 12, 18).
            let local_x = if span_x >= 15.0 {
                (sx - self.min_term_x) * 0.30
            } else {
                (sx - self.min_term_x) * scale_y
            };
            let local_y = (sy - self.term_top_y) * scale_y;
            (local_x, local_y)
        }
    }
}

/// Resolve o caminho de um SVG relativo ou absoluto.
pub fn resolve_svg_path(rel_path: &str) -> PathBuf {
    let p = Path::new(rel_path);
    if p.exists() {
        return p.to_path_buf();
    }
    // Procura na mesma pasta do executável (essencial para pacote portátil no Windows)
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            let p_exe = exe_dir.join(rel_path);
            if p_exe.exists() {
                return p_exe;
            }
        }
    }
    let p_sub = Path::new("cade-simu-rs").join(rel_path);
    if p_sub.exists() {
        return p_sub;
    }
    let p_manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join(rel_path);
    if p_manifest.exists() {
        return p_manifest;
    }
    p.to_path_buf()
}

static PARSED_CACHE: OnceLock<Mutex<HashMap<String, ParsedSvgSymbol>>> = OnceLock::new();

/// Registra explicitamente um símbolo SVG/QET no cache em memória.
pub fn register_parsed_svg(key: &str, symbol: ParsedSvgSymbol) {
    let cache = PARSED_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut guard) = cache.lock() {
        guard.insert(key.to_string(), symbol);
    }
}

/// Obtém um símbolo SVG compilado do cache ou faz o parse do disco.
pub fn get_parsed_svg(path_str: &str) -> Option<ParsedSvgSymbol> {
    let cache = PARSED_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    {
        if let Ok(guard) = cache.lock() {
            // Verifica primeiro chave direta (ex: URI QET embed://...)
            if let Some(parsed) = guard.get(path_str) {
                return Some(parsed.clone());
            }
        }
    }

    let path = resolve_svg_path(path_str);
    let key = path.to_string_lossy().to_string();

    {
        if let Ok(guard) = cache.lock() {
            if let Some(parsed) = guard.get(&key) {
                return Some(parsed.clone());
            }
        }
    }

    let content = std::fs::read_to_string(&path).ok()?;
    let parsed = parse_svg_content(&content).ok()?;

    if let Ok(mut guard) = cache.lock() {
        guard.insert(key, parsed.clone());
    }

    Some(parsed)
}

/// Faz o parse do XML de um SVG gerado pelo conversor QET.
pub fn parse_svg_content(xml: &str) -> Result<ParsedSvgSymbol, String> {
    let doc = roxmltree::Document::parse(xml).map_err(|e| e.to_string())?;
    let root = doc.root_element();

    let view_box = if let Some(vb) = root.attribute("viewBox") {
        let parts: Vec<f32> = vb
            .split_whitespace()
            .filter_map(|s| s.parse::<f32>().ok())
            .collect();
        if parts.len() == 4 {
            (parts[0], parts[1], parts[2], parts[3])
        } else {
            (0.0, 0.0, 100.0, 100.0)
        }
    } else {
        (0.0, 0.0, 100.0, 100.0)
    };

    let is_cade_svg = root
        .attribute("data-generator")
        .map(|g| g == "cade2svg")
        .unwrap_or(false);

    let is_qet_native = root
        .attribute("data-qet-native")
        .map(|g| g == "true")
        .unwrap_or(false);

    let mut shapes = Vec::new();
    let mut terminal_ys = Vec::new();
    let mut terminal_xs = Vec::new();

    // Itera pelos nós do SVG
    for node in root.descendants() {
        if !node.is_element() {
            continue;
        }

        let fill = parse_svg_fill_attr(node.attribute("fill"));

        match node.tag_name().name() {
            "line" => {
                let x1 = node.attribute("x1").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let y1 = node.attribute("y1").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let x2 = node.attribute("x2").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let y2 = node.attribute("y2").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let dashed = node.attribute("stroke-dasharray").is_some();
                shapes.push(SvgShape::Line {
                    x1,
                    y1,
                    x2,
                    y2,
                    dashed,
                });
            }
            "path" => {
                if let Some(arc_attr) = node.attribute("data-arc") {
                    let nums: Vec<f32> = arc_attr
                        .split(',')
                        .filter_map(|s| s.trim().parse().ok())
                        .collect();
                    if nums.len() == 6 {
                        let pts = qet_arc_to_polyline(nums[0], nums[1], nums[2], nums[3], nums[4], nums[5]);
                        if pts.len() >= 2 {
                            shapes.push(SvgShape::Polyline { points: pts });
                        }
                    }
                } else if let Some(d_attr) = node.attribute("d") {
                    let pts = parse_svg_arc_d(d_attr);
                    if pts.len() >= 2 {
                        shapes.push(SvgShape::Polyline { points: pts });
                    }
                }
            }
            "polyline" => {
                if let Some(pts_str) = node.attribute("points") {
                    let pts = parse_points(pts_str);
                    if !pts.is_empty() {
                        shapes.push(SvgShape::Polyline { points: pts });
                    }
                }
            }
            "polygon" => {
                if let Some(pts_str) = node.attribute("points") {
                    let pts = parse_points(pts_str);
                    if !pts.is_empty() {
                        shapes.push(SvgShape::Polygon { points: pts, fill });
                    }
                }
            }
            "circle" => {
                let cx = node.attribute("cx").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let cy = node.attribute("cy").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let r = node.attribute("r").and_then(|v| v.parse().ok()).unwrap_or(1.5);
                let is_terminal = node
                    .attribute("class")
                    .map(|c| c.contains("qet-terminal"))
                    .unwrap_or(false);

                if is_terminal {
                    terminal_ys.push(cy);
                    terminal_xs.push(cx);
                }
                shapes.push(SvgShape::Circle {
                    cx,
                    cy,
                    r,
                    is_terminal,
                    fill,
                });
            }
            "ellipse" => {
                let cx = node.attribute("cx").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let cy = node.attribute("cy").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let rx = node.attribute("rx").and_then(|v| v.parse().ok()).unwrap_or(5.0);
                let ry = node.attribute("ry").and_then(|v| v.parse().ok()).unwrap_or(5.0);
                shapes.push(SvgShape::Ellipse {
                    cx,
                    cy,
                    rx,
                    ry,
                    fill,
                });
            }
            "rect" => {
                let x = node.attribute("x").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let y = node.attribute("y").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let w = node.attribute("width").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let h = node.attribute("height").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let dashed = node.attribute("stroke-dasharray").is_some();
                shapes.push(SvgShape::Rect {
                    x,
                    y,
                    w,
                    h,
                    dashed,
                    fill,
                });
            }
            "text" => {
                let x = node.attribute("x").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let y = node.attribute("y").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                let size = node.attribute("font-size").and_then(|v| v.parse().ok()).unwrap_or(10.0);
                let rotation = node.attribute("transform").and_then(|t| {
                    if let Some(idx) = t.find("rotate(") {
                        let rest = &t[idx + 7..];
                        let num_str = rest.split([' ', ')', ',']).next()?;
                        num_str.parse::<f32>().ok()
                    } else {
                        None
                    }
                }).unwrap_or(0.0);
                let text = node.text().unwrap_or("").trim().to_string();
                if !text.is_empty() {
                    shapes.push(SvgShape::Text { x, y, text, size, rotation });
                }
            }
            _ => {}
        }
    }

    let (term_top_y, term_bottom_y) = if terminal_ys.len() >= 2 {
        terminal_ys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        (terminal_ys[0], *terminal_ys.last().unwrap())
    } else {
        (view_box.1 + 9.0, view_box.1 + view_box.3 - 9.0)
    };

    let min_term_x = if !terminal_xs.is_empty() {
        terminal_xs.iter().cloned().fold(f32::INFINITY, f32::min)
    } else {
        0.0
    };

    let max_term_x = if !terminal_xs.is_empty() {
        terminal_xs.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
    } else {
        0.0
    };

    shapes.sort_by_key(|s| match s {
        SvgShape::Rect { dashed: true, .. } => 0,
        SvgShape::Circle { fill: SvgFill::White, .. }
        | SvgShape::Ellipse { fill: SvgFill::White, .. }
        | SvgShape::Rect { fill: SvgFill::White, .. }
        | SvgShape::Polygon { fill: SvgFill::White, .. } => 1,
        SvgShape::Line { .. } | SvgShape::Polyline { .. } => 2,
        SvgShape::Circle { fill: SvgFill::Solid, .. }
        | SvgShape::Ellipse { fill: SvgFill::Solid, .. }
        | SvgShape::Rect { fill: SvgFill::Solid, .. }
        | SvgShape::Polygon { fill: SvgFill::Solid, .. } => 4,
        SvgShape::Text { .. } => 5,
        _ => 3,
    });

    Ok(ParsedSvgSymbol {
        view_box,
        shapes,
        term_top_y,
        term_bottom_y,
        min_term_x,
        max_term_x,
        is_cade_svg,
        is_qet_native,
    })
}

pub fn parse_svg_fill_attr(fill_opt: Option<&str>) -> SvgFill {
    let Some(f) = fill_opt else {
        return SvgFill::None;
    };
    let f = f.trim().to_ascii_lowercase();
    if f.is_empty() || f == "none" {
        SvgFill::None
    } else if f == "black" || f == "#000000" || f == "#000" {
        SvgFill::Solid
    } else if f == "white" || f == "#ffffff" || f == "#fff" || f.contains("white") || f.contains("ivory") {
        SvgFill::White
    } else {
        SvgFill::Tint
    }
}

pub fn qet_arc_to_polyline(
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    start_deg: f32,
    angle_deg: f32,
) -> Vec<(f32, f32)> {
    let rx = w / 2.0;
    let ry = h / 2.0;
    let cx = x + rx;
    let cy = y + ry;

    let steps = 16;
    let mut points = Vec::with_capacity(steps + 1);
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let deg = start_deg + angle_deg * t;
        let rad = -deg * std::f32::consts::PI / 180.0;
        points.push((cx + rx * rad.cos(), cy + ry * rad.sin()));
    }
    points
}

/// Converte um comando SVG `<path d="M x1 y1 A rx ry rot large_arc sweep x2 y2">` em polilinha.
fn parse_svg_arc_d(d: &str) -> Vec<(f32, f32)> {
    let tokens: Vec<f32> = d
        .replace(['M', 'A', 'L', ','], " ")
        .split_whitespace()
        .filter_map(|s| s.parse::<f32>().ok())
        .collect();
    // Formato "M x1 y1 A rx ry 0 large_arc sweep x2 y2" -> 9 números
    if tokens.len() >= 9 {
        let (x1, y1) = (tokens[0], tokens[1]);
        let (mut rx, mut ry) = (tokens[2].abs(), tokens[3].abs());
        let large_arc = tokens[5] > 0.5;
        let sweep = tokens[6] > 0.5;
        let (x2, y2) = (tokens[7], tokens[8]);

        if rx < 0.01 || ry < 0.01 || ((x1 - x2).abs() < 0.01 && (y1 - y2).abs() < 0.01) {
            return vec![(x1, y1), (x2, y2)];
        }

        // Parametrização padrão W3C SVG Endpoint-to-Center Arc
        let dx2 = (x1 - x2) / 2.0;
        let dy2 = (y1 - y2) / 2.0;
        let lambda = (dx2 * dx2) / (rx * rx) + (dy2 * dy2) / (ry * ry);
        if lambda > 1.0 {
            let s = lambda.sqrt();
            rx *= s;
            ry *= s;
        }
        let rx2 = rx * rx;
        let ry2 = ry * ry;
        let num = (rx2 * ry2 - rx2 * dy2 * dy2 - ry2 * dx2 * dx2).max(0.0);
        let den = (rx2 * dy2 * dy2 + ry2 * dx2 * dx2).max(1e-6);
        let sign = if large_arc == sweep { -1.0 } else { 1.0 };
        let coef = sign * (num / den).sqrt();
        let cxp = coef * (rx * dy2 / ry);
        let cyp = coef * (-ry * dx2 / rx);
        let cx = cxp + (x1 + x2) / 2.0;
        let cy = cyp + (y1 + y2) / 2.0;

        let start_ang = ((y1 - cy) / ry).atan2((x1 - cx) / rx);
        let end_ang = ((y2 - cy) / ry).atan2((x2 - cx) / rx);
        let mut d_ang = end_ang - start_ang;
        let two_pi = 2.0 * std::f32::consts::PI;
        if sweep && d_ang < 0.0 {
            d_ang += two_pi;
        } else if !sweep && d_ang > 0.0 {
            d_ang -= two_pi;
        }
        if large_arc && d_ang.abs() < std::f32::consts::PI {
            if d_ang >= 0.0 {
                d_ang -= two_pi;
            } else {
                d_ang += two_pi;
            }
        } else if !large_arc && d_ang.abs() > std::f32::consts::PI {
            if d_ang >= 0.0 {
                d_ang -= two_pi;
            } else {
                d_ang += two_pi;
            }
        }

        let steps = 16;
        let mut pts = Vec::with_capacity(steps + 1);
        for i in 0..=steps {
            let a = start_ang + d_ang * (i as f32 / steps as f32);
            pts.push((cx + rx * a.cos(), cy + ry * a.sin()));
        }
        return pts;
    }
    Vec::new()
}

fn parse_points(s: &str) -> Vec<(f32, f32)> {
    let mut out = Vec::new();
    for token in s.split_whitespace() {
        let coords: Vec<&str> = token.split(',').collect();
        if coords.len() == 2 {
            if let (Ok(x), Ok(y)) = (coords[0].parse::<f32>(), coords[1].parse::<f32>()) {
                out.push((x, y));
            }
        }
    }
    out
}
