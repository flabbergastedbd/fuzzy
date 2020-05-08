use std::error::Error;
use std::path::{Path, PathBuf};

use log::debug;
use tokio::fs;
use tonic::{Request, transport::channel::Channel};

use crate::models::{NewCorpus, Corpus};
use crate::xpc::{self, orchestrator_client::OrchestratorClient};
use super::file_watcher::InotifyFileWatcher;

pub struct CorpusSyncer {
    corpus_dir: PathBuf,
    corpus_watcher: InotifyFileWatcher,
}

impl CorpusSyncer {
    pub fn new(path: &Path) -> Result<Self, Box<dyn Error>> {
        let corpus_dir = PathBuf::from(path.as_os_str());
        let corpus_watcher = InotifyFileWatcher::new(path)?;

        Ok(Self { corpus_dir, corpus_watcher })
    }

    pub async fn download_corpus(&self) -> Result<usize, Box<dyn Error>> {
        debug!("Downloading corpus");
        Ok(0)
    }

    pub async fn upload_corpus(&self) -> Result<usize, Box<dyn Error>> {
        debug!("Downloading corpus");
        Ok(0)
    }
}
