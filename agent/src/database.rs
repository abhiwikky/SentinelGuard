//! SentinelGuard Database Module
//!
//! SQLite persistence for events, alerts, detector results, and quarantine records.

use crate::events::{Alert, DetectorResult, FileEvent, QuarantineStatus, Severity};
use anyhow::{Context, Result};
use parking_lot::Mutex;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

/// Database manager for SQLite operations
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Open or create the database at the given path
    pub fn open(path: &str, wal_mode: bool) -> Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create database directory: {}", parent.display()))?;
        }

        let conn = Connection::open(path)
            .with_context(|| format!("Failed to open database: {}", path))?;

        if wal_mode {
            conn.pragma_update(None, "journal_mode", "WAL")?;
        }
        conn.pragma_update(None, "synchronous", "NORMAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };

        db.initialize_schema()?;

        info!("Database opened: {}", path);
        Ok(db)
    }

    /// Create all tables if they don't exist
    fn initialize_schema(&self) -> Result<()> {
        let conn = self.conn.lock();

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS events (
                event_id        INTEGER PRIMARY KEY,
                process_id      INTEGER NOT NULL,
                process_name    TEXT NOT NULL,
                operation       INTEGER NOT NULL,
                file_path       TEXT NOT NULL,
                new_file_path   TEXT,
                file_size       INTEGER NOT NULL DEFAULT 0,
                entropy         REAL NOT NULL DEFAULT 0.0,
                timestamp_ns    INTEGER NOT NULL,
                file_extension  TEXT NOT NULL DEFAULT '',
                created_at      TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS alerts (
                alert_id            INTEGER PRIMARY KEY AUTOINCREMENT,
                process_id          INTEGER NOT NULL,
                process_name        TEXT NOT NULL,
                severity            INTEGER NOT NULL,
                risk_score          REAL NOT NULL,
                description         TEXT NOT NULL,
                quarantine_status   INTEGER NOT NULL DEFAULT 0,
                timestamp_ns        INTEGER NOT NULL,
                created_at          TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS detector_results (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                alert_id        INTEGER,
                detector_name   TEXT NOT NULL,
                score           REAL NOT NULL,
                evidence        TEXT NOT NULL DEFAULT '[]',
                timestamp_ns    INTEGER NOT NULL,
                process_id      INTEGER NOT NULL,
                FOREIGN KEY (alert_id) REFERENCES alerts(alert_id)
            );

            CREATE TABLE IF NOT EXISTS quarantine_log (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                process_id      INTEGER NOT NULL,
                process_name    TEXT NOT NULL,
                risk_score      REAL NOT NULL,
                action          TEXT NOT NULL,
                status          INTEGER NOT NULL,
                timestamp_ns    INTEGER NOT NULL,
                created_at      TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_events_process ON events(process_id);
            CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp_ns);
            CREATE INDEX IF NOT EXISTS idx_alerts_timestamp ON alerts(timestamp_ns);
            CREATE INDEX IF NOT EXISTS idx_detector_results_alert ON detector_results(alert_id);
            CREATE INDEX IF NOT EXISTS idx_quarantine_process ON quarantine_log(process_id);
            ",
        )
        .context("Failed to initialize database schema")?;

        debug!("Database schema initialized");
        Ok(())
    }

    /// Insert a file event
    pub fn insert_event(&self, event: &FileEvent) -> Result<()> {
        let conn = self.conn.lock();
        conn.execute(
            "INSERT INTO events (process_id, process_name, operation,
             file_path, new_file_path, file_size, entropy, timestamp_ns, file_extension)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                event.process_id,
                event.process_name,
                event.operation as u32,
                event.file_path,
                event.new_file_path,
                event.file_size as i64,
                event.entropy,
                event.timestamp_ns as i64,
                event.file_extension,
            ],
        )
        .context("Failed to insert event")?;
        Ok(())
    }

    /// Insert an alert and its detector results
    pub fn insert_alert(&self, alert: &Alert) -> Result<i64> {
        let conn = self.conn.lock();

        conn.execute(
            "INSERT INTO alerts (process_id, process_name, severity, risk_score,
             description, quarantine_status, timestamp_ns)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                alert.process_id,
                alert.process_name,
                alert.severity as i32,
                alert.risk_score,
                alert.description,
                alert.quarantine_status as i32,
                alert.timestamp_ns as i64,
            ],
        )
        .context("Failed to insert alert")?;

        let alert_id = conn.last_insert_rowid();

        // Insert associated detector results
        for result in &alert.detector_results {
            let evidence_json = serde_json::to_string(&result.evidence).unwrap_or_default();
            conn.execute(
                "INSERT INTO detector_results (alert_id, detector_name, score,
                 evidence, timestamp_ns, process_id)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    alert_id,
                    result.detector_name,
                    result.score,
                    evidence_json,
                    result.timestamp_ns as i64,
                    result.process_id,
                ],
            )
            .context("Failed to insert detector result")?;
        }

        Ok(alert_id)
    }

    /// Get recent alerts
    pub fn get_alerts(&self, limit: u32, since_ns: u64) -> Result<Vec<Alert>> {
        let conn = self.conn.lock();
        let limit = if limit == 0 { 100 } else { limit };

        let mut stmt = conn.prepare(
            "SELECT alert_id, process_id, process_name, severity, risk_score,
             description, quarantine_status, timestamp_ns
             FROM alerts WHERE timestamp_ns >= ?1
             ORDER BY timestamp_ns DESC LIMIT ?2",
        )?;

        let alerts: Vec<Alert> = stmt
            .query_map(params![since_ns, limit], |row| {
                let alert_id: i64 = row.get(0)?;
                let severity_val: i32 = row.get(3)?;
                let qs_val: i32 = row.get(6)?;

                Ok(Alert {
                    alert_id: alert_id as u64,
                    process_id: row.get(1)?,
                    process_name: row.get(2)?,
                    severity: match severity_val {
                        1 => Severity::Low,
                        2 => Severity::Medium,
                        3 => Severity::High,
                        4 => Severity::Critical,
                        _ => Severity::Unknown,
                    },
                    risk_score: row.get(4)?,
                    description: row.get(5)?,
                    detector_results: Vec::new(), // Populated below
                    quarantine_status: match qs_val {
                        1 => QuarantineStatus::Suspended,
                        2 => QuarantineStatus::Released,
                        3 => QuarantineStatus::ProcessExited,
                        _ => QuarantineStatus::Unknown,
                    },
                    timestamp_ns: row.get(7)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(alerts)
    }

    /// Get detector results for an alert
    pub fn get_detector_results(&self, limit: u32, since_ns: u64) -> Result<Vec<DetectorResult>> {
        let conn = self.conn.lock();
        let limit = if limit == 0 { 100 } else { limit };

        let mut stmt = conn.prepare(
            "SELECT detector_name, score, evidence, timestamp_ns, process_id
             FROM detector_results WHERE timestamp_ns >= ?1
             ORDER BY timestamp_ns DESC LIMIT ?2",
        )?;

        let results: Vec<DetectorResult> = stmt
            .query_map(params![since_ns, limit], |row| {
                let evidence_str: String = row.get(2)?;
                let evidence: Vec<String> =
                    serde_json::from_str(&evidence_str).unwrap_or_default();

                Ok(DetectorResult {
                    detector_name: row.get(0)?,
                    score: row.get(1)?,
                    evidence,
                    timestamp_ns: row.get(3)?,
                    process_id: row.get(4)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Log a quarantine action
    pub fn log_quarantine(
        &self,
        process_id: u32,
        process_name: &str,
        risk_score: f64,
        action: &str,
        status: QuarantineStatus,
    ) -> Result<()> {
        let conn = self.conn.lock();
        let now_ns = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        conn.execute(
            "INSERT INTO quarantine_log (process_id, process_name, risk_score,
             action, status, timestamp_ns) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                process_id,
                process_name,
                risk_score,
                action,
                status as i32,
                now_ns as i64,
            ],
        )
        .context("Failed to log quarantine action")?;

        Ok(())
    }

    /// Get quarantined processes (those with status=Suspended and no subsequent Release)
    pub fn get_quarantined_processes(&self) -> Result<Vec<(u32, String, f64, u64, QuarantineStatus)>> {
        let conn = self.conn.lock();

        let mut stmt = conn.prepare(
            "SELECT q.process_id, q.process_name, q.risk_score, q.timestamp_ns, q.status
             FROM quarantine_log q
             INNER JOIN (
                 SELECT process_id, MAX(timestamp_ns) as max_ts
                 FROM quarantine_log GROUP BY process_id
             ) latest ON q.process_id = latest.process_id AND q.timestamp_ns = latest.max_ts
             WHERE q.status = 1
             ORDER BY q.timestamp_ns DESC",
        )?;

        let results: Vec<(u32, String, f64, u64, QuarantineStatus)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    QuarantineStatus::Suspended,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(results)
    }

    /// Count total events
    pub fn count_events(&self) -> Result<u64> {
        let conn = self.conn.lock();
        let count: u64 = conn.query_row("SELECT COUNT(*) FROM events", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Count total alerts
    pub fn count_alerts(&self) -> Result<u64> {
        let conn = self.conn.lock();
        let count: u64 = conn.query_row("SELECT COUNT(*) FROM alerts", [], |row| row.get(0))?;
        Ok(count)
    }

    /// Check database connectivity
    pub fn is_connected(&self) -> bool {
        let conn = self.conn.lock();
        conn.execute_batch("SELECT 1").is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::OperationType;

    fn test_db() -> Database {
        Database::open(":memory:", false).unwrap()
    }

    #[test]
    fn test_schema_creation() {
        let db = test_db();
        assert!(db.is_connected());
    }

    #[test]
    fn test_insert_and_count_events() {
        let db = test_db();
        let event = FileEvent::new(
            1, 100, "test.exe".into(),
            OperationType::Write, r"C:\test.txt".into(),
        );
        db.insert_event(&event).unwrap();
        assert_eq!(db.count_events().unwrap(), 1);
    }

    #[test]
    fn test_insert_and_get_alerts() {
        let db = test_db();
        let alert = Alert {
            alert_id: 0,
            process_id: 100,
            process_name: "malware.exe".into(),
            severity: Severity::High,
            risk_score: 0.85,
            description: "Test alert".into(),
            detector_results: vec![DetectorResult::new("entropy_spike", 0.9, vec!["test".into()], 100)],
            quarantine_status: QuarantineStatus::Suspended,
            timestamp_ns: 1000,
        };

        let alert_id = db.insert_alert(&alert).unwrap();
        assert!(alert_id > 0);

        let alerts = db.get_alerts(10, 0).unwrap();
        assert_eq!(alerts.len(), 1);
        assert_eq!(alerts[0].process_name, "malware.exe");
    }

    #[test]
    fn test_quarantine_logging() {
        let db = test_db();
        db.log_quarantine(100, "malware.exe", 0.9, "suspend", QuarantineStatus::Suspended)
            .unwrap();

        let quarantined = db.get_quarantined_processes().unwrap();
        assert_eq!(quarantined.len(), 1);
        assert_eq!(quarantined[0].0, 100);
    }
}
