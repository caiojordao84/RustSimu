//! Motor de ar didático (P3) + acoplamento eletropneumático (P4).
//!
//! Sem Iced e sem o solver elétrico do RustSimu. Cada tubo é um nó com
//! [`NoAr`]. Válvulas direcionais ligam pares de vias; `PVS1`/`PVS3` são
//! lógica de pressão (OU/E). Cilindro: curso binário.
//!
//! P4: o bool `energizado` (A1–A2) entra de fora; o pressostato só exporta
//! `contato_fechado`. Este módulo **não** propaga `L1`/`N`.

use crate::pneu::contrato::{
    Acionamento, CursoCilindro, EstadoCarretel, EstadoPistao, NoAr, ParVias, Valvula,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

/// Velocidade máxima didática (curso/s) na pressão/vazão nominais (0,6 MPa e 1000 l/min → ~0,28 s por curso).
const V_MAX_CURSO_S: f32 = 3.6;
/// Amortecimento de 1ª ordem (1/s).
const AMORTECIMENTO: f32 = 25.0;
/// Pressão operacional padrão FluidSIM em kPa (0,60 MPa = 600 kPa).
pub const PRESSAO_PADRAO_KPA: u32 = 600;
/// Vazão máxima padrão FluidSIM em l/min (1000 l/min).
pub const VAZAO_PADRAO_LPM: u32 = 1000;

/// Porta nomeada ligada a um nó de tubo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Porta {
    pub nome: String,
    pub no: u32,
}

impl Porta {
    pub fn nova(nome: impl Into<String>, no: u32) -> Self {
        Self {
            nome: nome.into(),
            no,
        }
    }
}

/// Comando elétrico externo (bool vindo do solver elétrico no merge).
///
/// A válvula **não** é uma bobina: só consome o estado A1–A2.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ComandoEletrico {
    #[default]
    Nenhum,
    /// `PVWE*` monoestável: energizado → Acionado; senão mola → Repouso.
    Monoestavel { energizado: bool },
    /// `PVWE3` / `PVWP4` impulso: borda false→true alterna e retém.
    Impulso { energizado: bool, anterior: bool },
}

/// Dispositivo pneumático no circuito.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dispositivo {
    /// Injeta [`NoAr::Suprimento`] no nó.
    Fonte { id: u32, no: u32 },
    /// Marca o nó como atmosfera ([`NoAr::Escape`]).
    Escape { id: u32, no: u32 },
    /// Direcional: tabela de vias do [`Valvula`].
    Direcional {
        id: u32,
        valvula: Valvula,
        portas: Vec<Porta>,
        comando: ComandoEletrico,
    },
    /// `PVS1` — saída = OU das entradas.
    Alternancia { id: u32, a: u32, b: u32, saida: u32 },
    /// `PVS3` — saída = E das entradas.
    DuasPressoes { id: u32, a: u32, b: u32, saida: u32 },
    /// Cilindro dupla ação: câmaras avanço/retorno + pistão contínuo.
    CilindroDuplo {
        id: u32,
        avanco: u32,
        retorno: u32,
        pistao: EstadoPistao,
    },
    /// Cilindro simples ação: câmara + mola de retorno + pistão contínuo.
    CilindroSimples {
        id: u32,
        camara: u32,
        pistao: EstadoPistao,
    },
    /// Fim de curso: ativo só no curso associado; comuta a 3/2 embutida.
    FimDeCurso {
        id: u32,
        cilindro: u32,
        ativo_em: CursoCilindro,
        ativo: bool,
        valvula: Valvula,
        portas: Vec<Porta>,
    },

    /// `PVS4` — retenção: passa suprimento só entrada→saída.
    Retencao { id: u32, entrada: u32, saida: u32 },
    /// `PVG2`/`PVG3` — temporizador 3/2 (NF=false abre após atraso; NA=true fecha após).
    Temporizador {
        id: u32,
        valvula: Valvula,
        portas: Vec<Porta>,
        /// true = NA (PVG3); false = NF (PVG2).
        normalmente_aberto: bool,
        atraso_ms: u32,
        acumulado_ms: u32,
        sinal: u32,
    },
    /// `PAV1` — bomba de vácuo: injeta Escape (sucção didática) no nó.
    BombaVacuo { id: u32, no: u32 },
    /// `PAR2` — motor pneumático: conduta 1↔2; `girando` se há Δp didático.
    MotorPneumatico {
        id: u32,
        a: u32,
        b: u32,
        girando: bool,
    },

    /// `PSW1` / `PEW`: suprimento no nó de ar fecha o contato elétrico de saída.
    Pressostato {
        id: u32,
        no_ar: u32,
        contato_fechado: bool,
    },
}

