use std::path::Path;
use std::error::Error;

use log::debug;
use tonic::{Request, transport::channel::Channel};

use crate::models::NewCrash;
use crate::xpc::orchestrator_client::OrchestratorClient;
use crate::utils::{checksum, fs::read_file};

// Corpus related utilities
pub async fn upload_crash_from_disk(file_path: &Path,
                           label: String,
                           worker_task_id: Option<i32>,
                           client: &mut OrchestratorClient<Channel>) -> Result<(), Box<dyn Error>> {

    debug!("Trying to upload {:?} to crashes", file_path);
    let content = read_file(file_path).await?;

    // Generate checksum
    let checksum = checksum(&content);

    // Send request
    let new_crash = NewCrash {
        content,
        checksum,
        label,
        worker_task_id,
        // Verification happens somewhere else not on worker for now
        verified: false,
    };

    client.submit_crash(Request::new(new_crash)).await?;
    Ok(())
}
