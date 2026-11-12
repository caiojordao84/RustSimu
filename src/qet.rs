use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::model::{transform_offset_f32, SchematicDocument, SchematicItem};
use crate::svg_symbol::{register_parsed_svg, ParsedSvgSymbol, SvgFill, SvgShape};

/// Projeto completo do QElectroTech (.qet), suportando múltiplas páginas (folhas/diagramas).
#[derive(Debug, Clone, Default)]
pub struct QetProject {
    pub title: String,
    pub version: String,
    pub raw_properties: Option<String>,
    pub raw_newdiagrams: Option<String>,
    pub raw_collection: Option<String>,
    pub diagrams: Vec<QetDiagram>,
    pub active_diagram_index: usize,
    pub element_definitions: HashMap<String, QetDefinition>,
    pub terminal_coords: HashMap<(String, String), (f32, f32)>,
}

/// Página individual (diagrama/folio) do QElectroTech.
#[derive(Debug, Clone, Default)]
pub struct QetDiagram {
    pub order: i32,
    pub title: String,
    pub author: String,
    pub date: String,
    pub filename: String,
    pub rows: i32,
    pub cols: i32,
    pub rowsize: f32,
    pub colsize: f32,
    pub height: f32,
    pub width: f32,
    pub elements: Vec<QetElementInstance>,
    pub conductors: Vec<QetConductor>,
    pub inputs: Vec<QetInput>,
    pub raw_attributes: HashMap<String, String>,
}

/// Instância de um elemento em um diagrama QET.
#[derive(Debug, Clone, Default)]
pub struct QetElementInstance {
    pub uuid: String,
    pub element_type: String,
    pub x: f32,
    pub y: f32,
    pub orientation: i32,
    pub z: i32,
    pub prefix: String,
    pub labels: Vec<String>,
    pub raw_attributes: HashMap<String, String>,
}

/// Condutor elétrico (cabo/conexão) ligando terminais entre elementos no QET.
#[derive(Debug, Clone, Default)]
pub struct QetConductor {
    pub element1: String,
    pub terminal1: String,
    pub element2: String,
    pub terminal2: String,
    pub color: Option<String>,
    pub num: Option<String>,
    pub formula: Option<String>,
    pub segments: Vec<QetConductorSegment>,
    pub raw_attributes: HashMap<String, String>,
}

/// Segmento de roteamento do condutor (horizontal ou vertical com comprimento).
#[derive(Debug, Clone, Default)]
pub struct QetConductorSegment {
    pub orientation: String,
    pub length: f32,
}

/// Texto gráfico / anotação em um diagrama QET.
#[derive(Debug, Clone, Default)]
pub struct QetInput {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub font: String,
    pub rotation: i32,
}

/// Metadados de definição (.elmt) de um elemento embutido no `<collection>`.
#[derive(Debug, Clone, Default)]
pub struct QetDefinition {
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub hotspot_x: f32,
    pub hotspot_y: f32,
    pub terminals: Vec<QetTerminalDef>,
}

/// Terminal de conexão definido dentro de um elemento (.elmt).
#[derive(Debug, Clone, Default)]
pub struct QetTerminalDef {
    pub uuid: String,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub orientation: String,
}

/// Remove formatação HTML de textos ricos gerados pelo QET (ex: `<!DOCTYPE HTML...><p>Texto</p>`).
pub fn clean_html_text(raw: &str) -> String {
    let s = raw.trim();
    if s.is_empty() {
        return String::new();
    }
    if !s.contains('<') {
        return s.to_string();
    }

    let mut out = String::new();
    let mut in_tag = false;
    let mut in_head = false;
    let mut in_style = false;

    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if !in_tag && chars[i] == '<' {
            in_tag = true;
            // Verifica tags a ignorar
            let rest: String = chars[i..].iter().take(10).collect();
            let lower = rest.to_ascii_lowercase();
            if lower.starts_with("<style") {
                in_style = true;
            } else if lower.starts_with("</style") {
                in_style = false;
            } else if lower.starts_with("<head") {
                in_head = true;
            } else if lower.starts_with("</head") {
                in_head = false;
            }
            i += 1;
            continue;
        }

        if in_tag {
            if chars[i] == '>' {
                in_tag = false;
            }
            i += 1;
            continue;
        }

        if !in_head && !in_style {
            out.push(chars[i]);
        }
        i += 1;
    }

    // Decodifica entidades HTML básicas
    let res = out
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ");

    res.trim().to_string()
}

/// Converte uma cor hexadecimal "#RRGGBB" para um inteiro de 24 bits `0x00RRGGBB`.
pub fn parse_hex_color(hex_str: &str) -> i32 {
    let s = hex_str.trim().trim_start_matches('#');
    if s.len() == 6 {
        if let Ok(val) = u32::from_str_radix(s, 16) {
            // Se for preto puro (0x000000), usa 0x010101 para diferenciar de ausência de cor (0)
            if val == 0 {
                return 0x010101;
            }
            return (val & 0xFFFFFF) as i32;
        }
    }
    0
}

/// Converte um inteiro `0x00RRGGBB` de volta para string "#rrggbb".
pub fn format_hex_color(color_int: i32) -> String {
    let r = (color_int >> 16) & 0xFF;
    let g = (color_int >> 8) & 0xFF;
    let b = color_int & 0xFF;
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

/// Converte um arco elíptico QET em polilinha suave (16 passos).
fn arc_to_polyline(
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

/// Analisa estilo QET `style="..."` retornando `(dashed, fill)`.
fn parse_qet_style(style_str: &str) -> (bool, SvgFill) {
    let mut dashed = false;
    let mut fill = SvgFill::None;

    for part in style_str.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("line-style:") {
            if val == "dashed" || val == "dotted" || val == "dashdotted" {
                dashed = true;
            }
        } else if let Some(val) = part.strip_prefix("filling:") {
            let v = val.trim().to_ascii_lowercase();
            if v.is_empty() || v == "none" {
                fill = SvgFill::None;
            } else if v == "black" {
                fill = SvgFill::Solid;
            } else if v == "white" || v.contains("white") || v.contains("ivory") {
                fill = SvgFill::White;
            } else {
                fill = SvgFill::Tint;
            }
        }
    }

    (dashed, fill)
}

/// Carrega um projeto QET a partir de uma string XML.
pub fn load_qet_from_str(xml: &str) -> Result<QetProject, String> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| format!("Erro ao fazer parse do XML QET: {e}"))?;

    let root = doc.root_element();
    if root.tag_name().name() != "project" {
        return Err(
            "O arquivo não possui elemento raiz <project> válido do QElectroTech".to_string(),
        );
    }

    let mut project = QetProject {
        title: root.attribute("title").unwrap_or("Projeto QET").to_string(),
        version: root.attribute("version").unwrap_or("0.100.0").to_string(),
        ..Default::default()
    };

    // 1. Processa a coleção de elementos embutidos (<collection>)
    for child in root.children() {
        if child.is_element() {
            match child.tag_name().name() {
                "collection" => {
                    let mut defs = HashMap::new();
                    let mut term_map = HashMap::new();
                    scan_collection_node(&child, "", &mut defs, &mut term_map);
                    project.element_definitions = defs;
                    project.terminal_coords = term_map;
                }
                "diagram" => {
                    let diagram = parse_diagram_node(&child);
                    project.diagrams.push(diagram);
                }
                _ => {}
            }
        }
    }

    // Se o XML tiver coleção embutida como texto, preserva para gravação
    if let Some(coll_node) = root
        .children()
        .find(|c| c.tag_name().name() == "collection")
    {
        let coll_range = coll_node.range();
        project.raw_collection = Some(xml[coll_range].to_string());
    }

    // Ordena diagramas pela ordem numérica
    project.diagrams.sort_by_key(|d| d.order);

    Ok(project)
}

/// Varre recursivamente nós `<category>` e `<element>` em `<collection>`.
fn scan_collection_node<'a>(
    node: &roxmltree::Node<'a, 'a>,
    cat_path: &str,
    defs: &mut HashMap<String, QetDefinition>,
    term_map: &mut HashMap<(String, String), (f32, f32)>,
) {
    for child in node.children() {
        if !child.is_element() {
            continue;
        }

        match child.tag_name().name() {
            "category" => {
                let name = child.attribute("name").unwrap_or("");
                let next_path = if cat_path.is_empty() {
                    name.to_string()
                } else {
                    format!("{cat_path}/{name}")
                };
                scan_collection_node(&child, &next_path, defs, term_map);
            }
            "element" => {
                let el_name = child.attribute("name").unwrap_or("");
                let full_uri = format!("embed://{cat_path}/{el_name}");

                if let Some(def_node) = child
                    .children()
                    .find(|c| c.tag_name().name() == "definition")
                {
                    let (definition, parsed_symbol) = parse_definition_node(&def_node, el_name);

                    // Registra terminais
                    for t in &definition.terminals {
                        term_map.insert((full_uri.clone(), t.uuid.clone()), (t.x, t.y));
                        term_map.insert((el_name.to_string(), t.uuid.clone()), (t.x, t.y));
                    }

                    // Registra símbolo no cache de renderização SVG do CADe
                    register_parsed_svg(&full_uri, parsed_symbol.clone());
                    register_parsed_svg(el_name, parsed_symbol);

                    defs.insert(full_uri.clone(), definition.clone());
                    defs.insert(el_name.to_string(), definition);
                }
            }
            _ => {}
        }
    }
}

