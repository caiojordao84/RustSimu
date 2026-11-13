use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::canonical::CanonicalKind;
use crate::model::SchematicItem;

/// Domínio funcional ao qual um símbolo canônico do RustSimu pertence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SimulationDomain {
    ElectricPower,
    ElectricCommand,
    Pneumatic,
    Electropneumatic,
    Hydraulic,
    LadderLogic,
    PanelBlock,
    Annotation,
}

impl SimulationDomain {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::ElectricPower => "electric_power",
            Self::ElectricCommand => "electric_command",
            Self::Pneumatic => "pneumatic",
            Self::Electropneumatic => "electropneumatic",
            Self::Hydraulic => "hydraulic",
            Self::LadderLogic => "ladder_logic",
            Self::PanelBlock => "panel_block",
            Self::Annotation => "annotation",
        }
    }

    pub const fn label_pt(&self) -> &'static str {
        match self {
            Self::ElectricPower => "Potência Elétrica",
            Self::ElectricCommand => "Comando Elétrico",
            Self::Pneumatic => "Pneumática",
            Self::Electropneumatic => "Eletropneumática",
            Self::Hydraulic => "Hidráulica",
            Self::LadderLogic => "Lógica Ladder",
            Self::PanelBlock => "Bloco de Montagem",
            Self::Annotation => "Anotação Vetorial",
        }
    }
}

/// Forma de contato elétrico/lógico de atuadores e contatos auxiliares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactForm {
    No,
    Nc,
    NoNc,
    Changeover,
    ThreeWay,
}

/// Regime temporal/mecânico de bobinas e acionamentos.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoilBehavior {
    Instantaneous,
    OnDelay,
    OffDelay,
    Cyclic,
    Latching,
}

/// Nível de certeza com que o parser resolveu um elemento externo para o conceito canônico.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ResolutionConfidence {
    SemanticLexical,
    ExactQetAlias,
    ExactCadeTipo,
    ExactSymbolId,
    ExactConceptId,
}

/// Conceito semântico único do RustSimu (língua franca entre formatos internos e externos).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalConcept {
    pub concept_id: String,
    pub domain: SimulationDomain,
    pub kind: CanonicalKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_form: Option<ContactForm>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub coil_behavior: Option<CoilBehavior>,
    pub poles: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_symbol_id: Option<String>,
    #[serde(default)]
    pub variant_symbol_ids: Vec<String>,
    pub default_tag_prefix: String,
    pub canonical_pins: Vec<String>,
    pub name_pt: String,
    pub name_en: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cade_tipo: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qet_primary_path: Option<String>,
    #[serde(default)]
    pub qet_aliases: Vec<String>,
    /// Classe `OCO(...)` do FluidSIM (`ep.kb`), quando o conceito veio do catálogo pneumático.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fluidsim_class: Option<String>,
    /// Se o motor pneumático v1 deve simular este conceito.
    #[serde(default = "default_simular_true")]
    pub simular: bool,
}

fn default_simular_true() -> bool {
    true
}

/// Resultado da resolução semântica de um item ou assinatura externa.
#[derive(Debug, Clone)]
pub struct ResolvedConcept<'a> {
    pub concept: &'a CanonicalConcept,
    pub confidence: ResolutionConfidence,
    /// Mapeamento posicional `(índice_pino, nome_canônico)` quando os pinos externos diferem do padrão.
    pub pin_remap: Vec<(usize, String)>,
}

/// Arquivo JSON opcional `concepts.json` para extensão declarativa do dicionário.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConceptExtensionFile {
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub concepts: Vec<CanonicalConcept>,
}

/// Dicionário e parser bidirecional entre formatos externos (`.cad`, `.qet`, futuros domínios)
/// e o modelo conceitual unificado do RustSimu.
pub struct ConceptDictionary {
    concepts: Vec<CanonicalConcept>,
    by_concept_id: HashMap<String, usize>,
    by_cade_tipo: HashMap<i32, usize>,
    by_symbol_id: HashMap<String, usize>,
    by_qet_alias: HashMap<String, usize>,
    by_fluidsim_class: HashMap<String, usize>,
}

impl ConceptDictionary {
    /// Constrói o dicionário com os conceitos nativos do RustSimu/CADe_SIMU, aliases QET base
    /// e âncoras multimodais (pneumática, hidráulica e ladder).
    pub fn build_builtin() -> Self {
        let mut dict = Self {
            concepts: Vec::new(),
            by_concept_id: HashMap::new(),
            by_cade_tipo: HashMap::new(),
            by_symbol_id: HashMap::new(),
            by_qet_alias: HashMap::new(),
            by_fluidsim_class: HashMap::new(),
        };
        for concept in builtin_concepts() {
            dict.register(concept);
        }
        // Catálogo FluidSIM: novos conceitos pneumáticos; elétricos já existentes
        // só ganham índice `fluidsim_class` (sem sobrescrever o conceito CADe).
        for concept in crate::fluidsim_kb::fluidsim_concepts() {
            if dict.by_concept_id.contains_key(&concept.concept_id) {
                if let Some(ref fc) = concept.fluidsim_class {
                    if let Some(&idx) = dict.by_concept_id.get(&concept.concept_id) {
                        dict.by_fluidsim_class.insert(fc.clone(), idx);
                    }
                }
                continue;
            }
            dict.register(concept);
        }
        dict
    }

