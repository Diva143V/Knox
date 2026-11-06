//! In-memory graph traversal, subgraph extraction, and caching for Knox.

pub mod cache;
pub mod loader;
pub mod traversal;

pub use cache::{CacheKey, DEFAULT_CACHE_CAPACITY, GraphCache};
pub use loader::{
    BoundedSubgraph, DEFAULT_MAX_NODES, KnoxGraph, SubgraphDirection, build_node_index,
    find_node_index, load_full_graph, load_subgraph, load_subgraph_bounded,
    load_subgraph_directional,
};
pub use traversal::{ancestors, causal_chain, descendants, path_between};
