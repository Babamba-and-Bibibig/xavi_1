//! SQLite-backed development trace store.

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};
use xavi_application::ports::development_cycle_alias_store::{
    DevelopmentCycleAliasStore, DevelopmentCycleAliasStoreResult,
};
use xavi_application::ports::development_trace_store::{
    DevelopmentTraceStore, DevelopmentTraceStoreResult,
};
use xavi_domain::development_cycle::{
    DevelopmentCycleAlias, DevelopmentCycleAliasParts, DevelopmentCycleAliasRequest,
    DevelopmentCycleAliasReservation, format_development_cycle_alias,
    validate_development_cycle_alias,
};
use xavi_domain::development_trace::{
    DevelopmentTraceEntry, DevelopmentTraceFilter, DevelopmentTraceKind, NewDevelopmentTraceEntry,
    canonical_development_trace_columns,
};

/// `SQLite` implementation of the append-only development trace ledger.
pub struct SqliteDevelopmentTraceStore {
    connection: Mutex<Connection>,
}

impl SqliteDevelopmentTraceStore {
    /// Opens a `SQLite` database file and initializes the trace schema.
    ///
    /// # Errors
    ///
    /// Returns an error when the database path cannot be created, opened, or migrated.
    pub fn open(path: impl AsRef<Path>) -> DevelopmentTraceStoreResult<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)?;
        }

        let connection = Connection::open(path)?;
        let store = Self { connection: Mutex::new(connection) };
        store.init_schema()?;
        Ok(store)
    }

    /// Opens an in-memory `SQLite` database and initializes the trace schema.
    ///
    /// # Errors
    ///
    /// Returns an error when the in-memory connection or schema cannot be initialized.
    pub fn open_in_memory() -> DevelopmentTraceStoreResult<Self> {
        let connection = Connection::open_in_memory()?;
        let store = Self { connection: Mutex::new(connection) };
        store.init_schema()?;
        Ok(store)
    }

    fn connection(&self) -> DevelopmentTraceStoreResult<MutexGuard<'_, Connection>> {
        self.connection
            .lock()
            .map_err(|error| format!("sqlite development trace lock poisoned: {error}").into())
    }

    fn init_schema(&self) -> DevelopmentTraceStoreResult<()> {
        let connection = self.connection()?;
        connection.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS development_trace_entries (
                id INTEGER PRIMARY KEY,
                event_id TEXT NOT NULL UNIQUE,
                cycle_id TEXT NOT NULL,
                user_turn_id TEXT,
                kind TEXT NOT NULL,
                role_name TEXT,
                summary TEXT NOT NULL,
                body TEXT NOT NULL,
                metadata_json TEXT NOT NULL,
                schema_version INTEGER,
                sequence_no INTEGER,
                phase TEXT,
                parent_event_id TEXT,
                content_json TEXT,
                content_hash TEXT,
                source_kind TEXT,
                source_event_id TEXT,
                created_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS development_trace_entries_cycle_id_id_idx
                ON development_trace_entries (cycle_id, id);

            CREATE INDEX IF NOT EXISTS development_trace_entries_kind_id_idx
                ON development_trace_entries (kind, id);

            CREATE TABLE IF NOT EXISTS development_cycle_aliases (
                cycle_id TEXT PRIMARY KEY,
                cycle_alias TEXT NOT NULL UNIQUE,
                cycle_category TEXT NOT NULL,
                cycle_category_key TEXT NOT NULL,
                cycle_sequence INTEGER NOT NULL,
                cycle_title TEXT,
                created_at TEXT NOT NULL,
                UNIQUE(cycle_category_key, cycle_sequence)
            );

            CREATE INDEX IF NOT EXISTS development_cycle_aliases_category_sequence_idx
                ON development_cycle_aliases (cycle_category_key, cycle_sequence);
            ",
        )?;
        ensure_column(&connection, "schema_version", "INTEGER")?;
        ensure_column(&connection, "sequence_no", "INTEGER")?;
        ensure_column(&connection, "phase", "TEXT")?;
        ensure_column(&connection, "parent_event_id", "TEXT")?;
        ensure_column(&connection, "content_json", "TEXT")?;
        ensure_column(&connection, "content_hash", "TEXT")?;
        ensure_column(&connection, "source_kind", "TEXT")?;
        ensure_column(&connection, "source_event_id", "TEXT")?;
        connection.execute_batch(
            "
            CREATE INDEX IF NOT EXISTS development_trace_entries_cycle_sequence_idx
                ON development_trace_entries (cycle_id, sequence_no);

            CREATE INDEX IF NOT EXISTS development_trace_entries_source_event_id_idx
                ON development_trace_entries (source_event_id);
            ",
        )?;
        Ok(())
    }
}

