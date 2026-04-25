use std::{env, fs};

use postgres::{Client, NoTls, Row, Transaction};
use postgres_native_tls::MakeTlsConnector;
use serde::de::DeserializeOwned;

use crate::storage::recovery::{reconcile_recovered_state, RecoverySummary};
use crate::storage::schema::{
    SchemaName, BATCH_WINDOWS_TABLE, BOOTSTRAP_SESSIONS_TABLE, BRIDGES_TABLE,
    BRIDGE_COMMANDS_TABLE, CATALOG_ISSUANCE_TABLE, INGESTED_FRAMES_TABLE, PROGRESS_EVENTS_TABLE,
    SEQUENCE_STATE_TABLE, UPLOAD_SESSIONS_TABLE,
};
use crate::storage::{
    BatchWindowState, BootstrapSessionRecord, BridgeCommandRecord, BridgeRecord,
    CatalogIssuanceRecord, InMemoryAuthorityStorage, IngestedFrameRecord, SequenceState,
    StorageError, StorageResult, UploadSessionRecord,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostgresStorageConfig {
    pub connection_string: String,
    pub schema: String,
}

impl PostgresStorageConfig {
    pub fn from_env() -> StorageResult<Option<Self>> {
        let Ok(connection_string) = env::var("GBN_BRIDGE_POSTGRES_URL") else {
            return Self::from_split_env();
        };

        let schema = env::var("GBN_BRIDGE_POSTGRES_SCHEMA")
            .unwrap_or_else(|_| "conduit_publisher".to_string());
        SchemaName::parse(&schema)?;

        Ok(Some(Self {
            connection_string,
            schema,
        }))
    }

    fn from_split_env() -> StorageResult<Option<Self>> {
        let Ok(host) = env::var("GBN_BRIDGE_POSTGRES_HOST") else {
            return Ok(None);
        };

        let port = env::var("GBN_BRIDGE_POSTGRES_PORT").unwrap_or_else(|_| "5432".to_string());
        let database = env::var("GBN_BRIDGE_POSTGRES_DATABASE")
            .unwrap_or_else(|_| "veritas_conduit".to_string());
        let user = env::var("GBN_BRIDGE_POSTGRES_USER").unwrap_or_else(|_| "veritas".to_string());
        let password = env::var("GBN_BRIDGE_POSTGRES_PASSWORD").map_err(|_| {
            StorageError::Config(
                "GBN_BRIDGE_POSTGRES_PASSWORD is required when GBN_BRIDGE_POSTGRES_HOST is set"
                    .into(),
            )
        })?;
        let sslmode =
            env::var("GBN_BRIDGE_POSTGRES_SSLMODE").unwrap_or_else(|_| "disable".to_string());
        let schema = env::var("GBN_BRIDGE_POSTGRES_SCHEMA")
            .unwrap_or_else(|_| "conduit_publisher".to_string());
        SchemaName::parse(&schema)?;

        let connection_string = format!(
            "host={host} port={port} dbname={database} user={user} password={password} sslmode={sslmode}"
        );
        Ok(Some(Self {
            connection_string,
            schema,
        }))
    }
}

pub struct PostgresAuthorityStorage {
    client: Client,
    schema: SchemaName,
}

impl std::fmt::Debug for PostgresAuthorityStorage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresAuthorityStorage")
            .field("schema", &self.schema)
            .finish_non_exhaustive()
    }
}

impl PostgresAuthorityStorage {
    pub fn connect(config: &PostgresStorageConfig) -> StorageResult<Self> {
        let schema = SchemaName::parse(&config.schema)?;
        let client = connect_postgres(&config.connection_string)?;
        let mut storage = Self { client, schema };
        storage.initialize_schema()?;
        Ok(storage)
    }

