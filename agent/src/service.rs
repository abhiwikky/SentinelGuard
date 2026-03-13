use anyhow::{Context, Result};
use std::ffi::OsString;
use std::time::Duration;
use tokio::runtime::Builder;
use tokio::sync::watch;
use windows_service::define_windows_service;
use windows_service::Error as WindowsServiceError;
use windows_service::service::{
    ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus, ServiceType,
};
use windows_service::service_control_handler::{self, ServiceControlHandlerResult};
use windows_service::service_dispatcher;

const SERVICE_NAME: &str = "SentinelGuardAgent";

define_windows_service!(ffi_service_main, service_main);

pub fn try_run_service() -> Result<bool> {
    match service_dispatcher::start(SERVICE_NAME, ffi_service_main) {
        Ok(()) => Ok(true),
        Err(WindowsServiceError::Winapi(error)) if error.raw_os_error() == Some(1063) => Ok(false),
        Err(error) => Err(error).context("Failed to start service dispatcher"),
    }
}

fn service_main(_arguments: Vec<OsString>) {
    if let Err(error) = run_service() {
        eprintln!("SentinelGuard service failed: {error}");
    }
}

fn run_service() -> Result<()> {
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let status_handle = service_control_handler::register(SERVICE_NAME, move |control_event| match control_event {
        ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
        ServiceControl::Stop | ServiceControl::Shutdown => {
            let _ = shutdown_tx.send(true);
            ServiceControlHandlerResult::NoError
        }
        _ => ServiceControlHandlerResult::NotImplemented,
    })
    .context("Failed to register Windows service control handler")?;

    status_handle
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::StartPending,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 1,
            wait_hint: Duration::from_secs(15),
            process_id: None,
        })
        .context("Failed to report StartPending service status")?;

    let runtime = Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("Failed to create Tokio runtime for Windows service")?;

    status_handle
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })
        .context("Failed to report Running service status")?;

    let service_result = runtime.block_on(crate::run_agent(crate::ShutdownMode::Service(shutdown_rx)));

    let exit_code = if service_result.is_ok() { 0 } else { 1 };
    status_handle
        .set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(exit_code),
            checkpoint: 0,
            wait_hint: Duration::default(),
            process_id: None,
        })
        .context("Failed to report Stopped service status")?;

    service_result
}
