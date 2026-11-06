use serde::{Deserialize, Serialize};

/// Densidade da grade do diagrama (pontos por centímetro / passo da grade).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum GridDensity {
    Fine, // 10 pts/cm -> passo 3
    #[default]
    Medium, // 5 pts/cm  -> passo 6
    Coarse, // 2.5 pts/cm -> passo 12
}

impl GridDensity {
    pub const ALL: [GridDensity; 3] = [GridDensity::Fine, GridDensity::Medium, GridDensity::Coarse];

    pub fn label(self) -> &'static str {
        match self {
            GridDensity::Fine => "Fina (10 pts/cm)",
            GridDensity::Medium => "Média (5 pts/cm)",
            GridDensity::Coarse => "Larga (2.5 pts/cm)",
        }
    }

    pub fn step(self) -> i32 {
        match self {
            GridDensity::Fine => 3,
            GridDensity::Medium => 6,
            GridDensity::Coarse => 12,
        }
    }
}

impl std::fmt::Display for GridDensity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Potencial elétrico propagado nos barramentos, cabos e terminais durante a simulação.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Potential {
    #[default]
    None,
    L1,
    L2,
    L3,
    N,
    PE,
    DcPos,
    DcNeg,
    ShortCircuit,
}

impl Potential {
    pub fn is_energized(self) -> bool {
        !matches!(self, Potential::None)
    }

    pub fn label(self) -> &'static str {
        match self {
            Potential::None => "0",
            Potential::L1 => "L1",
            Potential::L2 => "L2",
            Potential::L3 => "L3",
            Potential::N => "N",
            Potential::PE => "PE",
            Potential::DcPos => "+",
            Potential::DcNeg => "-",
            Potential::ShortCircuit => "CC!",
        }
    }
}

/// Modo de operação de motores trifásicos (Partida Direta, Estrela `Y` ou Triângulo `Delta`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MotorMode {
    #[default]
    Off,
    Direct,
    Star,
    Delta,
}

impl MotorMode {
    pub fn label(self) -> &'static str {
        match self {
            MotorMode::Off => "PARADO",
            MotorMode::Direct => "DIRETA",
            MotorMode::Star => "ESTRELA (Y)",
            MotorMode::Delta => "TRIÂNGULO (Δ)",
        }
    }
}

/// Transformação exata de coordenadas `(dx, dy)` para as 8 orientações (`0..=7`)
/// extraída de `0x00846951` (`CCADe_SIMUView` / `Dibujo.cpp`) em `CADe_SIMU.exe`.
pub fn transform_offset(dx: i32, dy: i32, orientation: i32) -> (i32, i32) {
    match orientation.rem_euclid(8) {
        0 => (dx, dy),
        1 => (-dy, dx),
        2 => (-dx, -dy),
        3 => (dy, -dx),
        4 => (-dx, dy),
        5 => (dx, -dy),
        6 => (-dy, -dx),
        7 => (dy, dx),
        _ => (dx, dy),
    }
}

/// Versão em ponto flutuante da transformação de orientação para primitivas vetoriais `Dibujo` (escala 1/5).
pub fn transform_offset_f32(dx: f32, dy: f32, orientation: i32) -> (f32, f32) {
    match orientation.rem_euclid(8) {
        0 => (dx, dy),
        1 => (-dy, dx),
        2 => (-dx, -dy),
        3 => (dy, -dx),
        4 => (-dx, dy),
        5 => (dx, -dy),
        6 => (-dy, -dx),
        7 => (dy, dx),
        _ => (dx, dy),
    }
}

/// Bloco de legenda (Cajetín) do CADe_SIMU (serializado após `&&&` no formato `.cad`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TitleBlock {
    pub drawn_date: String,
    pub drawn_by: String,
    pub checked_date: String,
    pub checked_by: String,
    pub entity: String,
    pub title: String,
    pub subtitle: String,
    pub sheet: String,
    pub file_ref: String,
    pub signature: String,
}

