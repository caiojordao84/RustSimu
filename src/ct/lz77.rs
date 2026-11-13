//! Decompressor LZ77-custom do bloco COMPRESS (port de `decomprime_lz.py`).

const SEED: &[u8] = b"PREVIEW";

fn desmascara(data: &[u8], mascara: u8) -> Vec<u8> {
    data.iter().map(|b| b ^ mascara).collect()
}

struct Params {
    modo: u16,
    janela: usize,
    max_len: usize,
    bits: u16,
    #[allow(dead_code)]
    hdr: usize,
}

fn le_cabecalho(stream: &[u8]) -> Result<(Params, usize), String> {
    if stream.len() < 6 {
        return Err("stream curto demais para cabecalho".into());
    }
    let magica = u16::from_le_bytes([stream[0], stream[1]]);
    if magica != 1 {
        return Err(format!("magica invalida: {magica:#x}"));
    }
    let modo = u16::from_le_bytes([stream[2], stream[3]]);
    let hdr = u16::from_le_bytes([stream[4], stream[5]]) as usize;
    if modo == 0 {
        if stream.len() < 12 {
            return Err("cabecalho modo 0 incompleto".into());
        }
        let janela = u16::from_le_bytes([stream[6], stream[7]]) as usize;
        let max_len = u16::from_le_bytes([stream[8], stream[9]]) as usize;
        let bits = u16::from_le_bytes([stream[10], stream[11]]);
        return Ok((
            Params {
                modo,
                janela,
                max_len,
                bits,
                hdr,
            },
            hdr,
        ));
    }
    let bits_por_modo: &[(u16, u16)] = &[
        (0x80, 8),
        (0x200, 9),
        (0x400, 10),
        (0x800, 11),
        (0x1000, 0x70),
    ];
    let bits = bits_por_modo
        .iter()
        .find(|(m, _)| *m == modo)
        .map(|(_, b)| *b)
        .ok_or_else(|| format!("modo desconhecido: {modo:#x}"))?;
    Ok((
        Params {
            modo,
            janela: modo as usize,
            max_len: hdr,
            bits,
            hdr,
        },
        6,
    ))
}

fn decomprime(stream: &[u8]) -> Result<Vec<u8>, String> {
    let (params, mut p) = le_cabecalho(stream)?;
    let bits = params.bits as usize;
    let janela_sz = if params.janela == 0 {
        0x800
    } else {
        params.janela
    };
    let modo_hash = params.modo != 0;

    let mut jan = vec![0u8; 0x1000];
    let seed_len = SEED.len().min(jan.len());
    jan[..seed_len].copy_from_slice(&SEED[..seed_len]);
    let mut pos_j = seed_len;
    let mut saida = Vec::new();
    let n = stream.len();

    while p < n {
        let ctl = stream[p];
        if ctl < 0x80 {
            if p + 1 >= n {
                break;
            }
            let v = ((ctl as usize) << 8) | (stream[p + 1] as usize);
            let mut l = (v >> bits) + 1;
            let o = v & (janela_sz - 1);
            if l == 1 {
                if p + 3 > n {
                    break;
                }
                let lit = stream[p + 2];
                if o <= params.max_len && pos_j + o < janela_sz {
                    for i in 0..o {
                        jan[pos_j + i] = lit;
                    }
                    pos_j += o;
                }
                saida.resize(saida.len() + o, lit);
                p += 3;
            } else {
                if l == 2 && !modo_hash {
                    if p + 3 > n {
                        break;
                    }
                    l = stream[p + 2] as usize;
                    p += 3;
                } else {
                    p += 2;
                }
                if o + l <= jan.len() {
                    let chunk = jan[o..o + l].to_vec();
                    saida.extend_from_slice(&chunk);
                    if pos_j + 1 + l >= janela_sz {
                        pos_j = 0;
                    }
                    if pos_j + l <= jan.len() {
                        jan[pos_j..pos_j + l].copy_from_slice(&chunk);
                    }
                    pos_j += l;
                } else {
                    break;
                }
            }
        } else {
            let mut cnt = (ctl.wrapping_add(0x80)) as usize;
            if p + 1 + cnt > n {
                cnt = n.saturating_sub(p + 1);
            }
            let dados = &stream[p + 1..p + 1 + cnt];
            let mut i = 0;
            while i < cnt {
                if janela_sz <= pos_j + (cnt - i) {
                    let parte = janela_sz.saturating_sub(pos_j + 1);
                    if parte == 0 {
                        pos_j = 0;
                        continue;
                    }
                    jan[pos_j..pos_j + parte].copy_from_slice(&dados[i..i + parte]);
                    pos_j += parte;
                    i += parte;
                } else {
                    let parte = cnt - i;
                    jan[pos_j..pos_j + parte].copy_from_slice(&dados[i..i + parte]);
                    pos_j += parte;
                    i += parte;
                }
                if pos_j >= janela_sz {
                    pos_j = 0;
                }
            }
            saida.extend_from_slice(dados);
            p += 1 + cnt;
        }
    }
    Ok(saida)
}

/// Bytes brutos após a linha `COMPRESS\\n` → corpo texto.
pub fn decodifica_bloco_compress(bruto: &[u8]) -> Result<Vec<u8>, String> {
    let stream = desmascara(bruto, 0x9a);
    decomprime(&stream)
}

/// Comprime corpo texto → bloco COMPRESS (máscara `0x9a`).
///
/// Codificação didática: só literais (`ctl >= 0x80`), suficiente para o leitor.
pub fn codifica_bloco_compress(corpo: &[u8]) -> Result<Vec<u8>, String> {
    let mut stream = Vec::with_capacity(14 + corpo.len() + corpo.len() / 64 + 16);
    // Cabeçalho modo janela (14 bytes), shorts LE.
    stream.extend_from_slice(&1u16.to_le_bytes()); // mágica
    stream.extend_from_slice(&0u16.to_le_bytes()); // modo 0
    stream.extend_from_slice(&0x000eu16.to_le_bytes()); // hdr
    stream.extend_from_slice(&0x0800u16.to_le_bytes()); // janela 2048
    stream.extend_from_slice(&0x0010u16.to_le_bytes()); // max_len
    stream.extend_from_slice(&0x000bu16.to_le_bytes()); // bits
    stream.push(0x9a); // máscara (informativa; XOR aplica-se a todo o stream)
    stream.push(0x00);

    let mut i = 0usize;
    while i < corpo.len() {
        let n = (corpo.len() - i).min(127);
        stream.push(0x80u8.wrapping_add(n as u8));
        stream.extend_from_slice(&corpo[i..i + n]);
        i += n;
    }
    Ok(desmascara(&stream, 0x9a))
}

/// Stream completo XOR-mascarado (ex.: `lang_map.bin` modo hash `0x0200`).
pub fn decomprime_mascarado(bruto: &[u8], mascara: u8) -> Result<Vec<u8>, String> {
    let stream = desmascara(bruto, mascara);
    decomprime(&stream)
}
