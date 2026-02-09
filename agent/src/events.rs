//
// Event ingestion and processing
//

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error};

use crate::database::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEvent {
    pub event_type: EventType,
    pub process_id: u32,
    pub process_path: String,
    pub file_path: String,
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub timestamp: i64,
    pub entropy_preview: Vec<u8>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum EventType {
    FileCreate,
    FileRead,
    FileWrite,
    FileRename,
    FileDelete,
    DirectoryEnum,
    VSSDelete,
    ProcessCreate,
    RegistryChange,
}

pub struct EventIngestion {
    event_rx: mpsc::UnboundedReceiver<FileEvent>,
    detector_tx: mpsc::UnboundedSender<FileEvent>,
    db: Arc<Database>,
}

impl EventIngestion {
    pub fn new(
        event_rx: mpsc::UnboundedReceiver<FileEvent>,
        detector_tx: mpsc::UnboundedSender<FileEvent>,
        db: Arc<Database>,
    ) -> Self {
        Self {
            event_rx,
            detector_tx,
            db,
        }
    }

    pub async fn start(mut self) -> Result<()> {
        while let Some(event) = self.event_rx.recv().await {
            debug!("Received event: {:?}", event);

            // Store event in database
            if let Err(e) = self.db.store_event(&event).await {
                error!("Failed to store event: {}", e);
            }

            // Send to detector manager
            if let Err(e) = self.detector_tx.send(event) {
                error!("Failed to send event to detector: {}", e);
            }
        }

        Ok(())
    }
}

