//
// Kernel communication via Filter Manager communication port
//

use anyhow::{anyhow, Context, Result};
use std::ffi::c_void;
use std::os::windows::ffi::OsStringExt;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::config::CommunicationConfig;
use crate::events::{EventType, FileEvent};

const FLT_PORT_FLAG_SYNC_HANDLE: u32 = 0x00000001;

#[repr(C)]
struct FilterMessageHeader {
    reply_length: u32,
    message_id: u64,
}

#[repr(C)]
struct RawKernelEvent {
    event_type: u32,
    process_id: u32,
    process_path: [u16; 512],
    file_path: [u16; 1024],
    bytes_read: u64,
    bytes_written: u64,
    timestamp: u64,
    result: u32,
    entropy_preview: [u8; 16],
}

#[repr(C)]
struct KernelMessage {
    header: FilterMessageHeader,
    event: RawKernelEvent,
}

#[link(name = "FltLib")]
unsafe extern "system" {
    fn FilterConnectCommunicationPort(
        lpPortName: *const u16,
        dwOptions: u32,
        lpContext: *const c_void,
        wSizeOfContext: u16,
        lpSecurityAttributes: *const c_void,
        hPort: *mut isize,
    ) -> i32;

    fn FilterGetMessage(
        hPort: isize,
        lpMessageBuffer: *mut FilterMessageHeader,
        dwMessageBufferSize: u32,
        lpOverlapped: *mut c_void,
    ) -> i32;
}

pub struct KernelCommunication {
    event_tx: mpsc::UnboundedSender<FileEvent>,
    config: CommunicationConfig,
}

impl KernelCommunication {
    pub fn new(
        event_tx: mpsc::UnboundedSender<FileEvent>,
        config: CommunicationConfig,
    ) -> Result<Self> {
        Ok(Self { event_tx, config })
    }

    pub async fn start(&self) -> Result<()> {
        let event_tx = self.event_tx.clone();
        let config = self.config.clone();

        tokio::task::spawn_blocking(move || run_listener(event_tx, config))
            .await
            .context("Kernel communication worker panicked")?
    }
}

fn run_listener(
    event_tx: mpsc::UnboundedSender<FileEvent>,
    config: CommunicationConfig,
) -> Result<()> {
    loop {
        match connect_port(&config.port_name) {
            Ok(port) => {
                info!("Connected to kernel communication port {}", config.port_name);

                if let Err(error) = receive_loop(&port, &event_tx, config.buffer_size) {
                    warn!("Kernel communication loop ended: {}", error);
                }
            }
            Err(error) => {
                warn!(
                    "Failed to connect to kernel port {}: {}",
                    config.port_name, error
                );
            }
        }

        std::thread::sleep(Duration::from_secs(1));
    }
}

fn connect_port(port_name: &str) -> Result<OwnedHandle> {
    let wide_port_name = to_wide(port_name);
    let mut raw_handle: isize = 0;

    let hr = unsafe {
        FilterConnectCommunicationPort(
            wide_port_name.as_ptr(),
            FLT_PORT_FLAG_SYNC_HANDLE,
            std::ptr::null(),
            0,
            std::ptr::null(),
            &mut raw_handle,
        )
    };

    if hr < 0 {
        return Err(anyhow!(
            "FilterConnectCommunicationPort failed with HRESULT 0x{:08X}",
            hr as u32
        ));
    }

    if raw_handle == 0 {
        return Err(anyhow!("FilterConnectCommunicationPort returned a null handle"));
    }

    Ok(unsafe { OwnedHandle::from_raw_handle(raw_handle as *mut c_void) })
}

fn receive_loop(
    port: &OwnedHandle,
    event_tx: &mpsc::UnboundedSender<FileEvent>,
    buffer_size: usize,
) -> Result<()> {
    let message_size = std::mem::size_of::<KernelMessage>().max(buffer_size);
    let mut buffer = vec![0u8; message_size];

    loop {
        let header_ptr = buffer.as_mut_ptr() as *mut FilterMessageHeader;
        let hr = unsafe {
            FilterGetMessage(
                port.as_raw_handle() as isize,
                header_ptr,
                buffer.len() as u32,
                std::ptr::null_mut(),
            )
        };

        if hr < 0 {
            return Err(anyhow!(
                "FilterGetMessage failed with HRESULT 0x{:08X}",
                hr as u32
            ));
        }

        let message = unsafe { &*(buffer.as_ptr() as *const KernelMessage) };
        let event = decode_kernel_event(&message.event)?;
        debug!(process_id = event.process_id, event_type = ?event.event_type, "Received kernel event");

        if event_tx.send(event).is_err() {
            return Ok(());
        }
    }
}

fn decode_kernel_event(raw: &RawKernelEvent) -> Result<FileEvent> {
    Ok(FileEvent {
        event_type: decode_event_type(raw.event_type)?,
        process_id: raw.process_id,
        process_path: wide_string_to_string(&raw.process_path),
        file_path: wide_string_to_string(&raw.file_path),
        bytes_read: raw.bytes_read,
        bytes_written: raw.bytes_written,
        timestamp: raw.timestamp as i64,
        result: raw.result as i32,
        entropy_preview: raw
            .entropy_preview
            .iter()
            .copied()
            .filter(|byte| *byte != 0)
            .collect(),
    })
}

fn decode_event_type(raw_event_type: u32) -> Result<EventType> {
    match raw_event_type {
        0 => Ok(EventType::FileCreate),
        1 => Ok(EventType::FileRead),
        2 => Ok(EventType::FileWrite),
        3 => Ok(EventType::FileRename),
        4 => Ok(EventType::FileDelete),
        5 => Ok(EventType::DirectoryEnum),
        6 => Ok(EventType::VSSDelete),
        7 => Ok(EventType::ProcessCreate),
        8 => Ok(EventType::RegistryChange),
        _ => Err(anyhow!("Unknown kernel event type {}", raw_event_type)),
    }
}

fn wide_string_to_string(buffer: &[u16]) -> String {
    let len = buffer.iter().position(|value| *value == 0).unwrap_or(buffer.len());
    std::ffi::OsString::from_wide(&buffer[..len])
        .to_string_lossy()
        .into_owned()
}

fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}
