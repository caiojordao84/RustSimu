//! Módulo de decodificação e codificação Windows-1252 (CP-1252) para arquivos `.cad` legados do CADe_SIMU.
//!
//! No CADe_SIMU original (aplicação Win32 MFC), todas as strings de texto (etiquetas de componentes,
//! valores, campos do cajetín/carimbo e textos livres de `tipo == 8`) são gravadas na codificação
//! Windows-1252 (ANSI).
//!
//! A faixa de bytes `0x80..=0x9F` difere do ISO-8859-1 (Latin-1) e do Unicode puro, contendo caracteres
//! tipográficos e comerciais vitais (tais como '€', '“', '”', '–', '—', '•', '™', 'Š', 'Œ', etc.).
//!
//! Este módulo implementa conversão bidirecional sem perda (lossless roundtrip) para todos os 256 bytes,
//! além de detecção transparente entre UTF-8 e CP-1252 ao carregar esquemas.

/// Converte um único byte CP-1252 para o respectivo caractere Unicode (`char`).
#[inline]
pub fn cp1252_byte_to_char(b: u8) -> char {
    match b {
        // Faixa ASCII padrão (0x00..=0x7F)
        0x00..=0x7F => b as char,

        // Extensão Windows-1252 específica (0x80..=0x9F)
        0x80 => '€',        // U+20AC EURO SIGN
        0x81 => '\u{0081}', // Não atribuído no CP-1252 (preserva ponto de código)
        0x82 => '‚',        // U+201A SINGLE LOW-9 QUOTATION MARK
        0x83 => 'ƒ',        // U+0192 LATIN SMALL LETTER F WITH HOOK
        0x84 => '„',        // U+201E DOUBLE LOW-9 QUOTATION MARK
        0x85 => '…',        // U+2026 HORIZONTAL ELLIPSIS
        0x86 => '†',        // U+2020 DAGGER
        0x87 => '‡',        // U+2021 DOUBLE DAGGER
        0x88 => 'ˆ',        // U+02C6 MODIFIER LETTER CIRCUMFLEX ACCENT
        0x89 => '‰',        // U+2030 PER MILLE SIGN
        0x8A => 'Š',        // U+0160 LATIN CAPITAL LETTER S WITH CARON
        0x8B => '‹',        // U+2039 SINGLE LEFT-POINTING ANGLE QUOTATION MARK
        0x8C => 'Œ',        // U+0152 LATIN CAPITAL LIGATURE OE
        0x8D => '\u{008D}', // Não atribuído no CP-1252
        0x8E => 'Ž',        // U+017D LATIN CAPITAL LETTER Z WITH CARON
        0x8F => '\u{008F}', // Não atribuído no CP-1252
        0x90 => '\u{0090}', // Não atribuído no CP-1252
        0x91 => '‘',        // U+2018 LEFT SINGLE QUOTATION MARK
        0x92 => '’',        // U+2019 RIGHT SINGLE QUOTATION MARK
        0x93 => '“',        // U+201C LEFT DOUBLE QUOTATION MARK
        0x94 => '”',        // U+201D RIGHT DOUBLE QUOTATION MARK
        0x95 => '•',        // U+2022 BULLET
        0x96 => '–',        // U+2013 EN DASH
        0x97 => '—',        // U+2014 EM DASH
        0x98 => '˜',        // U+02DC SMALL TILDE
        0x99 => '™',        // U+2122 TRADE MARK SIGN
        0x9A => 'š',        // U+0161 LATIN SMALL LETTER S WITH CARON
        0x9B => '›',        // U+203A SINGLE RIGHT-POINTING ANGLE QUOTATION MARK
        0x9C => 'œ',        // U+0153 LATIN SMALL LIGATURE OE
        0x9D => '\u{009D}', // Não atribuído no CP-1252
        0x9E => 'ž',        // U+017E LATIN SMALL LETTER Z WITH CARON
        0x9F => 'Ÿ',        // U+0178 LATIN CAPITAL LETTER Y WITH DIAERESIS

        // Faixa Latin-1 estendida (0xA0..=0xFF) é idêntica ao Unicode U+00A0..=U+00FF
        0xA0..=0xFF => b as char,
    }
}

