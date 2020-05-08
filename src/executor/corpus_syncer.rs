use std::error::Error;
use std::path::{Path, PathBuf};
use std::time::{UNIX_EPOCH, SystemTime, Duration};

use log::debug;
use tokio::{fs, task};
use tonic::{Request, transport::channel::Channel};

use crate::models::{NewCorpus, Corpus};
use crate::xpc::{self, orchestrator_client::OrchestratorClient};
use super::file_watcher::InotifyFileWatcher;
use super::CorpusConfig;
use crate::common::{upload_corpus, download_corpus};

pub struct CorpusSyncer {
    config: CorpusConfig,
    worker_task_id: Option<i32>,
}

impl CorpusSyncer {
    pub fn new(config: CorpusConfig, worker_task_id: Option<i32>) -> Result<Self, Box<dyn Error>> {
        Ok(Self { config, worker_task_id })
    }

    pub async fn sync_corpus(
            &self,
            connect_addr: String,
            refresh_interval: Duration) -> Result<(), Box<dyn Error>> {

        debug!("Will try to keep corpus in sync at: {:?}", self.config.path);
        let mut client = OrchestratorClient::connect(connect_addr).await?;
        let worker_task_id = self.worker_task_id;

        debug!("Syncing initial corpus");
        let corpus_config = self.config.clone();
        download(corpus_config, worker_task_id, UNIX_EPOCH, &mut client).await?;
        let now = SystemTime::now();

        // Create a local set
        let local_set = task::LocalSet::new();

        // Create necessary clones and pass along for upload sync
        let client_clone = client.clone();
        let corpus_config = self.config.clone();
        local_set.spawn_local(async move {
            upload(corpus_config, worker_task_id, client_clone).await;
        });

        // Create necessary clones and pass along for download sync
        let corpus_config = self.config.clone();
        let client_clone = client.clone();
        local_set.spawn_local(async move {
            let mut interval = tokio::time::interval(refresh_interval);
            loop {
                download(corpus_config.clone(), worker_task_id, now, &mut client).await;
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
    debug!("Creating corpus watcher");
    let mut watcher = InotifyFileWatcher::new(&corpus.path)?;

    debug!("Uploading new corpus");
    while let Some(file) = watcher.get_new_file().await {
        let mut file_path = PathBuf::from(corpus.path.clone());
        file_path.push(file);
        upload_corpus(file_path.as_path(), corpus.label.clone(), worker_task_id, &mut client).await
    }
    Ok(0)
}

async fn download(
        corpus: CorpusConfig,
        worker_task_id: Option<i32>,
        created_after: SystemTime,
        client: &mut OrchestratorClient<Channel>) -> Result<usize, Box<dyn Error>> {

    debug!("Downloading corpus");
    let files = download_corpus(corpus.label, worker_task_id, created_after, client).await?;
    for f in files.iter() {
        let mut file_path = PathBuf::from(corpus.path.clone());
        file_path.set_file_name(&f.checksum);
        file_path.set_extension("fuzzy");
        fs::write(file_path, &f.content).await?;
    }
    debug!("Written {} corpus files to disk", files.len());
    Ok(0)
}