/// Circuito pneumático e estado dos nós.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MotorAr {
    pub nos: BTreeMap<u32, NoAr>,
    /// Pressão manométrica em cada nó em kPa (`600` = `0.60 MPa`, `0` = `0.00 MPa`).
    pub nos_pressao_kpa: BTreeMap<u32, u32>,
    /// Configuração de pressão (`kPa`) e vazão (`l/min`) por `id` de `Dispositivo::Fonte`.
    pub fontes_config: BTreeMap<u32, (u32, u32)>,
    pub dispositivos: Vec<Dispositivo>,
    pub tubos: Vec<(u32, u32)>,
}

impl MotorAr {
    pub fn novo() -> Self {
        Self::default()
    }

    pub fn garante_no(&mut self, id: u32) {
        self.nos.entry(id).or_insert(NoAr::SemPressao);
    }

    pub fn no(&self, id: u32) -> NoAr {
        self.nos.get(&id).copied().unwrap_or(NoAr::SemPressao)
    }

    /// Define a pressão operacional (`kPa`, ex: `600` = `0.60 MPa`) e vazão máxima (`l/min`) de uma fonte.
    pub fn set_fonte_config(&mut self, id: u32, pressao_kpa: u32, vazao_lpm: u32) {
        self.fontes_config
            .insert(id, (pressao_kpa.min(2000), vazao_lpm.min(5000)));
    }

    /// Retorna `(pressao_kpa, vazao_lpm)` configurados para a fonte `id` (padrão: `600 kPa`, `1000 l/min`).
    pub fn fonte_config(&self, id: u32) -> (u32, u32) {
        self.fontes_config
            .get(&id)
            .copied()
            .unwrap_or((PRESSAO_PADRAO_KPA, VAZAO_PADRAO_LPM))
    }

    /// Pressão efetiva no nó `id` em `kPa` (`600` = `0.60 MPa`, `0` em escape/sem pressão).
    pub fn pressao_kpa(&self, id: u32) -> u32 {
        if self.no(id) == NoAr::Suprimento {
            self.nos_pressao_kpa
                .get(&id)
                .copied()
                .unwrap_or(PRESSAO_PADRAO_KPA)
        } else {
            0
        }
    }

    /// Pressão efetiva no nó `id` em `MPa` (`0.60` no padrão FluidSIM, `0.00` em escape/sem pressão).
    pub fn pressao_mpa(&self, id: u32) -> f32 {
        self.pressao_kpa(id) as f32 / 1000.0
    }

    /// Vazão máxima efetiva do circuito em `l/min` (padrão `1000 l/min`).
    pub fn vazao_efetiva_lpm(&self) -> u32 {
        let mut tem_fonte = false;
        let mut max_q = 0u32;
        for d in &self.dispositivos {
            if let Dispositivo::Fonte { id, .. } = d {
                tem_fonte = true;
                let (_, q) = self.fonte_config(*id);
                max_q = max_q.max(q);
            }
        }
        if tem_fonte {
            max_q
        } else {
            VAZAO_PADRAO_LPM
        }
    }

    pub fn pistao(&self, id: u32) -> Option<EstadoPistao> {
        self.dispositivos.iter().find_map(|d| match d {
            Dispositivo::CilindroDuplo { id: i, pistao, .. } if *i == id => Some(*pistao),
            Dispositivo::CilindroSimples { id: i, pistao, .. } if *i == id => Some(*pistao),
            _ => None,
        })
    }

    /// Curso binário derivado dos limiares de [`EstadoPistao`].
    pub fn curso_cilindro(&self, id: u32) -> Option<CursoCilindro> {
        self.pistao(id).map(|p| p.curso())
    }

