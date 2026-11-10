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

/// Tipo canônico do elemento na biblioteca padronizada (Decisões D2 e D3).
/// Tipado como enum com serde snake_case para verificação exaustiva no compilador.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalKind {
    Block,
    CircuitBreaker,
    Connector,
    ContactorContact,
    Disconnector,
    EmergencyStop,
    Fuse,
    Lamp,
    LatchingSwitch,
    LimitSwitch,
    Load,
    MomentaryButton,
    Motor,
    Passive,
    RelayCoil,
    SelectorSwitch,
    Source,
    Terminal,
    ThermalContact,
}

impl CanonicalKind {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::CircuitBreaker => "circuit_breaker",
            Self::Connector => "connector",
            Self::ContactorContact => "contactor_contact",
            Self::Disconnector => "disconnector",
            Self::EmergencyStop => "emergency_stop",
            Self::Fuse => "fuse",
            Self::Lamp => "lamp",
            Self::LatchingSwitch => "latching_switch",
            Self::LimitSwitch => "limit_switch",
            Self::Load => "load",
            Self::MomentaryButton => "momentary_button",
            Self::Motor => "motor",
            Self::Passive => "passive",
            Self::RelayCoil => "relay_coil",
            Self::SelectorSwitch => "selector_switch",
            Self::Source => "source",
            Self::Terminal => "terminal",
            Self::ThermalContact => "thermal_contact",
        }
    }

    #[inline]
    pub const fn is_wireable(&self) -> bool {
        !matches!(self, Self::Block)
    }

    pub const fn default_tag_prefix(&self) -> &'static str {
        match self {
            Self::Block => "-BK",
            Self::CircuitBreaker => "-Q",
            Self::Fuse => "-F",
            Self::Disconnector => "-Q",
            Self::RelayCoil | Self::ContactorContact => "-KM",
            Self::ThermalContact => "-F",
            Self::MomentaryButton | Self::EmergencyStop => "-S",
            Self::LatchingSwitch | Self::SelectorSwitch => "-S",
            Self::LimitSwitch => "-SQ",
            Self::Motor => "-M",
            Self::Lamp => "-H",
            Self::Source => "-G",
            Self::Terminal | Self::Connector => "-X",
            Self::Load => "-R",
            Self::Passive => "-E",
        }
    }
}

