//! P7.5 — `lang_map.bin` (LZ77 modo hash `0x0200`).

use super::lz77;

/// Bytes embutidos do `lang_map.bin` do FluidSIM.
pub static LANG_MAP_BIN: &[u8] = include_bytes!("lang_map.bin");

/// Decomprime `lang_map.bin` (máscara `0x9a`, modo hash 512/9 bits).
pub fn decifra_lang_map(bruto: &[u8]) -> Result<String, String> {
    let out = lz77::decomprime_mascarado(bruto, 0x9a)?;
    Ok(String::from_utf8_lossy(&out).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lang_map_cabecalho_de_idioma() {
        let texto = decifra_lang_map(LANG_MAP_BIN).expect("decompress");
        assert!(
            texto.starts_with("049 Deutsch") || texto.contains("049 Deutsch"),
            "cabeçalho de idioma ausente; head={:?}",
            &texto[..texto.len().min(80)]
        );
        assert!(texto.contains("001 English"));
        assert!(texto.len() > 10_000);
    }
}
