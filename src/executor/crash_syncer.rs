use std::path::Path;
use std::error::Error;

use log::{info, error, debug};
use tonic::transport::channel::Channel;
use tokio::sync::broadcast;

use crate::xpc::orchestrator_client::OrchestratorClient;
use crate::executor;
use crate::utils::fs::InotifyFileWatcher;
use crate::fuzz_driver::CrashConfig;
use crate::common::crashes::upload_crash_from_disk;
use crate::common::xpc::get_orchestrator_client;
use crate::utils::fs::rm_r;

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

    async fn upload(
            &self,
            client: OrchestratorClient<Channel>) -> Result<(), Box<dyn Error>> {
        let mut client = client;
        info!("Creating crash upload sync");
        let mut watcher = InotifyFileWatcher::new(&self.config.path, Some(self.config.filter.clone()))?;

        while let Some(file) = watcher.get_new_file().await {
            // Match user provided match pattern
            let file_path = self.config.path.clone();
            let file_path = file_path.join(file);

            // Verify crash if profile mandates it
            let (output, verified) = match self.validate_crash(file_path.as_path()).await {
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

    async fn validate_crash(&self, crash: &Path) -> Result<(Option<String>, bool), Box<dyn Error>> {
        if let Some(mut exec_config) = self.config.validate.clone() {
            // Create a temporary name that will be passed as argv[-1]
            let relative_file_name = "crash.fuzzy";
            exec_config.args.push(relative_file_name.to_owned());

            // Create executor
            let mut executor = executor::new(exec_config, self.worker_task_id);
            executor.setup().await?;

            // Copy crash file into cwd of validate
            let cwd = executor.get_cwd_path();
            let temp_path = cwd.join(relative_file_name);
            // Copy file into cwd
            tokio::fs::copy(crash, temp_path.as_path()).await?;

            let output = executor.spawn_blocking().await?;

            // Any non zero exit code, we mark crash as verified
            let verified = output.status.success() == false;

            let stdout = std::str::from_utf8(&output.stdout)?;
            let stderr = std::str::from_utf8(&output.stderr)?;

            // Join stdout/stderr for now as output
            let output = format!("STDOUT\n\
                                  ------\n\
                                  {}\n\
                                  STDERR\n\
                                  ------\n\
                                  {}\n", stdout, stderr
            );

            rm_r(&cwd).await?;
            Ok((Some(output), verified))
        } else {
            info!("Not validating crash {:?} as no validate in profile", crash);
            Ok((None, false))
        }
    }
}
