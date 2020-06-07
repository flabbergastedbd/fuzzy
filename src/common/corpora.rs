use std::error::Error;
use std::io;
use std::path::Path;
use std::time::SystemTime;

use log::{debug, warn};
use tokio::fs;
use tonic::{transport::channel::Channel, Request};

use crate::models::{Corpus, NewCorpus};
use crate::utils::{checksum, fs::read_file};
use crate::xpc::{self, orchestrator_client::OrchestratorClient};

pub const CORPUS_FILE_EXT: &str = "fuzzy";

// Corpus related utilities
pub async fn upload_corpus_from_disk(
    file_path: &Path,
    label: String,
    worker_task_id: Option<i32>,
    client: &mut OrchestratorClient<Channel>,
) -> Result<(), Box<dyn Error>> {
    debug!("Trying to upload {:?} to corpus", file_path);
    let content = read_file(file_path).await?;

    // Generate checksum
    let checksum = checksum(&content);

    // Send request
    let new_corpus = NewCorpus {
        content,
        checksum,
        label,
        worker_task_id,
    };

    client.submit_corpus(Request::new(new_corpus)).await?;
    Ok(())
}

pub async fn download_corpus(
    label: String,
    not_worker_task_id: Option<i32>,
    for_worker_task_id: Option<i32>,
    latest: Option<i64>,
    created_after: SystemTime,
    client: &mut OrchestratorClient<Channel>,
) -> Result<Vec<Corpus>, Box<dyn Error>> {
    debug!(
        "Downloading corpus with label {} updated after {:?} for worker_task_id {:?}",
        label, created_after, not_worker_task_id
    );

    let filter_corpus = xpc::FilterCorpus {
        label,
        created_after: prost_types::Timestamp::from(created_after),
        not_worker_task_id,
        for_worker_task_id,
        latest,
    };
    let response = client.get_corpus(Request::new(filter_corpus)).await?;
    Ok(response.into_inner().data)
}

pub async fn delete_corpus(
    label: String,
    not_worker_task_id: Option<i32>,
    for_worker_task_id: Option<i32>,
    latest: Option<i64>,
    created_after: SystemTime,
    client: &mut OrchestratorClient<Channel>,
) -> Result<(), Box<dyn Error>> {
    warn!("Deleting corpus with label {} updated after {:?}", label, created_after);

    let filter_corpus = xpc::FilterCorpus {
        label,
        created_after: prost_types::Timestamp::from(created_after),
        not_worker_task_id,
        for_worker_task_id,
        latest,
    };
    let _ = client.delete_corpus(Request::new(filter_corpus)).await?;
    Ok(())
}

pub async fn download_corpus_to_disk(
    label: String,
    not_worker_task_id: Option<i32>,
    for_worker_task_id: Option<i32>,
    limit: Option<i64>,
    created_after: SystemTime,
    dir: &Path,
    client: &mut OrchestratorClient<Channel>,
) -> Result<usize, Box<dyn Error>> {
    let corpora = download_corpus(
        label,
        not_worker_task_id,
        for_worker_task_id,
        limit,
        created_after,
        client,
    )
    .await?;

    // Check if exists, if not create. If exists and not a directory, Err
    if dir.exists() == false {
        fs::create_dir_all(dir).await?;
    } else if dir.is_dir() == false {
        return Err(Box::new(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{:?} is not a directory to download corpus", dir),
        )));
    }

    for corpus in corpora.iter() {
        let mut file_path = dir.clone().join(&corpus.checksum);
        file_path.set_extension(CORPUS_FILE_EXT);
        fs::write(file_path, &corpus.content).await?;
    }

    debug!("Written {} corpus files to {:?}", corpora.len(), dir);

    Ok(corpora.len())
}
