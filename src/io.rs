use std::fs;
use std::io::Write;
use std::path::Path;

use crate::model::{
    DocumentSettings, ProjectDocument, ProjectPage, SchematicDocument, SchematicItem, TitleBlock,
};
use crate::symbols::find_symbol;

/// Serializa um `SchematicDocument` para o formato legado exato `.cad` do `CADe_SIMU.exe`
/// (`OnFileSave` em `0x00431600 - 0x00432343`).
pub fn serialize_cad_str(doc: &SchematicDocument) -> String {
    let mut out = String::from("CADe_SIMU");

    for item in &doc.items {
        if item.tipo < 1 && item.tipo != -1 {
            continue;
        }
        let extended = SchematicItem::is_extended_cad_type(item.tipo);

        // *id*tipo
        out.push_str(&format!("*{}*{}", item.id, item.tipo));

        // #tag#value#p1..#p8 (e #p9 se estendido)
        out.push('#');
        out.push_str(&item.tag);
        out.push('#');
        out.push_str(&item.value);
        let num_pins = if extended { 9 } else { 8 };
        for i in 0..num_pins {
            out.push('#');
            out.push_str(&item.pins[i]);
        }

        // *vis1..*vis8 (e *vis9 se estendido)
        for i in 0..num_pins {
            out.push_str(&format!("*{}", item.pin_visibility[i]));
        }

        // *x1*y1*x2*y2*aux0*aux1*aux2*aux3*color*orientation*subvariant*show_tag*show_value*extra1*extra2
        out.push_str(&format!(
            "*{}*{}*{}*{}*{}*{}*{}*{}*{}*{}*{}*{}*{}*{}*{}",
            item.x1,
            item.y1,
            item.x2,
            item.y2,
            item.aux_coords[0],
            item.aux_coords[1],
            item.aux_coords[2],
            item.aux_coords[3],
            item.color,
            item.orientation,
            item.subvariant,
            item.show_tag,
            item.show_value,
            item.extra_int1,
            item.extra_int2,
        ));

        // #trailing_str
        out.push('#');
        out.push_str(&item.trailing_str);
    }

    // Separador de fim dos itens
    out.push_str("$$$");

    // Configurações do documento
    let s = &doc.settings;
    out.push_str(&format!(
        "*{}*{}*{}*{}*{}*{}*{}*{}*{}*{:3}*{:3}&&&",
        s.format_code,
        s.format_orientation,
        s.border_ref,
        s.show_pin_refs,
        s.line_thickness,
        s.color_mode,
        s.sim_speed,
        s.auto_number,
        s.displace_lines,
        s.custom_height_mm,
        s.custom_width_mm,
    ));

    // Cajetín (Bloco de legenda)
    let tb = &doc.title_block;
    out.push_str(&format!(
        "*{:<11}*{:<11}*{:<11}*{:<11}*{:<25}*{:<25}*{:<11}*{:<11}*{:<11}*{:<21}",
        truncate_str(&tb.drawn_date, 11),
        truncate_str(&tb.drawn_by, 11),
        truncate_str(&tb.checked_date, 11),
        truncate_str(&tb.checked_by, 11),
        truncate_str(&tb.entity, 25),
        truncate_str(&tb.title, 25),
        truncate_str(&tb.subtitle, 11),
        truncate_str(&tb.sheet, 11),
        truncate_str(&tb.file_ref, 11),
        truncate_str(&tb.signature, 21),
    ));

    // 14 flags de 1 dígito + trailer
    for flag in &s.view_flags {
        out.push_str(&format!("*{:1}", flag.clamp(&0, &9)));
    }
    out.push_str("***$$$&&&***$$$&&&");
    out
}

fn truncate_str(s: &str, max_chars: usize) -> String {
    s.chars().take(max_chars).collect()
}

