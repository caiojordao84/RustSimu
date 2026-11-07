use std::collections::HashMap;
use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use cade_simu_rs::canonical::*;

fn parse_style(style_str: &str) -> (String, String, Option<String>) {
    let mut stroke = "black".to_string();
    let mut fill = "none".to_string();
    let mut dasharray = None;

    for part in style_str.split(';') {
        let part = part.trim();
        if let Some(val) = part.strip_prefix("color:") {
            stroke = match val {
                "none" => "none".to_string(),
                "black" => "black".to_string(),
                "white" => "white".to_string(),
                "blue" => "#0055d4".to_string(),
                "red" => "#d40000".to_string(),
                "gray" => "#888888".to_string(),
                other => other.to_string(),
            };
        } else if let Some(val) = part.strip_prefix("filling:") {
            fill = match val {
                "none" => "none".to_string(),
                "black" => "black".to_string(),
                "white" => "white".to_string(),
                "blue" => "#0055d4".to_string(),
                "red" => "#d40000".to_string(),
                "green" => "#00aa00".to_string(),
                "lightgray" | "HTMLGrayGainsboro" => "#dcdcdc".to_string(),
                "HTMLBlueDeepSkyBlue" => "#00bfff".to_string(),
                "HTMLOrangeTomato" => "#ff6347".to_string(),
                other => other.to_string(),
            };
        } else if let Some(val) = part.strip_prefix("line-style:") {
            match val {
                "dashed" => dasharray = Some("4,3".to_string()),
                "dotted" => dasharray = Some("1.5,2".to_string()),
                "dashdotted" => dasharray = Some("4,2,1.5,2".to_string()),
                _ => {}
            }
        }
    }

    (stroke, fill, dasharray)
}

fn arc_to_svg_path(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    start_deg: f64,
    angle_deg: f64,
) -> String {
    let rx = width / 2.0;
    let ry = height / 2.0;
    let cx = x + rx;
    let cy = y + ry;

    let rad = std::f64::consts::PI / 180.0;
    let qet_start_rad = -start_deg * rad;
    let qet_end_rad = -(start_deg + angle_deg) * rad;

    let x1 = cx + rx * qet_start_rad.cos();
    let y1 = cy + ry * qet_start_rad.sin();
    let x2 = cx + rx * qet_end_rad.cos();
    let y2 = cy + ry * qet_end_rad.sin();

    let large_arc_flag = if angle_deg.abs() > 180.0 { 1 } else { 0 };
    let sweep_flag = if angle_deg < 0.0 { 1 } else { 0 };

    format!(
        "M {:.2} {:.2} A {:.2} {:.2} 0 {} {} {:.2} {:.2}",
        x1, y1, rx, ry, large_arc_flag, sweep_flag, x2, y2
    )
}

fn sanitize_filename(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = s.trim_matches('_');
    if trimmed.is_empty() {
        "symbol".to_string()
    } else {
        trimmed.to_lowercase()
    }
}

