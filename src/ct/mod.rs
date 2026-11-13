//! Decifragem Str2 + decompressão LZ77 de arquivos `.ct` do FluidSIM.
//!
//! Port de `analise/scripts/decifra_ct.py` + `decomprime_lz.py`.
//! A tabela DEC (2 KiB) é embutida (gerada a partir de `ad_kb.dll`).

mod export;
mod import;
mod lang_map;
pub(crate) mod lz77;

pub use export::{corpo_ct_from_doc, export_doc_to_ct};
pub use import::{schematic_from_ct_bytes, schematic_from_ct_path};
pub use lang_map::{decifra_lang_map, LANG_MAP_BIN};

/// Tabela DEC 8×256 (`DLLDecryptStr2`).
pub static DEC: &[u8; 2048] = include_bytes!("fluidsim_dec.bin");

/// Decifra uma linha Str2 (EOL já removido): `row = len % 8`.
pub fn dec_str2(linha: &[u8]) -> Vec<u8> {
    let row = linha.len() % 8;
    linha.iter().map(|&b| DEC[row * 256 + b as usize]).collect()
}

/// Cifra uma linha Str2 (inversa de [`dec_str2`]).
pub fn enc_str2_line(plain: &[u8]) -> Vec<u8> {
    let row = plain.len() % 8;
    let base = row * 256;
    let mut inv = [0u8; 256];
    for i in 0..256 {
        inv[DEC[base + i] as usize] = i as u8;
    }
    plain.iter().map(|&b| inv[b as usize]).collect()
}

/// Resultado de abrir um `.ct` binário.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtDecifrado {
    /// Linhas de cabeçalho já em texto (cp1252 → lossy UTF-8).
    pub cabecalho: Vec<String>,
    /// Corpo descomprimido (texto do esquema, mesmo formato de `*.ct.corpo.txt`).
    pub corpo: Vec<u8>,
}

#[derive(Debug)]
pub enum ErroCt {
    SemCompress,
    Lz(String),
    Io(std::io::Error),
}

impl std::fmt::Display for ErroCt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SemCompress => write!(f, "linha COMPRESS não encontrada no .ct"),
            Self::Lz(s) => write!(f, "LZ77: {s}"),
            Self::Io(e) => write!(f, "io: {e}"),
        }
    }
}

impl std::error::Error for ErroCt {}

impl From<std::io::Error> for ErroCt {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

/// Decifra bytes brutos de um arquivo `.ct`.
pub fn decifra_ct_bytes(bruto: &[u8]) -> Result<CtDecifrado, ErroCt> {
    let mut cabecalho = Vec::new();
    let mut pos = 0usize;
    let mut inicio_binario = None;
    while pos < bruto.len() {
        let e = bruto[pos..]
            .iter()
            .position(|&b| b == b'\n')
            .map(|i| pos + i)
            .unwrap_or(bruto.len());
        let mut ln = &bruto[pos..e];
        if ln.ends_with(b"\r") {
            ln = &ln[..ln.len() - 1];
        }
        let s2 = dec_str2(ln);
        if s2.starts_with(b"COMPRESS") {
            inicio_binario = Some(e + 1);
            break;
        }
        cabecalho.push(String::from_utf8_lossy(&s2).into_owned());
        pos = e + 1;
    }
    let inicio = inicio_binario.ok_or(ErroCt::SemCompress)?;
    let corpo = lz77::decodifica_bloco_compress(&bruto[inicio..]).map_err(ErroCt::Lz)?;
    Ok(CtDecifrado { cabecalho, corpo })
}

/// Lê e decifra um arquivo `.ct` do disco.
pub fn decifra_ct_arquivo(path: impl AsRef<std::path::Path>) -> Result<CtDecifrado, ErroCt> {
    let bruto = std::fs::read(path)?;
    decifra_ct_bytes(&bruto)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dec_table_len() {
        assert_eq!(DEC.len(), 2048);
        // Identidade parcial na linha 0 para 0x00 (valor observado na geração).
        assert_eq!(DEC[0], 0x00);
    }

    #[test]
    fn decifra_tp101_se_presente() {
        let candidatos = [
            "/mnt/Dados/Downloads/FluidSim Pneumatic/FluidSim Pneumatic/ct/digital/d_01.ct",
            "/mnt/Dados/Downloads/FluidSim Pneumatic/FluidSim Pneumatic/ct/TP101_17.ct",
        ];
        for p in candidatos {
            let path = std::path::Path::new(p);
            if !path.exists() {
                continue;
            }
            let r = decifra_ct_arquivo(path).expect("decifrar .ct");
            assert!(!r.corpo.is_empty(), "corpo vazio em {p}");
            let texto = String::from_utf8_lossy(&r.corpo);
            assert!(
                texto.contains("O") || texto.contains("SYM") || texto.contains("CON"),
                "corpo sem tokens esperados em {p}"
            );
            return;
        }
        // Sem fixture no ambiente — só valida a tabela.
    }
}
