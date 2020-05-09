use std::path::Path;
use std::error::Error;
use std::collections::HashMap;

use regex::Regex;
use log::debug;
use serde::{Serialize, Deserialize};
// use serde_regex::{Serialize, Deserialize};
use tokio::{
    process::{ChildStdout, ChildStderr},
    io::{BufReader, Lines},
};

use corpus_syncer::CorpusSyncer;

pub mod corpus_syncer;
mod native;

/**
 * For every addition here, make changes to src/cli.yaml possible values
 */
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ExecutorEnum {
    Native,
    Docker,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CorpusConfig {
    path: Box<Path>,
    label: String,
    refresh_interval: u64,
    upload: bool,

    #[serde(with = "serde_regex")]
    upload_filter: Regex,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecutorConfig {
    executor: ExecutorEnum,
    executable: String,
    args: Vec<String>,
    cwd: Box<Path>,
    corpus: CorpusConfig,
    envs: HashMap<String, String>,
}

#[tonic::async_trait]
pub trait Executor {
    /// Create a new executor with this configuration
    fn new(config: ExecutorConfig, worker_task_id: Option<i32>) -> Self;

    /// Setup stage often involves preparing things like download
    /// corpus, make it ready for launch
    async fn setup(&self) -> Result<(), Box<dyn Error>>;

    /// Actually responsible for launching of the process
    async fn spawn(&mut self) -> Result<(), Box<dyn Error>>;

    // TODO: Improve these ChildStdout signatures to support other executors
    /// Get stdout reader
    fn get_stdout_reader(&mut self) -> Option<Lines<BufReader<ChildStdout>>>;
    /// Get stderr reader
    fn get_stderr_reader(&mut self) -> Option<Lines<BufReader<ChildStderr>>>;

    // TODO: Switch to generic trait based returns so we can swap file monitors
    // fn get_file_watcher(&self, path: Path) -> Box<dyn file_watcher::FileWatcher>;
    fn get_corpus_syncer(&self) -> Result<CorpusSyncer, Box<dyn Error>>;

    // Clean up all spawned children
    fn close(&mut self) -> Result<(), Box<dyn Error>>;
}

pub fn new(config: ExecutorConfig, worker_task_id: Option<i32>) -> impl Executor {
    match config.executor {
        _ => {
            debug!("Creating doccker executor");
            native::NativeExecutor::new(config, worker_task_id)
        },
    }
}