    /// Carrega extensões opcionais de `concepts.json` se presente junto ao catálogo.
    pub fn load_with_optional_file(concepts_json_path: Option<&Path>) -> Self {
        let mut dict = Self::build_builtin();
        if let Some(path) = concepts_json_path {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(ext) = serde_json::from_str::<ConceptExtensionFile>(&content) {
                    for concept in ext.concepts {
                        dict.register(concept);
                    }
                }
            }
        }
        dict
    }

    /// Registra ou sobrescreve um conceito e atualiza todos os índices O(1).
    pub fn register(&mut self, concept: CanonicalConcept) {
        let idx = if let Some(&existing_idx) = self.by_concept_id.get(&concept.concept_id) {
            self.concepts[existing_idx] = concept;
            existing_idx
        } else {
            let idx = self.concepts.len();
            self.by_concept_id.insert(concept.concept_id.clone(), idx);
            self.concepts.push(concept);
            idx
        };

        let c = &self.concepts[idx];
        if let Some(tipo) = c.cade_tipo {
            self.by_cade_tipo.entry(tipo).or_insert(idx);
        }
        if let Some(ref sym_id) = c.default_symbol_id {
            self.by_symbol_id
                .insert(sym_id.trim().to_ascii_uppercase(), idx);
        }
        for var_id in &c.variant_symbol_ids {
            self.by_symbol_id
                .insert(var_id.trim().to_ascii_uppercase(), idx);
        }
        if let Some(ref primary) = c.qet_primary_path {
            self.by_qet_alias.insert(normalize_qet_key(primary), idx);
        }
        for alias in &c.qet_aliases {
            self.by_qet_alias.insert(normalize_qet_key(alias), idx);
        }
        if let Some(ref fc) = c.fluidsim_class {
            self.by_fluidsim_class
                .insert(fc.trim().to_ascii_uppercase(), idx);
        }
    }

    pub fn get_by_fluidsim_class(&self, classe: &str) -> Option<&CanonicalConcept> {
        self.by_fluidsim_class
            .get(classe.trim())
            .map(|&idx| &self.concepts[idx])
    }

    pub fn concepts(&self) -> &[CanonicalConcept] {
        &self.concepts
    }

    pub fn get_by_id(&self, concept_id: &str) -> Option<&CanonicalConcept> {
        self.by_concept_id
            .get(concept_id.trim())
            .map(|&idx| &self.concepts[idx])
    }

    pub fn get_by_cade_tipo(&self, tipo: i32) -> Option<&CanonicalConcept> {
        self.by_cade_tipo.get(&tipo).map(|&idx| &self.concepts[idx])
    }

    /// Resolve um `SchematicItem` para o conceito canônico único do RustSimu seguindo
    /// precedência determinística: `concept_id` -> `symbol_id` -> `cade_tipo` -> `qet_type`.
    pub fn resolve_item(&self, item: &SchematicItem) -> Option<ResolvedConcept<'_>> {
        if let Some(ref cid) = item.concept_id {
            if let Some(concept) = self.get_by_id(cid) {
                return Some(ResolvedConcept {
                    concept,
                    confidence: ResolutionConfidence::ExactConceptId,
                    pin_remap: compute_pin_remap(&item.pins, &concept.canonical_pins),
                });
            }
        }

        if let Some(ref sym_id) = item.symbol_id {
            let key = sym_id.trim().to_ascii_uppercase();
            if let Some(&idx) = self
                .by_symbol_id
                .get(&key)
                .or_else(|| self.by_fluidsim_class.get(&key))
            {
                let concept = &self.concepts[idx];
                return Some(ResolvedConcept {
                    concept,
                    confidence: ResolutionConfidence::ExactSymbolId,
                    pin_remap: compute_pin_remap(&item.pins, &concept.canonical_pins),
                });
            }
        }

        if item.tipo != 20000 && !item.is_wire() && !item.is_junction() && !item.is_graphic() {
            if let Some(concept) = self.get_by_cade_tipo(item.tipo) {
                return Some(ResolvedConcept {
                    concept,
                    confidence: ResolutionConfidence::ExactCadeTipo,
                    pin_remap: compute_pin_remap(&item.pins, &concept.canonical_pins),
                });
            }
        }

        if let Some(qet_or_slug) = item.symbol_id_or_slug() {
            return self.resolve_external_qet(qet_or_slug, None, &item.pins);
        }

        if item.tipo == 20000 && !item.trailing_str.trim().is_empty() {
            return self.resolve_external_qet(&item.trailing_str, None, &item.pins);
        }

        None
    }

    /// Resolve uma assinatura externa vinda de `.qet`/`.elmt` ou slug de biblioteca para um conceito.
    pub fn resolve_external_qet(
        &self,
        qet_path_or_slug: &str,
        name_hint: Option<&str>,
        source_pins: &[String],
    ) -> Option<ResolvedConcept<'_>> {
        let norm = normalize_qet_key(qet_path_or_slug);
        if let Some(&idx) = self.by_qet_alias.get(&norm) {
            let concept = &self.concepts[idx];
            return Some(ResolvedConcept {
                concept,
                confidence: ResolutionConfidence::ExactQetAlias,
                pin_remap: compute_pin_remap(source_pins, &concept.canonical_pins),
            });
        }

        if let Some(concept_id) =
            infer_concept_id_from_tokens(qet_path_or_slug, name_hint, source_pins)
        {
            if let Some(concept) = self.get_by_id(concept_id) {
                return Some(ResolvedConcept {
                    concept,
                    confidence: ResolutionConfidence::SemanticLexical,
                    pin_remap: compute_pin_remap(source_pins, &concept.canonical_pins),
                });
            }
        }

        if let Some(stem) = extract_qet_stem(&norm) {
            if let Some(&idx) = self.by_qet_alias.get(&stem) {
                let concept = &self.concepts[idx];
                return Some(ResolvedConcept {
                    concept,
                    confidence: ResolutionConfidence::ExactQetAlias,
                    pin_remap: compute_pin_remap(source_pins, &concept.canonical_pins),
                });
            }
        }

        None
    }

    /// Retorna o código `tipo` equivalente no CADe_SIMU 4.0 ao exportar um item.
    /// Retorna `None` para itens de domínios sem representação nativa no `.cad` (ex.: pneumática/ladder).
    pub fn resolve_for_cade_export(&self, item: &SchematicItem) -> Option<i32> {
        if item.tipo != 20000 && !item.is_wire() && !item.is_junction() {
            return Some(item.tipo);
        }
        self.resolve_item(item).and_then(|r| r.concept.cade_tipo)
    }

    /// Retorna o caminho QElectroTech padrão (`.elmt`) equivalente ao exportar um item para `.qet`.
    pub fn resolve_for_qet_export<'a>(&'a self, item: &'a SchematicItem) -> Option<&'a str> {
        if let Some(ref qt) = item.qet_type {
            if qt.contains('/') || qt.ends_with(".elmt") || qt.starts_with("embed://") {
                return Some(qt.as_str());
            }
        }
        self.resolve_item(item)
            .and_then(|r| r.concept.qet_primary_path.as_deref())
    }
}

/// Dicionário global em memória para resolução semântica O(1).
pub fn global_concept_dictionary() -> &'static ConceptDictionary {
    static GLOBAL_CONCEPTS: OnceLock<ConceptDictionary> = OnceLock::new();
    GLOBAL_CONCEPTS.get_or_init(|| {
        let custom_path = crate::canonical::global_dictionary()
            .map(|d| d.base_dir.join("concepts.json"))
            .filter(|p| p.exists());
        ConceptDictionary::load_with_optional_file(custom_path.as_deref())
    })
}

/// Normaliza caminhos QET (`embed://`, barras invertidas, maiúsculas e `.elmt`).
pub fn normalize_qet_key(raw: &str) -> String {
    raw.trim()
        .replace('\\', "/")
        .trim_start_matches("embed://")
        .trim_start_matches("import/")
        .trim_start_matches("qet:")
        .to_ascii_lowercase()
}

fn extract_qet_stem(path: &str) -> Option<String> {
    let last = path.rsplit('/').next()?.trim_end_matches(".elmt").trim();
    if last.is_empty() {
        None
    } else {
        Some(last.to_string())
    }
}

fn compute_pin_remap(source_pins: &[String], canonical_pins: &[String]) -> Vec<(usize, String)> {
    let mut remap = Vec::new();
    let non_empty_src: Vec<(usize, &str)> = source_pins
        .iter()
        .enumerate()
        .map(|(idx, p)| (idx, p.trim()))
        .filter(|(_, p)| !p.is_empty())
        .collect();

    if canonical_pins.is_empty() {
        return remap;
    }

    if non_empty_src.is_empty() {
        for (idx, canon) in canonical_pins.iter().enumerate() {
            remap.push((idx, canon.clone()));
        }
        return remap;
    }

    if non_empty_src.len() == canonical_pins.len() {
        for ((idx, src), canon) in non_empty_src.iter().zip(canonical_pins.iter()) {
            if src != canon {
                remap.push((*idx, canon.clone()));
            }
        }
    }
    remap
}

fn tokenize_semantic_words(raw: &str) -> Vec<String> {
    normalize_qet_key(raw)
        .trim_end_matches(".elmt")
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .map(|w| w.to_string())
        .collect()
}

fn has_token(tokens: &[String], candidates: &[&str]) -> bool {
    tokens.iter().any(|tok| candidates.iter().any(|c| tok == c))
}

fn has_stem(joined: &str, stems: &[&str]) -> bool {
    stems.iter().any(|s| joined.contains(s))
}

