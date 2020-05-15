use std::io::{BufRead, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::error::Error;

use log::{trace, error, info, debug, warn};
use regex::Regex;
use tokio::{fs, sync::oneshot, sync::broadcast};
use tonic::Request;

use super::FuzzConfig;
use crate::executor::{self, CrashConfig};
use crate::common::worker_tasks::{mark_worker_task_active, mark_worker_task_inactive};
use crate::models::NewFuzzStat;
use crate::common::xpc::get_orchestrator_client;

const HONGGFUZZ_LOG: &str = "honggfuzz.log";

pub struct HonggfuzzDriver {
    config: FuzzConfig,
    worker_task_id: Option<i32>,
}

#[tonic::async_trait]
impl super::FuzzDriver for HonggfuzzDriver {
    fn new(config: FuzzConfig, worker_task_id: Option<i32>) -> Self {
        info!("Creating new libFuzzer driver with config {:#?}", config);
        Self { config, worker_task_id }
    }

    async fn start(&mut self, kill_switch: oneshot::Receiver<u8>) -> Result<(), Box<dyn Error>> {
        self.fix_args();
        info!("Starting libfuzzer driver for {:#?}", self.worker_task_id);

        let mut runner = executor::new(self.config.execution.clone(), self.worker_task_id);

        // let local = task::LocalSet::new();

        // Spawn off corpus sync
        let corpus_syncer = runner.get_corpus_syncer()?;
        corpus_syncer.setup_corpus().await?;

        // Spawn off crash sync
        let crash_config = CrashConfig {
            label: self.config.execution.corpus.label.clone(),
            path: runner.get_cwd_path().into_boxed_path(),
            filter: Regex::new(r".*\.fuzz")?,
        };
        let crash_syncer = runner.get_crash_syncer(crash_config)?;

        // Stat collector
        let log_path = runner.get_cwd_path();

        // Start the actual process
        runner.spawn().await?;

        mark_worker_task_active(self.worker_task_id).await?;
        // Listen and wait for all and kill switch
        let (longshot, longshot_recv) = broadcast::channel(5);
        let crash_longshot_recv = longshot.subscribe();
        let stat_longshot_recv = longshot.subscribe();
        let runner_longshot_recv = longshot.subscribe();
        tokio::select! {
            result = corpus_syncer.sync_corpus(longshot_recv) => {
                error!("Error in syncing corpus: {:?}", result);
            },
            result = crash_syncer.upload_crashes(crash_longshot_recv) => {
                error!("Error in syncing crashes: {:?}", result);
            },
            _ = kill_switch => {
                warn!("Received kill for lib fuzzer driver");
            },
            result = runner.wait(runner_longshot_recv) => {
                error!("Error in executor: {:?}", result);
            },
        }
        // If we are here it means select wrapped up from above
        // Close the fuzz process
        if let Err(e) = longshot.send(0) {
            error!("Error in sending longshot: {:?}", e);
        }
        info!("Sending kill signal for execturo {:?} as select! ended", self.worker_task_id);
        runner.close().await?;

        mark_worker_task_inactive(self.worker_task_id).await?;

        // local.await;
        // If we reached here means one of the watches failed or kill switch triggered
        info!("Kill fuzzer process for {:?}", self.worker_task_id);
        // runner.close().await?;

        Ok(())
    }
}

impl HonggfuzzDriver {
    fn fix_args(&mut self) {
        self.config.execution.args.insert(0, "--threads".to_owned());
        self.config.execution.args.insert(1, format!("{}", self.config.execution.cpus));

        self.config.execution.args.insert(0, "--logfile".to_owned());
        self.config.execution.args.insert(1, HONGGFUZZ_LOG.to_owned());
    }
}