/// Mapeia um caractere Unicode para o seu byte correspondente em Windows-1252, se existir mapeamento exato.
#[inline]
pub fn char_to_cp1252_byte(c: char) -> Option<u8> {
    let u = c as u32;
    if u <= 0x7F {
        return Some(u as u8);
    }
    if (0xA0..=0xFF).contains(&u) {
        return Some(u as u8);
    }

    match c {
        '€' => Some(0x80),
        '\u{0081}' => Some(0x81),
        '‚' => Some(0x82),
        'ƒ' => Some(0x83),
        '„' => Some(0x84),
        '…' => Some(0x85),
        '†' => Some(0x86),
        '‡' => Some(0x87),
        'ˆ' => Some(0x88),
        '‰' => Some(0x89),
        'Š' => Some(0x8A),
        '‹' => Some(0x8B),
        'Œ' => Some(0x8C),
        '\u{008D}' => Some(0x8D),
        'Ž' => Some(0x8E),
        '\u{008F}' => Some(0x8F),
        '\u{0090}' => Some(0x90),
        '‘' => Some(0x91),
        '’' => Some(0x92),
        '“' => Some(0x93),
        '”' => Some(0x94),
        '•' => Some(0x95),
        '–' => Some(0x96),
        '—' => Some(0x97),
        '˜' => Some(0x98),
        '™' => Some(0x99),
        'š' => Some(0x9A),
        '›' => Some(0x9B),
        'œ' => Some(0x9C),
        '\u{009D}' => Some(0x9D),
        'ž' => Some(0x9E),
        'Ÿ' => Some(0x9F),
        _ => None,
    }
}

/// Converte um caractere para byte CP-1252 com fallback tolerante (lossy/transliteração)
/// para caracteres comuns de engenharia elétrica e tipografia.
#[inline]
pub fn char_to_cp1252_byte_lossy(c: char) -> u8 {
    if let Some(b) = char_to_cp1252_byte(c) {
        return b;
    }

    // Transliterações inteligentes para símbolos comuns em esquemas elétricos
    match c {
        // Micro grego (U+03BC) -> Micro Sign Latin-1 (U+00B5 = 0xB5)
        'μ' => 0xB5,
        // Ohm (U+03A9 ou U+2126) -> 'R'
        'Ω' | 'Ω' => b'R',
        // Diâmetro elétrico/furo (U+2300) -> Ø (0xD8)
        '⌀' => 0xD8,
        // Travessão / hífen tipográfico não quebrável
        '―' | '−' | '‑' => 0x96, // En dash (0x96)
        // Aspas tipográficas curvas adicionais
        '‟' | '″' | '‶' => 0x94, // ”
        '‛' | '′' | '‵' => 0x92, // ’
        // Marcadores de lista alternativos
        '◦' | '∙' | '⁃' | '▪' => 0x95, // •
        // Espaços especiais
        '\u{2009}' | '\u{200B}' | '\u{202F}' => b' ',
        _ => b'?',
    }
}

/// Decodifica uma sequência de bytes codificada em Windows-1252 para uma `String` UTF-8 em Rust.
pub fn decode_cp1252(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| cp1252_byte_to_char(b)).collect()
}

/// Codifica uma `str` UTF-8 para um vetor de bytes Windows-1252 utilizando fallback tolerante.
pub fn encode_cp1252(s: &str) -> Vec<u8> {
    s.chars().map(char_to_cp1252_byte_lossy).collect()
}

/// Codifica uma `str` UTF-8 para um vetor de bytes Windows-1252 estrito, retornando erro se encontrar caractere não mapeável.
pub fn encode_cp1252_strict(s: &str) -> Result<Vec<u8>, char> {
    s.chars().map(|c| char_to_cp1252_byte(c).ok_or(c)).collect()
}

