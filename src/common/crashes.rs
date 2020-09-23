use std::error::Error;
use std::path::Path;
use std::time::SystemTime;

use tracing::debug;
use tokio::fs;
use tonic::{transport::channel::Channel, Request};

use crate::models::{Crash, NewCrash, PatchCrash};
use crate::utils::{
    checksum,
    fs::{mkdir_p, read_file},
};
use crate::xpc::{self, orchestrator_client::OrchestratorClient};

// Corpus related utilities
pub async fn upload_crash_from_disk(
    file_path: &Path,
    label: String,
    verified: bool,
    output: Option<String>,
    worker_task_id: Option<i32>,
    duplicate: Option<i32>,
    client: &mut OrchestratorClient<Channel>,
) -> Result<(), Box<dyn Error>> {
    debug!("Trying to upload {:?} to crashes", file_path);
    let content = read_file(file_path).await?;

    // Generate checksum
    let checksum = checksum(&content);

    // Send request
    let new_crash = NewCrash {
        content,
        checksum,
        label,
        verified,
        output,
        worker_task_id,
        duplicate,
    };

    client.submit_crash(Request::new(new_crash)).await?;
    Ok(())
}

pub async fn update_crash(
    id: i32,
    verified: bool,
    output: Option<String>,
    duplicate: Option<i32>,
    client: &mut OrchestratorClient<Channel>,
) -> Result<(), Box<dyn Error>> {
    // Send request
    let patch_crash = PatchCrash {
        id,
        verified,
        output,
        duplicate,
    };

    client.update_crash(Request::new(patch_crash)).await?;
    Ok(())
}

pub async fn download_crashes(
    label: Option<String>,
    verified: Option<bool>,
    output: Option<String>,
    task_id: Option<i32>,
    latest: Option<i64>,
    created_after: SystemTime,
    duplicate: bool,
    client: &mut OrchestratorClient<Channel>,
) -> Result<Vec<Crash>, Box<dyn Error>> {
    let filter_request = xpc::FilterCrash {
        label,
        verified,
        output,
        task_id,
        latest,
        created_after: prost_types::Timestamp::from(created_after),
        duplicate,
    };
    let response = client.get_crashes(Request::new(filter_request)).await?;
    Ok(response.into_inner().data)
}

pub async fn download_crashes_to_disk(
    label: Option<String>,
    verified: Option<bool>,
    output: Option<String>,
    task_id: Option<i32>,
    latest: Option<i64>,
    created_after: SystemTime,
    duplicate: bool,
    dir: &Path,
    client: &mut OrchestratorClient<Channel>,
) -> Result<usize, Box<dyn Error>> {
    debug!("Trying to download crashes to {:?}", dir);

    mkdir_p(dir).await?;

    let crashes = download_crashes(
        label,
        verified,
        output,
        task_id,
        latest,
        created_after,
        duplicate,
        client,
    )
    .await?;

    for crash in crashes.iter() {
        let mut crash_path = dir.join(&crash.checksum);
        let mut crash_output = crash_path.clone();
        let mut crash_verified = crash_path.clone();

        crash_path.set_extension("crash");
        fs::write(crash_path, &crash.content).await?;

        if crash.verified {
            crash_verified.set_extension("verified");
            fs::write(crash_verified, "").await?;
        }

        if let Some(output) = &crash.output {
            crash_output.set_extension("output");
            fs::write(crash_output, output).await?;
        }
    }

    Ok(crashes.len())
}