fn detect_contact_form(tokens: &[String], joined: &str) -> Option<ContactForm> {
    let has_no = has_token(tokens, &["no", "na", "schliesser", "travail", "open"]);
    let has_nc = has_token(
        tokens,
        &["nc", "nf", "oeffner", "offner", "repos", "closed"],
    );
    let has_co = has_token(
        tokens,
        &[
            "co",
            "con",
            "com",
            "spdt",
            "changeover",
            "inverseur",
            "wechsler",
        ],
    ) || joined.contains("two-way");

    if has_co || (has_no && has_nc && joined.contains("changeover")) {
        Some(ContactForm::Changeover)
    } else if has_no && has_nc {
        Some(ContactForm::NoNc)
    } else if has_nc || joined.contains("normally_closed") {
        Some(ContactForm::Nc)
    } else if has_no || joined.contains("normally_open") {
        Some(ContactForm::No)
    } else {
        None
    }
}

/// Parser léxico-semântico multi-idioma (PT / EN / FR / DE / ES) para símbolos externos sem alias exato.
fn infer_concept_id_from_tokens(
    path_or_slug: &str,
    name_hint: Option<&str>,
    pins: &[String],
) -> Option<&'static str> {
    let mut tokens = tokenize_semantic_words(path_or_slug);
    if let Some(hint) = name_hint {
        tokens.extend(tokenize_semantic_words(hint));
    }
    let joined = format!(
        "{} {}",
        normalize_qet_key(path_or_slug),
        name_hint.unwrap_or("").to_ascii_lowercase()
    );
    let pin_count = pins.iter().filter(|p| !p.trim().is_empty()).count();
    let form = detect_contact_form(&tokens, &joined);

    // 1. Domínios futuros explícitos (Pneumática, Hidráulica, Ladder)
    if joined.contains("pneumatic")
        || joined.contains("pneumatik")
        || joined.contains("compressed_air")
    {
        if let Some(cid) = crate::pneu::qet_map::concept_id_from_qet_pneumatic(path_or_slug) {
            return Some(cid);
        }
        if let Some(hint) = name_hint {
            if let Some(cid) = crate::pneu::qet_map::concept_id_from_qet_pneumatic(hint) {
                return Some(cid);
            }
        }
        if has_stem(
            &joined,
            &["cylinder", "actuator", "verin", "zylinder", "cilindro"],
        ) {
            return Some(if has_stem(&joined, &["double", "2_way"]) {
                "pneu.cylinder.double"
            } else {
                "pneu.cylinder.single"
            });
        }
        if has_stem(&joined, &["valve", "ventil", "distributeur", "valvula"]) {
            return Some(
                if joined.contains("5_2") || joined.contains("5-2") || joined.contains("5/2") {
                    "pneu.valve.5_2"
                } else {
                    "pneu.valve.3_2"
                },
            );
        }
        if has_stem(&joined, &["compressor", "air_supply", "source"]) {
            return Some("pneu.source.air");
        }
    }

    if joined.contains("hydraulic") || joined.contains("hydraulik") {
        if has_stem(&joined, &["pump", "bomba", "pompe"]) {
            return Some("hyd.pump");
        }
        if has_stem(&joined, &["tank", "reservoir"]) {
            return Some("hyd.tank");
        }
        if has_stem(&joined, &["cylinder", "verin", "cilindro"]) {
            return Some("hyd.cylinder");
        }
        if has_stem(&joined, &["valve", "ventil", "distributeur", "valvula"]) {
            return Some("hyd.valve.dir");
        }
    }

    if joined.contains("ladder") {
        if has_stem(&joined, &["coil", "bobine", "spule", "bobina"]) {
            return Some("ladder.coil");
        }
        return Some(match form {
            Some(ContactForm::Nc) => "ladder.contact.nc",
            _ => "ladder.contact.no",
        });
    }

    // 2. Emergência
    if has_stem(
        &joined,
        &[
            "emergency",
            "notaus",
            "arret_urgence",
            "emergencia",
            "cogumelo",
            "mushroom",
        ],
    ) {
        return Some(match form {
            Some(ContactForm::No) => "el.act.estop.no",
            Some(ContactForm::NoNc) => "el.act.estop.no_nc",
            _ => "el.act.estop.nc",
        });
    }

    // 3. Fim de curso / microswitch
    if has_stem(
        &joined,
        &[
            "limitswitch",
            "limit_switch",
            "endschalter",
            "microswitch",
            "fin_de_course",
            "fim_de_curso",
        ],
    ) {
        return Some(match form {
            Some(ContactForm::Nc) => "el.act.limit_switch.nc",
            Some(ContactForm::NoNc) => "el.act.limit_switch.no_nc",
            Some(ContactForm::Changeover) => "el.act.limit_switch.co",
            _ => "el.act.limit_switch.no",
        });
    }

    // 4. Botões pulsadores monoestáveis
    if has_stem(
        &joined,
        &[
            "push_button",
            "push-button",
            "pushbutton",
            "bouton_poussoir",
            "drucktaster",
            "taster",
            "pulsador",
            "horn_switch",
        ],
    ) {
        return Some(match form {
            Some(ContactForm::Nc) => "el.act.pb.nc",
            Some(ContactForm::NoNc) => "el.act.pb.no_nc",
            Some(ContactForm::Changeover) => "el.act.pb.co",
            _ => "el.act.pb.no",
        });
    }

    // 5. Chaves seletoras / interruptores
    if has_stem(
        &joined,
        &[
            "selector",
            "rotative",
            "interrupteur",
            "schalter",
            "ignition",
        ],
    ) {
        return Some(match form {
            Some(ContactForm::Nc) => "el.act.selector.nc",
            Some(ContactForm::NoNc) => "el.act.selector.no_nc",
            Some(ContactForm::Changeover) => "el.act.selector.co",
            _ => "el.act.selector.no",
        });
    }

    // 6. Contatos auxiliares / temporizados / térmicos
    if has_stem(
        &joined,
        &["thermal", "thermique", "overcurrent", "surcharge"],
    ) && has_stem(&joined, &["contact", "relais"])
    {
        return Some(match form {
            Some(ContactForm::No) => "el.aux.thermal_contact.no",
            Some(ContactForm::NoNc) => "el.aux.thermal_contact.no_nc",
            Some(ContactForm::Changeover) => "el.aux.thermal_contact.co",
            _ => "el.aux.thermal_contact.nc",
        });
    }

    if has_stem(
        &joined,
        &[
            "contactor",
            "contacteur",
            "schuetz",
            "schütz",
            "power_contactor",
        ],
    ) && pin_count >= 5
    {
        return Some("el.pwr.contactor.3p");
    }
    if has_stem(&joined, &["contactor", "contacteur", "power_contactor"]) && pin_count == 4 {
        return Some("el.pwr.contactor.2p");
    }

    if has_stem(&joined, &["contact"]) {
        return Some(match form {
            Some(ContactForm::Nc) => "el.aux.contact.nc",
            Some(ContactForm::NoNc) => "el.aux.contact.no_nc",
            Some(ContactForm::Changeover) => "el.aux.contact.no",
            _ => "el.aux.contact.no",
        });
    }

    // 7. Bobinas
    if has_stem(&joined, &["coil", "bobine", "spule", "bobina"]) {
        if has_stem(&joined, &["on_delay", "on-delay", "travail", "anzugsverz"]) {
            return Some("el.coil.timer.on_delay");
        }
        if has_stem(&joined, &["off_delay", "off-delay", "repos", "abfallverz"]) {
            return Some("el.coil.timer.off_delay");
        }
        return Some("el.coil.contactor");
    }

    // 8. Fusíveis & disjuntores
    if has_stem(&joined, &["fuse", "fusible", "sicherung", "fusivel"]) {
        return Some(if pin_count >= 5 {
            "el.prot.fuse.3p"
        } else {
            "el.prot.fuse.1p"
        });
    }
    if has_stem(
        &joined,
        &[
            "circuit_breaker",
            "circuit-breaker",
            "disjoncteur",
            "disjuntor",
            "schutzschalter",
            "magneto-thermal",
        ],
    ) {
        return Some(if pin_count >= 5 {
            "el.prot.mcb.3p"
        } else if pin_count >= 3 {
            "el.prot.mcb.2p"
        } else {
            "el.prot.mcb.1p"
        });
    }

    // 9. Sinalização e alarmes
    if has_stem(&joined, &["horn", "buzzer", "sirene", "klaxon", "hupe"]) {
        return Some("el.sig.buzzer");
    }
    if has_stem(
        &joined,
        &[
            "lamp",
            "lampe",
            "voyant",
            "light",
            "leuchte",
            "sinaleiro",
            "piloto",
        ],
    ) {
        return Some("el.sig.lamp");
    }

    // 10. Motores e fontes
    if has_stem(&joined, &["motor", "moteur", "starter_motor", "servomotor"]) {
        return Some(if pin_count >= 6 {
            "el.load.motor.3ph_6t"
        } else if pin_count >= 3 {
            "el.load.motor.3ph_3t"
        } else {
            "el.load.motor.1ph"
        });
    }
    if has_stem(
        &joined,
        &[
            "chassis_ground",
            "ground",
            "earth",
            "masse",
            "terra",
            "erde",
        ],
    ) {
        return Some("el.src.pe");
    }
    if has_stem(
        &joined,
        &["battery", "autobatterie", "terminal_plus", "vcc", "dc_pos"],
    ) {
        return Some("el.src.dc_pos");
    }
    if has_stem(
        &joined,
        &["terminal", "bornier", "borne", "klemme", "stecker"],
    ) {
        return Some("el.conn.terminal");
    }

    None
}

