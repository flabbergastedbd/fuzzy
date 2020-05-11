use std::error::Error;

use tonic::Request;

use crate::models::PatchWorkerTask;
use crate::xpc::orchestrator_client::OrchestratorClient;

pub async fn mark_worker_task_active(worker_task_id: Option<i32>, connect_str: String) -> Result<(), Box<dyn Error>> {
    if worker_task_id.is_some() {
        let worker_task_id = worker_task_id.unwrap();
        let mut client = OrchestratorClient::connect(connect_str).await?;

        let patch_worker_task = PatchWorkerTask {
            id: worker_task_id,
            active: true
        };

        client.update_worker_task(Request::new(patch_worker_task)).await?;
    }
    Ok(())
}

pub async fn mark_worker_task_inactive(worker_task_id: Option<i32>, connect_str: String) -> Result<(), Box<dyn Error>> {
    if worker_task_id.is_some() {
        let worker_task_id = worker_task_id.unwrap();
        let mut client = OrchestratorClient::connect(connect_str).await?;

        let patch_worker_task = PatchWorkerTask {
            id: worker_task_id,
            active: false
        };

        client.update_worker_task(Request::new(patch_worker_task)).await?;
    }
    Ok(())
}