impl DevelopmentCycleAliasStore for SqliteDevelopmentTraceStore {
    fn reserve_cycle_alias(
        &self,
        reservation: &DevelopmentCycleAliasReservation,
    ) -> DevelopmentCycleAliasStoreResult<DevelopmentCycleAlias> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;

        if cycle_alias_for_cycle_id_in_connection(&transaction, &reservation.cycle_id)?.is_some() {
            return Err(format!(
                "cycle alias already reserved for cycle_id {}",
                reservation.cycle_id
            )
            .into());
        }

        let alias = match &reservation.request {
            DevelopmentCycleAliasRequest::FullAlias(_) => {
                let parts = reservation.full_alias_parts()?;
                if cycle_alias_for_alias_in_connection(&transaction, &parts.cycle_alias)?.is_some()
                {
                    return Err(
                        format!("cycle alias already reserved: {}", parts.cycle_alias).into()
                    );
                }
                if cycle_alias_for_category_sequence_in_connection(
                    &transaction,
                    &parts.cycle_category_key,
                    parts.cycle_sequence,
                )?
                .is_some()
                {
                    return Err(format!(
                        "cycle alias sequence already reserved: {}-{:03}",
                        parts.cycle_category_key, parts.cycle_sequence
                    )
                    .into());
                }
                alias_from_parts(reservation, parts)
            }
            DevelopmentCycleAliasRequest::Category(_) => {
                let (cycle_category, cycle_category_key) = reservation.category_parts()?;
                let cycle_sequence = next_cycle_alias_sequence(&transaction, &cycle_category_key)?;
                let cycle_alias = format_development_cycle_alias(&cycle_category, cycle_sequence);
                let parts = validate_development_cycle_alias(&cycle_alias)?;
                alias_from_parts(reservation, parts)
            }
        };

        insert_cycle_alias(&transaction, &alias).map_err(|error| {
            format!(
                "cycle alias reservation collision or persistence failure for {}: {error}",
                alias.cycle_alias
            )
        })?;
        transaction.commit()?;
        Ok(alias)
    }

    fn get_cycle_alias_by_alias(
        &self,
        cycle_alias: &str,
    ) -> DevelopmentCycleAliasStoreResult<Option<DevelopmentCycleAlias>> {
        let connection = self.connection()?;
        cycle_alias_for_alias_in_connection(&connection, cycle_alias)
    }

    fn get_cycle_alias_by_cycle_id(
        &self,
        cycle_id: &str,
    ) -> DevelopmentCycleAliasStoreResult<Option<DevelopmentCycleAlias>> {
        let connection = self.connection()?;
        cycle_alias_for_cycle_id_in_connection(&connection, cycle_id)
    }

    fn list_cycle_aliases(&self) -> DevelopmentCycleAliasStoreResult<Vec<DevelopmentCycleAlias>> {
        let connection = self.connection()?;
        let mut statement = connection.prepare(
            "
            SELECT cycle_id, cycle_alias, cycle_category, cycle_category_key,
                cycle_sequence, cycle_title, created_at
            FROM development_cycle_aliases
            ORDER BY cycle_category_key, cycle_sequence, cycle_alias
            ",
        )?;
        let rows = statement.query_map([], development_cycle_alias_from_row)?;
        let mut aliases = Vec::new();
        for row in rows {
            aliases.push(row?);
        }
        Ok(aliases)
    }
}

