use std::error::Error;

use log::debug;
use serde::{Serialize, Deserialize};
use tokio::sync::oneshot;

use super::executor::ExecutorConfig;

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
pub trait FuzzDriver: std::marker::Send {
    fn new(config: FuzzConfig, worker_task_id: Option<i32>) -> Self where Self: Sized;

    async fn start(&mut self, kill_switch: oneshot::Receiver<u8>) -> Result<(), Box<dyn Error>>;
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
