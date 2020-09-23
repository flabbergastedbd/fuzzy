use std::error::Error;
use std::path::Path;

use tracing::{debug, error, info, warn};
use regex::Regex;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, oneshot};
use validator::Validate;

use super::executor::{self, Executor, ExecutorConfig};
use crate::common::profiles::{validate_fuzz_profile, validate_relative_path};
use crate::common::worker_tasks::{mark_worker_task_active, mark_worker_task_inactive};
use stats::{FuzzStatCollector, FuzzStatConfig};

mod honggfuzz;
mod libfuzzer;
pub mod stats;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FuzzDriverEnum {
    Fuzzy,
    Honggfuzz,
    Libfuzzer,
}

#[derive(Serialize, Deserialize, Validate, Debug, Clone)]
#[validate(schema(function = "validate_fuzz_profile"))]
pub struct FuzzConfig {
    pub driver: FuzzDriverEnum,
    pub execution: ExecutorConfig,
    pub corpus: CorpusConfig,
    pub crash: CrashConfig,
    pub fuzz_stat: Option<FuzzStatConfig>,
}

#[derive(Serialize, Deserialize, Validate, Debug, Clone)]
pub struct CrashConfig {
    #[validate(custom = "validate_relative_path")]
    pub path: Box<Path>,

    #[validate(length(min = 1))]
    pub label: String,

    #[serde(with = "serde_regex")]
    pub filter: Regex,

    #[serde(default)]
    pub validate: Option<ExecutorConfig>,

    #[serde(default)]
    pub deduplicate: Option<ExecutorConfig>,
}

#[derive(Serialize, Deserialize, Validate, Debug, Clone)]
pub struct CorpusConfig {
    #[validate(custom = "validate_relative_path")]
    pub path: Box<Path>,
    pub label: String,
    pub refresh_interval: u64,
    pub upload: bool,

    #[serde(with = "serde_regex")]
    pub upload_filter: Regex,
}

#[tonic::async_trait]
pub trait FuzzDriver: Send {
    // fn new(config: FuzzConfig, worker_task_id: Option<i32>) -> Self where Self: Sized;

    // All methods to be used for `start`
    fn get_worker_task_id(&self) -> Option<i32>;
    fn get_fuzz_config(&self) -> FuzzConfig;
    fn set_fuzz_config(&mut self, config: FuzzConfig);

    // Custom stat collector if need be
    fn get_custom_stat_collector(
        &self,
        executor: &Box<dyn Executor>,
    ) -> Result<Option<Box<dyn FuzzStatCollector>>, Box<dyn Error>>;
    fn get_stat_collector(
        &self,
        executor: &Box<dyn Executor>,
    ) -> Result<Option<Box<dyn FuzzStatCollector>>, Box<dyn Error>> {
        let full_config = self.get_fuzz_config();
        if let Some(stat_config) = full_config.clone().fuzz_stat {
            Ok(Some(stats::new(stat_config, full_config, self.get_worker_task_id())))
        } else if let Some(stat_collector) = self.get_custom_stat_collector(executor)? {
            Ok(Some(stat_collector))
        } else {
            Ok(None)
        }
    }

    fn fix_args(&mut self);

    // Create corpus and crash dir
    async fn setup(&mut self, executor: &Box<dyn Executor>) -> Result<(), Box<dyn Error>> {
        let config = self.get_fuzz_config();
        executor.create_relative_dirp(&config.corpus.path).await?;
        executor.create_relative_dirp(&config.crash.path).await?;
        Ok(())
    }

    // Create corpus and crash dir
    async fn teardown(&mut self, executor: &Box<dyn Executor>) -> Result<(), Box<dyn Error>> {
        let config = self.get_fuzz_config();
        executor.rm_relative_dirp(&config.corpus.path).await?;
        // TODO: Donot delete crashes dir as of now
        // executor.create_relative_dirp(&config.crash.path);
        Ok(())
    }

    async fn start(
        &mut self,
        kill_switch: oneshot::Receiver<u8>,
        death_switch: oneshot::Sender<u8>,
    ) -> Result<(), Box<dyn Error>> {
        // Before anything call fix args, so that drivers can do changes
        self.fix_args();

        // Get a copy of these & ensure all mutations are done prior
        let worker_task_id = self.get_worker_task_id();
        let config = self.get_fuzz_config();

        info!("Starting generic fuzz driver for {:#?}", worker_task_id);

        // Setup runner, corpus syncer, crash syncer, stat collector
        let mut runner = executor::new(config.execution.clone(), worker_task_id);
        runner.setup().await?;
        self.setup(&runner).await?;

        // Spawn off corpus sync
        let mut corpus_syncer = runner.get_corpus_syncer(config.corpus.clone())?;
        corpus_syncer.setup_corpus().await?;

        // Spawn off crash sync
        let crash_syncer = runner.get_crash_syncer(config.crash.clone())?;

        // Stat collector
        let stats_collector = self.get_stat_collector(&runner)?;

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
            // Only unwrap if it is some
            result = stats_collector.unwrap().start(stat_longshot_recv), if stats_collector.is_some() => {
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
        self.teardown(&runner).await?;
        runner.close().await?;

        mark_worker_task_inactive(worker_task_id).await?;
        Ok(())
    }
}

pub struct FuzzyDriver {
    config: FuzzConfig,
    worker_task_id: Option<i32>,
}

impl FuzzyDriver {
    pub fn new(config: FuzzConfig, worker_task_id: Option<i32>) -> Self {
        Self { config, worker_task_id }
    }
}

#[tonic::async_trait]
impl FuzzDriver for FuzzyDriver {
    fn get_fuzz_config(&self) -> FuzzConfig {
        self.config.clone()
    }

    fn set_fuzz_config(&mut self, config: FuzzConfig) {
        self.config = config;
    }

    fn get_worker_task_id(&self) -> Option<i32> {
        self.worker_task_id.clone()
    }

    fn get_custom_stat_collector(
        &self,
        _: &Box<dyn Executor>,
    ) -> Result<Option<Box<dyn FuzzStatCollector>>, Box<dyn Error>> {
        Ok(None)
    }

    fn fix_args(&mut self) {}
}

pub fn new(config: FuzzConfig, worker_task_id: Option<i32>) -> Box<dyn FuzzDriver> {
    match config.driver {
        FuzzDriverEnum::Libfuzzer => {
            debug!("Creating libFuzzer driver");
            Box::new(libfuzzer::LibFuzzerDriver::new(config, worker_task_id))
        }
        FuzzDriverEnum::Honggfuzz => {
            debug!("Creating honggfuzz driver");
            Box::new(honggfuzz::HonggfuzzDriver::new(config, worker_task_id))
        }
        FuzzDriverEnum::Fuzzy => {
            debug!("Creating Fuzzy driver");
            Box::new(FuzzyDriver::new(config, worker_task_id))
        }
    }
}