    pub fn load_state(
        &mut self,
        now_ms: u64,
    ) -> StorageResult<(InMemoryAuthorityStorage, RecoverySummary)> {
        let mut storage = InMemoryAuthorityStorage::default();

        for row in self.query_rows(BRIDGES_TABLE)? {
            let record: BridgeRecord = deserialize_json_value(row)?;
            storage.bridges.insert(record.bridge_id.clone(), record);
        }

        for row in self.query_rows(CATALOG_ISSUANCE_TABLE)? {
            let response = deserialize_json_from_column(&row, "response_json")?;
            let catalog = CatalogIssuanceRecord {
                catalog_id: row.get::<_, String>("catalog_id"),
                chain_id: row.get::<_, Option<String>>("chain_id"),
                issued_at_ms: from_i64(row.get::<_, i64>("issued_at_ms"))?,
                expires_at_ms: from_i64(row.get::<_, i64>("expires_at_ms"))?,
                response,
            };
            storage
                .catalog_issuance
                .insert(catalog.catalog_id.clone(), catalog);
        }

        for row in self.query_rows(BOOTSTRAP_SESSIONS_TABLE)? {
            let mut session: BootstrapSessionRecord =
                deserialize_json_from_column(&row, "record_json")?;
            session.progress_events = self.load_progress_events(&session.bootstrap_session_id)?;
            storage
                .bootstrap_sessions
                .insert(session.bootstrap_session_id.clone(), session);
        }

        for row in self.query_rows(BRIDGE_COMMANDS_TABLE)? {
            let record: BridgeCommandRecord = deserialize_json_from_column(&row, "record_json")?;
            storage
                .bridge_commands
                .insert(record.command_id.clone(), record);
        }

        for row in self.query_rows(UPLOAD_SESSIONS_TABLE)? {
            let mut session: UploadSessionRecord =
                deserialize_json_from_column(&row, "record_json")?;
            session.frames_by_sequence.clear();
            session.frame_id_to_sequence.clear();
            for frame_record in self.load_ingested_frames(&session.session_id)? {
                session.frame_id_to_sequence.insert(
                    frame_record.frame.frame_id.clone(),
                    frame_record.frame.sequence,
                );
                session
                    .frames_by_sequence
                    .insert(frame_record.frame.sequence, frame_record);
            }
            storage
                .upload_sessions
                .insert(session.session_id.clone(), session);
        }

        if let Some(row) = self.query_optional_row(BATCH_WINDOWS_TABLE)? {
            let batch: BatchWindowState = deserialize_json_from_column(&row, "record_json")?;
            storage.current_batch = Some(batch);
        }

        storage.apply_sequence_state(self.load_sequence_state()?);
        let recovery = reconcile_recovered_state(&mut storage, now_ms);

        if recovery != RecoverySummary::default() {
            self.persist_state(&storage)?;
        }

        Ok((storage, recovery))
    }

