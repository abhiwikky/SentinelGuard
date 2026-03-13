//
// Security Hardening Module
// Tamper detection, process protection, integrity checks
//

use anyhow::{anyhow, Result};
use ring::digest;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tracing::{debug, error, warn};

#[cfg(windows)]
use windows::Win32::Foundation::BOOL;
#[cfg(windows)]
use windows::Win32::System::Diagnostics::Debug::{CheckRemoteDebuggerPresent, IsDebuggerPresent};
#[cfg(windows)]
use windows::Win32::System::SystemServices::{
    PROCESS_MITIGATION_DYNAMIC_CODE_POLICY, PROCESS_MITIGATION_DYNAMIC_CODE_POLICY_0,
    PROCESS_MITIGATION_EXTENSION_POINT_DISABLE_POLICY,
    PROCESS_MITIGATION_EXTENSION_POINT_DISABLE_POLICY_0,
    PROCESS_MITIGATION_STRICT_HANDLE_CHECK_POLICY,
    PROCESS_MITIGATION_STRICT_HANDLE_CHECK_POLICY_0,
};
#[cfg(windows)]
use windows::Win32::System::Threading::{
    GetCurrentProcess, ProcessDynamicCodePolicy, ProcessExtensionPointDisablePolicy,
    ProcessStrictHandleCheckPolicy, SetProcessMitigationPolicy,
};

pub struct SecurityModule {
    agent_path: PathBuf,
    agent_hash: Vec<u8>,
    config_path: Option<PathBuf>,
    config_hash: Option<Vec<u8>>,
    driver_path: Option<PathBuf>,
    driver_hash: Option<Vec<u8>>,
    monitor_started: Arc<AtomicBool>,
}

impl SecurityModule {
    pub fn new() -> Result<Self> {
        let agent_path = std::env::current_exe()?;
        let agent_hash = Self::calculate_file_hash(&agent_path)?;
        let config_path = discover_config_path();
        let config_hash = config_path
            .as_ref()
            .map(Self::calculate_file_hash)
            .transpose()?;
        let driver_path = discover_driver_path();
        let driver_hash = driver_path
            .as_ref()
            .filter(|path| path.exists())
            .map(Self::calculate_file_hash)
            .transpose()?;

        debug!("Agent binary hash calculated: {:x?}", agent_hash);

        Ok(Self {
            agent_path,
            agent_hash,
            config_path,
            config_hash,
            driver_path,
            driver_hash,
            monitor_started: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn verify_agent_integrity(&self) -> Result<bool> {
        let current_hash = Self::calculate_file_hash(&self.agent_path)?;
        if current_hash != self.agent_hash {
            error!("Agent binary integrity check failed");
            return Ok(false);
        }

        Ok(true)
    }

    pub fn verify_config_integrity(&self) -> Result<bool> {
        match (&self.config_path, &self.config_hash) {
            (Some(config_path), Some(stored_hash)) => {
                let current_hash = Self::calculate_file_hash(config_path)?;
                if &current_hash != stored_hash {
                    warn!("Config file integrity check failed");
                    return Ok(false);
                }
                Ok(true)
            }
            _ => Ok(true),
        }
    }

    pub fn protect_process(&self) -> Result<()> {
        self.ensure_not_debugged()?;

        #[cfg(windows)]
        {
            unsafe {
                let strict_handle_policy = PROCESS_MITIGATION_STRICT_HANDLE_CHECK_POLICY {
                    Anonymous: PROCESS_MITIGATION_STRICT_HANDLE_CHECK_POLICY_0 { Flags: 0b11 },
                };
                SetProcessMitigationPolicy(
                    ProcessStrictHandleCheckPolicy,
                    &strict_handle_policy as *const _ as *const _,
                    std::mem::size_of::<PROCESS_MITIGATION_STRICT_HANDLE_CHECK_POLICY>(),
                )?;

                let extension_point_policy = PROCESS_MITIGATION_EXTENSION_POINT_DISABLE_POLICY {
                    Anonymous: PROCESS_MITIGATION_EXTENSION_POINT_DISABLE_POLICY_0 { Flags: 0b1 },
                };
                SetProcessMitigationPolicy(
                    ProcessExtensionPointDisablePolicy,
                    &extension_point_policy as *const _ as *const _,
                    std::mem::size_of::<PROCESS_MITIGATION_EXTENSION_POINT_DISABLE_POLICY>(),
                )?;

                let dynamic_code_policy = PROCESS_MITIGATION_DYNAMIC_CODE_POLICY {
                    Anonymous: PROCESS_MITIGATION_DYNAMIC_CODE_POLICY_0 { Flags: 0b1 },
                };
                SetProcessMitigationPolicy(
                    ProcessDynamicCodePolicy,
                    &dynamic_code_policy as *const _ as *const _,
                    std::mem::size_of::<PROCESS_MITIGATION_DYNAMIC_CODE_POLICY>(),
                )?;
            }
        }

        debug!("Process protection enabled");
        Ok(())
    }

    pub fn enable_tamper_detection(&self) -> Result<()> {
        if self.monitor_started.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        let agent_path = self.agent_path.clone();
        let agent_hash = self.agent_hash.clone();
        let config_path = self.config_path.clone();
        let config_hash = self.config_hash.clone();
        let driver_path = self.driver_path.clone();
        let driver_hash = self.driver_hash.clone();

        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(30));

            if let Ok(current_hash) = SecurityModule::calculate_file_hash(&agent_path) {
                if current_hash != agent_hash {
                    error!("Agent binary changed on disk after startup");
                }
            }

            if let (Some(config_path), Some(config_hash)) = (&config_path, &config_hash) {
                match SecurityModule::calculate_file_hash(config_path) {
                    Ok(current_hash) if current_hash != *config_hash => {
                        error!("Config file changed on disk after startup");
                    }
                    Err(err) => warn!("Config integrity check failed: {}", err),
                    _ => {}
                }
            }

            if let (Some(driver_path), Some(driver_hash)) = (&driver_path, &driver_hash) {
                match SecurityModule::calculate_file_hash(driver_path) {
                    Ok(current_hash) if current_hash != *driver_hash => {
                        error!("Driver binary changed on disk after startup");
                    }
                    Err(err) => warn!("Driver integrity check failed: {}", err),
                    _ => {}
                }
            }
        });

        debug!("Tamper detection enabled");
        Ok(())
    }