/// Faz o parse de um arquivo `.cad` legado do `CADe_SIMU.exe`
/// (`OnFileOpen` em `0x0042c290 - 0x00431400`).
pub fn parse_cad_str(input: &str) -> Result<SchematicDocument, String> {
    let trimmed = input.trim_start_matches('\u{feff}').trim();
    if !trimmed.starts_with("CADe_SIMU") {
        return Err("Cabeçalho inválido: arquivo não começa com 'CADe_SIMU'".to_string());
    }

    // Divide na marca "$$$" que encerra os itens do esquema
    let after_header = &trimmed["CADe_SIMU".len()..];
    // No CADe_SIMU original, pode haver caracteres de controle/espaço antes do primeiro '*'
    let first_star = after_header.find('*').unwrap_or(0);
    let body = &after_header[first_star..];

    let (items_part, rest_after_items) = match body.split_once("$$$") {
        Some((i, r)) => (i, r),
        None => (body, ""),
    };

    let mut items = Vec::new();
    let mut cursor = items_part;

    while !cursor.is_empty() {
        let Some(star_pos) = cursor.find('*') else {
            break;
        };
        cursor = &cursor[star_pos + 1..];
        if cursor.is_empty() {
            break;
        }

        // 1) id (*%d)
        let (id_str, r1) = next_token(cursor, &['*'])?;
        let id = id_str.trim().parse::<i32>().unwrap_or(0);

        // 2) tipo (*%d#)
        let (tipo_str, r2) = next_token(r1, &['#'])?;
        let tipo = tipo_str.trim().parse::<i32>().unwrap_or(0);
        let extended = SchematicItem::is_extended_cad_type(tipo);
        let num_pins = if extended { 9 } else { 8 };

        // 3) tag, value e pinos (#%s...)
        let (tag, mut cur) = next_token(r2, &['#'])?;
        let (value, next_cur) = next_token(cur, &['#'])?;
        cur = next_cur;

        let mut pins: [String; 9] = Default::default();
        for (i, slot) in pins.iter_mut().enumerate().take(num_pins) {
            let delim = if i + 1 == num_pins { '*' } else { '#' };
            let (p_val, next_c) = next_token(cur, &[delim])?;
            *slot = p_val;
            cur = next_c;
        }

        // 4) pin_visibility (8 ou 9 inteiros) + 15 inteiros de coordenadas/propriedades
        let total_ints = num_pins + 15;
        let mut int_vals = Vec::with_capacity(total_ints);
        for i in 0..total_ints {
            let delim = if i + 1 == total_ints { '#' } else { '*' };
            let (v_str, next_c) = next_token(cur, &[delim])?;
            let val = v_str.trim().parse::<i32>().unwrap_or(0);
            int_vals.push(val);
            cur = next_c;
        }

        let mut pin_visibility = [1; 9];
        pin_visibility[..num_pins].copy_from_slice(&int_vals[..num_pins]);
        let base = num_pins;
        let x1 = int_vals[base];
        let y1 = int_vals[base + 1];
        let x2 = int_vals[base + 2];
        let y2 = int_vals[base + 3];
        let aux_coords = [
            int_vals[base + 4],
            int_vals[base + 5],
            int_vals[base + 6],
            int_vals[base + 7],
        ];
        let color = int_vals[base + 8];
        let orientation = int_vals[base + 9];
        let subvariant = int_vals[base + 10];
        let show_tag = int_vals[base + 11];
        let show_value = int_vals[base + 12];
        let extra_int1 = int_vals[base + 13];
        let extra_int2 = int_vals[base + 14];

        // 5) trailing_str (#%s até o próximo '*' ou fim da seção de itens)
        let (trailing_str, next_c) = match cur.find('*') {
            Some(pos) => (cur[..pos].to_string(), &cur[pos..]),
            None => (cur.to_string(), ""),
        };
        cursor = next_c;

        items.push(SchematicItem {
            id,
            tipo,
            tag: tag.trim().to_string(),
            value: value.trim().to_string(),
            pins: pins.map(|p| p.trim().to_string()),
            pin_visibility,
            x1,
            y1,
            x2,
            y2,
            aux_coords,
            color,
            orientation,
            subvariant,
            show_tag,
            show_value,
            extra_int1,
            extra_int2,
            trailing_str: trailing_str.trim().to_string(),
            qet_type: None,
            qet_uuid: None,
        });
    }

    let mut settings = DocumentSettings::default();
    let mut title_block = TitleBlock::default();

    if let Some((settings_part, cajetin_part)) = rest_after_items.split_once("&&&") {
        let nums: Vec<i32> = settings_part
            .split('*')
            .filter(|s| !s.trim().is_empty())
            .filter_map(|s| s.trim().parse::<i32>().ok())
            .collect();
        if nums.len() >= 11 {
            settings.format_code = nums[0];
            settings.format_orientation = nums[1];
            settings.border_ref = nums[2];
            settings.show_pin_refs = nums[3];
            settings.line_thickness = nums[4];
            settings.color_mode = nums[5];
            settings.sim_speed = nums[6];
            settings.auto_number = nums[7];
            settings.displace_lines = nums[8];
            settings.custom_height_mm = nums[9];
            settings.custom_width_mm = nums[10];
        }

        let caj_fields: Vec<&str> = cajetin_part.split('*').skip(1).collect();
        if caj_fields.len() >= 10 {
            title_block.drawn_date = caj_fields[0].trim().to_string();
            title_block.drawn_by = caj_fields[1].trim().to_string();
            title_block.checked_date = caj_fields[2].trim().to_string();
            title_block.checked_by = caj_fields[3].trim().to_string();
            title_block.entity = caj_fields[4].trim().to_string();
            title_block.title = caj_fields[5].trim().to_string();
            title_block.subtitle = caj_fields[6].trim().to_string();
            title_block.sheet = caj_fields[7].trim().to_string();
            title_block.file_ref = caj_fields[8].trim().to_string();
            title_block.signature = caj_fields[9].trim().to_string();
        }
    }

    Ok(SchematicDocument {
        items,
        settings,
        title_block,
        ..Default::default()
    })
}

