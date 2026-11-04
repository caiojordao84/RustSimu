use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;

use cade_simu_rs::symbols::{symbol_catalog, parse_vector_cmd, VectorPrimitive};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = Path::new("symbols_qet_all/cade_native");
    create_dir_all(out_dir)?;

    let catalog = symbol_catalog();
    println!("Convertendo todos os {} componentes do CADe C++ para SVG animado...", catalog.len());

    let mut count = 0;
    for sym in catalog {
        let clean_name = sym.name
            .replace('/', "-")
            .replace(' ', "_")
            .replace('(', "")
            .replace(')', "")
            .replace('+', "_")
            .to_lowercase();

        let base_filename = format!("{:04}_{}", sym.tipo, clean_name);

        // 1. Gera o SVG do estado de Repouso (idle)
        let idle_svg = generate_svg_from_cmds(
            sym.name,
            &sym.idle_cmds,
            &sym.pins,
            sym.bounds,
            "#111827",
            false,
        );
        let idle_path = out_dir.join(format!("{}.svg", base_filename));
        File::create(&idle_path)?.write_all(idle_svg.as_bytes())?;

        // 2. Se possuir comandos ativos diferentes de repouso, gera o SVG Ativo (active/animado)
        if !sym.active_cmds.is_empty() {
            let active_svg = generate_svg_from_cmds(
                &format!("{} (Ativo)", sym.name),
                &sym.active_cmds,
                &sym.pins,
                sym.bounds,
                "#dc2626", // Vermelho de acionamento elétrico
                true,
            );
            let active_path = out_dir.join(format!("{}_active.svg", base_filename));
            File::create(&active_path)?.write_all(active_svg.as_bytes())?;
        }

        count += 1;
    }

    println!("[OK] Sucesso! {} símbolos exportados com seus pares de animação em '{}'", count, out_dir.display());
    Ok(())
}

fn generate_svg_from_cmds(
    title: &str,
    cmds: &[&str],
    pins: &[cade_simu_rs::symbols::SymbolPin],
    bounds: (i32, i32, i32, i32),
    stroke_color: &str,
    is_active: bool,
) -> String {
    let margin = 10;
    let min_x = bounds.0 - margin;
    let min_y = bounds.1 - margin;
    let width = (bounds.2 - bounds.0).abs() + margin * 2;
    let height = (bounds.3 - bounds.1).abs() + margin * 2;

    let mut svg = String::new();
    svg.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    svg.push_str(&format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"{min_x} {min_y} {width} {height}\" data-generator=\"cade2svg\">\n"
    ));
    svg.push_str(&format!("  <title>{title}</title>\n"));
    svg.push_str(&format!(
        "  <g stroke=\"{stroke_color}\" stroke-width=\"1.8\" fill=\"none\" stroke-linecap=\"round\" stroke-linejoin=\"round\">\n"
    ));

    for &cmd in cmds {
        if let Some(prim) = parse_vector_cmd(cmd) {
            match prim {
                VectorPrimitive::Line { x1, y1, x2, y2 } => {
                    svg.push_str(&format!(
                        "    <line x1=\"{x1}\" y1=\"{y1}\" x2=\"{x2}\" y2=\"{y2}\" />\n"
                    ));
                }
                VectorPrimitive::Ellipse { x1, y1, x2, y2 } => {
                    let cx = (x1 + x2) as f32 * 0.5;
                    let cy = (y1 + y2) as f32 * 0.5;
                    let rx = (x2 - x1).abs() as f32 * 0.5;
                    let ry = (y2 - y1).abs() as f32 * 0.5;
                    let fill = if is_active { "#fee2e2" } else { "none" };
                    svg.push_str(&format!(
                        "    <ellipse cx=\"{cx}\" cy=\"{cy}\" rx=\"{rx}\" ry=\"{ry}\" fill=\"{fill}\" />\n"
                    ));
                }
                VectorPrimitive::Polygon { pts } => {
                    let pts_str = pts
                        .iter()
                        .map(|(x, y)| format!("{x},{y}"))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let fill = if is_active { "#fee2e2" } else { "none" };
                    svg.push_str(&format!(
                        "    <polygon points=\"{pts_str}\" fill=\"{fill}\" />\n"
                    ));
                }
                VectorPrimitive::Arc { x1, y1, x2, y2, .. } => {
                    let cx = (x1 + x2) as f32 * 0.5;
                    let cy = (y1 + y2) as f32 * 0.5;
                    let r = ((x2 - x1).abs().min((y2 - y1).abs()) as f32) * 0.45;
                    svg.push_str(&format!(
                        "    <circle cx=\"{cx}\" cy=\"{cy}\" r=\"{r}\" />\n"
                    ));
                }
                VectorPrimitive::Text { x, y, text } => {
                    svg.push_str(&format!(
                        "    <text x=\"{x}\" y=\"{y}\" font-size=\"10\" fill=\"{stroke_color}\" stroke=\"none\">{text}</text>\n"
                    ));
                }
            }
        }
    }

    // Desenha os bornes / terminais
    for pin in pins {
        let (px, py) = (pin.conn_pos.0 * 5, pin.conn_pos.1 * 5);
        svg.push_str(&format!(
            "    <circle class=\"qet-terminal\" cx=\"{px}\" cy=\"{py}\" r=\"2.5\" fill=\"#dc2626\" stroke=\"none\" data-label=\"{}\" />\n",
            pin.default_label
        ));
    }

    svg.push_str("  </g>\n");
    svg.push_str("</svg>\n");

    svg
}
