use std::error::Error;
use std::time::{UNIX_EPOCH, SystemTime, Duration};

use regex::Regex;
use log::{info, error, debug};
use tonic::transport::channel::Channel;
use tokio::sync::broadcast;

use crate::xpc::orchestrator_client::OrchestratorClient;
use crate::utils::fs::FileWatcher;
use crate::fuzz_driver::CorpusConfig;
use crate::common::corpora::{upload_corpus_from_disk, download_corpus_to_disk, CORPUS_FILE_EXT};
use crate::common::xpc::get_orchestrator_client;

/// A file system corpus syncer. Need to convert this into trait when implementing docker
pub struct CorpusSyncer {
    config: CorpusConfig,
    worker_task_id: Option<i32>,
    last_download: SystemTime,
}

impl CorpusSyncer {
    pub fn new(config: CorpusConfig, worker_task_id: Option<i32>) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            config,
            worker_task_id,
            last_download: UNIX_EPOCH,
        })
    }

    pub async fn setup_corpus(&mut self) -> Result<(), Box<dyn Error>> {
        debug!("Syncing initial corpus");
        let mut client = get_orchestrator_client().await?;
        download_corpus_to_disk(
            self.config.label.clone(),
            self.worker_task_id,
            None,
            None,
            UNIX_EPOCH,
            &self.config.path,
            &mut client).await?;
        self.last_download = SystemTime::now();
        Ok(())
    }

    pub async fn sync_corpus(
            &self,
            mut kill_switch: broadcast::Receiver<u8>
        ) -> Result<(), Box<dyn Error>> {

        debug!("Will try to keep corpus in sync at: {:?}", self.config.path);
        let client = get_orchestrator_client().await?;

        tokio::select! {
            _ = self.download(client.clone()) => {
                error!("Downloading corpus exited first, whaaatt!");
            },
            // Doing this should be very necessary
            _ = self.upload(UNIX_EPOCH, client.clone(), true), if self.config.upload => {
                error!("Uploading corpus exited first, whaaatt!");
            },
            _ = kill_switch.recv() => {
                debug!("Kill receieved for corpus sync at {:?}", self.config);
            },
        }
        Ok(())
    }

    async fn upload(&self, last_upload: SystemTime, mut client: OrchestratorClient<Channel>, infinite_loop: bool) -> Result<(), Box<dyn Error>> {
        let mut interval = tokio::time::interval(Duration::from_secs(self.config.refresh_interval));
        // let mut client = client;
        info!("Creating corpus upload sync");
        let ext_regex = Regex::new(format!(".*\\.{}$", CORPUS_FILE_EXT).as_str()).unwrap();
        let mut watcher = FileWatcher::new(&self.config.path, Some(ext_regex), Some(self.config.upload_filter.clone()), last_upload)?;

        loop {
            // Match user provided match pattern
            let files = watcher.get_new_files()?;
            info!("Uploading {} new corpus to master", files.len());
            for file_path in files {
                info!("Uploading new corpus: {:?}", file_path);
                if let Err(e) = upload_corpus_from_disk(
                    file_path.as_path(),
                    self.config.label.clone(),
                    self.worker_task_id,
                    &mut client
                ).await {
                    error!("Failed to upload {:?} as corpus: {}", file_path.as_path(), e);
                }
            }

            if infinite_loop {
                interval.tick().await;
            } else {
                break;
            }
        }
        Ok(())
    }

    async fn download(&self, mut client: OrchestratorClient<Channel>) -> Result<(), Box<dyn Error>> {
        let mut interval = tokio::time::interval(Duration::from_secs(self.config.refresh_interval));
        let mut last_download = self.last_download.clone();
        loop {
            interval.tick().await;
            let result = download_corpus_to_disk(self.config.label.clone(),
                                                 self.worker_task_id,
                                                 None,
                                                 None,
                                                 last_download,
                                                 &self.config.path,
                                                 &mut client).await;
            // If successful update, set last_updated
            if let Err(e) = result {
                error!("Download sync job failed: {}", e);
            } else {
                last_download = SystemTime::now();
            }
        }
    }

    pub async fn close(self, last_upload: SystemTime) -> Result<(), Box<dyn Error>> {
        let client = get_orchestrator_client().await?;
        self.upload(last_upload, client, false).await?;
        Ok(())
    }
}