struct ConceptSeed {
    concept_id: &'static str,
    domain: SimulationDomain,
    kind: CanonicalKind,
    contact_form: Option<ContactForm>,
    coil_behavior: Option<CoilBehavior>,
    poles: u8,
    default_symbol_id: Option<&'static str>,
    variant_symbol_ids: &'static [&'static str],
    default_tag_prefix: &'static str,
    canonical_pins: &'static [&'static str],
    name_pt: &'static str,
    name_en: &'static str,
    cade_tipo: Option<i32>,
    qet_primary_path: Option<&'static str>,
    qet_aliases: &'static [&'static str],
}

impl From<ConceptSeed> for CanonicalConcept {
    fn from(s: ConceptSeed) -> Self {
        Self {
            concept_id: s.concept_id.to_string(),
            domain: s.domain,
            kind: s.kind,
            contact_form: s.contact_form,
            coil_behavior: s.coil_behavior,
            poles: s.poles,
            default_symbol_id: s.default_symbol_id.map(str::to_string),
            variant_symbol_ids: s
                .variant_symbol_ids
                .iter()
                .map(|v| (*v).to_string())
                .collect(),
            default_tag_prefix: s.default_tag_prefix.to_string(),
            canonical_pins: s.canonical_pins.iter().map(|p| (*p).to_string()).collect(),
            name_pt: s.name_pt.to_string(),
            name_en: s.name_en.to_string(),
            cade_tipo: s.cade_tipo,
            qet_primary_path: s.qet_primary_path.map(str::to_string),
            qet_aliases: s.qet_aliases.iter().map(|a| (*a).to_string()).collect(),
            fluidsim_class: None,
            simular: true,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn seed(
    concept_id: &'static str,
    domain: SimulationDomain,
    kind: CanonicalKind,
    contact_form: Option<ContactForm>,
    coil_behavior: Option<CoilBehavior>,
    poles: u8,
    default_symbol_id: Option<&'static str>,
    variant_symbol_ids: &'static [&'static str],
    default_tag_prefix: &'static str,
    canonical_pins: &'static [&'static str],
    name_pt: &'static str,
    name_en: &'static str,
    cade_tipo: Option<i32>,
    qet_primary_path: Option<&'static str>,
    qet_aliases: &'static [&'static str],
) -> CanonicalConcept {
    ConceptSeed {
        concept_id,
        domain,
        kind,
        contact_form,
        coil_behavior,
        poles,
        default_symbol_id,
        variant_symbol_ids,
        default_tag_prefix,
        canonical_pins,
        name_pt,
        name_en,
        cade_tipo,
        qet_primary_path,
        qet_aliases,
    }
    .into()
}

