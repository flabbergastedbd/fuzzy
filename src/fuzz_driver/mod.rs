use std::error::Error;

use log::{info, warn, error, debug};
use serde::{Serialize, Deserialize};
use tokio::sync::{oneshot, broadcast};
use tonic::Request;

use super::executor::{self, Executor, ExecutorConfig};
use crate::common::worker_tasks::{mark_worker_task_active, mark_worker_task_inactive};
use crate::models::NewFuzzStat;
use crate::common::xpc::get_orchestrator_client;
use crate::common::intervals::WORKER_FUZZDRIVER_STAT_UPLOAD_INTERVAL;

mod libfuzzer;
mod honggfuzz;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FuzzDriverEnum {
    Honggfuzz,
    Libfuzzer,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FuzzConfig {
    pub driver: FuzzDriverEnum,
    pub execution: ExecutorConfig,
}

#[tonic::async_trait]
pub trait FuzzDriver: Send {
    // fn new(config: FuzzConfig, worker_task_id: Option<i32>) -> Self where Self: Sized;

    // All methods to be used for `start`
    fn get_worker_task_id(&self) -> Option<i32>;
    fn get_fuzz_config(&self) -> FuzzConfig;
    fn set_fuzz_config(&mut self, config: FuzzConfig);

    // Custom stat collector if need be
    fn get_stat_collector(&self, executor: &Box<dyn Executor>) -> Result<Box<dyn FuzzStatCollector>, Box<dyn Error>>;

    fn fix_args(&mut self);

    fn setup(&mut self) {
        self.fix_args();
    }

    async fn start(&mut self, kill_switch: oneshot::Receiver<u8>, death_switch: oneshot::Sender<u8>) -> Result<(), Box<dyn Error>> {
        // Before anything call setup
        self.setup();

        // Get a copy of these & ensure all mutations are done prior
        let worker_task_id = self.get_worker_task_id();
        let config = self.get_fuzz_config();

        info!("Starting generic fuzz driver for {:#?}", worker_task_id);

        // Setup runner, corpus syncer, crash syncer, stat collector
        let mut runner = executor::new(config.execution.clone(), worker_task_id);
        runner.setup().await?;

        // Spawn off corpus sync
        let mut corpus_syncer = runner.get_corpus_syncer()?;
        corpus_syncer.setup_corpus().await?;

        // Spawn off crash sync
        let crash_syncer = runner.get_crash_syncer()?;

        // Stat collector
        let stats_collector = Box::new(self.get_stat_collector(&runner)?);

        // Start the actual process
        runner.spawn().await?;

        // Mark as task active
        mark_worker_task_active(worker_task_id).await?;
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
            result = stats_collector.start(stat_longshot_recv) => {
                error!("Error in collecting stats : {:?}", result);
            },
            _ = kill_switch => {
                warn!("Received kill for lib fuzzer driver");
            },
            result = runner.wait(runner_longshot_recv) => {
                error!("Error in executor: {:?}", result);
            },
        }
        let close_time = std::time::SystemTime::now();
        // If we are here it means select wrapped up from above
        // Close the fuzz process
        if let Err(e) = longshot.send(0) {
            error!("Error in sending longshot: {:?}", e);
        }
        if let Err(e) = death_switch.send(0) {
            error!("Error in sending death switch: {:?}", e);
        }
        info!("Sending kill signal for execturo {:?} as select! ended", worker_task_id);

        // Sync corpus first and then close the executor
        // Exactly reverse order of how things were created
        corpus_syncer.close(close_time).await?;
        runner.close().await?;

        mark_worker_task_inactive(worker_task_id).await?;
        Ok(())
    }
}

#[tonic::async_trait]
pub trait FuzzStatCollector: Send + Sync {
    async fn start(self: Box<Self>, mut _kill_switch: broadcast::Receiver<u8>) -> Result<(), Box<dyn Error>> {
        self.main_loop().await?;
        /* TODO: Stat collection kill switch disabled as we don't spawn as of now. Should be fine
         * https://users.rust-lang.org/t/explanation-on-fn-self-box-self-for-trait-objects/34024/3
        tokio::select! {
            result = self.main_loop() => {
                if let Err(e) = result {
                    error!("Stat collection exited with error: {}", e);
                }
            },
            _ = kill_switch.recv() => {},
        }
        */

        Ok(())
    }

    async fn main_loop(self: Box<Self>) -> Result<(), Box<dyn Error>> {
        let mut interval = tokio::time::interval(WORKER_FUZZDRIVER_STAT_UPLOAD_INTERVAL);
        let client = &get_orchestrator_client().await?;
        loop {
            interval.tick().await;
            let mut client = client.clone();
            // Iterate over logs and get stats
            let stat: Option<NewFuzzStat> = match self.get_stat().await {
                Ok(stat) => stat,
                Err(e) => {
                    error!("Failed to collect stat: {}", e);
                    None
                },
            };

            if let Some(stat) = stat {
                if let Err(e) = client.submit_fuzz_stat(Request::new(stat)).await {
                    error!("Failed to submit a fuzz stat: {}", e);
                }
            }
        }
    }

    async fn get_stat(&self) -> Result<Option<NewFuzzStat>, Box<dyn Error>>;
}

pub fn new(config: FuzzConfig, worker_task_id: Option<i32>) -> Box<dyn FuzzDriver> {
    match config.driver {
        FuzzDriverEnum::Libfuzzer => {
            debug!("Creating libFuzzer driver");
            Box::new(libfuzzer::LibFuzzerDriver::new(config, worker_task_id))
        },
        FuzzDriverEnum::Honggfuzz => {
            debug!("Creating honggfuzz driver");
            Box::new(honggfuzz::HonggfuzzDriver::new(config, worker_task_id))
        },
    }
}
