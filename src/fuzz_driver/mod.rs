use std::error::Error;

use log::debug;
use serde::{Serialize, Deserialize};
use tokio::sync::oneshot;

use super::executor::ExecutorConfig;

mod libfuzzer;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum FuzzDriverEnum {
    Aflpp,
    Honggfuzz,
    Fuzzilli,
    Libfuzzer,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FuzzConfig {
    pub driver: FuzzDriverEnum,
    pub execution: ExecutorConfig,
}

#[tonic::async_trait]
pub trait FuzzDriver {
    fn new(config: FuzzConfig, worker_task_id: Option<i32>) -> Self where Self: Sized;

    async fn start(&self, kill_switch: oneshot::Receiver<u8>) -> Result<(), Box<dyn Error>>;
}

pub fn new(config: FuzzConfig, worker_task_id: Option<i32>) -> impl FuzzDriver {
    // let executor = executor::new(config.execution.clone(), worker_task_id);
    match config.driver {
        _ => {
            debug!("Creating libFuzzer driver");
            libfuzzer::LibFuzzerDriver::new(config, worker_task_id)
        }
    }
}
