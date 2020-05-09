use std::error::Error;

use log::{error, info, debug, warn};
use regex::Regex;
use tokio::{task, sync::oneshot};

use super::FuzzConfig;
use crate::executor::{self, CrashConfig, Executor};

pub struct LibFuzzerDriver {
    config: FuzzConfig,
    worker_task_id: Option<i32>,
}

#[tonic::async_trait]
impl super::FuzzDriver for LibFuzzerDriver {
    fn new(config: FuzzConfig, worker_task_id: Option<i32>) -> Self {
        info!("Creating new libFuzzer driver with config {:#?}", config);
        Self { config, worker_task_id }
    }

    /// LibFuzzer driver needs to do couple of things
    /// 1. Setup corpus
    /// 2. Start corpus sync
    /// 3. Collect metrics from log files
    async fn start(&self, connect_addr: String, kill_switch: oneshot::Receiver<u8>) -> Result<(), Box<dyn Error>> {
        info!("Starting libfuzzer driver for {:#?}", self.worker_task_id);

        let mut runner = executor::new(self.config.execution.clone(), self.worker_task_id);

        // let local = task::LocalSet::new();

        // Spawn off corpus sync
        let corpus_syncer = runner.get_corpus_syncer().await?;
        corpus_syncer.setup_corpus(connect_addr.clone()).await?;
        let connect_addr_clone = connect_addr.clone();
        let corpus_sync_handle = task::spawn(async move {
            if let Err(e) = corpus_syncer.sync_corpus(connect_addr_clone).await {
                error!("Error in syncing corpus: {}", e);
            }
        });

        // Spawn off crash sync
        let crash_config = CrashConfig {
            label: self.config.execution.corpus.label.clone(),
            path: self.config.execution.cwd.clone(),
            filter: Regex::new("crash-.*")?,
        };
        let crash_syncer = runner.get_crash_syncer(crash_config).await?;
        let connect_addr_clone = connect_addr.clone();
        let crash_sync_handle = task::spawn(async move {
            if let Err(e) = crash_syncer.upload_crashes(connect_addr_clone).await {
                error!("Error in syncing crashes: {}", e);
            }
        });

        // Start the actual process
        runner.spawn().await?;

        // Listen and wait for all and kill switch
        tokio::select! {
            _ = corpus_sync_handle => {
                error!("Corpus sync exited first instead of kill switch");
            },
            _ = crash_sync_handle => {
                error!("Corpus sync exited first instead of kill switch");
            }
            _ = kill_switch => {
                info!("Received kill for lib fuzzer driver");
            },
        }

        // local.await;
        // If we reached here means one of the watches failed or kill switch triggered
        info!("Kill fuzzer process for {:?}", self.worker_task_id);
        runner.close()?;

        Ok(())
    }
}

impl LibFuzzerDriver {
}