pub fn convert_elmt_file(
    input_path: &Path,
    output_dir: &Path,
    category: &str,
    used_filenames: &mut HashMap<String, usize>,
) -> Result<SymbolMetadata, Box<dyn std::error::Error>> {
    let mut file = File::open(input_path)?;
    let mut xml_content = String::new();
    file.read_to_string(&mut xml_content)?;

    let doc = roxmltree::Document::parse(&xml_content)?;
    let root = doc.root_element();

    if root.tag_name().name() != "definition" {
        return Err("Not a valid QET definition".into());
    }

    let width: f64 = root
        .attribute("width")
        .unwrap_or("60")
        .parse()
        .unwrap_or(60.0);
    let height: f64 = root
        .attribute("height")
        .unwrap_or("60")
        .parse()
        .unwrap_or(60.0);
    let hotspot_x: f64 = root
        .attribute("hotspot_x")
        .unwrap_or("30")
        .parse()
        .unwrap_or(30.0);
    let hotspot_y: f64 = root
        .attribute("hotspot_y")
        .unwrap_or("30")
        .parse()
        .unwrap_or(30.0);

    let mut name_en = None;
    let mut name_pt = None;

    if let Some(names) = root.children().find(|n| n.tag_name().name() == "names") {
        for name_node in names.children() {
            if name_node.tag_name().name() == "name" {
                let lang = name_node.attribute("lang").unwrap_or("");
                let text = name_node.text().unwrap_or("").trim();
                if lang == "en" && !text.is_empty() {
                    name_en = Some(text.to_string());
                } else if (lang == "pt" || lang == "pt_BR") && !text.is_empty() && name_pt.is_none()
                {
                    name_pt = Some(text.to_string());
                }
            }
        }
    }

    let file_stem = input_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("symbol");

    let final_name_en = name_en.unwrap_or_else(|| file_stem.replace('_', " "));

    let safe_stem = sanitize_filename(&final_name_en);

    // Evita sobrescrever arquivos se dois elementos tiverem o mesmo nome em inglês na mesma pasta
    let count = used_filenames.entry(safe_stem.clone()).or_insert(0);
    *count += 1;
    let unique_stem = if *count > 1 {
        format!("{}_{}", safe_stem, count)
    } else {
        safe_stem.clone()
    };

    let svg_filename = format!("{}.svg", unique_stem);
    let out_file_path = output_dir.join(&svg_filename);

    let min_x = -hotspot_x;
    let min_y = -hotspot_y;

    let mut svg = String::new();
    svg.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{:.1}\" height=\"{:.1}\" viewBox=\"{:.1} {:.1} {:.1} {:.1}\" data-name=\"{}\">\n",
        width, height, min_x, min_y, width, height, final_name_en
    ));
    svg.push_str(&format!("  <title>{}</title>\n", final_name_en));
    svg.push_str("  <g stroke=\"black\" stroke-width=\"1.5\" fill=\"none\" stroke-linecap=\"round\" stroke-linejoin=\"round\">\n");

    let mut terminals = Vec::new();

    if let Some(description) = root
        .children()
        .find(|n| n.tag_name().name() == "description")
    {
        for child in description.children() {
            if !child.is_element() {
                continue;
            }

            let style_str = child.attribute("style").unwrap_or("");
            let (stroke, fill, dash) = parse_style(style_str);
            let dash_attr = match dash {
                Some(d) => format!(" stroke-dasharray=\"{}\"", d),
                None => String::new(),
            };

            match child.tag_name().name() {
                "line" => {
                    let x1: f64 = child.attribute("x1").unwrap_or("0").parse().unwrap_or(0.0);
                    let y1: f64 = child.attribute("y1").unwrap_or("0").parse().unwrap_or(0.0);
                    let x2: f64 = child.attribute("x2").unwrap_or("0").parse().unwrap_or(0.0);
                    let y2: f64 = child.attribute("y2").unwrap_or("0").parse().unwrap_or(0.0);
                    svg.push_str(&format!(
                        "    <line x1=\"{:.2}\" y1=\"{:.2}\" x2=\"{:.2}\" y2=\"{:.2}\" stroke=\"{}\" fill=\"{}\"{} />\n",
                        x1, y1, x2, y2, stroke, fill, dash_attr
                    ));
                }
                "rect" => {
                    let x: f64 = child.attribute("x").unwrap_or("0").parse().unwrap_or(0.0);
                    let y: f64 = child.attribute("y").unwrap_or("0").parse().unwrap_or(0.0);
                    let w: f64 = child
                        .attribute("width")
                        .unwrap_or("0")
                        .parse()
                        .unwrap_or(0.0);
                    let h: f64 = child
                        .attribute("height")
                        .unwrap_or("0")
                        .parse()
                        .unwrap_or(0.0);
                    svg.push_str(&format!(
                        "    <rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" stroke=\"{}\" fill=\"{}\"{} />\n",
                        x, y, w, h, stroke, fill, dash_attr
                    ));
                }
                "ellipse" => {
                    let x: f64 = child.attribute("x").unwrap_or("0").parse().unwrap_or(0.0);
                    let y: f64 = child.attribute("y").unwrap_or("0").parse().unwrap_or(0.0);
                    let w: f64 = child
                        .attribute("width")
                        .unwrap_or("0")
                        .parse()
                        .unwrap_or(0.0);
                    let h: f64 = child
                        .attribute("height")
                        .unwrap_or("0")
                        .parse()
                        .unwrap_or(0.0);
                    let rx = w / 2.0;
                    let ry = h / 2.0;
                    let cx = x + rx;
                    let cy = y + ry;
                    svg.push_str(&format!(
                        "    <ellipse cx=\"{:.2}\" cy=\"{:.2}\" rx=\"{:.2}\" ry=\"{:.2}\" stroke=\"{}\" fill=\"{}\"{} />\n",
                        cx, cy, rx, ry, stroke, fill, dash_attr
                    ));
                }
                "circle" => {
                    let x: f64 = child.attribute("x").unwrap_or("0").parse().unwrap_or(0.0);
                    let y: f64 = child.attribute("y").unwrap_or("0").parse().unwrap_or(0.0);
                    let diameter: f64 = child
                        .attribute("diameter")
                        .unwrap_or("0")
                        .parse()
                        .unwrap_or(0.0);
                    let r = diameter / 2.0;
                    let cx = x + r;
                    let cy = y + r;
                    svg.push_str(&format!(
                        "    <circle cx=\"{:.2}\" cy=\"{:.2}\" r=\"{:.2}\" stroke=\"{}\" fill=\"{}\"{} />\n",
                        cx, cy, r, stroke, fill, dash_attr
                    ));
                }
                "polygon" => {
                    let mut points = Vec::new();
                    let mut i = 1;
                    while let (Some(x), Some(y)) = (
                        child.attribute(format!("x{}", i).as_str()),
                        child.attribute(format!("y{}", i).as_str()),
                    ) {
                        points.push(format!("{},{}", x, y));
                        i += 1;
                    }
                    let is_closed = child.attribute("closed").unwrap_or("true") == "true";
                    if !points.is_empty() {
                        if is_closed {
                            svg.push_str(&format!(
                                "    <polygon points=\"{}\" stroke=\"{}\" fill=\"{}\"{} />\n",
                                points.join(" "),
                                stroke,
                                fill,
                                dash_attr
                            ));
                        } else {
                            svg.push_str(&format!(
                                "    <polyline points=\"{}\" stroke=\"{}\" fill=\"{}\"{} />\n",
                                points.join(" "),
                                stroke,
                                fill,
                                dash_attr
                            ));
                        }
                    }
                }
                "arc" => {
                    let x: f64 = child.attribute("x").unwrap_or("0").parse().unwrap_or(0.0);
                    let y: f64 = child.attribute("y").unwrap_or("0").parse().unwrap_or(0.0);
                    let w: f64 = child
                        .attribute("width")
                        .unwrap_or("0")
                        .parse()
                        .unwrap_or(0.0);
                    let h: f64 = child
                        .attribute("height")
                        .unwrap_or("0")
                        .parse()
                        .unwrap_or(0.0);
                    let start: f64 = child
                        .attribute("start")
                        .unwrap_or("0")
                        .parse()
                        .unwrap_or(0.0);
                    let angle: f64 = child
                        .attribute("angle")
                        .unwrap_or("0")
                        .parse()
                        .unwrap_or(0.0);
                    let d = arc_to_svg_path(x, y, w, h, start, angle);
                    svg.push_str(&format!(
                        "    <path d=\"{}\" data-arc=\"{:.2},{:.2},{:.2},{:.2},{:.2},{:.2}\" stroke=\"{}\" fill=\"none\"{} />\n",
                        d, x, y, w, h, start, angle, stroke, dash_attr
                    ));
                }
                "dynamic_text" | "text" => {
                    let x: f64 = child.attribute("x").unwrap_or("0").parse().unwrap_or(0.0);
                    let y: f64 = child.attribute("y").unwrap_or("0").parse().unwrap_or(0.0);
                    let rotation: f64 = child
                        .attribute("rotation")
                        .and_then(|r| r.parse().ok())
                        .unwrap_or(0.0);
                    let size: f64 = child
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
                        let escaped = text_str
                            .replace('&', "&amp;")
                            .replace('<', "&lt;")
                            .replace('>', "&gt;")
                            .replace('"', "&quot;");
                        let transform_attr = if rotation.abs() > 0.001 {
                            format!(" transform=\"rotate({:.1} {:.2} {:.2})\"", rotation, x, y)
                        } else {
                            String::new()
                        };
                        svg.push_str(&format!(
                            "    <text x=\"{:.2}\" y=\"{:.2}\" font-size=\"{:.1}\" font-family=\"sans-serif\" fill=\"black\" stroke=\"none\"{}>{}</text>\n",
                            x, y, size, transform_attr, escaped
                        ));
                    }
                }
                "terminal" => {
                    let x: f64 = child.attribute("x").unwrap_or("0").parse().unwrap_or(0.0);
                    let y: f64 = child.attribute("y").unwrap_or("0").parse().unwrap_or(0.0);
                    let name = child.attribute("name").unwrap_or("").to_string();
                    let orientation = child.attribute("orientation").unwrap_or("n").to_string();

                    terminals.push(TerminalMetadata {
                        name: name.clone(),
                        x,
                        y,
                        orientation: orientation.clone(),
                    });

                    svg.push_str(&format!(
                        "    <circle class=\"qet-terminal\" cx=\"{:.2}\" cy=\"{:.2}\" r=\"1.8\" fill=\"red\" stroke=\"none\" data-name=\"{}\" data-orientation=\"{}\" />\n",
                        x, y, name, orientation
                    ));
                }
                _ => {}
            }
        }
    }

    svg.push_str("  </g>\n");
    svg.push_str("</svg>\n");

    let mut out_file = File::create(&out_file_path)?;
    out_file.write_all(svg.as_bytes())?;

    Ok(SymbolMetadata {
        id: unique_stem,
        name_en: final_name_en,
        name_pt,
        category: category.to_string(),
        svg_file: svg_filename,
        width,
        height,
        hotspot_x,
        hotspot_y,
        terminals,
        source: Some("qet".to_string()),
        state: None,
        variant_of: None,
    })
}

