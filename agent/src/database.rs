//
// SQLite database for telemetry
//

use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error};
use crate::events::FileEvent;
use crate::detectors::DetectorScores;

pub struct Database {
    path: PathBuf,
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    pub async fn new(path: &PathBuf) -> Result<Self> {
        // Ensure directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        let db = Self {
            path: path.clone(),
            conn: Arc::new(Mutex::new(conn)),
        };

        Ok(db)
    }

    pub async fn initialize_schema(&self) -> Result<()> {
        let conn = self.conn.lock().await;
        
        // Events table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                process_id INTEGER NOT NULL,
                process_path TEXT,
                file_path TEXT,
                bytes_read INTEGER,
                bytes_written INTEGER,
                timestamp INTEGER NOT NULL
            )",
            [],
        )?;

        // Detector outputs table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS detector_outputs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                process_id INTEGER NOT NULL,
                entropy_score REAL,
                mass_write_score REAL,
                mass_rename_delete_score REAL,
                ransom_note_score REAL,
                shadow_copy_score REAL,
                process_behavior_score REAL,
                file_extension_score REAL,
                timestamp INTEGER NOT NULL
            )",
            [],
        )?;

        // ML inference results
        conn.execute(
            "CREATE TABLE IF NOT EXISTS ml_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                process_id INTEGER NOT NULL,
                ml_score REAL NOT NULL,
                timestamp INTEGER NOT NULL
            )",
            [],
        )?;

        // Alerts table
        conn.execute(
            "CREATE TABLE IF NOT EXISTS alerts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                process_id INTEGER NOT NULL,
                ml_score REAL NOT NULL,
                quarantined INTEGER NOT NULL,
                timestamp INTEGER NOT NULL
            )",
            [],
        )?;

        // Quarantine actions
        conn.execute(
            "CREATE TABLE IF NOT EXISTS quarantine_actions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                process_id INTEGER NOT NULL,
                action_type TEXT NOT NULL,
                success INTEGER NOT NULL,
                timestamp INTEGER NOT NULL
            )",
            [],
        )?;

        debug!("Database schema initialized");
        Ok(())
    }

    pub async fn store_event(&self, event: &FileEvent) -> Result<()> {
        let conn = self.conn.lock().await;
        
        conn.execute(
            "INSERT INTO events (event_type, process_id, process_path, file_path, bytes_read, bytes_written, timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                format!("{:?}", event.event_type),
                event.process_id,
                event.process_path,
                event.file_path,
                event.bytes_read,
                event.bytes_written,
                event.timestamp,
            ],
        )?;

        Ok(())
    }

    pub async fn log_alert(&self, scores: &DetectorScores, ml_score: f32) -> Result<()> {
        let conn = self.conn.lock().await;
        
        conn.execute(
            "INSERT INTO alerts (process_id, ml_score, quarantined, timestamp)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                scores.process_id,
                ml_score,
                1, // quarantined
                scores.timestamp,
            ],
        )?;

        Ok(())
    }
}

