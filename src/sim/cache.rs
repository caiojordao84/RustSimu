use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use crate::model::{SchematicItem, SimState};
use crate::sim::graph::{build_network_graph, NetworkGraph};

/// Gerenciador e utilitários de cache para a rede topológica do circuito.
/// Evita a reconstrução custosa $O(N^2)$ de colinearidade (`point_on_segment`) e
/// deduplicação de pontos a cada tick (dt_ms) da simulação.
pub struct NetworkCache;

impl NetworkCache {
    /// Calcula uma assinatura hash rápida de 64 bits baseada na geometria e topologia física dos elementos.
    /// Considera quantidade de itens, identificadores, tipos, coordenadas espaciais, orientação e arquivo QET.
    pub fn compute_topology_hash(items: &[SchematicItem]) -> u64 {
        let mut hasher = DefaultHasher::new();
        items.len().hash(&mut hasher);
        for item in items {
            item.id.hash(&mut hasher);
            item.tipo.hash(&mut hasher);
            item.x1.hash(&mut hasher);
            item.y1.hash(&mut hasher);
            item.x2.hash(&mut hasher);
            item.y2.hash(&mut hasher);
            item.orientation.hash(&mut hasher);
            if let Some(ref qet) = item.qet_type {
                qet.hash(&mut hasher);
            }
        }
        hasher.finish()
    }

    /// Obtém o grafo topológico pré-calculado do `SimState` ou reconstrói e armazena no cache se
    /// for a primeira execução ou se a geometria do circuito tiver sofrido mutação.
    pub fn get_or_build(items: &[SchematicItem], sim: &mut SimState) -> Arc<NetworkGraph> {
        let current_hash = Self::compute_topology_hash(items);
        let is_hit = match &sim.network_cache {
            Some(cached) => cached.topology_hash == current_hash,
            None => false,
        };

        if !is_hit {
            let graph = build_network_graph(items, current_hash);
            sim.network_cache = Some(Arc::new(graph));
        }

        sim.network_cache
            .as_ref()
            .expect("network_cache must be populated")
            .clone()
    }

    /// Invalida explicitamente o cache de rede do `SimState`.
    #[inline]
    pub fn invalidate(sim: &mut SimState) {
        sim.network_cache = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topology_hash_sensitivity() {
        let item1 = SchematicItem::new_wire(1, 4000, 10, 10, 100, 10);

        let hash1 = NetworkCache::compute_topology_hash(std::slice::from_ref(&item1));

        // Modifica coordenada -> hash deve mudar
        let item2 = SchematicItem::new_wire(1, 4000, 10, 10, 120, 10);
        let hash2 = NetworkCache::compute_topology_hash(std::slice::from_ref(&item2));
        assert_ne!(hash1, hash2);

        // Altera tag não-geométrica -> hash topológico permanece idêntico
        let mut item3 = item1.clone();
        item3.tag = "KM1".to_string();
        let hash3 = NetworkCache::compute_topology_hash(std::slice::from_ref(&item3));
        assert_eq!(hash1, hash3);
    }

    #[test]
    fn test_network_cache_hit_and_reuse() {
        let mut sim = SimState::default();
        let item = SchematicItem::new_wire(1, 4000, 0, 0, 10, 0);

        let g1 = NetworkCache::get_or_build(std::slice::from_ref(&item), &mut sim);
        let g2 = NetworkCache::get_or_build(std::slice::from_ref(&item), &mut sim);

        // Deve retornar o mesmo Arc (ponteiro idêntico no cache)
        assert!(Arc::ptr_eq(&g1, &g2));

        // Invalidação
        NetworkCache::invalidate(&mut sim);
        assert!(sim.network_cache.is_none());
    }
}
