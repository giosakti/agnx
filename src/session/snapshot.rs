//! Session snapshot schema for fast resume.
//!
//! Snapshots are written as YAML files and contain the complete session state
//! at a point in time. Combined with the event log, they enable fast resume
//! without replaying the entire event history.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::llm::Message;

/// A snapshot of session state for fast resume.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    /// Schema version for forward compatibility.
    pub schema_version: String,
    /// The session ID.
    pub session_id: String,
    /// The agent this session is using.
    pub agent: String,
    /// Current session status.
    pub status: SnapshotStatus,
    /// When the session was created.
    pub created_at: DateTime<Utc>,
    /// When this snapshot was taken.
    pub snapshot_at: DateTime<Utc>,
    /// The sequence number of the last event included in this snapshot.
    pub last_event_seq: u64,
    /// The conversation history.
    pub conversation: Vec<Message>,
    /// Session configuration.
    pub config: SessionConfig,
}

/// Session status in a snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SnapshotStatus {
    /// Session is active and ready for messages.
    Active,
    /// Session is paused (client disconnected with on_disconnect: pause).
    Paused,
    /// Session is running in background (client disconnected with on_disconnect: continue).
    Running,
    /// Session has completed.
    Completed,
}

/// Session configuration stored in the snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionConfig {
    /// Behavior when client disconnects.
    #[serde(default)]
    pub on_disconnect: OnDisconnect,
}

/// Behavior when client disconnects from a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnDisconnect {
    /// Pause the session and wait for reconnect (default).
    #[default]
    Pause,
    /// Continue executing in the background.
    Continue,
}

impl SessionSnapshot {
    /// Current schema version.
    pub const SCHEMA_VERSION: &'static str = "1";

    /// Create a new snapshot from session state.
    pub fn new(
        session_id: String,
        agent: String,
        status: SnapshotStatus,
        created_at: DateTime<Utc>,
        last_event_seq: u64,
        conversation: Vec<Message>,
        config: SessionConfig,
    ) -> Self {
        Self {
            schema_version: Self::SCHEMA_VERSION.to_string(),
            session_id,
            agent,
            status,
            created_at,
            snapshot_at: Utc::now(),
            last_event_seq,
            conversation,
            config,
        }
    }

    /// Check if this snapshot is compatible with the current schema.
    pub fn is_compatible(&self) -> bool {
        self.schema_version == Self::SCHEMA_VERSION
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::Role;

    #[test]
    fn snapshot_serialization_roundtrip() {
        let snapshot = SessionSnapshot::new(
            "session_abc123".to_string(),
            "my-agent".to_string(),
            SnapshotStatus::Active,
            Utc::now(),
            42,
            vec![
                Message {
                    role: Role::User,
                    content: "Hello".to_string(),
                },
                Message {
                    role: Role::Assistant,
                    content: "Hi there!".to_string(),
                },
            ],
            SessionConfig::default(),
        );

        let yaml = serde_saphyr::to_string(&snapshot).unwrap();
        assert!(yaml.contains("session_id: session_abc123"));
        assert!(yaml.contains("status: active"));
        assert!(yaml.contains("last_event_seq: 42"));

        let parsed: SessionSnapshot = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(parsed.session_id, "session_abc123");
        assert_eq!(parsed.agent, "my-agent");
        assert_eq!(parsed.status, SnapshotStatus::Active);
        assert_eq!(parsed.last_event_seq, 42);
        assert_eq!(parsed.conversation.len(), 2);
    }

    #[test]
    fn snapshot_with_continue_mode() {
        let snapshot = SessionSnapshot::new(
            "session_xyz".to_string(),
            "background-agent".to_string(),
            SnapshotStatus::Running,
            Utc::now(),
            100,
            vec![],
            SessionConfig {
                on_disconnect: OnDisconnect::Continue,
            },
        );

        let yaml = serde_saphyr::to_string(&snapshot).unwrap();
        assert!(yaml.contains("on_disconnect: continue"));

        let parsed: SessionSnapshot = serde_saphyr::from_str(&yaml).unwrap();
        assert_eq!(parsed.config.on_disconnect, OnDisconnect::Continue);
    }

    #[test]
    fn snapshot_status_values() {
        assert_eq!(
            serde_json::to_string(&SnapshotStatus::Active).unwrap(),
            "\"active\""
        );
        assert_eq!(
            serde_json::to_string(&SnapshotStatus::Paused).unwrap(),
            "\"paused\""
        );
        assert_eq!(
            serde_json::to_string(&SnapshotStatus::Running).unwrap(),
            "\"running\""
        );
        assert_eq!(
            serde_json::to_string(&SnapshotStatus::Completed).unwrap(),
            "\"completed\""
        );
    }

    #[test]
    fn on_disconnect_default() {
        let config = SessionConfig::default();
        assert_eq!(config.on_disconnect, OnDisconnect::Pause);
    }

    #[test]
    fn schema_version_check() {
        let snapshot = SessionSnapshot::new(
            "s".to_string(),
            "a".to_string(),
            SnapshotStatus::Active,
            Utc::now(),
            0,
            vec![],
            SessionConfig::default(),
        );
        assert!(snapshot.is_compatible());

        let mut old_snapshot = snapshot.clone();
        old_snapshot.schema_version = "0".to_string();
        assert!(!old_snapshot.is_compatible());
    }
}
