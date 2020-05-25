use std::time::SystemTime;
use std::path::Path;
use std::error::Error;

use log::debug;
use tonic::{Request, transport::channel::Channel};
use tokio::fs;

use crate::models::{NewCrash, Crash};
use crate::xpc::{self, orchestrator_client::OrchestratorClient};
use crate::utils::{checksum, fs::{mkdir_p, read_file}};

// Corpus related utilities
pub async fn upload_crash_from_disk(file_path: &Path,
                           label: String,
                           verified: bool,
                           output: Option<String>,
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
        verified,
        output,
        worker_task_id,
    };

    client.submit_crash(Request::new(new_crash)).await?;
    Ok(())
}

pub async fn download_crashes(
    label: String,
    verified: Option<bool>,
    output: Option<String>,
    task_id: Option<i32>,
    latest: Option<i64>,
    created_after: SystemTime,
    client: &mut OrchestratorClient<Channel>) -> Result<Vec<Crash>, Box<dyn Error>> {

    let filter_request = xpc::FilterCrash {
        label,
        verified,
        output,
        task_id,
        latest,
        created_after: prost_types::Timestamp::from(created_after)
    };
    let response = client.get_crashes(Request::new(filter_request)).await?;
    Ok(response.into_inner().data)
}

pub async fn download_crashes_to_disk(
    label: String,
    verified: Option<bool>,
    output: Option<String>,
    task_id: Option<i32>,
    latest: Option<i64>,
    created_after: SystemTime,
    dir: &Path,
    client: &mut OrchestratorClient<Channel>) -> Result<usize, Box<dyn Error>> {
    debug!("Trying to download crashes to {:?}", dir);

    mkdir_p(dir).await?;

    let crashes = download_crashes(label, verified, output, task_id, latest, created_after, client).await?;

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