    pub fn fim_de_curso_ativo(&self, id: u32) -> Option<bool> {
        self.dispositivos.iter().find_map(|d| match d {
            Dispositivo::FimDeCurso { id: i, ativo, .. } if *i == id => Some(*ativo),
            _ => None,
        })
    }

    /// Contato elétrico de saída de um [`Dispositivo::Pressostato`].
    pub fn contato_fechado(&self, id: u32) -> Option<bool> {
        self.dispositivos.iter().find_map(|d| match d {
            Dispositivo::Pressostato {
                id: i,
                contato_fechado,
                ..
            } if *i == id => Some(*contato_fechado),
            _ => None,
        })
    }

    /// Motor pneumático (`PAR2`) está girando sob Δp.
    pub fn motor_girando(&self, id: u32) -> Option<bool> {
        self.dispositivos.iter().find_map(|d| match d {
            Dispositivo::MotorPneumatico { id: i, girando, .. } if *i == id => Some(*girando),
            _ => None,
        })
    }

    /// Define o bool A1–A2 de uma direcional com comando elétrico.
    ///
    /// Retorna `false` se o id não for direcional elétrica.
    pub fn set_energizado(&mut self, id: u32, energizado: bool) -> bool {
        for d in &mut self.dispositivos {
            if let Dispositivo::Direcional { id: i, comando, .. } = d {
                if *i != id {
                    continue;
                }
                match comando {
                    ComandoEletrico::Monoestavel { energizado: e } => {
                        *e = energizado;
                        return true;
                    }
                    ComandoEletrico::Impulso { energizado: e, .. } => {
                        *e = energizado;
                        return true;
                    }
                    ComandoEletrico::Nenhum => return false,
                }
            }
        }
        false
    }

    /// Um passo com `dt` grande (≈1 s): completa o curso sob força típica (compat P3–P6).
    pub fn passo(&mut self) {
        self.passo_dt(1.0);
    }

    /// Passo com integração contínua do pistão (`dt` em segundos).
    pub fn passo_dt(&mut self, dt: f32) {
        self.aplica_comando_eletrico();
        self.atualiza_temporizadores(dt.max(0.0));
        self.sincroniza_fins_de_curso();
        self.propaga_ar();
        self.atualiza_cilindros(dt.max(0.0));
        self.sincroniza_fins_de_curso();
        self.propaga_ar();
        self.atualiza_pressostatos();
        self.atualiza_motores();
    }

    fn aplica_comando_eletrico(&mut self) {
        for d in &mut self.dispositivos {
            if let Dispositivo::Direcional {
                valvula, comando, ..
            } = d
            {
                match comando {
                    ComandoEletrico::Nenhum => {}
                    ComandoEletrico::Monoestavel { energizado } => {
                        valvula.estado = if *energizado {
                            EstadoCarretel::Acionado
                        } else {
                            EstadoCarretel::Repouso
                        };
                    }
                    ComandoEletrico::Impulso {
                        energizado,
                        anterior,
                    } => {
                        if *energizado && !*anterior {
                            valvula.estado = match valvula.estado {
                                EstadoCarretel::Repouso => EstadoCarretel::Acionado,
                                EstadoCarretel::Acionado => EstadoCarretel::Repouso,
                            };
                        }
                        *anterior = *energizado;
                    }
                }
            }
        }
    }

    fn atualiza_pressostatos(&mut self) {
        let nos = self.nos.clone();
        for d in &mut self.dispositivos {
            if let Dispositivo::Pressostato {
                no_ar,
                contato_fechado,
                ..
            } = d
            {
                let ar = nos.get(no_ar).copied().unwrap_or(NoAr::SemPressao);
                *contato_fechado = ar == NoAr::Suprimento;
            }
        }
    }