/// Faz o parse de uma `<definition>` e constrói um `ParsedSvgSymbol` nativo.
fn parse_definition_node<'a>(
    def_node: &roxmltree::Node<'a, 'a>,
    name: &str,
) -> (QetDefinition, ParsedSvgSymbol) {
    let width: f32 = def_node
        .attribute("width")
        .and_then(|v| v.parse().ok())
        .unwrap_or(40.0);
    let height: f32 = def_node
        .attribute("height")
        .and_then(|v| v.parse().ok())
        .unwrap_or(40.0);
    let hotspot_x: f32 = def_node
        .attribute("hotspot_x")
        .and_then(|v| v.parse().ok())
        .unwrap_or(20.0);
    let hotspot_y: f32 = def_node
        .attribute("hotspot_y")
        .and_then(|v| v.parse().ok())
        .unwrap_or(20.0);

    let mut terminals = Vec::new();
    let mut shapes = Vec::new();
    let mut min_term_x = f32::INFINITY;
    let mut max_term_x = f32::NEG_INFINITY;
    let mut term_top_y = f32::INFINITY;
    let mut term_bottom_y = f32::NEG_INFINITY;

    if let Some(desc_node) = def_node
        .children()
        .find(|c| c.tag_name().name() == "description")
    {
        for child in desc_node.children() {
            if !child.is_element() {
                continue;
            }

            let style = child.attribute("style").unwrap_or("");
            let (dashed, fill) = parse_qet_style(style);

            match child.tag_name().name() {
                "line" => {
                    let x1: f32 = child
                        .attribute("x1")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let y1: f32 = child
                        .attribute("y1")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let x2: f32 = child
                        .attribute("x2")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let y2: f32 = child
                        .attribute("y2")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    shapes.push(SvgShape::Line {
                        x1,
                        y1,
                        x2,
                        y2,
                        dashed,
                    });
                }
                "rect" => {
                    let x: f32 = child
                        .attribute("x")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let y: f32 = child
                        .attribute("y")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let w: f32 = child
                        .attribute("width")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let h: f32 = child
                        .attribute("height")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    shapes.push(SvgShape::Rect {
                        x,
                        y,
                        w,
                        h,
                        dashed,
                        fill,
                    });
                }
                "circle" => {
                    let x: f32 = child
                        .attribute("x")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let y: f32 = child
                        .attribute("y")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let diameter: f32 = child
                        .attribute("diameter")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let r = diameter / 2.0;
                    shapes.push(SvgShape::Circle {
                        cx: x + r,
                        cy: y + r,
                        r,
                        is_terminal: false,
                        fill,
                    });
                }
                "ellipse" => {
                    let x: f32 = child
                        .attribute("x")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let y: f32 = child
                        .attribute("y")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let w: f32 = child
                        .attribute("width")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let h: f32 = child
                        .attribute("height")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    shapes.push(SvgShape::Ellipse {
                        cx: x + w / 2.0,
                        cy: y + h / 2.0,
                        rx: w / 2.0,
                        ry: h / 2.0,
                        fill,
                    });
                }
                "polygon" => {
                    let mut points = Vec::new();
                    let mut i = 1;
                    while let (Some(x_str), Some(y_str)) = (
                        child.attribute(format!("x{i}").as_str()),
                        child.attribute(format!("y{i}").as_str()),
                    ) {
                        if let (Ok(x), Ok(y)) = (x_str.parse::<f32>(), y_str.parse::<f32>()) {
                            points.push((x, y));
                        }
                        i += 1;
                    }
                    let is_closed = child.attribute("closed").unwrap_or("true") == "true";
                    if !points.is_empty() {
                        if is_closed {
                            shapes.push(SvgShape::Polygon { points, fill });
                        } else {
                            shapes.push(SvgShape::Polyline { points });
                        }
                    }
                }
                "arc" => {
                    let x: f32 = child
                        .attribute("x")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let y: f32 = child
                        .attribute("y")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let w: f32 = child
                        .attribute("width")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let h: f32 = child
                        .attribute("height")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let start: f32 = child
                        .attribute("start")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let angle: f32 = child
                        .attribute("angle")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let points = arc_to_polyline(x, y, w, h, start, angle);
                    shapes.push(SvgShape::Polyline { points });
                }
                "dynamic_text" | "text" => {
                    let x: f32 = child
                        .attribute("x")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let y: f32 = child
                        .attribute("y")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let rotation: f32 = child
                        .attribute("rotation")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let size: f32 = child
                        .attribute("size")
                        .and_then(|v| v.parse().ok())
                        .or_else(|| {
                            child
                                .attribute("font")
                                .and_then(|f| f.split(',').nth(1))
                                .and_then(|s| s.trim().parse().ok())
                        })
                        .unwrap_or(7.0);

                    let mut text_str = child.attribute("text").unwrap_or("").to_string();
                    if text_str.is_empty() {
                        if let Some(t_child) =
                            child.children().find(|c| c.tag_name().name() == "text")
                        {
                            if let Some(txt) = t_child.text() {
                                text_str = txt.trim().to_string();
                            }
                        } else if let Some(txt) = child.text() {
                            text_str = txt.trim().to_string();
                        }
                    }

                    if !text_str.is_empty() && text_str != "_" {
                        shapes.push(SvgShape::Text {
                            x,
                            y,
                            text: text_str,
                            size,
                            rotation,
                        });
                    }
                }
                "terminal" => {
                    let x: f32 = child
                        .attribute("x")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let y: f32 = child
                        .attribute("y")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let uuid = child.attribute("uuid").unwrap_or("").to_string();
                    let t_name = child.attribute("name").unwrap_or("").to_string();
                    let ori = child.attribute("orientation").unwrap_or("n").to_string();

                    min_term_x = min_term_x.min(x);
                    max_term_x = max_term_x.max(x);
                    term_top_y = term_top_y.min(y);
                    term_bottom_y = term_bottom_y.max(y);

                    terminals.push(QetTerminalDef {
                        uuid,
                        name: t_name,
                        x,
                        y,
                        orientation: ori,
                    });

                    // Ponto do borne terminal
                    shapes.push(SvgShape::Circle {
                        cx: x,
                        cy: y,
                        r: 1.8,
                        is_terminal: true,
                        fill: SvgFill::Solid,
                    });
                }
                _ => {}
            }
        }
    }

    if min_term_x.is_infinite() {
        min_term_x = 0.0;
    }
    if max_term_x.is_infinite() {
        max_term_x = width;
    }
    if term_top_y.is_infinite() {
        term_top_y = 0.0;
    }
    if term_bottom_y.is_infinite() {
        term_bottom_y = height;
    }

    shapes.sort_by_key(|s| match s {
        SvgShape::Rect { dashed: true, .. } => 0,
        SvgShape::Circle {
            fill: SvgFill::White,
            ..
        }
        | SvgShape::Ellipse {
            fill: SvgFill::White,
            ..
        }
        | SvgShape::Rect {
            fill: SvgFill::White,
            ..
        }
        | SvgShape::Polygon {
            fill: SvgFill::White,
            ..
        } => 1,
        SvgShape::Line { .. } | SvgShape::Polyline { .. } => 2,
        SvgShape::Circle {
            fill: SvgFill::Solid,
            ..
        }
        | SvgShape::Ellipse {
            fill: SvgFill::Solid,
            ..
        }
        | SvgShape::Rect {
            fill: SvgFill::Solid,
            ..
        }
        | SvgShape::Polygon {
            fill: SvgFill::Solid,
            ..
        } => 4,
        SvgShape::Text { .. } => 5,
        _ => 3,
    });

    let view_box = (-hotspot_x, -hotspot_y, width, height);

    let term_points: Vec<(f32, f32)> = terminals.iter().map(|t| (t.x, t.y)).collect();
    let normalized_scale = crate::svg_symbol::calculate_normalized_scale(&term_points, false);

    let parsed_symbol = ParsedSvgSymbol {
        view_box,
        shapes,
        term_top_y,
        term_bottom_y,
        min_term_x,
        max_term_x,
        is_cade_svg: false,
        is_qet_native: true,
        normalized_scale,
    };

    let definition = QetDefinition {
        name: name.to_string(),
        width,
        height,
        hotspot_x,
        hotspot_y,
        terminals,
    };

    (definition, parsed_symbol)
}

/// Faz o parse de um nó `<diagram>` individual.
fn parse_diagram_node<'a>(diag_node: &roxmltree::Node<'a, 'a>) -> QetDiagram {
    let mut raw_attributes = HashMap::new();
    for attr in diag_node.attributes() {
        raw_attributes.insert(attr.name().to_string(), attr.value().to_string());
    }

    let order = diag_node
        .attribute("order")
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);
    let title = diag_node.attribute("title").unwrap_or("").to_string();
    let author = diag_node.attribute("author").unwrap_or("").to_string();
    let date = diag_node.attribute("date").unwrap_or("").to_string();
    let filename = diag_node.attribute("filename").unwrap_or("").to_string();
    let rows = diag_node
        .attribute("rows")
        .and_then(|v| v.parse().ok())
        .unwrap_or(8);
    let cols = diag_node
        .attribute("cols")
        .and_then(|v| v.parse().ok())
        .unwrap_or(6);
    let rowsize: f32 = diag_node
        .attribute("rowsize")
        .and_then(|v| v.parse().ok())
        .unwrap_or(80.0);
    let colsize: f32 = diag_node
        .attribute("colsize")
        .and_then(|v| v.parse().ok())
        .unwrap_or(170.0);
    let height: f32 = diag_node
        .attribute("height")
        .and_then(|v| v.parse().ok())
        .unwrap_or(660.0);
    let width = cols as f32 * colsize;

    let mut elements = Vec::new();
    let mut conductors = Vec::new();
    let mut inputs = Vec::new();

    for child in diag_node.children() {
        if !child.is_element() {
            continue;
        }

        match child.tag_name().name() {
            "elements" => {
                for el_node in child
                    .children()
                    .filter(|c| c.tag_name().name() == "element")
                {
                    let mut el_attrs = HashMap::new();
                    for attr in el_node.attributes() {
                        el_attrs.insert(attr.name().to_string(), attr.value().to_string());
                    }

                    let uuid = el_node.attribute("uuid").unwrap_or("").to_string();
                    let element_type = el_node.attribute("type").unwrap_or("").to_string();
                    let x: f32 = el_node
                        .attribute("x")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let y: f32 = el_node
                        .attribute("y")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let orientation = el_node
                        .attribute("orientation")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0);
                    let z = el_node
                        .attribute("z")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(10);
                    let prefix = el_node.attribute("prefix").unwrap_or("").to_string();

                    let mut labels = Vec::new();
                    for info_block in el_node
                        .children()
                        .filter(|c| c.tag_name().name() == "elementInformations")
                    {
                        for info in info_block
                            .children()
                            .filter(|c| c.tag_name().name() == "elementInformation")
                        {
                            if info.attribute("name") == Some("label") {
                                let t = clean_html_text(info.text().unwrap_or(""));
                                if !t.is_empty() && !labels.contains(&t) {
                                    labels.push(t);
                                }
                            }
                        }
                    }
                    for inp in el_node
                        .children()
                        .filter(|c| c.tag_name().name() == "input")
                    {
                        let t = clean_html_text(
                            inp.text().or_else(|| inp.attribute("text")).unwrap_or(""),
                        );
                        if !t.is_empty() && !labels.contains(&t) {
                            labels.push(t);
                        }
                    }

                    elements.push(QetElementInstance {
                        uuid,
                        element_type,
                        x,
                        y,
                        orientation,
                        z,
                        prefix,
                        labels,
                        raw_attributes: el_attrs,
                    });
                }
            }
            "conductors" => {
                for cond_node in child
                    .children()
                    .filter(|c| c.tag_name().name() == "conductor")
                {
                    let mut cond_attrs = HashMap::new();
                    for attr in cond_node.attributes() {
                        cond_attrs.insert(attr.name().to_string(), attr.value().to_string());
                    }

                    let element1 = cond_node.attribute("element1").unwrap_or("").to_string();
                    let terminal1 = cond_node.attribute("terminal1").unwrap_or("").to_string();
                    let element2 = cond_node.attribute("element2").unwrap_or("").to_string();
                    let terminal2 = cond_node.attribute("terminal2").unwrap_or("").to_string();
                    let color = cond_node.attribute("color").map(|s| s.to_string());
                    let num = cond_node.attribute("num").map(|s| s.to_string());
                    let formula = cond_node.attribute("formula").map(|s| s.to_string());

                    let mut segments = Vec::new();
                    for seg_node in cond_node
                        .children()
                        .filter(|c| c.tag_name().name() == "segment")
                    {
                        let ori = seg_node
                            .attribute("orientation")
                            .unwrap_or("horizontal")
                            .to_string();
                        let len: f32 = seg_node
                            .attribute("length")
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(0.0);
                        segments.push(QetConductorSegment {
                            orientation: ori,
                            length: len,
                        });
                    }

                    conductors.push(QetConductor {
                        element1,
                        terminal1,
                        element2,
                        terminal2,
                        color,
                        num,
                        formula,
                        segments,
                        raw_attributes: cond_attrs,
                    });
                }
            }
            "inputs" => {
                for inp_node in child.children().filter(|c| c.tag_name().name() == "input") {
                    let x: f32 = inp_node
                        .attribute("x")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let y: f32 = inp_node
                        .attribute("y")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0.0);
                    let text = clean_html_text(
                        inp_node
                            .attribute("text")
                            .or_else(|| inp_node.text())
                            .unwrap_or(""),
                    );
                    let font = inp_node.attribute("font").unwrap_or("").to_string();
                    let rotation = inp_node
                        .attribute("rotation")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0);

                    if !text.is_empty() {
                        inputs.push(QetInput {
                            x,
                            y,
                            text,
                            font,
                            rotation,
                        });
                    }
                }
            }
            _ => {}
        }
    }

    QetDiagram {
        order,
        title,
        author,
        date,
        filename,
        rows,
        cols,
        rowsize,
        colsize,
        height,
        width,
        elements,
        conductors,
        inputs,
        raw_attributes,
    }
}

/// Carrega um arquivo .qet do disco.
pub fn load_qet_file(path: &Path) -> Result<QetProject, String> {
    let bytes = fs::read(path).map_err(|e| format!("Falha ao ler {}: {e}", path.display()))?;
    let text = String::from_utf8(bytes)
        .map_err(|e| format!("Erro UTF-8 ao ler {}: {e}", path.display()))?;
    load_qet_from_str(&text)
}