    fn ensure_not_debugged(&self) -> Result<()> {
        #[cfg(windows)]
        unsafe {
            if IsDebuggerPresent().as_bool() {
                return Err(anyhow!("Debugger attached to SentinelGuard agent"));
            }

            let process = GetCurrentProcess();
            let mut debugger_present = BOOL(0);
            CheckRemoteDebuggerPresent(process, &mut debugger_present)?;
            if debugger_present.as_bool() {
                return Err(anyhow!("Remote debugger attached to SentinelGuard agent"));
            }
        }

        Ok(())
    }

    fn calculate_file_hash(path: &PathBuf) -> Result<Vec<u8>> {
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        let hash = digest::digest(&digest::SHA256, &buffer);
        Ok(hash.as_ref().to_vec())
    }

    pub fn verify_driver_integrity(&self) -> Result<bool> {
        match (&self.driver_path, &self.driver_hash) {
            (Some(driver_path), Some(stored_hash)) => {
                let current_hash = Self::calculate_file_hash(driver_path)?;
                Ok(current_hash == *stored_hash)
            }
            _ => Ok(false),
        }
    }
}

fn discover_config_path() -> Option<PathBuf> {
    let exe_path = std::env::current_exe().ok()?;
    let exe_dir = exe_path.parent()?;

    let candidates = [
        exe_dir.join("config").join("config.toml"),
        exe_dir.join("agent").join("config").join("config.toml"),
        std::env::current_dir().ok()?.join("config").join("config.toml"),
        std::env::current_dir().ok()?.join("agent").join("config").join("config.toml"),
    ];

    candidates.into_iter().find(|candidate| candidate.exists())
}

fn discover_driver_path() -> Option<PathBuf> {
    let exe_path = std::env::current_exe().ok()?;
    let exe_dir = exe_path.parent()?;

    let candidates = [
        exe_dir.join("SentinelGuard.sys"),
        exe_dir.join("kernel").join("build").join("Release").join("SentinelGuard.sys"),
        std::env::current_dir().ok()?.join("kernel").join("build").join("Release").join("SentinelGuard.sys"),
    ];

    candidates.into_iter().find(|candidate| candidate.exists())
}