fn process_category_recursive(
    input_cat_dir: &Path,
    output_base_dir: &Path,
    category_slug: &str,
    catalog: &mut Vec<SymbolMetadata>,
    used_filenames: &mut HashMap<String, usize>,
) -> usize {
    let out_dir = output_base_dir.join(category_slug);
    if let Err(e) = fs::create_dir_all(&out_dir) {
        eprintln!("Failed to create output dir {:?}: {}", out_dir, e);
        return 0;
    }

    let mut count = 0;
    let mut entries = Vec::new();
    if let Ok(rd) = fs::read_dir(input_cat_dir) {
        for entry in rd.flatten() {
            entries.push(entry.path());
        }
    }
    entries.sort();

    for path in entries {
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("elmt") {
            match convert_elmt_file(&path, &out_dir, category_slug, used_filenames) {
                Ok(meta) => {
                    count += 1;
                    catalog.push(meta);
                }
                Err(e) => {
                    eprintln!("Error converting {:?}: {}", path, e);
                }
            }
        } else if path.is_dir() {
            let sub_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("sub");
            let sub_slug = format!("{}/{}", category_slug, sanitize_filename(sub_name));
            count += process_category_recursive(
                &path,
                output_base_dir,
                &sub_slug,
                catalog,
                used_filenames,
            );
        }
    }

    count
}