    pub fn persist_state(&mut self, storage: &InMemoryAuthorityStorage) -> StorageResult<()> {
        let bridges_table = self.schema.qualified(BRIDGES_TABLE);
        let catalog_table = self.schema.qualified(CATALOG_ISSUANCE_TABLE);
        let bootstrap_table = self.schema.qualified(BOOTSTRAP_SESSIONS_TABLE);
        let progress_table = self.schema.qualified(PROGRESS_EVENTS_TABLE);
        let commands_table = self.schema.qualified(BRIDGE_COMMANDS_TABLE);
        let upload_table = self.schema.qualified(UPLOAD_SESSIONS_TABLE);
        let frames_table = self.schema.qualified(INGESTED_FRAMES_TABLE);
        let batch_table = self.schema.qualified(BATCH_WINDOWS_TABLE);
        let sequence_table = self.schema.qualified(SEQUENCE_STATE_TABLE);
        let mut tx = self
            .client
            .transaction()
            .map_err(|error| StorageError::Backend(error.to_string()))?;

        clear_table(&mut tx, &progress_table)?;
        clear_table(&mut tx, &commands_table)?;
        clear_table(&mut tx, &frames_table)?;
        clear_table(&mut tx, &batch_table)?;
        clear_table(&mut tx, &bootstrap_table)?;
        clear_table(&mut tx, &upload_table)?;
        clear_table(&mut tx, &catalog_table)?;
        clear_table(&mut tx, &bridges_table)?;
        clear_table(&mut tx, &sequence_table)?;

        for record in storage.bridges.values() {
            let sql = format!(
                "INSERT INTO {} (bridge_id, reachability_class, udp_punch_port, lease_expiry_ms, revoked_reason, updated_at_ms, record_json) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                bridges_table
            );
            let record_json = serde_json::to_value(record)
                .map_err(|error| StorageError::Serialization(error.to_string()))?;
            tx.execute(
                &sql,
                &[
                    &record.bridge_id,
                    &format!("{:?}", record.reachability_class).to_lowercase(),
                    &i32::from(record.assigned_udp_punch_port),
                    &to_i64(record.current_lease.lease_expiry_ms)?,
                    &record
                        .revoked_reason
                        .as_ref()
                        .map(|reason| format!("{reason:?}").to_lowercase()),
                    &to_i64(record.last_heartbeat.heartbeat_at_ms)?,
                    &record_json,
                ],
            )
            .map_err(|error| StorageError::Backend(error.to_string()))?;
        }

        for issuance in storage.catalog_issuance.values() {
            let sql = format!(
                "INSERT INTO {} (catalog_id, chain_id, issued_at_ms, expires_at_ms, bridge_count, response_json) VALUES ($1, $2, $3, $4, $5, $6)",
                catalog_table
            );
            let response_json = serde_json::to_value(&issuance.response)
                .map_err(|error| StorageError::Serialization(error.to_string()))?;
            tx.execute(
                &sql,
                &[
                    &issuance.catalog_id,
                    &issuance.chain_id,
                    &to_i64(issuance.issued_at_ms)?,
                    &to_i64(issuance.expires_at_ms)?,
                    &(issuance.response.bridges.len() as i32),
                    &response_json,
                ],
            )
            .map_err(|error| StorageError::Backend(error.to_string()))?;
        }

        for session in storage.bootstrap_sessions.values() {
            let sql = format!(
                "INSERT INTO {} (bootstrap_session_id, chain_id, seed_bridge_id, created_at_ms, response_expiry_ms, record_json) VALUES ($1, $2, $3, $4, $5, $6)",
                bootstrap_table
            );
            let record_json = serde_json::to_value(session)
                .map_err(|error| StorageError::Serialization(error.to_string()))?;
            tx.execute(
                &sql,
                &[
                    &session.bootstrap_session_id,
                    &session.chain_id,
                    &session.seed_bridge_id,
                    &to_i64(session.created_at_ms)?,
                    &to_i64(session.response_expiry_ms)?,
                    &record_json,
                ],
            )
            .map_err(|error| StorageError::Backend(error.to_string()))?;

            let progress_sql = format!(
                "INSERT INTO {} (bootstrap_session_id, event_index, chain_id, reporter_id, stage, reported_at_ms, event_json) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                progress_table
            );
            for (event_index, event) in session.progress_events.iter().enumerate() {
                let event_json = serde_json::to_value(event)
                    .map_err(|error| StorageError::Serialization(error.to_string()))?;
                tx.execute(
                    &progress_sql,
                    &[
                        &session.bootstrap_session_id,
                        &(event_index as i32),
                        &session.chain_id,
                        &event.reporter_id,
                        &format!("{:?}", event.stage).to_lowercase(),
                        &to_i64(event.reported_at_ms)?,
                        &event_json,
                    ],
                )
                .map_err(|error| StorageError::Backend(error.to_string()))?;
            }
        }

        for command in storage.bridge_commands.values() {
            let sql = format!(
                "INSERT INTO {} (command_id, bridge_id, seq_no, chain_id, issued_at_ms, acked_at_ms, ack_status, sent_count, last_sent_at_ms, record_json) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
                commands_table
            );
            let record_json = serde_json::to_value(command)
                .map_err(|error| StorageError::Serialization(error.to_string()))?;
            tx.execute(
                &sql,
                &[
                    &command.command_id,
                    &command.bridge_id,
                    &to_i64(command.seq_no)?,
                    &command.chain_id,
                    &to_i64(command.issued_at_ms)?,
                    &option_to_i64(command.acked_at_ms)?,
                    &command
                        .ack_status
                        .map(|status| format!("{status:?}").to_lowercase()),
                    &(command.sent_count as i32),
                    &option_to_i64(command.last_sent_at_ms)?,
                    &record_json,
                ],
            )
            .map_err(|error| StorageError::Backend(error.to_string()))?;
        }

        for session in storage.upload_sessions.values() {
            let sql = format!(
                "INSERT INTO {} (session_id, chain_id, creator_id, opened_at_ms, closed_at_ms, completed_at_ms, record_json) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                upload_table
            );
            let record_json = serde_json::to_value(session)
                .map_err(|error| StorageError::Serialization(error.to_string()))?;
            tx.execute(
                &sql,
                &[
                    &session.session_id,
                    &session.chain_id,
                    &session.creator_id,
                    &to_i64(session.opened_at_ms)?,
                    &option_to_i64(session.closed_at_ms)?,
                    &option_to_i64(session.completed_at_ms)?,
                    &record_json,
                ],
            )
            .map_err(|error| StorageError::Backend(error.to_string()))?;

            let frame_sql = format!(
                "INSERT INTO {} (session_id, frame_id, sequence, via_bridge_id, chain_id, received_at_ms, frame_json) VALUES ($1, $2, $3, $4, $5, $6, $7)",
                frames_table
            );
            for frame in session.frames_by_sequence.values() {
                let frame_json = serde_json::to_value(&frame.frame)
                    .map_err(|error| StorageError::Serialization(error.to_string()))?;
                tx.execute(
                    &frame_sql,
                    &[
                        &session.session_id,
                        &frame.frame.frame_id,
                        &(frame.frame.sequence as i32),
                        &frame.via_bridge_id,
                        &frame.chain_id,
                        &to_i64(frame.received_at_ms)?,
                        &frame_json,
                    ],
                )
                .map_err(|error| StorageError::Backend(error.to_string()))?;
            }
        }

        if let Some(batch) = &storage.current_batch {
            let sql = format!(
                "INSERT INTO {} (batch_id, window_started_at_ms, record_json) VALUES ($1, $2, $3)",
                batch_table
            );
            let record_json = serde_json::to_value(batch)
                .map_err(|error| StorageError::Serialization(error.to_string()))?;
            tx.execute(
                &sql,
                &[
                    &batch.batch_id,
                    &to_i64(batch.window_started_at_ms)?,
                    &record_json,
                ],
            )
            .map_err(|error| StorageError::Backend(error.to_string()))?;
        }

        persist_sequences(&mut tx, &sequence_table, storage.sequence_state())?;

        tx.commit()
            .map_err(|error| StorageError::Backend(error.to_string()))
    }

