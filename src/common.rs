use std::path::Path;
use std::time::SystemTime;
use std::error::Error;

use data_encoding::HEXUPPER;
use log::{error, debug, info};
use ring::digest;
use tokio::fs::File;
use tokio::prelude::*;
use tonic::{Request, transport::channel::Channel};

use crate::models::{NewCorpus, Corpus};
use crate::xpc::{self, orchestrator_client::OrchestratorClient};

pub fn checksum(bytes: &Vec<u8>) -> String {
    let actual = digest::digest(&digest::SHA256, bytes);
    HEXUPPER.encode(actual.as_ref())
}

pub async fn read_file(file_path: &Path) -> Result<Vec<u8>, Box<dyn Error>> {
    debug!("Reading full file");
    let mut content = vec![];
    let mut file = File::open(file_path).await?;
    file.read_to_end(&mut content).await?;
    Ok(content)
}

// Corpus related utilities
pub async fn upload_corpus(
        file_path: &Path,
        label: String,
        worker_task_id: Option<i32>,
        client: &mut OrchestratorClient<Channel>) {

    let content = read_file(file_path).await;

    if let Err(e) = content {
        error!("Unable to upload provided corpus {:?}: {}", file_path, e);
        return
    }

    let content = content.unwrap();
    // Generate checksum
    let checksum = checksum(&content);

    // Send request
    let new_corpus = NewCorpus {
        content,
        checksum,
        label,
        worker_task_id,
    };

    let response = client.submit_corpus(Request::new(new_corpus)).await;
    if let Err(e) = response {
        error!("Failed to add {:?}: {:?}", file_path, e);
    } else {
        info!("Successfully added: {:?}", file_path);
    }
}

pub async fn download_corpus(
        label: String,
        worker_task_id: Option<i32>,
        created_after: SystemTime,
        client: &mut OrchestratorClient<Channel>) -> Result<Vec<Corpus>, Box<dyn Error>> {
    debug!("Getting corpus");

    let filter_corpus = xpc::FilterCorpus {
        label,
        created_after: prost_types::Timestamp::from(created_after),
        worker_task_id,
    };
    let response = client.get_corpus(Request::new(filter_corpus)).await?;
    Ok(response.into_inner().data)
}