    fn sincroniza_fins_de_curso(&mut self) {
        let pistoes: BTreeMap<u32, EstadoPistao> = self
            .dispositivos
            .iter()
            .filter_map(|d| match d {
                Dispositivo::CilindroDuplo { id, pistao, .. }
                | Dispositivo::CilindroSimples { id, pistao, .. } => Some((*id, *pistao)),
                _ => None,
            })
            .collect();

        for d in &mut self.dispositivos {
            if let Dispositivo::FimDeCurso {
                cilindro,
                ativo_em,
                ativo,
                valvula,
                ..
            } = d
            {
                let no_fim = pistoes.get(cilindro).is_some_and(|p| match *ativo_em {
                    CursoCilindro::Avancado => p.no_avancado(),
                    CursoCilindro::Recolhido => p.no_recolhido(),
                });
                *ativo = no_fim;
                valvula.estado = if no_fim {
                    EstadoCarretel::Acionado
                } else {
                    EstadoCarretel::Repouso
                };
            }
        }
    }

    fn propaga_ar(&mut self) {
        for no in self.nos.values_mut() {
            *no = NoAr::SemPressao;
        }
        self.nos_pressao_kpa.clear();

        // 1) Injeções diretas.
        let mut fontes = Vec::new();
        let mut fontes_kpa = Vec::new();
        let mut escapes = Vec::new();
        for d in &self.dispositivos {
            match d {
                Dispositivo::Fonte { id, no } => {
                    let (p_kpa, _) = self.fonte_config(*id);
                    if p_kpa > 0 {
                        fontes.push(*no);
                        fontes_kpa.push((*no, p_kpa));
                    }
                }
                Dispositivo::Escape { no, .. } | Dispositivo::BombaVacuo { no, .. } => {
                    escapes.push(*no)
                }
                _ => {}
            }
        }
        for &(n, p_kpa) in &fontes_kpa {
            self.garante_no(n);
            self.nos.insert(n, NoAr::Suprimento);
            let cur = self.nos_pressao_kpa.entry(n).or_insert(0);
            *cur = (*cur).max(p_kpa);
        }
        for n in &escapes {
            self.garante_no(*n);
            // Escape só se ainda não for fonte no mesmo nó.
            if self.no(*n) != NoAr::Suprimento {
                self.nos.insert(*n, NoAr::Escape);
            }
        }

        // 2) Grafo: arestas pelas vias abertas das direcionais e fins de curso.
        let mut adj: BTreeMap<u32, BTreeSet<u32>> = BTreeMap::new();
        let liga = |a: u32, b: u32, adj: &mut BTreeMap<u32, BTreeSet<u32>>| {
            adj.entry(a).or_default().insert(b);
            adj.entry(b).or_default().insert(a);
        };

        for &(na, nb) in &self.tubos {
            liga(na, nb, &mut adj);
        }

        for d in &self.dispositivos {
            let (valvula, portas) = match d {
                Dispositivo::Direcional {
                    valvula, portas, ..
                }
                | Dispositivo::FimDeCurso {
                    valvula, portas, ..
                } => (valvula, portas),
                _ => continue,
            };
            let mapa: BTreeMap<&str, u32> =
                portas.iter().map(|p| (p.nome.as_str(), p.no)).collect();
            for par in valvula.pares_ligados() {
                if let (Some(&na), Some(&nb)) = (mapa.get(par.a.as_str()), mapa.get(par.b.as_str()))
                {
                    liga(na, nb, &mut adj);
                }
            }
        }

        // Temporizadores usam a mesma tabela 3/2 via `valvula`.
        for d in &self.dispositivos {
            if let Dispositivo::Temporizador {
                valvula, portas, ..
            } = d
            {
                let mapa: BTreeMap<&str, u32> =
                    portas.iter().map(|p| (p.nome.as_str(), p.no)).collect();
                for par in valvula.pares_ligados() {
                    if let (Some(&na), Some(&nb)) =
                        (mapa.get(par.a.as_str()), mapa.get(par.b.as_str()))
                    {
                        liga(na, nb, &mut adj);
                    }
                }
            }
        }

        // Motor: conduta simétrica.
        for d in &self.dispositivos {
            if let Dispositivo::MotorPneumatico { a, b, .. } = d {
                liga(*a, *b, &mut adj);
            }
        }

        // 3) Propagação BFS de suprimento e escape pelo grafo.
        self.propaga_rotulo(NoAr::Suprimento, &fontes, &adj);
        self.propaga_pressao_kpa(&fontes_kpa, &adj);
        // Escape não sobrescreve suprimento.
        self.propaga_rotulo_fraco(NoAr::Escape, &escapes, &adj);

        // 4) Lógica PVS1 / PVS3 (itera até estabilizar — cadeias curtas).
        for _ in 0..8 {
            let snapshot = self.nos.clone();
            let snap_kpa = self.nos_pressao_kpa.clone();
            let mut updates: Vec<(u32, NoAr, u32)> = Vec::new();
            for d in &self.dispositivos {
                match d {
                    Dispositivo::Retencao { entrada, saida, .. } => {
                        let ve = snapshot.get(entrada).copied().unwrap_or(NoAr::SemPressao);
                        let pk = snap_kpa.get(entrada).copied().unwrap_or(PRESSAO_PADRAO_KPA);
                        // Unidirecional: só propaga suprimento para a frente.
                        let (out, out_kpa) = if ve == NoAr::Suprimento {
                            (NoAr::Suprimento, pk)
                        } else {
                            (NoAr::SemPressao, 0)
                        };
                        updates.push((*saida, out, out_kpa));
                    }
                    Dispositivo::Alternancia { a, b, saida, .. } => {
                        let va = snapshot.get(a).copied().unwrap_or(NoAr::SemPressao);
                        let vb = snapshot.get(b).copied().unwrap_or(NoAr::SemPressao);
                        let pa = if va == NoAr::Suprimento {
                            snap_kpa.get(a).copied().unwrap_or(PRESSAO_PADRAO_KPA)
                        } else {
                            0
                        };
                        let pb = if vb == NoAr::Suprimento {
                            snap_kpa.get(b).copied().unwrap_or(PRESSAO_PADRAO_KPA)
                        } else {
                            0
                        };
                        let (out, out_kpa) = if va == NoAr::Suprimento || vb == NoAr::Suprimento {
                            (NoAr::Suprimento, pa.max(pb))
                        } else if va == NoAr::Escape || vb == NoAr::Escape {
                            (NoAr::Escape, 0)
                        } else {
                            (NoAr::SemPressao, 0)
                        };
                        updates.push((*saida, out, out_kpa));
                    }
                    Dispositivo::DuasPressoes { a, b, saida, .. } => {
                        let va = snapshot.get(a).copied().unwrap_or(NoAr::SemPressao);
                        let vb = snapshot.get(b).copied().unwrap_or(NoAr::SemPressao);
                        let pa = snap_kpa.get(a).copied().unwrap_or(PRESSAO_PADRAO_KPA);
                        let pb = snap_kpa.get(b).copied().unwrap_or(PRESSAO_PADRAO_KPA);
                        let (out, out_kpa) = if va == NoAr::Suprimento && vb == NoAr::Suprimento {
                            (NoAr::Suprimento, pa.min(pb))
                        } else if va == NoAr::Escape && vb == NoAr::Escape {
                            (NoAr::Escape, 0)
                        } else {
                            // AND incompleto ou ambas vazias → sem pressão na saída.
                            (NoAr::SemPressao, 0)
                        };
                        updates.push((*saida, out, out_kpa));
                    }
                    _ => {}
                }
            }
            let mut mudou = false;
            for (saida, out, out_kpa) in updates {
                self.garante_no(saida);
                if self.no(saida) != out {
                    self.nos.insert(saida, out);
                    mudou = true;
                }
                if out == NoAr::Suprimento && out_kpa > 0 {
                    let cur = self.nos_pressao_kpa.entry(saida).or_insert(0);
                    if *cur != (*cur).max(out_kpa) {
                        *cur = (*cur).max(out_kpa);
                        mudou = true;
                    }
                }
            }
            if !mudou {
                break;
            }
            // Repropaga pelos tubos após lógica (saída pode alimentar outra válvula).
            let seeds_s: Vec<u32> = self
                .nos
                .iter()
                .filter(|(_, e)| **e == NoAr::Suprimento)
                .map(|(k, _)| *k)
                .collect();
            let seeds_kpa: Vec<(u32, u32)> = seeds_s
                .iter()
                .map(|&k| {
                    let pk = self
                        .nos_pressao_kpa
                        .get(&k)
                        .copied()
                        .unwrap_or(PRESSAO_PADRAO_KPA);
                    (k, pk)
                })
                .collect();
            let seeds_e: Vec<u32> = self
                .nos
                .iter()
                .filter(|(_, e)| **e == NoAr::Escape)
                .map(|(k, _)| *k)
                .collect();
            self.propaga_rotulo(NoAr::Suprimento, &seeds_s, &adj);
            self.propaga_pressao_kpa(&seeds_kpa, &adj);
            self.propaga_rotulo_fraco(NoAr::Escape, &seeds_e, &adj);
        }
    }