/// Decodifica bytes brutos de um arquivo `.cad`.
///
/// Tenta primeiro validação UTF-8 pura (para esquemas modernos criados/salvos em UTF-8).
/// Se encontrar bytes não-UTF8 (o caso padrão de arquivos criados no CADe_SIMU original com acentuação),
/// decodifica usando Windows-1252 com mapeamento integral dos 256 bytes.
pub fn decode_cad_bytes(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => decode_cp1252(bytes),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_256_bytes_roundtrip() {
        for b in 0..=255u8 {
            let ch = cp1252_byte_to_char(b);
            let back = char_to_cp1252_byte(ch);
            assert_eq!(
                back,
                Some(b),
                "Byte 0x{:02X} falhou no roundtrip: char = {:?} (U+{:04X})",
                b,
                ch,
                ch as u32
            );
        }
    }

    #[test]
    fn test_cp1252_special_range_80_to_9f() {
        // Testa caracteres chave da faixa 0x80..=0x9F
        assert_eq!(cp1252_byte_to_char(0x80), '€');
        assert_eq!(cp1252_byte_to_char(0x85), '…');
        assert_eq!(cp1252_byte_to_char(0x8A), 'Š');
        assert_eq!(cp1252_byte_to_char(0x8C), 'Œ');
        assert_eq!(cp1252_byte_to_char(0x8E), 'Ž');
        assert_eq!(cp1252_byte_to_char(0x91), '‘');
        assert_eq!(cp1252_byte_to_char(0x92), '’');
        assert_eq!(cp1252_byte_to_char(0x93), '“');
        assert_eq!(cp1252_byte_to_char(0x94), '”');
        assert_eq!(cp1252_byte_to_char(0x95), '•');
        assert_eq!(cp1252_byte_to_char(0x96), '–');
        assert_eq!(cp1252_byte_to_char(0x97), '—');
        assert_eq!(cp1252_byte_to_char(0x99), '™');
        assert_eq!(cp1252_byte_to_char(0x9A), 'š');
        assert_eq!(cp1252_byte_to_char(0x9C), 'œ');
        assert_eq!(cp1252_byte_to_char(0x9E), 'ž');
        assert_eq!(cp1252_byte_to_char(0x9F), 'Ÿ');
    }

    #[test]
    fn test_portuguese_and_latin1_accents() {
        let text = "Inversão de rotação motor trifásico – Linha 1: 380V ± 5%, 60Hz, 10µF, Grau 45°";
        let encoded = encode_cp1252(text);
        let decoded = decode_cp1252(&encoded);
        assert_eq!(text, decoded);
    }

    #[test]
    fn test_lossy_transliterations() {
        // Micro grego U+03BC vira Micro Sign U+00B5 (0xB5)
        assert_eq!(char_to_cp1252_byte_lossy('μ'), 0xB5);
        // Ohm vira 'R'
        assert_eq!(char_to_cp1252_byte_lossy('Ω'), b'R');
        // Diâmetro vira Ø (0xD8)
        assert_eq!(char_to_cp1252_byte_lossy('⌀'), 0xD8);
        // Hífen tipográfico vira en dash (0x96)
        assert_eq!(char_to_cp1252_byte_lossy('−'), 0x96);
    }

    #[test]
    fn test_decode_cad_bytes_utf8_and_cp1252() {
        // UTF-8 válido
        let utf8_bytes = "Motor de Indução".as_bytes();
        assert_eq!(decode_cad_bytes(utf8_bytes), "Motor de Indução");

        // Bytes CP-1252 (com 'É' = 0xC9 e '–' = 0x96 que quebram UTF-8)
        let cp1252_raw = b"BLOG ENSINANDO EL\xC9TRICA \x96 PAINEL \x93PRINCIPAL\x94";
        let decoded = decode_cad_bytes(cp1252_raw);
        assert_eq!(decoded, "BLOG ENSINANDO ELÉTRICA – PAINEL “PRINCIPAL”");
    }
}
