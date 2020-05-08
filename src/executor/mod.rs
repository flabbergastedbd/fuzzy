use std::path::Path;
use std::error::Error;

use log::debug;
use serde::{Serialize, Deserialize};
use tokio::{
    process::{ChildStdout, ChildStderr},
    io::{BufReader, Lines},
};

/**
 * For every addition here, make changes to src/cli.yaml possible values
 */

mod native;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ExecutorEnum {
    Native,
    Docker,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecutorConfig {
    executor: ExecutorEnum,
    executable: String,
    args: Vec<String>,
    cwd: Box<Path>,
    envs: Vec<(String, String)>,
}

#[tonic::async_trait]
pub trait Executor {
    fn new(config: ExecutorConfig) -> Self;

    async fn setup(&self) -> Result<(), Box<dyn Error>>;
    async fn launch(&mut self) -> Result<(), Box<dyn Error>>;

    // TODO: Improve these ChildStdout signatures to support other executors
    fn get_stdout_reader(&mut self) -> Option<Lines<BufReader<ChildStdout>>>;
    fn get_stderr_reader(&mut self) -> Option<Lines<BufReader<ChildStderr>>>;

    fn add_watch(&mut self, path: &Path) -> Result<usize, Box<dyn Error>>;
    async fn get_watched_files(&mut self, watch_index: usize) -> Option<String>;
    fn rm_watch(&mut self, watch_index: usize) -> Result<bool, Box<dyn Error>>;

    fn get_pid(&self) -> u32;
}

pub fn new(config: ExecutorConfig) -> impl Executor {
    match config.executor {
        _ => {
            debug!("Creating doccker executor");
            native::NativeExecutor::new(config)
        },
    }
}
