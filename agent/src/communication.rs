//! SentinelGuard Communication Module
//!
//! Connects to the kernel minifilter driver via the Filter Manager
//! communication port and receives file-system telemetry events.
//!
//! Uses the `windows` crate to call FilterConnectCommunicationPort
//! and FilterGetMessage.

use crate::events::{FileEvent, OperationType};
use anyhow::Result;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

/// Size of the kernel event struct (must match driver's SG_EVENT)
/// Fields: StructSize(4) + Operation(4) + ProcessId(4) + Timestamp(8) + FileSize(8) +
///         FilePath(520*2) + NewFilePath(520*2) + ProcessName(260*2) + FileExtension(32*2)
const SG_MAX_PATH_LENGTH: usize = 520;
const SG_MAX_PROCESS_NAME: usize = 260;
const SG_MAX_EXTENSION: usize = 32;

/// Raw event structure matching the kernel driver's SG_EVENT layout.
/// Uses #[repr(C, packed(8))] to match the kernel's #pragma pack(push, 8).
#[repr(C)]
#[derive(Clone)]
struct RawSgEvent {
    struct_size: u32,
    operation: u32,
    process_id: u32,
    _padding: u32, // alignment padding before LARGE_INTEGER
    timestamp: i64,
    file_size: i64,
    file_path: [u16; SG_MAX_PATH_LENGTH],
    new_file_path: [u16; SG_MAX_PATH_LENGTH],
    process_name: [u16; SG_MAX_PROCESS_NAME],
    file_extension: [u16; SG_MAX_EXTENSION],
}

/// Filter message header size (matches FILTER_MESSAGE_HEADER)
const FILTER_MESSAGE_HEADER_SIZE: usize = 16; // ReplyLength(4) + MessageId(8) + padding

/// Manages the connection to the kernel driver communication port
pub struct DriverConnection {
    connected: Arc<AtomicBool>,
    event_counter: Arc<AtomicU64>,
}

