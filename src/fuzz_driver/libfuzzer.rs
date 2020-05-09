use log::debug;

use super::FuzzConfig;
use crate::executor::Executor;

pub struct LibFuzzerDriver {
    config: FuzzConfig,
    worker_task_id: Option<i32>,
}

impl super::FuzzDriver for LibFuzzerDriver {
    fn new(config: FuzzConfig, worker_task_id: Option<i32>) -> Self {
        debug!("Creating new libFuzzer driver with config {:#?}", config);
        Self { config, worker_task_id }
    }
}
