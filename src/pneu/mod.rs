//! Simulação e símbolos pneumáticos / eletropneumáticos (branch `pneumatica`).
//!
//! Portado de `integracao-rustsimu/testes/ct-parser` (contrato, motor, PIC, escala).

pub mod contrato;
pub mod escala;
pub mod motor;
pub mod palette;
pub mod pic;
pub mod qet_map;
pub mod symbols;

#[derive(Debug)]
pub struct ErroParse(pub String);

impl std::fmt::Display for ErroParse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "erro de parse pneumático: {}", self.0)
    }
}

impl std::error::Error for ErroParse {}

pub use contrato::{CursoCilindro, EstadoCarretel, EstadoPistao, NoAr, GRADE_PX, RAIO_BORNE_PX};
pub use motor::{valvula_5_2, ComandoEletrico, Dispositivo, MotorAr, Porta};
pub use qet_map::concept_id_from_qet_pneumatic;
pub use symbols::{
    catalogo_pic, px_adaptador_para_world, svg_pal1, svg_v1_classe, TerminalSimbolo,
};
