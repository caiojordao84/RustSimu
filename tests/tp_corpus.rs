//! Corpus `TP*.ct` em `/home/caiojordao84/Downloads/ct`: abrir + simular.
//!
//! Variáveis:
//! - `CT_CORPUS_DIR` — raiz (default `/home/caiojordao84/Downloads/ct`)
//! - `CT_CORPUS_REPORT` — caminho do relatório markdown gerado

use cade_simu_rs::ct::{decifra_ct_bytes, schematic_from_ct_path};
use cade_simu_rs::sim::step_simulation;
use ct_parser::adaptador::adaptar;
use ct_parser::Pagina;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct Fail {
    rel: String,
    stage: &'static str,
    reason: String,
}

fn collect_tp_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                let lower = name.to_ascii_lowercase();
                if lower.starts_with("tp") && lower.ends_with(".ct") {
                    out.push(p);
                }
            }
        }
    }
    walk(root, &mut out);
    out.sort();
    out
}

fn rel_to(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn empty_import_detail(path: &Path) -> String {
    let Ok(bruto) = fs::read(path) else {
        return "documento vazio (0 itens); não foi possível reler o .ct".into();
    };
    let Ok(dec) = decifra_ct_bytes(&bruto) else {
        return "documento vazio (0 itens); falha ao re-decifrar".into();
    };
    let Ok(pagina) = Pagina::parse(&dec.corpo) else {
        return "documento vazio (0 itens); falha no parse do corpo".into();
    };
    let ad = adaptar(&pagina);
    let mut ign: BTreeMap<String, usize> = BTreeMap::new();
    for (_, t) in &ad.ignorados {
        *ign.entry(t.clone()).or_default() += 1;
    }
    let ign_s = ign
        .iter()
        .map(|(k, n)| format!("{k}×{n}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "documento vazio (0 itens): objetos={} ignorados=[{}] — tipos sem `simular`/catálogo v1 (ex.: Generator/VMeter/LoopController/pol*)",
        pagina.objetos.len(),
        ign_s
    )
}

fn classify_reason(stage: &str, msg: &str) -> String {
    let m = msg.to_ascii_lowercase();
    let bucket = if m.contains("compress") || m.contains("lz77") || m.contains("magica") {
        "decifragem/LZ77"
    } else if m.contains("parse corpo") || m.contains("parse") {
        "parse do corpo .ct"
    } else if m.contains("panic") || m.contains("index") || m.contains("bound") {
        "panic na simulação"
    } else if stage == "import" {
        "importação"
    } else {
        "simulação"
    };
    format!("[{bucket}] {msg}")
}

#[test]
fn tp_corpus_abrir_e_simular() {
    let root = PathBuf::from(
        std::env::var("CT_CORPUS_DIR").unwrap_or_else(|_| "/home/caiojordao84/Downloads/ct".into()),
    );
    if !root.is_dir() {
        eprintln!("CT_CORPUS_DIR ausente: {}; skip", root.display());
        return;
    }

    let files = collect_tp_files(&root);
    assert!(!files.is_empty(), "nenhum TP*.ct em {}", root.display());

    let mut ok = 0usize;
    let mut fails: Vec<Fail> = Vec::new();

    for path in &files {
        let rel = rel_to(&root, path);
        let import = std::panic::catch_unwind(|| schematic_from_ct_path(path));
        let mut doc = match import {
            Ok(Ok(d)) => d,
            Ok(Err(e)) => {
                fails.push(Fail {
                    rel,
                    stage: "import",
                    reason: classify_reason("import", &e),
                });
                continue;
            }
            Err(payload) => {
                let msg = payload
                    .downcast_ref::<String>()
                    .map(|s| s.as_str())
                    .or_else(|| payload.downcast_ref::<&str>().copied())
                    .unwrap_or("panic sem mensagem");
                fails.push(Fail {
                    rel,
                    stage: "import",
                    reason: classify_reason("import", &format!("panic: {msg}")),
                });
                continue;
            }
        };

        if doc.items.is_empty() {
            let detail = empty_import_detail(path);
            fails.push(Fail {
                rel,
                stage: "import",
                reason: classify_reason("import", &detail),
            });
            continue;
        }

        let sim = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            for _ in 0..5 {
                step_simulation(&mut doc, 80);
            }
        }));
        if let Err(payload) = sim {
            let msg = payload
                .downcast_ref::<String>()
                .map(|s| s.as_str())
                .or_else(|| payload.downcast_ref::<&str>().copied())
                .unwrap_or("panic sem mensagem");
            fails.push(Fail {
                rel,
                stage: "sim",
                reason: classify_reason("sim", &format!("panic: {msg}")),
            });
            continue;
        }
        ok += 1;
    }

    let report_path = PathBuf::from(std::env::var("CT_CORPUS_REPORT").unwrap_or_else(|_| {
        "/mnt/Dados/Downloads/FluidSim Pneumatic/FluidSim Pneumatic/integracao-rustsimu/analise/tp_corpus_report.md"
            .into()
    }));
    if let Some(parent) = report_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let mut md = String::new();
    md.push_str("# Corpus TP — abrir + simular (RustSimu)\n\n");
    md.push_str(&format!("Raiz: `{}`\n\n", root.display()));
    md.push_str(&format!(
        "- Total: **{}**\n- OK: **{}**\n- Falhas: **{}**\n\n",
        files.len(),
        ok,
        fails.len()
    ));
    if fails.is_empty() {
        md.push_str("Todos os `TP*.ct` abriram e simularam sem erro.\n");
    } else {
        md.push_str("## Falhas\n\n");
        md.push_str("| Arquivo | Estágio | Motivo |\n|---|---|---|\n");
        for f in &fails {
            let reason = f.reason.replace('|', "\\|").replace('\n', " ");
            md.push_str(&format!("| `{}` | {} | {} |\n", f.rel, f.stage, reason));
        }
        md.push_str("\n## Motivos únicos (agrupados)\n\n");
        let mut by_reason: std::collections::BTreeMap<String, Vec<String>> =
            std::collections::BTreeMap::new();
        for f in &fails {
            by_reason
                .entry(f.reason.clone())
                .or_default()
                .push(f.rel.clone());
        }
        for (reason, files) in &by_reason {
            md.push_str(&format!("### {}\n\n", reason.replace('|', "\\|")));
            md.push_str(&format!("- Ocorrências: **{}**\n", files.len()));
            for file in files {
                md.push_str(&format!("  - `{file}`\n"));
            }
            md.push('\n');
        }
    }
    fs::write(&report_path, &md).expect("escrever relatório");
    eprintln!("{}", md);

    // Não falha o cargo test: o relatório + TODO.md são a entrega de triagem.
    // Use CT_CORPUS_STRICT=1 para exigir 100%.
    if std::env::var("CT_CORPUS_STRICT").ok().as_deref() == Some("1") {
        assert!(
            fails.is_empty(),
            "{}/{} TP falharam — ver {}",
            fails.len(),
            files.len(),
            report_path.display()
        );
    }
}
