use log::debug;
use serde::{Serialize, Deserialize};

use super::executor::{self, ExecutorConfig};

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
    driver: FuzzDriverEnum,
    // execution: ExecutorConfig,
}

#[tonic::async_trait]
pub trait FuzzDriver {
    fn new(config: FuzzConfig, worker_task_id: Option<i32>) -> Self;
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
