//
// SQLite database for telemetry
//

use anyhow::Result;
use rusqlite::{params, Connection};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::debug;

use crate::detectors::DetectorScores;
use crate::events::FileEvent;

pub struct Database {
    _path: PathBuf,
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub async fn new(path: &PathBuf) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        Ok(Self {
            _path: path.clone(),
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub async fn initialize_schema(&self) -> Result<()> {
        let conn = self.conn.lock().await;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                process_id INTEGER NOT NULL,
                process_path TEXT,
                file_path TEXT,
                bytes_read INTEGER,
                bytes_written INTEGER,
                timestamp INTEGER NOT NULL,
                result INTEGER NOT NULL DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS detector_outputs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                process_id INTEGER NOT NULL,
                process_path TEXT,
                entropy_score REAL,
                mass_write_score REAL,
                mass_rename_delete_score REAL,
                ransom_note_score REAL,
                shadow_copy_score REAL,
                process_behavior_score REAL,
                file_extension_score REAL,
                event_rate REAL,
                avg_entropy_per_sec REAL,
                rename_delete_freq REAL,
                burst_interval REAL,
                num_detectors_firing REAL,
                file_diversity REAL,
                bytes_written_per_sec REAL,
                unique_extensions REAL,
                timestamp INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS ml_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                process_id INTEGER NOT NULL,
                ml_score REAL NOT NULL,
                timestamp INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS alerts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                process_id INTEGER NOT NULL,
                ml_score REAL NOT NULL,
                quarantined INTEGER NOT NULL,
                timestamp INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS quarantine_actions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                process_id INTEGER NOT NULL,
                action_type TEXT NOT NULL,
                success INTEGER NOT NULL,
                timestamp INTEGER NOT NULL
            );
            ",
        )?;

        ensure_column_exists(&conn, "events", "result", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column_exists(&conn, "detector_outputs", "process_path", "TEXT")?;
        ensure_column_exists(&conn, "detector_outputs", "event_rate", "REAL")?;
        ensure_column_exists(&conn, "detector_outputs", "avg_entropy_per_sec", "REAL")?;
        ensure_column_exists(&conn, "detector_outputs", "rename_delete_freq", "REAL")?;
        ensure_column_exists(&conn, "detector_outputs", "burst_interval", "REAL")?;
        ensure_column_exists(&conn, "detector_outputs", "num_detectors_firing", "REAL")?;
        ensure_column_exists(&conn, "detector_outputs", "file_diversity", "REAL")?;
        ensure_column_exists(&conn, "detector_outputs", "bytes_written_per_sec", "REAL")?;
        ensure_column_exists(&conn, "detector_outputs", "unique_extensions", "REAL")?;

        debug!("Database schema initialized");
        Ok(())
    }

    pub async fn store_event(&self, event: &FileEvent) -> Result<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO events (event_type, process_id, process_path, file_path, bytes_read, bytes_written, timestamp, result)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                format!("{:?}", event.event_type),
                event.process_id,
                event.process_path,
                event.file_path,
                event.bytes_read,
                event.bytes_written,
                event.timestamp,
                event.result,
            ],
        )?;

        Ok(())
    }

    pub async fn store_detector_scores(&self, scores: &DetectorScores) -> Result<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO detector_outputs (
                process_id, process_path, entropy_score, mass_write_score, mass_rename_delete_score,
                ransom_note_score, shadow_copy_score, process_behavior_score, file_extension_score,
                event_rate, avg_entropy_per_sec, rename_delete_freq, burst_interval,
                num_detectors_firing, file_diversity, bytes_written_per_sec, unique_extensions, timestamp
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                scores.process_id,
                scores.process_path,
                scores.entropy_score,
                scores.mass_write_score,
                scores.mass_rename_delete_score,
                scores.ransom_note_score,
                scores.shadow_copy_score,
                scores.process_behavior_score,
                scores.file_extension_score,
                scores.event_rate,
                scores.avg_entropy_per_sec,
                scores.rename_delete_freq,
                scores.burst_interval,
                scores.num_detectors_firing,
                scores.file_diversity,
                scores.bytes_written_per_sec,
                scores.unique_extensions,
                scores.timestamp,
            ],
        )?;

        Ok(())
    }

    pub async fn log_ml_result(&self, process_id: u32, ml_score: f32, timestamp: i64) -> Result<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO ml_results (process_id, ml_score, timestamp) VALUES (?1, ?2, ?3)",
            params![process_id, ml_score, timestamp],
        )?;
        Ok(())
    }

    pub async fn log_alert(&self, scores: &DetectorScores, ml_score: f32) -> Result<()> {
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT INTO alerts (process_id, ml_score, quarantined, timestamp)
             VALUES (?1, ?2, ?3, ?4)",
            params![scores.process_id, ml_score, 1, scores.timestamp],
        )?;

        Ok(())
    }

    pub async fn get_system_metrics(&self) -> Result<(i64, i64, i32, i32)> {
        let conn = self.conn.lock().await;
        let now = chrono::Utc::now().timestamp();

        let total_events: i64 = conn.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;

        let events_last_5s: i64 = conn.query_row(
            "SELECT COUNT(*) FROM events WHERE timestamp >= ?1",
            params![now - 5],
            |row| row.get(0),
        )?;

        let active_processes: i32 = conn.query_row(
            "SELECT COUNT(DISTINCT process_id) FROM events WHERE timestamp >= ?1 AND result = 0",
            params![now - 60],
            |row| row.get(0),
        )?;

        let quarantined_count: i32 = conn.query_row(
            "SELECT COUNT(*) FROM alerts WHERE quarantined = 1",
            [],
            |row| row.get(0),
        )?;

        Ok((total_events, events_last_5s, active_processes, quarantined_count))
    }
}

fn ensure_column_exists(conn: &Connection, table: &str, column: &str, definition: &str) -> Result<()> {
    let pragma = format!("PRAGMA table_info({})", table);
    let mut stmt = conn.prepare(&pragma)?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let existing: String = row.get(1)?;
        if existing.eq_ignore_ascii_case(column) {
            return Ok(());
        }
    }

    let alter = format!("ALTER TABLE {} ADD COLUMN {} {}", table, column, definition);
    conn.execute(&alter, [])?;
    Ok(())
}