impl DriverConnection {
    pub fn new() -> Self {
        Self {
            connected: Arc::new(AtomicBool::new(false)),
            event_counter: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Check if currently connected to the driver
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }

    /// Get total events received
    pub fn events_received(&self) -> u64 {
        self.event_counter.load(Ordering::Relaxed)
    }

    /// Start receiving events from the driver and send them to the channel.
    /// This runs in a background task and will retry connection on failure.
    pub async fn start_receiving(
        &self,
        port_name: String,
        sender: mpsc::Sender<FileEvent>,
        shutdown: tokio::sync::watch::Receiver<bool>,
    ) -> Result<()> {
        let connected = self.connected.clone();
        let event_counter = self.event_counter.clone();

        tokio::task::spawn_blocking(move || {
            Self::receive_loop(port_name, sender, shutdown, connected, event_counter);
        });

        Ok(())
    }

    fn receive_loop(
        port_name: String,
        sender: mpsc::Sender<FileEvent>,
        shutdown: tokio::sync::watch::Receiver<bool>,
        connected: Arc<AtomicBool>,
        event_counter: Arc<AtomicU64>,
    ) {
        use windows::core::PCWSTR;
        use windows::Win32::Storage::InstallableFileSystems::{
            FilterConnectCommunicationPort, FilterGetMessage,
        };

        loop {
            // Check shutdown
            if *shutdown.borrow() {
                info!("Driver communication shutting down");
                break;
            }

            // Convert port name to wide string
            let wide_port: Vec<u16> = port_name.encode_utf16().chain(std::iter::once(0)).collect();

            // Connect to the driver's communication port
            let port_handle = match unsafe {
                FilterConnectCommunicationPort(
                    PCWSTR(wide_port.as_ptr()),
                    0,        // Options
                    None,     // Context
                    0,        // SizeOfContext
                    None,     // SecurityAttributes
                )
            } {
                Ok(handle) => handle,
                Err(e) => {
                    warn!("Failed to connect to driver port '{}': {:?}. Retrying in 5s...", port_name, e);
                    connected.store(false, Ordering::Relaxed);
                    std::thread::sleep(std::time::Duration::from_secs(5));
                    continue;
                }
            };

            info!("Connected to driver communication port: {}", port_name);
            connected.store(true, Ordering::Relaxed);

            // Allocate buffer for receiving messages
            let msg_size = FILTER_MESSAGE_HEADER_SIZE + std::mem::size_of::<RawSgEvent>();
            let mut buffer = vec![0u8; msg_size];

            loop {
                // Check shutdown
                if *shutdown.borrow() {
                    break;
                }

                let get_result = unsafe {
                    FilterGetMessage(
                        port_handle,
                        buffer.as_mut_ptr() as *mut _,
                        msg_size as u32,
                        None, // No overlapped - blocking call
                    )
                };

                match get_result {
                    Ok(()) => {
                        // Parse the raw event from after the header
                        if buffer.len() >= FILTER_MESSAGE_HEADER_SIZE + std::mem::size_of::<RawSgEvent>() {
                            let raw_event: &RawSgEvent = unsafe {
                                &*(buffer[FILTER_MESSAGE_HEADER_SIZE..].as_ptr() as *const RawSgEvent)
                            };

                            if let Some(event) = Self::parse_raw_event(raw_event, &event_counter) {
                                if sender.blocking_send(event).is_err() {
                                    warn!("Event channel full or closed, dropping event");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        error!("FilterGetMessage failed: {:?}. Reconnecting...", e);
                        connected.store(false, Ordering::Relaxed);
                        // Close the handle and break to reconnect
                        unsafe {
                            windows::Win32::Foundation::CloseHandle(port_handle).ok();
                        }
                        break;
                    }
                }
            }

            // Small delay before reconnecting
            std::thread::sleep(std::time::Duration::from_secs(2));
        }
    }

    /// Parse a raw kernel event into a normalized FileEvent
    fn parse_raw_event(raw: &RawSgEvent, counter: &AtomicU64) -> Option<FileEvent> {
        let event_id = counter.fetch_add(1, Ordering::Relaxed);
        let operation = OperationType::from(raw.operation);

        // Convert wide strings to Rust Strings
        let file_path = wchar_to_string(&raw.file_path);
        let new_file_path = wchar_to_string(&raw.new_file_path);
        let process_name = wchar_to_string(&raw.process_name);
        let file_extension = wchar_to_string(&raw.file_extension);

        if file_path.is_empty() {
            debug!("Skipping event with empty file path");
            return None;
        }

        // Convert Windows FILETIME to nanoseconds since Unix epoch
        // Windows FILETIME is 100ns intervals since Jan 1, 1601
        // Unix epoch offset = 116444736000000000 (100ns units)
        const EPOCH_OFFSET: i64 = 116_444_736_000_000_000;
        let timestamp_ns = if raw.timestamp > EPOCH_OFFSET {
            ((raw.timestamp - EPOCH_OFFSET) * 100) as u64
        } else {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos() as u64
        };

        let new_file_path_opt = if new_file_path.is_empty() {
            None
        } else {
            Some(new_file_path)
        };

        Some(FileEvent {
            event_id,
            process_id: raw.process_id,
            process_name,
            operation,
            file_path,
            new_file_path: new_file_path_opt,
            file_size: raw.file_size as u64,
            entropy: 0.0, // Computed later by entropy detector
            timestamp_ns,
            file_extension,
        })
    }
}

/// Convert a null-terminated wide character buffer to a Rust String
fn wchar_to_string(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wchar_to_string() {
        let empty: [u16; 4] = [0, 0, 0, 0];
        assert_eq!(wchar_to_string(&empty), "");

        let hello: Vec<u16> = "hello".encode_utf16().chain(std::iter::once(0)).collect();
        assert_eq!(wchar_to_string(&hello), "hello");
    }

    #[test]
    fn test_driver_connection_creation() {
        let conn = DriverConnection::new();
        assert!(!conn.is_connected());
        assert_eq!(conn.events_received(), 0);
    }
}
