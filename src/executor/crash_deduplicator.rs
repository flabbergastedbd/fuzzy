use std::error::Error;
use std::time::UNIX_EPOCH;

use tracing::info;
use tonic::transport::channel::Channel;

use crate::common::{crashes::download_crashes, xpc::get_orchestrator_client};
use crate::executor;
use crate::fuzz_driver::CrashConfig;
use crate::utils::fs::rm_r;
use crate::xpc::{self, orchestrator_client::OrchestratorClient};

pub struct CrashDeduplicator {
    config: CrashConfig,
    worker_task_id: Option<i32>,
}

impl CrashDeduplicator {
    pub fn new(config: CrashConfig, worker_task_id: Option<i32>) -> Result<Self, Box<dyn Error>> {
        Ok(Self { config, worker_task_id })
    }

    async fn get_task_id(
        worker_task_id: Option<i32>,
        client: &mut OrchestratorClient<Channel>,
    ) -> Result<Option<i32>, Box<dyn Error>> {
        let mut task_id = None;
        if let Some(worker_task_id) = worker_task_id {
            let id = xpc::Id { value: worker_task_id };
            let wtask = client.fetch_worker_task(id).await?.into_inner();
            task_id = Some(wtask.task.id);
        }
        Ok(task_id)
    }

    // Returns id of crash if this is duplicate of
    pub async fn dedup_crash(&self, output: &str) -> Result<Option<i32>, Box<dyn Error>> {
        if let Some(mut exec_config) = self.config.deduplicate.clone() {
            // Get task id first
            let mut client = get_orchestrator_client().await?;
            let task_id = Self::get_task_id(self.worker_task_id, &mut client).await?;

            // Already present crashes as sent as arg[-2]
            let existing_output_filename = "original.fuzzy";
            exec_config.args.push(existing_output_filename.to_owned());

            // Create a temporary name that will be passed as argv[-1]
            let new_output_filename = "crash.fuzzy";
            exec_config.args.push(new_output_filename.to_owned());

            // Get all non duplicate crashes
            let crashes =
                download_crashes(None, Some(true), None, task_id, None, UNIX_EPOCH, false, &mut client).await?;
            let mut dup_crash_id = None;

            // Create executor to be used repeatedly
            let mut executor = executor::new(exec_config, self.worker_task_id);
            executor.setup().await?;

            // Create paths to write contents to
            let cwd = executor.get_cwd_path();
            let new_output_path = cwd.join(new_output_filename);
            tokio::fs::write(new_output_path.as_path(), output).await?; // Since this is done only once
            let existing_output_path = cwd.join(existing_output_filename);

            // Copy new output file into cwd of validate
            for crash in crashes {
                if let Some(output) = crash.output.clone() {
                    // Write output to crash file
                    tokio::fs::write(&existing_output_path, output).await?;

                    // Launch command
                    let output = executor.spawn_blocking().await?;

                    // Zero exit code means duplicate just like diff command
                    if output.status.success() == true {
                        dup_crash_id = Some(crash.id);
                        break;
                    }
                }
            }

            // Remove cwd
            rm_r(&cwd).await?;
            Ok(dup_crash_id)
        } else {
            info!("Not deduplicating crash {:?} as no validate in profile", output);
            Ok(None)
        }
    }
}
