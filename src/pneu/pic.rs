//! Parser + renderizador do `PIC.BIN` (símbolos vetoriais do FluidSIM P4).
//!
//! Formato decifrado via REA/Ghidra (ver `analise/NOTAS.md`, seção "Elementos
//! gráficos"):
//!
//! ```text
//! [u16 count][count x record 36B][payload = soma(n_k) x 33B]
//! record: name[13] | ptr u32@13 (lixo de heap) | n u16@17 | W u32@19
//!         | H u32@23 | X0 u32@27 | Y0 u32@31 | pad u8@35
//! comando 33B: [u8 opcode][8 x i32 LE]
//! ```
//!
//! // RE: loader FUN_0051fd30 (malloc (n+1)x0x21 por record, sobrescreve o ptr)
//! // RE: interpretador FUN_00531b80, jump table 0x543e28 -> handlers FUN_0046xxxx
//! // RE: ops 10-15 (rótulos) só são consumidas por FUN_004bec40 (posição de terminal)
//! // RE: coordenadas relativas à âncora; X0,Y0 = âncora dentro da ink box W x H;
//! // RE: y+ para baixo (GDI); unidade 1/10240 de grade; escala zoom x espelho / 100
//! // RE: (0x5ce218 guardado x100; mágica 0x51eb851f + SAR 5 = divisão por 100)
//!
//! Especificação de referência: `analise/scripts/parse_pic.py` (renderização
//! validada visualmente — galeria `decifrados/pics_svg/index.html`).
//! Fixtures: `tests/fixtures/PIC.BIN` (binário real) + SVGs dourados gerados
//! pelo Python (paridade byte a byte).

use crate::pneu::ErroParse;

/// Comando do blob: opcode + 8 parâmetros `i32` (`ps[0..8]`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Comando {
    pub op: u8,
    pub ps: [i32; 8],
}

/// Um símbolo do catálogo (ex.: `152841.PIC`).
#[derive(Debug, Clone)]
pub struct Registro {
    /// Nome latin-1 sem o NUL (ex.: `152841.PIC`).
    pub nome: String,
    /// Nº de comandos declarado no record (u16@17).
    pub n: u16,
    /// Ink box em unidades (1/10240 de grade).
    pub w: u32,
    pub h: u32,
    /// Âncora dentro da ink box (contada do canto superior esquerdo).
    pub x0: u32,
    pub y0: u32,
    pub comandos: Vec<Comando>,
}

/// 48 px por grade (grade do RustSimu); unidades internas = 1/10240 de grade.
///
/// Públicos: o módulo irmão [`crate::sym`] compartilha a mesma escala.
pub const PX_POR_GRADE: f64 = 48.0;
pub const UN_POR_GRADE: f64 = 10240.0;

/// Decodifica latin-1 (byte -> char) para os nomes dos records.
fn latin1(b: &[u8]) -> String {
    b.iter().map(|&c| c as char).collect()
}

/// Lê `u16`/`u32` little-endian de `d[o..]`.
fn u16le(d: &[u8], o: usize) -> u16 {
    u16::from_le_bytes([d[o], d[o + 1]])
}