fn next_token<'a>(input: &'a str, delims: &[char]) -> Result<(String, &'a str), String> {
    if let Some(pos) = input.find(delims) {
        Ok((input[..pos].to_string(), &input[pos + 1..]))
    } else {
        Ok((input.to_string(), ""))
    }
}

/// Grava um arquivo de forma atômica utilizando um arquivo temporário no mesmo diretório,
/// sincronização com o disco (fsync) e rename para prevenir corrupção em caso de falha de energia.
pub fn atomic_write(path: &Path, content: impl AsRef<[u8]>) -> Result<(), String> {
    let tmp_path = path.with_extension(format!(
        "{}.tmp",
        path.extension().and_then(|e| e.to_str()).unwrap_or("dat")
    ));
    let mut file = fs::File::create(&tmp_path).map_err(|e| {
        format!(
            "Falha ao criar arquivo temporário {}: {e}",
            tmp_path.display()
        )
    })?;
    file.write_all(content.as_ref()).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!(
            "Falha ao gravar arquivo temporário {}: {e}",
            tmp_path.display()
        )
    })?;
    file.sync_all().map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!(
            "Falha ao sincronizar arquivo temporário com o disco {}: {e}",
            tmp_path.display()
        )
    })?;
    drop(file);
    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!("Falha ao renomear {}: {e}", path.display())
    })
}

/// Salva um documento em `.cad`, `.json`, `.toml` ou `.qet` com base na extensão do arquivo.
pub fn save_document(path: &Path, doc: &SchematicDocument) -> Result<(), String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("json")
        .to_ascii_lowercase();

    if ext == "qet" {
        return crate::qet::export_doc_to_qet(doc, path);
    }

    let content = match ext.as_str() {
        "cad" => serialize_cad_str(doc),
        "toml" => toml::to_string_pretty(doc).map_err(|e| format!("Erro TOML: {e}"))?,
        _ => serde_json::to_string_pretty(doc).map_err(|e| format!("Erro JSON: {e}"))?,
    };

    atomic_write(path, content)
}