    fn propaga_pressao_kpa(&mut self, sementes: &[(u32, u32)], adj: &BTreeMap<u32, BTreeSet<u32>>) {
        let mut fila: VecDeque<(u32, u32)> = sementes.iter().copied().collect();
        while let Some((n, pk)) = fila.pop_front() {
            let cur = self.nos_pressao_kpa.entry(n).or_insert(0);
            if pk > *cur {
                *cur = pk;
            }
            let p_atual = *cur;
            if let Some(viz) = adj.get(&n) {
                for &v in viz {
                    let vp = self.nos_pressao_kpa.get(&v).copied().unwrap_or(0);
                    if p_atual > vp {
                        self.nos_pressao_kpa.insert(v, p_atual);
                        fila.push_back((v, p_atual));
                    }
                }
            }
        }
    }

    fn propaga_rotulo(
        &mut self,
        rotulo: NoAr,
        sementes: &[u32],
        adj: &BTreeMap<u32, BTreeSet<u32>>,
    ) {
        let mut fila: VecDeque<u32> = sementes.iter().copied().collect();
        let mut visto: BTreeSet<u32> = sementes.iter().copied().collect();
        while let Some(n) = fila.pop_front() {
            self.garante_no(n);
            self.nos.insert(n, rotulo);
            if let Some(viz) = adj.get(&n) {
                for &v in viz {
                    if visto.insert(v) {
                        fila.push_back(v);
                    }
                }
            }
        }
    }

