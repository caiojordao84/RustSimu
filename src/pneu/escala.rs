//! Escala FluidSIM → grade do RustSimu (P2).
//!
//! Unidade do FluidSIM: 1/10240 de grade (`pic::UN_POR_GRADE`).
//! Módulo do RustSimu: 48 px (`contrato::GRADE_PX` = `pic::PX_POR_GRADE`).
//!
//! Terminais: ops 10–15 do `PIC.BIN` (`FUN_004bec40`). Posição relativa à
//! âncora = `(ps[4], ps[5])`. Número do borne = `ps[3]` (0 → autoincremento).

use crate::pneu::contrato::{Borne, GRADE_PX, RAIO_BORNE_PX};
use crate::pneu::pic::{Comando, Registro, PX_POR_GRADE, UN_POR_GRADE};

/// Fator unidades → px. Idêntico a `PX_POR_GRADE / UN_POR_GRADE`.
pub fn unidades_para_px(u: f64) -> f64 {
    u * PX_POR_GRADE / UN_POR_GRADE
}

/// Fator px → unidades.
pub fn px_para_unidades(px: f64) -> f64 {
    px * UN_POR_GRADE / PX_POR_GRADE
}

/// Raio do borne do RustSimu, em unidades do FluidSIM.
pub fn raio_borne_unidades() -> f64 {
    px_para_unidades(f64::from(RAIO_BORNE_PX))
}

/// Nome do pic a partir da classe (`PVWE4` → `PVWE4.PIC`).
pub fn nome_pic(classe: &str) -> String {
    format!("{classe}.PIC")
}

/// Terminal extraído de um opcode 10–15.
#[derive(Debug, Clone, PartialEq)]
pub struct TerminalPic {
    /// Número do borne (1-based), já com autoincremento.
    pub numero: u32,
    /// Espécie do rótulo: opcode − 9 (op10→1 … op15→6).
    pub especie: u8,
    /// Coordenada relativa à âncora, em unidades FluidSIM.
    pub x_un: i32,
    pub y_un: i32,
}

impl TerminalPic {
    pub fn x_px(&self) -> f64 {
        unidades_para_px(f64::from(self.x_un))
    }

    pub fn y_px(&self) -> f64 {
        unidades_para_px(f64::from(self.y_un))
    }

    /// Borne na grade do RustSimu, snapped ao módulo de 48 px.
    pub fn borne_snapped(&self, nome: impl Into<String>) -> Borne {
        Borne::novo(nome, snap_px(self.x_px()), snap_px(self.y_px()))
    }
}

/// Arredonda para o múltiplo mais próximo de [`GRADE_PX`].
pub fn snap_px(px: f64) -> i32 {
    let g = f64::from(GRADE_PX);
    ((px / g).round() as i32) * GRADE_PX
}

/// Extrai terminais de um record do `PIC.BIN`.
pub fn terminais(rec: &Registro) -> Vec<TerminalPic> {
    let mut out = Vec::new();
    let mut auto = 1u32;
    for Comando { op, ps } in &rec.comandos {
        if !(10..=15).contains(op) {
            continue;
        }
        let numero = if ps[3] != 0 {
            ps[3] as u32
        } else {
            let n = auto;
            auto += 1;
            n
        };
        if ps[3] != 0 {
            auto = auto.max(numero + 1);
        }
        out.push(TerminalPic {
            numero,
            especie: op - 9,
            x_un: ps[4],
            y_un: ps[5],
        });
    }
    out
}

/// Busca um record pelo nome exato (`PVWM8.PIC`).
pub fn por_nome<'a>(catalogo: &'a [Registro], nome: &str) -> Option<&'a Registro> {
    catalogo.iter().find(|r| r.nome == nome)
}

/// Passo mínimo positivo entre abscissas de terminais, em unidades (0 se <2).
pub fn passo_x_un(terms: &[TerminalPic]) -> i32 {
    let mut xs: Vec<i32> = terms.iter().map(|t| t.x_un).collect();
    xs.sort_unstable();
    xs.dedup();
    xs.windows(2)
        .map(|w| w[1] - w[0])
        .filter(|&d| d > 0)
        .min()
        .unwrap_or(0)
}

/// Par (repouso, acionado) para uma classe v1.
///
/// Sem pic próprio (`PAL1`): `None`. Com pic: o acionado começa igual ao
/// repouso; cilindro duplo ganha haste estendida como sobreposição.
pub fn svg_estados(classe: &str, catalogo: &[Registro], pen: f64) -> Option<(String, String)> {
    let rec = por_nome(catalogo, &nome_pic(classe))?;
    let repouso = svg_rustsimu(rec, pen);
    let acionado = if classe == "PAL2" || classe == "PAL1" {
        svg_com_haste(rec, pen, 1.0)
    } else {
        repouso.clone()
    };
    Some((repouso, acionado))
}

/// Cilindro com haste à direita da âncora; `pos ∈ [0, 1]` interpola o comprimento.
pub fn svg_com_haste(rec: &Registro, pen: f64, pos: f64) -> String {
    let mut svg = svg_rustsimu(rec, pen);
    let frac = pos.clamp(0.0, 1.0);
    if frac <= 0.02 {
        return svg;
    }
    let (tx, ty) = (f64::from(rec.x0), f64::from(rec.y0));
    // Haste: linha horizontal típica (~2.5 grades) a partir da âncora.
    let x2 = tx + UN_POR_GRADE * 2.5 * frac;
    let haste = format!(
        "\n    <line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\" stroke-width=\"{}\"/>",
        crate::pneu::pic::fmt(tx),
        crate::pneu::pic::fmt(ty),
        crate::pneu::pic::fmt(x2),
        crate::pneu::pic::fmt(ty),
        crate::pneu::pic::fmt(pen * 1.5),
    );
    if let Some(pos) = svg.rfind("  </g>") {
        svg.insert_str(pos, &haste);
        svg.insert(pos + haste.len(), '\n');
    }
    svg
}

/// Renderiza SVG na grade do RustSimu: desenho do pic + círculos de borne (r=4 px).
///
/// Sem rótulos de debug. ViewBox em unidades; `width`/`height` em px.
pub fn svg_rustsimu(rec: &Registro, pen: f64) -> String {
    let corpo_simbolo = {
        // Reusa o renderizador sem rótulos e injeta os bornes antes do fechamento.
        let base = crate::pneu::pic::render_svg(rec, pen, false);
        let terms = terminais(rec);
        if terms.is_empty() {
            return base;
        }
        let r = raio_borne_unidades();
        let (tx, ty) = (f64::from(rec.x0), f64::from(rec.y0));
        let mut extras = String::new();
        for t in &terms {
            extras.push_str(&format!(
                "\n    <circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"#000\" stroke=\"none\"/>",
                crate::pneu::pic::fmt(tx + f64::from(t.x_un)),
                crate::pneu::pic::fmt(ty + f64::from(t.y_un)),
                crate::pneu::pic::fmt(r)
            ));
        }
        // Insere antes de `  </g>\n</svg>\n`
        if let Some(pos) = base.rfind("  </g>") {
            let mut out = String::with_capacity(base.len() + extras.len());
            out.push_str(&base[..pos]);
            out.push_str(&extras);
            out.push('\n');
            out.push_str(&base[pos..]);
            out
        } else {
            base
        }
    };
    corpo_simbolo
}
