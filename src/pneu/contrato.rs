//! Contrato local dos tipos que o merge no RustSimu vai receber.
//!
//! Espelha os pontos de `cade-simu-rs` que a integração precisa e não depende
//! daquele crate. A pressão de ar é [`NoAr`]; o potencial elétrico fica em
//! [`PotencialEletrico`], espelho de `Potential` em `src/model.rs`.
//!
//! Grade: `GRADE_PX` = `GRID_MODULE_PX_300DPI` (48). Orientação de item: `0..=7`,
//! igual a `SchematicItem.orientation`. Condutor: extremos `(x1, y1)`–`(x2, y2)`,
//! igual ao cabo desenhado por coordenada.

use std::any::TypeId;

/// Módulo da grade do RustSimu, em px a 300 DPI (`GRID_MODULE_PX_300DPI`).
pub const GRADE_PX: i32 = 48;

/// Raio do borne no SVG do RustSimu (`src/symbols.rs`).
pub const RAIO_BORNE_PX: i32 = 4;

/// Espelho de `Potential` (`src/model.rs` do RustSimu). Só eletricidade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PotencialEletrico {
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

impl PotencialEletrico {
    /// Rótulo igual a `Potential::label` no RustSimu.
    pub const fn rotulo(self) -> &'static str {
        match self {
            Self::None => "0",
            Self::L1 => "L1",
            Self::L2 => "L2",
            Self::L3 => "L3",
            Self::N => "N",
            Self::PE => "PE",
            Self::DcPos => "+",
            Self::DcNeg => "-",
            Self::ShortCircuit => "CC!",
        }
    }
}

/// Estado de um nó de tubo. Não converte de [`PotencialEletrico`] e não o implementa.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NoAr {
    Suprimento,
    Escape,
    SemPressao,
}

impl NoAr {
    pub const fn rotulo(self) -> &'static str {
        match self {
            Self::Suprimento => "suprimento",
            Self::Escape => "escape",
            Self::SemPressao => "sem_pressao",
        }
    }
}

/// Curso binário derivado (fins de curso / API legada).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum CursoCilindro {
    #[default]
    Recolhido,
    Avancado,
}

/// Limiar (milésimos) para considerar o pistão no fim de curso.
pub const CURSO_LIMIAR_PM: u16 = 50;

/// Estado contínuo do pistão (didático; espelha slots `cylinder.xa`… do FluidSIM).
///
/// `pos_pm`: 0 = recolhido, 1000 = avançado. `vel_pm_s`: milésimos de curso por segundo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct EstadoPistao {
    pub pos_pm: u16,
    pub vel_pm_s: i32,
}

impl EstadoPistao {
    pub const fn recolhido() -> Self {
        Self {
            pos_pm: 0,
            vel_pm_s: 0,
        }
    }

    pub const fn avancado() -> Self {
        Self {
            pos_pm: 1000,
            vel_pm_s: 0,
        }
    }

    pub fn de_curso(c: CursoCilindro) -> Self {
        match c {
            CursoCilindro::Recolhido => Self::recolhido(),
            CursoCilindro::Avancado => Self::avancado(),
        }
    }

    /// Posição normalizada `0.0..=1.0`.
    pub fn pos(self) -> f32 {
        f32::from(self.pos_pm) / 1000.0
    }

    pub fn curso(self) -> CursoCilindro {
        if self.no_avancado() {
            CursoCilindro::Avancado
        } else {
            CursoCilindro::Recolhido
        }
    }

    pub fn no_avancado(self) -> bool {
        self.pos_pm >= 1000 - CURSO_LIMIAR_PM
    }

    pub fn no_recolhido(self) -> bool {
        self.pos_pm <= CURSO_LIMIAR_PM
    }
}

impl From<CursoCilindro> for EstadoPistao {
    fn from(c: CursoCilindro) -> Self {
        Self::de_curso(c)
    }
}

/// Entrada que muda o carretel. A válvula guarda a lista; cada entrada não é a válvula.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Acionamento {
    Manual,
    Mecanico,
    Piloto,
    Eletrico,
}

/// Posição do carretel. Cada posição escolhe uma linha da tabela de vias.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum EstadoCarretel {
    #[default]
    Repouso,
    Acionado,
}