/// Converte um `QetDiagram` em um `SchematicDocument` para exibição e simulação no CADe_SIMU RS.
pub fn diagram_to_document(project: &QetProject, diagram_idx: usize) -> SchematicDocument {
    let mut doc = SchematicDocument::default();
    let Some(diag) = project.diagrams.get(diagram_idx) else {
        return doc;
    };

    // 1. Configurações da folha e Cajetín
    doc.title_block.title = diag.title.clone();
    doc.title_block.drawn_by = diag.author.clone();
    doc.title_block.drawn_date = diag.date.clone();
    doc.title_block.sheet = format!("{} / {}", diag.order, project.diagrams.len());
    doc.title_block.file_ref = diag.filename.clone();
    doc.title_block.subtitle = project.title.clone();
    doc.title_block.entity = "QElectroTech Native Import".to_string();

    let is_cade_scaled = diag
        .raw_attributes
        .get("cade_scale")
        .map(|s| s.as_str() == "5")
        .unwrap_or_else(|| {
            diag.elements
                .iter()
                .any(|e| e.element_type.contains("cade_native/type_"))
        });
    let coord_div: f32 = if is_cade_scaled { 5.0 } else { 1.0 };

    if !is_cade_scaled {
        let sheet_w = if diag.cols > 0 && diag.colsize > 0.0 {
            (diag.cols as f32 * diag.colsize).round() as i32
        } else {
            1020
        };
        let sheet_h = if diag.rows > 0 && diag.rowsize > 0.0 {
            (diag.rows as f32 * diag.rowsize).round() as i32
        } else {
            660
        };
        doc.settings.custom_height_mm = sheet_w;
        doc.settings.custom_width_mm = sheet_h;
    } else {
        doc.title_block.entity = diag
            .raw_attributes
            .get("cade_entity")
            .cloned()
            .unwrap_or_default();
    }

    let max_cade_id = diag
        .elements
        .iter()
        .filter_map(|e| {
            e.raw_attributes
                .get("cade_id")
                .and_then(|s| s.parse::<i32>().ok())
        })
        .chain(diag.conductors.iter().filter_map(|c| {
            c.raw_attributes
                .get("cade_id")
                .and_then(|s| s.parse::<i32>().ok())
        }))
        .max()
        .unwrap_or(0);
    let mut id_counter = max_cade_id + 1;
    let mut elem_by_uuid: HashMap<String, &QetElementInstance> = HashMap::new();
    let dict = crate::dictionary::global_concept_dictionary();

    // 2. Elementos gráficos / componentes
    for el in &diag.elements {
        elem_by_uuid.insert(el.uuid.clone(), el);
        if el.uuid.starts_with("node_")
            || el.element_type == "junction"
            || el.element_type.ends_with("/corner.elmt")
        {
            continue;
        }
        if el.element_type.ends_with("/junction.elmt") {
            if el.prefix == "junction_4001"
                || el
                    .raw_attributes
                    .get("cade_junction")
                    .is_some_and(|v| v == "1")
            {
                let jx = (el.x / coord_div).round() as i32;
                let jy = (el.y / coord_div).round() as i32;
                let mut j_item = SchematicItem::new_junction(id_counter, jx, jy);
                id_counter += 1;
                j_item.qet_uuid = Some(el.uuid.clone());
                doc.items.push(j_item);
            }
            continue;
        }
        if el.element_type.contains("cade_native/graphic_") {
            if let Some(g_tipo) = el
                .raw_attributes
                .get("cade_graphic_tipo")
                .and_then(|s| s.parse::<i32>().ok())
            {
                let x1 = (el.x / coord_div).round() as i32;
                let y1 = (el.y / coord_div).round() as i32;
                let x2 = el
                    .raw_attributes
                    .get("cade_x2")
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(x1);
                let y2 = el
                    .raw_attributes
                    .get("cade_y2")
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or(y1);
                let gid = el
                    .raw_attributes
                    .get("cade_id")
                    .and_then(|s| s.parse::<i32>().ok())
                    .unwrap_or_else(|| {
                        let v = id_counter;
                        id_counter += 1;
                        v
                    });
                let mut g_item = SchematicItem::new_graphic(gid, g_tipo, x1, y1, x2, y2);
                if let Some(aux_str) = el.raw_attributes.get("cade_aux") {
                    for (idx, part) in aux_str.split(',').enumerate().take(4) {
                        if let Ok(v) = part.parse::<i32>() {
                            g_item.aux_coords[idx] = v;
                        }
                    }
                }
                g_item.color = el
                    .raw_attributes
                    .get("cade_color")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                g_item.subvariant = el
                    .raw_attributes
                    .get("cade_subvariant")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                g_item.extra_int1 = el
                    .raw_attributes
                    .get("cade_extra1")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                g_item.extra_int2 = el
                    .raw_attributes
                    .get("cade_extra2")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);
                g_item.qet_uuid = Some(el.uuid.clone());
                doc.items.push(g_item);
            }
            continue;
        }

        doc.imported_uuids.insert(el.uuid.clone());

        let item_x = (el.x / coord_div).round() as i32;
        let item_y = (el.y / coord_div).round() as i32;

        if let Some((cade_tipo, mirror_ori)) = parse_cade_native_elmt_uri(&el.element_type) {
            let ori = if mirror_ori >= 4 {
                mirror_ori
            } else {
                el.orientation.rem_euclid(4)
            };
            let cid = el
                .raw_attributes
                .get("cade_id")
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or_else(|| {
                    let v = id_counter;
                    id_counter += 1;
                    v
                });
            let mut item = if let Some(sym) = crate::symbols::find_symbol(cade_tipo) {
                sym.instantiate(cid, item_x, item_y, ori)
            } else {
                SchematicItem::new_wire(cid, cade_tipo, item_x, item_y, item_x, item_y)
            };
            item.orientation = ori;
            item.qet_uuid = Some(el.uuid.clone());
            item.tag = el.labels.first().cloned().unwrap_or_default();
            item.show_tag = el
                .raw_attributes
                .get("cade_show_tag")
                .and_then(|s| s.parse().ok())
                .unwrap_or(if item.tag.is_empty() { 0 } else { 1 });
            if let Some(val) = el.raw_attributes.get("cade_value") {
                item.value = val.clone();
            } else if let Some(val2) = el.labels.get(1) {
                item.value = val2.clone();
            }
            item.show_value = el
                .raw_attributes
                .get("cade_show_value")
                .and_then(|s| s.parse().ok())
                .unwrap_or(if item.value.is_empty() { 0 } else { 1 });
            if let Some(sub) = el
                .raw_attributes
                .get("cade_subvariant")
                .and_then(|s| s.parse().ok())
            {
                item.subvariant = sub;
            }
            if let Some(col) = el
                .raw_attributes
                .get("cade_color")
                .and_then(|s| s.parse().ok())
            {
                item.color = col;
            }
            if let Some(e1) = el
                .raw_attributes
                .get("cade_extra1")
                .and_then(|s| s.parse().ok())
            {
                item.extra_int1 = e1;
            }
            if let Some(e2) = el
                .raw_attributes
                .get("cade_extra2")
                .and_then(|s| s.parse().ok())
            {
                item.extra_int2 = e2;
            }
            if let Some(tr) = el.raw_attributes.get("cade_trailing") {
                item.trailing_str = tr.clone();
            }
            if let Some(pins_str) = el.raw_attributes.get("cade_pins") {
                for (pidx, pval) in pins_str.split('|').enumerate().take(9) {
                    item.pins[pidx] = pval.to_string();
                }
            }
            if let Some(vis_str) = el.raw_attributes.get("cade_pin_vis") {
                for (pidx, vval) in vis_str.split(',').enumerate().take(9) {
                    if let Ok(v) = vval.parse::<i32>() {
                        item.pin_visibility[pidx] = v;
                    }
                }
            }
            item.concept_id = dict
                .get_by_cade_tipo(cade_tipo)
                .map(|c| c.concept_id.clone());
            doc.items.push(item);
            continue;
        }

        let mut item = SchematicItem::new_wire(id_counter, 20000, item_x, item_y, item_x, item_y);
        id_counter += 1;

        item.qet_type = Some(el.element_type.clone());
        item.qet_uuid = Some(el.uuid.clone());
        item.orientation = el.orientation;
        item.show_tag = if el.labels.is_empty() { 0 } else { 1 };
        item.tag = el.labels.first().cloned().unwrap_or_default();

        if let Some(def) = project.element_definitions.get(&el.element_type) {
            for (pidx, term) in def.terminals.iter().enumerate().take(9) {
                item.pins[pidx] = term.name.clone();
                item.pin_visibility[pidx] = 1;
            }
        }

        let def_name = project
            .element_definitions
            .get(&el.element_type)
            .map(|d| d.name.as_str());
        if let Some(resolved) = dict.resolve_external_qet(&el.element_type, def_name, &item.pins) {
            item.concept_id = Some(resolved.concept.concept_id.clone());
        }

        doc.items.push(item);
    }

    // 3. Condutores (cabos elétricos)
    for (cond_idx, cond) in diag.conductors.iter().enumerate() {
        let el1 = elem_by_uuid.get(&cond.element1);
        let el2 = elem_by_uuid.get(&cond.element2);
        if el1.is_none() || el2.is_none() {
            continue;
        }
        let el1 = el1.unwrap();
        let el2 = el2.unwrap();
        let cond_uuid = format!("cond_{cond_idx}");
        doc.imported_uuids.insert(cond_uuid.clone());

        let t1_offset = project
            .terminal_coords
            .get(&(el1.element_type.clone(), cond.terminal1.clone()))
            .copied()
            .unwrap_or((0.0, 0.0));
        let t2_offset = project
            .terminal_coords
            .get(&(el2.element_type.clone(), cond.terminal2.clone()))
            .copied()
            .unwrap_or((0.0, 0.0));

        let (rx1, ry1) = transform_offset_f32(t1_offset.0, t1_offset.1, el1.orientation);
        let (rx2, ry2) = transform_offset_f32(t2_offset.0, t2_offset.1, el2.orientation);

        let w1x = ((el1.x + rx1) / coord_div).round() as i32;
        let w1y = ((el1.y + ry1) / coord_div).round() as i32;
        let w2x = ((el2.x + rx2) / coord_div).round() as i32;
        let w2y = ((el2.y + ry2) / coord_div).round() as i32;

        let wire_color_int = cond.color.as_deref().map(parse_hex_color).unwrap_or(0);
        let tipo = if let Some(ct) = cond
            .raw_attributes
            .get("cade_tipo")
            .and_then(|s| s.parse::<i32>().ok())
        {
            ct
        } else if wire_color_int == 0xFF0000 {
            4003 // DC+ (Vermelho)
        } else if wire_color_int == 0x010101 || wire_color_int == 0xAA5500 {
            4004 // DC- / Terra / Marrom automotivo
        } else if is_cade_scaled && wire_color_int == 0x0000FF {
            4009 // Neutro
        } else if is_cade_scaled && wire_color_int == 0x00AA00 {
            4010 // PE
        } else {
            4000
        };

        if !cond.segments.is_empty() {
            let mut cx_f = (el1.x + rx1) / coord_div;
            let mut cy_f = (el1.y + ry1) / coord_div;
            let mut cx = w1x;
            let mut cy = w1y;
            for seg in &cond.segments {
                if seg.orientation == "horizontal" {
                    cx_f += seg.length / coord_div;
                } else {
                    cy_f += seg.length / coord_div;
                }
                let nx = cx_f.round() as i32;
                let ny = cy_f.round() as i32;
                if cx == nx && cy == ny {
                    continue;
                }

                let mut wire = SchematicItem::new_wire(id_counter, tipo, cx, cy, nx, ny);
                id_counter += 1;
                wire.color = if is_cade_scaled { 0 } else { wire_color_int };
                wire.trailing_str = cond.num.clone().unwrap_or_default();
                wire.qet_uuid = Some(cond_uuid.clone());
                doc.items.push(wire);

                cx = nx;
                cy = ny;
            }

            // Segmento final de alinhamento com terminal 2 se houver pequeno arredondamento
            if cx != w2x || cy != w2y {
                let mut wire = SchematicItem::new_wire(id_counter, tipo, cx, cy, w2x, w2y);
                id_counter += 1;
                wire.color = if is_cade_scaled { 0 } else { wire_color_int };
                wire.qet_uuid = Some(cond_uuid.clone());
                doc.items.push(wire);
            }
        } else if w1x == w2x || w1y == w2y {
            // Reta única direta
            let wid = cond
                .raw_attributes
                .get("cade_id")
                .and_then(|s| s.parse::<i32>().ok())
                .unwrap_or_else(|| {
                    let v = id_counter;
                    id_counter += 1;
                    v
                });
            let mut wire = SchematicItem::new_wire(wid, tipo, w1x, w1y, w2x, w2y);
            wire.color = if is_cade_scaled { 0 } else { wire_color_int };
            wire.trailing_str = cond.num.clone().unwrap_or_default();
            wire.qet_uuid = Some(cond_uuid.clone());
            doc.items.push(wire);
        } else {
            // Rota ortogonal padrão em L (Manhattan)
            let mut wire1 = SchematicItem::new_wire(id_counter, tipo, w1x, w1y, w2x, w1y);
            id_counter += 1;
            wire1.color = if is_cade_scaled { 0 } else { wire_color_int };
            wire1.trailing_str = cond.num.clone().unwrap_or_default();
            wire1.qet_uuid = Some(cond_uuid.clone());
            doc.items.push(wire1);

            let mut wire2 = SchematicItem::new_wire(id_counter, tipo, w2x, w1y, w2x, w2y);
            id_counter += 1;
            wire2.color = if is_cade_scaled { 0 } else { wire_color_int };
            wire2.qet_uuid = Some(cond_uuid.clone());
            doc.items.push(wire2);
        }
    }

    // 4. Textos e anotações do diagrama
    for inp in &diag.inputs {
        let text_clean = clean_html_text(&inp.text);
        if !text_clean.is_empty() {
            let tx = (inp.x / coord_div).round() as i32;
            let ty = (inp.y / coord_div).round() as i32;
            let mut text_item = SchematicItem::new_graphic(id_counter, 8, tx, ty, tx, ty);
            id_counter += 1;
            text_item.trailing_str = text_clean;
            let font_parts: Vec<&str> = inp.font.split(',').collect();
            if let Some(pt) = font_parts.get(1).and_then(|s| s.trim().parse::<f32>().ok()) {
                let h = if is_cade_scaled {
                    (pt / 2.2).clamp(2.0, 16.0)
                } else {
                    (pt * 0.355).clamp(2.0, 16.0)
                };
                text_item.set_text_world_height(h);
            }
            let is_bold = font_parts
                .get(4)
                .and_then(|s| s.trim().parse::<i32>().ok())
                .is_some_and(|w| w >= 65)
                || inp.font.to_ascii_lowercase().contains("bold");
            text_item.set_text_bold(is_bold);
            text_item.orientation = (inp.rotation.rem_euclid(360) + 45) / 90 % 4;
            text_item.sync_text_bounds();
            doc.items.push(text_item);
        }
    }

    doc
}

