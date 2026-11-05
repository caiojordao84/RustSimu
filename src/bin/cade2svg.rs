use std::fs::{create_dir_all, File};
use std::io::Write;
use std::path::Path;

use cade_simu_rs::symbols::{generate_svg_from_cmds, symbol_catalog};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = Path::new("symbols_qet_all/cade_native");
    create_dir_all(out_dir)?;

    let catalog = symbol_catalog();
    println!(
        "Convertendo todos os {} componentes do CADe C++ para SVG animado...",
        catalog.len()
    );

    let mut count = 0;
    for sym in catalog {
        let clean_name = sym
            .name
            .replace('/', "-")
            .replace(' ', "_")
            .replace(['(', ')'], "")
            .replace('+', "_")
            .to_lowercase();

        let base_filename = format!("{:04}_{}", sym.tipo, clean_name);

        // 1. Gera o SVG do estado de Repouso (idle)
        let idle_svg = generate_svg_from_cmds(
            sym.name,
            sym.idle_cmds,
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
                sym.active_cmds,
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

    println!(
        "[OK] Sucesso! {} símbolos exportados com seus pares de animação em '{}'",
        count,
        out_dir.display()
    );
    Ok(())
}