impl Default for TitleBlock {
    fn default() -> Self {
        Self {
            drawn_date: "31/10/2026".to_string(),
            drawn_by: "CADe_SIMU RS".to_string(),
            checked_date: "31/10/2026".to_string(),
            checked_by: "Engenharia".to_string(),
            entity: "RustSimu".to_string(),
            title: "Partida Direta Trifasica".to_string(),
            subtitle: "Potencia e Comando".to_string(),
            sheet: "1 / 1".to_string(),
            file_ref: "DOL_3F.cad".to_string(),
            signature: "".to_string(),
        }
    }
}

/// Configurações da folha e visualização do documento (serializadas entre `$$$` e `&&&`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DocumentSettings {
    pub format_code: i32,
    pub format_orientation: i32,
    pub border_ref: i32,
    pub show_pin_refs: i32,
    pub line_thickness: i32,
    pub color_mode: i32,
    pub sim_speed: i32,
    pub auto_number: i32,
    pub displace_lines: i32,
    pub custom_height_mm: i32,
    pub custom_width_mm: i32,
    pub view_flags: [i32; 14],
}

impl Default for DocumentSettings {
    fn default() -> Self {
        Self {
            format_code: 1,
            format_orientation: 1,
            border_ref: 1,
            show_pin_refs: 2,
            line_thickness: 4,
            color_mode: 0,
            sim_speed: 1,
            auto_number: 0,
            displace_lines: 0,
            custom_height_mm: 279,
            custom_width_mm: 210,
            view_flags: [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
        }
    }
}

/// Item do diagrama esquemático, compatível 1:1 com os arrays paralelos de `CCADe_SIMUView`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchematicItem {
    pub id: i32,
    /// Código `tipo` original do CADe_SIMU (`1`, `8`, `1000..10550`).
    pub tipo: i32,
    /// Identificador do componente (`this + 0x3f64`, ex: `-KM1`, `-Q1`, `-S1`, `-M1`).
    pub tag: String,
    /// Valor / Texto secundário (`this + 0x21424`, ex: tempo do temporizador `"5"` s).
    pub value: String,
    /// Nomes dos até 9 terminais (`this + 0x465e4 .. 0x55fe4`, ex: `["1","3","5","2","4","6",...]`).
    pub pins: [String; 9],
    /// Visibilidade dos terminais (`this + 0x57f24 .. 0x67924`, `1` = visível).
    pub pin_visibility: [i32; 9],
    /// Coordenada principal X1 (`this + 0x5ea4`) na escala `.cad` (1 passo da grade = 3).
    pub x1: i32,
    /// Coordenada principal Y1 (`this + 0x7de4`) na escala `.cad` (1 passo da grade = 3).
    pub y1: i32,
    /// Coordenada secundária X2 (`this + 0x9d24`, usada em cabos/linhas/retângulos).
    pub x2: i32,
    /// Coordenada secundária Y2 (`this + 0xbc64`, usada em cabos/linhas/retângulos).
    pub y2: i32,
    /// Coordenadas auxiliares de caixa delimitadora (`0xdba4`, `0xfae4`, `0x11a24`, `0x13964`).
    pub aux_coords: [i32; 4],
    /// Cor / estado salvo (`this + 0x177e4`).
    pub color: i32,
    /// Orientação `0..=7` (`this + 0x19724`).
    pub orientation: i32,
    /// Espessura / subvariante (`this + 0x1b664`).
    pub subvariant: i32,
    /// Exibir tag (`this + 0x1d5a4`, `1` = sim).
    pub show_tag: i32,
    /// Exibir valor (`this + 0x1f4e4`, `1` = sim).
    pub show_value: i32,
    /// Inteiros de configuração (`this + 0x23364`, `0x252a4`).
    pub extra_int1: i32,
    pub extra_int2: i32,
    /// String final (`this + 0x69864`, usada para textos `tipo == 8`).
    pub trailing_str: String,
    /// Tipo de elemento original do QElectroTech (ex: "embed://import/classic_car/Chassis_ground_001.elmt")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qet_type: Option<String>,
    /// UUID original do elemento no arquivo .qet
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub qet_uuid: Option<String>,