/// Atualiza as posições e tags dos elementos em um `QetDiagram` a partir do `SchematicDocument` editado.
pub fn update_diagram_from_document(diagram: &mut QetDiagram, doc: &SchematicDocument) {
    if !doc.title_block.title.is_empty() {
        diagram.title = doc.title_block.title.clone();
    }
    if !doc.title_block.drawn_by.is_empty() {
        diagram.author = doc.title_block.drawn_by.clone();
    }
    if !doc.title_block.drawn_date.is_empty() {
        diagram.date = doc.title_block.drawn_date.clone();
    }

    // 1. Identifica UUIDs atualmente presentes no documento editado
    let current_uuids: std::collections::HashSet<&str> = doc
        .items
        .iter()
        .filter_map(|i| i.qet_uuid.as_deref())
        .collect();

    // 2. Remove elementos que foram importados originalmente do QET mas foram apagados pelo usuário
    let removed_element_uuids: std::collections::HashSet<String> = diagram
        .elements
        .iter()
        .filter(|e| {
            doc.imported_uuids.contains(&e.uuid) && !current_uuids.contains(e.uuid.as_str())
        })
        .map(|e| e.uuid.clone())
        .collect();

    diagram
        .elements
        .retain(|e| !removed_element_uuids.contains(&e.uuid));

    // 3. Remove condutores conectados a elementos removidos ou condutores importados apagados
    let mut retained_conductors = Vec::with_capacity(diagram.conductors.len());
    for (cond_idx, cond) in diagram.conductors.drain(..).enumerate() {
        let cond_id = format!("cond_{cond_idx}");
        if removed_element_uuids.contains(&cond.element1)
            || removed_element_uuids.contains(&cond.element2)
        {
            continue;
        }
        if doc.imported_uuids.contains(&cond_id) && !current_uuids.contains(cond_id.as_str()) {
            continue;
        }
        retained_conductors.push(cond);
    }
    diagram.conductors = retained_conductors;

    // Limpa nós de junção gerados para fios ("node_...") que ficaram sem nenhum condutor conectado
    let active_conductor_elements: std::collections::HashSet<&str> = diagram
        .conductors
        .iter()
        .flat_map(|c| [&c.element1[..], &c.element2[..]])
        .collect();
    diagram.elements.retain(|e| {
        !e.uuid.starts_with("node_") || active_conductor_elements.contains(e.uuid.as_str())
    });

    // 4. Atualiza elementos existentes e insere novos elementos e novos condutores
    for item in &doc.items {
        if let Some(ref uuid) = item.qet_uuid {
            if let Some(el) = diagram.elements.iter_mut().find(|e| &e.uuid == uuid) {
                el.x = item.x1 as f32;
                el.y = item.y1 as f32;
                el.orientation = item.orientation;
                if !item.tag.is_empty() {
                    if el.labels.is_empty() {
                        el.labels.push(item.tag.clone());
                    } else {
                        el.labels[0] = item.tag.clone();
                    }
                }
            }
        } else if item.is_wire() {
            // Novo fio desenhado pelo usuário no editor
            let junc_uri = "embed://import/cade_native/junction.elmt";
            let node1 = make_qet_uuid(&format!("diag_{}_wnode_{}_1", diagram.order, item.id));
            let node2 = make_qet_uuid(&format!("diag_{}_wnode_{}_2", diagram.order, item.id));
            diagram.elements.push(QetElementInstance {
                uuid: node1.clone(),
                element_type: junc_uri.to_string(),
                x: item.x1 as f32,
                y: item.y1 as f32,
                ..Default::default()
            });
            diagram.elements.push(QetElementInstance {
                uuid: node2.clone(),
                element_type: junc_uri.to_string(),
                x: item.x2 as f32,
                y: item.y2 as f32,
                ..Default::default()
            });

            let (dir12, dir21) = wire_endpoint_dirs(item.x1, item.y1, item.x2, item.y2);
            let term1 = make_qet_uuid(&format!("term_{junc_uri}_dir_{dir12}"));
            let term2 = make_qet_uuid(&format!("term_{junc_uri}_dir_{dir21}"));

            let seg_h = item.x2 - item.x1;
            let seg_v = item.y2 - item.y1;
            let mut segs = Vec::new();
            if seg_h != 0 && seg_v != 0 {
                segs.push(QetConductorSegment {
                    orientation: "horizontal".to_string(),
                    length: seg_h as f32,
                });
                segs.push(QetConductorSegment {
                    orientation: "vertical".to_string(),
                    length: seg_v as f32,
                });
            }
            diagram.conductors.push(QetConductor {
                element1: node1,
                terminal1: term1,
                element2: node2,
                terminal2: term2,
                color: default_qet_wire_color(item),
                num: if !item.trailing_str.is_empty() {
                    Some(item.trailing_str.clone())
                } else {
                    None
                },
                segments: segs,
                ..Default::default()
            });
        } else if !item.is_junction() && !(1..=8).contains(&item.tipo) {
            // Novo elemento adicionado sem UUID QET prévio
            let cade_tipo = crate::dictionary::global_concept_dictionary()
                .resolve_for_cade_export(item)
                .unwrap_or(item.tipo);
            let el_type = item
                .qet_type
                .clone()
                .unwrap_or_else(|| format!("embed://import/cade_native/type_{cade_tipo}.elmt"));
            let uuid = format!("item_{}", item.id);
            diagram.elements.push(QetElementInstance {
                uuid,
                element_type: el_type,
                x: item.x1 as f32,
                y: item.y1 as f32,
                orientation: item.orientation.rem_euclid(4),
                z: 10,
                prefix: String::new(),
                labels: if item.tag.is_empty() {
                    vec![]
                } else {
                    vec![item.tag.clone()]
                },
                ..Default::default()
            });
        }
    }
}