/// Converte um stem de arquivo em nome legível: `brake_light_switch_001` → `Brake light switch`.
/// O sufixo numérico de desambiguação (`_001`) é removido apenas do nome de exibição;
/// o `id` continua sendo o stem completo.
fn humanize_stem(stem: &str) -> String {
    let mut base = stem;
    if let Some((head, tail)) = stem.rsplit_once('_') {
        if !head.is_empty() && !tail.is_empty() && tail.chars().all(|c| c.is_ascii_digit()) {
            base = head;
        }
    }
    let spaced = base.replace('_', " ");
    let spaced = spaced.trim();
    let mut chars = spaced.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Remove sufixos de estado que alguns geradores colocam no `<title>`
/// (`Motor ... (Ativo)`, `Push-button (Pressed)`), pois o estado já é codificado no nome do arquivo.
fn strip_state_suffix(title: &str) -> String {
    let mut out = title.trim().to_string();
    for suffix in [" (Ativo)", " (Active)", " (Pressed)", " (Acionado)"] {
        if let Some(stripped) = out.strip_suffix(suffix) {
            out = stripped.trim_end().to_string();
        }
    }
    out
}

/// Extrai metadados de um SVG já convertido, sem depender da coleção QElectroTech.
/// Usado pelo modo `--index-only` para catalogar arquivos gerados por `cade2svg` e importações.
fn parse_svg_metadata(content: &str, stem: &str, category: &str) -> Option<SymbolMetadata> {
    let doc = roxmltree::Document::parse(content).ok()?;
    let root = doc.root_element();

    let width: f64 = root.attribute("width")?.parse().ok()?;
    let height: f64 = root.attribute("height")?.parse().ok()?;

    // O viewBox usa o hotspot como origem negativa: `min_x = -hotspot_x`.
    let (hotspot_x, hotspot_y) = root
        .attribute("viewBox")
        .and_then(|vb| {
            let parts: Vec<f64> = vb
                .split_whitespace()
                .filter_map(|v| v.parse::<f64>().ok())
                .collect();
            if parts.len() == 4 {
                Some((-parts[0], -parts[1]))
            } else {
                None
            }
        })
        .unwrap_or((width / 2.0, height / 2.0));

    let title = doc
        .descendants()
        .find(|n| n.has_tag_name("title"))
        .and_then(|n| n.text())
        .map(strip_state_suffix)
        .filter(|t| !t.is_empty());

    let mut terminals = Vec::new();
    for node in doc.descendants().filter(|n| n.has_tag_name("circle")) {
        let is_terminal = node
            .attribute("class")
            .map(|c| c.split_whitespace().any(|k| k == "qet-terminal"))
            .unwrap_or(false);
        if !is_terminal {
            continue;
        }
        terminals.push(TerminalMetadata {
            name: node
                .attribute("data-name")
                .or_else(|| node.attribute("data-label"))
                .unwrap_or("")
                .to_string(),
            x: node
                .attribute("cx")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0),
            y: node
                .attribute("cy")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0.0),
            orientation: node
                .attribute("data-orientation")
                .unwrap_or("n")
                .to_string(),
        });
    }

    let is_active = stem.ends_with("_active");
    let base_stem = stem.strip_suffix("_active").unwrap_or(stem);
    let is_native = root.attribute("data-generator") == Some("cade2svg");

    let display = title.unwrap_or_else(|| humanize_stem(base_stem));
    // `cade_native` deriva de `symbols.rs`, cujo catálogo é em pt-BR: o título já é o nome pt.
    // Para os demais, o título/stem é em inglês e a tradução fica pendente (name_pt = None).
    let (name_en, name_pt) = if is_native {
        (display.clone(), Some(display))
    } else {
        (display, None)
    };

    Some(SymbolMetadata {
        id: stem.to_string(),
        name_en,
        name_pt,
        category: category.to_string(),
        svg_file: format!("{}.svg", stem),
        width,
        height,
        hotspot_x,
        hotspot_y,
        terminals,
        source: Some(if is_native { "cade_native" } else { "qet" }.to_string()),
        state: if is_active {
            Some("active".to_string())
        } else {
            None
        },
        variant_of: if is_active {
            Some(base_stem.to_string())
        } else {
            None
        },
    })
}