impl DevelopmentTraceStore for SqliteDevelopmentTraceStore {
    fn append_entry(
        &self,
        entry: &NewDevelopmentTraceEntry,
    ) -> DevelopmentTraceStoreResult<DevelopmentTraceEntry> {
        let connection = self.connection()?;
        let canonical =
            canonical_development_trace_columns(entry.kind, &entry.body, &entry.metadata_json)
                .map_err(|error| format!("trace canonical column extraction failed: {error}"))?;
        connection.execute(
            "
            INSERT INTO development_trace_entries (
                event_id, cycle_id, user_turn_id, kind, role_name,
                summary, body, metadata_json,
                schema_version, sequence_no, phase, parent_event_id,
                content_json, content_hash, source_kind, source_event_id,
                created_at
            )
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)
            ",
            params![
                entry.event_id,
                entry.cycle_id,
                entry.user_turn_id,
                entry.kind.as_str(),
                entry.role_name,
                entry.summary,
                entry.body,
                entry.metadata_json,
                canonical.schema_version,
                canonical.sequence_no,
                canonical.phase,
                canonical.parent_event_id,
                canonical.content_json,
                canonical.content_hash,
                canonical.source_kind,
                canonical.source_event_id,
                entry.created_at,
            ],
        )?;

        let row_id = connection.last_insert_rowid();
        get_entry_by_row_id(&connection, row_id)?
            .ok_or_else(|| format!("inserted development trace entry not found: {row_id}").into())
    }

    fn list_entries(
        &self,
        filter: &DevelopmentTraceFilter,
    ) -> DevelopmentTraceStoreResult<Vec<DevelopmentTraceEntry>> {
        let connection = self.connection()?;
        let limit = limit_as_i64(filter.limit);

        match (&filter.cycle_id, filter.kind) {
            (Some(cycle_id), Some(kind)) => query_entries(
                &connection,
                "
                SELECT * FROM development_trace_entries
                WHERE cycle_id = ?1 AND kind = ?2
                ORDER BY id
                LIMIT ?3
                ",
                params![cycle_id, kind.as_str(), limit],
            ),
            (Some(cycle_id), None) => query_entries(
                &connection,
                "
                SELECT * FROM development_trace_entries
                WHERE cycle_id = ?1
                ORDER BY id
                LIMIT ?2
                ",
                params![cycle_id, limit],
            ),
            (None, Some(kind)) => query_entries(
                &connection,
                "
                SELECT * FROM development_trace_entries
                WHERE kind = ?1
                ORDER BY id
                LIMIT ?2
                ",
                params![kind.as_str(), limit],
            ),
            (None, None) => query_entries(
                &connection,
                "
                SELECT * FROM development_trace_entries
                ORDER BY id
                LIMIT ?1
                ",
                params![limit],
            ),
        }
    }

    fn list_latest_entries(
        &self,
        filter: &DevelopmentTraceFilter,
    ) -> DevelopmentTraceStoreResult<Vec<DevelopmentTraceEntry>> {
        let connection = self.connection()?;
        let limit = limit_as_i64(filter.limit);

        let mut entries = match (&filter.cycle_id, filter.kind) {
            (Some(cycle_id), Some(kind)) => query_entries(
                &connection,
                "
                SELECT * FROM development_trace_entries
                WHERE cycle_id = ?1 AND kind = ?2
                ORDER BY id DESC
                LIMIT ?3
                ",
                params![cycle_id, kind.as_str(), limit],
            )?,
            (Some(cycle_id), None) => query_entries(
                &connection,
                "
                SELECT * FROM development_trace_entries
                WHERE cycle_id = ?1
                ORDER BY id DESC
                LIMIT ?2
                ",
                params![cycle_id, limit],
            )?,
            (None, Some(kind)) => query_entries(
                &connection,
                "
                SELECT * FROM development_trace_entries
                WHERE kind = ?1
                ORDER BY id DESC
                LIMIT ?2
                ",
                params![kind.as_str(), limit],
            )?,
            (None, None) => query_entries(
                &connection,
                "
                SELECT * FROM development_trace_entries
                ORDER BY id DESC
                LIMIT ?1
                ",
                params![limit],
            )?,
        };
        entries.sort_by_key(|entry| entry.id);
        Ok(entries)
    }

    fn get_entry_by_event_id(
        &self,
        event_id: &str,
    ) -> DevelopmentTraceStoreResult<Option<DevelopmentTraceEntry>> {
        let connection = self.connection()?;
        let entry = connection
            .query_row(
                "SELECT * FROM development_trace_entries WHERE event_id = ?1",
                params![event_id],
                development_trace_entry_from_row,
            )
            .optional()?;
        Ok(entry)
    }
}