/// Serializa um `QetProject` em uma string XML válida do QElectroTech.
pub fn serialize_qet_xml(project: &QetProject) -> String {
    let mut xml = String::with_capacity(64 * 1024);
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str(&format!(
        "<project title=\"{}\" version=\"{}\">\n",
        escape_xml_attr(&project.title),
        escape_xml_attr(&project.version)
    ));

    xml.push_str("  <properties/>\n");
    xml.push_str("  <newdiagrams/>\n");

    // Diagramas / Folhas
    for diag in &project.diagrams {
        let mut extra_diag_attrs = String::new();
        if let Some(cs) = diag.raw_attributes.get("cade_scale") {
            extra_diag_attrs.push_str(&format!(" cade_scale=\"{}\"", escape_xml_attr(cs)));
        }
        if let Some(ce) = diag.raw_attributes.get("cade_entity") {
            if !ce.is_empty() {
                extra_diag_attrs.push_str(&format!(" cade_entity=\"{}\"", escape_xml_attr(ce)));
            }
        }
        xml.push_str(&format!(
            "  <diagram order=\"{}\" title=\"{}\" author=\"{}\" date=\"{}\" filename=\"{}\" rows=\"{}\" cols=\"{}\" rowsize=\"{:.1}\" colsize=\"{:.1}\" height=\"{:.1}\" displayrows=\"true\" displaycols=\"true\"{extra_diag_attrs}>\n",
            diag.order,
            escape_xml_attr(&diag.title),
            escape_xml_attr(&diag.author),
            escape_xml_attr(&diag.date),
            escape_xml_attr(&diag.filename),
            diag.rows,
            diag.cols,
            diag.rowsize,
            diag.colsize,
            diag.height
        ));

        // Elementos
        xml.push_str("    <elements>\n");
        for el in &diag.elements {
            let mut extra_el_attrs = String::new();
            let mut sorted_keys: Vec<&String> = el
                .raw_attributes
                .keys()
                .filter(|k| k.starts_with("cade_"))
                .collect();
            sorted_keys.sort();
            for k in sorted_keys {
                if let Some(v) = el.raw_attributes.get(k) {
                    extra_el_attrs.push_str(&format!(" {k}=\"{}\"", escape_xml_attr(v)));
                }
            }
            let prefix_attr = if el.prefix.is_empty() {
                String::new()
            } else {
                format!(" prefix=\"{}\"", escape_xml_attr(&el.prefix))
            };
            xml.push_str(&format!(
                "      <element type=\"{}\" x=\"{:.1}\" y=\"{:.1}\" orientation=\"{}\" uuid=\"{}\" z=\"{}\" freezeLabel=\"false\"{prefix_attr}{extra_el_attrs}>\n",
                escape_xml_attr(&el.element_type),
                el.x,
                el.y,
                el.orientation,
                escape_xml_attr(&el.uuid),
                el.z
            ));
            if let Some(first_lbl) = el.labels.first() {
                if !first_lbl.is_empty() {
                    xml.push_str("        <elementInformations>\n");
                    xml.push_str(&format!(
                        "          <elementInformation name=\"formula\" show=\"1\">{}</elementInformation>\n",
                        escape_xml_attr(first_lbl)
                    ));
                    xml.push_str(&format!(
                        "          <elementInformation name=\"label\" show=\"1\">{}</elementInformation>\n",
                        escape_xml_attr(first_lbl)
                    ));
                    if let Some(val_lbl) = el.labels.get(1) {
                        if !val_lbl.is_empty() {
                            xml.push_str(&format!(
                                "          <elementInformation name=\"comment\" show=\"1\">{}</elementInformation>\n",
                                escape_xml_attr(val_lbl)
                            ));
                        }
                    }
                    xml.push_str("        </elementInformations>\n");
                    let dyn_uuid = make_qet_uuid(&format!("dyn_inst_{}_label", el.uuid));
                    xml.push_str("        <dynamic_texts>\n");
                    xml.push_str(&format!(
                        "          <dynamic_elmt_text x=\"-28\" y=\"12\" z=\"18\" rotation=\"0\" font=\"Sans Serif,7,-1,5,50,0,0,0,0,0\" uuid=\"{dyn_uuid}\" frame=\"false\" text_width=\"-1\" keep_visual_rotation=\"true\" Halignment=\"AlignLeft\" Valignment=\"AlignTop\" text_from=\"ElementInfo\">\n"
                    ));
                    xml.push_str(&format!(
                        "            <text>{}</text>\n",
                        escape_xml_attr(first_lbl)
                    ));
                    xml.push_str("            <info_name>label</info_name>\n");
                    xml.push_str("          </dynamic_elmt_text>\n");
                    if let Some(val_lbl) = el.labels.get(1) {
                        if !val_lbl.is_empty() {
                            let val_uuid = make_qet_uuid(&format!("dyn_inst_{}_val", el.uuid));
                            xml.push_str(&format!(
                                "          <dynamic_elmt_text x=\"-28\" y=\"24\" z=\"19\" rotation=\"0\" font=\"Sans Serif,6,-1,5,50,0,0,0,0,0\" uuid=\"{val_uuid}\" frame=\"false\" text_width=\"-1\" keep_visual_rotation=\"true\" Halignment=\"AlignLeft\" Valignment=\"AlignTop\" text_from=\"UserText\">\n"
                            ));
                            xml.push_str(&format!(
                                "            <text>{}</text>\n",
                                escape_xml_attr(val_lbl)
                            ));
                            xml.push_str("          </dynamic_elmt_text>\n");
                        }
                    }
                    xml.push_str("        </dynamic_texts>\n");
                }
            }
            for lbl in &el.labels {
                xml.push_str(&format!(
                    "        <input text=\"{}\" />\n",
                    escape_xml_attr(lbl)
                ));
            }
            xml.push_str("      </element>\n");
        }
        xml.push_str("    </elements>\n");

        // Condutores
        xml.push_str("    <conductors>\n");
        for cond in &diag.conductors {
            let color_attr = cond
                .color
                .as_deref()
                .map(|c| format!(" color=\"{}\"", escape_xml_attr(c)))
                .unwrap_or_default();
            let num_attr = cond
                .num
                .as_deref()
                .map(|n| {
                    format!(
                        " num=\"{}\" formula=\"{}\"",
                        escape_xml_attr(n),
                        escape_xml_attr(n)
                    )
                })
                .unwrap_or_default();
            let cade_tipo_attr = cond
                .raw_attributes
                .get("cade_tipo")
                .map(|ct| format!(" cade_tipo=\"{}\"", escape_xml_attr(ct)))
                .unwrap_or_default();
            let cade_id_attr = cond
                .raw_attributes
                .get("cade_id")
                .map(|cid| format!(" cade_id=\"{}\"", escape_xml_attr(cid)))
                .unwrap_or_default();
            let style_attr = cond
                .raw_attributes
                .get("style")
                .map(|st| format!(" style=\"{}\"", escape_xml_attr(st)))
                .unwrap_or_default();

            xml.push_str(&format!(
                "      <conductor element1=\"{}\" terminal1=\"{}\" element2=\"{}\" terminal2=\"{}\" type=\"multi\" condsize=\"1\" x=\"0\" y=\"0\" displaytext=\"1\" onetextperfolio=\"0\" numsize=\"6\" vertirotatetext=\"270\" horizrotatetext=\"0\"{}{}{}{}{}>\n",
                escape_xml_attr(&cond.element1),
                escape_xml_attr(&cond.terminal1),
                escape_xml_attr(&cond.element2),
                escape_xml_attr(&cond.terminal2),
                color_attr,
                num_attr,
                cade_tipo_attr,
                cade_id_attr,
                style_attr
            ));
            for seg in &cond.segments {
                xml.push_str(&format!(
                    "        <segment orientation=\"{}\" length=\"{:.4}\"/>\n",
                    escape_xml_attr(&seg.orientation),
                    seg.length
                ));
            }
            xml.push_str("        <sequentialNumbers/>\n");
            xml.push_str("      </conductor>\n");
        }
        xml.push_str("    </conductors>\n");

        // Inputs (Textos)
        xml.push_str("    <inputs>\n");
        for inp in &diag.inputs {
            let font_attr = if inp.font.is_empty() {
                String::new()
            } else {
                format!(" font=\"{}\"", escape_xml_attr(&inp.font))
            };
            xml.push_str(&format!(
                "      <input x=\"{:.1}\" y=\"{:.1}\" text=\"{}\"{font_attr} rotation=\"{}\"/>\n",
                inp.x,
                inp.y,
                escape_xml_attr(&inp.text),
                inp.rotation
            ));
        }
        xml.push_str("    </inputs>\n");

        xml.push_str("  </diagram>\n");
    }

    // Embute ou estende a coleção (<collection>) com definições completas (.elmt)
    let coll_xml = build_embedded_collection_xml(project);
    xml.push_str(&coll_xml);
    if !coll_xml.ends_with('\n') {
        xml.push('\n');
    }

    xml.push_str("</project>\n");
    xml
}

/// Salva um `QetProject` no caminho especificado de forma atômica utilizando o helper centralizado.
pub fn save_qet_file(path: &Path, project: &QetProject) -> Result<(), String> {
    let xml = serialize_qet_xml(project);
    crate::io::atomic_write(path, xml)
}