fn u32le(d: &[u8], o: usize) -> u32 {
    u32::from_le_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

/// Parser do arquivo inteiro: valida o payload (`soma(n) x 33`) e decodifica
/// os comandos de cada record.
pub fn parse_pic_bin(d: &[u8]) -> Result<Vec<Registro>, ErroParse> {
    if d.len() < 2 {
        return Err(ErroParse("PIC.BIN truncado (sem count)".into()));
    }
    let count = u16le(d, 0) as usize;
    let mut off = 2usize;
    let mut registros: Vec<(String, u16, u32, u32, u32, u32)> = Vec::with_capacity(count);
    for _ in 0..count {
        if off + 36 > d.len() {
            return Err(ErroParse(format!(
                "PIC.BIN truncado nos records (off {off}, len {})",
                d.len()
            )));
        }
        let rec = &d[off..off + 36];
        off += 36;
        let fim_nome = rec[..13].iter().position(|&c| c == 0).unwrap_or(13);
        registros.push((
            latin1(&rec[..fim_nome]),
            u16le(rec, 17),
            u32le(rec, 19),
            u32le(rec, 23),
            u32le(rec, 27),
            u32le(rec, 31),
        ));
    }
    let payload = &d[off..];
    let total: usize = registros.iter().map(|r| r.1 as usize).sum();
    if payload.len() != total * 33 {
        return Err(ErroParse(format!(
            "payload {} != soma(n) x 33 = {}",
            payload.len(),
            total * 33
        )));
    }
    let mut po = 0usize;
    let mut out = Vec::with_capacity(count);
    for (nome, n, w, h, x0, y0) in registros {
        let mut comandos = Vec::with_capacity(n as usize);
        for _ in 0..n {
            if po + 33 > payload.len() {
                return Err(ErroParse("payload truncado nos comandos".into()));
            }
            let op = payload[po];
            let mut ps = [0i32; 8];
            for (i, p) in ps.iter_mut().enumerate() {
                let b = &payload[po + 1 + i * 4..po + 5 + i * 4];
                *p = i32::from_le_bytes([b[0], b[1], b[2], b[3]]);
            }
            po += 33;
            comandos.push(Comando { op, ps });
        }
        out.push(Registro {
            nome,
            n,
            w,
            h,
            x0,
            y0,
            comandos,
        });
    }
    Ok(out)
}

/// Formata com 1 casa decimal e corta zeros/ponto (`'%.1f' % v` do Python,
/// seguido de `rstrip('0').rstrip('.')`) — paridade com os renderizadores de
/// referência (pública: o módulo [`crate::sym`] compartilha a mesma paridade).
pub fn fmt(v: f64) -> String {
    let mut s = format!("{v:.1}");
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    s
}

/// Arco entre ângulos `a0 -> a1` (graus, y+ para baixo) como path SVG.
/// Direção empírica (sweep 0 = anti-horário no SVG); ângulos em graus.
fn arco_svg(cx: f64, cy: f64, r: f64, a0: f64, a1: f64) -> String {
    let t0 = a0.to_radians();
    let t1 = a1.to_radians();
    let (x0, y0) = (cx + r * t0.cos(), cy + r * t0.sin());
    let (x1, y1) = (cx + r * t1.cos(), cy + r * t1.sin());
    let delta = (a1 - a0) % 360.0;
    let large = i32::from(delta > 180.0);
    format!(
        "M {} {} A {} {} 0 {} 0 {} {}",
        fmt(x0),
        fmt(y0),
        fmt(r),
        fmt(r),
        large,
        fmt(x1),
        fmt(y1)
    )
}

/// Renderiza um record em SVG (escala 48 px/grade; viewBox em unidades).
///
/// `mostrar_rotulos=true` replica o renderizador Python de referência: as ops
/// 10-15 saem como marcadores vermelhos de rótulo (círculo r=600 + número,
/// fonte 1600 em `cy+560`) — NÃO fazem parte do símbolo (o exe usa essas ops
/// só para posicionar terminais, `FUN_004bec40`).
pub fn render_svg(rec: &Registro, pen: f64, mostrar_rotulos: bool) -> String {
    let (tx, ty) = (f64::from(rec.x0), f64::from(rec.y0));
    let mut partes: Vec<String> = Vec::new();
    let mut auto = 1u32;

    for Comando { op, ps } in &rec.comandos {
        let p = |i: usize| f64::from(ps[i]);
        match *op {
            1 | 7 => {
                partes.push(format!(
                    "<line x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"/>",
                    fmt(tx + p(4)),
                    fmt(ty + p(5)),
                    fmt(tx + p(6)),
                    fmt(ty + p(7))
                ));
            }
            3 => {
                partes.push(format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\"/>",
                    fmt(tx + p(4)),
                    fmt(ty + p(5)),
                    fmt(p(2))
                ));
            }
            5 => {
                partes.push(format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"#000\"/>",
                    fmt(tx + p(4)),
                    fmt(ty + p(5)),
                    fmt(p(2))
                ));
            }
            4 => {
                partes.push(format!(
                    "<path d=\"{}\"/>",
                    arco_svg(
                        tx + p(4),
                        ty + p(5),
                        p(2),
                        ps[0] as f64 / 1024.0,
                        ps[1] as f64 / 1024.0
                    )
                ));
            }
            6 => {
                // bézier P1..P4 = (p4,p5),(p6,p7),(p2,p3),(p0,p1)
                partes.push(format!(
                    "<path d=\"M {} {} C {} {} {} {} {} {}\"/>",
                    fmt(tx + p(4)),
                    fmt(ty + p(5)),
                    fmt(tx + p(6)),
                    fmt(ty + p(7)),
                    fmt(tx + p(2)),
                    fmt(ty + p(3)),
                    fmt(tx + p(0)),
                    fmt(ty + p(1))
                ));
            }
            8 => {
                partes.push(format!(
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
                    fmt(tx + f64::from(ps[4].min(ps[6]))),
                    fmt(ty + f64::from(ps[5].min(ps[7]))),
                    fmt(f64::from(ps[6].abs_diff(ps[4]))),
                    fmt(f64::from(ps[7].abs_diff(ps[5])))
                ));
            }
            10..=15 if mostrar_rotulos => {
                let num = if ps[3] != 0 { ps[3] as u32 } else { auto };
                auto += 1;
                let (cx, cy) = (tx + p(4), ty + p(5));
                partes.push(format!(
                    "<circle cx=\"{}\" cy=\"{}\" r=\"600\" fill=\"#e11\" stroke=\"none\"/>",
                    fmt(cx),
                    fmt(cy)
                ));
                partes.push(format!(
                    "<text x=\"{}\" y=\"{}\" font-size=\"1600\" fill=\"#e11\" \
                     text-anchor=\"middle\">{num}</text>",
                    fmt(cx),
                    fmt(cy + 560.0)
                ));
            }
            _ => {}
        }
    }

    svg_frame(f64::from(rec.w), f64::from(rec.h), pen, &partes.join("\n"))
}

/// Moldura SVG comum aos renderizadores PIC e SYM: viewBox em unidades,
/// escala de 48 px por grade, caneta padrão e corpo já pronto.
pub fn svg_frame(w: f64, h: f64, pen: f64, corpo: &str) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {0} {1}\" \
         width=\"{2}\" height=\"{3}\">\n\
         \x20 <g stroke=\"#000\" stroke-width=\"{4}\" fill=\"none\" \
         stroke-linecap=\"round\" stroke-linejoin=\"round\">\n{5}\n  </g>\n</svg>\n",
        fmt(w),
        fmt(h),
        fmt(w * PX_POR_GRADE / UN_POR_GRADE),
        fmt(h * PX_POR_GRADE / UN_POR_GRADE),
        fmt(pen),
        corpo
    )
}
