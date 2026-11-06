//! SQLite vector embedding storage and similarity search (Workstream H).

use chrono::Utc;
use rusqlite::params;

use crate::core::error::KnoxResult;
use crate::core::ids::ActionId;
use crate::storage::db::KnoxDb;

/// Converts a slice of f32 vector coordinates into a little-endian byte buffer.
pub fn vector_to_blob(vector: &[f32]) -> Vec<u8> {
    let mut blob = Vec::with_capacity(vector.len() * 4);
    for val in vector {
        blob.extend_from_slice(&val.to_le_bytes());
    }
    blob
}

/// Converts a little-endian byte buffer back into a Vec<f32>.
pub fn blob_to_vector(blob: &[u8]) -> Vec<f32> {
    blob.chunks_exact(4)
        .map(|chunk| f32::from_le_bytes(chunk.try_into().unwrap_or_default()))
        .collect()
}

/// Computes the cosine similarity between two float vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f32;
    let mut norm_a = 0.0f32;
    let mut norm_b = 0.0f32;

    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        norm_a += x * x;
        norm_b += y * y;
    }

    let denom = norm_a.sqrt() * norm_b.sqrt();
    if denom == 0.0 {
        0.0
    } else {
        dot / denom
    }
}

/// Inserts or replaces a vector embedding for an action using an existing connection.
pub fn insert_embedding_conn(
    conn: &rusqlite::Connection,
    action_id: &ActionId,
    vector: &[f32],
    model: &str,
) -> KnoxResult<()> {
    let blob = vector_to_blob(vector);
    let now = Utc::now().to_rfc3339();

    conn.execute(
        "INSERT INTO embeddings (action_id, vector, dimension, model, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)
         ON CONFLICT(action_id) DO UPDATE SET
             vector = excluded.vector,
             dimension = excluded.dimension,
             model = excluded.model,
             created_at = excluded.created_at",
        params![
            action_id.as_str(),
            blob,
            vector.len() as i64,
            model,
            now
        ],
    )?;
    Ok(())
}

/// Inserts or replaces a vector embedding for an action.
pub fn insert_embedding(
    db: &KnoxDb,
    action_id: &ActionId,
    vector: &[f32],
    model: &str,
) -> KnoxResult<()> {
    insert_embedding_conn(db.conn(), action_id, vector, model)
}

/// Retrieves the float vector embedding for an action, if present.
pub fn get_embedding(db: &KnoxDb, action_id: &ActionId) -> KnoxResult<Option<Vec<f32>>> {
    let mut stmt = db.conn().prepare("SELECT vector FROM embeddings WHERE action_id = ?1")?;
    let mut rows = stmt.query(params![action_id.as_str()])?;

    if let Some(row) = rows.next()? {
        let blob: Vec<u8> = row.get(0)?;
        Ok(Some(blob_to_vector(&blob)))
    } else {
        Ok(None)
    }
}

/// Retrieves multiple embeddings in chunks for high-throughput batch operations.
pub fn get_embeddings_batch(
    db: &KnoxDb,
    action_ids: &[ActionId],
) -> KnoxResult<std::collections::HashMap<ActionId, Vec<f32>>> {
    let mut map = std::collections::HashMap::new();
    if action_ids.is_empty() {
        return Ok(map);
    }
    for chunk in action_ids.chunks(50) {
        let placeholders = chunk.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!("SELECT action_id, vector FROM embeddings WHERE action_id IN ({placeholders})");
        let mut stmt = db.conn().prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(chunk.iter().map(|id| id.as_str())), |row| {
            let id_str: String = row.get(0)?;
            let blob: Vec<u8> = row.get(1)?;
            Ok((ActionId::from(id_str), blob_to_vector(&blob)))
        })?;
        for row in rows {
            let (id, vec) = row?;
            map.insert(id, vec);
        }
    }
    Ok(map)
}

/// Scans stored embeddings and returns the top matching action IDs ordered by cosine similarity.
pub fn find_similar_actions(
    db: &KnoxDb,
    target_vector: &[f32],
    limit: usize,
    min_similarity: f32,
) -> KnoxResult<Vec<(ActionId, f32)>> {
    let mut stmt = db.conn().prepare("SELECT action_id, vector FROM embeddings")?;
    let rows = stmt.query_map([], |row| {
        let id_str: String = row.get(0)?;
        let blob: Vec<u8> = row.get(1)?;
        Ok((id_str, blob))
    })?;

    let mut matches: Vec<(ActionId, f32)> = Vec::new();
    for row in rows {
        let (id_str, blob) = row?;
        let cand_vec = blob_to_vector(&blob);
        let sim = cosine_similarity(target_vector, &cand_vec);
        if sim >= min_similarity {
            matches.push((ActionId::from(id_str), sim));
        }
    }

    matches.sort_by(|a, b| b.1.total_cmp(&a.1));
    matches.truncate(limit);
    Ok(matches)
}