/// Converte um `SchematicDocument` em uma página `QetDiagram` com terminais, junções e condutores conectados.
pub fn document_to_diagram(
    doc: &SchematicDocument,
    page_name: &str,
    order: i32,
    filename: &str,
) -> QetDiagram {
    let is_qet_native_doc = doc.title_block.entity == "QElectroTech Native Import"
        || doc.items.iter().any(|i| {
            i.tipo == 20000
                && i.qet_type
                    .as_deref()
                    .is_some_and(|t| !t.contains("cade_native/"))
        });
    let scale: f32 = if is_qet_native_doc { 1.0 } else { 5.0 };

    let mut max_coord_x = 300i32;
    let mut max_coord_y = 220i32;
    for item in &doc.items {
        max_coord_x = max_coord_x.max(item.x1).max(item.x2);
        max_coord_y = max_coord_y.max(item.y1).max(item.y2);
    }

    let (cols, colsize, rows, rowsize, height, width) = if is_qet_native_doc {
        (6, 170.0, 8, 80.0, 660.0, 1020.0)
    } else {
        let cs = 180.0f32;
        let rs = 120.0f32;
        let c = ((max_coord_x as f32 * scale + 160.0) / cs)
            .ceil()
            .clamp(8.0, 24.0) as i32;
        let r = ((max_coord_y as f32 * scale + 160.0) / rs)
            .ceil()
            .clamp(8.0, 24.0) as i32;
        (c, cs, r, rs, r as f32 * rs, c as f32 * cs)
    };

    let mut raw_attributes = HashMap::new();
    if !is_qet_native_doc {
        raw_attributes.insert("cade_scale".to_string(), "5".to_string());
        if !doc.title_block.entity.is_empty() {
            raw_attributes.insert("cade_entity".to_string(), doc.title_block.entity.clone());
        }
    }

    let mut diagram = QetDiagram {
        order,
        title: if !page_name.is_empty() {
            page_name.to_string()
        } else if !doc.title_block.title.is_empty() {
            doc.title_block.title.clone()
        } else {
            format!("Folha {order}")
        },
        author: doc.title_block.drawn_by.clone(),
        date: doc.title_block.drawn_date.clone(),
        filename: filename.to_string(),
        rows,
        cols,
        rowsize,
        colsize,
        height,
        width,
        raw_attributes,
        ..Default::default()
    };

    let dict = crate::dictionary::global_concept_dictionary();

    // 1. Mapeia junções explícitas e grau de conexão de extremidades de cabos
    let mut explicit_junctions: std::collections::HashSet<(i32, i32)> =
        std::collections::HashSet::new();
    let mut wire_degree: HashMap<(i32, i32), usize> = HashMap::new();
    let mut wire_segments_list: Vec<(i32, i32, i32, i32)> = Vec::new();

    for item in &doc.items {
        if item.is_junction() {
            explicit_junctions.insert((item.x1, item.y1));
        } else if item.is_wire() && (item.x1 != item.x2 || item.y1 != item.y2) {
            *wire_degree.entry((item.x1, item.y1)).or_insert(0) += 1;
            *wire_degree.entry((item.x2, item.y2)).or_insert(0) += 1;
            wire_segments_list.push((item.x1, item.y1, item.x2, item.y2));
        }
    }

    // 2. Processa componentes, anotações gráficas e textos, registrando bornes em `pin_at_coord`
    // (wx, wy) -> Vec<(elem_uuid, term_uuid, world_dir)>
    type PinCoordMap = HashMap<(i32, i32), Vec<(String, String, &'static str)>>;
    let mut pin_at_coord: PinCoordMap = HashMap::new();

    for item in &doc.items {
        if item.is_wire()
            || item.is_junction()
            || (item.tipo == 20000
                && item.tag == "XXX"
                && item.trailing_str.is_empty()
                && item.qet_type.is_none())
        {
            continue;
        }
        if item.tipo == 8 {
            let pt = if is_qet_native_doc {
                (item.text_world_height() / 0.355).round().clamp(6.0, 48.0) as i32
            } else {
                (item.text_world_height() * 2.2).round().clamp(6.0, 48.0) as i32
            };
            let weight = if item.text_is_bold() { 75 } else { 50 };
            diagram.inputs.push(QetInput {
                x: item.x1 as f32 * scale,
                y: item.y1 as f32 * scale,
                text: item.trailing_str.clone(),
                font: format!("Sans Serif,{pt},-1,5,{weight},0,0,0,0,0"),
                rotation: item.orientation.rem_euclid(4) * 90,
            });
            continue;
        }
        if (1..=5).contains(&item.tipo) {
            let el_type = format!(
                "embed://import/cade_native/graphic_{order}_{}.elmt",
                item.id
            );
            let uuid = make_qet_uuid(&format!("diag_{order}_graphic_{}", item.id));
            let mut el_attrs = HashMap::new();
            el_attrs.insert("cade_id".to_string(), item.id.to_string());
            el_attrs.insert("cade_graphic_tipo".to_string(), item.tipo.to_string());
            el_attrs.insert("cade_x1".to_string(), item.x1.to_string());
            el_attrs.insert("cade_y1".to_string(), item.y1.to_string());
            el_attrs.insert("cade_x2".to_string(), item.x2.to_string());
            el_attrs.insert("cade_y2".to_string(), item.y2.to_string());
            el_attrs.insert(
                "cade_aux".to_string(),
                format!(
                    "{},{},{},{}",
                    item.aux_coords[0], item.aux_coords[1], item.aux_coords[2], item.aux_coords[3]
                ),
            );
            el_attrs.insert("cade_color".to_string(), item.color.to_string());
            el_attrs.insert("cade_subvariant".to_string(), item.subvariant.to_string());
            el_attrs.insert("cade_extra1".to_string(), item.extra_int1.to_string());
            el_attrs.insert("cade_extra2".to_string(), item.extra_int2.to_string());
            diagram.elements.push(QetElementInstance {
                uuid,
                element_type: el_type,
                x: item.x1 as f32 * scale,
                y: item.y1 as f32 * scale,
                orientation: 0,
                z: 1,
                prefix: String::new(),
                labels: Vec::new(),
                raw_attributes: el_attrs,
            });
            continue;
        }

        let cade_tipo = dict.resolve_for_cade_export(item).unwrap_or(item.tipo);
        let ori8 = item.orientation.rem_euclid(8);
        let (el_type, qet_ori, mirror_ori) = if item.tipo == 20000
            && cade_tipo == 20000
            && item
                .qet_type
                .as_deref()
                .is_some_and(|t| t.starts_with("embed://"))
        {
            (
                item.qet_type.clone().unwrap(),
                item.orientation.rem_euclid(4),
                0,
            )
        } else if ori8 >= 4 {
            (
                format!("embed://import/cade_native/type_{cade_tipo}_m{ori8}.elmt"),
                0,
                ori8,
            )
        } else {
            (
                format!("embed://import/cade_native/type_{cade_tipo}.elmt"),
                ori8,
                0,
            )
        };

        let uuid = item
            .qet_uuid
            .as_deref()
            .filter(|u| u.starts_with('{') && u.ends_with('}'))
            .map(|u| u.to_string())
            .unwrap_or_else(|| make_qet_uuid(&format!("diag_{order}_elem_{}", item.id)));

        if let Some(sym) = crate::symbols::find_symbol(cade_tipo) {
            let mut item_for_pins = item.clone();
            item_for_pins.tipo = cade_tipo;
            for (pin_idx, _lbl, (px, py)) in sym.world_pins(&item_for_pins) {
                let local_dir = pin_local_dir(&sym, sym.pins[pin_idx].conn_pos, mirror_ori);
                let world_dir = rotate_dir(local_dir, qet_ori);
                let term_uuid = make_qet_uuid(&format!("term_{el_type}_pin_{pin_idx}"));
                pin_at_coord.entry((px, py)).or_default().push((
                    uuid.clone(),
                    term_uuid,
                    world_dir,
                ));
            }
        }

        let mut labels = Vec::new();
        if !item.tag.is_empty() {
            labels.push(item.tag.clone());
        }
        if !item.value.is_empty() && item.show_value != 0 {
            if labels.is_empty() {
                labels.push(String::new());
            }
            labels.push(item.value.clone());
        }

        let mut el_attrs = HashMap::new();
        el_attrs.insert("cade_id".to_string(), item.id.to_string());
        el_attrs.insert("cade_value".to_string(), item.value.clone());
        el_attrs.insert("cade_show_tag".to_string(), item.show_tag.to_string());
        el_attrs.insert("cade_show_value".to_string(), item.show_value.to_string());
        if item.subvariant != 0 {
            el_attrs.insert("cade_subvariant".to_string(), item.subvariant.to_string());
        }
        if item.color != 0 {
            el_attrs.insert("cade_color".to_string(), item.color.to_string());
        }
        if item.extra_int1 != 0 {
            el_attrs.insert("cade_extra1".to_string(), item.extra_int1.to_string());
        }
        if item.extra_int2 != 0 {
            el_attrs.insert("cade_extra2".to_string(), item.extra_int2.to_string());
        }
        if !item.trailing_str.is_empty() {
            el_attrs.insert("cade_trailing".to_string(), item.trailing_str.clone());
        }
        el_attrs.insert("cade_pins".to_string(), item.pins.join("|"));
        if item.pin_visibility.iter().any(|&v| v != 1) {
            let vis_s: Vec<String> = item.pin_visibility.iter().map(|v| v.to_string()).collect();
            el_attrs.insert("cade_pin_vis".to_string(), vis_s.join(","));
        }

        diagram.elements.push(QetElementInstance {
            uuid,
            element_type: el_type,
            x: item.x1 as f32 * scale,
            y: item.y1 as f32 * scale,
            orientation: qet_ori,
            z: 10,
            prefix: String::new(),
            labels,
            raw_attributes: el_attrs,
        });
    }

    // 3. Processa cabos/condutores, conectando aos bornes dos componentes ou a nós de junção/canto
    let mut junction_nodes: HashMap<(i32, i32), (String, String)> = HashMap::new();
    let mut emitted_explicit_junctions: std::collections::HashSet<(i32, i32)> =
        std::collections::HashSet::new();

    let mut get_or_create_node_terminal = |wx: i32,
                                           wy: i32,
                                           w_dir: &'static str,
                                           elements: &mut Vec<QetElementInstance>|
     -> (String, String) {
        if let Some(pins) = pin_at_coord.get(&(wx, wy)) {
            if let Some((comp_uuid, term_uuid, _)) = pins
                .iter()
                .find(|(_, _, p_dir)| is_axis_compatible(p_dir, w_dir))
            {
                return (comp_uuid.clone(), term_uuid.clone());
            }
            if wire_degree.get(&(wx, wy)).copied().unwrap_or(0) <= 1 {
                let (comp_uuid, term_uuid, _) = &pins[0];
                return (comp_uuid.clone(), term_uuid.clone());
            }
        }

        let (node_uuid, node_uri) = junction_nodes
            .entry((wx, wy))
            .or_insert_with(|| {
                let is_explicit = explicit_junctions.contains(&(wx, wy));
                let deg = wire_degree.get(&(wx, wy)).copied().unwrap_or(0);
                let on_wire_interior = point_lies_on_any_wire_interior(wx, wy, &wire_segments_list);
                let is_visible_junc = is_explicit || deg >= 3 || on_wire_interior;
                let uri = if is_visible_junc {
                    "embed://import/cade_native/junction.elmt"
                } else {
                    "embed://import/cade_native/corner.elmt"
                };
                let u = make_qet_uuid(&format!("diag_{order}_node_{wx}_{wy}"));
                let mut attrs = HashMap::new();
                let prefix = if is_explicit {
                    emitted_explicit_junctions.insert((wx, wy));
                    attrs.insert("cade_junction".to_string(), "1".to_string());
                    "junction_4001".to_string()
                } else {
                    String::new()
                };
                elements.push(QetElementInstance {
                    uuid: u.clone(),
                    element_type: uri.to_string(),
                    x: wx as f32 * scale,
                    y: wy as f32 * scale,
                    orientation: 0,
                    z: 5,
                    prefix,
                    labels: Vec::new(),
                    raw_attributes: attrs,
                });
                (u, uri.to_string())
            })
            .clone();

        let term_uuid = make_qet_uuid(&format!("term_{node_uri}_dir_{w_dir}"));
        (node_uuid, term_uuid)
    };

    for item in &doc.items {
        if !item.is_wire() {
            continue;
        }
        if item.x1 == item.x2 && item.y1 == item.y2 {
            continue;
        }
        let (dir12, dir21) = wire_endpoint_dirs(item.x1, item.y1, item.x2, item.y2);
        let (el1_uuid, term1_uuid) =
            get_or_create_node_terminal(item.x1, item.y1, dir12, &mut diagram.elements);
        let (el2_uuid, term2_uuid) =
            get_or_create_node_terminal(item.x2, item.y2, dir21, &mut diagram.elements);

        let seg_h = item.x2 - item.x1;
        let seg_v = item.y2 - item.y1;
        let mut segs = Vec::new();
        if seg_h != 0 && seg_v != 0 {
            segs.push(QetConductorSegment {
                orientation: "horizontal".to_string(),
                length: seg_h as f32 * scale,
            });
            segs.push(QetConductorSegment {
                orientation: "vertical".to_string(),
                length: seg_v as f32 * scale,
            });
        }

        let mut cond_attrs = HashMap::new();
        cond_attrs.insert("cade_id".to_string(), item.id.to_string());
        cond_attrs.insert("cade_tipo".to_string(), item.tipo.to_string());
        if matches!(item.tipo, 4002 | 4010 | 4012 | 4013) {
            cond_attrs.insert("style".to_string(), "line-style: dashed;".to_string());
        }

        diagram.conductors.push(QetConductor {
            element1: el1_uuid,
            terminal1: term1_uuid,
            element2: el2_uuid,
            terminal2: term2_uuid,
            color: default_qet_wire_color(item),
            num: if !item.trailing_str.is_empty() {
                Some(item.trailing_str.clone())
            } else {
                None
            },
            formula: if !item.trailing_str.is_empty() {
                Some(item.trailing_str.clone())
            } else {
                None
            },
            segments: segs,
            raw_attributes: cond_attrs,
        });
    }

    // 4. Emite junções explícitas que estavam sobre bornes ou no meio de cabos e ainda não foram emitidas
    for (jx, jy) in explicit_junctions {
        if !emitted_explicit_junctions.contains(&(jx, jy)) {
            let uri = "embed://import/cade_native/junction.elmt";
            let u = make_qet_uuid(&format!("diag_{order}_expjunc_{jx}_{jy}"));
            let mut attrs = HashMap::new();
            attrs.insert("cade_junction".to_string(), "1".to_string());
            diagram.elements.push(QetElementInstance {
                uuid: u,
                element_type: uri.to_string(),
                x: jx as f32 * scale,
                y: jy as f32 * scale,
                orientation: 0,
                z: 6,
                prefix: "junction_4001".to_string(),
                labels: Vec::new(),
                raw_attributes: attrs,
            });
        }
    }

    diagram
}

/// Gera um UUID v4 determinístico válido para o QElectroTech (`{xxxxxxxx-xxxx-4xxx-8xxx-xxxxxxxxxxxx}`).
pub fn make_qet_uuid(seed: &str) -> String {
    let mut h1: u64 = 0xcbf29ce484222325;
    let mut h2: u64 = 0x84222325cbf29ce4;
    for (i, b) in seed.as_bytes().iter().enumerate() {
        h1 ^= (*b as u64).wrapping_add(i as u64);
        h1 = h1.wrapping_mul(0x100000001b3);
        h2 ^= (*b as u64).wrapping_mul((i as u64).wrapping_add(0x9e3779b9));
        h2 = h2.rotate_left(13).wrapping_mul(0x100000001b3);
    }
    let a = (h1 >> 32) as u32;
    let b = ((h1 >> 16) & 0xFFFF) as u16;
    let c = ((h1 & 0x0FFF) as u16) | 0x4000;
    let d = (((h2 >> 48) & 0x3FFF) as u16) | 0x8000;
    let e = h2 & 0x0000_FFFF_FFFF_FFFF;
    format!("{{{a:08x}-{b:04x}-{c:04x}-{d:04x}-{e:012x}}}")
}

fn parse_cade_native_elmt_uri(uri: &str) -> Option<(i32, i32)> {
    let name = uri.rsplit('/').next()?;
    let stem = name.strip_suffix(".elmt").unwrap_or(name);
    let rest = stem.strip_prefix("type_")?;
    if let Some((tipo_s, m_s)) = rest.split_once("_m") {
        let tipo = tipo_s.parse::<i32>().ok()?;
        let m_ori = m_s.parse::<i32>().ok()?;
        Some((tipo, m_ori))
    } else {
        let tipo = rest.parse::<i32>().ok()?;
        Some((tipo, 0))
    }
}

fn pin_local_dir(
    sym: &crate::symbols::SymbolDef,
    conn_pos: (i32, i32),
    mirror_ori: i32,
) -> &'static str {
    let (tx, ty) = crate::model::transform_offset(conn_pos.0 * 5, conn_pos.1 * 5, mirror_ori);
    let (b1x, b1y) = crate::model::transform_offset(sym.bounds.0, sym.bounds.1, mirror_ori);
    let (b2x, b2y) = crate::model::transform_offset(sym.bounds.2, sym.bounds.3, mirror_ori);
    let min_x = b1x.min(b2x);
    let max_x = b1x.max(b2x);
    let min_y = b1y.min(b2y);
    let max_y = b1y.max(b2y);
    let center_x = (min_x + max_x) as f32 / 2.0;
    let center_y = (min_y + max_y) as f32 / 2.0;

    if ty <= min_y + 5 && max_y > min_y + 15 {
        "n"
    } else if ty >= max_y - 5 && max_y > min_y + 15 {
        "s"
    } else if (tx as f32) < center_x - 10.0 {
        "w"
    } else if (tx as f32) > center_x + 10.0 {
        "e"
    } else if (ty as f32) <= center_y {
        "n"
    } else {
        "s"
    }
}

