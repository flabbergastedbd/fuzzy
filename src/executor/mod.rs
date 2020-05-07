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
    // cwd: String,
    envs: Vec<(String, String)>,
}

#[tonic::async_trait]
pub trait Executor {
    fn new(config: ExecutorConfig) -> Self;

    async fn setup(&self) -> Result<(), Box<dyn Error>>;
    fn launch(&mut self) -> Result<(), Box<dyn Error>>;

    async fn get_stdout_line(&mut self) -> Option<String>;
    fn get_stderr_reader(self) -> Option<BufReader<ChildStderr>>;


    fn id(&self) -> u32;
}

pub fn new(config: ExecutorConfig) -> impl Executor {
    match config.executor {
        _ => {
            debug!("Creating doccker executor");
            native::NativeExecutor::new(config)
        },
    }
}