fn builtin_concepts() -> Vec<CanonicalConcept> {
    use CanonicalKind as K;
    use CoilBehavior as B;
    use ContactForm as F;
    use SimulationDomain as D;

    vec![
        // Alimentações (3000..=3009)
        seed("el.src.phase_l", D::ElectricPower, K::Source, None, None, 1, Some("00009"), &[], "-X", &["L"], "Alimentação Fase (L)", "Phase Supply (L)", Some(3000), Some("embed://import/electric/10_allpole/100_network/phase_l.elmt"), &[]),
        seed("el.src.neutral_n", D::ElectricPower, K::Source, None, None, 1, Some("0000A"), &[], "-X", &["N"], "Alimentação Neutro (N)", "Neutral Supply (N)", Some(3001), Some("embed://import/electric/10_allpole/100_network/neutral_n.elmt"), &[]),
        seed("el.src.pe", D::ElectricPower, K::Source, None, None, 1, Some("0000B"), &[], "-PE", &["PE"], "Proteção Terra (PE)", "Protective Earth (PE)", Some(3002), Some("embed://import/electric/10_allpole/100_network/earth_pe.elmt"), &["chassis_ground", "power/chassis_ground.elmt"]),
        seed("el.src.l_n", D::ElectricPower, K::Source, None, None, 2, Some("0000C"), &[], "-X", &["L", "N"], "Alimentação Monofásica (L+N)", "Single-Phase Supply (L+N)", Some(3003), None, &[]),
        seed("el.src.3ph_l1_l2_l3", D::ElectricPower, K::Source, None, None, 3, Some("0000D"), &[], "-X", &["L1", "L2", "L3"], "Alimentação Trifásica (L1+L2+L3)", "Three-Phase Supply (L1+L2+L3)", Some(3004), None, &[]),
        seed("el.src.3ph_l1_l2_l3_pe", D::ElectricPower, K::Source, None, None, 4, Some("0000E"), &[], "-X", &["L1", "L2", "L3", "PE"], "Alimentação Trifásica + PE", "Three-Phase + PE Supply", Some(3005), None, &[]),
        seed("el.src.dc_pos", D::ElectricCommand, K::Source, None, None, 1, Some("0000F"), &[], "+", &["+"], "Alimentação CC Positivo (+)", "DC Positive Supply (+)", Some(3008), None, &["dc_power/terminal_plus.elmt", "power/battery_pack.elmt"]),
        seed("el.src.dc_neg", D::ElectricCommand, K::Source, None, None, 1, Some("00010"), &[], "-", &["-"], "Alimentação CC Negativo (-)", "DC Negative Supply (-)", Some(3009), None, &[]),

        // Fusíveis (5000, 5003, 5008)
        seed("el.prot.fuse.1p", D::ElectricPower, K::Fuse, None, None, 1, Some("00011"), &[], "-F", &["1", "2"], "Fusível 1P", "Single-Pole Fuse 1P", Some(5000), Some("embed://import/electric/10_allpole/110_network_supplies/fuse.elmt"), &["qet:protection/fuse.elmt", "fuse_10a.elmt", "fusible_nh00.elmt"]),
        seed("el.prot.fuse.3p", D::ElectricPower, K::Fuse, None, None, 3, Some("00012"), &[], "-F", &["1", "3", "5", "2", "4", "6"], "Fusível Tripolar 3P", "Three-Pole Fuse 3P", Some(5003), None, &[]),
        seed("el.prot.fuse_switch.3p", D::ElectricPower, K::Fuse, None, None, 3, Some("00013"), &[], "-Q", &["1", "3", "5", "2", "4", "6"], "Fusível Seccionador 3P", "Switch-Disconnector Fuse 3P", Some(5008), None, &[]),

        // Disjuntores e Relés Térmicos (6000..=6009, 2008)
        seed("el.prot.mcb.1p", D::ElectricPower, K::CircuitBreaker, None, None, 1, Some("00014"), &[], "-Q", &["1", "2"], "Disjuntor Termomagnético 1P", "Miniature Circuit Breaker 1P", Some(6000), Some("embed://import/electric/10_allpole/130_circuit_breakers/circuit_breaker_1p.elmt"), &[]),
        seed("el.prot.mcb.2p", D::ElectricPower, K::CircuitBreaker, None, None, 2, Some("00017"), &[], "-Q", &["1", "3", "2", "4"], "Disjuntor Termomagnético 2P", "Miniature Circuit Breaker 2P", Some(6008), None, &[]),
        seed("el.prot.mcb.3p", D::ElectricPower, K::CircuitBreaker, None, None, 3, Some("00015"), &[], "-Q", &["1", "3", "5", "2", "4", "6"], "Disjuntor Termomagnético 3P", "Miniature Circuit Breaker 3P", Some(6003), Some("embed://import/electric/10_allpole/130_circuit_breakers/circuit_breaker_3p.elmt"), &[]),
        seed("el.prot.mcb.4p", D::ElectricPower, K::CircuitBreaker, None, None, 4, None, &[], "-Q", &["1", "3", "5", "7", "2", "4", "6", "8"], "Disjuntor Termomagnético 4P", "Miniature Circuit Breaker 4P", Some(6004), None, &[]),
        seed("el.prot.thermal_relay.3p", D::ElectricPower, K::ThermalContact, None, None, 3, Some("00016"), &[], "-F", &["1", "3", "5", "2", "4", "6"], "Relé Térmico de Sobrecarga 3P", "Thermal Overload Relay 3P", Some(6007), None, &[]),
        seed("el.prot.motor_breaker.3p", D::ElectricPower, K::CircuitBreaker, None, None, 3, Some("00018"), &[], "-Q", &["1", "3", "5", "2", "4", "6"], "Disjuntor-Motor Guardamotor 3P", "Motor Protection Circuit Breaker 3P", Some(6009), None, &[]),
        seed("el.prot.contactor_breaker.3p", D::ElectricPower, K::CircuitBreaker, None, None, 3, Some("00008"), &[], "-Q", &["1", "3", "5", "2", "4", "6"], "Contator Disjuntor Trifásico 3P", "Three-Phase Contactor Breaker 3P", Some(2008), None, &[]),

        // Contatores de Potência e Seccionadora (2000, 2001, 2004, 2006)
        seed("el.pwr.contactor.1p", D::ElectricPower, K::ContactorContact, Some(F::No), None, 1, Some("00007"), &["0015E"], "-KM", &["1", "2"], "Contator de Potência 1P", "Power Contactor 1P", Some(2006), Some("embed://import/electric/10_allpole/140_contacts/contact_power_contactor.elmt"), &["contact_power_contactor"]),
        seed("el.pwr.contactor.2p", D::ElectricPower, K::ContactorContact, Some(F::No), None, 2, Some("00004"), &["00161"], "-KM", &["1", "3", "2", "4"], "Contator de Potência 2P", "Power Contactor 2P", Some(2000), Some("embed://import/electric/10_allpole/140_contacts/contact_power_contactor_4.elmt"), &["contact_power_contactor_4"]),
        seed("el.pwr.contactor.3p", D::ElectricPower, K::ContactorContact, Some(F::No), None, 3, Some("00005"), &["0015F"], "-KM", &["1", "3", "5", "2", "4", "6"], "Contator Trifásico 3P", "Three-Phase Power Contactor 3P", Some(2001), Some("embed://import/electric/10_allpole/140_contacts/contact_power_contactor_2.elmt"), &["contact_power_contactor_2"]),
        seed("el.pwr.disconnector.3p", D::ElectricPower, K::Disconnector, None, None, 3, Some("00006"), &[], "-Q", &["1", "3", "5", "2", "4", "6"], "Chave Seccionadora Tripolar 3P", "Three-Pole Switch-Disconnector 3P", Some(2004), None, &[]),

        // Motores (1000..=1002)
        seed("el.load.motor.3ph_3t", D::ElectricPower, K::Motor, None, None, 3, Some("00001"), &["0028F"], "-M", &["U1", "V1", "W1", "PE"], "Motor Assíncrono Trifásico", "Three-Phase Induction Motor", Some(1000), Some("embed://import/electric/10_allpole/391_consumers&actuators/10_engines/ac_motor_3-phases.elmt"), &["ac_motor_3-phases", "engines/starter_001.elmt"]),
        seed("el.load.motor.3ph_6t", D::ElectricPower, K::Motor, None, None, 6, Some("00002"), &[], "-M", &["U1", "V1", "W1", "PE", "W2", "U2", "V2"], "Motor Trifásico 6 Terminais (Y-D)", "Three-Phase 6-Terminal Motor", Some(1001), None, &[]),
        seed("el.load.motor.1ph", D::ElectricPower, K::Motor, None, None, 2, Some("00003"), &[], "-M", &["U1", "V1", "PE"], "Motor Monofásico", "Single-Phase Motor", Some(1002), None, &[]),

        // Contatos Auxiliares e Temporizados (7000..=7009)
        seed("el.aux.contact.no", D::ElectricCommand, K::ContactorContact, Some(F::No), Some(B::Instantaneous), 1, Some("00019"), &[], "-KM", &["13", "14"], "Contato Auxiliar NA (NO)", "Auxiliary Contact NO", Some(7000), Some("embed://import/electric/10_allpole/310_relays_contactors&contacts/02_contacts/contact_no.elmt"), &[]),
        seed("el.aux.contact.nc", D::ElectricCommand, K::ContactorContact, Some(F::Nc), Some(B::Instantaneous), 1, Some("0001A"), &[], "-KM", &["11", "12"], "Contato Auxiliar NF (NC)", "Auxiliary Contact NC", Some(7001), Some("embed://import/electric/10_allpole/310_relays_contactors&contacts/02_contacts/contact_nc.elmt"), &[]),
        seed("el.aux.contact.no_nc", D::ElectricCommand, K::ContactorContact, Some(F::NoNc), Some(B::Instantaneous), 2, Some("0001B"), &[], "-KM", &["11", "13", "12", "14"], "Contato Auxiliar Duplo NA+NF", "Dual Auxiliary Contact NO+NC", Some(7002), None, &[]),
        seed("el.aux.timer_on.no", D::ElectricCommand, K::ContactorContact, Some(F::No), Some(B::OnDelay), 1, Some("0001C"), &[], "-KT", &["67", "68"], "Contato Temporizado NA (On-Delay)", "On-Delay Timer Contact NO", Some(7004), None, &[]),
        seed("el.aux.timer_on.nc", D::ElectricCommand, K::ContactorContact, Some(F::Nc), Some(B::OnDelay), 1, Some("0001D"), &[], "-KT", &["55", "56"], "Contato Temporizado NF (On-Delay)", "On-Delay Timer Contact NC", Some(7005), None, &[]),
        seed("el.aux.timer_on.co", D::ElectricCommand, K::ContactorContact, Some(F::Changeover), Some(B::OnDelay), 1, Some("0001E"), &[], "-KT", &["15", "17", "16", "18"], "Contato Temporizado Comutador (On-Delay)", "On-Delay Changeover Timer Contact", Some(7006), None, &[]),
        seed("el.aux.timer_off.no", D::ElectricCommand, K::ContactorContact, Some(F::No), Some(B::OffDelay), 1, Some("0001F"), &[], "-KT", &["27", "28"], "Contato Temporizado NA (Off-Delay)", "Off-Delay Timer Contact NO", Some(7008), None, &[]),
        seed("el.aux.timer_off.nc", D::ElectricCommand, K::ContactorContact, Some(F::Nc), Some(B::OffDelay), 1, Some("00020"), &[], "-KT", &["15", "16"], "Contato Temporizado NF (Off-Delay)", "Off-Delay Timer Contact NC", Some(7009), None, &[]),

        // Botões Pulsadores Monoestáveis (8000..=8003) — convergência CADe_SIMU + QET + RustSimu
        seed(
            "el.act.pb.no",
            D::ElectricCommand,
            K::MomentaryButton,
            Some(F::No),
            None,
            1,
            Some("00021"),
            &["001F8", "001F9", "001FA"],
            "-S",
            &["13", "14"],
            "Botão Pulsador Monoestável NA (NO)",
            "Momentary Pushbutton NO",
            Some(8000),
            Some("embed://import/electric/10_allpole/380_signaling_operating/20_push_buttons/push-button.elmt"),
            &[
                "electric/10_allpole/380_signaling_operating/20_push_buttons/push-button.elmt",
                "electric/10_allpole/380_signaling_operating/20_push_buttons/push-button_2.elmt",
                "electric/10_allpole/380_signaling_operating/20_push_buttons/push-button_3.elmt",
                "buttons/pushbutton_green.elmt",
                "commandes/bouton_poussoir.elmt",
                "bedienung/drucktaster_rot.elmt",
                "push-button",
                "push-button_2",
                "push-button_3",
                "push_button",
            ],
        ),
        seed(
            "el.act.pb.nc",
            D::ElectricCommand,
            K::MomentaryButton,
            Some(F::Nc),
            None,
            1,
            Some("00022"),
            &["001FB"],
            "-S",
            &["11", "12"],
            "Botão Pulsador Monoestável NF (NC)",
            "Momentary Pushbutton NC",
            Some(8001),
            Some("embed://import/electric/10_allpole/380_signaling_operating/20_push_buttons/push-button__nc.elmt"),
            &[
                "electric/10_allpole/380_signaling_operating/20_push_buttons/push-button__nc.elmt",
                "push-button__nc",
            ],
        ),
        seed("el.act.pb.no_nc", D::ElectricCommand, K::MomentaryButton, Some(F::NoNc), None, 2, Some("00023"), &[], "-S", &["11", "13", "12", "14"], "Botão Pulsador Duplo NA+NF", "Dual Pushbutton NO+NC", Some(8002), None, &[]),
        seed("el.act.pb.co", D::ElectricCommand, K::MomentaryButton, Some(F::Changeover), None, 1, Some("00024"), &[], "-S", &["11", "12", "14"], "Botão Pulsador Comutador (CON)", "Changeover Pushbutton", Some(8003), None, &[]),

        // Botões Cogumelo (8004..=8007)
        seed("el.act.mushroom.no", D::ElectricCommand, K::MomentaryButton, Some(F::No), None, 1, Some("00025"), &[], "-S", &["13", "14"], "Botão Cogumelo NA", "Mushroom Pushbutton NO", Some(8004), None, &[]),
        seed("el.act.mushroom.nc", D::ElectricCommand, K::MomentaryButton, Some(F::Nc), None, 1, None, &[], "-S", &["11", "12"], "Botão Cogumelo NF", "Mushroom Pushbutton NC", Some(8005), None, &[]),
        seed("el.act.mushroom.no_nc", D::ElectricCommand, K::MomentaryButton, Some(F::NoNc), None, 2, None, &[], "-S", &["11", "13", "12", "14"], "Botão Cogumelo Duplo NA+NF", "Dual Mushroom Pushbutton NO+NC", Some(8006), None, &[]),
        seed("el.act.mushroom.co", D::ElectricCommand, K::MomentaryButton, Some(F::Changeover), None, 1, None, &[], "-S", &["11", "12", "14"], "Botão Cogumelo Comutador", "Changeover Mushroom Pushbutton", Some(8007), None, &[]),

        // Chaves Seletoras / Interruptores (8008..=8011, 8020..=8022)
        seed("el.act.selector.no", D::ElectricCommand, K::LatchingSwitch, Some(F::No), Some(B::Latching), 1, None, &["00218"], "-S", &["13", "14"], "Chave Seletora / Interruptor NA", "Selector / Toggle Switch NO", Some(8008), None, &["two-positions_luminous_rotative_button"]),
        seed("el.act.selector.nc", D::ElectricCommand, K::LatchingSwitch, Some(F::Nc), Some(B::Latching), 1, None, &[], "-S", &["11", "12"], "Chave Seletora / Interruptor NF", "Selector / Toggle Switch NC", Some(8009), None, &[]),
        seed("el.act.selector.no_nc", D::ElectricCommand, K::LatchingSwitch, Some(F::NoNc), Some(B::Latching), 2, None, &[], "-S", &["11", "13", "12", "14"], "Chave Seletora Dupla NA+NF", "Dual Selector Switch NO+NC", Some(8010), None, &[]),
        seed("el.act.selector.co", D::ElectricCommand, K::LatchingSwitch, Some(F::Changeover), Some(B::Latching), 1, None, &[], "-S", &["11", "12", "14"], "Chave Seletora Comutadora", "Changeover Selector Switch", Some(8011), None, &[]),
        seed("el.act.selector_3pos.i_0_ii", D::ElectricCommand, K::SelectorSwitch, Some(F::ThreeWay), Some(B::Latching), 2, None, &[], "-S", &["1", "2", "4"], "Chave Seletora 3 Posições (I-0-II)", "3-Position Selector Switch (I-0-II)", Some(8020), None, &[]),
        seed("el.act.selector_3pos.i_ii_iii", D::ElectricCommand, K::SelectorSwitch, Some(F::ThreeWay), Some(B::Latching), 3, None, &[], "-S", &["1", "2", "4", "6"], "Chave Seletora 3 Posições (I-II-III)", "3-Position Selector Switch (I-II-III)", Some(8021), None, &[]),
        seed("el.act.crossover_switch", D::ElectricCommand, K::SelectorSwitch, Some(F::ThreeWay), Some(B::Latching), 2, None, &[], "-S", &["1", "2", "3", "4"], "Chave Inversora / Cruzamento", "Crossover Switch", Some(8022), None, &[]),

        // Sensores Fim de Curso (8012..=8015)
        seed("el.act.limit_switch.no", D::ElectricCommand, K::LimitSwitch, Some(F::No), None, 1, None, &[], "-SQ", &["13", "14"], "Sensor Fim de Curso NA", "Limit Switch NO", Some(8012), None, &["sensors/limitswitch_no.elmt"]),
        seed("el.act.limit_switch.nc", D::ElectricCommand, K::LimitSwitch, Some(F::Nc), None, 1, None, &[], "-SQ", &["11", "12"], "Sensor Fim de Curso NF", "Limit Switch NC", Some(8013), None, &["automatisme/endschalter_nc.elmt"]),
        seed("el.act.limit_switch.no_nc", D::ElectricCommand, K::LimitSwitch, Some(F::NoNc), None, 2, None, &[], "-SQ", &["11", "13", "12", "14"], "Sensor Fim de Curso Duplo NA+NF", "Dual Limit Switch NO+NC", Some(8014), None, &[]),
        seed("el.act.limit_switch.co", D::ElectricCommand, K::LimitSwitch, Some(F::Changeover), None, 1, None, &[], "-SQ", &["11", "12", "14"], "Sensor Fim de Curso Comutador", "Changeover Limit Switch", Some(8015), None, &[]),

        // Contatos Auxiliares do Relé Térmico (8016..=8019)
        seed("el.aux.thermal_contact.no", D::ElectricCommand, K::ThermalContact, Some(F::No), None, 1, Some("00026"), &[], "-F", &["97", "98"], "Contato Auxiliar Relé Térmico NA (97-98)", "Thermal Relay Auxiliary Contact NO (97-98)", Some(8016), None, &[]),
        seed("el.aux.thermal_contact.nc", D::ElectricCommand, K::ThermalContact, Some(F::Nc), None, 1, Some("00027"), &[], "-F", &["95", "96"], "Contato Auxiliar Relé Térmico NF (95-96)", "Thermal Relay Auxiliary Contact NC (95-96)", Some(8017), None, &[]),
        seed("el.aux.thermal_contact.no_nc", D::ElectricCommand, K::ThermalContact, Some(F::NoNc), None, 2, None, &[], "-F", &["95", "97", "96", "98"], "Contato Auxiliar Relé Térmico Duplo", "Thermal Relay Dual Contact NO+NC", Some(8018), None, &[]),
        seed("el.aux.thermal_contact.co", D::ElectricCommand, K::ThermalContact, Some(F::Changeover), None, 1, None, &[], "-F", &["95", "96", "98"], "Contato Auxiliar Relé Térmico Comutador", "Thermal Relay Changeover Contact", Some(8019), None, &[]),

        // Bloco 2D + Botões de Emergência (8023, 8030..=8034)
        seed("el.block.pb_2d_panel", D::PanelBlock, K::Block, None, None, 0, Some("00028"), &[], "-BK", &[], "Botão Pulsador 2D (Painel)", "2D Panel Pushbutton", Some(8023), None, &[]),
        seed("el.act.estop.nc", D::ElectricCommand, K::EmergencyStop, Some(F::Nc), Some(B::Latching), 1, None, &[], "-S", &["11", "12"], "Botão de Emergência com Trava NF", "Emergency Stop Latch NC", Some(8030), None, &[]),
        seed("el.act.estop.no", D::ElectricCommand, K::EmergencyStop, Some(F::No), Some(B::Latching), 1, None, &[], "-S", &["13", "14"], "Botão de Emergência com Trava NA", "Emergency Stop Latch NO", Some(8031), None, &[]),
        seed("el.act.estop.no_nc", D::ElectricCommand, K::EmergencyStop, Some(F::NoNc), Some(B::Latching), 2, None, &[], "-S", &["11", "13", "12", "14"], "Botão de Emergência com Trava Duplo NA+NF", "Dual Emergency Stop Latch NO+NC", Some(8032), None, &[]),
        seed("el.act.estop.turn_nc", D::ElectricCommand, K::EmergencyStop, Some(F::Nc), Some(B::Latching), 1, None, &[], "-S", &["11", "12"], "Botão de Emergência Girar para Destravar NF", "Emergency Stop Turn-to-Release NC", Some(8033), None, &[]),
        seed("el.act.estop.key_nc", D::ElectricCommand, K::EmergencyStop, Some(F::Nc), Some(B::Latching), 1, None, &[], "-S", &["11", "12"], "Botão de Emergência com Chave NF", "Emergency Stop Key-Release NC", Some(8034), None, &[]),

        // Bobinas e Sinalização (9000, 9004, 9005, 9008, 9013, 9014)
        seed("el.coil.contactor", D::ElectricCommand, K::RelayCoil, None, Some(B::Instantaneous), 1, Some("00029"), &["0013F", "00140", "00141", "00142"], "-KM", &["A1", "A2"], "Bobina de Contator / Relé", "Contactor / Relay Coil", Some(9000), Some("embed://import/electric/10_allpole/310_relays_contactors&contacts/01_coils/coil.elmt"), &["coil", "coil_2", "coil_3", "coil_4"]),
        seed("el.coil.solenoid", D::ElectricCommand, K::RelayCoil, None, Some(B::Instantaneous), 1, None, &[], "-Y", &["A1", "A2"], "Bobina de Solenoide / Eletroválvula", "Solenoid Valve Actuator Coil", Some(9014), Some("symbols_qet_all/electric/10_allpole/391_consumers_actuators/20_valves/solenoid_valve.svg"), &["solenoid_valve", "solenoid"]),
        seed("el.coil.timer.on_delay", D::ElectricCommand, K::RelayCoil, None, Some(B::OnDelay), 1, Some("0002A"), &[], "-KT", &["A1", "A2"], "Temporizador ao Trabalho (On-Delay)", "On-Delay Timer Coil", Some(9004), None, &[]),
        seed("el.coil.timer.off_delay", D::ElectricCommand, K::RelayCoil, None, Some(B::OffDelay), 1, Some("0002B"), &[], "-KT", &["A1", "A2"], "Temporizador à Desconexão (Off-Delay)", "Off-Delay Timer Coil", Some(9005), None, &[]),
        seed("el.sig.lamp", D::ElectricCommand, K::Lamp, None, None, 1, Some("0002C"), &["01B75", "01B76", "01B77", "01B78"], "-H", &["X1", "X2"], "Sinaleiro Luminoso (Piloto)", "Pilot Indicator Lamp", Some(9008), Some("embed://import/electric/10_allpole/380_signaling_operating/11_optical_signaling/pilot_light.elmt"), &["voyant-blanc", "voyant-bleu", "voyant-orange", "voyant-rouge", "signal/warning_light_red.elmt", "eclairage/lampe_temoin.elmt", "signalisation/voyant_led.elmt"]),
        seed("el.sig.buzzer", D::ElectricCommand, K::Load, None, None, 1, Some("0002D"), &[], "-H", &["X1", "X2"], "Sirene / Alarme Sonoro (Buzzer)", "Audible Buzzer / Horn", Some(9013), None, &["actuators/horn_12v.elmt", "alarmes/sirene_electronique.elmt", "horn_001.elmt"]),

        // Conectores & Âncoras (pneumática v1 vem de `fluidsim_kb`)
        seed("el.conn.terminal", D::ElectricCommand, K::Terminal, None, None, 1, None, &[], "-X", &["1", "2"], "Borne de Passagem / Conector", "Feed-Through Terminal / Connector", None, None, &[]),
        seed("hyd.valve.dir", D::Hydraulic, K::SelectorSwitch, None, None, 4, None, &[], "-V", &["P", "T", "A", "B"], "Válvula Direcional Hidráulica", "Hydraulic Directional Valve", None, None, &[]),
        seed("hyd.pump", D::Hydraulic, K::Source, None, None, 2, None, &["0213C", "02142"], "-P", &["1", "2"], "Bomba / Motor Hidráulico", "Hydraulic Pump / Motor", None, None, &[]),
        seed("hyd.cylinder", D::Hydraulic, K::Load, None, None, 2, None, &[], "-A", &["1", "2"], "Cilindro Hidráulico", "Hydraulic Cylinder", None, None, &[]),
        seed("hyd.tank", D::Hydraulic, K::Source, None, None, 1, None, &[], "-T", &["T"], "Reservatório Hidráulico", "Hydraulic Tank", None, None, &[]),
        seed("ladder.contact.no", D::LadderLogic, K::ContactorContact, Some(F::No), None, 1, None, &[], "-I", &["1", "2"], "Contato Ladder NA", "Ladder NO Contact", None, None, &[]),
        seed("ladder.contact.nc", D::LadderLogic, K::ContactorContact, Some(F::Nc), None, 1, None, &[], "-I", &["1", "2"], "Contato Ladder NF", "Ladder NC Contact", None, None, &[]),
        seed("ladder.coil", D::LadderLogic, K::RelayCoil, None, Some(B::Instantaneous), 1, None, &[], "-Q", &["1", "2"], "Bobina de Saída Ladder", "Ladder Output Coil", None, None, &[]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbols::symbol_catalog;

    #[test]
    fn test_all_legacy_cade_symbols_resolve_deterministically() {
        let dict = ConceptDictionary::build_builtin();
        let catalog = symbol_catalog();
        assert_eq!(catalog.len(), 68);
        for def in catalog {
            let resolved = dict.get_by_cade_tipo(def.tipo).unwrap_or_else(|| {
                panic!(
                    "Símbolo CADe_SIMU tipo={} ({}) sem conceito canônico",
                    def.tipo, def.name
                )
            });
            assert_eq!(resolved.cade_tipo, Some(def.tipo));
        }
    }

    #[test]
    fn test_pushbutton_no_convergence_across_cade_qet_and_canonical() {
        let dict = ConceptDictionary::build_builtin();

        // 1. CADe_SIMU legado (tipo = 8000)
        let cade_item = SchematicItem::new_symbol(1, 8000, 50, 50, "-S1");
        let r_cade = dict.resolve_item(&cade_item).expect("CADe 8000");
        assert_eq!(r_cade.concept.concept_id, "el.act.pb.no");
        assert_eq!(r_cade.confidence, ResolutionConfidence::ExactCadeTipo);

        // 2. Biblioteca canônica (symbol_id = 00021)
        let mut canon_item = SchematicItem::new_symbol(2, 20000, 60, 50, "-S2");
        canon_item.symbol_id = Some("00021".to_string());
        canon_item.pins[0] = "13".to_string();
        canon_item.pins[1] = "14".to_string();
        let r_canon = dict.resolve_item(&canon_item).expect("Canônico 00021");
        assert_eq!(r_canon.concept.concept_id, "el.act.pb.no");
        assert_eq!(r_canon.confidence, ResolutionConfidence::ExactSymbolId);

        // 3. Variantes QElectroTech em EN / FR / DE com pinos genéricos ["1", "2"]
        for qet_path in [
            "embed://import/electric/10_allpole/380_signaling_operating/20_push_buttons/push-button.elmt",
            "embed://import/electric/10_allpole/380_signaling_operating/20_push_buttons/push-button_2.elmt",
            "buttons/pushbutton_green.elmt",
            "commandes/bouton_poussoir.elmt",
            "bedienung/drucktaster_rot.elmt",
        ] {
            let mut qet_item = SchematicItem::new_symbol(3, 20000, 70, 50, "-S3");
            qet_item.qet_type = Some(qet_path.to_string());
            qet_item.pins[0] = "1".to_string();
            qet_item.pins[1] = "2".to_string();
            let r_qet = dict
                .resolve_item(&qet_item)
                .unwrap_or_else(|| panic!("Deveria resolver alias QET {qet_path}"));
            assert_eq!(r_qet.concept.concept_id, "el.act.pb.no");
            assert_eq!(r_qet.concept.cade_tipo, Some(8000));
            assert_eq!(
                r_qet.pin_remap,
                vec![(0, "13".to_string()), (1, "14".to_string())]
            );
            assert_eq!(dict.resolve_for_cade_export(&qet_item), Some(8000));
        }

        // 4. Botão NF nunca deve colapsar para NA
        let mut qet_nc = SchematicItem::new_symbol(4, 20000, 80, 50, "-S0");
        qet_nc.qet_type = Some("embed://import/electric/10_allpole/380_signaling_operating/20_push_buttons/push-button__nc.elmt".to_string());
        let r_nc = dict.resolve_item(&qet_nc).expect("QET NC");
        assert_eq!(r_nc.concept.concept_id, "el.act.pb.nc");
        assert_eq!(r_nc.concept.contact_form, Some(ContactForm::Nc));
        assert_eq!(dict.resolve_for_cade_export(&qet_nc), Some(8001));
    }

    #[test]
    fn test_multidomain_semantic_parser_and_export_compat() {
        let dict = ConceptDictionary::build_builtin();

        let pneu = dict
            .resolve_external_qet(
                "pneumatic/5010_compressed_air/valves/5_2_monostable_valve.elmt",
                Some("5/2 Directional Valve"),
                &["1".into(), "2".into(), "3".into(), "4".into(), "5".into()],
            )
            .expect("Válvula pneumática");
        assert_eq!(pneu.concept.domain, SimulationDomain::Pneumatic);
        assert_eq!(pneu.concept.concept_id, "pneu.valve.5_2");
        assert_eq!(pneu.concept.kind, CanonicalKind::DirectionalValve);
        assert_eq!(pneu.concept.cade_tipo, None);

        let fs = dict.get_by_fluidsim_class("PVWE4").expect("PVWE4");
        assert_eq!(fs.concept_id, "pneu.valve.5_2.solenoid");
        assert!(fs.simular);
        assert!(dict.get_by_fluidsim_class("PPE1").is_some());
        assert!(dict.get_by_fluidsim_class("E_S011").is_some());
        assert_eq!(
            dict.concepts()
                .iter()
                .filter(|c| c.simular && c.kind.is_pneumatic_family())
                .count(),
            24 // v1 pneumáticos (20) + P7.2 (PVG2/PVG3/PVS4/PAV1); PAR2=Motor fora da família
        );

        let ladder = dict
            .resolve_external_qet(
                "logic/2010_logic_gates/201040_ladder_diagram/contact_nc.elmt",
                None,
                &["1".into(), "2".into()],
            )
            .expect("Contato Ladder NF");
        assert_eq!(ladder.concept.domain, SimulationDomain::LadderLogic);
        assert_eq!(ladder.concept.concept_id, "ladder.contact.nc");
    }
}