impl std::fmt::Display for CanonicalKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CanonicalElement {
    pub id: String,          // 5 hex maiúsculo ("00020")
    pub slug: String,        // slug original ("ac_motor_3-phases")
    pub category: String,    // código de 3 dígitos ("012")
    pub family: String,      // família primária ("electric")
    pub kind: CanonicalKind, // tipo canônico obrigatório (§5.2)
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

impl CanonicalElement {
    /// Indica se o elemento é cabável (pode ser conectado eletricamente/ligado por fios).
    /// Retorna `false` para blocos de montagem visual (painel, trilhos DIN, módulos CLP e acessórios de layout).
    #[inline]
    pub fn is_wireable(&self) -> bool {
        self.kind.is_wireable() && !self.terminals.is_empty()
    }
}

/// Escopo de filtragem de elementos na biblioteca unificada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum CanonicalScope {
    /// Todos os elementos (cabáveis de esquema e blocos de painel/montagem).
    #[default]
    All,
    /// Apenas elementos cabáveis com bornes/terminais para diagramas de comando e potência.
    Wireable,
    /// Apenas blocos de montagem visual (gabaritos 2D, trilhos DIN, módulos CLP e acessórios de painel).
    Block,
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

/// Verifica se o elemento corresponde a um controlador/módulo CLP (PLC) ou bloco de controle lógico.
/// Conforme diretriz do projeto, blocos de CLP são tratados como blocos de montagem visual não cabáveis (kind: "block").
pub fn is_plc_element(slug: &str, category: &str, name_en: &str) -> bool {
    let s = slug.to_ascii_lowercase();
    let cat = category.to_ascii_lowercase();
    let name = name_en.to_ascii_lowercase();

    // Categorias de fabricantes e controladores
    if cat.contains("plc_controllers")
        || cat.contains("plc_and_controllers")
        || cat.contains("41_plc_controllers")
        || cat.contains("arduino")
        || cat.contains("unitronics_plc")
        || cat.contains("020_controller")
        || cat.contains("safety_controllers")
        || cat.contains("6es7")
        || cat.contains("6es5")
        || cat.contains("s7_300")
        || cat.contains("s7_1500")
    {
        return true;
    }

    // Slugs e identificadores
    if s.contains("plc_")
        || s.contains("_plc")
        || s == "plc"
        || s.contains("automate_")
        || s.contains("_automate")
        || s == "automate"
        || s.contains("automatisme")
    {
        return true;
    }

    // Nomes em inglês
    name.contains("plc ")
        || name.contains(" plc")
        || name.starts_with("plc")
        || name.contains("automate")
}

/// Classifica o elemento em um dos tipos canônicos da biblioteca.
pub fn classify_element_kind(
    slug: &str,
    category: &str,
    name_en: &str,
    terminals_count: usize,
) -> CanonicalKind {
    if terminals_count == 0 || is_plc_element(slug, category, name_en) {
        return CanonicalKind::Block;
    }
    let s = slug.to_ascii_lowercase();
    let cat = category.to_ascii_lowercase();
    let name = name_en.to_ascii_lowercase();

    if s.contains("emergency") || name.contains("emergency") {
        return CanonicalKind::EmergencyStop;
    }
    if s.contains("limitswitch") || s.contains("limit_switch") || s.contains("endschalter") {
        return CanonicalKind::LimitSwitch;
    }
    if s.contains("push_button")
        || s.contains("pushbutton")
        || s.contains("taster")
        || s.contains("horn_switch")
        || s.contains("pulsador")
    {
        return CanonicalKind::MomentaryButton;
    }
    if s.contains("selector")
        || (s.contains("schalter") && (s.contains("pos") || s.contains("step")))
    {
        return CanonicalKind::SelectorSwitch;
    }
    if s.contains("breaker")
        || s.contains("disjoncteur")
        || s.contains("leistungs_schalter")
        || s.contains("disjuntor")
    {
        return CanonicalKind::CircuitBreaker;
    }
    if s.contains("fuse")
        || s.contains("sicherung")
        || s.contains("fusivel")
        || s.contains("fusível")
    {
        return CanonicalKind::Fuse;
    }
    if s.contains("disconnector") || s.contains("seccionador") || s.contains("trennschalter") {
        return CanonicalKind::Disconnector;
    }
    if s.contains("switch") || s.contains("interrupteur") || s.contains("chave") {
        return CanonicalKind::LatchingSwitch;
    }
    if s.contains("motor") || s.contains("moteur") || cat.contains("engines") {
        return CanonicalKind::Motor;
    }
    if s.contains("lamp")
        || s.contains("light")
        || s.contains("voyant")
        || s.contains("sinaleiro")
        || s.contains("led")
    {
        return CanonicalKind::Lamp;
    }
    if s.contains("coil") || s.contains("bobina") || s.contains("rele") || s.contains("relay") {
        return CanonicalKind::RelayCoil;
    }
    if s.contains("thermal") || s.contains("thermique") || s.contains("bimetal") {
        return CanonicalKind::ThermalContact;
    }
    if s.contains("contact") || s.contains("contato") {
        return CanonicalKind::ContactorContact;
    }
    if s.contains("battery")
        || s.contains("source")
        || s.contains("power_supply")
        || s.contains("ground")
        || s.contains("earth")
        || s.contains("alimenta")
        || cat.contains("network_supplies")
    {
        return CanonicalKind::Source;
    }
    if s.contains("terminal") || s.contains("borne") || cat.contains("terminals") {
        return CanonicalKind::Terminal;
    }
    if s.contains("connector")
        || s.contains("plug")
        || s.contains("stecker")
        || cat.contains("connectors")
    {
        return CanonicalKind::Connector;
    }
    if s.contains("siren")
        || s.contains("buzzer")
        || s.contains("horn")
        || s.contains("heater")
        || s.contains("resistor")
        || cat.contains("consumers")
    {
        return CanonicalKind::Load;
    }
    if s.contains("splitter")
        || s.contains("verteiler")
        || s.contains("splice")
        || s.contains("distributor")
    {
        return CanonicalKind::Passive;
    }

    CanonicalKind::Passive
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
    kind: CanonicalKind,
    category: &str,
) -> Vec<String> {
    let mut tags = std::collections::BTreeSet::new();
    tags.insert(kind.as_str().to_string());

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
pub fn determine_overlays(kind: CanonicalKind) -> Vec<String> {
    match kind {
        CanonicalKind::Motor => vec![
            "fault".into(),
            "short_circuit".into(),
            "rotation".into(),
            "selected".into(),
        ],
        CanonicalKind::CircuitBreaker
        | CanonicalKind::Fuse
        | CanonicalKind::Disconnector
        | CanonicalKind::ThermalContact => {
            vec!["fault".into(), "short_circuit".into(), "selected".into()]
        }
        CanonicalKind::Lamp | CanonicalKind::RelayCoil | CanonicalKind::Load => {
            vec!["fault".into(), "selected".into()]
        }
        _ => vec!["selected".into()],
    }
}

/// Normaliza e garante a unicidade e preenchimento de nomes de terminais em elementos canônicos (P3-#17).
/// Segue as normas IEC 60947-1, IEC 60204-1, EN 50005 e IEC 81346:
/// - Bobinas (relay_coil): A1, A2 (e B1, B2 se dupla)
/// - Sinalizadores (lamp): X1, X2...
/// - Contatos térmicos (thermal_contact): 95, 96 (NF) ou 97, 98 (NA)
/// - Aparelhos de proteção e polos (circuit_breaker, fuse, disconnector, contactor_contact):
///   Polo 1: 1, 2; Polo 2: 3, 4; Polo 3: 5, 6; Polo 4: 7, 8
/// - Motores (motor): U, V, W, PE ou U1, V1, W1, U2, V2, W2
/// - Botoeiras e chaves: 11, 12 (NF) ou 13, 14 (NA)
/// - Fontes (source): L1, L2, L3, N, PE ou +, -
/// - Passivos, bornes e conectores: numeração sequencial 1, 2, 3...
/// - Desduplicação: qualquer duplicata restante recebe sufixo _2, _3...
pub fn normalize_canonical_terminals(
    kind: CanonicalKind,
    slug: &str,
    name_pt: &str,
    name_en: &str,
    terminals: &mut [TerminalMetadata],
) {
    if terminals.is_empty() || !kind.is_wireable() {
        return;
    }

    // Se todos os terminais já possuem nomes não-vazios e todos são mutuamente exclusivos, preserva.
    let all_non_empty = terminals.iter().all(|t| !t.name.trim().is_empty());
    if all_non_empty {
        let mut set = HashSet::new();
        let all_unique = terminals
            .iter()
            .all(|t| set.insert(t.name.trim().to_string()));
        if all_unique {
            return;
        }
    }

    let n = terminals.len();
    let name_l = format!("{} {} {}", slug, name_pt, name_en).to_ascii_lowercase();

    let is_nc = name_l.contains("nc")
        || name_l.contains("nf")
        || name_l.contains("fechado")
        || name_l.contains("ferme")
        || name_l.contains("geschlossen")
        || name_l.contains("break")
        || name_l.contains("emergency")
        || name_l.contains("parada")
        || name_l.contains("desarme");

    let is_no = name_l.contains("no")
        || name_l.contains("na")
        || name_l.contains("aberto")
        || name_l.contains("ouvert")
        || name_l.contains("schliesser")
        || name_l.contains("make")
        || name_l.contains("partida");

    let mut assigned: Vec<Option<String>> = vec![None; n];

    // Helper: índices ordenados por (y, x)
    let mut by_y_x: Vec<usize> = (0..n).collect();
    by_y_x.sort_by(|&a, &b| {
        terminals[a]
            .y
            .partial_cmp(&terminals[b].y)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                terminals[a]
                    .x
                    .partial_cmp(&terminals[b].x)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    // Helper: índices ordenados por (x, y)
    let mut by_x_y: Vec<usize> = (0..n).collect();
    by_x_y.sort_by(|&a, &b| {
        terminals[a]
            .x
            .partial_cmp(&terminals[b].x)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| {
                terminals[a]
                    .y
                    .partial_cmp(&terminals[b].y)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    });

    match kind {
        CanonicalKind::RelayCoil => {
            if n == 2 {
                assigned[by_y_x[0]] = Some("A1".to_string());
                assigned[by_y_x[1]] = Some("A2".to_string());
            } else if n == 3 {
                assigned[by_y_x[0]] = Some("A1".to_string());
                assigned[by_y_x[1]] = Some("A2".to_string());
                assigned[by_y_x[2]] = Some("A3".to_string());
            } else if n == 4 {
                assigned[by_x_y[0]] = Some("A1".to_string());
                assigned[by_x_y[1]] = Some("A2".to_string());
                assigned[by_x_y[2]] = Some("B1".to_string());
                assigned[by_x_y[3]] = Some("B2".to_string());
            }
        }
        CanonicalKind::Lamp => {
            if n == 2 {
                assigned[by_y_x[0]] = Some("X1".to_string());
                assigned[by_y_x[1]] = Some("X2".to_string());
            } else {
                for (r, &idx) in by_x_y.iter().enumerate() {
                    assigned[idx] = Some(format!("X{}", r + 1));
                }
            }
        }
        CanonicalKind::ThermalContact => {
            if n == 2 {
                if is_no && !is_nc {
                    assigned[by_y_x[0]] = Some("97".to_string());
                    assigned[by_y_x[1]] = Some("98".to_string());
                } else {
                    assigned[by_y_x[0]] = Some("95".to_string());
                    assigned[by_y_x[1]] = Some("96".to_string());
                }
            } else if n == 3 {
                assigned[by_y_x[0]] = Some("95".to_string());
                assigned[by_y_x[1]] = Some("96".to_string());
                assigned[by_y_x[2]] = Some("98".to_string());
            } else if n == 4 {
                assigned[by_x_y[0]] = Some("95".to_string());
                assigned[by_x_y[1]] = Some("96".to_string());
                assigned[by_x_y[2]] = Some("97".to_string());
                assigned[by_x_y[3]] = Some("98".to_string());
            }
        }
        CanonicalKind::Motor => {
            if n == 2 {
                assigned[by_y_x[0]] = Some("U1".to_string());
                assigned[by_y_x[1]] = Some("U2".to_string());
            } else if n == 3 {
                assigned[by_x_y[0]] = Some("U".to_string());
                assigned[by_x_y[1]] = Some("V".to_string());
                assigned[by_x_y[2]] = Some("W".to_string());
            } else if n == 4 {
                assigned[by_x_y[0]] = Some("U".to_string());
                assigned[by_x_y[1]] = Some("V".to_string());
                assigned[by_x_y[2]] = Some("W".to_string());
                assigned[by_x_y[3]] = Some("PE".to_string());
            } else if n == 6 {
                let mut top: Vec<usize> = by_y_x[..3].to_vec();
                let mut bot: Vec<usize> = by_y_x[3..].to_vec();
                top.sort_by(|&a, &b| terminals[a].x.partial_cmp(&terminals[b].x).unwrap());
                bot.sort_by(|&a, &b| terminals[a].x.partial_cmp(&terminals[b].x).unwrap());
                assigned[top[0]] = Some("U1".to_string());
                assigned[top[1]] = Some("V1".to_string());
                assigned[top[2]] = Some("W1".to_string());
                assigned[bot[0]] = Some("U2".to_string());
                assigned[bot[1]] = Some("V2".to_string());
                assigned[bot[2]] = Some("W2".to_string());
            }
        }
        CanonicalKind::MomentaryButton
        | CanonicalKind::LatchingSwitch
        | CanonicalKind::LimitSwitch
        | CanonicalKind::EmergencyStop
        | CanonicalKind::SelectorSwitch => {
            if n == 2 {
                if kind == CanonicalKind::EmergencyStop || (is_nc && !is_no) {
                    assigned[by_y_x[0]] = Some("11".to_string());
                    assigned[by_y_x[1]] = Some("12".to_string());
                } else {
                    assigned[by_y_x[0]] = Some("13".to_string());
                    assigned[by_y_x[1]] = Some("14".to_string());
                }
            } else if n == 3 {
                assigned[by_y_x[0]] = Some("11".to_string());
                assigned[by_y_x[1]] = Some("12".to_string());
                assigned[by_y_x[2]] = Some("14".to_string());
            } else if n == 4 {
                assigned[by_x_y[0]] = Some("13".to_string());
                assigned[by_x_y[1]] = Some("14".to_string());
                assigned[by_x_y[2]] = Some("21".to_string());
                assigned[by_x_y[3]] = Some("22".to_string());
            }
        }
        CanonicalKind::CircuitBreaker
        | CanonicalKind::Fuse
        | CanonicalKind::Disconnector
        | CanonicalKind::ContactorContact => {
            if n.is_multiple_of(2) {
                let num_poles = n / 2;
                let mut ys: Vec<f64> = terminals.iter().map(|t| t.y).collect();
                ys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                let median_y = ys[n / 2];

                let mut top_half: Vec<usize> =
                    (0..n).filter(|&i| terminals[i].y < median_y).collect();
                let mut bot_half: Vec<usize> =
                    (0..n).filter(|&i| terminals[i].y >= median_y).collect();

                if top_half.len() == num_poles && bot_half.len() == num_poles {
                    top_half.sort_by(|&a, &b| {
                        terminals[a]
                            .x
                            .partial_cmp(&terminals[b].x)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    bot_half.sort_by(|&a, &b| {
                        terminals[a]
                            .x
                            .partial_cmp(&terminals[b].x)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });

                    for p in 0..num_poles {
                        let in_pin = format!("{}", 2 * p + 1);
                        let out_pin = format!("{}", 2 * p + 2);
                        assigned[top_half[p]] = Some(in_pin);
                        assigned[bot_half[p]] = Some(out_pin);
                    }
                } else {
                    // Disposição horizontal (partição por x)
                    let mut xs: Vec<f64> = terminals.iter().map(|t| t.x).collect();
                    xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    let median_x = xs[n / 2];

                    let mut left_half: Vec<usize> =
                        (0..n).filter(|&i| terminals[i].x < median_x).collect();
                    let mut right_half: Vec<usize> =
                        (0..n).filter(|&i| terminals[i].x >= median_x).collect();

                    if left_half.len() == num_poles && right_half.len() == num_poles {
                        left_half.sort_by(|&a, &b| {
                            terminals[a]
                                .y
                                .partial_cmp(&terminals[b].y)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                        right_half.sort_by(|&a, &b| {
                            terminals[a]
                                .y
                                .partial_cmp(&terminals[b].y)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });

                        for p in 0..num_poles {
                            let in_pin = format!("{}", 2 * p + 1);
                            let out_pin = format!("{}", 2 * p + 2);
                            assigned[left_half[p]] = Some(in_pin);
                            assigned[right_half[p]] = Some(out_pin);
                        }
                    }
                }
            }
        }
        CanonicalKind::Source => {
            if n == 1 {
                if name_l.contains("pe") || name_l.contains("terra") || name_l.contains("earth") {
                    assigned[0] = Some("PE".to_string());
                } else if name_l.contains("neutro") || name_l.contains("neutral") {
                    assigned[0] = Some("N".to_string());
                } else if name_l.contains('-') || name_l.contains("negativo") {
                    assigned[0] = Some("-".to_string());
                } else if name_l.contains('+') || name_l.contains("positivo") {
                    assigned[0] = Some("+".to_string());
                } else {
                    assigned[0] = Some("L".to_string());
                }
            } else if n == 2 {
                if name_l.contains('+')
                    || name_l.contains('-')
                    || name_l.contains("cc")
                    || name_l.contains("dc")
                {
                    assigned[by_x_y[0]] = Some("+".to_string());
                    assigned[by_x_y[1]] = Some("-".to_string());
                } else {
                    assigned[by_x_y[0]] = Some("L".to_string());
                    assigned[by_x_y[1]] = Some("N".to_string());
                }
            } else if n == 3 {
                assigned[by_x_y[0]] = Some("L1".to_string());
                assigned[by_x_y[1]] = Some("L2".to_string());
                assigned[by_x_y[2]] = Some("L3".to_string());
            } else if n == 4 {
                assigned[by_x_y[0]] = Some("L1".to_string());
                assigned[by_x_y[1]] = Some("L2".to_string());
                assigned[by_x_y[2]] = Some("L3".to_string());
                if name_l.contains("pe") || name_l.contains("terra") || name_l.contains("earth") {
                    assigned[by_x_y[3]] = Some("PE".to_string());
                } else {
                    assigned[by_x_y[3]] = Some("N".to_string());
                }
            } else if n == 5 {
                assigned[by_x_y[0]] = Some("L1".to_string());
                assigned[by_x_y[1]] = Some("L2".to_string());
                assigned[by_x_y[2]] = Some("L3".to_string());
                assigned[by_x_y[3]] = Some("N".to_string());
                assigned[by_x_y[4]] = Some("PE".to_string());
            }
        }
        CanonicalKind::Terminal => {
            if n == 2 {
                assigned[by_y_x[0]] = Some("1".to_string());
                assigned[by_y_x[1]] = Some("2".to_string());
            } else {
                for (r, &idx) in by_y_x.iter().enumerate() {
                    assigned[idx] = Some(format!("{}", r + 1));
                }
            }
        }
        _ => {}
    }

    // Preserva e desambigua nomes existentes válidos (não vazios)
    let mut used_names = HashSet::new();
    for name in assigned.iter().flatten() {
        used_names.insert(name.clone());
    }

    let mut seen_existing: HashMap<String, usize> = HashMap::new();
    for idx in 0..n {
        let curr = terminals[idx].name.trim().to_string();
        if assigned[idx].is_none() && !curr.is_empty() {
            let count = seen_existing.entry(curr.clone()).or_insert(0);
            *count += 1;
            let cand = if *count == 1 && !used_names.contains(&curr) {
                curr
            } else {
                let mut suffix = *count;
                let mut disambiguated = format!("{}_{}", curr, suffix);
                while used_names.contains(&disambiguated) {
                    suffix += 1;
                    disambiguated = format!("{}_{}", curr, suffix);
                }
                disambiguated
            };
            used_names.insert(cand.clone());
            assigned[idx] = Some(cand);
        }
    }

    // Para qualquer terminal ainda não preenchido, atribui numeração sequencial 1, 2, 3...
    let mut counter = 1;
    for &idx in &by_y_x {
        if assigned[idx].is_none() {
            while used_names.contains(&format!("{counter}")) {
                counter += 1;
            }
            let cand = format!("{counter}");
            used_names.insert(cand.clone());
            assigned[idx] = Some(cand);
            counter += 1;
        }
    }

    // Passo final de garantia estrita de unicidade: disambigua duplicatas com _2, _3...
    let mut seen_counts: HashMap<String, usize> = HashMap::new();
    for idx in 0..n {
        let base_name = assigned[idx]
            .take()
            .unwrap_or_else(|| format!("{}", idx + 1));
        let count = seen_counts.entry(base_name.clone()).or_insert(0);
        *count += 1;
        if *count > 1 {
            terminals[idx].name = format!("{}_{}", base_name, *count);
        } else {
            terminals[idx].name = base_name;
        }
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
        let is_plc = is_plc_element(&meta.id, &meta.category, &name_en);
        let mut terminals = if is_plc {
            Vec::new()
        } else {
            meta.terminals.clone()
        };
        let kind = classify_element_kind(&meta.id, &meta.category, &name_en, terminals.len());
        if kind.is_wireable() && !terminals.is_empty() {
            normalize_canonical_terminals(kind, &meta.id, &name_pt, &name_en, &mut terminals);
        }
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
            kind,
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
            terminals,
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

        // Valida invariantes de bornes P3-#17 para elementos cabáveis
        if el.kind.is_wireable() {
            let mut names_set = HashSet::new();
            for term in &el.terminals {
                if term.name.trim().is_empty() {
                    errors.push(format!(
                        "Elemento {} ({}) possui borne com nome vazio",
                        el.id, el.slug
                    ));
                }
                if !names_set.insert(term.name.trim().to_string()) {
                    errors.push(format!(
                        "Elemento {} ({}) possui borne duplicado: {:?}",
                        el.id, el.slug, term.name
                    ));
                }
            }
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
    pub category_paths: HashMap<(String, String), String>,
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
        let mut category_paths = HashMap::with_capacity(catalog.categories.len());

        for cat in &catalog.categories {
            category_paths.insert((cat.family.clone(), cat.code.clone()), cat.path.clone());
        }

        for (idx, el) in catalog.elements.iter().enumerate() {
            elements_by_id.insert(el.id.clone(), idx);
            slug_to_id.insert(el.slug.clone(), el.id.clone());
        }

        Ok(Self {
            base_dir,
            catalog,
            elements_by_id,
            slug_to_id,
            category_paths,
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
    pub fn kind(&self, id_or_slug: &str) -> Option<CanonicalKind> {
        self.get(id_or_slug).map(|el| el.kind)
    }

    /// Busca textual rápida por query sobre id, slug, tags e nomes pt/en.
    pub fn search(&self, query: &str) -> Vec<&CanonicalElement> {
        self.filter_elements(query, None, None, CanonicalScope::All)
    }

    /// Verifica se um elemento corresponde a uma subdivisão temática (kind, família ou subcategoria).
    pub fn element_matches_subdivision(&self, el: &CanonicalElement, sub: &str) -> bool {
        let cat_path = self
            .category_paths
            .get(&(el.family.clone(), el.category.clone()))
            .map(|s| s.as_str())
            .unwrap_or("");
        let cat_path_lower = cat_path.to_ascii_lowercase();

        match sub {
            // Elétrica / Geral
            "motor" => {
                el.kind == CanonicalKind::Motor
                    || cat_path_lower.contains("engine")
                    || cat_path_lower.contains("motor")
            }
            "circuit_breaker" => {
                el.kind == CanonicalKind::CircuitBreaker
                    || el.kind == CanonicalKind::Disconnector
                    || cat_path_lower.contains("breaker")
                    || cat_path_lower.contains("disjoncteur")
                    || cat_path_lower.contains("disjuntor")
            }
            "fuse" => {
                el.kind == CanonicalKind::Fuse
                    || cat_path_lower.contains("fuse")
                    || cat_path_lower.contains("sicherung")
                    || cat_path_lower.contains("fusivel")
            }
            "relay_coil" => {
                el.kind == CanonicalKind::RelayCoil
                    || cat_path_lower.contains("coil")
                    || cat_path_lower.contains("bobina")
                    || cat_path_lower.contains("rele")
            }
            "contactor_contact" => {
                el.kind == CanonicalKind::ContactorContact
                    || el.kind == CanonicalKind::ThermalContact
                    || cat_path_lower.contains("contact")
                    || cat_path_lower.contains("contato")
            }
            "momentary_button" => {
                el.kind == CanonicalKind::MomentaryButton
                    || el.kind == CanonicalKind::EmergencyStop
                    || cat_path_lower.contains("button")
                    || cat_path_lower.contains("pulsador")
                    || cat_path_lower.contains("botao")
            }
            "latching_switch" => {
                el.kind == CanonicalKind::LatchingSwitch
                    || el.kind == CanonicalKind::SelectorSwitch
                    || el.kind == CanonicalKind::LimitSwitch
                    || cat_path_lower.contains("switch")
                    || cat_path_lower.contains("schalter")
                    || cat_path_lower.contains("chave")
            }
            "lamp" => {
                el.kind == CanonicalKind::Lamp
                    || cat_path_lower.contains("light")
                    || cat_path_lower.contains("lamp")
                    || cat_path_lower.contains("voyant")
                    || cat_path_lower.contains("sinaleiro")
            }
            "source" => {
                el.kind == CanonicalKind::Source
                    || cat_path_lower.contains("supplies")
                    || cat_path_lower.contains("source")
                    || cat_path_lower.contains("battery")
                    || cat_path_lower.contains("alimenta")
            }
            "terminal" => {
                el.kind == CanonicalKind::Terminal
                    || el.kind == CanonicalKind::Connector
                    || cat_path_lower.contains("terminal")
                    || cat_path_lower.contains("connector")
                    || cat_path_lower.contains("borne")
            }
            "load" => {
                el.kind == CanonicalKind::Load
                    || cat_path_lower.contains("consumer")
                    || cat_path_lower.contains("resistor")
                    || cat_path_lower.contains("heater")
            }
            "block" => el.kind == CanonicalKind::Block || !el.is_wireable(),

            // Pneumática
            "pneu_valves" => {
                cat_path_lower.contains("velve")
                    || cat_path_lower.contains("valve")
                    || cat_path_lower.contains("distributor")
                    || el.slug.contains("valve")
            }
            "pneu_air" => {
                cat_path_lower.contains("compressed_air")
                    || cat_path_lower.contains("air_generation")
                    || cat_path_lower.contains("tank")
                    || cat_path_lower.contains("treatment")
                    || cat_path_lower.contains("filter")
                    || cat_path_lower.contains("pipes")
            }
            "pneu_actuators" => {
                cat_path_lower.contains("actuator")
                    || cat_path_lower.contains("cylinder")
                    || el.slug.contains("cylinder")
            }
            "pneu_sensors" => {
                cat_path_lower.contains("sensor")
                    || cat_path_lower.contains("indicator")
                    || el.slug.contains("sensor")
            }

            // Hidráulica
            "hyd_valves" => cat_path_lower.contains("valve") || el.slug.contains("valve"),
            "hyd_pumps" => {
                cat_path_lower.contains("pump")
                    || el.kind == CanonicalKind::Motor
                    || el.slug.contains("pump")
            }
            "hyd_cylinders" => cat_path_lower.contains("cylinder") || el.slug.contains("cylinder"),
            "hyd_tanks" => cat_path_lower.contains("tank") || el.slug.contains("tank"),
            "hyd_filters" => {
                cat_path_lower.contains("filter") || cat_path_lower.contains("exchanger")
            }

            // Lógica
            "logic_gates" => cat_path_lower.contains("logic_gates") || el.slug.contains("gate"),
            "logic_ladder" => cat_path_lower.contains("ladder") || el.slug.contains("ladder"),
            "logic_flowchart" => {
                cat_path_lower.contains("flow_chart") || cat_path_lower.contains("grafcet")
            }

            // Energia
            "energy_water" => {
                cat_path_lower.contains("water")
                    || cat_path_lower.contains("calefaccion")
                    || cat_path_lower.contains("agua")
            }
            "energy_refrig" => {
                cat_path_lower.contains("refrigeration") || cat_path_lower.contains("refrig")
            }
            "energy_solar" => cat_path_lower.contains("solar"),
            "energy_mfg" => cat_path_lower.contains("manufacturers"),

            // CADe nativos
            "cade_power" => el.kind == CanonicalKind::Source,
            "cade_protection" => {
                el.kind == CanonicalKind::CircuitBreaker
                    || el.kind == CanonicalKind::Fuse
                    || el.kind == CanonicalKind::Disconnector
            }
            "cade_control" => {
                el.kind == CanonicalKind::ContactorContact
                    || el.kind == CanonicalKind::RelayCoil
                    || el.kind == CanonicalKind::ThermalContact
            }
            "cade_motors" => el.kind == CanonicalKind::Motor,
            "cade_switches" => {
                el.kind == CanonicalKind::MomentaryButton
                    || el.kind == CanonicalKind::LatchingSwitch
                    || el.kind == CanonicalKind::SelectorSwitch
            }
            "cade_signaling" => el.kind == CanonicalKind::Lamp,

            // Fallback genérico: compara com kind ou substring no path ou slug
            other => {
                el.kind.as_str().eq_ignore_ascii_case(other)
                    || cat_path_lower.contains(&other.to_ascii_lowercase())
                    || el.slug.contains(&other.to_ascii_lowercase())
            }
        }
    }

    /// Filtra elementos por query de busca opcional, família opcional, subdivisão temática opcional e escopo.
    pub fn filter_elements(
        &self,
        query: &str,
        family: Option<&str>,
        subdivision: Option<&str>,
        scope: CanonicalScope,
    ) -> Vec<&CanonicalElement> {
        let q = query.trim().to_lowercase();
        self.catalog
            .elements
            .iter()
            .filter(|el| {
                match scope {
                    CanonicalScope::All => {}
                    CanonicalScope::Wireable => {
                        if !el.is_wireable() {
                            return false;
                        }
                    }
                    CanonicalScope::Block => {
                        if el.is_wireable() {
                            return false;
                        }
                    }
                }
                if let Some(f) = family {
                    if !el.family.eq_ignore_ascii_case(f) {
                        return false;
                    }
                }
                if let Some(sub) = subdivision {
                    if !self.element_matches_subdivision(el, sub) {
                        return false;
                    }
                }
                if q.is_empty() {
                    return true;
                }
                el.id.to_lowercase().contains(&q)
                    || el.slug.to_lowercase().contains(&q)
                    || el.name_pt.to_lowercase().contains(&q)
                    || el.name_en.to_lowercase().contains(&q)
                    || el.tags.iter().any(|t| t.to_lowercase().contains(&q))
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
