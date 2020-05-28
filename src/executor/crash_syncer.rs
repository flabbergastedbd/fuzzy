use std::error::Error;

use log::{info, error, debug};
use tonic::transport::channel::Channel;
use tokio::sync::broadcast;

use crate::xpc::orchestrator_client::OrchestratorClient;
use crate::fuzz_driver::CrashConfig;
use crate::common::crashes::upload_crash_from_disk;
use crate::common::xpc::get_orchestrator_client;

/// A file system corpus syncer. Need to convert this into trait when implementing docker
pub struct CrashSyncer {
    config: CrashConfig,
    worker_task_id: Option<i32>,
}

impl CrashSyncer {
    pub fn new(config: CrashConfig, worker_task_id: Option<i32>) -> Result<Self, Box<dyn Error>> {
        Ok(Self { config, worker_task_id })
    }

    pub async fn upload_crashes(
            &self,
            mut kill_switch: broadcast::Receiver<u8>
        ) -> Result<(), Box<dyn Error>> {

        debug!("Will try to keep crashes in sync at: {:?}", self.config.path);
        let client = get_orchestrator_client().await?;

        // Create necessary clones and pass along for upload sync if upload enabled
        tokio::select! {
            result = self.upload(client) => {
                error!("Crash upload sync job failed: {:?}", result);
            },
            _ = kill_switch.recv() => {}
        }

        // crash_sync_handle.await?;

        Ok(())
    }

    #[cfg(target_os = "linux")]
    async fn upload(
            &self,
            client: OrchestratorClient<Channel>) -> Result<(), Box<dyn Error>> {
        let mut client = client;
        info!("Creating crash upload sync");
        let mut watcher = crate::utils::fs::InotifyFileWatcher::new(&self.config.path, Some(self.config.filter.clone()))?;
        let validator = super::crash_validator::CrashValidator::new(self.config.clone(), self.worker_task_id)?;

        while let Some(file) = watcher.get_new_file().await {
            // Match user provided match pattern
            let file_path = self.config.path.clone();
            let file_path = file_path.join(file);

            // Verify crash if profile mandates it
            let (output, verified) = match validator.validate_crash(file_path.as_path()).await {
                Ok((output, verified)) => {
                    (output, verified)
                },
                Err(e) => {
                    error!("Unable to validate crash {:?} due to error: {}", file_path, e);
                    (None, false)
                },
            };

            info!("Uploading new crash: {:?}", file_path);
            upload_crash_from_disk(
                file_path.as_path(),
                self.config.label.clone(),
                verified,
                output,
                self.worker_task_id,
                &mut client
            ).await?
        }
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    async fn upload(
            &self,
            client: OrchestratorClient<Channel>) -> Result<(), Box<dyn Error>> {
        error!("Crash syncer is not ported yet to work on non linux systems");
        Ok(())
    }
}