    /// Como [`propaga_rotulo`], mas não sobrescreve [`NoAr::Suprimento`].
    fn propaga_rotulo_fraco(
        &mut self,
        rotulo: NoAr,
        sementes: &[u32],
        adj: &BTreeMap<u32, BTreeSet<u32>>,
    ) {
        let mut fila: VecDeque<u32> = sementes.iter().copied().collect();
        let mut visto: BTreeSet<u32> = sementes.iter().copied().collect();
        while let Some(n) = fila.pop_front() {
            self.garante_no(n);
            if self.no(n) != NoAr::Suprimento {
                self.nos.insert(n, rotulo);
            }
            if let Some(viz) = adj.get(&n) {
                for &v in viz {
                    if visto.insert(v) {
                        fila.push_back(v);
                    }
                }
            }
        }
    }

    fn atualiza_cilindros(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let vazao_fator =
            (self.vazao_efetiva_lpm() as f32 / VAZAO_PADRAO_LPM as f32).clamp(0.0, 5.0);
        let nos = self.nos.clone();
        let nos_kpa = self.nos_pressao_kpa.clone();
        let fator_no = |n: u32| -> f32 {
            if nos.get(&n).copied().unwrap_or(NoAr::SemPressao) == NoAr::Suprimento {
                let kpa = nos_kpa.get(&n).copied().unwrap_or(PRESSAO_PADRAO_KPA);
                (kpa as f32 / PRESSAO_PADRAO_KPA as f32).clamp(0.0, 3.5)
            } else {
                0.0
            }
        };
        for d in &mut self.dispositivos {
            match d {
                Dispositivo::CilindroDuplo {
                    avanco,
                    retorno,
                    pistao,
                    ..
                } => {
                    let a = fator_no(*avanco);
                    let r = fator_no(*retorno);
                    integra_pistao(pistao, (a - r) * vazao_fator, dt);
                }
                Dispositivo::CilindroSimples { camara, pistao, .. } => {
                    let c = fator_no(*camara);
                    // Com pressão acima do limiar da mola, avança proporcional à pressão e vazão;
                    // em escape/sem pressão, a mola pré-comprimida retorna o êmbolo rapidamente (<1s).
                    let forca = if c > 0.10 {
                        c * vazao_fator
                    } else {
                        -vazao_fator.max(0.25)
                    };
                    integra_pistao(pistao, forca, dt);
                }
                _ => {}
            }
        }
    }

