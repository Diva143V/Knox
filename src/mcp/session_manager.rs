//! In-memory tracking and background timeout eviction for active Knox sessions.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::{Duration, Instant};

use crate::core::error::KnoxResult;
use crate::core::ids::SessionId;
use crate::storage::db::KnoxDb;

/// Tracks in-memory runtime metadata and recent activity for a single session.
#[derive(Debug, Clone)]
pub struct SessionState {
    /// Unique identifier of the session.
    pub session_id: SessionId,

    /// Agent or software actor operating within this session.
    pub actor: String,

    /// Optional project scope associated with the session.
    pub project: Option<String>,

    /// Monotonic timestamp of the most recent recorded activity.
    pub last_activity: Instant,
}

impl SessionState {
    /// Creates a new active session state initialized to the current instant.
    pub fn new(session_id: SessionId, actor: String, project: Option<String>) -> Self {
        Self {
            session_id,
            actor,
            project,
            last_activity: Instant::now(),
        }
    }

    /// Returns how long this session has been idle.
    pub fn idle_duration(&self) -> Duration {
        self.last_activity.elapsed()
    }
}

/// In-memory tracker of active sessions with thread-safe access.
#[derive(Debug, Default)]
pub struct SessionManager {
    sessions: RwLock<HashMap<SessionId, SessionState>>,
}

impl SessionManager {
    /// Creates a new empty session manager.
    pub fn new() -> Self {
        Self {
            sessions: RwLock::new(HashMap::new()),
        }
    }

    /// Registers a newly opened session in the in-memory tracker.
    pub fn register(&self, session_id: SessionId, actor: String, project: Option<String>) {
        let state = SessionState::new(session_id.clone(), actor, project);
        if let Ok(mut map) = self.sessions.write() {
            map.insert(session_id, state);
        }
    }

    /// Updates the `last_activity` timestamp for the specified session.
    /// Returns `true` if the session was found and updated, `false` otherwise.
    pub fn touch(&self, session_id: &SessionId) -> bool {
        if let Ok(mut map) = self.sessions.write() {
            if let Some(state) = map.get_mut(session_id) {
                state.last_activity = Instant::now();
                return true;
            }
        }
        false
    }

    /// Removes a session from the in-memory tracker (e.g. upon explicit closure).
    pub fn remove(&self, session_id: &SessionId) -> Option<SessionState> {
        self.sessions.write().ok()?.remove(session_id)
    }

    /// Retrieves a clone of the session state if currently active.
    pub fn get(&self, session_id: &SessionId) -> Option<SessionState> {
        self.sessions.read().ok()?.get(session_id).cloned()
    }

    /// Returns `true` if the session is currently registered in memory.
    pub fn is_active(&self, session_id: &SessionId) -> bool {
        self.sessions
            .read()
            .is_ok_and(|map| map.contains_key(session_id))
    }

    /// Returns the number of currently active in-memory sessions.
    pub fn active_count(&self) -> usize {
        self.sessions.read().map(|m| m.len()).unwrap_or(0)
    }

    /// Returns all currently registered active session IDs.
    pub fn active_session_ids(&self) -> Vec<SessionId> {
        self.sessions
            .read()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Identifies and purges in-memory sessions idle for longer than `max_idle`.
    pub fn check_stale_sessions(&self, max_idle: Duration) -> Vec<SessionId> {
        let mut expired = Vec::new();
        if let Ok(mut map) = self.sessions.write() {
            map.retain(|id, state| {
                if state.last_activity.elapsed() >= max_idle {
                    expired.push(id.clone());
                    false
                } else {
                    true
                }
            });
        }
        expired
    }

    /// Runs a synchronous purge of stale sessions both from memory and SQLite.
    pub fn timeout_stale_now(&self, db: &KnoxDb, max_idle: Duration) -> KnoxResult<Vec<SessionId>> {
        let _in_memory_purged = self.check_stale_sessions(max_idle);
        crate::storage::sessions::timeout_stale_sessions(db, max_idle)
    }

    /// Spawns a background Tokio task that periodically sweeps stale sessions.
    pub fn spawn_timeout_task(
        manager: Arc<Self>,
        db: Arc<Mutex<KnoxDb>>,
        check_interval: Duration,
        max_idle: Duration,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(check_interval);
            loop {
                ticker.tick().await;

                let memory_stale = manager.check_stale_sessions(max_idle);
                if !memory_stale.is_empty() {
                    tracing::info!(
                        stale_count = memory_stale.len(),
                        "Evicted idle sessions from memory tracker"
                    );
                }

                let db_result = {
                    let db_guard = match db.lock() {
                        Ok(g) => g,
                        Err(poisoned) => poisoned.into_inner(),
                    };
                    crate::storage::sessions::timeout_stale_sessions(&db_guard, max_idle)
                };

                match db_result {
                    Ok(closed) if !closed.is_empty() => {
                        tracing::info!(
                            closed_count = closed.len(),
                            "Closed timed-out sessions in database"
                        );
                    }
                    Err(err) => {
                        tracing::warn!(error = %err, "Failed timing out stale sessions in database");
                    }
                    _ => {}
                }
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_manager_register_touch_and_remove() {
        let manager = SessionManager::new();
        let sid = SessionId::new();

        assert_eq!(manager.active_count(), 0);
        assert!(!manager.is_active(&sid));

        manager.register(sid.clone(), "agent-1".to_string(), Some("knox".to_string()));
        assert_eq!(manager.active_count(), 1);
        assert!(manager.is_active(&sid));

        let state = manager.get(&sid).expect("must find session");
        assert_eq!(state.actor, "agent-1");
        assert_eq!(state.project.as_deref(), Some("knox"));

        // Touch updates activity
        let touched = manager.touch(&sid);
        assert!(touched);

        let untracked = SessionId::new();
        assert!(!manager.touch(&untracked));

        // Remove
        let removed = manager.remove(&sid);
        assert!(removed.is_some());
        assert_eq!(manager.active_count(), 0);
        assert!(!manager.is_active(&sid));
    }

    #[test]
    fn test_session_manager_stale_detection() {
        let manager = SessionManager::new();
        let sid1 = SessionId::new();
        let sid2 = SessionId::new();

        manager.register(sid1.clone(), "agent-1".to_string(), None);
        manager.register(sid2.clone(), "agent-2".to_string(), None);

        // Immediate check with 1-hour max_idle should not evict anything
        let evicted = manager.check_stale_sessions(Duration::from_secs(3600));
        assert!(evicted.is_empty());
        assert_eq!(manager.active_count(), 2);

        // Check with Duration::ZERO should evict all
        let evicted_all = manager.check_stale_sessions(Duration::ZERO);
        assert_eq!(evicted_all.len(), 2);
        assert_eq!(manager.active_count(), 0);
    }

    #[tokio::test]
    async fn test_session_manager_timeout_task_spawns_and_cancels() {
        let manager = Arc::new(SessionManager::new());
        let db = Arc::new(Mutex::new(KnoxDb::open_memory().unwrap()));
        let sid = SessionId::new();

        manager.register(sid.clone(), "agent-tokio".to_string(), None);

        let handle = SessionManager::spawn_timeout_task(
            manager.clone(),
            db,
            Duration::from_millis(50),
            Duration::from_secs(1800),
        );

        tokio::time::sleep(Duration::from_millis(70)).await;
        assert_eq!(manager.active_count(), 1);

        handle.abort();
        assert!(handle.await.unwrap_err().is_cancelled());
    }
}
