use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Metadados de terminal elétrico de conexão.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TerminalMetadata {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub orientation: String,
}

/// Metadados Schema v1 (compatibilidade com catálogo pré-migração).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SymbolMetadata {
    pub id: String,
    pub svg_file: String,
    pub category: String,
    pub name_en: String,
    pub name_pt: Option<String>,
    pub width: f64,
    pub height: f64,
    pub hotspot_x: f64,
    pub hotspot_y: f64,
    pub terminals: Vec<TerminalMetadata>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant_of: Option<String>,
}

/// Dicionário canônico Schema v2 (Decisão D2).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CanonicalCatalog {
    pub schema_version: u32,
    pub generated_at: String,
    pub source: CatalogSource,
    pub categories: Vec<CanonicalCategory>,
    pub elements: Vec<CanonicalElement>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CatalogSource {
    pub kind: String,
    pub version: String,
    pub path: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CanonicalCategory {
    pub code: String,   // 3 dígitos decimais ("012")
    pub family: String, // "electric", "energy", etc.
    pub path: String,   // caminho original
    pub name_pt: String,
    pub name_en: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CanonicalElement {
    pub id: String,       // 5 hex maiúsculo ("00020")
    pub slug: String,     // slug original ("ac_motor_3-phases")
    pub category: String, // código de 3 dígitos ("012")
    pub family: String,   // família primária ("electric")
    pub kind: String,     // tipo canônico obrigatório (§5.2)
    pub name_pt: String,
    pub name_en: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<ElementSource>,
    pub svg_base: String, // "<cat>-<id>-0.svg" (exato 15 chars)
    pub bbox: BBox,
    pub hotspot: Hotspot,
    pub states: Vec<CanonicalState>,
    pub overlays: Vec<String>,
    pub terminals: Vec<TerminalMetadata>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ElementSource {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qet_uuid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qet_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BBox {
    pub w: f64,
    pub h: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Hotspot {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CanonicalState {
    pub code: String, // "0", "1", ...
    pub name: String, // "repouso", "ativo", ...
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub svg: Option<String>,
}

/// Resumo da verificação de integridade do catálogo canônico D2.
#[derive(Debug, Clone)]
pub struct CanonicalCheckSummary {
    pub schema_version: u32,
    pub categories_count: usize,
    pub elements_count: usize,
    pub svgs_checked: usize,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

/// Converte um stem num nome legível em caixa alta/baixa.
pub fn humanize_stem(stem: &str) -> String {
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

/// Classifica o elemento em um dos 18 tipos canônicos obrigatórios da Decisão D2 (§5.2).
pub fn classify_element_kind(
    slug: &str,
    category: &str,
    name_en: &str,
    terminals_count: usize,
) -> &'static str {
    if terminals_count == 0 {
        return "graphic";
    }
    let s = slug.to_ascii_lowercase();
    let cat = category.to_ascii_lowercase();
    let name = name_en.to_ascii_lowercase();

    if s.contains("emergency") || name.contains("emergency") {
        return "emergency_stop";
    }
    if s.contains("limitswitch") || s.contains("limit_switch") || s.contains("endschalter") {
        return "limit_switch";
    }
    if s.contains("push_button")
        || s.contains("pushbutton")
        || s.contains("taster")
        || s.contains("horn_switch")
        || s.contains("pulsador")
    {
        return "momentary_button";
    }
    if s.contains("selector")
        || (s.contains("schalter") && (s.contains("pos") || s.contains("step")))
    {
        return "selector_switch";
    }
    if s.contains("breaker")
        || s.contains("disjoncteur")
        || s.contains("leistungs_schalter")
        || s.contains("disjuntor")
    {
        return "circuit_breaker";
    }
    if s.contains("fuse")
        || s.contains("sicherung")
        || s.contains("fusivel")
        || s.contains("fusível")
    {
        return "fuse";
    }
    if s.contains("disconnector") || s.contains("seccionador") || s.contains("trennschalter") {
        return "disconnector";
    }
    if s.contains("switch") || s.contains("interrupteur") || s.contains("chave") {
        return "latching_switch";
    }
    if s.contains("motor") || s.contains("moteur") || cat.contains("engines") {
        return "motor";
    }
    if s.contains("lamp")
        || s.contains("light")
        || s.contains("voyant")
        || s.contains("sinaleiro")
        || s.contains("led")
    {
        return "lamp";
    }
    if s.contains("coil") || s.contains("bobina") || s.contains("rele") || s.contains("relay") {
        return "relay_coil";
    }
    if s.contains("thermal") || s.contains("thermique") || s.contains("bimetal") {
        return "thermal_contact";
    }
    if s.contains("contact") || s.contains("contato") {
        return "contactor_contact";
    }
    if s.contains("battery")
        || s.contains("source")
        || s.contains("power_supply")
        || s.contains("ground")
        || s.contains("earth")
        || s.contains("alimenta")
        || cat.contains("network_supplies")
    {
        return "source";
    }
    if s.contains("terminal") || s.contains("borne") || cat.contains("terminals") {
        return "terminal";
    }
    if s.contains("connector")
        || s.contains("plug")
        || s.contains("stecker")
        || cat.contains("connectors")
    {
        return "connector";
    }
    if s.contains("siren")
        || s.contains("buzzer")
        || s.contains("horn")
        || s.contains("heater")
        || s.contains("resistor")
        || cat.contains("consumers")
    {
        return "load";
    }
    if s.contains("splitter")
        || s.contains("verteiler")
        || s.contains("splice")
        || s.contains("distributor")
    {
        return "passive";
    }

    "passive"
}

/// Extrai o nome legível em inglês e português para uma categoria a partir de seu caminho.
pub fn humanize_category_name(path: &str) -> (String, String) {
    let leaf = path.split('/').next_back().unwrap_or(path);
    let clean = leaf.trim_start_matches(|c: char| c.is_ascii_digit() || c == '_');
    let en = humanize_stem(if clean.is_empty() { leaf } else { clean });

    let pt = match clean {
        "folio_referencing" => "Referenciamento de folhas",
        "network_supplies" => "Alimentação de rede",
        "connections" => "Conexões",
        "cables_wiring" => "Cabos e fiação",
        "terminals_terminal_strips" => "Bornes e réguas de bornes",
        "connectors_plugs" => "Conectores e plugues",
        "fuses_protective_gears" => "Fusíveis e proteções",
        "fuses" => "Fusíveis",
        "circuit_breakers" => "Disjuntores",
        "magneto_thermal_circuit_breakers" => "Disjuntores termomagnéticos",
        "relays_contactors_contacts" => "Relés, contatores e contatos",
        "contactors" => "Contatores",
        "relays" => "Relés",
        "thermal_relays" => "Relés térmicos",
        "signaling_operating" => "Sinalização e comando",
        "push_buttons" => "Botões pulsadores",
        "switches" => "Chaves e interruptores",
        "sensors_instruments" => "Sensores e instrumentos",
        "limit_switches" => "Fins de curso",
        "consumers_actuators" => "Consumidores e atuadores",
        "engines" => "Motores",
        "transformers" => "Transformadores",
        "electronic" => "Eletrônica",
        "cade_native" => "Símbolos nativos CADe",
        "air_generation" => "Geração de ar",
        "tank" => "Reservatório",
        "treatment" => "Tratamento",
        "control" => "Controle",
        "logic_gates" => "Portas lógicas",
        "tanks" => "Tanques",
        "control_valves" => "Válvulas de controle",
        "water" => "Água",
        "classic_car" => "Veículos clássicos",
        other => {
            let lower = other.to_lowercase();
            if lower.contains("motor") || lower.contains("engine") {
                "Motores"
            } else if lower.contains("switch") {
                "Interruptores"
            } else if lower.contains("button") {
                "Botões"
            } else if lower.contains("sensor") {
                "Sensores"
            } else if lower.contains("valve") {
                "Válvulas"
            } else if lower.contains("pump") {
                "Bombas"
            } else {
                return (en.clone(), en);
            }
        }
    }
    .to_string();
    (en, pt)
}

/// Extrai tags textuais normalizadas para busca eficiente.
pub fn extract_tags(
    slug: &str,
    name_en: &str,
    name_pt: &str,
    kind: &str,
    category: &str,
) -> Vec<String> {
    let mut tags = std::collections::BTreeSet::new();
    tags.insert(kind.to_string());

    let combined = format!("{} {} {} {}", slug, name_en, name_pt, category);
    for token in combined.split(|c: char| !c.is_alphanumeric()) {
        let t = token.trim().to_lowercase();
        if t.len() >= 2
            && !matches!(
                t.as_str(),
                "and"
                    | "the"
                    | "for"
                    | "with"
                    | "von"
                    | "und"
                    | "para"
                    | "com"
                    | "sem"
                    | "dos"
                    | "das"
                    | "der"
                    | "die"
                    | "des"
            )
        {
            tags.insert(t);
        }
    }
    tags.into_iter().collect()
}

/// Determina a lista de overlays dinâmicos de runtime conforme o kind do elemento.
pub fn determine_overlays(kind: &str) -> Vec<String> {
    match kind {
        "motor" => vec![
            "fault".into(),
            "short_circuit".into(),
            "rotation".into(),
            "selected".into(),
        ],
        "circuit_breaker" | "fuse" | "disconnector" | "thermal_contact" => {
            vec!["fault".into(), "short_circuit".into(), "selected".into()]
        }
        "lamp" | "relay_coil" | "load" => vec!["fault".into(), "selected".into()],
        _ => vec!["selected".into()],
    }
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

/// Migra os símbolos existentes em `source_base` (formato antigo / árvore profunda)
/// para o Formato Canônico D2 em `target_base` (`<familia>/<subcategoria:3>-<id:5hex>-<estado:1hex>.svg`).
pub fn migrate_to_canonical(
    source_base: &Path,
    target_base: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let catalog_path = source_base.join("catalog.json");
    if !catalog_path.exists() {
        return Err(format!("Catálogo de origem não encontrado em {:?}", catalog_path).into());
    }

    let source_catalog: Vec<SymbolMetadata> =
        serde_json::from_str(&fs::read_to_string(&catalog_path)?)?;
    println!(
        "Carregadas {} entradas do catálogo v1.",
        source_catalog.len()
    );

    // 1. Separa elementos base e variantes de estado
    let mut base_elements = Vec::new();
    let mut variants_by_base: HashMap<String, Vec<&SymbolMetadata>> = HashMap::new();

    for meta in &source_catalog {
        if meta.state.as_deref() == Some("active") || meta.variant_of.is_some() {
            if let Some(ref base_id) = meta.variant_of {
                variants_by_base
                    .entry(base_id.clone())
                    .or_default()
                    .push(meta);
            }
        } else {
            base_elements.push(meta);
        }
    }

    // 2. Coleta e mapeia categorias únicas por família
    let mut fam_cats: HashMap<String, Vec<String>> = HashMap::new();
    for meta in &base_elements {
        let fam = meta
            .category
            .split('/')
            .next()
            .unwrap_or("electric")
            .to_string();
        let list = fam_cats.entry(fam).or_default();
        if !list.contains(&meta.category) {
            list.push(meta.category.clone());
        }
    }

    let mut cat_code_map: HashMap<String, String> = HashMap::new();
    let mut canonical_categories = Vec::new();

    let mut families: Vec<_> = fam_cats.keys().cloned().collect();
    families.sort();

    for fam in &families {
        let cats = fam_cats.get_mut(fam).unwrap();
        cats.sort();
        for (idx, cat_path) in cats.iter().enumerate() {
            let code = format!("{:03}", idx);
            cat_code_map.insert(cat_path.clone(), code.clone());
            let (en, pt) = humanize_category_name(cat_path);
            canonical_categories.push(CanonicalCategory {
                code,
                family: fam.clone(),
                path: cat_path.clone(),
                name_pt: pt,
                name_en: en,
            });
        }
    }

    // 3. Ordena os elementos base determinísticamente e atribui IDs de 5 hex maiúsculo
    base_elements.sort_by(|a, b| {
        (a.category.as_str(), a.id.as_str()).cmp(&(b.category.as_str(), b.id.as_str()))
    });

    let mut canonical_elements = Vec::new();
    let mut slug_to_id = HashMap::new();
    let mut total_files_copied = 0;

    // Cria diretórios das famílias em target_base
    for fam in &families {
        fs::create_dir_all(target_base.join(fam))?;
    }

    for (idx, meta) in base_elements.iter().enumerate() {
        let hex_id = format!("{:05X}", idx + 1);
        let fam = meta.category.split('/').next().unwrap_or("electric");
        let subcat = cat_code_map
            .get(&meta.category)
            .cloned()
            .unwrap_or_else(|| "000".to_string());

        let svg_base_filename = format!("{}-{}-0.svg", subcat, hex_id);
        let src_file_path = source_base.join(&meta.category).join(&meta.svg_file);
        let dest_file_path = target_base.join(fam).join(&svg_base_filename);

        fs::copy(&src_file_path, &dest_file_path).map_err(|e| {
            format!(
                "Erro copiando {:?} para {:?}: {}",
                src_file_path, dest_file_path, e
            )
        })?;
        total_files_copied += 1;

        let name_en = meta.name_en.clone();
        let name_pt = meta.name_pt.clone().unwrap_or_else(|| name_en.clone());
        let kind = classify_element_kind(&meta.id, &meta.category, &name_en, meta.terminals.len());
        let tags = extract_tags(&meta.id, &name_en, &name_pt, kind, &meta.category);
        let overlays = determine_overlays(kind);

        let mut states = vec![CanonicalState {
            code: "0".to_string(),
            name: "repouso".to_string(),
            svg: None,
        }];

        if let Some(vars) = variants_by_base.get(&meta.id) {
            for (v_idx, var_meta) in vars.iter().enumerate() {
                let state_code = format!("{:X}", v_idx + 1);
                let var_filename = format!("{}-{}-{}.svg", subcat, hex_id, state_code);
                let var_src = source_base
                    .join(&var_meta.category)
                    .join(&var_meta.svg_file);
                let var_dest = target_base.join(fam).join(&var_filename);
                fs::copy(&var_src, &var_dest).map_err(|e| {
                    format!(
                        "Erro copiando variante {:?} para {:?}: {}",
                        var_src, var_dest, e
                    )
                })?;
                total_files_copied += 1;

                states.push(CanonicalState {
                    code: state_code,
                    name: "ativo".to_string(),
                    svg: Some(var_filename),
                });
            }
        }

        slug_to_id.insert(meta.id.clone(), hex_id.clone());

        canonical_elements.push(CanonicalElement {
            id: hex_id,
            slug: meta.id.clone(),
            category: subcat,
            family: fam.to_string(),
            kind: kind.to_string(),
            name_pt,
            name_en,
            source: meta.source.as_ref().map(|s| ElementSource {
                qet_uuid: None,
                qet_path: None,
                origin: Some(s.clone()),
            }),
            svg_base: svg_base_filename,
            bbox: BBox {
                w: meta.width,
                h: meta.height,
            },
            hotspot: Hotspot {
                x: meta.hotspot_x,
                y: meta.hotspot_y,
            },
            states,
            overlays,
            terminals: meta.terminals.clone(),
            tags,
        });
    }

    let canonical_catalog = CanonicalCatalog {
        schema_version: 2,
        generated_at: "2026-11-07T12:00:00Z".to_string(),
        source: CatalogSource {
            kind: "qelectrotech".to_string(),
            version: "0.9".to_string(),
            path: source_base.to_string_lossy().to_string(),
        },
        categories: canonical_categories,
        elements: canonical_elements,
    };

    let target_catalog_json = target_base.join("catalog.json");
    let mut file = File::create(&target_catalog_json)?;
    let serialized = serde_json::to_vec_pretty(&canonical_catalog)?;
    file.write_all(&serialized)?;

    let slug_map_json = target_base.join("slug_map.json");
    let mut map_file = File::create(&slug_map_json)?;
    let map_serialized = serde_json::to_vec_pretty(&slug_to_id)?;
    map_file.write_all(&map_serialized)?;

    println!("==================================================");
    println!("Migração Canônica D2 Concluída!");
    println!("Categorias: {}", canonical_catalog.categories.len());
    println!("Elementos Base: {}", canonical_catalog.elements.len());
    println!("Arquivos SVG canônicos gerados: {}", total_files_copied);
    println!("Catálogo v2: {:?}", target_catalog_json);
    println!("Mapa de slugs: {:?}", slug_map_json);

    Ok(())
}

/// Valida todas as invariantes da Decisão D2 (§6). Falha se qualquer invariante for violada.
pub fn check_canonical_catalog(target_base: &Path) -> Result<CanonicalCheckSummary, String> {
    let catalog_path = target_base.join("catalog.json");
    if !catalog_path.exists() {
        return Err(format!("Catálogo não encontrado em {:?}", catalog_path));
    }

    let mut content = String::new();
    let mut file = File::open(&catalog_path).map_err(|e| format!("Erro abrindo catálogo: {e}"))?;
    file.read_to_string(&mut content)
        .map_err(|e| format!("Erro lendo catálogo: {e}"))?;

    let catalog: CanonicalCatalog =
        serde_json::from_str(&content).map_err(|e| format!("Erro parse JSON: {e}"))?;

    if catalog.schema_version != 2 {
        return Err(format!(
            "schema_version esperada: 2, encontrada: {}",
            catalog.schema_version
        ));
    }

    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    let mut seen_ids = HashSet::new();
    let mut referenced_svgs = HashSet::new();

    // 1. Valida categorias
    let mut fam_cat_codes: HashMap<String, HashSet<String>> = HashMap::new();
    for cat in &catalog.categories {
        if cat.code.len() != 3 || !cat.code.chars().all(|c| c.is_ascii_digit()) {
            errors.push(format!(
                "Código de categoria inválido (deve ter 3 dígitos decimais): {:?}",
                cat.code
            ));
        }
        let set = fam_cat_codes.entry(cat.family.clone()).or_default();
        if !set.insert(cat.code.clone()) {
            errors.push(format!(
                "Código de categoria duplicado na família {}: {}",
                cat.family, cat.code
            ));
        }
    }

    // 2. Valida elementos
    for el in &catalog.elements {
        if el.id.len() != 5
            || !el
                .id
                .chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_lowercase())
        {
            errors.push(format!(
                "ID inválido (deve ter 5 hex maiúsculos): {}",
                el.id
            ));
        }
        if !seen_ids.insert(el.id.clone()) {
            errors.push(format!("ID duplicado: {}", el.id));
        }

        // Valida svg_base
        if el.svg_base.len() != 15 {
            errors.push(format!("svg_base com tamanho != 15: {}", el.svg_base));
        }
        let base_path = target_base.join(&el.family).join(&el.svg_base);
        if !base_path.exists() {
            errors.push(format!(
                "Arquivo svg_base não encontrado no disco: {:?}",
                base_path
            ));
        } else {
            referenced_svgs.insert(base_path);
        }

        // Valida estados
        for st in &el.states {
            if let Some(ref svg) = st.svg {
                if svg.len() != 15 {
                    errors.push(format!("SVG de estado com tamanho != 15: {}", svg));
                }
                let st_path = target_base.join(&el.family).join(svg);
                if !st_path.exists() {
                    errors.push(format!(
                        "Arquivo SVG de estado não encontrado no disco: {:?}",
                        st_path
                    ));
                } else {
                    referenced_svgs.insert(st_path);
                }
            }
        }

        // Valida bbox e hotspot
        if el.bbox.w <= 0.0 || el.bbox.h <= 0.0 {
            warnings.push(format!(
                "Elemento {} ({}) com bbox não-positiva: {}x{}",
                el.id, el.slug, el.bbox.w, el.bbox.h
            ));
        }
    }

    // 3. Valida se existem SVGs no disco fora do catálogo
    let mut disk_svgs = Vec::new();
    collect_svg_paths(target_base, &mut disk_svgs)
        .map_err(|e| format!("Erro lendo diretório de SVGs: {e}"))?;
    for path in disk_svgs {
        if !referenced_svgs.contains(&path) {
            errors.push(format!(
                "SVG órfão em disco não referenciado no catálogo: {:?}",
                path
            ));
        }
    }

    let summary = CanonicalCheckSummary {
        schema_version: catalog.schema_version,
        categories_count: catalog.categories.len(),
        elements_count: catalog.elements.len(),
        svgs_checked: referenced_svgs.len(),
        warnings,
        errors,
    };

    Ok(summary)
}

/// Dicionário canônico carregado em memória para resolução em O(1).
pub struct CanonicalDictionary {
    pub base_dir: PathBuf,
    pub catalog: CanonicalCatalog,
    pub elements_by_id: HashMap<String, usize>,
    pub slug_to_id: HashMap<String, String>,
}

impl CanonicalDictionary {
    /// Carrega o catálogo canônico a partir do diretório raiz `symbols_qet/`.
    pub fn load(base_dir: PathBuf) -> Result<Self, String> {
        let catalog_path = base_dir.join("catalog.json");
        let content = fs::read_to_string(&catalog_path)
            .map_err(|e| format!("Falha ao ler {:?}: {}", catalog_path, e))?;
        let catalog: CanonicalCatalog = serde_json::from_str(&content)
            .map_err(|e| format!("Falha ao parsear {:?}: {}", catalog_path, e))?;

        let mut elements_by_id = HashMap::with_capacity(catalog.elements.len());
        let mut slug_to_id = HashMap::with_capacity(catalog.elements.len());

        for (idx, el) in catalog.elements.iter().enumerate() {
            elements_by_id.insert(el.id.clone(), idx);
            slug_to_id.insert(el.slug.clone(), el.id.clone());
        }

        Ok(Self {
            base_dir,
            catalog,
            elements_by_id,
            slug_to_id,
        })
    }

    /// Retorna uma referência ao elemento pelo ID de 5 hex maiúsculo.
    pub fn get_by_id(&self, id: &str) -> Option<&CanonicalElement> {
        let key = id.trim().to_ascii_uppercase();
        self.elements_by_id
            .get(&key)
            .map(|&idx| &self.catalog.elements[idx])
    }

    /// Retorna uma referência ao elemento pelo seu slug original.
    pub fn get_by_slug(&self, slug: &str) -> Option<&CanonicalElement> {
        let id = self.slug_to_id.get(slug.trim())?;
        self.get_by_id(id)
    }

    /// Retorna uma referência ao elemento por ID canônico ou slug.
    pub fn get(&self, id_or_slug: &str) -> Option<&CanonicalElement> {
        self.get_by_id(id_or_slug)
            .or_else(|| self.get_by_slug(id_or_slug))
    }

    /// Retorna o caminho absoluto do arquivo SVG correspondente a um elemento e estado.
    pub fn resolve_svg_path(&self, id_or_slug: &str, state: Option<char>) -> Option<PathBuf> {
        let el = self.get(id_or_slug)?;
        let state_char = state.unwrap_or('0').to_ascii_uppercase();

        if state_char != '0' {
            let state_str = state_char.to_string();
            if let Some(st) = el.states.iter().find(|s| s.code == state_str) {
                if let Some(ref svg_name) = st.svg {
                    let path = self.base_dir.join(&el.family).join(svg_name);
                    if path.exists() {
                        return Some(path);
                    }
                }
            }
        }

        let base_path = self.base_dir.join(&el.family).join(&el.svg_base);
        if base_path.exists() {
            Some(base_path)
        } else {
            None
        }
    }

    /// Retorna o kind canônico de um elemento.
    pub fn kind(&self, id_or_slug: &str) -> Option<&str> {
        self.get(id_or_slug).map(|el| el.kind.as_str())
    }

    /// Busca textual rápida por query sobre id, slug, tags e nomes pt/en.
    pub fn search(&self, query: &str) -> Vec<&CanonicalElement> {
        let q = query.trim().to_lowercase();
        if q.is_empty() {
            return self.catalog.elements.iter().take(100).collect();
        }
        self.catalog
            .elements
            .iter()
            .filter(|el| {
                el.id.contains(&q)
                    || el.slug.to_lowercase().contains(&q)
                    || el.name_pt.to_lowercase().contains(&q)
                    || el.name_en.to_lowercase().contains(&q)
                    || el.tags.iter().any(|t| t.contains(&q))
            })
            .collect()
    }
}

static GLOBAL_DICT: OnceLock<Option<CanonicalDictionary>> = OnceLock::new();

/// Retorna a instância global singleton do dicionário canônico.
pub fn global_dictionary() -> Option<&'static CanonicalDictionary> {
    GLOBAL_DICT
        .get_or_init(|| {
            // Procura symbols_qet no diretório de trabalho, no CARGO_MANIFEST_DIR ou relativo ao exe
            let candidates = [
                PathBuf::from("symbols_qet"),
                PathBuf::from("cade-simu-rs/symbols_qet"),
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("symbols_qet"),
            ];
            for candidate in &candidates {
                if candidate.join("catalog.json").exists() {
                    if let Ok(dict) = CanonicalDictionary::load(candidate.clone()) {
                        return Some(dict);
                    }
                }
            }
            if let Ok(exe) = std::env::current_exe() {
                if let Some(parent) = exe.parent() {
                    let p = parent.join("symbols_qet");
                    if p.join("catalog.json").exists() {
                        if let Ok(dict) = CanonicalDictionary::load(p) {
                            return Some(dict);
                        }
                    }
                }
            }
            None
        })
        .as_ref()
}
