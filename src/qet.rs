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
        return Err("O arquivo não possui elemento raiz <project> válido do QElectroTech".to_string());
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
    if let Some(coll_node) = root.children().find(|c| c.tag_name().name() == "collection") {
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

                if let Some(def_node) = child.children().find(|c| c.tag_name().name() == "definition") {
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
    let width: f32 = def_node.attribute("width").and_then(|v| v.parse().ok()).unwrap_or(40.0);
    let height: f32 = def_node.attribute("height").and_then(|v| v.parse().ok()).unwrap_or(40.0);
    let hotspot_x: f32 = def_node.attribute("hotspot_x").and_then(|v| v.parse().ok()).unwrap_or(20.0);
    let hotspot_y: f32 = def_node.attribute("hotspot_y").and_then(|v| v.parse().ok()).unwrap_or(20.0);

    let mut terminals = Vec::new();
    let mut shapes = Vec::new();
    let mut min_term_x = f32::INFINITY;
    let mut max_term_x = f32::NEG_INFINITY;
    let mut term_top_y = f32::INFINITY;
    let mut term_bottom_y = f32::NEG_INFINITY;

    if let Some(desc_node) = def_node.children().find(|c| c.tag_name().name() == "description") {
        for child in desc_node.children() {
            if !child.is_element() {
                continue;
            }

            let style = child.attribute("style").unwrap_or("");
            let (dashed, fill) = parse_qet_style(style);

            match child.tag_name().name() {
                "line" => {
                    let x1: f32 = child.attribute("x1").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let y1: f32 = child.attribute("y1").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let x2: f32 = child.attribute("x2").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let y2: f32 = child.attribute("y2").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    shapes.push(SvgShape::Line { x1, y1, x2, y2, dashed });
                }
                "rect" => {
                    let x: f32 = child.attribute("x").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let y: f32 = child.attribute("y").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let w: f32 = child.attribute("width").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let h: f32 = child.attribute("height").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    shapes.push(SvgShape::Rect { x, y, w, h, dashed, fill });
                }
                "circle" => {
                    let x: f32 = child.attribute("x").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let y: f32 = child.attribute("y").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let diameter: f32 = child.attribute("diameter").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let r = diameter / 2.0;
                    shapes.push(SvgShape::Circle { cx: x + r, cy: y + r, r, is_terminal: false, fill });
                }
                "ellipse" => {
                    let x: f32 = child.attribute("x").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let y: f32 = child.attribute("y").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let w: f32 = child.attribute("width").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let h: f32 = child.attribute("height").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    shapes.push(SvgShape::Ellipse { cx: x + w / 2.0, cy: y + h / 2.0, rx: w / 2.0, ry: h / 2.0, fill });
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
                    let x: f32 = child.attribute("x").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let y: f32 = child.attribute("y").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let w: f32 = child.attribute("width").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let h: f32 = child.attribute("height").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let start: f32 = child.attribute("start").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let angle: f32 = child.attribute("angle").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let points = arc_to_polyline(x, y, w, h, start, angle);
                    shapes.push(SvgShape::Polyline { points });
                }
                "dynamic_text" | "text" => {
                    let x: f32 = child.attribute("x").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let y: f32 = child.attribute("y").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let rotation: f32 = child.attribute("rotation").and_then(|v| v.parse().ok()).unwrap_or(0.0);
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
                        if let Some(t_child) = child.children().find(|c| c.tag_name().name() == "text") {
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
                    let x: f32 = child.attribute("x").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let y: f32 = child.attribute("y").and_then(|v| v.parse().ok()).unwrap_or(0.0);
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

    if min_term_x.is_infinite() { min_term_x = 0.0; }
    if max_term_x.is_infinite() { max_term_x = width; }
    if term_top_y.is_infinite() { term_top_y = 0.0; }
    if term_bottom_y.is_infinite() { term_bottom_y = height; }

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

    let view_box = (-hotspot_x, -hotspot_y, width, height);

    let parsed_symbol = ParsedSvgSymbol {
        view_box,
        shapes,
        term_top_y,
        term_bottom_y,
        min_term_x,
        max_term_x,
        is_cade_svg: false,
        is_qet_native: true,
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

    let order = diag_node.attribute("order").and_then(|v| v.parse().ok()).unwrap_or(1);
    let title = diag_node.attribute("title").unwrap_or("").to_string();
    let author = diag_node.attribute("author").unwrap_or("").to_string();
    let date = diag_node.attribute("date").unwrap_or("").to_string();
    let filename = diag_node.attribute("filename").unwrap_or("").to_string();
    let rows = diag_node.attribute("rows").and_then(|v| v.parse().ok()).unwrap_or(8);
    let cols = diag_node.attribute("cols").and_then(|v| v.parse().ok()).unwrap_or(6);
    let rowsize: f32 = diag_node.attribute("rowsize").and_then(|v| v.parse().ok()).unwrap_or(80.0);
    let colsize: f32 = diag_node.attribute("colsize").and_then(|v| v.parse().ok()).unwrap_or(170.0);
    let height: f32 = diag_node.attribute("height").and_then(|v| v.parse().ok()).unwrap_or(660.0);
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
                for el_node in child.children().filter(|c| c.tag_name().name() == "element") {
                    let mut el_attrs = HashMap::new();
                    for attr in el_node.attributes() {
                        el_attrs.insert(attr.name().to_string(), attr.value().to_string());
                    }

                    let uuid = el_node.attribute("uuid").unwrap_or("").to_string();
                    let element_type = el_node.attribute("type").unwrap_or("").to_string();
                    let x: f32 = el_node.attribute("x").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let y: f32 = el_node.attribute("y").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let orientation = el_node.attribute("orientation").and_then(|v| v.parse().ok()).unwrap_or(0);
                    let z = el_node.attribute("z").and_then(|v| v.parse().ok()).unwrap_or(10);
                    let prefix = el_node.attribute("prefix").unwrap_or("").to_string();

                    let mut labels = Vec::new();
                    for info_block in el_node.children().filter(|c| c.tag_name().name() == "elementInformations") {
                        for info in info_block.children().filter(|c| c.tag_name().name() == "elementInformation") {
                            if info.attribute("name") == Some("label") {
                                let t = clean_html_text(info.text().unwrap_or(""));
                                if !t.is_empty() && !labels.contains(&t) {
                                    labels.push(t);
                                }
                            }
                        }
                    }
                    for inp in el_node.children().filter(|c| c.tag_name().name() == "input") {
                        let t = clean_html_text(inp.text().or_else(|| inp.attribute("text")).unwrap_or(""));
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
                for cond_node in child.children().filter(|c| c.tag_name().name() == "conductor") {
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
                    for seg_node in cond_node.children().filter(|c| c.tag_name().name() == "segment") {
                        let ori = seg_node.attribute("orientation").unwrap_or("horizontal").to_string();
                        let len: f32 = seg_node.attribute("length").and_then(|v| v.parse().ok()).unwrap_or(0.0);
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
                    let x: f32 = inp_node.attribute("x").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let y: f32 = inp_node.attribute("y").and_then(|v| v.parse().ok()).unwrap_or(0.0);
                    let text = clean_html_text(inp_node.attribute("text").or_else(|| inp_node.text()).unwrap_or(""));
                    let font = inp_node.attribute("font").unwrap_or("").to_string();
                    let rotation = inp_node.attribute("rotation").and_then(|v| v.parse().ok()).unwrap_or(0);

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
    let text = String::from_utf8(bytes).map_err(|e| format!("Erro UTF-8 ao ler {}: {e}", path.display()))?;
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

    let mut id_counter = 1;
    let mut elem_by_uuid: HashMap<String, &QetElementInstance> = HashMap::new();

    // 2. Elementos gráficos / componentes
    for el in &diag.elements {
        elem_by_uuid.insert(el.uuid.clone(), el);

        let mut item = SchematicItem::new_wire(
            id_counter,
            20000,
            el.x.round() as i32,
            el.y.round() as i32,
            el.x.round() as i32,
            el.y.round() as i32,
        );
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

        doc.items.push(item);
    }

    // 3. Condutores (cabos elétricos)
    for cond in &diag.conductors {
        let el1 = elem_by_uuid.get(&cond.element1);
        let el2 = elem_by_uuid.get(&cond.element2);
        if el1.is_none() || el2.is_none() {
            continue;
        }
        let el1 = el1.unwrap();
        let el2 = el2.unwrap();

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

        let w1x = (el1.x + rx1).round() as i32;
        let w1y = (el1.y + ry1).round() as i32;
        let w2x = (el2.x + rx2).round() as i32;
        let w2y = (el2.y + ry2).round() as i32;

        let wire_color_int = cond.color.as_deref().map(parse_hex_color).unwrap_or(0);
        let tipo = if wire_color_int == 0xFF0000 {
            4003 // DC+ (Vermelho)
        } else if wire_color_int == 0x010101 || wire_color_int == 0xAA5500 {
            4004 // DC- / Terra / Marrom automotivo
        } else {
            4000
        };

        if !cond.segments.is_empty() {
            let mut cx_f = el1.x + rx1;
            let mut cy_f = el1.y + ry1;
            let mut cx = w1x;
            let mut cy = w1y;
            for seg in &cond.segments {
                if seg.orientation == "horizontal" {
                    cx_f += seg.length;
                } else {
                    cy_f += seg.length;
                }
                let nx = cx_f.round() as i32;
                let ny = cy_f.round() as i32;
                if cx == nx && cy == ny {
                    continue;
                }

                let mut wire = SchematicItem::new_wire(id_counter, tipo, cx, cy, nx, ny);
                id_counter += 1;
                wire.color = wire_color_int;
                wire.trailing_str = cond.num.clone().unwrap_or_default();
                doc.items.push(wire);

                cx = nx;
                cy = ny;
            }

            // Segmento final de alinhamento com terminal 2 se houver pequeno arredondamento
            if cx != w2x || cy != w2y {
                let mut wire = SchematicItem::new_wire(id_counter, tipo, cx, cy, w2x, w2y);
                id_counter += 1;
                wire.color = wire_color_int;
                doc.items.push(wire);
            }
        } else if w1x == w2x || w1y == w2y {
            // Reta única direta
            let mut wire = SchematicItem::new_wire(id_counter, tipo, w1x, w1y, w2x, w2y);
            id_counter += 1;
            wire.color = wire_color_int;
            wire.trailing_str = cond.num.clone().unwrap_or_default();
            doc.items.push(wire);
        } else {
            // Rota ortogonal padrão em L (Manhattan)
            let mut wire1 = SchematicItem::new_wire(id_counter, tipo, w1x, w1y, w2x, w1y);
            id_counter += 1;
            wire1.color = wire_color_int;
            wire1.trailing_str = cond.num.clone().unwrap_or_default();
            doc.items.push(wire1);

            let mut wire2 = SchematicItem::new_wire(id_counter, tipo, w2x, w1y, w2x, w2y);
            id_counter += 1;
            wire2.color = wire_color_int;
            doc.items.push(wire2);
        }
    }

    // 4. Textos e anotações do diagrama
    for inp in &diag.inputs {
        let text_clean = clean_html_text(&inp.text);
        if !text_clean.is_empty() {
            let mut text_item = SchematicItem::new_wire(
                id_counter,
                8,
                inp.x.round() as i32,
                inp.y.round() as i32,
                inp.x.round() as i32,
                inp.y.round() as i32,
            );
            id_counter += 1;
            text_item.trailing_str = text_clean;
            doc.items.push(text_item);
        }
    }

    doc
}

/// Atualiza as posições e tags dos elementos em um `QetDiagram` a partir do `SchematicDocument` editado.
pub fn update_diagram_from_document(diagram: &mut QetDiagram, doc: &SchematicDocument) {
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
        xml.push_str(&format!(
            "  <diagram order=\"{}\" title=\"{}\" author=\"{}\" date=\"{}\" filename=\"{}\" rows=\"{}\" cols=\"{}\" rowsize=\"{:.1}\" colsize=\"{:.1}\" height=\"{:.1}\">\n",
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
            xml.push_str(&format!(
                "      <element type=\"{}\" x=\"{:.1}\" y=\"{:.1}\" orientation=\"{}\" uuid=\"{}\" z=\"{}\">\n",
                escape_xml_attr(&el.element_type),
                el.x,
                el.y,
                el.orientation,
                escape_xml_attr(&el.uuid),
                el.z
            ));
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
                .map(|n| format!(" num=\"{}\"", escape_xml_attr(n)))
                .unwrap_or_default();

            xml.push_str(&format!(
                "      <conductor element1=\"{}\" terminal1=\"{}\" element2=\"{}\" terminal2=\"{}\"{}{}>\n",
                escape_xml_attr(&cond.element1),
                escape_xml_attr(&cond.terminal1),
                escape_xml_attr(&cond.element2),
                escape_xml_attr(&cond.terminal2),
                color_attr,
                num_attr
            ));
            for seg in &cond.segments {
                xml.push_str(&format!(
                    "        <segment orientation=\"{}\" length=\"{:.4}\"/>\n",
                    escape_xml_attr(&seg.orientation),
                    seg.length
                ));
            }
            xml.push_str("      </conductor>\n");
        }
        xml.push_str("    </conductors>\n");

        // Inputs (Textos)
        xml.push_str("    <inputs>\n");
        for inp in &diag.inputs {
            xml.push_str(&format!(
                "      <input x=\"{:.1}\" y=\"{:.1}\" text=\"{}\" rotation=\"{}\"/>\n",
                inp.x,
                inp.y,
                escape_xml_attr(&inp.text),
                inp.rotation
            ));
        }
        xml.push_str("    </inputs>\n");

        xml.push_str("  </diagram>\n");
    }

    // Preserva a coleção embutida se existente
    if let Some(ref coll_str) = project.raw_collection {
        xml.push_str(coll_str);
        if !coll_str.ends_with('\n') {
            xml.push('\n');
        }
    } else {
        xml.push_str("  <collection/>\n");
    }

    xml.push_str("</project>\n");
    xml
}

/// Salva um `QetProject` no caminho especificado.
pub fn save_qet_file(path: &Path, project: &QetProject) -> Result<(), String> {
    let xml = serialize_qet_xml(project);
    fs::write(path, xml).map_err(|e| format!("Falha ao gravar arquivo QET {}: {e}", path.display()))
}

/// Exporta um documento `SchematicDocument` simples como um arquivo `.qet` válido.
pub fn export_doc_to_qet(doc: &SchematicDocument, path: &Path) -> Result<(), String> {
    let mut proj = QetProject {
        title: if doc.title_block.title.is_empty() {
            "Export CADe_SIMU".to_string()
        } else {
            doc.title_block.title.clone()
        },
        version: "0.100.0".to_string(),
        ..Default::default()
    };

    let mut diagram = QetDiagram {
        order: 1,
        title: proj.title.clone(),
        author: doc.title_block.drawn_by.clone(),
        date: doc.title_block.drawn_date.clone(),
        filename: path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(),
        rows: 8,
        cols: 6,
        rowsize: 80.0,
        colsize: 170.0,
        height: 660.0,
        width: 1020.0,
        ..Default::default()
    };

    for item in &doc.items {
        if item.is_wire() {
            // Converte fio CADe em condutor com segmentos
            let seg_h = item.x2 - item.x1;
            let seg_v = item.y2 - item.y1;
            let mut segs = Vec::new();
            if seg_h != 0 {
                segs.push(QetConductorSegment {
                    orientation: "horizontal".to_string(),
                    length: seg_h as f32,
                });
            }
            if seg_v != 0 {
                segs.push(QetConductorSegment {
                    orientation: "vertical".to_string(),
                    length: seg_v as f32,
                });
            }
            diagram.conductors.push(QetConductor {
                element1: format!("node_{}", item.id),
                terminal1: "1".to_string(),
                element2: format!("node_{}_end", item.id),
                terminal2: "2".to_string(),
                color: if item.color != 0 {
                    Some(format_hex_color(item.color))
                } else {
                    None
                },
                num: if !item.trailing_str.is_empty() {
                    Some(item.trailing_str.clone())
                } else {
                    None
                },
                segments: segs,
                ..Default::default()
            });
        } else if item.tipo == 8 {
            diagram.inputs.push(QetInput {
                x: item.x1 as f32,
                y: item.y1 as f32,
                text: item.trailing_str.clone(),
                font: "Sans Serif,9".to_string(),
                rotation: 0,
            });
        } else {
            let el_type = item.qet_type.clone().unwrap_or_else(|| format!("type_{}", item.tipo));
            let uuid = item.qet_uuid.clone().unwrap_or_else(|| format!("item_{}", item.id));
            diagram.elements.push(QetElementInstance {
                uuid,
                element_type: el_type,
                x: item.x1 as f32,
                y: item.y1 as f32,
                orientation: item.orientation,
                z: 10,
                prefix: String::new(),
                labels: if item.tag.is_empty() { vec![] } else { vec![item.tag.clone()] },
                ..Default::default()
            });
        }
    }

    proj.diagrams.push(diagram);
    save_qet_file(path, &proj)
}

fn escape_xml_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