fn alias_from_parts(
    reservation: &DevelopmentCycleAliasReservation,
    parts: DevelopmentCycleAliasParts,
) -> DevelopmentCycleAlias {
    DevelopmentCycleAlias {
        cycle_id: reservation.cycle_id.clone(),
        cycle_alias: parts.cycle_alias,
        cycle_category: parts.cycle_category,
        cycle_category_key: parts.cycle_category_key,
        cycle_sequence: parts.cycle_sequence,
        cycle_title: reservation.cycle_title.clone(),
        created_at: reservation.created_at.clone(),
    }
}

fn next_cycle_alias_sequence(
    connection: &Connection,
    cycle_category_key: &str,
) -> DevelopmentCycleAliasStoreResult<u64> {
    let next_sequence = connection.query_row(
        "
        SELECT COALESCE(MAX(cycle_sequence), 0) + 1
        FROM development_cycle_aliases
        WHERE cycle_category_key = ?1
        ",
        params![cycle_category_key],
        |row| row.get::<_, i64>(0),
    )?;
    if next_sequence <= 0 {
        return Err(format!("invalid next cycle alias sequence: {next_sequence}").into());
    }
    u64::try_from(next_sequence)
        .map_err(|error| format!("next cycle alias sequence conversion failed: {error}").into())
}

fn insert_cycle_alias(
    connection: &Connection,
    alias: &DevelopmentCycleAlias,
) -> DevelopmentCycleAliasStoreResult<()> {
    let cycle_sequence = i64::try_from(alias.cycle_sequence)
        .map_err(|error| format!("cycle alias sequence too large for sqlite: {error}"))?;
    connection.execute(
        "
        INSERT INTO development_cycle_aliases (
            cycle_id, cycle_alias, cycle_category, cycle_category_key,
            cycle_sequence, cycle_title, created_at
        )
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        ",
        params![
            alias.cycle_id,
            alias.cycle_alias,
            alias.cycle_category,
            alias.cycle_category_key,
            cycle_sequence,
            alias.cycle_title.as_deref(),
            alias.created_at,
        ],
    )?;
    Ok(())
}

fn cycle_alias_for_alias_in_connection(
    connection: &Connection,
    cycle_alias: &str,
) -> DevelopmentCycleAliasStoreResult<Option<DevelopmentCycleAlias>> {
    Ok(connection
        .query_row(
            "
            SELECT cycle_id, cycle_alias, cycle_category, cycle_category_key,
                cycle_sequence, cycle_title, created_at
            FROM development_cycle_aliases
            WHERE cycle_alias = ?1
            ",
            params![cycle_alias],
            development_cycle_alias_from_row,
        )
        .optional()?)
}

fn cycle_alias_for_cycle_id_in_connection(
    connection: &Connection,
    cycle_id: &str,
) -> DevelopmentCycleAliasStoreResult<Option<DevelopmentCycleAlias>> {
    Ok(connection
        .query_row(
            "
            SELECT cycle_id, cycle_alias, cycle_category, cycle_category_key,
                cycle_sequence, cycle_title, created_at
            FROM development_cycle_aliases
            WHERE cycle_id = ?1
            ",
            params![cycle_id],
            development_cycle_alias_from_row,
        )
        .optional()?)
}