    // --- Estado transitório de simulação (não salvo no .cad) ---
    #[serde(skip)]
    pub sim_active: bool,
    #[serde(skip)]
    pub sim_manual_latched: bool,
    #[serde(skip)]
    pub sim_timer_elapsed_ms: u64,
    #[serde(skip)]
    pub sim_fault: bool,
    #[serde(skip)]
    pub sim_motor_dir: i8, // 0 = parado, 1 = horário (CW), -1 = anti-horário (CCW)
    #[serde(skip)]
    pub sim_motor_mode: MotorMode,
    #[serde(skip)]
    pub sim_potential: Potential,
}

impl SchematicItem {
    /// Verifica se este `tipo` utiliza o formato estendido (11 strings + 24 inteiros) no `.cad`
    /// conforme `0x00431886` em `CADe_SIMU.exe` (`1052`, `1073`, `10500..=10550`).
    pub fn is_extended_cad_type(tipo: i32) -> bool {
        tipo == 1052 || tipo == 1073 || (10500..=10550).contains(&tipo)
    }

    /// Verifica se este item é um cabo/condutor elétrico (`4000` Fase, `4009` Neutro, `4010` PE, etc.).
    /// Nota: `4001` no `.cad` é o ponto de conexão (Empalme/Nó) e `1` é linha gráfica.
    pub fn is_wire(&self) -> bool {
        matches!(
            self.tipo,
            4000 | 4002
                | 4003
                | 4004
                | 4005
                | 4006
                | 4007
                | 4008
                | 4009
                | 4010
                | 4011
                | 4018
                | 4019
        )
    }

    /// Verifica se este item é um ponto de conexão (nó / junção `4001`, `4013` ou `4014`).
    pub fn is_junction(&self) -> bool {
        matches!(self.tipo, 4001 | 4013 | 4014)
    }

    /// Verifica se este item é um sinaleiro luminoso / lâmpada indicadora / piloto.
    /// Contatos auxiliares (7000..=7009) NUNCA são sinalizadores luminosos.
    pub fn is_lamp_or_signaling(&self) -> bool {
        if (7000..=7009).contains(&self.tipo) {
            return false;
        }
        if self.tipo == 9008 {
            return true;
        }
        if let Some(ref qet) = self.qet_type {
            let lower = qet.to_lowercase();
            if lower.contains("voyant")
                || lower.contains("lamp")
                || lower.contains("light")
                || lower.contains("pilot")
                || lower.contains("sinaleiro")
                || lower.contains("sinalizador")
                || lower.contains("indicator")
            {
                return true;
            }
        }
        crate::symbols::find_symbol(self.tipo)
            .map(|s| {
                let n = s.name.to_lowercase();
                let sl = s.short_label.to_lowercase();
                n.contains("sinaleiro")
                    || n.contains("piloto")
                    || n.contains("lâmpada")
                    || n.contains("lampada")
                    || n.contains("lamp")
                    || n.contains("sinalizador")
                    || n.contains("indicador")
                    || n.contains("voyant")
                    || sl.contains("piloto")
                    || sl.contains("lamp")
            })
            .unwrap_or(false)
    }

    pub fn new_wire(id: i32, tipo: i32, x1: i32, y1: i32, x2: i32, y2: i32) -> Self {
        Self {
            id,
            tipo,
            tag: String::new(),
            value: String::new(),
            pins: Default::default(),
            pin_visibility: [0; 9],
            x1,
            y1,
            x2,
            y2,
            aux_coords: [0, 0, 0, 0],
            color: 0,
            orientation: 0,
            subvariant: 0,
            show_tag: 0,
            show_value: 0,
            extra_int1: 0,
            extra_int2: 0,
            trailing_str: String::new(),
            qet_type: None,
            qet_uuid: None,
            sim_active: false,
            sim_manual_latched: false,
            sim_timer_elapsed_ms: 0,
            sim_fault: false,
            sim_motor_dir: 0,
            sim_motor_mode: MotorMode::Off,
            sim_potential: Potential::None,
        }
    }

