use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::{UNIX_EPOCH, SystemTime, Duration};

use regex::Regex;
use log::{info, error, debug};
use tokio::{fs, task};
use tonic::{Request, transport::channel::Channel};

use crate::models::{NewCorpus, Corpus};
use crate::xpc::{self, orchestrator_client::OrchestratorClient};
use super::file_watcher::InotifyFileWatcher;
use super::CorpusConfig;
use crate::common::{upload_corpus, download_corpus};

const CORPUS_FILE_EXT: &str = "fuzzy";

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
        download(self.config.clone(), self.worker_task_id, UNIX_EPOCH, &mut client).await?;
        Ok(())
    }

    pub async fn sync_corpus(
            &self,
            connect_addr: String,
        ) -> Result<(), Box<dyn Error>> {

        debug!("Will try to keep corpus in sync at: {:?}", self.config.path);
        let mut client = OrchestratorClient::connect(connect_addr).await?;
        let worker_task_id = self.worker_task_id;

        let now = SystemTime::now();

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
        let corpus_config = self.config.clone();
        let refresh_interval = self.config.refresh_interval;
        local_set.spawn_local(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(refresh_interval));
            loop {
                if let Err(e) = download(corpus_config.clone(), worker_task_id, now, &mut client).await {
                    error!("Download sync job failed: {}", e);
                }
                interval.tick().await;
            }
        });

        local_set.await;

        Ok(())
    }
}

async fn upload(
        corpus: CorpusConfig,
        worker_task_id: Option<i32>,
        client: OrchestratorClient<Channel>) -> Result<usize, Box<dyn Error>> {
    let mut client = client;
    info!("Creating corpus upload sync");
    let mut watcher = InotifyFileWatcher::new(&corpus.path)?;
    let ext_regex = Regex::new(format!(".*\\.{}$", CORPUS_FILE_EXT).as_str()).unwrap();

    while let Some(file) = watcher.get_new_file().await {
        // Don't match fuzzy downloaded file pattern & match user provided match pattern
        if ext_regex.is_match(file.as_str()) == false && corpus.pattern.is_match(file.as_str()) {
            let file_path = corpus.path.clone();
            let file_path = file_path.join(file);
            info!("Uploading new corpus: {:?}", file_path);
            upload_corpus(file_path.as_path(), corpus.label.clone(), worker_task_id, &mut client).await
        } else {
            debug!("Skipping a fuzzy file: {}", file);
        }
    }
    Ok(0)
}

async fn download(
        corpus: CorpusConfig,
        worker_task_id: Option<i32>,
        created_after: SystemTime,
        client: &mut OrchestratorClient<Channel>) -> Result<usize, Box<dyn Error>> {

    let files = download_corpus(corpus.label, worker_task_id, created_after, client).await?;
    for f in files.iter() {
        let file_path = corpus.path.clone();
        let mut file_path = file_path.join(&f.checksum);
        file_path.set_extension("fuzzy");
        debug!("Uploading new corpus: {:?}", file_path);
        fs::write(file_path, &f.content).await?;
    }
    info!("Written {} corpus files to {:?}", files.len(), corpus.path);
    Ok(0)
}