/// Salva um projeto completo (mono ou multipáginas) em arquivo `.json`, `.toml` ou `.qet`.
pub fn save_project(
    path: &Path,
    pages: &[ProjectPage],
    active_page_index: usize,
) -> Result<(), String> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("json")
        .to_ascii_lowercase();

    if ext == "qet" {
        return crate::qet::export_pages_to_qet(pages, active_page_index, path, None);
    }

    if ext == "cad" {
        let active_doc = pages
            .get(active_page_index)
            .map(|p| &p.doc)
            .or_else(|| pages.first().map(|p| &p.doc))
            .ok_or_else(|| "Nenhuma folha disponível para salvar em CAD".to_string())?;
        return save_document(path, active_doc);
    }

    let project = ProjectDocument::new(pages.to_vec(), active_page_index);
    let content = match ext.as_str() {
        "toml" => toml::to_string_pretty(&project).map_err(|e| format!("Erro TOML: {e}"))?,
        _ => serde_json::to_string_pretty(&project).map_err(|e| format!("Erro JSON: {e}"))?,
    };

    atomic_write(path, content)
}

/// Carrega um projeto completo (mono ou multipáginas) a partir de `.json`, `.toml`, `.cad` ou `.qet`.
pub fn load_project(path: &Path) -> Result<ProjectDocument, String> {
    let bytes = fs::read(path).map_err(|e| format!("Falha ao ler {}: {e}", path.display()))?;
    // Arquivos .cad originais usam codificação Latin-1 (ISO-8859-1) / Windows-1252
    let text = match String::from_utf8(bytes.clone()) {
        Ok(s) => s,
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    };

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let proj: ProjectDocument = if ext == "qet" || text.trim_start().starts_with("<project") {
        let proj = crate::qet::load_qet_from_str(&text)?;
        let mut pages = Vec::new();
        for (idx, diag) in proj.diagrams.iter().enumerate() {
            let doc = crate::qet::diagram_to_document(&proj, idx);
            let title = if diag.title.trim().is_empty() {
                format!("Folha {}", idx + 1)
            } else {
                diag.title.clone()
            };
            pages.push(ProjectPage::new(idx + 1, title, doc));
        }
        let active_idx = pages
            .iter()
            .position(|p| !p.doc.items.is_empty())
            .unwrap_or(0);
        ProjectDocument::new(pages, active_idx)
    } else if ext == "cad" || text.trim_start().starts_with("CADe_SIMU") {
        let doc = parse_cad_str(&text)?;
        let title = if doc.title_block.title.trim().is_empty() {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Esquema")
                .to_string()
        } else {
            doc.title_block.title.clone()
        };
        ProjectDocument::from_single_document(title, doc)
    } else if ext == "toml" {
        // Tenta desserializar primeiro como ProjectDocument (multipáginas)
        match toml::from_str::<ProjectDocument>(&text) {
            Ok(proj) => proj,
            Err(err_proj) => {
                // Fallback para SchematicDocument (monofolha legado)
                match toml::from_str::<SchematicDocument>(&text) {
                    Ok(doc) => {
                        let title = if doc.title_block.title.trim().is_empty() {
                            path.file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Esquema")
                                .to_string()
                        } else {
                            doc.title_block.title.clone()
                        };
                        ProjectDocument::from_single_document(title, doc)
                    }
                    Err(err_legacy) => {
                        if text.contains("pages") {
                            return Err(format!("Erro ao ler projeto TOML: {err_proj}"));
                        } else {
                            return Err(format!(
                                "Erro ao ler TOML: {err_legacy} (tentativa de projeto: {err_proj})"
                            ));
                        }
                    }
                }
            }
        }
    } else {
        // JSON: Tenta desserializar primeiro como ProjectDocument (multipáginas)
        match serde_json::from_str::<ProjectDocument>(&text) {
            Ok(proj) => proj,
            Err(err_proj) => {
                // Fallback para SchematicDocument (monofolha legado)
                match serde_json::from_str::<SchematicDocument>(&text) {
                    Ok(doc) => {
                        let title = if doc.title_block.title.trim().is_empty() {
                            path.file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("Esquema")
                                .to_string()
                        } else {
                            doc.title_block.title.clone()
                        };
                        ProjectDocument::from_single_document(title, doc)
                    }
                    Err(err_legacy) => {
                        if text.contains("\"pages\"") {
                            return Err(format!("Erro ao ler projeto JSON: {err_proj}"));
                        } else {
                            return Err(format!(
                                "Erro ao ler JSON: {err_legacy} (tentativa de projeto: {err_proj})"
                            ));
                        }
                    }
                }
            }
        }
    };

    if proj.version > crate::model::CURRENT_PROJECT_VERSION {
        return Err(format!(
            "Versão do projeto ({}) não suportada por esta versão do RustSimu (máxima suportada: {})",
            proj.version,
            crate::model::CURRENT_PROJECT_VERSION
        ));
    }

    Ok(proj)
}

