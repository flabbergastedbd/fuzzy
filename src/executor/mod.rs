use std::time::Duration;
use std::path::Path;
use std::error::Error;

use log::debug;
use serde::{Serialize, Deserialize};
use tokio::{
    process::{ChildStdout, ChildStderr},
    io::{BufReader, Lines},
};

use corpus_syncer::CorpusSyncer;

pub mod file_watcher;
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
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecutorConfig {
    executor: ExecutorEnum,
    executable: String,
    args: Vec<String>,
    cwd: Box<Path>,
    corpus: CorpusConfig,
    envs: Vec<(String, String)>,
    refresh_interval: u64,
}

#[tonic::async_trait]
pub trait Executor {
    fn new(config: ExecutorConfig, worker_task_id: Option<i32>) -> Self;

    async fn setup(&self) -> Result<(), Box<dyn Error>>;
    async fn launch(&mut self) -> Result<(), Box<dyn Error>>;

    // TODO: Improve these ChildStdout signatures to support other executors
    fn get_stdout_reader(&mut self) -> Option<Lines<BufReader<ChildStdout>>>;
    fn get_stderr_reader(&mut self) -> Option<Lines<BufReader<ChildStderr>>>;

    // TODO: Switch to generic trait based returns so we can swap file monitors
    // fn get_file_watcher(&self, path: Path) -> Box<dyn file_watcher::FileWatcher>;
    fn get_corpus_syncer(&self) -> Result<CorpusSyncer, Box<dyn Error>>;

    fn get_pid(&self) -> u32;
}

pub fn new(config: ExecutorConfig, worker_task_id: Option<i32>) -> impl Executor {
    match config.executor {
        _ => {
            debug!("Creating doccker executor");
            native::NativeExecutor::new(config, worker_task_id)
        },
    }
}
