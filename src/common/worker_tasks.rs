use std::error::Error;

use tonic::Request;

use crate::common::xpc::get_orchestrator_client;
use crate::models::PatchWorkerTask;

pub async fn mark_worker_task_active(worker_task_id: Option<i32>) -> Result<(), Box<dyn Error>> {
    if worker_task_id.is_some() {
        let worker_task_id = worker_task_id.unwrap();
        let mut client = get_orchestrator_client().await?;

        let patch_worker_task = PatchWorkerTask {
            id: worker_task_id,
            running: true,
        };

        client.update_worker_task(Request::new(patch_worker_task)).await?;
    }
    Ok(())
}

pub async fn mark_worker_task_inactive(worker_task_id: Option<i32>) -> Result<(), Box<dyn Error>> {
    if worker_task_id.is_some() {
        let worker_task_id = worker_task_id.unwrap();
        let mut client = get_orchestrator_client().await?;

        let patch_worker_task = PatchWorkerTask {
            id: worker_task_id,
            running: false,
        };

        client.update_worker_task(Request::new(patch_worker_task)).await?;
    }
    Ok(())
}