fn cycle_alias_for_category_sequence_in_connection(
    connection: &Connection,
    cycle_category_key: &str,
    cycle_sequence: u64,
) -> DevelopmentCycleAliasStoreResult<Option<DevelopmentCycleAlias>> {
    let cycle_sequence = i64::try_from(cycle_sequence)
        .map_err(|error| format!("cycle alias sequence too large for sqlite: {error}"))?;
    Ok(connection
        .query_row(
            "
            SELECT cycle_id, cycle_alias, cycle_category, cycle_category_key,
                cycle_sequence, cycle_title, created_at
            FROM development_cycle_aliases
            WHERE cycle_category_key = ?1 AND cycle_sequence = ?2
            ",
            params![cycle_category_key, cycle_sequence],
            development_cycle_alias_from_row,
        )
        .optional()?)
}

fn development_cycle_alias_from_row(row: &Row<'_>) -> rusqlite::Result<DevelopmentCycleAlias> {
    let cycle_sequence = row.get::<_, i64>("cycle_sequence")?;
    if cycle_sequence <= 0 {
        return Err(rusqlite::Error::FromSqlConversionFailure(
            4,
            rusqlite::types::Type::Integer,
            format!("invalid cycle alias sequence: {cycle_sequence}").into(),
        ));
    }
    Ok(DevelopmentCycleAlias {
        cycle_id: row.get("cycle_id")?,
        cycle_alias: row.get("cycle_alias")?,
        cycle_category: row.get("cycle_category")?,
        cycle_category_key: row.get("cycle_category_key")?,
        cycle_sequence: u64::try_from(cycle_sequence).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                4,
                rusqlite::types::Type::Integer,
                format!("cycle alias sequence conversion failed: {error}").into(),
            )
        })?,
        cycle_title: row.get("cycle_title")?,
        created_at: row.get("created_at")?,
    })
}

fn ensure_column(
    connection: &Connection,
    column_name: &str,
    definition: &str,
) -> DevelopmentTraceStoreResult<()> {
    let mut statement = connection.prepare("PRAGMA table_info(development_trace_entries)")?;
    let rows = statement.query_map([], |row| row.get::<_, String>("name"))?;
    for row in rows {
        if row? == column_name {
            return Ok(());
        }
    }
    connection.execute(
        &format!("ALTER TABLE development_trace_entries ADD COLUMN {column_name} {definition}"),
        [],
    )?;
    Ok(())
}

