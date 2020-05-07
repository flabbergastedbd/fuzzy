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

pub async fn read_file(file_path: String) -> Result<Vec<u8>, Box<dyn Error>> {
    debug!("Reading full file");
    let mut content = vec![];
    let mut file = File::open(file_path).await?;
    file.read_to_end(&mut content).await?;
    Ok(content)
}

// Corpus related utilities
pub async fn upload_corpus(
        file_path: String,
        label: String,
        client: &mut OrchestratorClient<Channel>) {

    let content = read_file(file_path.clone()).await;

    if let Err(e) = content {
        error!("Unable to upload provided corpus {}: {}", file_path, e);
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
    };

    let response = client.submit_corpus(Request::new(new_corpus)).await;
    if let Err(e) = response {
        error!("Failed to add {}: {:?}", file_path, e);
    } else {
        info!("Successfully added: {}", file_path);
    }
}

pub async fn get_corpus(
        label: String,
        client: &mut OrchestratorClient<Channel>) -> Vec<Corpus> {
    debug!("Getting corpus");

    let filter_corpus = xpc::FilterCorpus { label };
    let response = client.get_corpus(Request::new(filter_corpus)).await;
    if let Err(e) = response {
        error!("Failed to get corpus: {}", e);
        return vec![]
    } else {
        response.unwrap().into_inner().data
    }
}
