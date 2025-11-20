//
// Quarantine Controller
//

use anyhow::Result;
use std::path::PathBuf;
use std::process::Command;
use tracing::{info, error};

pub struct QuarantineController {
    quarantine_exe_path: PathBuf,
}

impl QuarantineController {
    pub fn new(quarantine_path: &PathBuf) -> Result<Self> {
        Ok(Self {
            quarantine_exe_path: quarantine_path.clone(),
        })
    }

    pub async fn quarantine_process(&self, process_id: u32) -> Result<()> {
        info!("Quarantining process: {}", process_id);

        // Call C++ quarantine executable
        let output = Command::new(&self.quarantine_exe_path)
            .arg("--suspend")
            .arg(process_id.to_string())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Quarantine failed: {}", stderr);
            return Err(anyhow::anyhow!("Quarantine command failed: {}", stderr));
        }

        info!("Process {} quarantined successfully", process_id);
        Ok(())
    }

    pub async fn release_process(&self, process_id: u32) -> Result<()> {
        info!("Releasing process from quarantine: {}", process_id);

        // Call C++ quarantine executable with release flag
        let output = Command::new(&self.quarantine_exe_path)
            .arg("--release")
            .arg(process_id.to_string())
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("Release failed: {}", stderr);
            return Err(anyhow::anyhow!("Release command failed: {}", stderr));
        }

        info!("Process {} released from quarantine", process_id);
        Ok(())
    }
}