fn index_svg_file(path: &Path, category: &str) -> Option<SymbolMetadata> {
    let content = fs::read_to_string(path).ok()?;
    let stem = path.file_stem()?.to_str()?;
    parse_svg_metadata(&content, stem, category)
}

fn collect_svg_paths(dir: &Path, out: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_svg_paths(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("svg") {
            out.push(path);
        }
    }
    Ok(())
}

/// Garante unicidade **global** do `id`. O catálogo histórico foi construído em várias
/// execuções do conversor (uma por categoria), então a deduplicação por nome só valia
/// dentro de cada execução e deixou 2 colisões (`bell_2`, `temperature_sensor_2`).
fn normalize_ids(catalog: &mut [SymbolMetadata]) -> usize {
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut renamed = 0usize;

    for m in catalog.iter_mut() {
        if seen.insert(m.id.clone()) {
            continue;
        }
        let base = m.id.clone();
        let mut n = 2;
        let mut candidate = format!("{}_{}", base, n);
        while seen.contains(&candidate) {
            n += 1;
            candidate = format!("{}_{}", base, n);
        }
        seen.insert(candidate.clone());
        m.id = candidate;
        renamed += 1;
    }

    renamed
}

/// Modo `--index-only`: indexa todo SVG da árvore que ainda não tenha entrada no `catalog.json`.
/// Não requer a coleção QElectroTech instalada e é idempotente (rodar duas vezes não duplica).
fn index_only_mode(output_base: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let catalog_path = output_base.join("catalog.json");
    let mut catalog: Vec<SymbolMetadata> = if catalog_path.exists() {
        serde_json::from_str(&fs::read_to_string(&catalog_path)?)?
    } else {
        Vec::new()
    };

    let mut known: std::collections::HashSet<(String, String)> = catalog
        .iter()
        .map(|m| (m.category.clone(), m.svg_file.clone()))
        .collect();
    let mut ids: std::collections::HashSet<String> = catalog.iter().map(|m| m.id.clone()).collect();

    let mut svgs = Vec::new();
    collect_svg_paths(output_base, &mut svgs)?;
    svgs.sort();

    let mut added = 0usize;
    let mut failed = 0usize;

    for path in svgs {
        let rel_dir = path
            .parent()
            .and_then(|p| p.strip_prefix(output_base).ok())
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let file = path
            .file_name()
            .and_then(|f| f.to_str())
            .unwrap_or_default()
            .to_string();

        if known.contains(&(rel_dir.clone(), file.clone())) {
            continue;
        }

        match index_svg_file(&path, &rel_dir) {
            Some(mut meta) => {
                // Garante unicidade global do id (o catálogo histórico já tinha 2 colisões).
                if ids.contains(&meta.id) {
                    let base = meta.id.clone();
                    let mut n = 2;
                    while ids.contains(&format!("{}_{}", base, n)) {
                        n += 1;
                    }
                    meta.id = format!("{}_{}", base, n);
                }
                ids.insert(meta.id.clone());
                known.insert((rel_dir.clone(), file));
                catalog.push(meta);
                added += 1;
            }
            None => {
                eprintln!("Falha ao indexar {:?}", path);
                failed += 1;
            }
        }
    }

    catalog.sort_by(|a, b| {
        (a.category.as_str(), a.id.as_str()).cmp(&(b.category.as_str(), b.id.as_str()))
    });

    let renamed = normalize_ids(&mut catalog);
    if renamed > 0 {
        // Reordena após renomear para manter a ordenação determinística.
        catalog.sort_by(|a, b| {
            (a.category.as_str(), a.id.as_str()).cmp(&(b.category.as_str(), b.id.as_str()))
        });
    }

    fs::write(&catalog_path, serde_json::to_vec_pretty(&catalog)?)?;

    println!("==================================================");
    println!("Novos SVGs indexados: {}", added);
    println!("Falhas de indexação: {}", failed);
    println!("IDs duplicados renomeados: {}", renamed);
    println!("Total no catálogo: {}", catalog.len());
    println!("Catálogo: {:?}", catalog_path);

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let index_only = args.iter().any(|a| a == "--index-only");
    let check_mode = args.iter().any(|a| a == "--check");
    let migrate_mode = args
        .iter()
        .any(|a| a == "--migrate-canonical" || a == "--migrate");
    let positional: Vec<&String> = args.iter().filter(|a| !a.starts_with("--")).collect();

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let default_output = manifest_dir.join("symbols_qet_all");
    let canonical_dir = manifest_dir.join("symbols_qet");

    if check_mode {
        let target = if !positional.is_empty() {
            PathBuf::from(positional[0])
        } else {
            canonical_dir
        };
        let summary = check_canonical_catalog(&target)?;
        println!("================== VERIFICAÇÃO D2 ==================");
        println!("Schema Version: {}", summary.schema_version);
        println!("Categorias: {}", summary.categories_count);
        println!("Elementos: {}", summary.elements_count);
        println!("SVGs em disco verificados: {}", summary.svgs_checked);
        println!("Avisos: {}", summary.warnings.len());
        println!("Erros: {}", summary.errors.len());

        if !summary.warnings.is_empty() {
            println!("Primeiros avisos:");
            for w in summary.warnings.iter().take(5) {
                println!("  [WARN] {}", w);
            }
        }

        if !summary.errors.is_empty() {
            println!("Primeiros erros:");
            for e in summary.errors.iter().take(10) {
                println!("  [ERRO] {}", e);
            }
            return Err(
                format!("Verificação D2 falhou com {} erros.", summary.errors.len()).into(),
            );
        }

        println!("✅ TODAS AS INVARIANTES D2 APROVADAS COM SUCESSO!");
        return Ok(());
    }

    if migrate_mode {
        let src = if !positional.is_empty() {
            PathBuf::from(positional[0])
        } else {
            default_output
        };
        let dst = if positional.len() > 1 {
            PathBuf::from(positional[1])
        } else {
            canonical_dir
        };
        return migrate_to_canonical(&src, &dst);
    }

    let output_base = if positional.len() > 1 {
        PathBuf::from(positional[1])
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("symbols_qet_all")
    };

    if index_only {
        return index_only_mode(&output_base);
    }

    let qet_base = if !positional.is_empty() {
        PathBuf::from(positional[0])
    } else {
        PathBuf::from(
            "/tmp/.mount_QElectDhPJcI/usr/share/qelectrotech/elements/10_electric/10_allpole",
        )
    };

    println!("=== QElectroTech .elmt to SVG Batch Converter ===");
    println!("Source: {:?}", qet_base);
    println!("Destination: {:?}", output_base);

    fs::create_dir_all(&output_base)?;

    let mut catalog = Vec::new();
    let mut used_filenames = HashMap::new();

    // Se a fonte for a raiz de elementos ou uma pasta específica
    let mut total_converted = 0;

    let subdirs = fs::read_dir(&qet_base)?;
    let mut paths: Vec<_> = subdirs.flatten().map(|e| e.path()).collect();
    paths.sort();

    for path in paths {
        if path.is_dir() {
            let dir_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("cat");
            // Remove prefixos numéricos como '10_', '200_', etc. para um slug limpo em inglês
            let clean_slug = dir_name
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == '_')
                .to_string();
            let slug = if clean_slug.is_empty() {
                dir_name.to_string()
            } else {
                clean_slug
            };

            print!("Converting category '{}' ... ", slug);
            let n = process_category_recursive(
                &path,
                &output_base,
                &slug,
                &mut catalog,
                &mut used_filenames,
            );
            println!("{} symbols converted.", n);
            total_converted += n;
        }
    }

    let catalog_json_path = output_base.join("catalog.json");
    let json_bytes = serde_json::to_vec_pretty(&catalog)?;
    fs::write(&catalog_json_path, json_bytes)?;

    println!("==================================================");
    println!("Conversion completed! Total symbols: {}", total_converted);
    println!("Index catalog generated at: {:?}", catalog_json_path);

    // Garante que SVGs pré-existentes (cade_native, importações, variantes `_active`)
    // também fiquem indexados: nenhum arquivo em disco pode ficar fora do catálogo.
    index_only_mode(&output_base)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SVG_BASE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="20.0" height="60.0" viewBox="-12.0 -30.0 20.0 60.0">
  <title>Push-button</title>
  <circle class="qet-terminal" cx="0.00" cy="-10.00" r="1.8" fill="red" data-name="13" data-orientation="n" />
  <circle class="qet-terminal" cx="0.00" cy="10.00" r="1.8" fill="red" data-label="14" />
