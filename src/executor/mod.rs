use std::path::{Path, PathBuf};
use std::error::Error;
use std::collections::HashMap;

use log::debug;
use serde::{Serialize, Deserialize};
// use serde_regex::{Serialize, Deserialize};
use tokio::{
    process::{ChildStdout, ChildStderr},
    io::{BufReader, Lines},
    sync::broadcast,
};

use crate::fuzz_driver::{CrashConfig, CorpusConfig};
use corpus_syncer::CorpusSyncer;
use crash_syncer::CrashSyncer;

// Both of filesystem variants, need to change
pub mod corpus_syncer;
pub mod crash_syncer;
pub mod crash_validator;
pub mod crash_deduplicator;
mod native;
mod docker;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ExecutorEnum {
    Native,
    Docker,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecutorConfig {
    pub executor: ExecutorEnum,

    pub cpus: i32,

    // Only used if executor is docker
    #[serde(default)]
    pub image: String,

    pub executable: String,

    #[serde(default)]
    pub args: Vec<String>,

    pub cwd: Box<Path>,

    #[serde(default)]
    pub envs: HashMap<String, String>,
}


// Only fear was tokio::process::Child which seems to obey Send so we do too
#[tonic::async_trait]
pub trait Executor: Send + Sync {
    // Create a new executor with this configuration
    // fn new(config: ExecutorConfig, worker_task_id: Option<i32>) -> Self;
    async fn create_relative_dirp(&self, path: &Path) -> Result<(), Box<dyn Error>>;
    async fn rm_relative_dirp(&self, path: &Path) -> Result<(), Box<dyn Error>>;

    /// Setup stage often involves preparing things like cwd
    async fn setup(&self) -> Result<(), Box<dyn Error>>;

    /// Actually responsible for launching of the process
    async fn spawn(&mut self) -> Result<(), Box<dyn Error>>;
    async fn spawn_blocking(&mut self) -> Result<std::process::Output, Box<dyn Error>>;

    // TODO: Improve these ChildStdout signatures to support other executors
    /// Get stdout reader
    fn get_stdout_reader(&mut self) -> Option<Lines<BufReader<ChildStdout>>>;
    /// Get stderr reader
    fn get_stderr_reader(&mut self) -> Option<Lines<BufReader<ChildStderr>>>;

    // TODO: Switch to generic trait based returns so we can swap file monitors
    // fn get_file_watcher(&self, path: Path) -> Box<dyn file_watcher::FileWatcher>;
    fn get_corpus_syncer(&self, config: CorpusConfig) -> Result<CorpusSyncer, Box<dyn Error>>;
    fn get_crash_syncer(&self, config: CrashConfig) -> Result<CrashSyncer, Box<dyn Error>>;

    // Get absolute path for relative to cwd
    fn get_cwd_path(&self) -> PathBuf;

    async fn wait(&self, mut kill_switch: broadcast::Receiver<u8>) -> Result<(), Box<dyn Error>>;

    // Clean up all spawned children
    async fn close(mut self: Box<Self>) -> Result<(), Box<dyn Error>>;
}

pub fn new(config: ExecutorConfig, worker_task_id: Option<i32>) -> Box<dyn Executor> {
    match config.executor {
        ExecutorEnum::Native => {
            debug!("Creating native executor");
            Box::new(native::NativeExecutor::new(config, worker_task_id))
        },
        ExecutorEnum::Docker => {
            debug!("Creating docker executor");
            Box::new(docker::DockerExecutor::new(config, worker_task_id))
        }
    }
}
