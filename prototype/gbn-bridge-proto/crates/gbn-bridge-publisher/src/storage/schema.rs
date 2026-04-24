use crate::storage::{StorageError, StorageResult};

pub const BRIDGES_TABLE: &str = "conduit_bridges";
pub const CATALOG_ISSUANCE_TABLE: &str = "conduit_catalog_issuance";
pub const BOOTSTRAP_SESSIONS_TABLE: &str = "conduit_bootstrap_sessions";
pub const BRIDGE_COMMANDS_TABLE: &str = "conduit_bridge_commands";
pub const UPLOAD_SESSIONS_TABLE: &str = "conduit_upload_sessions";
pub const INGESTED_FRAMES_TABLE: &str = "conduit_ingested_frames";
pub const BATCH_WINDOWS_TABLE: &str = "conduit_batch_windows";
pub const PROGRESS_EVENTS_TABLE: &str = "conduit_progress_events";
pub const SEQUENCE_STATE_TABLE: &str = "conduit_sequence_state";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaName(String);

impl SchemaName {
    pub fn parse(value: &str) -> StorageResult<Self> {
        if value.is_empty() {
            return Err(StorageError::Config(
                "postgres schema name must be non-empty".into(),
            ));
        }

        if !value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        {
            return Err(StorageError::Config(format!(
                "postgres schema name must contain only ascii alphanumeric characters or underscores, got {value:?}"
            )));
        }

        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn qualified(&self, table: &str) -> String {
        format!("\"{}\".\"{}\"", self.0, table)
    }
}