</svg>"#;

    #[test]
    fn humaniza_stem_removendo_sufixo_numerico() {
        assert_eq!(
            humanize_stem("brake_light_switch_001"),
            "Brake light switch"
        );
        assert_eq!(humanize_stem("coming_arrow"), "Coming arrow");
        assert_eq!(humanize_stem("1"), "1");
        assert_eq!(humanize_stem(""), "");
    }

    #[test]
    fn remove_sufixo_de_estado_do_titulo() {
        assert_eq!(strip_state_suffix("Motor (Ativo)"), "Motor");
        assert_eq!(strip_state_suffix("Push-button (Pressed)"), "Push-button");
        assert_eq!(strip_state_suffix("Motor trifásico"), "Motor trifásico");
    }

    #[test]
    fn indexa_svg_com_viewbox_terminais_e_hotspot() {
        let meta = parse_svg_metadata(SVG_BASE, "push-button_3", "electric/x")
            .expect("SVG válido deve ser indexado");

        assert_eq!(meta.id, "push-button_3");
        assert_eq!(meta.svg_file, "push-button_3.svg");
        assert_eq!(meta.name_en, "Push-button");
        assert_eq!(meta.name_pt, None);
        assert_eq!(meta.width, 20.0);
        assert_eq!(meta.height, 60.0);
        assert_eq!(meta.hotspot_x, 12.0);
        assert_eq!(meta.hotspot_y, 30.0);
        assert_eq!(meta.source.as_deref(), Some("qet"));
        assert_eq!(meta.state, None);
        assert_eq!(meta.variant_of, None);
        assert_eq!(meta.terminals.len(), 2);
        assert_eq!(meta.terminals[0].name, "13");
        assert_eq!(meta.terminals[0].orientation, "n");
        assert_eq!(meta.terminals[1].name, "14");
        assert_eq!(meta.terminals[1].orientation, "n");
    }

    #[test]
    fn variante_active_declara_estado_e_origem() {
        let meta = parse_svg_metadata(SVG_BASE, "push-button_3_active", "electric/x")
            .expect("SVG válido deve ser indexado");

        assert_eq!(meta.state.as_deref(), Some("active"));
        assert_eq!(meta.variant_of.as_deref(), Some("push-button_3"));
    }

    #[test]
    fn svg_nativo_usa_titulo_como_nome_pt() {
        let svg = SVG_BASE
            .replace("<svg xmlns", "<svg data-generator=\"cade2svg\" xmlns")
            .replace("Push-button", "Motor Assíncrono Trifásico");
        let meta = parse_svg_metadata(&svg, "1000_motor", "cade_native")
            .expect("SVG válido deve ser indexado");

        assert_eq!(meta.source.as_deref(), Some("cade_native"));
        assert_eq!(meta.name_pt.as_deref(), Some("Motor Assíncrono Trifásico"));
        assert_eq!(meta.name_en, "Motor Assíncrono Trifásico");
    }
}