    fn atualiza_temporizadores(&mut self, dt: f32) {
        if dt <= 0.0 {
            return;
        }
        let dt_ms = (dt * 1000.0).round().max(0.0) as u32;
        let nos = self.nos.clone();
        for d in &mut self.dispositivos {
            if let Dispositivo::Temporizador {
                valvula,
                normalmente_aberto,
                atraso_ms,
                acumulado_ms,
                sinal,
                ..
            } = d
            {
                let sinal_ok =
                    nos.get(sinal).copied().unwrap_or(NoAr::SemPressao) == NoAr::Suprimento;
                if sinal_ok {
                    *acumulado_ms = (*acumulado_ms).saturating_add(dt_ms).min(*atraso_ms + 1000);
                } else {
                    *acumulado_ms = 0;
                }
                let disparou = *acumulado_ms >= *atraso_ms;
                // 3/2 NC: Repouso=2↔3, Acionado=1↔2. NF abre após atraso; NA fecha após.
                let acionado = if *normalmente_aberto {
                    !disparou
                } else {
                    disparou
                };
                valvula.estado = if acionado {
                    EstadoCarretel::Acionado
                } else {
                    EstadoCarretel::Repouso
                };
            }
        }
    }

    fn atualiza_motores(&mut self) {
        let nos = self.nos.clone();
        for d in &mut self.dispositivos {
            if let Dispositivo::MotorPneumatico { a, b, girando, .. } = d {
                let va = nos.get(a).copied().unwrap_or(NoAr::SemPressao);
                let vb = nos.get(b).copied().unwrap_or(NoAr::SemPressao);
                *girando = va == NoAr::Suprimento || vb == NoAr::Suprimento;
            }
        }
    }
}

fn integra_pistao(p: &mut EstadoPistao, forca: f32, dt: f32) {
    let vel_alvo = forca.clamp(-3.5, 3.5) * V_MAX_CURSO_S;
    let mut vel = p.vel_pm_s as f32 / 1000.0;
    let alpha = (AMORTECIMENTO * dt).min(1.0);
    vel += (vel_alvo - vel) * alpha;
    let mut pos = p.pos() + vel * dt;
    if pos <= 0.0 {
        pos = 0.0;
        vel = 0.0;
    } else if pos >= 1.0 {
        pos = 1.0;
        vel = 0.0;
    }
    p.pos_pm = (pos * 1000.0).round().clamp(0.0, 1000.0) as u16;
    p.vel_pm_s = (vel * 1000.0).round() as i32;
}

/// Tabela ISO usual de 5/2: repouso 1–2 e 4–5; acionado 1–4 e 2–3.
pub fn valvula_5_2(estado: EstadoCarretel, acionamentos: Vec<Acionamento>) -> Valvula {
    Valvula::nova(
        estado,
        vec![
            (
                EstadoCarretel::Repouso,
                vec![ParVias::novo("1", "2"), ParVias::novo("4", "5")],
            ),
            (
                EstadoCarretel::Acionado,
                vec![ParVias::novo("1", "4"), ParVias::novo("2", "3")],
            ),
        ],
        acionamentos,
    )
}

/// Tabela ISO usual de 3/2 (via 1→2 em acionado; 2→3 em repouso — normalmente fechada).
pub fn valvula_3_2_nc(estado: EstadoCarretel, acionamentos: Vec<Acionamento>) -> Valvula {
    Valvula::nova(
        estado,
        vec![
            (EstadoCarretel::Repouso, vec![ParVias::novo("2", "3")]),
            (EstadoCarretel::Acionado, vec![ParVias::novo("1", "2")]),
        ],
        acionamentos,
    )
}
