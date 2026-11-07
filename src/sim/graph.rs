use std::collections::HashMap;

use crate::model::SchematicItem;
use crate::symbols::find_symbol_ref;

/// Verifica se um ponto `(px, py)` está exatamente sobre o segmento de cabo `(x1, y1) - (x2, y2)`.
pub fn point_on_segment(px: i32, py: i32, x1: i32, y1: i32, x2: i32, y2: i32) -> bool {
    if x1 == x2 && px == x1 {
        return py >= y1.min(y2) && py <= y1.max(y2);
    }
    if y1 == y2 && py == y1 {
        return px >= x1.min(x2) && px <= x1.max(x2);
    }
    // Caso diagonal geral (colinear e dentro da caixa delimitadora)
    let min_x = x1.min(x2);
    let max_x = x1.max(x2);
    let min_y = y1.min(y2);
    let max_y = y1.max(y2);
    if px < min_x || px > max_x || py < min_y || py > max_y {
        return false;
    }
    let cross = (px - x1) as i64 * (y2 - y1) as i64 - (py - y1) as i64 * (x2 - x1) as i64;
    cross == 0
}

/// Grafo elétrico e topológico estático pré-calculado de uma folha de circuito.
/// Armazena todos os pontos mundiais únicos (pontas de fios, nós 4001 e pinos de componentes),
/// mapeamentos de pinos por componente e a matriz de adjacência base formada pelos condutores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkGraph {
    /// Hash criptográfico/numérico dos elementos geométricos e conexões que geraram esta topologia.
    pub topology_hash: u64,
    /// Lista indexada de coordenadas globais únicas `(x, y)` dos nós elétricos.
    pub all_points: Vec<(i32, i32)>,
    /// Mapeamento reverso `(x, y) -> node_index`.
    pub pt_map: HashMap<(i32, i32), usize>,
    /// Índices dos fios: `(item_idx, node1, node2)`.
    pub wire_nodes: Vec<(usize, usize, usize)>,
    /// Mapeamento de pinos por índice de item no vetor `items`: `item_idx -> [node_a, node_b, ...]`.
    pub item_pin_nodes: HashMap<usize, Vec<usize>>,
    /// Lista de adjacência base formada exclusivamente por cabos e derivações de condutores.
    pub base_adj: Vec<Vec<usize>>,
}

impl NetworkGraph {
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.all_points.is_empty()
    }

    #[inline]
    pub fn num_nodes(&self) -> usize {
        self.all_points.len()
    }
}

/// Constrói o grafo topológico elétrico a partir da lista de elementos do circuito.
/// Mapeia todas as pontas de fios, junções (4001), pinos de símbolos nativos e pinos de símbolos QET.
/// Em seguida, subdivide cada condutor em todos os nós colineares interceptados (`point_on_segment`).
pub fn build_network_graph(items: &[SchematicItem], topology_hash: u64) -> NetworkGraph {
    let mut all_points: Vec<(i32, i32)> = Vec::new();
    let mut pt_map: HashMap<(i32, i32), usize> = HashMap::new();
    let mut add_unique_pt = |pt: (i32, i32)| -> usize {
        if let Some(&idx) = pt_map.get(&pt) {
            idx
        } else {
            let idx = all_points.len();
            all_points.push(pt);
            pt_map.insert(pt, idx);
            idx
        }
    };

    // 1. Registra pontas de cabos e pontos de conexão (empalmes 4001)
    let mut wire_nodes: Vec<(usize, usize, usize)> = Vec::new(); // (item_idx, n1, n2)
    for (idx, item) in items.iter().enumerate() {
        if item.is_wire() {
            let n1 = add_unique_pt((item.x1, item.y1));
            let n2 = add_unique_pt((item.x2, item.y2));
            wire_nodes.push((idx, n1, n2));
        } else if item.is_junction() {
            add_unique_pt((item.x1, item.y1));
        }
    }

    // 2. Registra os pinos de todos os símbolos
    let mut item_pin_nodes: HashMap<usize, Vec<usize>> = HashMap::new();
    for (idx, item) in items.iter().enumerate() {
        if let Some(sym) = find_symbol_ref(item.tipo) {
            let mut p_nodes = Vec::new();
            for (_, _, wpos) in sym.world_pins(item) {
                p_nodes.push(add_unique_pt(wpos));
            }
            item_pin_nodes.insert(idx, p_nodes);
        } else if let Some(qet_type) = item.symbol_id_or_slug() {
            if let Some(parsed) = crate::svg_symbol::get_parsed_svg(qet_type) {
                let mut p_nodes = Vec::new();
                for shape in &parsed.shapes {
                    if let crate::svg_symbol::SvgShape::Circle {
                        cx,
                        cy,
                        is_terminal: true,
                        ..
                    } = shape
                    {
                        let (lx, ly) = parsed.map_point(*cx, *cy, 1.0);
                        let (rx, ry) = crate::model::transform_offset_f32(lx, ly, item.orientation);
                        let wpos = (
                            (item.x1 as f32 + rx).round() as i32,
                            (item.y1 as f32 + ry).round() as i32,
                        );
                        p_nodes.push(add_unique_pt(wpos));
                    }
                }
                item_pin_nodes.insert(idx, p_nodes);
            }
        }
    }

    let num_nodes = all_points.len();
    let mut base_adj: Vec<Vec<usize>> = vec![Vec::new(); num_nodes];
    if num_nodes > 0 {
        // 3. Constrói adjacência estática dos cabos subdividindo cada segmento pelos pontos especiais (pinos, pontas e nós 4001)
        for &(_w_idx, n1, n2) in &wire_nodes {
            let (x1, y1) = all_points[n1];
            let (x2, y2) = all_points[n2];
            let mut on_seg: Vec<usize> = (0..num_nodes)
                .filter(|&u| {
                    let (px, py) = all_points[u];
                    point_on_segment(px, py, x1, y1, x2, y2)
                })
                .collect();
            on_seg.sort_by_key(|&u| all_points[u]);
            for pair in on_seg.windows(2) {
                let u = pair[0];
                let v = pair[1];
                if u != v {
                    base_adj[u].push(v);
                    base_adj[v].push(u);
                }
            }
        }
    }

    NetworkGraph {
        topology_hash,
        all_points,
        pt_map,
        wire_nodes,
        item_pin_nodes,
        base_adj,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_point_on_segment_collinearity() {
        // Horizontal
        assert!(point_on_segment(50, 20, 10, 20, 100, 20));
        assert!(!point_on_segment(50, 21, 10, 20, 100, 20));

        // Vertical
        assert!(point_on_segment(30, 45, 30, 10, 30, 90));
        assert!(!point_on_segment(31, 45, 30, 10, 30, 90));

        // Diagonal
        assert!(point_on_segment(20, 20, 10, 10, 30, 30));
        assert!(!point_on_segment(21, 20, 10, 10, 30, 30));
    }
}
