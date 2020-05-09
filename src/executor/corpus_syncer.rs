use std::error::Error;
use std::time::{UNIX_EPOCH, SystemTime, Duration};

use regex::Regex;
use log::{info, error, debug};
use tokio::task;
use tonic::transport::channel::Channel;

use crate::xpc::orchestrator_client::OrchestratorClient;
use crate::utils::fs::InotifyFileWatcher;
use super::CorpusConfig;
use crate::common::corpora::{upload_corpus_from_disk, download_corpus_to_disk, CORPUS_FILE_EXT};

pub struct CorpusSyncer {
    config: CorpusConfig,
    worker_task_id: Option<i32>,
}

impl CorpusSyncer {
    pub fn new(config: CorpusConfig, worker_task_id: Option<i32>) -> Result<Self, Box<dyn Error>> {
        Ok(Self { config, worker_task_id })
    }

    pub async fn setup_corpus(&self, connect_addr: String) -> Result<(), Box<dyn Error>> {
        debug!("Syncing initial corpus");
        let mut client = OrchestratorClient::connect(connect_addr).await?;
        download_corpus_to_disk(
            self.config.label.clone(),
            self.worker_task_id,
            UNIX_EPOCH,
            &self.config.path,
            &mut client).await?;
        Ok(())
    }

    pub async fn sync_corpus(
            &self,
            connect_addr: String,
        ) -> Result<(), Box<dyn Error>> {

        debug!("Will try to keep corpus in sync at: {:?}", self.config.path);
        let mut client = OrchestratorClient::connect(connect_addr).await?;
        let worker_task_id = self.worker_task_id;


        // Create a local set
        let local_set = task::LocalSet::new();

        // Create necessary clones and pass along for upload sync if upload enabled
        if self.config.upload {
            let client_clone = client.clone();
            let corpus_config = self.config.clone();
            local_set.spawn_local(async move {
                if let Err(e) = upload(corpus_config, worker_task_id, client_clone).await {
                    error!("Upload sync job failed: {}", e);
                }
            });
        }

        // Create necessary clones and pass along for download sync
        let mut last_updated = SystemTime::now();
        let corpus_config = self.config.clone();
        let refresh_interval = self.config.refresh_interval;
        local_set.spawn_local(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(refresh_interval));
            loop {
                interval.tick().await;
                let result = download_corpus_to_disk(corpus_config.label.clone(),
                                                     worker_task_id,
                                                     last_updated,
                                                     &corpus_config.path,
                                                     &mut client).await;
                // If successful update, set last_updated
                if let Err(e) = result {
                    error!("Download sync job failed: {}", e);
                } else {
                    last_updated = SystemTime::now();
                }
            }
        });

        local_set.await;

        Ok(())
    }
}

async fn upload(
        corpus: CorpusConfig,
        worker_task_id: Option<i32>,
        client: OrchestratorClient<Channel>) -> Result<(), Box<dyn Error>> {
    let mut client = client;
    info!("Creating corpus upload sync");
    let ext_regex = Regex::new(format!(".*\\.{}$", CORPUS_FILE_EXT).as_str()).unwrap();
    let mut watcher = InotifyFileWatcher::new(&corpus.path, Some(ext_regex))?;

    while let Some(file) = watcher.get_new_file().await {
        // Match user provided match pattern
        if corpus.upload_filter.is_match(file.as_str()) {
            let file_path = corpus.path.clone();
            let file_path = file_path.join(file);
            info!("Uploading new corpus: {:?}", file_path);
            upload_corpus_from_disk(file_path.as_path(), corpus.label.clone(), worker_task_id, &mut client).await?
        } else {
            debug!("Skipping upload of a user unmatched pattern: {:?}", file);
        }
    }
    Ok(())
}
