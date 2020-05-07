use data_encoding::HEXUPPER;
use log::{error, info};
use ring::digest;
use tokio::fs::File;
use tokio::prelude::*;
use tonic::{Request, transport::channel::Channel};

use crate::models::NewCorpus;
use crate::xpc::orchestrator_client::OrchestratorClient;

pub fn checksum(bytes: &Vec<u8>) -> String {
    let actual = digest::digest(&digest::SHA256, bytes);
    HEXUPPER.encode(actual.as_ref())
}

// Corpus related utilities
pub async fn upload_corpus(
        file_path: String,
        label: String,
        client: &mut OrchestratorClient<Channel>) {

    let mut content = vec![];
    let file = File::open(file_path.clone()).await;

    if let Err(e) = file {
        error!("Unable to process file {}: {}", file_path, e);
        return
    } else {
        let mut file = file.unwrap();
        if let Err(e) = file.read_to_end(&mut content).await {
            error!("Unable to process file {}: {}", file_path, e);
            return
        }

        // Generate checksum
        let checksum = checksum(&content);

        // Send request
        let new_corpus = NewCorpus {
            content,
            checksum,
            label,
        };

        let response = client.submit_corpus(Request::new(new_corpus)).await;
        if let Err(e) = response {
            error!("Failed to add {}: {:?}", file_path, e);
        } else {
            info!("Successfully added: {}", file_path);
        }
    }
}
