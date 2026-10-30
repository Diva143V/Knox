//! In-memory LRU cache for scoped petgraph subgraphs.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use super::loader::{KnoxGraph, load_full_graph, load_subgraph};
use crate::core::error::KnoxResult;
use crate::core::ids::ProjectId;
use crate::core::types::QueryFilter;
use crate::storage::db::KnoxDb;
use crate::storage::queries::query_actions;

/// Default capacity for the subgraph LRU cache.
pub const DEFAULT_CACHE_CAPACITY: usize = 32;

/// Scope key for caching subgraphs by project and/or target file path.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// Optional project scope.
    pub project_id: Option<ProjectId>,
    /// Optional relative file path scope.
    pub file_path: Option<String>,
}

impl CacheKey {
    /// Creates a new `CacheKey`.
    pub fn new(project_id: Option<ProjectId>, file_path: Option<String>) -> Self {
        Self {
            project_id,
            file_path,
        }
    }

    /// Creates a global cache key (no project or file filter).
    pub fn global() -> Self {
        Self {
            project_id: None,
            file_path: None,
        }
    }
}

impl From<(Option<ProjectId>, Option<String>)> for CacheKey {
    fn from((project_id, file_path): (Option<ProjectId>, Option<String>)) -> Self {
        Self {
            project_id,
            file_path,
        }
    }
}

/// Least-Recently-Used (LRU) in-memory cache for petgraph subgraphs.
pub struct GraphCache {
    capacity: usize,
    entries: HashMap<CacheKey, Arc<KnoxGraph>>,
    order: VecDeque<CacheKey>,
}

impl GraphCache {
    /// Creates a new `GraphCache` with the specified maximum capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity: capacity.max(1),
            entries: HashMap::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
        }
    }

    /// Retrieves a cached subgraph, refreshing its position in the LRU order.
    pub fn get(&mut self, key: &CacheKey) -> Option<Arc<KnoxGraph>> {
        if let Some(graph) = self.entries.get(key).cloned() {
            // Move key to the back of the queue (most recently used)
            if let Some(pos) = self.order.iter().position(|k| k == key) {
                self.order.remove(pos);
            }
            self.order.push_back(key.clone());
            Some(graph)
        } else {
            None
        }
    }

    /// Retrieves a cached subgraph or loads it from SQLite persistence on a cache miss.
    pub fn get_or_load(&mut self, db: &KnoxDb, key: &CacheKey) -> KnoxResult<Arc<KnoxGraph>> {
        if let Some(cached) = self.get(key) {
            return Ok(cached);
        }

        // Cache miss: load from storage based on scope
        let graph = if let Some(ref fp) = key.file_path {
            let filter = QueryFilter {
                file_path: Some(fp.clone()),
                current_only: false,
                limit: 100,
                ..Default::default()
            };

            let query_res = query_actions(db, &filter)?;
            let anchor_ids: Vec<_> = query_res.results.iter().map(|a| a.id.clone()).collect();
            load_subgraph(db, &anchor_ids, 3)?
        } else if let Some(ref pid) = key.project_id {
            let filter = QueryFilter {
                project: Some(pid.to_string()),
                current_only: false,
                limit: 200,
                ..Default::default()
            };

            let query_res = query_actions(db, &filter)?;
            let anchor_ids: Vec<_> = query_res.results.iter().map(|a| a.id.clone()).collect();
            load_subgraph(db, &anchor_ids, 3)?
        } else {
            load_full_graph(db)?
        };

        let arc_graph = Arc::new(graph);
        self.insert(key.clone(), Arc::clone(&arc_graph));
        Ok(arc_graph)
    }

    /// Stores a subgraph in the cache, evicting the least-recently used entry if at capacity.
    pub fn insert(&mut self, key: CacheKey, graph: Arc<KnoxGraph>) {
        if self.entries.contains_key(&key) {
            if let Some(pos) = self.order.iter().position(|k| *k == key) {
                self.order.remove(pos);
            }
        } else if self.entries.len() >= self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            }
        }

        self.order.push_back(key.clone());
        self.entries.insert(key, graph);
    }

    /// Invalidates an exact cache entry.
    pub fn invalidate(&mut self, key: &CacheKey) {
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            self.order.remove(pos);
        }
        self.entries.remove(key);
    }

    /// Write-through invalidation: evicts any cached subgraphs matching the affected project or files,
    /// and drops the cached global full graph.
    pub fn invalidate_write(&mut self, project_id: Option<&ProjectId>, files: &[String]) {
        let global_key = CacheKey::global();
        self.invalidate(&global_key);

        let keys_to_remove: Vec<CacheKey> = self
            .entries
            .keys()
            .filter(|k| {
                if let (Some(p_target), Some(p_key)) = (project_id, &k.project_id) {
                    if p_target == p_key {
                        return true;
                    }
                }
                if let Some(ref fp) = k.file_path {
                    if files.iter().any(|f| f == fp) {
                        return true;
                    }
                }
                false
            })
            .cloned()
            .collect();

        for key in keys_to_remove {
            self.invalidate(&key);
        }
    }

    /// Flushes all cached graphs from memory.
    pub fn invalidate_all(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    /// Current number of cached subgraphs.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Indicates whether the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Maximum capacity of the cache.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Default for GraphCache {
    fn default() -> Self {
        Self::new(DEFAULT_CACHE_CAPACITY)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_lru_eviction() {
        let mut cache = GraphCache::new(2);

        let k1 = CacheKey::new(None, Some("a.rs".to_string()));
        let k2 = CacheKey::new(None, Some("b.rs".to_string()));
        let k3 = CacheKey::new(None, Some("c.rs".to_string()));

        cache.insert(k1.clone(), Arc::new(KnoxGraph::new()));
        cache.insert(k2.clone(), Arc::new(KnoxGraph::new()));
        assert_eq!(cache.len(), 2);

        // Accessing k1 makes k2 the least recently used
        let _ = cache.get(&k1);

        // Inserting k3 should evict k2
        cache.insert(k3.clone(), Arc::new(KnoxGraph::new()));
        assert_eq!(cache.len(), 2);
        assert!(cache.get(&k1).is_some());
        assert!(cache.get(&k2).is_none());
        assert!(cache.get(&k3).is_some());
    }

    #[test]
    fn test_cache_invalidation() {
        let mut cache = GraphCache::default();
        let key = CacheKey::new(None, Some("test.rs".to_string()));

        cache.insert(key.clone(), Arc::new(KnoxGraph::new()));
        assert_eq!(cache.len(), 1);

        cache.invalidate(&key);
        assert_eq!(cache.len(), 0);
        assert!(cache.get(&key).is_none());
    }

    #[test]
    fn test_cache_invalidate_write() {
        let mut cache = GraphCache::default();
        let global = CacheKey::global();
        let file_key = CacheKey::new(None, Some("src/main.rs".to_string()));
        let other_key = CacheKey::new(None, Some("src/other.rs".to_string()));

        cache.insert(global.clone(), Arc::new(KnoxGraph::new()));
        cache.insert(file_key.clone(), Arc::new(KnoxGraph::new()));
        cache.insert(other_key.clone(), Arc::new(KnoxGraph::new()));
        assert_eq!(cache.len(), 3);

        // Invalidate write touching src/main.rs
        cache.invalidate_write(None, &["src/main.rs".to_string()]);
        assert_eq!(cache.len(), 1);
        assert!(cache.get(&global).is_none());
        assert!(cache.get(&file_key).is_none());
        assert!(cache.get(&other_key).is_some());
    }
}