fn rotate_dir(dir: &str, qet_ori: i32) -> &'static str {
    let idx: i32 = match dir {
        "n" => 0,
        "e" => 1,
        "s" => 2,
        "w" => 3,
        _ => 0,
    };
    match (idx + qet_ori).rem_euclid(4) {
        0 => "n",
        1 => "e",
        2 => "s",
        _ => "w",
    }
}

fn is_axis_compatible(pin_dir: &str, wire_dir: &str) -> bool {
    matches!(
        (pin_dir, wire_dir),
        ("n" | "s", "n" | "s") | ("e" | "w", "e" | "w")
    )
}

fn wire_endpoint_dirs(x1: i32, y1: i32, x2: i32, y2: i32) -> (&'static str, &'static str) {
    let dx = x2 - x1;
    let dy = y2 - y1;
    if dx.abs() >= dy.abs() {
        if dx >= 0 {
            ("e", "w")
        } else {
            ("w", "e")
        }
    } else if dy >= 0 {
        ("s", "n")
    } else {
        ("n", "s")
    }
}

fn point_lies_on_any_wire_interior(px: i32, py: i32, wires: &[(i32, i32, i32, i32)]) -> bool {
    for &(x1, y1, x2, y2) in wires {
        if x1 == x2 && px == x1 {
            let min_y = y1.min(y2);
            let max_y = y1.max(y2);
            if py > min_y && py < max_y {
                return true;
            }
        } else if y1 == y2 && py == y1 {
            let min_x = x1.min(x2);
            let max_x = x1.max(x2);
            if px > min_x && px < max_x {
                return true;
            }
        }
    }
    false
}

fn default_qet_wire_color(item: &SchematicItem) -> Option<String> {
    if item.color != 0 {
        return Some(format_hex_color(item.color));
    }
    match item.tipo {
        4009 => Some("#0000ff".to_string()),
        4002 | 4010 => Some("#00aa00".to_string()),
        4003 => Some("#ff0000".to_string()),
        4004 => Some("#000080".to_string()),
        _ => Some("#800000".to_string()),
    }
}

/// Constrói o bloco `<collection>` contendo as definições `.elmt` de todos os símbolos `cade_native`
/// utilizados nos diagramas do projeto.
fn build_embedded_collection_xml(project: &QetProject) -> String {
    let mut needed_uris: Vec<(String, Option<QetElementInstance>)> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();

    for diag in &project.diagrams {
        for el in &diag.elements {
            if el.element_type.contains("cade_native/")
                && !project.element_definitions.contains_key(&el.element_type)
                && seen.insert(el.element_type.clone())
            {
                let sample = if el.element_type.contains("graphic_") {
                    Some(el.clone())
                } else {
                    None
                };
                needed_uris.push((el.element_type.clone(), sample));
            }
        }
    }

    if needed_uris.is_empty() {
        if let Some(ref raw_coll) = project.raw_collection {
            return raw_coll.clone();
        }
        return "  <collection/>\n".to_string();
    }

    let mut elements_xml = String::new();
    for (uri, sample) in &needed_uris {
        let el_name = uri.rsplit('/').next().unwrap_or("element.elmt");
        if el_name == "junction.elmt" {
            elements_xml.push_str(&synthesize_junction_elmt_definition(el_name, uri, true));
        } else if el_name == "corner.elmt" {
            elements_xml.push_str(&synthesize_junction_elmt_definition(el_name, uri, false));
        } else if let Some((cade_tipo, mirror_ori)) = parse_cade_native_elmt_uri(uri) {
            elements_xml.push_str(&synthesize_cade_elmt_definition(
                cade_tipo, mirror_ori, el_name, uri,
            ));
        } else if let Some(el_inst) = sample {
            elements_xml.push_str(&synthesize_graphic_elmt_definition(el_name, uri, el_inst));
        }
    }

    if let Some(ref raw_coll) = project.raw_collection {
        if let Some(pos) = raw_coll.rfind("</collection>") {
            let mut merged = String::with_capacity(raw_coll.len() + elements_xml.len() + 256);
            merged.push_str(&raw_coll[..pos]);
            merged.push_str("    <category name=\"import\">\n");
            merged.push_str("      <category name=\"cade_native\">\n");
            merged.push_str(&elements_xml);
            merged.push_str("      </category>\n");
            merged.push_str("    </category>\n");
            merged.push_str(&raw_coll[pos..]);
            return merged;
        }
    }

    let mut out = String::with_capacity(elements_xml.len() + 512);
    out.push_str("  <collection>\n");
    out.push_str("    <category name=\"import\">\n");
    out.push_str("      <names>\n");
    out.push_str("        <name lang=\"pt\">Elementos Importados</name>\n");
    out.push_str("        <name lang=\"en\">Imported elements</name>\n");
    out.push_str("      </names>\n");
    out.push_str("      <category name=\"cade_native\">\n");
    out.push_str("        <names>\n");
    out.push_str("          <name lang=\"pt\">RustSimu / CADe Nativo</name>\n");
    out.push_str("          <name lang=\"en\">RustSimu / CADe Native</name>\n");
    out.push_str("        </names>\n");
    out.push_str(&elements_xml);
    out.push_str("      </category>\n");
    out.push_str("    </category>\n");
    out.push_str("  </collection>\n");
    out
}

fn synthesize_junction_elmt_definition(el_name: &str, full_uri: &str, visible_dot: bool) -> String {
    let def_uuid = make_qet_uuid(&format!("def_{full_uri}"));
    let mut xml = String::new();
    xml.push_str(&format!(
        "        <element name=\"{}\">\n",
        escape_xml_attr(el_name)
    ));
    xml.push_str("          <definition version=\"0.80\" type=\"element\" link_type=\"terminal\" width=\"10\" height=\"10\" hotspot_x=\"5\" hotspot_y=\"5\">\n");
    xml.push_str(&format!("            <uuid uuid=\"{def_uuid}\"/>\n"));
    xml.push_str("            <names>\n");
    if visible_dot {
        xml.push_str("              <name lang=\"pt\">Juncao / Empalme</name>\n");
        xml.push_str("              <name lang=\"en\">Junction</name>\n");
    } else {
        xml.push_str("              <name lang=\"pt\">Canto de Condutor</name>\n");
        xml.push_str("              <name lang=\"en\">Wire Corner</name>\n");
    }
    xml.push_str("            </names>\n");
    xml.push_str("            <elementInformations/>\n");
    xml.push_str("            <informations/>\n");
    xml.push_str("            <description>\n");
    if visible_dot {
        xml.push_str("              <circle x=\"-2\" y=\"-2\" diameter=\"4\" antialias=\"true\" style=\"line-style:normal;line-weight:normal;filling:black;color:black\"/>\n");
    }
    for dir in ["n", "e", "s", "w"] {
        let t_uuid = make_qet_uuid(&format!("term_{full_uri}_dir_{dir}"));
        xml.push_str(&format!(
            "              <terminal uuid=\"{t_uuid}\" name=\"\" x=\"0\" y=\"0\" orientation=\"{dir}\" type=\"Generic\"/>\n"
        ));
    }
    xml.push_str("            </description>\n");
    xml.push_str("          </definition>\n");
    xml.push_str("        </element>\n");
    xml
}