    pub fn new_junction(id: i32, x: i32, y: i32) -> Self {
        Self::new_wire(id, 4001, x, y, x, y)
    }

    pub fn new_symbol(id: i32, tipo: i32, x: i32, y: i32, tag: impl Into<String>) -> Self {
        let tag_str = tag.into();
        let mut pins: [String; 9] = Default::default();
        if let Some(sym) = crate::symbols::find_symbol(tipo) {
            for (i, p) in sym.pins.iter().enumerate().take(9) {
                pins[i] = p.default_label.to_string();
            }
        }
        Self {
            id,
            tipo,
            tag: tag_str,
            value: String::new(),
            pins,
            pin_visibility: [1; 9],
            x1: x,
            y1: y,
            x2: 0,
            y2: 0,
            aux_coords: [0, 0, 0, 0],
            color: 0,
            orientation: 0,
            subvariant: 0,
            show_tag: 1,
            show_value: 0,
            extra_int1: 0,
            extra_int2: 0,
            trailing_str: String::new(),
            qet_type: None,
            qet_uuid: None,
            sim_active: false,
            sim_manual_latched: false,
            sim_timer_elapsed_ms: 0,
            sim_fault: false,
            sim_motor_dir: 0,
            sim_motor_mode: MotorMode::Off,
            sim_potential: Potential::None,
        }
    }
}

/// Página individual de um projeto esquemático multipáginas.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectPage {
    pub id: usize,
    pub name: String,
    pub doc: SchematicDocument,
}

impl ProjectPage {
    pub fn new(id: usize, name: impl Into<String>, doc: SchematicDocument) -> Self {
        Self {
            id,
            name: name.into(),
            doc,
        }
    }
}

/// Versão corrente suportada do formato de projeto RustSimu.
pub const CURRENT_PROJECT_VERSION: u32 = 1;

fn default_project_version() -> u32 {
    CURRENT_PROJECT_VERSION
}

/// Projeto esquemático completo (mono ou multipáginas), formato nativo RustSimu (.json / .toml).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectDocument {
    #[serde(default = "default_project_version")]
    pub version: u32,
    pub pages: Vec<ProjectPage>,
    #[serde(default)]
    pub active_page_index: usize,
}

impl Default for ProjectDocument {
    fn default() -> Self {
        Self {
            version: 1,
            pages: Vec::new(),
            active_page_index: 0,
        }
    }
}

impl ProjectDocument {
    pub fn new(pages: Vec<ProjectPage>, active_page_index: usize) -> Self {
        Self {
            version: 1,
            pages,
            active_page_index,
        }
    }

    pub fn from_single_document(title: impl Into<String>, doc: SchematicDocument) -> Self {
        Self {
            version: 1,
            pages: vec![ProjectPage::new(1, title, doc)],
            active_page_index: 0,
        }
    }

    pub fn active_document(&self) -> Option<&SchematicDocument> {
        self.pages
            .get(self.active_page_index)
            .map(|p| &p.doc)
            .or_else(|| self.pages.first().map(|p| &p.doc))
    }
}

/// Documento completo do CADe_SIMU (compatível com `.cad`, `.json` e `.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct SchematicDocument {
    pub items: Vec<SchematicItem>,
    pub settings: DocumentSettings,
    pub title_block: TitleBlock,
}

impl SchematicDocument {
    pub fn next_id(&self) -> i32 {
        self.items.iter().map(|i| i.id).max().unwrap_or(0) + 1
    }

    pub fn reset_simulation_state(&mut self) {
        for item in &mut self.items {
            item.sim_active = false;
            item.sim_manual_latched = false;
            item.sim_timer_elapsed_ms = 0;
            item.sim_fault = false;
            item.sim_motor_dir = 0;
            item.sim_motor_mode = MotorMode::Off;
            item.sim_potential = Potential::None;
        }
    }
}
