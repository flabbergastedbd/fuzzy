use std::path::Path;
use std::error::Error;

use log::debug;
use serde::{Serialize, Deserialize};
use strum_macros::{Display, EnumString};

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

    async fn setup(self) -> Result<(), Box<dyn Error>>;
    fn launch(self) -> Result<(), Box<dyn Error>>;
    async fn grab_stdout(self) -> Result<Vec<u8>, Box<dyn Error>>;
}

pub fn new(executor_type: ExecutorEnum, config: ExecutorConfig) -> impl Executor {
    match executor_type {
        _ => {
            debug!("Creating doccker executor");
            native::NativeExecutor::new(config)
        },
    }
}