fn synthesize_cade_elmt_definition(
    cade_tipo: i32,
    mirror_ori: i32,
    el_name: &str,
    full_uri: &str,
) -> String {
    let def_uuid = make_qet_uuid(&format!("def_{full_uri}"));
    let dict = crate::dictionary::global_concept_dictionary();
    let concept = dict.get_by_cade_tipo(cade_tipo);

    let Some(sym) = crate::symbols::find_symbol(cade_tipo) else {
        return synthesize_junction_elmt_definition(el_name, full_uri, true);
    };

    let name_pt = concept.map(|c| c.name_pt.as_str()).unwrap_or(sym.name);
    let name_en = concept
        .map(|c| c.name_en.as_str())
        .unwrap_or(sym.short_label);

    let (b1x, b1y) = crate::model::transform_offset(sym.bounds.0, sym.bounds.1, mirror_ori);
    let (b2x, b2y) = crate::model::transform_offset(sym.bounds.2, sym.bounds.3, mirror_ori);
    let mut min_x = b1x.min(b2x).min(0);
    let mut max_x = b1x.max(b2x).max(0);
    let mut min_y = b1y.min(b2y).min(0);
    let mut max_y = b1y.max(b2y).max(0);

    let mut desc_xml = String::new();

    for cmd in sym.idle_cmds {
        if let Some(prim) = crate::symbols::parse_vector_cmd(cmd) {
            match prim {
                crate::symbols::VectorPrimitive::Line { x1, y1, x2, y2 } => {
                    let (px1, py1) = crate::model::transform_offset(x1, y1, mirror_ori);
                    let (px2, py2) = crate::model::transform_offset(x2, y2, mirror_ori);
                    min_x = min_x.min(px1).min(px2);
                    max_x = max_x.max(px1).max(px2);
                    min_y = min_y.min(py1).min(py2);
                    max_y = max_y.max(py1).max(py2);
                    desc_xml.push_str(&format!(
                        "              <line x1=\"{px1}\" y1=\"{py1}\" x2=\"{px2}\" y2=\"{py2}\" end1=\"none\" end2=\"none\" length1=\"1.5\" length2=\"1.5\" style=\"line-style:normal;line-weight:normal;filling:none;color:black\" antialias=\"false\"/>\n"
                    ));
                }
                crate::symbols::VectorPrimitive::Ellipse { x1, y1, x2, y2 } => {
                    let (px1, py1) = crate::model::transform_offset(x1, y1, mirror_ori);
                    let (px2, py2) = crate::model::transform_offset(x2, y2, mirror_ori);
                    let rx = px1.min(px2);
                    let ry = py1.min(py2);
                    let rw = (px2 - px1).abs().max(1);
                    let rh = (py2 - py1).abs().max(1);
                    min_x = min_x.min(rx);
                    max_x = max_x.max(rx + rw);
                    min_y = min_y.min(ry);
                    max_y = max_y.max(ry + rh);
                    if rw == rh {
                        desc_xml.push_str(&format!(
                            "              <circle x=\"{rx}\" y=\"{ry}\" diameter=\"{rw}\" style=\"line-style:normal;line-weight:normal;filling:none;color:black\" antialias=\"true\"/>\n"
                        ));
                    } else {
                        desc_xml.push_str(&format!(
                            "              <ellipse x=\"{rx}\" y=\"{ry}\" width=\"{rw}\" height=\"{rh}\" style=\"line-style:normal;line-weight:normal;filling:none;color:black\" antialias=\"true\"/>\n"
                        ));
                    }
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
                    let arc = crate::symbols::VectorPrimitive::arc_geometry(
                        x1, y1, x2, y2, x3, y3, x4, y4,
                    );
                    if mirror_ori == 0 {
                        let cx5 = arc.cx * 5.0;
                        let cy5 = arc.cy * 5.0;
                        let rx5 = (arc.rx * 5.0).max(1.0);
                        let ry5 = (arc.ry * 5.0).max(1.0);
                        let ax = cx5 - rx5;
                        let ay = cy5 - ry5;
                        let aw = rx5 * 2.0;
                        let ah = ry5 * 2.0;
                        let start_deg = -arc.start_angle.to_degrees();
                        let angle_deg = arc.sweep_angle.to_degrees();
                        min_x = min_x.min(ax.floor() as i32);
                        max_x = max_x.max((ax + aw).ceil() as i32);
                        min_y = min_y.min(ay.floor() as i32);
                        max_y = max_y.max((ay + ah).ceil() as i32);
                        desc_xml.push_str(&format!(
                            "              <arc x=\"{ax:.1}\" y=\"{ay:.1}\" width=\"{aw:.1}\" height=\"{ah:.1}\" start=\"{start_deg:.1}\" angle=\"{angle_deg:.1}\" style=\"line-style:normal;line-weight:normal;filling:none;color:black\" antialias=\"true\"/>\n"
                        ));
                    } else {
                        let steps = 12;
                        let mut poly_attrs = String::new();
                        for i in 0..=steps {
                            let t = i as f32 / steps as f32;
                            let ang = arc.start_angle - arc.sweep_angle * t;
                            let px = (arc.cx + arc.rx * ang.cos()) * 5.0;
                            let py = (arc.cy + arc.ry * ang.sin()) * 5.0;
                            let (tx, ty) = transform_offset_f32(px, py, mirror_ori);
                            min_x = min_x.min(tx.floor() as i32);
                            max_x = max_x.max(tx.ceil() as i32);
                            min_y = min_y.min(ty.floor() as i32);
                            max_y = max_y.max(ty.ceil() as i32);
                            let idx = i + 1;
                            poly_attrs.push_str(&format!(" x{idx}=\"{tx:.1}\" y{idx}=\"{ty:.1}\""));
                        }
                        desc_xml.push_str(&format!(
                            "              <polygon{poly_attrs} closed=\"false\" style=\"line-style:normal;line-weight:normal;filling:none;color:black\" antialias=\"true\"/>\n"
                        ));
                    }
                }
                crate::symbols::VectorPrimitive::Polygon { pts } => {
                    let mut poly_attrs = String::new();
                    for (idx, (vx, vy)) in pts.iter().enumerate() {
                        let (tx, ty) = crate::model::transform_offset(*vx, *vy, mirror_ori);
                        min_x = min_x.min(tx);
                        max_x = max_x.max(tx);
                        min_y = min_y.min(ty);
                        max_y = max_y.max(ty);
                        let p_i = idx + 1;
                        poly_attrs.push_str(&format!(" x{p_i}=\"{tx}\" y{p_i}=\"{ty}\""));
                    }
                    desc_xml.push_str(&format!(
                        "              <polygon{poly_attrs} closed=\"true\" style=\"line-style:normal;line-weight:normal;filling:none;color:black\" antialias=\"true\"/>\n"
                    ));
                }
                crate::symbols::VectorPrimitive::Text { x, y, text } => {
                    let (tx, ty) = crate::model::transform_offset(x, y, mirror_ori);
                    min_x = min_x.min(tx);
                    max_x = max_x.max(tx + 20);
                    min_y = min_y.min(ty - 8);
                    max_y = max_y.max(ty + 8);
                    desc_xml.push_str(&format!(
                        "              <text x=\"{tx}\" y=\"{ty}\" text=\"{}\" font=\"Sans Serif,5,-1,5,50,0,0,0,0,0\" rotation=\"0\" color=\"#000000\"/>\n",
                        escape_xml_attr(&text)
                    ));
                }
            }
        }
    }

    for pin in &sym.pins {
        let (tx, ty) =
            crate::model::transform_offset(pin.conn_pos.0 * 5, pin.conn_pos.1 * 5, mirror_ori);
        let (lx, ly) =
            crate::model::transform_offset(pin.label_pos.0 * 5, pin.label_pos.1 * 5, mirror_ori);
        min_x = min_x.min(tx).min(lx);
        max_x = max_x.max(tx).max(lx + 10);
        min_y = min_y.min(ty).min(ly - 6);
        max_y = max_y.max(ty).max(ly + 6);

        if !pin.default_label.is_empty() {
            desc_xml.push_str(&format!(
                "              <text x=\"{lx}\" y=\"{ly}\" text=\"{}\" font=\"Sans Serif,4,-1,5,25,0,0,0,0,0\" rotation=\"0\" color=\"#000000\"/>\n",
                escape_xml_attr(pin.default_label)
            ));
        }

        let term_ori = pin_local_dir(&sym, pin.conn_pos, mirror_ori);
        let term_uuid = make_qet_uuid(&format!("term_{full_uri}_pin_{}", pin.index));
        desc_xml.push_str(&format!(
            "              <terminal uuid=\"{term_uuid}\" name=\"{}\" x=\"{tx}\" y=\"{ty}\" orientation=\"{term_ori}\" type=\"Generic\"/>\n",
            escape_xml_attr(pin.default_label)
        ));
    }

    let dyn_uuid = make_qet_uuid(&format!("dyn_def_{full_uri}_label"));
    let label_x = min_x - 24;
    let label_y = (min_y + max_y) / 2 - 4;
    desc_xml.push_str(&format!(
        "              <dynamic_text x=\"{label_x}\" y=\"{label_y}\" z=\"18\" rotation=\"0\" font=\"Sans Serif,7,-1,5,50,0,0,0,0,0\" uuid=\"{dyn_uuid}\" frame=\"false\" text_width=\"-1\" keep_visual_rotation=\"true\" Halignment=\"AlignLeft\" Valignment=\"AlignTop\" text_from=\"ElementInfo\">\n"
    ));
    desc_xml.push_str("                <text></text>\n");
    desc_xml.push_str("                <info_name>label</info_name>\n");
    desc_xml.push_str("              </dynamic_text>\n");

    let hs_x = (((-min_x + 15).max(10) + 9) / 10) * 10;
    let hs_y = (((-min_y + 15).max(10) + 9) / 10) * 10;
    let width = (((max_x + 15 + hs_x).max(20) + 9) / 10) * 10;
    let height = (((max_y + 15 + hs_y).max(20) + 9) / 10) * 10;

    let mut xml = String::with_capacity(desc_xml.len() + 512);
    xml.push_str(&format!(
        "        <element name=\"{}\">\n",
        escape_xml_attr(el_name)
    ));
    xml.push_str(&format!(
        "          <definition version=\"0.80\" type=\"element\" link_type=\"simple\" width=\"{width}\" height=\"{height}\" hotspot_x=\"{hs_x}\" hotspot_y=\"{hs_y}\">\n"
    ));
    xml.push_str(&format!("            <uuid uuid=\"{def_uuid}\"/>\n"));
    xml.push_str("            <names>\n");
    xml.push_str(&format!(
        "              <name lang=\"pt\">{}</name>\n",
        escape_xml_attr(name_pt)
    ));
    xml.push_str(&format!(
        "              <name lang=\"en\">{}</name>\n",
        escape_xml_attr(name_en)
    ));
    xml.push_str("            </names>\n");
    xml.push_str("            <elementInformations/>\n");
    xml.push_str("            <informations/>\n");
    xml.push_str("            <description>\n");
    xml.push_str(&desc_xml);
    xml.push_str("            </description>\n");
    xml.push_str("          </definition>\n");
    xml.push_str("        </element>\n");
    xml
}

fn synthesize_graphic_elmt_definition(
    el_name: &str,
    full_uri: &str,
    el_inst: &QetElementInstance,
) -> String {
    let def_uuid = make_qet_uuid(&format!("def_{full_uri}"));
    let g_tipo = el_inst
        .raw_attributes
        .get("cade_graphic_tipo")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(2);
    let x1 = el_inst
        .raw_attributes
        .get("cade_x1")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0)
        * 5;
    let y1 = el_inst
        .raw_attributes
        .get("cade_y1")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0)
        * 5;
    let x2 = el_inst
        .raw_attributes
        .get("cade_x2")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0)
        * 5;
    let y2 = el_inst
        .raw_attributes
        .get("cade_y2")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0)
        * 5;
    let line_style_raw = el_inst
        .raw_attributes
        .get("cade_extra1")
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(0);
    let qet_line_style = match line_style_raw {
        1 => "dashed",
        2 => "dotted",
        3 => "dashdotted",
        _ => "normal",
    };

    let dx = x2 - x1;
    let dy = y2 - y1;
    let min_x = 0.min(dx);
    let max_x = 0.max(dx);
    let min_y = 0.min(dy);
    let max_y = 0.max(dy);

    let hs_x = (((-min_x + 10).max(10) + 9) / 10) * 10;
    let hs_y = (((-min_y + 10).max(10) + 9) / 10) * 10;
    let width = (((max_x + 10 + hs_x).max(20) + 9) / 10) * 10;
    let height = (((max_y + 10 + hs_y).max(20) + 9) / 10) * 10;

    let shape_xml = match g_tipo {
        1 => format!(
            "              <line x1=\"0\" y1=\"0\" x2=\"{dx}\" y2=\"{dy}\" end1=\"none\" end2=\"none\" length1=\"1.5\" length2=\"1.5\" style=\"line-style:{qet_line_style};line-weight:normal;filling:none;color:black\" antialias=\"false\"/>\n"
        ),
        3 => format!(
            "              <ellipse x=\"{min_x}\" y=\"{min_y}\" width=\"{}\" height=\"{}\" style=\"line-style:{qet_line_style};line-weight:normal;filling:none;color:black\" antialias=\"true\"/>\n",
            (max_x - min_x).max(2),
            (max_y - min_y).max(2)
        ),
        _ => format!(
            "              <rect x=\"{min_x}\" y=\"{min_y}\" rx=\"0\" ry=\"0\" width=\"{}\" height=\"{}\" style=\"line-style:{qet_line_style};line-weight:normal;filling:none;color:black\" antialias=\"false\"/>\n",
            (max_x - min_x).max(2),
            (max_y - min_y).max(2)
        ),
    };

    let mut xml = String::new();
    xml.push_str(&format!(
        "        <element name=\"{}\">\n",
        escape_xml_attr(el_name)
    ));
    xml.push_str(&format!(
        "          <definition version=\"0.80\" type=\"element\" link_type=\"simple\" width=\"{width}\" height=\"{height}\" hotspot_x=\"{hs_x}\" hotspot_y=\"{hs_y}\">\n"
    ));
    xml.push_str(&format!("            <uuid uuid=\"{def_uuid}\"/>\n"));
    xml.push_str("            <names>\n");
    xml.push_str("              <name lang=\"pt\">Anotacao Vetorial</name>\n");
    xml.push_str("              <name lang=\"en\">Vector Annotation</name>\n");
    xml.push_str("            </names>\n");
    xml.push_str("            <elementInformations/>\n");
    xml.push_str("            <informations/>\n");
    xml.push_str("            <description>\n");
    xml.push_str(&shape_xml);
    xml.push_str("            </description>\n");
    xml.push_str("          </definition>\n");
    xml.push_str("        </element>\n");
    xml
}

/// Exporta todas as folhas de um projeto (`ProjectPage`) como um projeto `.qet` multipáginas.
/// Quando `existing_proj` é fornecido, preserva as dimensões, propriedades e condutores de cada folha existente,
/// atualizando apenas o conteúdo editado e criando novos diagramas apenas para folhas adicionadas.
pub fn export_pages_to_qet(
    pages: &[crate::model::ProjectPage],
    active_page_index: usize,
    path: &Path,
    existing_proj: Option<&QetProject>,
) -> Result<(), String> {
    let filename = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let mut proj = if let Some(p) = existing_proj {
        p.clone()
    } else {
        let title = pages
            .first()
            .map(|p| p.doc.title_block.title.as_str())
            .filter(|t| !t.is_empty())
            .unwrap_or("Projeto RustSimu")
            .to_string();
        QetProject {
            title,
            version: "0.90".to_string(),
            ..Default::default()
        }
    };

    let mut new_diagrams = Vec::with_capacity(pages.len());
    for (idx, page) in pages.iter().enumerate() {
        let order = (idx + 1) as i32;
        let matched_diag = page
            .origin_index
            .and_then(|orig_i| proj.diagrams.get(orig_i));
        if let Some(existing_diag) = matched_diag {
            let mut diag = existing_diag.clone();
            diag.order = order;
            update_diagram_from_document(&mut diag, &page.doc);
            if !page.name.is_empty() {
                diag.title = page.name.clone();
            }
            new_diagrams.push(diag);
        } else {
            let diagram = document_to_diagram(&page.doc, &page.name, order, &filename);
            new_diagrams.push(diagram);
        }
    }
    proj.diagrams = new_diagrams;
    proj.active_diagram_index = active_page_index.min(pages.len().saturating_sub(1));

    save_qet_file(path, &proj)
}

/// Exporta um documento `SchematicDocument` simples como um arquivo `.qet` válido.
pub fn export_doc_to_qet(doc: &SchematicDocument, path: &Path) -> Result<(), String> {
    let page = crate::model::ProjectPage::new(1, doc.title_block.title.clone(), doc.clone());
    export_pages_to_qet(&[page], 0, path, None)
}

fn escape_xml_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