    pub fn is_healthy(&mut self) -> StorageResult<()> {
        self.client
            .simple_query("SELECT 1")
            .map_err(|error| StorageError::Backend(error.to_string()))?;
        Ok(())
    }

    fn initialize_schema(&mut self) -> StorageResult<()> {
        let schema_name = self.schema.as_str();
        self.client
            .batch_execute(&format!("CREATE SCHEMA IF NOT EXISTS \"{}\";", schema_name))
            .map_err(|error| StorageError::Backend(error.to_string()))?;

        for statement in self.schema_statements() {
            self.client
                .batch_execute(&statement)
                .map_err(|error| StorageError::Backend(error.to_string()))?;
        }

        Ok(())
    }

    fn schema_statements(&self) -> Vec<String> {
        vec![
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    bridge_id TEXT PRIMARY KEY,
                    reachability_class TEXT NOT NULL,
                    udp_punch_port INTEGER NOT NULL,
                    lease_expiry_ms BIGINT NOT NULL,
                    revoked_reason TEXT NULL,
                    updated_at_ms BIGINT NOT NULL,
                    record_json JSONB NOT NULL
                );",
                self.schema.qualified(BRIDGES_TABLE)
            ),
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    catalog_id TEXT PRIMARY KEY,
                    chain_id TEXT NULL,
                    issued_at_ms BIGINT NOT NULL,
                    expires_at_ms BIGINT NOT NULL,
                    bridge_count INTEGER NOT NULL,
                    response_json JSONB NOT NULL
                );",
                self.schema.qualified(CATALOG_ISSUANCE_TABLE)
            ),
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    bootstrap_session_id TEXT PRIMARY KEY,
                    chain_id TEXT NOT NULL,
                    seed_bridge_id TEXT NOT NULL,
                    created_at_ms BIGINT NOT NULL,
                    response_expiry_ms BIGINT NOT NULL,
                    record_json JSONB NOT NULL
                );",
                self.schema.qualified(BOOTSTRAP_SESSIONS_TABLE)
            ),
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    command_id TEXT PRIMARY KEY,
                    bridge_id TEXT NOT NULL,
                    seq_no BIGINT NOT NULL,
                    chain_id TEXT NOT NULL,
                    issued_at_ms BIGINT NOT NULL,
                    acked_at_ms BIGINT NULL,
                    ack_status TEXT NULL,
                    sent_count INTEGER NOT NULL,
                    last_sent_at_ms BIGINT NULL,
                    record_json JSONB NOT NULL
                );",
                self.schema.qualified(BRIDGE_COMMANDS_TABLE)
            ),
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    bootstrap_session_id TEXT NOT NULL,
                    event_index INTEGER NOT NULL,
                    chain_id TEXT NOT NULL,
                    reporter_id TEXT NOT NULL,
                    stage TEXT NOT NULL,
                    reported_at_ms BIGINT NOT NULL,
                    event_json JSONB NOT NULL,
                    PRIMARY KEY (bootstrap_session_id, event_index)
                );",
                self.schema.qualified(PROGRESS_EVENTS_TABLE)
            ),
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    session_id TEXT PRIMARY KEY,
                    chain_id TEXT NULL,
                    creator_id TEXT NOT NULL,
                    opened_at_ms BIGINT NOT NULL,
                    closed_at_ms BIGINT NULL,
                    completed_at_ms BIGINT NULL,
                    record_json JSONB NOT NULL
                );",
                self.schema.qualified(UPLOAD_SESSIONS_TABLE)
            ),
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    session_id TEXT NOT NULL,
                    frame_id TEXT NOT NULL,
                    sequence INTEGER NOT NULL,
                    via_bridge_id TEXT NOT NULL,
                    chain_id TEXT NULL,
                    received_at_ms BIGINT NOT NULL,
                    frame_json JSONB NOT NULL,
                    PRIMARY KEY (session_id, frame_id)
                );",
                self.schema.qualified(INGESTED_FRAMES_TABLE)
            ),
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    batch_id TEXT PRIMARY KEY,
                    window_started_at_ms BIGINT NOT NULL,
                    record_json JSONB NOT NULL
                );",
                self.schema.qualified(BATCH_WINDOWS_TABLE)
            ),
            format!(
                "CREATE TABLE IF NOT EXISTS {} (
                    sequence_name TEXT PRIMARY KEY,
                    next_value BIGINT NOT NULL
                );",
                self.schema.qualified(SEQUENCE_STATE_TABLE)
            ),
        ]
    }

    fn query_rows(&mut self, table: &str) -> StorageResult<Vec<Row>> {
        let sql = format!(
            "SELECT * FROM {} ORDER BY 1 ASC",
            self.schema.qualified(table)
        );
        self.client
            .query(&sql, &[])
            .map_err(|error| StorageError::Backend(error.to_string()))
    }

    fn query_optional_row(&mut self, table: &str) -> StorageResult<Option<Row>> {
        let sql = format!("SELECT * FROM {} LIMIT 1", self.schema.qualified(table));
        self.client
            .query_opt(&sql, &[])
            .map_err(|error| StorageError::Backend(error.to_string()))
    }

    fn load_progress_events(
        &mut self,
        bootstrap_session_id: &str,
    ) -> StorageResult<Vec<gbn_bridge_protocol::BootstrapProgress>> {
        let sql = format!(
            "SELECT event_json FROM {} WHERE bootstrap_session_id = $1 ORDER BY event_index ASC",
            self.schema.qualified(PROGRESS_EVENTS_TABLE)
        );
        let rows = self
            .client
            .query(&sql, &[&bootstrap_session_id])
            .map_err(|error| StorageError::Backend(error.to_string()))?;
        rows.into_iter()
            .map(|row| deserialize_json_from_column(&row, "event_json"))
            .collect()
    }

    fn load_ingested_frames(
        &mut self,
        session_id: &str,
    ) -> StorageResult<Vec<IngestedFrameRecord>> {
        let sql = format!(
            "SELECT frame_json, via_bridge_id, chain_id, received_at_ms FROM {} WHERE session_id = $1 ORDER BY sequence ASC",
            self.schema.qualified(INGESTED_FRAMES_TABLE)
        );
        let rows = self
            .client
            .query(&sql, &[&session_id])
            .map_err(|error| StorageError::Backend(error.to_string()))?;
        rows.into_iter()
            .map(|row| {
                let frame = deserialize_json_from_column::<gbn_bridge_protocol::BridgeData>(
                    &row,
                    "frame_json",
                )?;
                Ok(IngestedFrameRecord {
                    via_bridge_id: row.get::<_, String>("via_bridge_id"),
                    chain_id: row.get::<_, Option<String>>("chain_id"),
                    frame,
                    received_at_ms: from_i64(row.get::<_, i64>("received_at_ms"))?,
                })
            })
            .collect()
    }

    fn load_sequence_state(&mut self) -> StorageResult<SequenceState> {
        let sql = format!(
            "SELECT sequence_name, next_value FROM {}",
            self.schema.qualified(SEQUENCE_STATE_TABLE)
        );
        let rows = self
            .client
            .query(&sql, &[])
            .map_err(|error| StorageError::Backend(error.to_string()))?;

        let mut state = SequenceState::default();
        for row in rows {
            let sequence_name = row.get::<_, String>("sequence_name");
            let next_value = from_i64(row.get::<_, i64>("next_value"))?;
            match sequence_name.as_str() {
                "lease" => state.next_lease_seq = next_value,
                "catalog" => state.next_catalog_seq = next_value,
                "bootstrap" => state.next_bootstrap_seq = next_value,
                "batch" => state.next_batch_seq = next_value,
                _ => {}
            }
        }

        Ok(state)
    }
}