fn query_entries<P>(
    connection: &Connection,
    sql: &str,
    params: P,
) -> DevelopmentTraceStoreResult<Vec<DevelopmentTraceEntry>>
where
    P: rusqlite::Params,
{
    let mut statement = connection.prepare(sql)?;
    let rows = statement.query_map(params, development_trace_entry_from_row)?;
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

fn get_entry_by_row_id(
    connection: &Connection,
    row_id: i64,
) -> DevelopmentTraceStoreResult<Option<DevelopmentTraceEntry>> {
    let entry = connection
        .query_row(
            "SELECT * FROM development_trace_entries WHERE id = ?1",
            params![row_id],
            development_trace_entry_from_row,
        )
        .optional()?;
    Ok(entry)
}

fn development_trace_entry_from_row(row: &Row<'_>) -> rusqlite::Result<DevelopmentTraceEntry> {
    let kind_text: String = row.get("kind")?;
    let kind = DevelopmentTraceKind::parse(&kind_text).ok_or_else(|| {
        rusqlite::Error::FromSqlConversionFailure(
            4,
            rusqlite::types::Type::Text,
            format!("unknown development trace kind: {kind_text}").into(),
        )
    })?;

    Ok(DevelopmentTraceEntry {
        id: row.get("id")?,
        event_id: row.get("event_id")?,
        cycle_id: row.get("cycle_id")?,
        user_turn_id: row.get("user_turn_id")?,
        kind,
        role_name: row.get("role_name")?,
        summary: row.get("summary")?,
        body: row.get("body")?,
        metadata_json: row.get("metadata_json")?,
        created_at: row.get("created_at")?,
    })
}

fn limit_as_i64(limit: Option<usize>) -> i64 {
    limit.and_then(|value| i64::try_from(value).ok()).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use xavi_application::ports::development_trace_store::DevelopmentTraceStore;
    use xavi_domain::development_cycle::{
        DevelopmentCycleAliasRequest, DevelopmentCycleAliasReservation,
    };

    #[test]
    fn init_schema_adds_canonical_columns_without_rewriting_legacy_rows() {
        let connection = Connection::open_in_memory().expect("sqlite memory should open");
        connection
            .execute_batch(
                "
                CREATE TABLE development_trace_entries (
                    id INTEGER PRIMARY KEY,
                    event_id TEXT NOT NULL UNIQUE,
                    cycle_id TEXT NOT NULL,
                    user_turn_id TEXT,
                    kind TEXT NOT NULL,
                    role_name TEXT,
                    summary TEXT NOT NULL,
                    body TEXT NOT NULL,
                    metadata_json TEXT NOT NULL,
                    created_at TEXT NOT NULL
                );
                ",
            )
            .expect("legacy schema should create");
        connection
            .execute(
                "
                INSERT INTO development_trace_entries (
                    event_id, cycle_id, user_turn_id, kind, role_name,
                    summary, body, metadata_json, created_at
                )
                VALUES (?1, ?2, NULL, ?3, NULL, ?4, ?5, ?6, ?7)
                ",
                params![
                    "legacy-event-1",
                    "cycle-legacy",
                    "project_knowledge_note",
                    "legacy summary",
                    "legacy body",
                    "{}",
                    "unix:1",
                ],
            )
            .expect("legacy row should insert");
        let store = SqliteDevelopmentTraceStore { connection: Mutex::new(connection) };

        store.init_schema().expect("schema migration should be additive");

        let connection = store.connection().expect("connection should lock");
        let (summary, body, metadata_json, content_json, content_hash): (
            String,
            String,
            String,
            Option<String>,
            Option<String>,
        ) = connection
            .query_row(
                "
                SELECT summary, body, metadata_json, content_json, content_hash
                FROM development_trace_entries
                WHERE event_id = ?1
                ",
                params!["legacy-event-1"],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .expect("legacy row should remain readable");

        assert_eq!(summary, "legacy summary");
        assert_eq!(body, "legacy body");
        assert_eq!(metadata_json, "{}");
        assert_eq!(content_json, None);
        assert_eq!(content_hash, None);
    }

    #[test]
    fn append_entry_persists_canonical_columns_from_content_contract() {
        let store = SqliteDevelopmentTraceStore::open_in_memory()
            .expect("in-memory trace store should open");
        let stored = store
            .append_entry(&recovered_event_entry())
            .expect("complete recovered event should append");

        let connection = store.connection().expect("connection should lock");
        let (
            schema_version,
            sequence_no,
            phase,
            parent_event_id,
            content_json,
            content_hash,
            source_kind,
            source_event_id,
        ): (i64, i64, String, Option<String>, String, String, String, String) = connection
            .query_row(
                "
                SELECT schema_version, sequence_no, phase, parent_event_id,
                    content_json, content_hash, source_kind, source_event_id
                FROM development_trace_entries
                WHERE event_id = ?1
                ",
                params![stored.event_id],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                        row.get(7)?,
                    ))
                },
            )
            .expect("canonical columns should be queryable");

        assert_eq!(schema_version, 1);
        assert_eq!(sequence_no, 7);
        assert_eq!(phase, "recovery-1");
        assert_eq!(parent_event_id, None);
        assert!(content_json.contains("\"evidence\""));
        assert!(content_hash.starts_with("fnv1a64:"));
        assert_eq!(source_kind, "user_query");
        assert_eq!(source_event_id, "user-query-1");
    }

    #[test]
    fn append_entry_rejects_strict_contract_without_required_content() {
        let store = SqliteDevelopmentTraceStore::open_in_memory()
            .expect("in-memory trace store should open");
        let error = store
            .append_entry(&NewDevelopmentTraceEntry {
                event_id: "user-query-incomplete".to_owned(),
                cycle_id: "cycle-test".to_owned(),
                user_turn_id: None,
                kind: DevelopmentTraceKind::UserQuery,
                role_name: Some("user".to_owned()),
                summary: "incomplete".to_owned(),
                body: "body".to_owned(),
                metadata_json: r#"{"trace_contract_version":1,"phase_id":"user-query","cycle_step":1,"role":"user","status":"requested"}"#.to_owned(),
                created_at: "unix:1".to_owned(),
            })
            .expect_err("strict contract without content should not persist")
            .to_string();

        assert!(error.contains("trace canonical column extraction failed"));
        assert!(error.contains("missing_content_json"));
    }

    #[test]
    fn list_latest_entries_reads_newest_window_and_returns_ascending_ids() {
        let store = SqliteDevelopmentTraceStore::open_in_memory()
            .expect("in-memory trace store should open");
        let mut stored_ids = Vec::new();
        for step in 1..=5 {
            stored_ids.push(
                store
                    .append_entry(&recovered_event_entry_with_step(step))
                    .expect("trace entry should append")
                    .id,
            );
        }

        let latest = store
            .list_latest_entries(&DevelopmentTraceFilter {
                cycle_id: Some("cycle-test".to_owned()),
                kind: None,
                limit: Some(2),
            })
            .expect("latest entries should list");

        assert_eq!(
            latest.iter().map(|entry| entry.id).collect::<Vec<_>>(),
            vec![stored_ids[3], stored_ids[4]]
        );
        assert_eq!(latest[0].event_id, "recovered-event-4");
        assert_eq!(latest[1].event_id, "recovered-event-5");
    }

    #[test]
    fn reserve_full_cycle_alias_persists_exact_alias_and_resolves_cycle() {
        let store = SqliteDevelopmentTraceStore::open_in_memory()
            .expect("in-memory trace store should open");
        let reservation = alias_reservation(
            "cycle-alias-test",
            DevelopmentCycleAliasRequest::FullAlias("Feature_한글-007".to_owned()),
            Some("별칭 예약 테스트"),
        );

        let stored = store.reserve_cycle_alias(&reservation).expect("full alias should reserve");
        let by_alias = store
            .get_cycle_alias_by_alias("Feature_한글-007")
            .expect("alias should resolve")
            .expect("alias should exist");
        let by_cycle = store
            .get_cycle_alias_by_cycle_id("cycle-alias-test")
            .expect("cycle alias should resolve")
            .expect("cycle should have alias");

        assert_eq!(stored.cycle_alias, "Feature_한글-007");
        assert_eq!(stored.cycle_category, "Feature_한글");
        assert_eq!(stored.cycle_category_key, "feature_한글");
        assert_eq!(stored.cycle_sequence, 7);
        assert_eq!(stored.cycle_title.as_deref(), Some("별칭 예약 테스트"));
        assert_eq!(by_alias, stored);
        assert_eq!(by_cycle, stored);
    }

    #[test]
    fn reserve_category_only_allocates_next_sequence_transactionally() {
        let store = SqliteDevelopmentTraceStore::open_in_memory()
            .expect("in-memory trace store should open");
        let first = store
            .reserve_cycle_alias(&alias_reservation(
                "cycle-category-1",
                DevelopmentCycleAliasRequest::Category("Feature".to_owned()),
                None,
            ))
            .expect("first category alias should reserve");
        let second = store
            .reserve_cycle_alias(&alias_reservation(
                "cycle-category-2",
                DevelopmentCycleAliasRequest::Category("feature".to_owned()),
                None,
            ))
            .expect("second category alias should reserve");

        assert_eq!(first.cycle_alias, "Feature-001");
        assert_eq!(first.cycle_sequence, 1);
        assert_eq!(second.cycle_alias, "feature-002");
        assert_eq!(second.cycle_sequence, 2);
    }

    #[test]
    fn reserve_cycle_alias_fails_closed_on_collisions_without_fallback() {
        let store = SqliteDevelopmentTraceStore::open_in_memory()
            .expect("in-memory trace store should open");
        store
            .reserve_cycle_alias(&alias_reservation(
                "cycle-collision-1",
                DevelopmentCycleAliasRequest::FullAlias("feature-001".to_owned()),
                None,
            ))
            .expect("initial alias should reserve");

        let alias_error = store
            .reserve_cycle_alias(&alias_reservation(
                "cycle-collision-2",
                DevelopmentCycleAliasRequest::FullAlias("feature-001".to_owned()),
                None,
            ))
            .expect_err("exact alias collision should fail")
            .to_string();
        let category_sequence_error = store
            .reserve_cycle_alias(&alias_reservation(
                "cycle-collision-3",
                DevelopmentCycleAliasRequest::FullAlias("Feature-001".to_owned()),
                None,
            ))
            .expect_err("category key sequence collision should fail")
            .to_string();
        let cycle_error = store
            .reserve_cycle_alias(&alias_reservation(
                "cycle-collision-1",
                DevelopmentCycleAliasRequest::FullAlias("feature-002".to_owned()),
                None,
            ))
            .expect_err("cycle id collision should fail")
            .to_string();

        assert!(alias_error.contains("already reserved"));
        assert!(category_sequence_error.contains("sequence already reserved"));
        assert!(cycle_error.contains("cycle_id"));
        assert!(store.get_cycle_alias_by_alias("feature-002").unwrap().is_none());
    }

    fn recovered_event_entry() -> NewDevelopmentTraceEntry {
        NewDevelopmentTraceEntry {
            event_id: "recovered-event-1".to_owned(),
            cycle_id: "cycle-test".to_owned(),
            user_turn_id: None,
            kind: DevelopmentTraceKind::OrchestraJudgment,
            role_name: Some("orchestra".to_owned()),
            summary: "recovered event".to_owned(),
            body: "recovered event body".to_owned(),
            metadata_json: r#"{"trace_contract_version":1,"phase_id":"recovery-1","cycle_step":7,"role":"orchestra","status":"reported","record_kind":"recovered_event","content_json":{"evidence":"stored source row confirms this","source_kind":"user_query","source_event_id":"user-query-1"}}"#.to_owned(),
            created_at: "unix:1".to_owned(),
        }
    }

    fn recovered_event_entry_with_step(step: i64) -> NewDevelopmentTraceEntry {
        NewDevelopmentTraceEntry {
            event_id: format!("recovered-event-{step}"),
            cycle_id: "cycle-test".to_owned(),
            user_turn_id: None,
            kind: DevelopmentTraceKind::OrchestraJudgment,
            role_name: Some("orchestra".to_owned()),
            summary: "recovered event".to_owned(),
            body: "recovered event body".to_owned(),
            metadata_json: format!(
                r#"{{"trace_contract_version":1,"phase_id":"recovery-{step}","cycle_step":{step},"role":"orchestra","status":"reported","record_kind":"recovered_event","content_json":{{"evidence":"stored source row confirms this","source_kind":"user_query","source_event_id":"user-query-{step}"}}}}"#
            ),
            created_at: "unix:1".to_owned(),
        }
    }

    fn alias_reservation(
        cycle_id: &str,
        request: DevelopmentCycleAliasRequest,
        title: Option<&str>,
    ) -> DevelopmentCycleAliasReservation {
        DevelopmentCycleAliasReservation::new(cycle_id, request, title.map(str::to_owned), "unix:1")
            .expect("alias reservation should validate")
    }
}