/// Carrega um documento `.cad`, `.json`, `.toml` ou `.qet`.
/// Se o arquivo for um projeto multipáginas (.json/.toml/.qet), retorna a folha ativa.
pub fn load_document(path: &Path) -> Result<SchematicDocument, String> {
    let proj = load_project(path)?;
    proj.active_document()
        .cloned()
        .ok_or_else(|| "Nenhum documento encontrado no projeto".to_string())
}

/// Constrói um circuito completo de **Partida Direta de Motor Trifásico com Selo e Sinalização**
/// (Circuito de Potência à esquerda + Circuito de Comando à direita) nas coordenadas nativas `.cad` (1 passo = 3).
pub fn build_dol_starter_preset() -> SchematicDocument {
    let mut doc = SchematicDocument::default();
    let mut id_counter = 1;
    let mut next_id = || {
        let current = id_counter;
        id_counter += 1;
        current
    };

    // ==================== CIRCUITO DE POTÊNCIA (X = 24..42) ====================
    // 1. Alimentação 3F + PE (tipo 3005) em (24, 18): L1=(24,18), L2=(30,18), L3=(36,18), PE=(42,18)
    if let Some(sym) = find_symbol(3005) {
        let mut item = sym.instantiate(next_id(), 24, 18, 0);
        item.tag = "-X1".to_string();
        doc.items.push(item);
    }

    // 2. Disjuntor Termomagnético 3P (tipo 6003) em (24, 24): Top=(24..36, 24), Bot=(24..36, 45)
    if let Some(sym) = find_symbol(6003) {
        let mut item = sym.instantiate(next_id(), 24, 24, 0);
        item.tag = "-Q1".to_string();
        doc.items.push(item);
    }

    // 3. Contator Trifásico 3P (tipo 2001) em (24, 51): Top=(24..36, 51), Bot=(24..36, 63)
    if let Some(sym) = find_symbol(2001) {
        let mut item = sym.instantiate(next_id(), 24, 51, 0);
        item.tag = "-KM1".to_string();
        doc.items.push(item);
    }

    // 4. Relé Térmico 3P (tipo 6007) em (24, 69): Top=(24..36, 69), Bot=(24..36, 81)
    if let Some(sym) = find_symbol(6007) {
        let mut item = sym.instantiate(next_id(), 24, 69, 0);
        item.tag = "-F1".to_string();
        doc.items.push(item);
    }

    // 5. Motor Trifásico (tipo 1000) em (24, 87): U1=(24,87), V1=(30,87), W1=(36,87), PE=(42,87)
    if let Some(sym) = find_symbol(1000) {
        let mut item = sym.instantiate(next_id(), 24, 87, 0);
        item.tag = "-M1".to_string();
        doc.items.push(item);
    }

    // Cabos de potência (L1, L2, L3 entre Alimentação -> Q1 -> KM1 -> F1 -> M1)
    for dx in [0, 6, 12] {
        let x = 24 + dx;
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4000, x, 18, x, 24));
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4000, x, 45, x, 51));
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4000, x, 63, x, 69));
        doc.items
            .push(SchematicItem::new_wire(next_id(), 4000, x, 81, x, 87));
    }
    // Cabo PE (42, 18) -> (42, 87)
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4010, 42, 18, 42, 87));

    // ==================== CIRCUITO DE COMANDO (X = 72..96) ====================
    // Alimentação Fase L (tipo 3000) em (72, 12)
    if let Some(sym) = find_symbol(3000) {
        let mut item = sym.instantiate(next_id(), 72, 12, 0);
        item.tag = "-X2".to_string();
        doc.items.push(item);
    }

    // Fusível de comando 1P (tipo 5000) em (72, 15): Top=(72, 15), Bot=(72, 27)
    if let Some(sym) = find_symbol(5000) {
        let mut item = sym.instantiate(next_id(), 72, 15, 0);
        item.tag = "-F2".to_string();
        doc.items.push(item);
    }

    // Contato NF do Relé Térmico 95-96 (tipo 8017) em (72, 30): Top=(72, 30), Bot=(72, 42)
    if let Some(sym) = find_symbol(8017) {
        let mut item = sym.instantiate(next_id(), 72, 30, 0);
        item.tag = "-F1".to_string();
        doc.items.push(item);
    }

    // Botão Desliga NF (tipo 8001) em (72, 45): Top=(72, 45), Bot=(72, 57)
    if let Some(sym) = find_symbol(8001) {
        let mut item = sym.instantiate(next_id(), 72, 45, 0);
        item.tag = "-S0".to_string();
        doc.items.push(item);
    }

    // Botão Liga NA (tipo 8000) em (72, 60): Top=(72, 60), Bot=(72, 72)
    if let Some(sym) = find_symbol(8000) {
        let mut item = sym.instantiate(next_id(), 72, 60, 0);
        item.tag = "-S1".to_string();
        doc.items.push(item);
    }

    // Contato de Selo NA (tipo 7000) em paralelo com -S1 em (84, 60): Top=(84, 60), Bot=(84, 72)
    if let Some(sym) = find_symbol(7000) {
        let mut item = sym.instantiate(next_id(), 84, 60, 0);
        item.tag = "-KM1".to_string();
        doc.items.push(item);
    }

    // Bobina do Contator -KM1 (tipo 9000) em (72, 78): Top=(72, 78), Bot=(72, 90)
    if let Some(sym) = find_symbol(9000) {
        let mut item = sym.instantiate(next_id(), 72, 78, 0);
        item.tag = "-KM1".to_string();
        doc.items.push(item);
    }

    // Sinaleiro Luminoso -H1 (Motor Ligado, tipo 9008) em paralelo com a bobina em (96, 78): Top=(96, 78), Bot=(96, 90)
    if let Some(sym) = find_symbol(9008) {
        let mut item = sym.instantiate(next_id(), 96, 78, 0);
        item.tag = "-H1".to_string();
        doc.items.push(item);
    }

    // Alimentação Neutro N (tipo 3001) em (72, 96) rotacionada 180° (orientação 2)
    if let Some(sym) = find_symbol(3001) {
        let mut item = sym.instantiate(next_id(), 72, 96, 2);
        item.tag = "-X3".to_string();
        doc.items.push(item);
    }

    // Cabos do circuito de comando
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 72, 12, 72, 15));
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 72, 27, 72, 30));
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 72, 42, 72, 45));
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 72, 57, 72, 60));
    // Paralelo de selo superior (72, 57) -> (84, 57) -> (84, 60)
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 72, 57, 84, 57));
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 84, 57, 84, 60));
    doc.items
        .push(SchematicItem::new_junction(next_id(), 72, 57));
    // Paralelo de selo inferior (84, 72) -> (84, 75) -> (72, 75)
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 72, 72, 72, 78));
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 84, 72, 84, 75));
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 72, 75, 96, 75));
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4000, 96, 75, 96, 78));
    doc.items
        .push(SchematicItem::new_junction(next_id(), 72, 75));
    doc.items
        .push(SchematicItem::new_junction(next_id(), 84, 75));
    // Retorno Neutro (A2 de KM1 e X2 de H1 até N em 72, 96)
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4009, 72, 90, 72, 96));
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4009, 96, 90, 96, 93));
    doc.items
        .push(SchematicItem::new_wire(next_id(), 4009, 72, 93, 96, 93));
    doc.items
        .push(SchematicItem::new_junction(next_id(), 72, 93));

    doc
}