fn connect_postgres(connection_string: &str) -> StorageResult<Client> {
    if postgres_connection_requires_tls(connection_string) {
        let mut builder = native_tls::TlsConnector::builder();
        if postgres_tls_accept_invalid_certs() {
            builder.danger_accept_invalid_certs(true);
        }
        if let Some(certificate) = postgres_tls_root_certificate()? {
            builder.add_root_certificate(certificate);
        }
        let connector = builder
            .build()
            .map_err(|error| StorageError::Backend(error.to_string()))?;
        return Client::connect(connection_string, MakeTlsConnector::new(connector))
            .map_err(|error| StorageError::Backend(format!("{error:?}")));
    }

    Client::connect(connection_string, NoTls)
        .map_err(|error| StorageError::Backend(format!("{error:?}")))
}

fn postgres_connection_requires_tls(connection_string: &str) -> bool {
    connection_string
        .split_whitespace()
        .find_map(|part| part.strip_prefix("sslmode="))
        .map(|sslmode| {
            matches!(
                sslmode.to_ascii_lowercase().as_str(),
                "require" | "verify-ca" | "verify-full"
            )
        })
        .unwrap_or(false)
}

fn postgres_tls_accept_invalid_certs() -> bool {
    env::var("GBN_BRIDGE_POSTGRES_TLS_ACCEPT_INVALID_CERTS")
        .map(|value| matches!(value.to_ascii_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false)
}

fn postgres_tls_root_certificate() -> StorageResult<Option<native_tls::Certificate>> {
    if let Ok(pem) = env::var("GBN_BRIDGE_POSTGRES_TLS_CA_PEM") {
        return native_tls::Certificate::from_pem(pem.as_bytes())
            .map(Some)
            .map_err(|error| StorageError::Config(error.to_string()));
    }

    let Ok(path) = env::var("GBN_BRIDGE_POSTGRES_TLS_CA_FILE") else {
        return Ok(None);
    };
    let pem = fs::read(&path).map_err(|error| {
        StorageError::Config(format!(
            "failed to read postgres TLS CA file {path:?}: {error}"
        ))
    })?;
    native_tls::Certificate::from_pem(&pem)
        .map(Some)
        .map_err(|error| StorageError::Config(error.to_string()))
}

fn persist_sequences(
    tx: &mut Transaction<'_>,
    table_name: &str,
    state: SequenceState,
) -> StorageResult<()> {
    let sql = format!(
        "INSERT INTO {} (sequence_name, next_value) VALUES ($1, $2)",
        table_name
    );
    for (name, next_value) in [
        ("lease", state.next_lease_seq),
        ("catalog", state.next_catalog_seq),
        ("bootstrap", state.next_bootstrap_seq),
        ("batch", state.next_batch_seq),
    ] {
        tx.execute(&sql, &[&name, &to_i64(next_value)?])
            .map_err(|error| StorageError::Backend(error.to_string()))?;
    }
    Ok(())
}

fn deserialize_json_value<T>(row: Row) -> StorageResult<T>
where
    T: DeserializeOwned,
{
    deserialize_json_from_column(&row, "record_json")
}

fn deserialize_json_from_column<T>(row: &Row, column: &str) -> StorageResult<T>
where
    T: DeserializeOwned,
{
    let value = row.get::<_, serde_json::Value>(column);
    serde_json::from_value(value).map_err(|error| StorageError::Serialization(error.to_string()))
}

fn to_i64(value: u64) -> StorageResult<i64> {
    i64::try_from(value).map_err(|_| {
        StorageError::Serialization(format!("value {value} exceeds postgres BIGINT range"))
    })
}

fn from_i64(value: i64) -> StorageResult<u64> {
    u64::try_from(value).map_err(|_| {
        StorageError::Serialization(format!(
            "value {value} is negative and cannot be converted to u64"
        ))
    })
}

fn option_to_i64(value: Option<u64>) -> StorageResult<Option<i64>> {
    value.map(to_i64).transpose()
}

fn clear_table(tx: &mut Transaction<'_>, table_name: &str) -> StorageResult<()> {
    let sql = format!("DELETE FROM {table_name}");
    tx.execute(&sql, &[])
        .map_err(|error| StorageError::Backend(error.to_string()))?;
    Ok(())
}