/// Par de vias ligadas entre si. A ordem dos nomes não importa.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParVias {
    pub a: String,
    pub b: String,
}

impl ParVias {
    pub fn novo(a: impl Into<String>, b: impl Into<String>) -> Self {
        Self {
            a: a.into(),
            b: b.into(),
        }
    }

    pub fn liga(&self, x: &str, y: &str) -> bool {
        (self.a == x && self.b == y) || (self.a == y && self.b == x)
    }
}

/// Válvula direcional: estado do carretel e, por estado, os pares de vias abertos.
///
/// Não há um booleano único de condução. Uma 5/2 em repouso liga dois pares ao
/// mesmo tempo e deixa outros fechados.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Valvula {
    pub estado: EstadoCarretel,
    tabela: Vec<(EstadoCarretel, Vec<ParVias>)>,
    pub acionamentos: Vec<Acionamento>,
}

impl Valvula {
    pub fn nova(
        estado: EstadoCarretel,
        tabela: Vec<(EstadoCarretel, Vec<ParVias>)>,
        acionamentos: Vec<Acionamento>,
    ) -> Self {
        Self {
            estado,
            tabela,
            acionamentos,
        }
    }

    /// Pares abertos no estado atual. Vazio se o estado não tem linha.
    pub fn pares_ligados(&self) -> &[ParVias] {
        self.tabela
            .iter()
            .find(|(estado, _)| *estado == self.estado)
            .map(|(_, pares)| pares.as_slice())
            .unwrap_or(&[])
    }

    pub fn via_liga(&self, a: &str, b: &str) -> bool {
        self.pares_ligados().iter().any(|par| par.liga(a, b))
    }
}

/// Borne nomeado numa coordenada da grade. O nome segue o pino de `SchematicItem.pins`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Borne {
    pub nome: String,
    pub x: i32,
    pub y: i32,
}

impl Borne {
    pub fn novo(nome: impl Into<String>, x: i32, y: i32) -> Self {
        Self {
            nome: nome.into(),
            x,
            y,
        }
    }
}

/// Item já traduzido para o esquema do RustSimu.
///
/// No merge, `concept_id` preenche `SchematicItem.concept_id`, `x`/`y` a posição
/// na grade de 48 px e `orientacao` o campo `orientation` (`0..=7`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemEsquema {
    pub id: u32,
    pub concept_id: String,
    pub tag: String,
    pub x: i32,
    pub y: i32,
    pub orientacao: i32,
    pub bornes: Vec<Borne>,
    pub inicial_acionado: bool,
}

impl ItemEsquema {
    pub fn novo(
        id: u32,
        concept_id: impl Into<String>,
        tag: impl Into<String>,
        x: i32,
        y: i32,
        orientacao: i32,
        bornes: Vec<Borne>,
    ) -> Result<Self, ErroContrato> {
        if !(0..=7).contains(&orientacao) {
            return Err(ErroContrato::Orientacao(orientacao));
        }
        Ok(Self {
            id,
            concept_id: concept_id.into(),
            tag: tag.into(),
            x,
            y,
            orientacao,
            bornes,
            inicial_acionado: false,
        })
    }

    pub fn borne(&self, nome: &str) -> Option<&Borne> {
        self.bornes.iter().find(|b| b.nome == nome)
    }
}

/// Trecho de condutor entre duas coordenadas, no mesmo papel do cabo do RustSimu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Condutor {
    pub id: u32,
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
    pub eletrico: bool,
}

/// Página traduzida: itens com conceito e condutores por coordenada.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Esquema {
    pub itens: Vec<ItemEsquema>,
    pub condutores: Vec<Condutor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErroContrato {
    Orientacao(i32),
}

impl std::fmt::Display for ErroContrato {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Orientacao(v) => write!(f, "orientação {v} fora de 0..=7"),
        }
    }
}

impl std::error::Error for ErroContrato {}

/// Os dois domínios são tipos diferentes. Não existe conversão entre eles.
pub fn pressao_e_eletricidade_sao_tipos_distintos() -> bool {
    TypeId::of::<NoAr>() != TypeId::of::<PotencialEletrico>()
}

/// Garante orientação no intervalo `0..=7` (campo `SchematicItem.orientation`).
pub fn snap_orientacao(o: i32) -> i32 {
    o.rem_euclid(8)
}
